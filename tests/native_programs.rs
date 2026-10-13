use std::{ops::Range, rc::Rc};

use zirql::{
    Bindings, Budget, EmissionPolicy, EvaluationError, Expression, ExpressionKind, ExpressionStep,
    Host, Located, Location, NativeInput, NativeValue, Pipeline, PredicateKind, Program,
    ProgramHost, ProgramLimits, QueryError, Stage, Statement, Truth, evaluate, evaluate_expression,
    evaluate_program,
};

fn limits() -> ProgramLimits {
    ProgramLimits {
        work: 10_000,
        results: 1_000,
        nesting: 16,
        stream_items: 8,
    }
}

fn at<T>(value: T, start: usize) -> Located<T> {
    Located::new(value, start..start + 1)
}

fn sequence<S, O, E>(
    input: Expression<S, O, E>,
    steps: Vec<Located<ExpressionStep<S, E>>>,
) -> Expression<S, O, E> {
    Located::new(
        ExpressionKind::Sequence {
            input: Box::new(input),
            steps,
        },
        0..100,
    )
}

fn output<S, O>(
    expression: Expression<S, O, &'static str>,
    implicit_output: Option<&'static str>,
) -> Located<Statement<S, O, &'static str>> {
    at(
        Statement::Evaluate {
            expression,
            implicit_output,
        },
        90,
    )
}

fn bind<S, O>(
    name: &str,
    start: usize,
    expression: Expression<S, O, &'static str>,
) -> Located<Statement<S, O, &'static str>> {
    at(
        Statement::Bind {
            name: at(name.to_owned(), start),
            expression,
        },
        start,
    )
}

fn emit<S>(
    name: &'static str,
    policy: EmissionPolicy,
    start: usize,
) -> Located<ExpressionStep<S, &'static str>> {
    at(
        ExpressionStep::Emit {
            emitter: name,
            policy,
        },
        start,
    )
}

#[derive(Debug, PartialEq, Eq)]
struct Word<'a> {
    text: &'a str,
    range: Range<usize>,
}

#[derive(Debug, PartialEq, Eq)]
enum SourceValue<'a> {
    Words(Vec<Word<'a>>),
    Count(usize),
    Record(Vec<(String, SourceValue<'a>)>),
}

type SourceExpression = Expression<SourceStage, (), &'static str>;

#[derive(Debug, PartialEq, Eq)]
enum SourceStage {
    Prefix(&'static str),
    Pipeline(Pipeline<()>),
    Select(Box<SourceExpression>),
    Fail,
}

struct SourceContext<'a> {
    source: &'a str,
    prefix: &'static str,
    emissions: Vec<(&'static str, SourceValue<'a>)>,
}

#[derive(Debug, PartialEq, Eq)]
enum SourceFailure {
    Source(Range<usize>),
}

struct SourceHost;

impl<'a> Host<Word<'a>, &mut SourceContext<'a>, ()> for SourceHost {
    type Error = SourceFailure;
    type Key = ();

    fn evaluate_leaf(
        &self,
        _leaf: &(),
        word: &Word<'a>,
        context: &mut &mut SourceContext<'a>,
        _budget: &mut Budget,
        _range: &Range<usize>,
    ) -> Result<Truth, EvaluationError<Self::Error>> {
        Ok(if word.text.starts_with(context.prefix) {
            Truth::True
        } else {
            Truth::False
        })
    }
}

impl<'a> ProgramHost<SourceContext<'a>> for SourceHost {
    type Value = SourceValue<'a>;
    type Stage = SourceStage;
    type SetOperator = ();
    type Emitter = &'static str;
    type Error = SourceFailure;

    fn copy_value(
        &self,
        value: &Self::Value,
        _context: &mut SourceContext<'a>,
        budget: &mut Budget,
        range: &Range<usize>,
    ) -> Result<Self::Value, EvaluationError<Self::Error>> {
        let location = Location::Query(range.clone());
        Ok(match value {
            SourceValue::Words(words) => {
                budget.check_items(words.len(), location.clone())?;
                budget.charge(words.len(), location.clone())?;
                budget.charge_results(words.len(), location)?;
                SourceValue::Words(
                    words
                        .iter()
                        .map(|word| Word {
                            text: word.text,
                            range: word.range.clone(),
                        })
                        .collect(),
                )
            }
            SourceValue::Count(count) => {
                budget.charge_results(1, location)?;
                SourceValue::Count(*count)
            }
            SourceValue::Record(fields) => {
                budget.charge_results(1 + fields.len(), location)?;
                let mut copied = Vec::with_capacity(fields.len());
                for (label, value) in fields {
                    budget.charge(label.len(), Location::Query(range.clone()))?;
                    copied.push((
                        label.clone(),
                        self.copy_value(value, _context, budget, range)?,
                    ));
                }
                SourceValue::Record(copied)
            }
        })
    }

    fn evaluate_stage(
        &self,
        stage: &SourceStage,
        value: Self::Value,
        context: &mut SourceContext<'a>,
        bindings: &Bindings<Self::Value>,
        budget: &mut Budget,
        range: &Range<usize>,
    ) -> Result<Self::Value, EvaluationError<Self::Error>> {
        match stage {
            SourceStage::Prefix(prefix) => {
                context.prefix = prefix;
                Ok(value)
            }
            SourceStage::Pipeline(pipeline) => {
                let SourceValue::Words(words) = value else {
                    return Err(
                        QueryError::query(range.clone(), "expected source selections").into(),
                    );
                };
                let input = NativeInput::from_admitted(
                    context,
                    words,
                    budget,
                    Location::Query(range.clone()),
                )?;
                Ok(match evaluate(pipeline, self, input, budget)?.value {
                    NativeValue::Stream(words) => SourceValue::Words(words),
                    NativeValue::Count(count) => SourceValue::Count(count),
                    NativeValue::Tally(_) => unreachable!("this host does not enable tally"),
                })
            }
            SourceStage::Select(selector) => {
                let SourceValue::Words(words) = value else {
                    return Err(
                        QueryError::query(range.clone(), "expected source selections").into(),
                    );
                };
                let mut selected = Vec::new();
                for word in words {
                    // Transfer the admitted row into relative input; it is not a copy.
                    let relative = SourceValue::Words(vec![word]);
                    let SourceValue::Words(rows) =
                        evaluate_expression(selector, self, relative, context, bindings, budget)?
                    else {
                        return Err(
                            QueryError::query(range.clone(), "expected selector stream").into()
                        );
                    };
                    budget
                        .check_items(selected.len() + rows.len(), Location::Query(range.clone()))?;
                    selected.extend(rows);
                }
                Ok(SourceValue::Words(selected))
            }
            SourceStage::Fail => Err(EvaluationError::Host(SourceFailure::Source(
                0..context.source.len(),
            ))),
        }
    }

    fn combine(
        &self,
        _operator: &(),
        left: Self::Value,
        right: Self::Value,
        _context: &mut SourceContext<'a>,
        budget: &mut Budget,
        range: &Range<usize>,
    ) -> Result<Self::Value, EvaluationError<Self::Error>> {
        let (SourceValue::Words(mut left), SourceValue::Words(right)) = (left, right) else {
            return Err(QueryError::query(range.clone(), "expected source selections").into());
        };
        for word in right {
            if !left.contains(&word) {
                budget.check_items(left.len() + 1, Location::Query(range.clone()))?;
                left.push(word);
            }
        }
        Ok(SourceValue::Words(left))
    }

    fn construct_record(
        &self,
        fields: Vec<(String, Self::Value)>,
        _context: &mut SourceContext<'a>,
        _budget: &mut Budget,
        _range: &Range<usize>,
    ) -> Result<Self::Value, EvaluationError<Self::Error>> {
        // Move the admitted field staging into the native record.
        Ok(SourceValue::Record(fields))
    }

    fn emit(
        &self,
        emitter: &&'static str,
        value: &Self::Value,
        context: &mut SourceContext<'a>,
        budget: &mut Budget,
        range: &Range<usize>,
    ) -> Result<(), EvaluationError<Self::Error>> {
        let snapshot = self.copy_value(value, context, budget, range)?;
        context.emissions.push((emitter, snapshot));
        Ok(())
    }
}

fn source_input<'a>(source: &'a str, budget: &mut Budget) -> SourceValue<'a> {
    let mut words = Vec::new();
    for text in source.split_whitespace() {
        let start = text.as_ptr() as usize - source.as_ptr() as usize;
        let range = start..start + text.len();
        budget
            .check_items(words.len() + 1, Location::Source(range.clone()))
            .unwrap();
        budget.charge(1, Location::Source(range.clone())).unwrap();
        budget
            .charge_results(1, Location::Source(range.clone()))
            .unwrap();
        words.push(Word { text, range });
    }
    SourceValue::Words(words)
}

fn count_stage() -> Located<ExpressionStep<SourceStage, &'static str>> {
    at(
        ExpressionStep::Stage(SourceStage::Pipeline(
            Pipeline::new(vec![at(Stage::Count, 40)], 40..41, 16).unwrap(),
        )),
        40,
    )
}

#[test]
fn borrowed_program_restores_native_bindings_and_relative_input_with_one_context() {
    let source = String::from("βeta alpha apple");
    let mut context = SourceContext {
        source: &source,
        prefix: "",
        emissions: Vec::new(),
    };
    let filter = Pipeline::new(
        vec![at(Stage::Filter(at(PredicateKind::Leaf(()), 5)), 4)],
        4..6,
        16,
    )
    .unwrap();
    let selector = at(
        ExpressionKind::Group(Box::new(sequence(
            at(ExpressionKind::Input, 3),
            vec![at(ExpressionStep::Stage(SourceStage::Pipeline(filter)), 4)],
        ))),
        2,
    );
    let selected = sequence(
        at(ExpressionKind::Input, 0),
        vec![
            at(ExpressionStep::Stage(SourceStage::Prefix("a")), 1),
            at(
                ExpressionStep::Stage(SourceStage::Select(Box::new(selector))),
                2,
            ),
        ],
    );
    let report = at(
        ExpressionKind::Record(vec![
            (
                at("selection".to_owned(), 50),
                at(ExpressionKind::Binding("selected".to_owned()), 51),
            ),
            (
                at("size".to_owned(), 52),
                sequence(
                    at(ExpressionKind::Binding("selected".to_owned()), 53),
                    vec![count_stage()],
                ),
            ),
        ]),
        49,
    );
    let program = Program::new(
        vec![
            bind("selected", 10, selected),
            output(
                sequence(
                    at(ExpressionKind::Binding("selected".to_owned()), 20),
                    vec![
                        emit("selection", EmissionPolicy::PassThrough, 21),
                        count_stage(),
                        emit("count", EmissionPolicy::Terminal, 41),
                    ],
                ),
                Some("suppressed"),
            ),
            bind("report", 48, report),
            output(
                at(ExpressionKind::Binding("report".to_owned()), 60),
                Some("report"),
            ),
        ],
        0..100,
    );
    let mut budget = Budget::for_program(limits());
    let input = source_input(&source, &mut budget);
    let bindings = evaluate_program(
        &program,
        &SourceHost,
        input,
        &mut context,
        Bindings::default(),
        &mut budget,
    )
    .unwrap();
    assert_eq!(context.prefix, "a");
    assert_eq!(context.source.as_ptr(), source.as_ptr());
    assert_eq!(
        context
            .emissions
            .iter()
            .map(|(name, _)| *name)
            .collect::<Vec<_>>(),
        ["selection", "count", "report"]
    );
    assert_eq!(context.emissions[1].1, SourceValue::Count(2));
    let SourceValue::Words(words) = bindings.get("selected", 20..21).unwrap() else {
        panic!("expected native selections")
    };
    assert_eq!(
        words,
        &[
            Word {
                text: &source[6..11],
                range: 6..11
            },
            Word {
                text: &source[12..17],
                range: 12..17
            },
        ]
    );
    assert_eq!(words[0].text.as_ptr(), source[6..11].as_ptr());
    let SourceValue::Record(fields) = &context.emissions[2].1 else {
        panic!("expected native report")
    };
    assert_eq!(
        fields
            .iter()
            .map(|(name, _)| name.as_str())
            .collect::<Vec<_>>(),
        ["selection", "size"]
    );
    assert_eq!(&fields[0].1, bindings.get("selected", 20..21).unwrap());
    assert_eq!(fields[1].1, SourceValue::Count(2));

    // Every stream fits three rows. Repeated native copies exhaust the aggregate.
    let copy_program = Program::new(
        vec![
            bind("all", 10, at(ExpressionKind::Input, 0)),
            output(
                at(ExpressionKind::Binding("all".to_owned()), 20),
                Some("all"),
            ),
        ],
        0..100,
    );
    context.emissions.clear();
    let mut budget = Budget::for_program(ProgramLimits {
        results: 8,
        ..limits()
    });
    let input = source_input(&source, &mut budget);
    assert_eq!(
        evaluate_program(
            &copy_program,
            &SourceHost,
            input,
            &mut context,
            Bindings::default(),
            &mut budget
        )
        .unwrap_err(),
        EvaluationError::Core(QueryError::query(20..21, "result limit exceeded"))
    );
    assert!(context.emissions.is_empty());

    // A forward read in a later statement prevents the earlier emitter running.
    let forward = Program::new(
        vec![
            output(
                sequence(
                    at(ExpressionKind::Input, 0),
                    vec![emit("early", EmissionPolicy::PassThrough, 1)],
                ),
                None,
            ),
            output(at(ExpressionKind::Binding("later".to_owned()), 30), None),
            bind("later", 40, at(ExpressionKind::Input, 41)),
        ],
        0..100,
    );
    let mut budget = Budget::for_program(limits());
    let input = source_input(&source, &mut budget);
    assert_eq!(
        evaluate_program(
            &forward,
            &SourceHost,
            input,
            &mut context,
            Bindings::default(),
            &mut budget
        )
        .unwrap_err(),
        EvaluationError::Core(QueryError::query(30..31, "undefined binding: later"))
    );
    assert!(context.emissions.is_empty());

    let duplicate = Program::new(
        vec![
            bind("all", 10, at(ExpressionKind::Input, 0)),
            bind("all", 30, at(ExpressionKind::Input, 31)),
        ],
        0..100,
    );
    let input = source_input(&source, &mut budget);
    assert_eq!(
        evaluate_program(
            &duplicate,
            &SourceHost,
            input,
            &mut context,
            Bindings::default(),
            &mut budget
        )
        .unwrap_err(),
        EvaluationError::Core(QueryError::query(30..31, "binding is already defined"))
    );
    let initial =
        Bindings::new([(at("all".to_owned(), 0), source_input(&source, &mut budget))]).unwrap();
    let input = source_input(&source, &mut budget);
    assert_eq!(
        evaluate_program(
            &copy_program,
            &SourceHost,
            input,
            &mut context,
            initial,
            &mut budget
        )
        .unwrap_err(),
        EvaluationError::Core(QueryError::query(10..11, "binding is already defined"))
    );

    // Opaque relative selectors share active nesting. A refusal restores it.
    let nested = at(
        ExpressionKind::Group(Box::new(sequence(
            at(ExpressionKind::Input, 0),
            vec![at(
                ExpressionStep::Stage(SourceStage::Select(Box::new(at(
                    ExpressionKind::Group(Box::new(at(ExpressionKind::Input, 71))),
                    70,
                )))),
                69,
            )],
        ))),
        68,
    );
    let mut budget = Budget::for_program(ProgramLimits {
        nesting: 1,
        ..limits()
    });
    let input = source_input(&source, &mut budget);
    assert_eq!(
        evaluate_expression(
            &nested,
            &SourceHost,
            input,
            &mut context,
            &Bindings::default(),
            &mut budget
        )
        .unwrap_err(),
        EvaluationError::Core(QueryError::query(70..71, "nesting limit exceeded"))
    );
    let input = source_input(&source, &mut budget);
    assert!(
        matches!(evaluate_expression(&at(ExpressionKind::Input, 0), &SourceHost, input, &mut context, &Bindings::default(), &mut budget).unwrap(), SourceValue::Words(words) if words.len() == 3)
    );

    // A native runtime failure preserves its type and the host's collected tap.
    let failure = sequence(
        at(ExpressionKind::Input, 0),
        vec![
            emit("tap", EmissionPolicy::PassThrough, 80),
            at(ExpressionStep::Stage(SourceStage::Fail), 81),
        ],
    );
    let input = source_input(&source, &mut budget);
    assert_eq!(
        evaluate_expression(
            &failure,
            &SourceHost,
            input,
            &mut context,
            &Bindings::default(),
            &mut budget
        )
        .unwrap_err(),
        EvaluationError::Host(SourceFailure::Source(0..source.len()))
    );
    assert_eq!(context.emissions[0].0, "tap");

    // Representation visits do not add work to an existing legacy budget.
    let mut budget = Budget::new(zirql::Limits {
        work: 4,
        items: 8,
        nesting: 16,
    });
    let input = source_input(&source, &mut budget);
    assert_eq!(
        evaluate_expression(
            &sequence(at(ExpressionKind::Input, 0), vec![count_stage()]),
            &SourceHost,
            input,
            &mut context,
            &Bindings::default(),
            &mut budget,
        )
        .unwrap(),
        SourceValue::Count(3)
    );
}

#[test]
fn borrowed_flat_sets_keep_operand_order_and_independent_input() {
    let source = String::from("βeta alpha apple");
    let mut context = SourceContext {
        source: &source,
        prefix: "",
        emissions: Vec::new(),
    };
    let operand = |index: usize| {
        let (prefix, emitter) = if index.is_multiple_of(2) {
            ("β", "beta")
        } else {
            ("a", "a")
        };
        sequence(
            at(ExpressionKind::Input, index * 4),
            vec![
                at(
                    ExpressionStep::Stage(SourceStage::Prefix(prefix)),
                    index * 4 + 1,
                ),
                at(
                    ExpressionStep::Stage(SourceStage::Pipeline(
                        Pipeline::new(
                            vec![at(
                                Stage::Filter(at(PredicateKind::Leaf(()), index * 4 + 2)),
                                index * 4 + 2,
                            )],
                            index * 4 + 2..index * 4 + 3,
                            16,
                        )
                        .unwrap(),
                    )),
                    index * 4 + 2,
                ),
                emit(emitter, EmissionPolicy::PassThrough, index * 4 + 3),
            ],
        )
    };
    let mut expression = operand(0);
    for index in 1..=200 {
        expression = at(
            ExpressionKind::Set {
                operator: (),
                left: Box::new(expression),
                right: Box::new(operand(index)),
            },
            1_000 + index,
        );
    }
    let program = Program::new(vec![bind("combined", 2_000, expression)], 0..2_001);
    let mut budget = Budget::for_program(ProgramLimits {
        nesting: 3,
        ..limits()
    });
    let input = source_input(&source, &mut budget);
    let bindings = evaluate_program(
        &program,
        &SourceHost,
        input,
        &mut context,
        Bindings::default(),
        &mut budget,
    )
    .unwrap();
    let SourceValue::Words(words) = bindings.get("combined", 2_000..2_001).unwrap() else {
        panic!("expected native selections")
    };
    assert_eq!(
        words.iter().map(|word| word.text).collect::<Vec<_>>(),
        ["βeta", "alpha", "apple"]
    );
    assert_eq!(words[0].text.as_ptr(), source.as_ptr());
    assert_eq!(context.emissions.len(), 201);
    for (index, (emitter, value)) in context.emissions.iter().enumerate() {
        let SourceValue::Words(words) = value else {
            panic!("expected native selections")
        };
        if index.is_multiple_of(2) {
            assert_eq!(*emitter, "beta");
            assert_eq!(
                words.iter().map(|word| word.text).collect::<Vec<_>>(),
                ["βeta"]
            );
        } else {
            assert_eq!(*emitter, "a");
            assert_eq!(
                words.iter().map(|word| word.text).collect::<Vec<_>>(),
                ["alpha", "apple"]
            );
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
struct OperationId {
    owner: usize,
    index: usize,
}

struct Document {
    owner: usize,
    operations: Vec<&'static str>,
}

impl Document {
    fn resolve(&self, id: &OperationId) -> Option<&'static str> {
        (id.owner == self.owner)
            .then(|| self.operations.get(id.index).copied())
            .flatten()
    }
}

#[derive(Debug, PartialEq, Eq)]
enum DocumentValue {
    Handles(Vec<OperationId>),
    Record(Vec<(String, DocumentValue)>),
}

#[derive(Debug, PartialEq, Eq)]
enum DocumentStage {
    Pipeline(Pipeline<&'static str>),
    Select(Box<Expression<DocumentStage, (), &'static str>>),
}

struct DocumentContext {
    document: Rc<Document>,
    emissions: Vec<(&'static str, DocumentValue)>,
}
struct DocumentHost;

impl Host<OperationId, &mut DocumentContext, &'static str> for DocumentHost {
    type Error = QueryError;
    type Key = ();

    fn evaluate_leaf(
        &self,
        leaf: &&'static str,
        item: &OperationId,
        context: &mut &mut DocumentContext,
        _budget: &mut Budget,
        range: &Range<usize>,
    ) -> Result<Truth, EvaluationError<QueryError>> {
        let name = context.document.resolve(item).ok_or_else(|| {
            EvaluationError::Host(QueryError::query(range.clone(), "invalid operation owner"))
        })?;
        Ok(if name == *leaf {
            Truth::True
        } else {
            Truth::False
        })
    }
}

impl ProgramHost<DocumentContext> for DocumentHost {
    type Value = DocumentValue;
    type Stage = DocumentStage;
    type SetOperator = ();
    type Emitter = &'static str;
    type Error = QueryError;

    fn copy_value(
        &self,
        value: &DocumentValue,
        _context: &mut DocumentContext,
        budget: &mut Budget,
        range: &Range<usize>,
    ) -> Result<DocumentValue, EvaluationError<QueryError>> {
        let location = Location::Query(range.clone());
        Ok(match value {
            DocumentValue::Handles(ids) => {
                budget.check_items(ids.len(), location.clone())?;
                budget.charge(ids.len(), location.clone())?;
                budget.charge_results(ids.len(), location)?;
                DocumentValue::Handles(
                    ids.iter()
                        .map(|id| OperationId {
                            owner: id.owner,
                            index: id.index,
                        })
                        .collect(),
                )
            }
            DocumentValue::Record(fields) => {
                budget.charge_results(1 + fields.len(), location)?;
                let mut copied = Vec::with_capacity(fields.len());
                for (label, value) in fields {
                    budget.charge(label.len(), Location::Query(range.clone()))?;
                    copied.push((
                        label.clone(),
                        self.copy_value(value, _context, budget, range)?,
                    ));
                }
                DocumentValue::Record(copied)
            }
        })
    }

    fn evaluate_stage(
        &self,
        stage: &DocumentStage,
        value: DocumentValue,
        context: &mut DocumentContext,
        bindings: &Bindings<DocumentValue>,
        budget: &mut Budget,
        range: &Range<usize>,
    ) -> Result<DocumentValue, EvaluationError<QueryError>> {
        match stage {
            DocumentStage::Pipeline(pipeline) => {
                let DocumentValue::Handles(ids) = value else {
                    return Err(QueryError::query(range.clone(), "expected handles").into());
                };
                let input = NativeInput::from_admitted(
                    context,
                    ids,
                    budget,
                    Location::Query(range.clone()),
                )?;
                let NativeValue::Stream(ids) = evaluate(pipeline, self, input, budget)?.value
                else {
                    return Err(QueryError::query(range.clone(), "expected handle stream").into());
                };
                Ok(DocumentValue::Handles(ids))
            }
            DocumentStage::Select(selector) => {
                evaluate_expression(selector, self, value, context, bindings, budget)
            }
        }
    }

    fn combine(
        &self,
        _operator: &(),
        left: DocumentValue,
        right: DocumentValue,
        _context: &mut DocumentContext,
        budget: &mut Budget,
        range: &Range<usize>,
    ) -> Result<DocumentValue, EvaluationError<QueryError>> {
        let (DocumentValue::Handles(mut left), DocumentValue::Handles(right)) = (left, right)
        else {
            return Err(QueryError::query(range.clone(), "expected handles").into());
        };
        for id in right {
            if !left.contains(&id) {
                budget.check_items(left.len() + 1, Location::Query(range.clone()))?;
                left.push(id);
            }
        }
        Ok(DocumentValue::Handles(left))
    }

    fn construct_record(
        &self,
        fields: Vec<(String, DocumentValue)>,
        _context: &mut DocumentContext,
        _budget: &mut Budget,
        _range: &Range<usize>,
    ) -> Result<DocumentValue, EvaluationError<QueryError>> {
        Ok(DocumentValue::Record(fields))
    }

    fn emit(
        &self,
        emitter: &&'static str,
        value: &DocumentValue,
        context: &mut DocumentContext,
        budget: &mut Budget,
        range: &Range<usize>,
    ) -> Result<(), EvaluationError<QueryError>> {
        let snapshot = self.copy_value(value, context, budget, range)?;
        context.emissions.push((emitter, snapshot));
        Ok(())
    }
}

fn document_input(budget: &mut Budget) -> DocumentValue {
    budget.check_items(4, Location::Source(0..3)).unwrap();
    budget.charge(4, Location::Source(0..3)).unwrap();
    budget.charge_results(4, Location::Source(0..3)).unwrap();
    DocumentValue::Handles(
        [2, 0, 2, 1]
            .map(|index| OperationId { owner: 7, index })
            .into(),
    )
}

fn document_head(
    count: usize,
    start: usize,
) -> Located<ExpressionStep<DocumentStage, &'static str>> {
    at(
        ExpressionStep::Stage(DocumentStage::Pipeline(
            Pipeline::new(vec![at(Stage::Head(count), start)], start..start + 1, 16).unwrap(),
        )),
        start,
    )
}

#[test]
fn document_program_keeps_owner_order_set_operands_and_native_records() {
    let document = Rc::new(Document {
        owner: 7,
        operations: vec!["add", "mul", "add"],
    });
    let mut context = DocumentContext {
        document: Rc::clone(&document),
        emissions: Vec::new(),
    };
    let left = sequence(
        at(ExpressionKind::Binding("chosen".to_owned()), 20),
        vec![emit("left", EmissionPolicy::PassThrough, 21)],
    );
    let right = at(
        ExpressionKind::Group(Box::new(sequence(
            at(ExpressionKind::Input, 30),
            vec![
                document_head(1, 31),
                emit("right", EmissionPolicy::PassThrough, 32),
            ],
        ))),
        29,
    );
    let set = at(
        ExpressionKind::Set {
            operator: (),
            left: Box::new(left),
            right: Box::new(right),
        },
        19,
    );
    let selector = at(
        ExpressionKind::Group(Box::new(sequence(
            at(ExpressionKind::Input, 41),
            vec![document_head(2, 42)],
        ))),
        40,
    );
    let program = Program::new(
        vec![
            bind(
                "chosen",
                10,
                sequence(at(ExpressionKind::Input, 0), vec![document_head(2, 1)]),
            ),
            output(
                sequence(
                    set,
                    vec![
                        emit("union", EmissionPolicy::PassThrough, 33),
                        at(
                            ExpressionStep::Stage(DocumentStage::Select(Box::new(selector))),
                            39,
                        ),
                        emit("final", EmissionPolicy::Terminal, 43),
                    ],
                ),
                Some("suppressed"),
            ),
            output(
                at(
                    ExpressionKind::Record(vec![(
                        at("handles".to_owned(), 50),
                        at(ExpressionKind::Binding("chosen".to_owned()), 51),
                    )]),
                    49,
                ),
                Some("record"),
            ),
        ],
        0..100,
    );
    let mut budget = Budget::for_program(limits());
    let input = document_input(&mut budget);
    let bindings = evaluate_program(
        &program,
        &DocumentHost,
        input,
        &mut context,
        Bindings::default(),
        &mut budget,
    )
    .unwrap();
    assert!(Rc::ptr_eq(&context.document, &document));
    assert_eq!(
        context
            .emissions
            .iter()
            .map(|(name, _)| *name)
            .collect::<Vec<_>>(),
        ["left", "right", "union", "final", "record"]
    );
    let expected = DocumentValue::Handles(vec![
        OperationId { owner: 7, index: 2 },
        OperationId { owner: 7, index: 0 },
    ]);
    assert_eq!(bindings.get("chosen", 20..21).unwrap(), &expected);
    assert_eq!(context.emissions[2].1, expected);
    assert_eq!(context.emissions[3].1, expected);
    let DocumentValue::Record(fields) = &context.emissions[4].1 else {
        panic!("expected record")
    };
    assert_eq!(fields[0].0, "handles");
    assert_eq!(fields[0].1, expected);
    let DocumentValue::Handles(ids) = &fields[0].1 else {
        panic!("expected handles")
    };
    assert_eq!(document.resolve(&ids[0]), Some("add"));

    // Final input and already-admitted returns move without another admission.
    context.emissions.clear();
    let mut budget = Budget::for_program(ProgramLimits {
        results: 4,
        ..limits()
    });
    let input = document_input(&mut budget);
    let moved = evaluate_expression(
        &sequence(at(ExpressionKind::Input, 0), vec![document_head(2, 1)]),
        &DocumentHost,
        input,
        &mut context,
        &Bindings::default(),
        &mut budget,
    )
    .unwrap();
    assert_eq!(moved, expected);

    // Terminal placement is rejected before a preceding emitter can run.
    let terminal = sequence(
        at(ExpressionKind::Input, 0),
        vec![
            emit("terminal", EmissionPolicy::Terminal, 60),
            document_head(1, 61),
        ],
    );
    let mut budget = Budget::for_program(limits());
    let input = document_input(&mut budget);
    assert_eq!(
        evaluate_expression(
            &terminal,
            &DocumentHost,
            input,
            &mut context,
            &Bindings::default(),
            &mut budget
        )
        .unwrap_err(),
        EvaluationError::Core(QueryError::query(
            61..62,
            "a step cannot follow terminal emission"
        ))
    );
    assert!(context.emissions.is_empty());

    let copy_program = Program::new(
        vec![
            bind("all", 10, at(ExpressionKind::Input, 0)),
            output(at(ExpressionKind::Binding("all".to_owned()), 20), None),
        ],
        0..100,
    );
    let mut budget = Budget::for_program(ProgramLimits {
        results: 11,
        ..limits()
    });
    let input = document_input(&mut budget);
    assert_eq!(
        evaluate_program(
            &copy_program,
            &DocumentHost,
            input,
            &mut context,
            Bindings::default(),
            &mut budget
        )
        .unwrap_err(),
        EvaluationError::Core(QueryError::query(20..21, "result limit exceeded"))
    );
}

mod projection {
    use std::{collections::BTreeMap, ops::Range, sync::Arc};
    use zirql::{
        ArrayType, Bindings, Budget, ConstructedValue as V, EvaluationError, Expression,
        ExpressionKind, ExpressionStep, Located, Location, NativeOperands, Predicate, ProgramHost,
        ProgramLimits, Projection, ProjectionHost, QueryError, ReadExpression,
        ValueExpressionKind as K, ValueReaderHost, ValueType,
    };

    #[derive(Debug, PartialEq)]
    enum Value {
        Subjects(Vec<&'static str>),
        Strings(Vec<String>),
        Count(usize),
        Constructed(V),
    }
    #[derive(Clone, PartialEq)]
    enum Kind {
        Subjects,
        Strings,
        Count,
        Constructed(Arc<Vec<ValueType>>),
    }
    #[derive(Debug, PartialEq)]
    enum Stage {
        Names,
        Count,
        Fail,
        Edit,
        Saved(String),
        Projection(Box<Projection<Query>>),
    }
    struct Host;
    type Query = Expression<Stage, (), ()>;
    impl ProgramHost<()> for Host {
        type Value = Value;
        type Stage = Stage;
        type SetOperator = ();
        type Emitter = ();
        type Error = QueryError;
        fn copy_value(
            &self,
            value: &Value,
            _: &mut (),
            budget: &mut Budget,
            range: &Range<usize>,
        ) -> Result<Value, EvaluationError<QueryError>> {
            let location = Location::Query(range.clone());
            Ok(match value {
                Value::Subjects(rows) => {
                    budget.charge(rows.len(), location.clone())?;
                    budget.charge_results(rows.len(), location)?;
                    Value::Subjects(rows.clone())
                }
                Value::Strings(values) => {
                    budget.charge(values.iter().map(String::len).sum(), location.clone())?;
                    budget.charge_results(values.len(), location)?;
                    Value::Strings(values.clone())
                }
                Value::Count(count) => {
                    budget.charge_results(1, location)?;
                    Value::Count(*count)
                }
                Value::Constructed(value) => {
                    Value::Constructed(zirql::copy_constructed_value(value, budget, location)?)
                }
            })
        }
        fn evaluate_stage(
            &self,
            stage: &Stage,
            value: Value,
            context: &mut (),
            bindings: &Bindings<Value>,
            budget: &mut Budget,
            range: &Range<usize>,
        ) -> Result<Value, EvaluationError<QueryError>> {
            budget.charge(1, Location::Query(range.clone()))?;
            match stage {
                Stage::Saved(name) => {
                    self.copy_value(bindings.get(name, range.clone())?, context, budget, range)
                }
                Stage::Projection(projection) => {
                    zirql::evaluate_projection(projection, self, value, context, bindings, budget)
                        .map(Value::Constructed)
                }
                Stage::Names => {
                    let Value::Subjects(rows) = value else {
                        return Err(QueryError::query(range.clone(), "names needs subjects").into());
                    };
                    budget.charge_results(rows.len(), Location::Query(range.clone()))?;
                    Ok(Value::Strings(
                        rows.into_iter().map(str::to_owned).collect(),
                    ))
                }
                Stage::Count => {
                    let count = match value {
                        Value::Subjects(rows) => rows.len(),
                        Value::Strings(rows) => rows.len(),
                        _ => {
                            return Err(
                                QueryError::query(range.clone(), "count needs a stream").into()
                            );
                        }
                    };
                    Ok(Value::Count(count))
                }
                Stage::Fail => Err(EvaluationError::Host(QueryError::source(
                    20..21,
                    "native failure",
                ))),
                Stage::Edit => unreachable!("preflight refuses edits"),
            }
        }
        fn combine(
            &self,
            _: &(),
            left: Value,
            right: Value,
            _: &mut (),
            budget: &mut Budget,
            range: &Range<usize>,
        ) -> Result<Value, EvaluationError<QueryError>> {
            let (Value::Strings(mut left), Value::Strings(right)) = (left, right) else {
                return Err(QueryError::query(range.clone(), "set operands need strings").into());
            };
            for value in right {
                budget.charge(1, Location::Query(range.clone()))?;
                if !left.contains(&value) {
                    left.push(value);
                }
            }
            Ok(Value::Strings(left))
        }
        fn construct_record(
            &self,
            _: Vec<(String, Value)>,
            _: &mut (),
            _: &mut Budget,
            range: &Range<usize>,
        ) -> Result<Value, EvaluationError<QueryError>> {
            Err(QueryError::query(range.clone(), "native records unavailable").into())
        }
        fn emit(
            &self,
            _: &(),
            _: &Value,
            _: &mut (),
            _: &mut Budget,
            _: &Range<usize>,
        ) -> Result<(), EvaluationError<QueryError>> {
            unreachable!("preflight refuses emitters")
        }
    }
    impl ProjectionHost<()> for Host {
        type Kind = Kind;
        fn kind(
            &self,
            value: &Value,
            budget: &mut Budget,
            range: &Range<usize>,
        ) -> Result<Kind, QueryError> {
            Ok(match value {
                Value::Subjects(_) => Kind::Subjects,
                Value::Strings(_) => Kind::Strings,
                Value::Count(_) => Kind::Count,
                Value::Constructed(value) => {
                    Kind::Constructed(Arc::new(vec![zirql::constructed_value_type(
                        value,
                        budget,
                        Location::Query(range.clone()),
                    )?]))
                }
            })
        }
        fn scalar_types(
            &self,
            kind: &Kind,
            budget: &mut Budget,
            range: &Range<usize>,
        ) -> Result<Option<Vec<ValueType>>, QueryError> {
            Ok(match kind {
                Kind::Subjects => None,
                Kind::Strings => Some(vec![ValueType::Array(ArrayType::Elements(vec![
                    ValueType::String,
                ]))]),
                Kind::Count => Some(vec![ValueType::Integer]),
                Kind::Constructed(types) => Some(zirql::copy_value_types(
                    types,
                    budget,
                    Location::Query(range.clone()),
                )?),
            })
        }
        fn constructed_kind(&self, types: Vec<ValueType>) -> Kind {
            Kind::Constructed(Arc::new(types))
        }
        fn stage_kind(
            &self,
            stage: &Stage,
            input: &Kind,
            scope: &BTreeMap<String, Kind>,
            budget: &mut Budget,
            range: &Range<usize>,
        ) -> Result<Kind, QueryError> {
            match stage {
                Stage::Saved(name) => Ok(scope[name].clone()),
                Stage::Projection(projection) => Ok(Kind::Constructed(Arc::new(
                    zirql::projection_types::<(), _>(
                        projection,
                        self,
                        input.clone(),
                        scope.clone(),
                        budget,
                    )?,
                ))),
                Stage::Names if *input == Kind::Subjects => Ok(Kind::Strings),
                Stage::Count if matches!(input, Kind::Subjects | Kind::Strings) => Ok(Kind::Count),
                Stage::Fail | Stage::Edit => Ok(input.clone()),
                _ => Err(QueryError::query(
                    range.clone(),
                    "incompatible native stage",
                )),
            }
        }
        fn is_pure(&self, stage: &Stage) -> bool {
            *stage != Stage::Edit
        }
        fn nested_queries(
            &self,
            _: &Stage,
            _: &mut Budget,
            _: &Range<usize>,
        ) -> Result<NativeOperands<(), Self>, QueryError> {
            Ok(vec![])
        }
        fn subjects(
            &self,
            input: Value,
            _: &mut (),
            budget: &mut Budget,
            range: &Range<usize>,
        ) -> Result<Vec<Value>, EvaluationError<QueryError>> {
            let Value::Subjects(rows) = input else {
                return Err(QueryError::query(range.clone(), "projection needs subjects").into());
            };
            budget.charge_results(rows.len(), Location::Query(range.clone()))?;
            Ok(rows
                .into_iter()
                .map(|row| Value::Subjects(vec![row]))
                .collect())
        }
        fn project_value(
            &self,
            value: Value,
            _: &mut (),
            budget: &mut Budget,
            range: &Range<usize>,
        ) -> Result<V, EvaluationError<QueryError>> {
            match value {
                Value::Subjects(_) => Err(QueryError::query(
                    range.clone(),
                    "native handles require an explicit scalar projection",
                )
                .into()),
                Value::Strings(values) => {
                    budget.charge_results(1, Location::Query(range.clone()))?;
                    Ok(V::Array(values.into_iter().map(V::String).collect()))
                }
                Value::Count(count) => Ok(zirql::count_value(
                    count,
                    budget,
                    Location::Query(range.clone()),
                )?),
                Value::Constructed(value) => Ok(value),
            }
        }
        fn lift_constructed(
            &self,
            value: V,
            _: Vec<ValueType>,
            _: &mut (),
            _: &mut Budget,
            _: &Range<usize>,
        ) -> Result<Value, EvaluationError<QueryError>> {
            Ok(Value::Constructed(value))
        }
    }
    impl ValueReaderHost for Host {
        type Stage = Stage;
        type SetOperator = ();
        type Emitter = ();
        type Predicate = ();
        fn is_binding(&self, name: &str) -> bool {
            name == "f"
        }
        fn is_stage(&self, name: &str) -> bool {
            matches!(name, "names" | "count" | "fail" | "edit" | "project")
        }
        fn parse_stage(
            &mut self,
            reader: &mut zirql::ArgumentReader<'_>,
            _: &mut Budget,
        ) -> Result<ExpressionStep<Stage, ()>, QueryError> {
            let word = reader.word()?;
            let stage = match word.value {
                "names" => Stage::Names,
                "count" => Stage::Count,
                "fail" => Stage::Fail,
                "edit" => Stage::Edit,
                _ => return Err(QueryError::query(word.range, "unsupported stage")),
            };
            Ok(ExpressionStep::Stage(stage))
        }
        fn parse_predicate_leaf(
            &mut self,
            reader: &mut zirql::ArgumentReader<'_>,
            _: &mut Budget,
        ) -> Result<(), QueryError> {
            Err(reader.error("predicates unavailable"))
        }
        fn filter(&mut self, _: Predicate<()>, _: Range<usize>) -> Stage {
            unreachable!()
        }
        fn projection(
            &mut self,
            projection: Projection<ReadExpression<Self>>,
            _: Range<usize>,
        ) -> Stage {
            Stage::Projection(Box::new(projection))
        }
        fn set_operator(&self, word: &str) -> Option<()> {
            (word == "union").then_some(())
        }
    }
    struct SavedNames;
    impl ValueReaderHost for SavedNames {
        type Stage = Stage;
        type SetOperator = ();
        type Emitter = ();
        type Predicate = ();
        fn is_binding(&self, name: &str) -> bool {
            matches!(name, "f" | "one" | "project")
        }
        fn is_stage(&self, name: &str) -> bool {
            Host.is_stage(name)
        }
        fn parse_stage(
            &mut self,
            reader: &mut zirql::ArgumentReader<'_>,
            budget: &mut Budget,
        ) -> Result<ExpressionStep<Stage, ()>, QueryError> {
            let mut probe = zirql::ArgumentReader::new(reader.source(), reader.position())?;
            let name = probe.word()?;
            if matches!(name.value, "one" | "project") {
                budget.charge(name.value.len(), Location::Query(name.range.clone()))?;
                reader.advance(probe.position() - reader.position())?;
                Ok(ExpressionStep::Stage(Stage::Saved(name.value.to_owned())))
            } else {
                Host.parse_stage(reader, budget)
            }
        }
        fn parse_predicate_leaf(
            &mut self,
            reader: &mut zirql::ArgumentReader<'_>,
            budget: &mut Budget,
        ) -> Result<(), QueryError> {
            Host.parse_predicate_leaf(reader, budget)
        }
        fn filter(&mut self, predicate: Predicate<()>, range: Range<usize>) -> Stage {
            Host.filter(predicate, range)
        }
        fn projection(
            &mut self,
            projection: Projection<ReadExpression<Self>>,
            range: Range<usize>,
        ) -> Stage {
            Host.projection(projection, range)
        }
        fn set_operator(&self, word: &str) -> Option<()> {
            Host.set_operator(word)
        }
    }
    fn budget() -> Budget {
        Budget::for_program(ProgramLimits {
            work: 100_000,
            results: 10_000,
            nesting: 32,
            stream_items: 100,
        })
    }
    fn parse(source: &str, budget: &mut Budget) -> Projection<Query> {
        zirql::read_projection(
            &mut zirql::ArgumentReader::new(source, 0).unwrap(),
            &mut Host,
            budget,
            32,
        )
        .unwrap()
    }

    #[test]
    fn native_rows_reader_preflight_and_lazy_requirement_share_the_value_runner() {
        let mut allowance = budget();
        let bindings = Bindings::new([
            (
                Located::new("f".into(), 0..0),
                Value::Subjects(vec!["beta"]),
            ),
            (
                Located::new("one".into(), 0..0),
                Value::Strings(vec!["saved".into()]),
            ),
            (
                Located::new("project".into(), 0..0),
                Value::Strings(vec!["legacy".into()]),
            ),
        ])
        .unwrap();
        for (source, expected) in [
            ("f | names | one", V::Array(vec![V::String("saved".into())])),
            (
                "project(w, {\"n\": w | count})",
                V::Array(vec![V::Record(vec![("n".into(), V::Integer(1))])]),
            ),
            ("project", V::Array(vec![V::String("legacy".into())])),
            ("f | project", V::Array(vec![V::String("legacy".into())])),
            ("one(f | names)", V::String("beta".into())),
        ] {
            let mut reader = zirql::ArgumentReader::new(source, 0).unwrap();
            let expression =
                zirql::read_value_expression(&mut reader, &mut SavedNames, &mut allowance, 32)
                    .unwrap();
            assert!(reader.remaining().is_empty());
            let value = zirql::evaluate_native_value(
                &expression,
                &Host,
                Value::Subjects(vec!["beta"]),
                &mut (),
                &bindings,
                &mut allowance,
            )
            .unwrap();
            assert_eq!(value, expected);
        }
        for (source, expected) in [
            ("(f | names) | count", Kind::Count),
            ("f | names union (f | names)", Kind::Strings),
        ] {
            let mut allowance = budget();
            let mut reader = zirql::ArgumentReader::new(source, 0).unwrap();
            let value =
                zirql::read_value_expression(&mut reader, &mut Host, &mut allowance, 32).unwrap();
            assert!(reader.remaining().is_empty());
            assert_eq!(value.range, 0..source.len());
            let K::NativeQuery(query) = value.value else {
                panic!("native operand")
            };
            let scope = BTreeMap::from([("f".to_owned(), Kind::Subjects)]);
            assert!(
                zirql::native_expression_kind::<(), _>(
                    &query,
                    &Host,
                    &Kind::Subjects,
                    &scope,
                    &mut allowance
                )
                .unwrap()
                    == expected
            );
            let bindings = Bindings::new([(
                Located::new("f".into(), 0..0),
                Value::Subjects(vec!["beta"]),
            )])
            .unwrap();
            let value = zirql::evaluate_expression(
                &query,
                &Host,
                Value::Subjects(vec!["beta"]),
                &mut (),
                &bindings,
                &mut allowance,
            )
            .unwrap();
            assert!(matches!(value, Value::Count(1) | Value::Strings(_)));
            zirql::dispose_expression(*query);
        }
        let source = std::iter::repeat_n("f | names", 40)
            .collect::<Vec<_>>()
            .join(" union ");
        let mut allowance = budget();
        let mut reader = zirql::ArgumentReader::new(&source, 0).unwrap();
        let value =
            zirql::read_value_expression(&mut reader, &mut Host, &mut allowance, 3).unwrap();
        let K::NativeQuery(query) = value.value else {
            panic!("native set operand")
        };
        let mut allowance = Budget::for_program(ProgramLimits {
            nesting: 3,
            work: 100_000,
            results: 10_000,
            stream_items: 100,
        });
        let scope = BTreeMap::from([("f".to_owned(), Kind::Subjects)]);
        assert!(
            zirql::native_expression_kind::<(), _>(
                &query,
                &Host,
                &Kind::Subjects,
                &scope,
                &mut allowance
            )
            .unwrap()
                == Kind::Strings
        );
        let bindings = Bindings::new([(
            Located::new("f".into(), 0..0),
            Value::Subjects(vec!["beta"]),
        )])
        .unwrap();
        assert!(
            matches!(zirql::evaluate_expression(&query,&Host,Value::Subjects(vec!["beta"]),&mut (),&bindings,&mut allowance).unwrap(),Value::Strings(values) if values==["beta"])
        );
        zirql::dispose_expression(*query);
        let mut reader = zirql::ArgumentReader::new("(1 + 2) * 3", 0).unwrap();
        let mut allowance = budget();
        let scalar =
            zirql::read_value_expression(&mut reader, &mut Host, &mut allowance, 32).unwrap();
        assert!(reader.remaining().is_empty());
        assert!(matches!(
            scalar.value,
            K::Binary {
                operator: zirql::ValueBinaryOperator::Multiply,
                ..
            }
        ));
        let commented = "# π before\n[\"# β\" # head\n for n in [1]] # end";
        let mut reader = zirql::ArgumentReader::new(commented, 0).unwrap();
        let expression =
            zirql::read_value_expression(&mut reader, &mut Host, &mut allowance, 32).unwrap();
        assert!(reader.remaining().is_empty());
        assert_eq!(
            zirql::evaluate_native_value(
                &expression,
                &Host,
                Value::Subjects(vec![]),
                &mut (),
                &Bindings::default(),
                &mut allowance,
            )
            .unwrap(),
            V::Array(vec![V::String("# β".into())])
        );
        let invalid = "# π before\nmissing";
        let mut reader = zirql::ArgumentReader::new(invalid, 0).unwrap();
        let expression =
            zirql::read_value_expression(&mut reader, &mut Host, &mut allowance, 32).unwrap();
        let error = zirql::native_value_types::<(), _>(
            &expression,
            &Host,
            Kind::Subjects,
            BTreeMap::new(),
            &mut allowance,
        )
        .unwrap_err();
        assert_eq!(
            error.location,
            Location::Query(invalid.find("missing").unwrap()..invalid.len())
        );
        let names = (0..200)
            .map(|index| format!("v{index:04}"))
            .collect::<Vec<_>>();
        let expression = Located::new(
            K::Array(
                names
                    .iter()
                    .map(|name| Located::new(K::Name(name.clone()), 0..1))
                    .collect(),
            ),
            0..1,
        );
        let scope = names.into_iter().map(|name| (name, Kind::Count)).collect();
        let mut low = Budget::for_program(ProgramLimits {
            work: 20_000,
            results: 100_000,
            nesting: 32,
            stream_items: 1_000,
        });
        assert!(
            zirql::native_value_types::<(), _>(&expression, &Host, Kind::Subjects, scope, &mut low)
                .unwrap_err()
                .message
                .contains("work limit")
        );
        let source = r#"project(f, {"name": f | names | one, "n": f | count})"#;
        let mut allowance = budget();
        let projection = parse(source, &mut allowance);
        let result = zirql::evaluate_projection(
            &projection,
            &Host,
            Value::Subjects(vec!["beta", "alpha", "beta"]),
            &mut (),
            &Bindings::default(),
            &mut allowance,
        )
        .unwrap();
        let row = |name: &str| {
            V::Record(vec![
                ("name".into(), V::String(name.into())),
                ("n".into(), V::Integer(1)),
            ])
        };
        assert_eq!(
            result,
            V::Array(vec![row("beta"), row("alpha"), row("beta")])
        );
        let types = zirql::projection_types::<(), _>(
            &projection,
            &Host,
            Kind::Subjects,
            BTreeMap::new(),
            &mut allowance,
        )
        .unwrap();
        assert_eq!(
            types,
            vec![ValueType::Array(ArrayType::Elements(vec![
                ValueType::Record(vec![
                    ("name".into(), ValueType::String),
                    ("n".into(), ValueType::Integer)
                ])
            ]))]
        );
        assert_eq!(
            zirql::evaluate_projection(
                &projection,
                &Host,
                Value::Subjects(vec![]),
                &mut (),
                &Bindings::default(),
                &mut allowance
            )
            .unwrap(),
            V::Array(vec![])
        );
        for invalid in [
            r#"project(f, {"x": missing | names})"#,
            r#"project(f, {"x": sum(f | names)})"#,
            r#"project(f, {"x": f | edit | names})"#,
            r#"project(f, {"x": f})"#,
        ] {
            let projection = parse(invalid, &mut allowance);
            assert!(
                zirql::evaluate_projection(
                    &projection,
                    &Host,
                    Value::Subjects(vec![]),
                    &mut (),
                    &Bindings::default(),
                    &mut allowance
                )
                .is_err()
            );
        }
        let skipped = parse(
            r#"project(f, {"x": require(false, f | fail | names, "stop")})"#,
            &mut allowance,
        );
        let error = zirql::evaluate_projection(
            &skipped,
            &Host,
            Value::Subjects(vec!["alpha"]),
            &mut (),
            &Bindings::default(),
            &mut allowance,
        )
        .unwrap_err();
        assert!(matches!(error,EvaluationError::Core(QueryError{message,..}) if message=="stop"));
        let active = parse(
            r#"project(f, {"x": require(true, f | fail | names, "stop")})"#,
            &mut allowance,
        );
        assert_eq!(
            zirql::evaluate_projection(
                &active,
                &Host,
                Value::Subjects(vec!["alpha"]),
                &mut (),
                &Bindings::default(),
                &mut allowance
            )
            .unwrap_err(),
            EvaluationError::Host(QueryError::source(20..21, "native failure"))
        );
        let alternatives = Projection {
            subject: Located::new("f".into(), 0..1),
            fields: vec![(
                Located::new("choice".into(), 2..3),
                Located::new(
                    K::Conditional {
                        branches: vec![(
                            Located::new(K::Literal(V::Boolean(true)), 3..4),
                            Located::new(K::Literal(V::Integer(1)), 4..5),
                        )],
                        otherwise: Box::new(Located::new(
                            K::Literal(V::String("fallback".into())),
                            5..6,
                        )),
                    },
                    3..6,
                ),
            )],
        };
        let types = zirql::projection_types::<(), _>(
            &alternatives,
            &Host,
            Kind::Subjects,
            BTreeMap::new(),
            &mut allowance,
        )
        .unwrap();
        assert_eq!(
            types,
            vec![ValueType::Array(ArrayType::Elements(vec![
                ValueType::Record(vec![("choice".into(), ValueType::Integer)]),
                ValueType::Record(vec![("choice".into(), ValueType::String)])
            ]))]
        );
        let empty: Vec<String> = vec![];
        assert_eq!(
            zirql::take_one(empty, 4..7).unwrap_err().location,
            Location::Query(4..7)
        );
        assert!(zirql::take_one(vec!["a", "b"], 4..7).is_err());
        assert_eq!(zirql::take_one(vec!["a"], 4..7).unwrap(), "a");
        if usize::BITS > 63 {
            assert!(zirql::count_value(usize::MAX, &mut allowance, Location::Query(8..9)).is_err());
        }
        let projection = parse(source, &mut allowance);
        let mut low = Budget::for_program(ProgramLimits {
            work: 100_000,
            results: 2,
            nesting: 32,
            stream_items: 100,
        });
        assert!(
            zirql::evaluate_projection(
                &projection,
                &Host,
                Value::Subjects(vec!["alpha"]),
                &mut (),
                &Bindings::default(),
                &mut low
            )
            .is_err()
        );
        let mut prefix = zirql::ArgumentReader::new("let label = \"\\u03b2\";", 0).unwrap();
        assert!(matches!(
            zirql::read_statement_prefix(&mut prefix, &mut allowance).unwrap(),
            Some(zirql::CommonStatement::Let(_))
        ));
        let value =
            zirql::read_value_expression(&mut prefix, &mut Host, &mut allowance, 32).unwrap();
        assert!(matches!(value.value,K::Literal(V::String(value)) if value=="β"));
        let pure = Located::new(K::Literal(V::Integer(2)), 0..1);
        assert_eq!(
            zirql::value_expression_types(&pure, &BTreeMap::new(), &mut allowance).unwrap(),
            vec![ValueType::Integer]
        );
        let binding = Located::new(
            ExpressionKind::<Stage, (), ()>::Binding("saved".into()),
            0..1,
        );
        assert!(
            zirql::native_expression_kind::<(), _>(
                &binding,
                &Host,
                &Kind::Subjects,
                &BTreeMap::new(),
                &mut allowance
            )
            .is_err()
        );
    }
}
