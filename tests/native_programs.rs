use std::{ops::Range, rc::Rc};

use zirql::{
    Bindings, Budget, DeclaredType, EmissionPolicy, EvaluationError, Expression, ExpressionKind,
    ExpressionStep, Host, Located, Location, NativeInput, NativeValue, OperationSignature,
    Pipeline, PredicateKind, Program, ProgramHost, ProgramLimits, QueryError, QueryErrorCategory,
    RuntimeTypeHost, Stage, Statement, Truth, checked_invoke, evaluate, evaluate_expression,
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

#[test]
fn parameter_declarations_are_contextual_located_and_require_a_terminator() {
    let source = "  param factor: int; param label: string; param enabled: bool;";
    let mut reader = zirql::ArgumentReader::new(source, 0).unwrap();
    let mut budget = Budget::for_program(limits());
    for (name, ty) in [
        ("factor", zirql::ParameterType::Int),
        ("label", zirql::ParameterType::String),
        ("enabled", zirql::ParameterType::Bool),
    ] {
        let Some(zirql::CommonStatement::Param(declaration)) =
            zirql::read_statement_prefix(&mut reader, &mut budget).unwrap()
        else {
            panic!("expected parameter declaration");
        };
        assert_eq!(declaration.name.value, name);
        assert_eq!(&source[declaration.name.range], name);
        assert_eq!(declaration.ty.value, ty);
        assert!(reader.consume(';'));
    }
    for source in [
        "param = names;",
        "param | count",
        "param union other",
        "param",
        "param;",
    ] {
        let mut reader = zirql::ArgumentReader::new(source, 0).unwrap();
        assert!(
            zirql::read_statement_prefix(&mut reader, &mut budget)
                .unwrap()
                .is_none()
        );
        assert_eq!(reader.position(), 0);
    }
    for source in ["param : int;", "param factor: float;", "param factor: ;"] {
        let mut reader = zirql::ArgumentReader::new(source, 0).unwrap();
        let error = zirql::read_statement_prefix(&mut reader, &mut budget).unwrap_err();
        let Location::Query(range) = error.location else {
            panic!("expected query location");
        };
        assert!(range.start < source.len() || range.start == range.end);
        assert_eq!(reader.position(), 0);
    }
    for source in [
        "param factor: int = 2;",
        "param factor: int extra;",
        "param factor: int",
    ] {
        let mut reader = zirql::ArgumentReader::new(source, 0).unwrap();
        assert!(matches!(
            zirql::read_statement_prefix(&mut reader, &mut budget).unwrap(),
            Some(zirql::CommonStatement::Param(_))
        ));
        assert!(reader.expect(';').is_err());
    }
    let mut reader = zirql::ArgumentReader::new("param label: string;", 0).unwrap();
    let error = zirql::read_statement_prefix(
        &mut reader,
        &mut Budget::for_program(ProgramLimits {
            work: 1,
            ..limits()
        }),
    )
    .unwrap_err();
    assert_eq!(error.location, Location::Query(6..11));
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

#[derive(Clone, Debug, PartialEq, Eq)]
enum SourceKind {
    Words,
}

impl<'a> RuntimeTypeHost<SourceContext<'a>> for SourceHost {
    type Kind = SourceKind;

    fn admit_native(
        &self,
        value: &SourceValue<'a>,
        expected: &DeclaredType<SourceKind>,
        context: &mut SourceContext<'a>,
        budget: &mut Budget,
        location: Location,
    ) -> Result<(), EvaluationError<SourceFailure>> {
        let (DeclaredType::Native(SourceKind::Words), SourceValue::Words(words)) =
            (expected, value)
        else {
            return Err(runtime_refusal("expected borrowed words", budget, location)?.into());
        };
        budget.check_items(words.len(), location.clone())?;
        for word in words {
            budget.charge(1, location.clone())?;
            let valid = context.source.get(word.range.clone()).is_some_and(|text| {
                text.as_ptr() == word.text.as_ptr() && text.len() == word.text.len()
            });
            if !valid {
                return Err(EvaluationError::Host(SourceFailure::Source(
                    word.range.clone(),
                )));
            }
        }
        Ok(())
    }
}

fn runtime_refusal(
    message: &str,
    budget: &mut Budget,
    location: Location,
) -> Result<QueryError, QueryError> {
    budget.charge(message.len(), location.clone())?;
    Ok(QueryError {
        message: message.into(),
        location,
        category: QueryErrorCategory::Other,
    })
}

impl<'a> zirql::ProjectionHost<SourceContext<'a>> for SourceHost {
    type Kind = SourceKind;
    fn kind(
        &self,
        _: &SourceValue<'a>,
        _: &mut Budget,
        _: &Range<usize>,
    ) -> Result<SourceKind, QueryError> {
        Ok(SourceKind::Words)
    }
    fn scalar_types(
        &self,
        _: &SourceKind,
        _: &mut Budget,
        _: &Range<usize>,
    ) -> Result<Option<Vec<zirql::ValueType>>, QueryError> {
        Ok(None)
    }
    fn constructed_kind(&self, _: Vec<zirql::ValueType>) -> SourceKind {
        SourceKind::Words
    }
    fn stage_kind(
        &self,
        _: &SourceStage,
        _: &SourceKind,
        _: &std::collections::BTreeMap<String, SourceKind>,
        _: &mut Budget,
        _: &Range<usize>,
    ) -> Result<SourceKind, QueryError> {
        Ok(SourceKind::Words)
    }
    fn is_pure(&self, _: &SourceStage) -> bool {
        true
    }
    fn nested_queries(
        &self,
        stage: &SourceStage,
        _: &mut Budget,
        _: &Range<usize>,
    ) -> Result<zirql::NativeOperands<SourceContext<'a>, Self>, QueryError> {
        if matches!(stage, SourceStage::Select(_)) {
            return Err(QueryError::query(
                0..1,
                "selectors unavailable in this native-only fixture",
            ));
        }
        Ok(vec![])
    }
    fn subjects(
        &self,
        input: SourceValue<'a>,
        _: &mut SourceContext<'a>,
        _: &mut Budget,
        _: &Range<usize>,
    ) -> Result<Vec<SourceValue<'a>>, EvaluationError<SourceFailure>> {
        Ok(vec![input])
    }
    fn project_value(
        &self,
        _: SourceValue<'a>,
        _: &mut SourceContext<'a>,
        _: &mut Budget,
        r: &Range<usize>,
    ) -> Result<zirql::ConstructedValue, EvaluationError<SourceFailure>> {
        Err(QueryError::query(r.clone(), "native-only fixture").into())
    }
    fn lift_constructed(
        &self,
        _: zirql::ConstructedValue,
        _: Vec<zirql::ValueType>,
        _: &mut SourceContext<'a>,
        _: &mut Budget,
        r: &Range<usize>,
    ) -> Result<SourceValue<'a>, EvaluationError<SourceFailure>> {
        Err(QueryError::query(r.clone(), "native-only fixture").into())
    }
}
impl<'a> zirql::FunctionHost<SourceContext<'a>> for SourceHost {
    fn declaration_kind(
        &self,
        _: &DeclaredType<SourceKind>,
        _: &mut Budget,
        _: &Range<usize>,
    ) -> Result<SourceKind, QueryError> {
        Ok(SourceKind::Words)
    }
    fn is_operation_name(&self, name: &str) -> bool {
        name == "prefix"
    }
}
#[test]
fn functions_keep_borrowed_source_coordinates_and_reject_foreign_context() {
    use zirql::{
        FunctionDeclaration, FunctionLimits, FunctionParameter, FunctionRequest,
        ValueExpressionKind as K,
    };
    let source = String::from("alpha beta");
    let other = String::from("alpha beta");
    let mut context = SourceContext {
        source: &source,
        prefix: "",
        emissions: vec![],
    };
    let functions = [FunctionDeclaration {
        name: at("identity".into(), 1),
        parameters: vec![at(
            FunctionParameter {
                name: at("word".into(), 2),
                declaration: DeclaredType::Native(SourceKind::Words),
            },
            2,
        )],
        result: at(DeclaredType::Native(SourceKind::Words), 3),
        bindings: vec![],
        value: at(K::Name("word".into()), 4),
    }];
    let mut budget = Budget::for_program(ProgramLimits {
        work: 100_000,
        results: 10_000,
        ..limits()
    });
    let bindings =
        Bindings::new([(at("source".into(), 10), source_input(&source, &mut budget))]).unwrap();
    let entry = at(
        K::Call {
            function: at("identity".into(), 11),
            arguments: vec![at(
                K::NativeValue(Box::new(at(ExpressionKind::Binding("source".into()), 12))),
                12,
            )],
        },
        11,
    );
    let result = FunctionRequest::prepare(
        &functions,
        &SourceHost,
        &mut context,
        &mut budget,
        FunctionLimits { active_depth: 1 },
    )
    .unwrap()
    .evaluate(&entry, &bindings)
    .unwrap();
    let SourceValue::Words(words) = result else {
        panic!("native words")
    };
    assert_eq!(words[0].text.as_ptr(), source.as_ptr());
    assert_eq!(words[1].range, 6..10);
    context.source = &other;
    let error = FunctionRequest::prepare(
        &functions,
        &SourceHost,
        &mut context,
        &mut budget,
        FunctionLimits { active_depth: 1 },
    )
    .unwrap()
    .evaluate(&entry, &bindings)
    .unwrap_err();
    assert_eq!(
        error.error,
        EvaluationError::Host(SourceFailure::Source(0..5))
    );
    context.source = &source;
    assert!(
        FunctionRequest::prepare(
            &functions,
            &SourceHost,
            &mut context,
            &mut budget,
            FunctionLimits { active_depth: 1 }
        )
        .unwrap()
        .evaluate(&entry, &bindings)
        .is_ok()
    );
}

#[test]
fn typed_borrowed_calls_keep_source_identity_and_locate_contract_failures() {
    let source = String::from("alpha beta");
    let mut context = SourceContext {
        source: &source,
        prefix: "",
        emissions: Vec::new(),
    };
    let signature = OperationSignature {
        parameters: vec![DeclaredType::Native(SourceKind::Words)],
        result: DeclaredType::Native(SourceKind::Words),
    };
    let mut budget = Budget::for_program(limits());
    let input = source_input(&source, &mut budget);
    let result = checked_invoke(
        &signature,
        vec![at(input, 10)],
        &SourceHost,
        &mut context,
        &mut budget,
        5..20,
        |mut arguments, _, _| Ok(arguments.remove(0).value),
    )
    .unwrap();
    let arguments = [at(result, 10)];
    let copied = checked_invoke(
        &signature,
        &arguments,
        &SourceHost,
        &mut context,
        &mut budget,
        5..20,
        |arguments, context, budget| {
            SourceHost.copy_value(&arguments[0].value, context, budget, &(5..20))
        },
    )
    .unwrap();
    let SourceValue::Words(words) = copied else {
        panic!("native result")
    };
    assert_eq!(words[0].text.as_ptr(), source.as_ptr());
    assert_eq!(words[0].range, 0..5);
    assert_eq!(words[1].text.as_ptr(), source[6..].as_ptr());
    assert_eq!(words[1].range, 6..10);

    let wrong_argument = checked_invoke(
        &signature,
        [at(SourceValue::Count(1), 30)],
        &SourceHost,
        &mut context,
        &mut budget,
        25..40,
        |_, _, _| panic!("incompatible arguments cannot run"),
    )
    .unwrap_err();
    assert_eq!(
        wrong_argument,
        EvaluationError::Core(QueryError::query(30..31, "expected borrowed words"))
    );
    let wrong_result = checked_invoke(
        &signature,
        &arguments,
        &SourceHost,
        &mut context,
        &mut budget,
        45..60,
        |_, _, budget| {
            budget.charge_results(1, Location::Query(45..60))?;
            Ok(SourceValue::Count(1))
        },
    )
    .unwrap_err();
    assert_eq!(
        wrong_result,
        EvaluationError::Core(QueryError::query(45..60, "expected borrowed words"))
    );
    let arity = checked_invoke(
        &signature,
        [],
        &SourceHost,
        &mut context,
        &mut budget,
        65..70,
        |_, _, _| panic!("wrong arity cannot run"),
    )
    .unwrap_err();
    assert_eq!(
        arity,
        EvaluationError::Core(QueryError::query(
            65..70,
            "operation argument count mismatch"
        ))
    );

    let mut continuing = Budget::for_program(limits());
    let input = source_input(&source, &mut continuing);
    let exhausted = checked_invoke(
        &signature,
        vec![at(input, 10)],
        &SourceHost,
        &mut context,
        &mut continuing,
        5..20,
        |mut arguments, _, budget| {
            budget.charge(budget.remaining_work(), Location::Query(12..13))?;
            Ok(arguments.remove(0).value)
        },
    )
    .unwrap_err();
    assert!(
        matches!(exhausted, EvaluationError::Core(QueryError { location: Location::Query(range), category: QueryErrorCategory::WorkLimit, .. }) if range == (5..20))
    );
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
        EvaluationError::Core(
            QueryError::query(20..21, "result limit exceeded")
                .with_category(QueryErrorCategory::ResultLimit)
        )
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

#[derive(Debug, PartialEq, Eq)]
enum DocumentKind {
    Operation,
}

impl RuntimeTypeHost<DocumentContext> for DocumentHost {
    type Kind = DocumentKind;

    fn admit_native(
        &self,
        value: &DocumentValue,
        expected: &DeclaredType<DocumentKind>,
        context: &mut DocumentContext,
        budget: &mut Budget,
        location: Location,
    ) -> Result<(), EvaluationError<QueryError>> {
        let DocumentValue::Handles(ids) = value else {
            return Err(runtime_refusal("expected operation handles", budget, location)?.into());
        };
        let expected = match expected {
            DeclaredType::Optional(element) => element.as_ref(),
            expected => expected,
        };
        match expected {
            DeclaredType::Native(DocumentKind::Operation) if ids.len() == 1 => {}
            DeclaredType::Sequence(element)
                if matches!(
                    element.as_ref(),
                    DeclaredType::Native(DocumentKind::Operation)
                ) => {}
            _ => {
                return Err(
                    runtime_refusal("expected operation handles", budget, location)?.into(),
                );
            }
        }
        budget.check_items(ids.len(), location.clone())?;
        for id in ids {
            budget.charge(1, location.clone())?;
            if context.document.resolve(id).is_none() {
                return Err(EvaluationError::Host(runtime_refusal(
                    "invalid operation owner or index",
                    budget,
                    location,
                )?));
            }
        }
        Ok(())
    }
}

#[test]
fn typed_document_calls_retain_owner_and_native_failures() {
    let document = Rc::new(Document {
        owner: 7,
        operations: vec!["add", "mul", "add"],
    });
    let mut context = DocumentContext {
        document: Rc::clone(&document),
        emissions: Vec::new(),
    };
    let signature = OperationSignature {
        parameters: vec![DeclaredType::Sequence(Box::new(DeclaredType::Native(
            DocumentKind::Operation,
        )))],
        result: DeclaredType::Optional(Box::new(DeclaredType::Sequence(Box::new(
            DeclaredType::Native(DocumentKind::Operation),
        )))),
    };
    let mut budget = Budget::for_program(limits());
    let input = document_input(&mut budget);
    let result = checked_invoke(
        &signature,
        vec![at(input, 10)],
        &DocumentHost,
        &mut context,
        &mut budget,
        5..20,
        |mut arguments, _, _| Ok(arguments.remove(0).value),
    )
    .unwrap();
    let DocumentValue::Handles(ids) = result else {
        panic!("native result")
    };
    assert!(Rc::ptr_eq(&document, &context.document));
    assert_eq!(
        ids,
        [2, 0, 2, 1].map(|index| OperationId { owner: 7, index })
    );
    assert_eq!(document.resolve(&ids[0]), Some("add"));

    budget.charge_results(2, Location::Query(30..31)).unwrap();
    let foreign = DocumentValue::Handles(vec![
        OperationId { owner: 7, index: 0 },
        OperationId { owner: 9, index: 0 },
    ]);
    let owner_error = checked_invoke(
        &signature,
        [at(foreign, 30)],
        &DocumentHost,
        &mut context,
        &mut budget,
        25..40,
        |_, _, _| panic!("foreign handles cannot run"),
    )
    .unwrap_err();
    assert_eq!(
        owner_error,
        EvaluationError::Host(QueryError::query(
            30..31,
            "invalid operation owner or index"
        ))
    );
    let input = document_input(&mut budget);
    let native_error = QueryError::source(4..8, "host read failed");
    let failed = checked_invoke(
        &signature,
        [at(input, 10)],
        &DocumentHost,
        &mut context,
        &mut budget,
        5..20,
        |_, _, _| Err(EvaluationError::Host(native_error.clone())),
    )
    .unwrap_err();
    assert_eq!(failed, EvaluationError::Host(native_error));
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

    let terminal_field = at(
        ExpressionKind::Field {
            value: Box::new(sequence(
                at(ExpressionKind::Input, 0),
                vec![emit("terminal", EmissionPolicy::Terminal, 60)],
            )),
            field: at("item".to_owned(), 61),
        },
        60,
    );
    let input = document_input(&mut budget);
    assert_eq!(
        evaluate_expression(
            &terminal_field,
            &DocumentHost,
            input,
            &mut context,
            &Bindings::default(),
            &mut budget
        )
        .unwrap_err(),
        EvaluationError::Core(QueryError::query(
            61..62,
            "field cannot follow terminal emission"
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
        EvaluationError::Core(
            QueryError::query(20..21, "result limit exceeded")
                .with_category(QueryErrorCategory::ResultLimit)
        )
    );
}

mod projection {
    use std::{collections::BTreeMap, ops::Range, sync::Arc};
    use zirql::{
        ArrayType, Bindings, Budget, ConstructedValue as V, EvaluationError, Expression,
        ExpressionKind, ExpressionStep, Located, Location, MatchHost, NativeCapture, NativeMatch,
        NativeOperands, NativePaths, PathHost, Predicate, ProgramHost, ProgramLimits, Projection,
        ProjectionHost, QueryError, QueryErrorCategory, ReadExpression, Truth,
        ValueExpressionKind as K, ValueReaderHost, ValueType,
    };

    #[derive(Debug, PartialEq)]
    enum Value {
        Subjects(Vec<&'static str>),
        Strings(Vec<String>),
        Count(usize),
        Constructed(V),
        Shared(Arc<V>),
        Capture(NativeCapture<Value>),
        Captures {
            rows: Vec<Value>,
            schema: Arc<NativeCapture<Kind>>,
        },
    }
    #[derive(Clone, Debug, PartialEq)]
    enum Kind {
        Subjects,
        Strings,
        Count,
        Constructed(Arc<Vec<ValueType>>),
        Capture(Arc<NativeCapture<Kind>>),
        Captures(Arc<NativeCapture<Kind>>),
    }
    #[derive(Debug, PartialEq)]
    enum Stage {
        Names,
        Count,
        NativeArgument(Arc<zirql::ValueExpression<Query>>),
        Argument {
            value: Box<zirql::ValueExpression<Query>>,
            selected: bool,
        },
        Fail,
        Edit,
        Saved(String),
        Projection(Box<Projection<Query>>),
        Match(Box<NativeMatch<Query, Truth>>),
        Paths(Box<NativePaths<Stage, Truth>>),
        Neighbors,
    }
    struct Host;

    impl zirql::RuntimeTypeHost<()> for Host {
        type Kind = Kind;

        fn constructed_value<'v>(&self, value: &'v Value) -> Option<&'v V> {
            match value {
                Value::Constructed(value) => Some(value),
                Value::Shared(value) => Some(value),
                _ => None,
            }
        }

        fn admit_native(
            &self,
            value: &Value,
            expected: &zirql::DeclaredType<Kind>,
            _: &mut (),
            budget: &mut Budget,
            location: Location,
        ) -> Result<(), EvaluationError<QueryError>> {
            let expected = match expected {
                zirql::DeclaredType::Sequence(element) => element.as_ref(),
                other => other,
            };
            let (zirql::DeclaredType::Native(Kind::Subjects), Value::Subjects(rows)) =
                (expected, value)
            else {
                return Err(super::runtime_refusal("expected subjects", budget, location)?.into());
            };
            budget.check_items(rows.len(), location.clone())?;
            budget.charge(rows.len(), location)?;
            Ok(())
        }
    }
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
                Value::Shared(value) => {
                    budget.charge(1, location)?;
                    Value::Shared(Arc::clone(value))
                }
                Value::Constructed(value) => {
                    Value::Constructed(zirql::copy_constructed_value(value, budget, location)?)
                }
                Value::Capture(record) => {
                    budget.charge_results(record.fields.len() + 1, location.clone())?;
                    budget.charge(record.fields.len(), location.clone())?;
                    let mut fields = Vec::with_capacity(record.fields.len());
                    for (name, value) in &record.fields {
                        budget.charge(name.len(), location.clone())?;
                        fields.push((
                            name.clone(),
                            self.copy_value(value, &mut (), budget, range)?,
                        ));
                    }
                    Value::Capture(NativeCapture { fields })
                }
                Value::Captures { rows, schema } => {
                    budget.charge_results(rows.len() + 1, location.clone())?;
                    budget.charge(rows.len(), location)?;
                    Value::Captures {
                        rows: rows
                            .iter()
                            .map(|row| self.copy_value(row, &mut (), budget, range))
                            .collect::<Result<_, _>>()?,
                        schema: Arc::clone(schema),
                    }
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
                Stage::Argument { .. } => unreachable!("argument stage uses scoped preparation"),
                Stage::Saved(name) => {
                    self.copy_value(bindings.get(name, range.clone())?, context, budget, range)
                }
                Stage::Projection(projection) => {
                    zirql::evaluate_projection(projection, self, value, context, bindings, budget)
                        .map(Value::Constructed)
                }
                Stage::Match(matching) => {
                    let input_kind = self.kind(&value, budget, range)?;
                    let mut scope = BTreeMap::new();
                    for (name, value) in bindings.iter() {
                        budget.charge(name.len() + 1, Location::Query(range.clone()))?;
                        scope.insert(name.to_owned(), self.kind(value, budget, range)?);
                    }
                    let schema = Arc::new(zirql::match_types(
                        matching,
                        self,
                        &input_kind,
                        &scope,
                        budget,
                    )?);
                    let rows =
                        zirql::evaluate_match(matching, self, value, context, bindings, budget)?;
                    budget.charge_results(1, Location::Query(range.clone()))?;
                    Ok(Value::Captures { rows, schema })
                }
                Stage::Paths(paths) => {
                    let input_kind = self.kind(&value, budget, range)?;
                    let schema = Arc::new(zirql::path_types(
                        paths,
                        self,
                        &input_kind,
                        &BTreeMap::new(),
                        budget,
                    )?);
                    let rows =
                        zirql::evaluate_paths(paths, self, value, context, bindings, budget)?;
                    budget.charge_results(1, Location::Query(range.clone()))?;
                    Ok(Value::Captures { rows, schema })
                }
                Stage::NativeArgument(_) => unreachable!("prepared argument runner"),
                Stage::Neighbors => unreachable!("registered edge only"),
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
            fields: Vec<(String, Value)>,
            _: &mut (),
            _: &mut Budget,
            _: &Range<usize>,
        ) -> Result<Value, EvaluationError<QueryError>> {
            Ok(Value::Capture(NativeCapture { fields }))
        }
        fn capture_record<'v>(&self, value: &'v Value) -> Option<&'v NativeCapture<Value>> {
            match value {
                Value::Capture(record) => Some(record),
                _ => None,
            }
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
        fn value_arguments<'s>(
            &self,
            stage: &'s Stage,
            budget: &mut Budget,
            range: &Range<usize>,
        ) -> Result<zirql::NativeValueArguments<'s, (), Self>, QueryError> {
            if let Stage::NativeArgument(value) = stage {
                budget.charge(1, Location::Query(range.clone()))?;
                Ok(vec![(value.as_ref(), Kind::Subjects)])
            } else if let Stage::Argument { value, .. } = stage {
                budget.charge(1, Location::Query(range.clone()))?;
                Ok(vec![(value.as_ref(), Kind::Subjects)])
            } else {
                Ok(vec![])
            }
        }
        fn evaluate_stage_with_arguments(
            &self,
            stage: &Stage,
            input: Value,
            arguments: &mut dyn zirql::NativeArguments<(), Self>,
            budget: &mut Budget,
            range: &Range<usize>,
        ) -> Result<Value, EvaluationError<QueryError>> {
            if let Stage::NativeArgument(value) = stage {
                let read = arguments.evaluate_native(value, Value::Subjects(vec![]), budget)?;
                if matches!(value.value, K::Call { .. }) {
                    assert!(read.declared_type().is_some());
                }
                let owner = read.into_shared();
                let (context, _) = arguments.parts();
                return self.copy_value(&owner, context, budget, range);
            }
            if let Stage::Argument { value, selected } = stage {
                if !selected {
                    return Ok(Value::Count(0));
                }
                let mut total = 0;
                for _ in 0..2 {
                    let read = arguments.evaluate(value, Value::Subjects(vec![]), budget)?;
                    if matches!(value.value, K::Call { .. }) {
                        let declaration = read.declared_type().ok_or_else(|| {
                            QueryError::query(
                                range.clone(),
                                "function argument needs its result contract",
                            )
                        })?;
                        zirql::admit_constructed(
                            read.value(),
                            declaration,
                            budget,
                            Location::Query(range.clone()),
                        )?;
                    }
                    total += match read.value().data() {
                        V::String(value) => value.len(),
                        V::Array(values)
                            if values.iter().all(|value| matches!(value, V::Integer(_))) =>
                        {
                            values
                                .iter()
                                .map(|value| {
                                    let V::Integer(value) = value else {
                                        unreachable!()
                                    };
                                    usize::try_from(*value).unwrap()
                                })
                                .sum::<usize>()
                        }
                        V::Array(values) => values.len(),
                        _ => {
                            return Err(QueryError::query(
                                range.clone(),
                                "argument needs string or array",
                            )
                            .into());
                        }
                    };
                }
                return Ok(Value::Count(total));
            }
            let (context, bindings) = arguments.parts();
            self.evaluate_stage(stage, input, context, bindings, budget, range)
        }
        fn share_constructed(
            &self,
            value: &Value,
            _: &mut (),
            budget: &mut Budget,
            range: &Range<usize>,
        ) -> Result<Option<Arc<V>>, EvaluationError<QueryError>> {
            budget.charge(1, Location::Query(range.clone()))?;
            Ok(if let Value::Shared(value) = value {
                Some(Arc::clone(value))
            } else {
                None
            })
        }
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
                Value::Capture(record) => {
                    budget.charge(record.fields.len(), Location::Query(range.clone()))?;
                    let mut fields = Vec::with_capacity(record.fields.len());
                    for (name, value) in &record.fields {
                        budget.charge(name.len(), Location::Query(range.clone()))?;
                        fields.push((name.clone(), self.kind(value, budget, range)?));
                    }
                    Kind::Capture(Arc::new(NativeCapture { fields }))
                }
                Value::Captures { schema, .. } => Kind::Captures(Arc::clone(schema)),
                Value::Shared(value) => {
                    Kind::Constructed(Arc::new(vec![zirql::constructed_value_type(
                        value,
                        budget,
                        Location::Query(range.clone()),
                    )?]))
                }
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
                Kind::Subjects | Kind::Capture(_) | Kind::Captures(_) => None,
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
                Stage::Match(matching) => Ok(Kind::Captures(Arc::new(zirql::match_types(
                    matching, self, input, scope, budget,
                )?))),
                Stage::Paths(paths) => Ok(Kind::Captures(Arc::new(zirql::path_types(
                    paths, self, input, scope, budget,
                )?))),
                Stage::Projection(projection) => Ok(Kind::Constructed(Arc::new(
                    zirql::projection_types::<(), _>(
                        projection,
                        self,
                        input.clone(),
                        scope.clone(),
                        budget,
                    )?,
                ))),
                Stage::NativeArgument(_) => Ok(Kind::Subjects),
                Stage::Argument { .. } => Ok(Kind::Count),
                Stage::Names if *input == Kind::Subjects => Ok(Kind::Strings),
                Stage::Count if matches!(input, Kind::Subjects | Kind::Strings) => Ok(Kind::Count),
                Stage::Fail | Stage::Edit => Ok(input.clone()),
                _ => Err(QueryError::query(
                    range.clone(),
                    "incompatible native stage",
                )),
            }
        }
        fn is_explicit_source(&self, stage: &Stage) -> bool {
            matches!(stage, Stage::NativeArgument(_))
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
            let rows = match input {
                Value::Captures { rows, .. } => return Ok(rows),
                Value::Capture(record) => {
                    budget.charge_results(1, Location::Query(range.clone()))?;
                    return Ok(vec![Value::Capture(record)]);
                }
                Value::Subjects(rows) => rows,
                _ => {
                    return Err(
                        QueryError::query(range.clone(), "projection needs subjects").into(),
                    );
                }
            };
            budget.charge_results(rows.len(), Location::Query(range.clone()))?;
            Ok(rows
                .into_iter()
                .map(|row| Value::Subjects(vec![row]))
                .collect())
        }
        fn subject_kind(
            &self,
            input: &Kind,
            _: &mut Budget,
            range: &Range<usize>,
        ) -> Result<Kind, QueryError> {
            match input {
                Kind::Subjects => Ok(Kind::Subjects),
                Kind::Capture(schema) | Kind::Captures(schema) => {
                    Ok(Kind::Capture(Arc::clone(schema)))
                }
                _ => Err(QueryError::query(
                    range.clone(),
                    "projection needs subjects",
                )),
            }
        }
        fn capture_record_kind<'k>(&self, kind: &'k Kind) -> Option<&'k NativeCapture<Kind>> {
            match kind {
                Kind::Capture(schema) => Some(schema),
                _ => None,
            }
        }
        fn project_value(
            &self,
            value: Value,
            _: &mut (),
            budget: &mut Budget,
            range: &Range<usize>,
        ) -> Result<V, EvaluationError<QueryError>> {
            match value {
                Value::Subjects(_) | Value::Capture(_) | Value::Captures { .. } => {
                    Err(QueryError::query(
                        range.clone(),
                        "native handles require an explicit scalar projection",
                    )
                    .into())
                }
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
                Value::Shared(value) => Ok(zirql::copy_constructed_value(
                    &value,
                    budget,
                    Location::Query(range.clone()),
                )?),
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
    impl zirql::FunctionHost<()> for Host {
        fn declaration_kind(
            &self,
            d: &zirql::DeclaredType<Kind>,
            _budget: &mut Budget,
            _range: &Range<usize>,
        ) -> Result<Kind, QueryError> {
            Ok(match d {
                zirql::DeclaredType::Native(kind) => kind.clone(),
                zirql::DeclaredType::Sequence(element)
                    if matches!(
                        element.as_ref(),
                        zirql::DeclaredType::Native(Kind::Subjects)
                    ) =>
                {
                    Kind::Subjects
                }
                _ => self.constructed_kind(vec![ValueType::Unestablished]),
            })
        }
        fn is_operation_name(&self, name: &str) -> bool {
            matches!(name, "names" | "count" | "edit")
        }
    }
    fn function_call(
        name: &str,
        args: Vec<zirql::ValueExpression<Query>>,
    ) -> zirql::ValueExpression<Query> {
        Located::new(
            K::Call {
                function: Located::new(name.into(), 30..34),
                arguments: args,
            },
            30..40,
        )
    }
    fn function_value(value: K<Query>) -> zirql::ValueExpression<Query> {
        Located::new(value, 40..50)
    }
    fn native_operand(stage: Stage) -> Query {
        super::sequence(
            super::at(ExpressionKind::Binding("xs".into()), 20),
            vec![super::at(ExpressionStep::Stage(stage), 21)],
        )
    }
    fn native_function(
        name: &str,
        value: zirql::ValueExpression<Query>,
    ) -> zirql::FunctionDeclaration<Query, Kind> {
        zirql::FunctionDeclaration {
            name: super::at(name.into(), 1),
            parameters: vec![
                super::at(
                    zirql::FunctionParameter {
                        name: super::at("n".into(), 2),
                        declaration: zirql::DeclaredType::Integer,
                    },
                    2,
                ),
                super::at(
                    zirql::FunctionParameter {
                        name: super::at("xs".into(), 3),
                        declaration: zirql::DeclaredType::Native(Kind::Subjects),
                    },
                    3,
                ),
            ],
            result: super::at(zirql::DeclaredType::Native(Kind::Subjects), 4),
            bindings: vec![],
            value,
        }
    }
    fn fn_name(name: &str) -> zirql::ValueExpression<Query> {
        function_value(K::Name(name.into()))
    }
    fn fn_int(value: i64) -> zirql::ValueExpression<Query> {
        function_value(K::Literal(V::Integer(value)))
    }
    fn fn_binary(
        op: zirql::ValueBinaryOperator,
        left: zirql::ValueExpression<Query>,
        right: zirql::ValueExpression<Query>,
    ) -> zirql::ValueExpression<Query> {
        function_value(K::Binary {
            operator: op,
            left: Box::new(left),
            right: Box::new(right),
        })
    }
    fn function_budget() -> Budget {
        Budget::for_program(ProgramLimits {
            work: 5_000_000,
            results: 500_000,
            nesting: 64,
            stream_items: 100,
        })
    }
    #[test]
    fn named_native_functions_keep_arguments_results_projections_and_callback_limits() {
        use zirql::{DeclaredType as D, FunctionLimits, FunctionRequest, ValueBinaryOperator as B};
        let body = function_value(K::Conditional {
            branches: vec![(fn_binary(B::Equal, fn_name("n"), fn_int(0)), fn_name("xs"))],
            otherwise: Box::new(function_call(
                "walk",
                vec![
                    fn_binary(B::Subtract, fn_name("n"), fn_int(1)),
                    fn_name("xs"),
                ],
            )),
        });
        let mut walk = native_function("walk", body);
        walk.bindings.push((
            super::at("chosen".into(), 9),
            function_value(K::Conditional {
                branches: vec![(function_value(K::Literal(V::Boolean(true))), fn_name("xs"))],
                otherwise: Box::new(fn_name("xs")),
            }),
        ));
        walk.bindings.push((
            super::at("size".into(), 10),
            function_value(K::NativeQuery(Box::new(super::sequence(
                super::at(ExpressionKind::Binding("chosen".into()), 20),
                vec![super::at(ExpressionStep::Stage(Stage::Count), 21)],
            )))),
        ));
        let functions = [walk];
        let bindings = Bindings::new([(
            super::at("input".into(), 60),
            Value::Subjects(vec!["alpha"]),
        )])
        .unwrap();
        let mut budget = function_budget();
        let entry = function_call(
            "walk",
            vec![
                fn_int(1000),
                function_value(K::NativeValue(Box::new(super::at(
                    ExpressionKind::Binding("input".into()),
                    61,
                )))),
            ],
        );
        let result = FunctionRequest::prepare(
            &functions,
            &Host,
            &mut (),
            &mut budget,
            FunctionLimits { active_depth: 1 },
        )
        .unwrap()
        .evaluate(&entry, &bindings)
        .unwrap();
        assert!(matches!(result,Value::Subjects(rows) if rows==vec!["alpha"]));
        let mut items = native_function("items", function_value(K::Array(vec![fn_int(1)])));
        items.parameters.clear();
        items.result = super::at(D::Sequence(Box::new(D::Integer)), 4);
        let mut apply = native_function(
            "apply",
            function_value(K::NativeQuery(Box::new(native_operand(Stage::Argument {
                value: Box::new(function_call("items", vec![])),
                selected: true,
            })))),
        );
        apply.result = super::at(D::Integer, 4);
        let functions = [items, apply];
        let applied = function_call(
            "apply",
            vec![
                fn_int(0),
                function_value(K::NativeValue(Box::new(super::at(
                    ExpressionKind::Binding("input".into()),
                    61,
                )))),
            ],
        );
        let result = FunctionRequest::prepare(
            &functions,
            &Host,
            &mut (),
            &mut budget,
            FunctionLimits { active_depth: 2 },
        )
        .unwrap()
        .evaluate(&applied, &bindings)
        .unwrap();
        assert!(matches!(result,Value::Constructed(value) if value.data()==&V::Integer(2)));
        for saved in [true, false] {
            let mut missing = native_function("missing", function_value(K::Literal(V::Absent)));
            missing.parameters.clear();
            missing.result = super::at(D::Optional(Box::new(D::Integer)), 4);
            let fallback = function_value(K::Default {
                value: Box::new(fn_name(if saved { "saved" } else { "n" })),
                fallback: Box::new(function_value(K::Literal(V::String("bad".into())))),
            });
            let mut apply = native_function(
                "apply",
                function_value(K::NativeQuery(Box::new(native_operand(Stage::Argument {
                    value: Box::new(fallback),
                    selected: true,
                })))),
            );
            apply.result = super::at(D::Integer, 4);
            if saved {
                apply.bindings.push((
                    super::at("saved".into(), 8),
                    function_call("missing", vec![]),
                ));
            } else {
                apply.parameters[0].value.declaration = D::Optional(Box::new(D::Integer));
            }
            let functions = [missing, apply];
            let entry = function_call(
                "apply",
                vec![
                    if saved {
                        fn_int(0)
                    } else {
                        function_call("missing", vec![])
                    },
                    function_value(K::NativeValue(Box::new(super::at(
                        ExpressionKind::Binding("input".into()),
                        61,
                    )))),
                ],
            );
            let error = FunctionRequest::prepare(
                &functions,
                &Host,
                &mut (),
                &mut budget,
                FunctionLimits { active_depth: 2 },
            )
            .unwrap()
            .evaluate(&entry, &bindings)
            .unwrap_err();
            assert!(matches!(error.error, EvaluationError::Core(ref e)
                if e.message.contains("integer") && e.location == Location::Query(40..50)));
        }
        let mut failure = native_function(
            "failure",
            function_value(K::NativeValue(Box::new(native_operand(Stage::Fail)))),
        );
        failure.result = super::at(D::Native(Kind::Subjects), 4);
        let functions = [failure];
        let entry = function_call(
            "failure",
            vec![
                fn_int(0),
                function_value(K::NativeValue(Box::new(super::at(
                    ExpressionKind::Binding("input".into()),
                    61,
                )))),
            ],
        );
        let error = FunctionRequest::prepare(
            &functions,
            &Host,
            &mut (),
            &mut budget,
            FunctionLimits { active_depth: 2 },
        )
        .unwrap()
        .evaluate(&entry, &bindings)
        .unwrap_err();
        assert_eq!(
            error.error,
            EvaluationError::Host(QueryError::source(20..21, "native failure"))
        );
        assert_eq!(error.calls.len(), 1);
        assert_eq!(
            zirql::evaluate_value(
                &super::at(K::Literal(V::Integer(3)), 1),
                &Bindings::default(),
                &mut budget
            )
            .unwrap(),
            V::Integer(3)
        );
        let recursive = function_call("reenter", vec![fn_name("n"), fn_name("xs")]);
        let mut reenter = native_function(
            "reenter",
            function_value(K::NativeQuery(Box::new(native_operand(Stage::Argument {
                value: Box::new(recursive),
                selected: true,
            })))),
        );
        reenter.result = super::at(D::Integer, 4);
        let functions = [reenter];
        let entry = function_call(
            "reenter",
            vec![
                fn_int(1),
                function_value(K::NativeValue(Box::new(super::at(
                    ExpressionKind::Binding("input".into()),
                    61,
                )))),
            ],
        );
        let error = FunctionRequest::prepare(
            &functions,
            &Host,
            &mut (),
            &mut budget,
            FunctionLimits { active_depth: 1000 },
        )
        .unwrap()
        .evaluate(&entry, &bindings)
        .unwrap_err();
        assert!(
            matches!(error.error,EvaluationError::Core(ref e) if e.message.contains("evaluation call limit")||e.message.contains("nesting limit"))
        );
        assert!(error.calls.len() <= 16);
        assert_eq!(
            zirql::evaluate_value(
                &super::at(K::Literal(V::Integer(4)), 1),
                &Bindings::default(),
                &mut budget
            )
            .unwrap(),
            V::Integer(4)
        );
    }
    #[test]
    fn function_native_preflight_covers_unused_effects_emitters_and_argument_bodies() {
        use zirql::{FunctionLimits, FunctionRequest};
        for query in [
            native_operand(Stage::Edit),
            super::sequence(
                super::at(ExpressionKind::Binding("xs".into()), 20),
                vec![super::at(
                    ExpressionStep::Emit {
                        emitter: (),
                        policy: zirql::EmissionPolicy::PassThrough,
                    },
                    22,
                )],
            ),
            native_operand(Stage::Argument {
                value: Box::new(function_call("missing", vec![])),
                selected: false,
            }),
        ] {
            let functions = [native_function(
                "unused",
                function_value(K::NativeValue(Box::new(query))),
            )];
            let mut budget = function_budget();
            assert!(
                FunctionRequest::prepare(
                    &functions,
                    &Host,
                    &mut (),
                    &mut budget,
                    FunctionLimits { active_depth: 2 }
                )
                .is_err()
            );
        }
        let functions = [native_function("names", fn_name("xs"))];
        let mut budget = function_budget();
        assert!(
            FunctionRequest::prepare(
                &functions,
                &Host,
                &mut (),
                &mut budget,
                FunctionLimits { active_depth: 2 }
            )
            .is_err()
        );
    }
    #[test]
    fn parsed_functions_share_preparation_native_arguments_and_generator_kinds() {
        use zirql::{DeclaredType as D, FunctionLimits, PreparedFunctions};
        let source = r#"
            fn odd(n: int) -> bool { let next = n - 1; if n == 0 { false } else { even(next) } }
            fn even(n: int) -> bool { if n == 0 { true } else if n == 1 { false } else { odd(n - 1) } }
            fn identity(xs: Subjects) -> Subjects { let alias = xs; alias }
            fn counts(xs: sequence<Subjects>) -> int { sum([identity(item) | count for item in xs]) }
            fn parameter(if: int) -> int { let saved = if; if saved == 0 { if } else { saved } }
            fn local() -> int { let if = 3; if if == 3 { if } else { 0 } }
            fn forwarded(if: Subjects) -> int { native(if true { if } else { if }) | count }
            fn forwarded_local(xs: Subjects) -> int { let if = xs; native(if) | count }
            counts(source)
        "#;
        let mut budget = function_budget();
        let mut reader = zirql::ArgumentReader::new(source, 0).unwrap();
        let mut declarations = Vec::new();
        while let Some(function) =
            zirql::read_function_declaration(&mut reader, &mut Host, &mut budget, 64, |name, _| {
                match name.value {
                    "Subjects" => Ok(D::Native(Kind::Subjects)),
                    _ => Err(QueryError::query(name.range, "unknown signature type")),
                }
            })
            .unwrap()
        {
            declarations.push(function);
        }
        let entry = zirql::read_value_expression(&mut reader, &mut Host, &mut budget, 64).unwrap();
        let mut functions = PreparedFunctions::prepare(
            declarations[..2].iter().chain(&declarations[2..]),
            &Host,
            &mut budget,
        )
        .unwrap();
        let scope = BTreeMap::from([("source".into(), Kind::Subjects)]);
        let entry = functions
            .prepare_value(&entry, &scope, None, &mut budget)
            .unwrap();
        let limits = FunctionLimits { active_depth: 2 };
        let bindings = Bindings::new([(
            super::at("source".into(), 0),
            Value::Subjects(vec!["alpha", "beta", "gamma"]),
        )])
        .unwrap();
        assert_eq!(
            functions
                .evaluate(&entry, &bindings, None, &mut (), &mut budget, limits)
                .unwrap(),
            Value::Constructed(V::Declared {
                value: Box::new(V::Integer(3)),
                declaration: D::Integer
            })
        );
        let empty =
            Bindings::new([(super::at("source".into(), 0), Value::Subjects(vec![]))]).unwrap();
        assert_eq!(
            functions
                .evaluate(&entry, &empty, None, &mut (), &mut budget, limits)
                .unwrap(),
            Value::Constructed(V::Declared {
                value: Box::new(V::Integer(0)),
                declaration: D::Integer
            })
        );
        let mut reader = zirql::ArgumentReader::new("even(200)", 0).unwrap();
        let tail = zirql::read_value_expression(&mut reader, &mut Host, &mut budget, 64).unwrap();
        let tail = functions
            .prepare_value(&tail, &BTreeMap::new(), None, &mut budget)
            .unwrap();
        assert_eq!(
            functions
                .evaluate(
                    &tail,
                    &Bindings::default(),
                    None,
                    &mut (),
                    &mut budget,
                    FunctionLimits { active_depth: 1 }
                )
                .unwrap(),
            Value::Constructed(V::Declared {
                value: Box::new(V::Boolean(true)),
                declaration: D::Boolean
            })
        );
        let mut reader = zirql::ArgumentReader::new("parameter(0) + local()", 0).unwrap();
        let named_if =
            zirql::read_value_expression(&mut reader, &mut Host, &mut budget, 64).unwrap();
        let named_if = functions
            .prepare_value(&named_if, &BTreeMap::new(), None, &mut budget)
            .unwrap();
        assert_eq!(
            functions
                .evaluate(
                    &named_if,
                    &Bindings::default(),
                    None,
                    &mut (),
                    &mut budget,
                    limits
                )
                .unwrap(),
            Value::Constructed(V::Integer(3))
        );
        let mut reader =
            zirql::ArgumentReader::new("forwarded(source) + forwarded_local(source)", 0).unwrap();
        let forwarded =
            zirql::read_value_expression(&mut reader, &mut Host, &mut budget, 64).unwrap();
        let forwarded = functions
            .prepare_value(&forwarded, &scope, None, &mut budget)
            .unwrap();
        assert_eq!(
            functions
                .evaluate(&forwarded, &bindings, None, &mut (), &mut budget, limits)
                .unwrap(),
            Value::Constructed(V::Integer(6))
        );
        let mut reader=zirql::ArgumentReader::new("fn guarded(if: int, xs: Subjects) -> int { xs | paths(neighbors, to=bound(if), max_depth=0) }",0).unwrap();
        let guarded =
            zirql::read_function_declaration(&mut reader, &mut Host, &mut budget, 64, |name, _| {
                Ok::<_, QueryError>(D::Native(match name.value {
                    "Subjects" => Kind::Subjects,
                    _ => unreachable!(),
                }))
            })
            .unwrap()
            .unwrap();
        let K::NativeQuery(query) = guarded.value.value else {
            panic!("native guarded query")
        };
        let ExpressionKind::Sequence { steps, .. } = query.value else {
            panic!("guarded native sequence")
        };
        let ExpressionStep::Stage(Stage::Paths(paths)) = &steps[0].value else {
            panic!("guarded paths")
        };
        assert_eq!(paths.target.value, zirql::PredicateKind::Leaf(Truth::True));
        // A typed caller may share the same source tree in two lexical scopes.
        let mut reader = zirql::ArgumentReader::new("identity(source) | count", 0).unwrap();
        let pipeline =
            zirql::read_value_expression(&mut reader, &mut Host, &mut budget, 64).unwrap();
        let first = functions
            .prepare_value(&pipeline, &scope, None, &mut budget)
            .unwrap();
        let mut expanded = scope.clone();
        expanded.insert("unused".into(), Kind::Subjects);
        let _second = functions
            .prepare_value(&pipeline, &expanded, None, &mut budget)
            .unwrap();
        assert_eq!(
            functions
                .evaluate(&first, &bindings, None, &mut (), &mut budget, limits)
                .unwrap(),
            Value::Constructed(V::Integer(3))
        );
        let other = PreparedFunctions::prepare(&declarations, &Host, &mut budget).unwrap();
        assert!(
            other
                .evaluate(&entry, &empty, None, &mut (), &mut budget, limits)
                .unwrap_err()
                .to_string()
                .contains("different prepared")
        );
    }

    #[test]
    fn shared_native_argument_trees_keep_each_function_lexical_kind() {
        use zirql::{
            DeclaredType as D, FunctionDeclaration, FunctionLimits, FunctionParameter,
            PreparedFunctions,
        };
        let shared = Arc::new(function_value(K::NativeValue(Box::new(super::at(
            ExpressionKind::Binding("xs".into()),
            0,
        )))));
        let declaration = |name: &str, ty: D<Kind>| FunctionDeclaration {
            name: super::at(name.into(), 0),
            parameters: vec![super::at(
                FunctionParameter {
                    name: super::at("xs".into(), 0),
                    declaration: ty.clone(),
                },
                0,
            )],
            result: super::at(ty, 0),
            bindings: vec![],
            value: function_value(K::NativeValue(Box::new(super::sequence(
                super::at(ExpressionKind::Input, 0),
                vec![super::at(
                    ExpressionStep::Stage(Stage::NativeArgument(Arc::clone(&shared))),
                    0,
                )],
            )))),
        };
        let declarations = [
            declaration("native", D::Native(Kind::Subjects)),
            declaration("scalar", D::String),
        ];
        let mut budget = function_budget();
        let mut functions = PreparedFunctions::prepare(&declarations, &Host, &mut budget).unwrap();
        let native = function_call("native", vec![fn_name("source")]);
        let scalar = function_call(
            "scalar",
            vec![function_value(K::Literal(V::String("alpha".into())))],
        );
        let native = functions
            .prepare_value(
                &native,
                &BTreeMap::from([("source".into(), Kind::Subjects)]),
                None,
                &mut budget,
            )
            .unwrap();
        let scalar = functions
            .prepare_value(&scalar, &BTreeMap::new(), None, &mut budget)
            .unwrap();
        let bindings = Bindings::new([(
            super::at("source".into(), 0),
            Value::Subjects(vec!["alpha"]),
        )])
        .unwrap();
        let limits = FunctionLimits { active_depth: 1 };
        assert_eq!(
            functions
                .evaluate(&native, &bindings, None, &mut (), &mut budget, limits)
                .unwrap(),
            Value::Subjects(vec!["alpha"])
        );
        let result = functions
            .evaluate(
                &scalar,
                &Bindings::default(),
                None,
                &mut (),
                &mut budget,
                limits,
            )
            .unwrap();
        let Value::Constructed(result) = result else {
            panic!("expected string")
        };
        assert_eq!(result.data(), &V::String("alpha".into()));
    }

    #[test]
    fn signature_reader_keeps_complete_types_and_located_refusals() {
        use zirql::{DeclaredType as D, SetElementType as S};
        let source = "fn types(x: opt record {field: tuple<int, sequence<Subjects>>, flags: set<bool>, cells: keyed<float>, label: string}) -> int { 0 }";
        let mut budget = function_budget();
        let mut reader = zirql::ArgumentReader::new(source, 0).unwrap();
        let declaration =
            zirql::read_function_declaration(&mut reader, &mut Host, &mut budget, 64, |name, _| {
                if name.value == "Subjects" {
                    Ok(D::Native(Kind::Subjects))
                } else {
                    Err(QueryError::query(name.range, "unknown signature type"))
                }
            })
            .unwrap()
            .unwrap();
        assert_eq!(
            declaration.parameters[0].value.declaration,
            D::Optional(Box::new(D::Record(vec![
                (
                    "field".into(),
                    D::Tuple(vec![
                        D::Integer,
                        D::Sequence(Box::new(D::Native(Kind::Subjects)))
                    ])
                ),
                ("flags".into(), D::Set(S::Boolean)),
                ("cells".into(), D::Keyed(Box::new(D::Float))),
                ("label".into(), D::String),
            ])))
        );
        for (source, expected) in [
            ("fn f(x: Missing) -> int { 0 }", "unknown signature type"),
            (
                "fn f(x: set<string>) -> int { 0 }",
                "unsupported set element",
            ),
            ("fn f() -> int { if true { 1 } }", "final else"),
        ] {
            let mut reader = zirql::ArgumentReader::new(source, 0).unwrap();
            let error = zirql::read_function_declaration(
                &mut reader,
                &mut Host,
                &mut budget,
                64,
                |name, _| {
                    Err::<D<Kind>, _>(QueryError::query(name.range, "unknown signature type"))
                },
            )
            .unwrap_err();
            assert!(error.to_string().contains(expected), "{error}");
            assert!(matches!(error.location, Location::Query(_)));
        }
    }

    // A concrete opaque argument parser starts a second common reader, just
    // like a host stage with an explicit native argument.
    struct ArgumentScope<'a>(&'a [&'a str]);
    impl ValueReaderHost for ArgumentScope<'_> {
        type Stage = Stage;
        type SetOperator = ();
        type Emitter = ();
        type Predicate = Truth;
        fn is_binding(&self, name: &str) -> bool {
            self.0.contains(&name)
        }
        fn is_stage(&self, name: &str) -> bool {
            Host.is_stage(name)
        }
        fn parse_stage(
            &mut self,
            reader: &mut zirql::ArgumentReader<'_>,
            budget: &mut Budget,
        ) -> Result<ExpressionStep<Stage, ()>, QueryError> {
            Host.parse_stage_in_scope(reader, self.0, budget)
        }
        fn parse_predicate_leaf(
            &mut self,
            reader: &mut zirql::ArgumentReader<'_>,
            budget: &mut Budget,
        ) -> Result<Truth, QueryError> {
            Host.parse_predicate_leaf_in_scope(reader, self.0, budget)
        }
        fn filter(&mut self, _: Predicate<Truth>, _: Range<usize>) -> Stage {
            unreachable!()
        }
        fn projection(&mut self, _: Projection<Query>, _: Range<usize>) -> Stage {
            unreachable!()
        }
        fn parse_path_edge(
            &mut self,
            edge: Located<&str>,
            budget: &mut Budget,
        ) -> Result<Stage, QueryError> {
            Host.parse_path_edge(edge, budget)
        }
        fn set_operator(&self, word: &str) -> Option<()> {
            Host.set_operator(word)
        }
    }
    impl ValueReaderHost for Host {
        type Stage = Stage;
        type SetOperator = ();
        type Emitter = ();
        type Predicate = Truth;
        fn is_binding(&self, name: &str) -> bool {
            name == "f"
        }
        fn is_stage(&self, name: &str) -> bool {
            matches!(
                name,
                "names" | "count" | "fail" | "edit" | "project" | "paths" | "native"
            )
        }
        fn parse_stage_in_scope(
            &mut self,
            reader: &mut zirql::ArgumentReader<'_>,
            locals: &[&str],
            budget: &mut Budget,
        ) -> Result<ExpressionStep<Stage, ()>, QueryError> {
            let mut probe = zirql::ArgumentReader::new(reader.source(), reader.position())?;
            if probe.keyword("native") {
                reader.word()?;
                reader.expect('(')?;
                let value =
                    zirql::read_value_expression(reader, &mut ArgumentScope(locals), budget, 64)?;
                reader.expect(')')?;
                return Ok(ExpressionStep::Stage(Stage::NativeArgument(Arc::new(
                    value,
                ))));
            }
            let mut probe = zirql::ArgumentReader::new(reader.source(), reader.position())?;
            if probe.keyword("paths") {
                return zirql::read_paths(reader, &mut ArgumentScope(locals), budget, 32)
                    .map(|paths| ExpressionStep::Stage(Stage::Paths(Box::new(paths))));
            }
            self.parse_stage(reader, budget)
        }
        fn parse_stage(
            &mut self,
            reader: &mut zirql::ArgumentReader<'_>,
            budget: &mut Budget,
        ) -> Result<ExpressionStep<Stage, ()>, QueryError> {
            let mut probe = zirql::ArgumentReader::new(reader.source(), reader.position())?;
            if probe.keyword("paths") {
                return zirql::read_paths(reader, self, budget, 32)
                    .map(|paths| ExpressionStep::Stage(Stage::Paths(Box::new(paths))));
            }
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
        fn parse_predicate_leaf_in_scope(
            &mut self,
            reader: &mut zirql::ArgumentReader<'_>,
            locals: &[&str],
            budget: &mut Budget,
        ) -> Result<Truth, QueryError> {
            let mut probe = zirql::ArgumentReader::new(reader.source(), reader.position())?;
            if probe.keyword("bound") {
                reader.word()?;
                reader.expect('(')?;
                let value =
                    zirql::read_value_expression(reader, &mut ArgumentScope(locals), budget, 64)?;
                reader.expect(')')?;
                if matches!(value.value,K::Name(ref name) if locals.contains(&name.as_str())) {
                    return Ok(Truth::True);
                }
                return Err(QueryError::query(value.range, "guard needs a lexical name"));
            }
            self.parse_predicate_leaf(reader, budget)
        }
        fn parse_predicate_leaf(
            &mut self,
            reader: &mut zirql::ArgumentReader<'_>,
            _: &mut Budget,
        ) -> Result<Truth, QueryError> {
            let word = reader.word()?;
            match word.value {
                "true" => Ok(Truth::True),
                "false" => Ok(Truth::False),
                "unknown" => Ok(Truth::Unknown),
                _ => Err(QueryError::query(word.range, "unsupported guard")),
            }
        }
        fn filter(&mut self, _: Predicate<Truth>, _: Range<usize>) -> Stage {
            unreachable!()
        }
        fn projection(
            &mut self,
            projection: Projection<ReadExpression<Self>>,
            _: Range<usize>,
        ) -> Stage {
            Stage::Projection(Box::new(projection))
        }
        fn matching(
            &mut self,
            matching: NativeMatch<ReadExpression<Self>, Truth>,
            _: Range<usize>,
        ) -> Result<Stage, QueryError> {
            Ok(Stage::Match(Box::new(matching)))
        }
        fn parse_path_edge(
            &mut self,
            edge: Located<&str>,
            _: &mut Budget,
        ) -> Result<Stage, QueryError> {
            match edge.value {
                "neighbors" => Ok(Stage::Neighbors),
                _ => Err(QueryError::query(edge.range, "unsupported path edge")),
            }
        }
        fn value_source(
            &mut self,
            value: zirql::ValueExpression<Query>,
            _: Range<usize>,
        ) -> Result<Stage, QueryError> {
            Ok(Stage::NativeArgument(Arc::new(value)))
        }
        fn set_operator(&self, word: &str) -> Option<()> {
            (word == "union").then_some(())
        }
    }
    impl MatchHost<()> for Host {
        type Guard = Truth;
        fn validate_guard_leaf(
            &self,
            _: &Truth,
            _: &Kind,
            _: &BTreeMap<String, Kind>,
            _: &mut Budget,
            _: &Range<usize>,
        ) -> Result<(), QueryError> {
            Ok(())
        }
        fn evaluate_guard_leaf(
            &self,
            guard: &Truth,
            _: &Value,
            _: &mut (),
            _: &Bindings<Value>,
            _: &mut Budget,
            _: &Range<usize>,
        ) -> Result<Truth, EvaluationError<QueryError>> {
            Ok(*guard)
        }
    }
    impl PathHost<()> for Host {
        type Target = Truth;
        fn validate_path_edge(
            &self,
            edge: &Stage,
            input: &Kind,
            _: &BTreeMap<String, Kind>,
            _: &mut Budget,
            range: &Range<usize>,
        ) -> Result<(), QueryError> {
            if *edge != Stage::Neighbors || *input != Kind::Subjects {
                return Err(QueryError::query(
                    range.clone(),
                    "paths need native graph subjects",
                ));
            }
            Ok(())
        }
        fn validate_target_leaf(
            &self,
            _: &Truth,
            input: &Kind,
            _: &BTreeMap<String, Kind>,
            _: &mut Budget,
            range: &Range<usize>,
        ) -> Result<(), QueryError> {
            if *input != Kind::Subjects {
                return Err(QueryError::query(
                    range.clone(),
                    "target needs graph subjects",
                ));
            }
            Ok(())
        }
        fn evaluate_target_leaf(
            &self,
            target: &Truth,
            _: &Value,
            _: &mut (),
            _: &Bindings<Value>,
            _: &mut Budget,
            _: &Range<usize>,
        ) -> Result<Truth, EvaluationError<QueryError>> {
            Ok(*target)
        }
        fn path_neighbors(
            &self,
            _: &Stage,
            input: &Value,
            _: &mut (),
            _: &Bindings<Value>,
            budget: &mut Budget,
            range: &Range<usize>,
        ) -> Result<Vec<Value>, EvaluationError<QueryError>> {
            let Value::Subjects(subjects) = input else {
                unreachable!()
            };
            let [subject] = subjects.as_slice() else {
                unreachable!()
            };
            let neighbors: &[&'static str] = match *subject {
                "alpha" => &["beta", "gamma"],
                "beta" => &["delta", "alpha"],
                "gamma" => &["delta", "epsilon"],
                "delta" => &["epsilon"],
                "epsilon" => &["beta"],
                _ => &[],
            };
            budget.check_items(neighbors.len(), Location::Query(range.clone()))?;
            budget.charge(neighbors.len(), Location::Query(range.clone()))?;
            budget.charge_results(neighbors.len() * 2, Location::Query(range.clone()))?;
            Ok(neighbors
                .iter()
                .map(|subject| Value::Subjects(vec![subject]))
                .collect())
        }
        fn same_path_identity(
            &self,
            left: &Value,
            right: &Value,
            _: &mut (),
            _: &mut Budget,
            _: &Range<usize>,
        ) -> Result<bool, EvaluationError<QueryError>> {
            // This borrowed host's subjects identify their native word value.
            Ok(left == right)
        }
        fn path_steps(
            &self,
            steps: Vec<Value>,
            _: &mut (),
            budget: &mut Budget,
            range: &Range<usize>,
        ) -> Result<Value, EvaluationError<QueryError>> {
            budget.charge(steps.len(), Location::Query(range.clone()))?;
            budget.charge_results(steps.len(), Location::Query(range.clone()))?;
            let mut subjects = Vec::with_capacity(steps.len());
            for step in steps {
                let Value::Subjects(value) = step else {
                    unreachable!()
                };
                let [value] = value.as_slice() else {
                    unreachable!()
                };
                subjects.push(*value);
            }
            Ok(Value::Subjects(subjects))
        }
    }
    struct SavedNames;
    impl ValueReaderHost for SavedNames {
        type Stage = Stage;
        type SetOperator = ();
        type Emitter = ();
        type Predicate = Truth;
        fn is_binding(&self, name: &str) -> bool {
            matches!(name, "f" | "one" | "project" | "if")
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
        ) -> Result<Truth, QueryError> {
            Host.parse_predicate_leaf(reader, budget)
        }
        fn filter(&mut self, predicate: Predicate<Truth>, range: Range<usize>) -> Stage {
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
    #[test]
    fn typed_constructed_wrappers_use_shared_admission_at_native_call_boundaries() {
        use zirql::{DeclaredType as D, OperationSignature, admit_value, checked_invoke};
        let signature = OperationSignature {
            parameters: vec![D::Sequence(Box::new(D::Integer))],
            result: D::Sequence(Box::new(D::Integer)),
        };
        let mut budget = budget();
        let arguments = vec![Located::new(
            Value::Constructed(V::Array(vec![V::Integer(3)])),
            10..15,
        )];
        let result = checked_invoke(
            &signature,
            arguments,
            &Host,
            &mut (),
            &mut budget,
            5..20,
            |mut arguments, _, _| Ok(arguments.remove(0).value),
        )
        .unwrap();
        admit_value(
            &result,
            &signature.result,
            &Host,
            &mut (),
            &mut budget,
            Location::Query(25..30),
        )
        .unwrap();
        let error = checked_invoke(
            &signature,
            [Located::new(result, 10..15)],
            &Host,
            &mut (),
            &mut budget,
            5..20,
            |_, _, budget| {
                // One container, one slot, one scalar and five copied bytes.
                budget.charge(8, Location::Query(5..20))?;
                budget.charge_results(3, Location::Query(5..20))?;
                Ok(Value::Constructed(V::Array(vec![V::String(
                    "wrong".into(),
                )])))
            },
        )
        .unwrap_err();
        assert!(
            matches!(error, EvaluationError::Core(QueryError { location: Location::Query(range), message, .. }) if range == (5..20) && message == "sequence element 1: expected integer")
        );
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

    fn parse_match(source: &str) -> NativeMatch<Query, Truth> {
        let mut reader = zirql::ArgumentReader::new(source, 0).unwrap();
        let matching = zirql::read_match(&mut reader, &mut Host, &mut budget(), 32).unwrap();
        assert!(reader.remaining().is_empty());
        matching
    }

    fn parse_paths(source: &str) -> NativePaths<Stage, Truth> {
        let mut reader = zirql::ArgumentReader::new(source, 0).unwrap();
        let paths = zirql::read_paths(&mut reader, &mut Host, &mut budget(), 32).unwrap();
        assert_eq!(reader.position(), source.len());
        paths
    }

    #[test]
    fn prepared_native_arguments_keep_demand_shared_reads_and_nested_scope() {
        use zirql::{ValueConstructor, ValueGenerator};
        fn scalar(kind: K<Query>) -> zirql::ValueExpression<Query> {
            Located::new(kind, 0..1)
        }
        fn binding(name: &str) -> Query {
            Located::new(ExpressionKind::Binding(name.into()), 0..1)
        }
        fn native(query: Query) -> zirql::ValueExpression<Query> {
            scalar(K::NativeQuery(Box::new(query)))
        }
        fn argument(
            value: zirql::ValueExpression<Query>,
            selected: bool,
        ) -> zirql::ValueExpression<Query> {
            native(Located::new(
                ExpressionKind::Sequence {
                    input: Box::new(Located::new(ExpressionKind::Input, 0..1)),
                    steps: vec![Located::new(
                        ExpressionStep::Stage(Stage::Argument {
                            value: Box::new(value),
                            selected,
                        }),
                        0..1,
                    )],
                },
                0..1,
            ))
        }
        let failing = native(Located::new(
            ExpressionKind::Sequence {
                input: Box::new(binding("w")),
                steps: vec![
                    Located::new(ExpressionStep::Stage(Stage::Fail), 0..1),
                    Located::new(ExpressionStep::Stage(Stage::Names), 0..1),
                ],
            },
            0..1,
        ));
        let own_generator = scalar(K::Comprehension {
            constructor: ValueConstructor::Array,
            clauses: vec![Located::new(
                ValueGenerator::In {
                    binders: vec![Located::new(Some("m".into()), 0..1)],
                    source: scalar(K::Array(vec![
                        scalar(K::Name("n".into())),
                        scalar(K::Literal(V::Integer(3))),
                    ])),
                    filter: None,
                },
                0..1,
            )],
            head: Box::new(native(binding("m"))),
        });
        let nested = scalar(K::Comprehension {
            constructor: ValueConstructor::Array,
            clauses: vec![Located::new(
                ValueGenerator::In {
                    binders: vec![Located::new(Some("n".into()), 0..1)],
                    source: scalar(K::Array(vec![
                        scalar(K::Literal(V::Integer(1))),
                        scalar(K::Literal(V::Integer(2))),
                    ])),
                    filter: None,
                },
                0..1,
            )],
            head: Box::new(argument(own_generator, true)),
        });
        let projection = Projection {
            subject: Located::new("w".into(), 0..1),
            fields: vec![
                (
                    Located::new("size".into(), 0..1),
                    argument(scalar(K::Name("big".into())), true),
                ),
                (
                    Located::new("label_width".into(), 0..1),
                    argument(
                        scalar(K::One(Box::new(native(Located::new(
                            ExpressionKind::Sequence {
                                input: Box::new(binding("w")),
                                steps: vec![Located::new(
                                    ExpressionStep::Stage(Stage::Names),
                                    0..1,
                                )],
                            },
                            0..1,
                        ))))),
                        true,
                    ),
                ),
                (Located::new("skip".into(), 0..1), argument(failing, false)),
                (Located::new("nested".into(), 0..1), nested),
            ],
        };
        let original = Arc::new(V::Array(
            (0..1000).map(|_| V::String("x".repeat(128))).collect(),
        ));
        let bindings = Bindings::new([(
            Located::new("big".into(), 0..1),
            Value::Shared(Arc::clone(&original)),
        )])
        .unwrap();
        for subjects in [vec!["alpha", "beta"], vec!["other"]] {
            let result = zirql::evaluate_projection(
                &projection,
                &Host,
                Value::Subjects(subjects.clone()),
                &mut (),
                &bindings,
                &mut budget(),
            )
            .unwrap();
            assert_eq!(
                result,
                V::Array(
                    subjects
                        .iter()
                        .map(|subject| V::Record(vec![
                            ("size".into(), V::Integer(2000)),
                            (
                                "label_width".into(),
                                V::Integer(i64::try_from(subject.len() * 2).unwrap())
                            ),
                            ("skip".into(), V::Integer(0)),
                            (
                                "nested".into(),
                                V::Array(vec![V::Integer(8), V::Integer(10)])
                            )
                        ]))
                        .collect()
                )
            );
        }
        assert!(matches!(original.as_ref(),V::Array(values) if values.len()==1000));
        let invalid = Projection {
            subject: Located::new("w".into(), 0..1),
            fields: vec![(
                Located::new("skip".into(), 0..1),
                argument(scalar(K::Name("missing".into())), false),
            )],
        };
        assert!(
            zirql::evaluate_projection(
                &invalid,
                &Host,
                Value::Subjects(vec![]),
                &mut (),
                &bindings,
                &mut budget()
            )
            .is_err()
        );
    }

    #[test]
    fn prepared_projection_keeps_nested_slots_rows_and_fresh_inputs() {
        let projection = parse(
            "project(w, {\"name\": one(w | names), \"pairs\": [n ++ suffix for n in w | names for suffix in [\"!\", \"?\"]]})",
            &mut budget(),
        );
        let mut first = None;
        for subjects in [vec!["alpha", "beta", "alpha"], vec!["other"], vec![]] {
            let result = zirql::evaluate_projection_with_types(
                &projection,
                &Host,
                Value::Subjects(subjects.clone()),
                &mut (),
                &Bindings::default(),
                &mut budget(),
            )
            .unwrap();
            assert!(!result.types.is_empty());
            let expected = V::Array(
                subjects
                    .iter()
                    .map(|subject| {
                        V::Record(vec![
                            ("name".into(), V::String((*subject).into())),
                            (
                                "pairs".into(),
                                V::Array(vec![
                                    V::String(format!("{subject}!")),
                                    V::String(format!("{subject}?")),
                                ]),
                            ),
                        ])
                    })
                    .collect(),
            );
            assert_eq!(result.value, expected);
            if first.is_none() {
                first = Some(result.value);
            }
        }
        let V::Array(rows) = first.unwrap() else {
            unreachable!()
        };
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0], rows[2]);
    }

    #[test]
    fn native_paths_choose_shortest_witnesses_and_keep_native_fields() {
        let paths = parse_paths("paths(neighbors, to=true, max_depth=3)");
        let rows = zirql::evaluate_paths(
            &paths,
            &Host,
            Value::Subjects(vec!["gamma", "alpha", "gamma"]),
            &mut (),
            &Bindings::default(),
            &mut budget(),
        )
        .unwrap();
        let witnesses = rows
            .iter()
            .map(|row| {
                let Value::Capture(record) = row else {
                    panic!("native path record")
                };
                assert_eq!(
                    record
                        .fields
                        .iter()
                        .map(|(name, _)| name.as_str())
                        .collect::<Vec<_>>(),
                    ["source", "sink", "steps"]
                );
                let native = |index: usize| {
                    let Value::Subjects(subjects) = &record.fields[index].1 else {
                        panic!("native field")
                    };
                    subjects.clone()
                };
                (native(0), native(1), native(2))
            })
            .collect::<Vec<_>>();
        assert_eq!(
            witnesses,
            [
                (vec!["gamma"], vec!["gamma"], vec!["gamma"]),
                (vec!["gamma"], vec!["delta"], vec!["gamma", "delta"]),
                (vec!["gamma"], vec!["epsilon"], vec!["gamma", "epsilon"]),
                (
                    vec!["gamma"],
                    vec!["beta"],
                    vec!["gamma", "epsilon", "beta"]
                ),
                (
                    vec!["gamma"],
                    vec!["alpha"],
                    vec!["gamma", "epsilon", "beta", "alpha"]
                ),
                (vec!["alpha"], vec!["alpha"], vec!["alpha"]),
                (vec!["alpha"], vec!["beta"], vec!["alpha", "beta"]),
                (vec!["alpha"], vec!["gamma"], vec!["alpha", "gamma"]),
                (vec!["alpha"], vec!["delta"], vec!["alpha", "beta", "delta"]),
                (
                    vec!["alpha"],
                    vec!["epsilon"],
                    vec!["alpha", "gamma", "epsilon"]
                ),
            ]
        );

        let zero = parse_paths("paths(neighbors, to=true, max_depth=0)");
        let rows = zirql::evaluate_paths(
            &zero,
            &Host,
            Value::Subjects(vec!["alpha"]),
            &mut (),
            &Bindings::default(),
            &mut budget(),
        )
        .unwrap();
        let Value::Capture(record) = &rows[0] else {
            panic!("native record")
        };
        assert_eq!(rows.len(), 1);
        assert_eq!(record.fields[2].1, Value::Subjects(vec!["alpha"]));
        let one = parse_paths("paths(neighbors, to=true, max_depth=1)");
        let rows = zirql::evaluate_paths(
            &one,
            &Host,
            Value::Subjects(vec!["alpha"]),
            &mut (),
            &Bindings::default(),
            &mut budget(),
        )
        .unwrap();
        assert_eq!(rows.len(), 3); // delta and epsilon have no witness at one edge

        let source = r#"input | paths(neighbors, to=true, max_depth=2) | project(p, {"source": p.source | names | one, "steps": p.steps | names})"#;
        let mut reader = zirql::ArgumentReader::new(source, 0).unwrap();
        let query =
            zirql::read_native_expression(&mut reader, &mut Host, &mut budget(), 32).unwrap();
        let value = zirql::evaluate_expression(
            &query,
            &Host,
            Value::Subjects(vec!["alpha"]),
            &mut (),
            &Bindings::default(),
            &mut budget(),
        )
        .unwrap();
        let Value::Constructed(V::Array(values)) = value else {
            panic!("projected rows")
        };
        assert_eq!(
            values[3],
            V::Record(vec![
                ("source".into(), V::String("alpha".into())),
                (
                    "steps".into(),
                    V::Array(
                        ["alpha", "beta", "delta"]
                            .map(|s| V::String(s.into()))
                            .into_iter()
                            .collect()
                    )
                ),
            ])
        );
        let empty = zirql::evaluate_expression(
            &query,
            &Host,
            Value::Subjects(vec![]),
            &mut (),
            &Bindings::default(),
            &mut budget(),
        )
        .unwrap();
        assert_eq!(empty, Value::Constructed(V::Array(vec![])));
        zirql::dispose_expression(query);
        for paths in [paths, zero, one] {
            zirql::dispose_paths(paths);
        }
    }

    #[test]
    fn native_paths_preflight_empty_inputs_and_refuse_unknown_and_exhaustion() {
        let mut paths = parse_paths("paths(neighbors, to=not unknown, max_depth=2)");
        let error = zirql::evaluate_paths(
            &paths,
            &Host,
            Value::Subjects(vec!["alpha"]),
            &mut (),
            &Bindings::default(),
            &mut budget(),
        )
        .unwrap_err();
        assert_eq!(
            error,
            EvaluationError::Core(
                QueryError::query(paths.target.range.clone(), "path target is Unknown")
                    .with_category(QueryErrorCategory::UnknownPredicate)
            )
        );
        assert!(
            zirql::evaluate_paths(
                &paths,
                &Host,
                Value::Subjects(vec![]),
                &mut (),
                &Bindings::default(),
                &mut budget()
            )
            .unwrap()
            .is_empty()
        );
        paths.target = Located::new(
            zirql::PredicateKind::Or(vec![
                Located::new(zirql::PredicateKind::Leaf(Truth::True), 0..1),
                Located::new(zirql::PredicateKind::And(vec![]), 2..3),
            ]),
            0..3,
        );
        assert!(
            matches!(zirql::evaluate_paths(&paths, &Host, Value::Subjects(vec![]),
            &mut (), &Bindings::default(), &mut budget()),
            Err(EvaluationError::Core(QueryError { message, .. })) if message == "a Boolean combination needs at least one predicate")
        );
        let valid = parse_paths("paths(neighbors, to=unknown or true, max_depth=2)");
        assert_eq!(
            zirql::evaluate_paths(
                &valid,
                &Host,
                Value::Subjects(vec!["alpha"]),
                &mut (),
                &Bindings::default(),
                &mut budget()
            )
            .unwrap()
            .len(),
            5
        );
        assert!(
            zirql::evaluate_paths(
                &valid,
                &Host,
                Value::Strings(vec![]),
                &mut (),
                &Bindings::default(),
                &mut budget()
            )
            .is_err()
        );
        let mut legacy = Budget::new(zirql::Limits {
            work: 10_000,
            items: 100,
            nesting: 32,
        });
        assert!(
            matches!(zirql::evaluate_paths(&valid, &Host, Value::Subjects(vec![]),
            &mut (), &Bindings::default(), &mut legacy),
            Err(EvaluationError::Core(QueryError { message, .. })) if message == "native paths require aggregate result limits")
        );
        let mut scarce = Budget::for_program(ProgramLimits {
            results: 20,
            ..super::limits()
        });
        assert!(
            matches!(zirql::evaluate_paths(&valid, &Host, Value::Subjects(vec!["alpha"]),
            &mut (), &Bindings::default(), &mut scarce),
            Err(EvaluationError::Core(QueryError { message, .. })) if message == "result limit exceeded")
        );
        let mut little_work = Budget::for_program(ProgramLimits {
            work: 50,
            ..super::limits()
        });
        assert!(
            matches!(zirql::evaluate_paths(&valid, &Host, Value::Subjects(vec!["alpha"]),
            &mut (), &Bindings::default(), &mut little_work),
            Err(EvaluationError::Core(QueryError { message, .. })) if message == "work limit exceeded")
        );
        let mut small_stream = Budget::for_program(ProgramLimits {
            stream_items: 2,
            ..super::limits()
        });
        assert!(
            matches!(zirql::evaluate_paths(&valid, &Host, Value::Subjects(vec!["alpha"]),
            &mut (), &Bindings::default(), &mut small_stream),
            Err(EvaluationError::Core(QueryError { message, .. })) if message == "item limit exceeded")
        );
        paths.edge.value = Stage::Edit;
        assert!(
            matches!(zirql::evaluate_paths(&paths, &Host, Value::Subjects(vec![]),
            &mut (), &Bindings::default(), &mut budget()),
            Err(EvaluationError::Core(QueryError { message, .. })) if message == "path edges must be pure")
        );
        for paths in [paths, valid] {
            zirql::dispose_paths(paths);
        }

        for source in [
            "paths(neighbors, to=true)",
            "paths(neighbors, to=true, max_depth=-1)",
            "paths(neighbors, to=true, max_depth=184467440737095516160)",
            "paths(neighbors | names, to=true, max_depth=2)",
            "paths(neighbors, max_depth=2, to=true)",
            "paths(neighbors, to=true, max_depth=2, extra=1)",
        ] {
            let mut reader = zirql::ArgumentReader::new(source, 0).unwrap();
            assert!(zirql::read_paths(&mut reader, &mut Host, &mut budget(), 32).is_err());
        }
        let source = "paths(neighbors, to=true, max_depth=2)";
        let mut reader = zirql::ArgumentReader::new(source, 0).unwrap();
        assert!(
            matches!(zirql::read_paths(&mut reader, &mut SavedNames, &mut budget(), 32),
            Err(QueryError { message, .. }) if message == "native paths are unsupported by this host")
        );
    }

    #[test]
    fn native_captures_keep_written_fields_and_feed_shared_projection() {
        let source = "match { from add in input; bind lhs = add; bind rhs = lhs; where not false and (true or unknown); }";
        let matching = parse_match(source);
        let mut allowance = budget();
        let schema = zirql::match_types(
            &matching,
            &Host,
            &Kind::Subjects,
            &BTreeMap::new(),
            &mut allowance,
        )
        .unwrap();
        assert_eq!(
            schema
                .fields
                .iter()
                .map(|(name, _)| name.as_str())
                .collect::<Vec<_>>(),
            ["add", "lhs", "rhs"]
        );
        let rows = zirql::evaluate_match(
            &matching,
            &Host,
            Value::Subjects(vec!["beta", "alpha", "beta"]),
            &mut (),
            &Bindings::default(),
            &mut allowance,
        )
        .unwrap();
        assert_eq!(rows.len(), 3);
        for (row, name) in rows.iter().zip(["beta", "alpha", "beta"]) {
            let Value::Capture(record) = row else {
                panic!("native capture")
            };
            assert_eq!(
                record
                    .fields
                    .iter()
                    .map(|(name, _)| name.as_str())
                    .collect::<Vec<_>>(),
                ["add", "lhs", "rhs"]
            );
            assert!(
                record
                    .fields
                    .iter()
                    .all(|(_, value)| *value == Value::Subjects(vec![name]))
            );
        }
        let query_source = format!(
            r#"{source} | project(m, {{"name": m.add | names | one, "n": m.rhs | count}})"#
        );
        let mut reader = zirql::ArgumentReader::new(&query_source, 0).unwrap();
        let query =
            zirql::read_native_expression(&mut reader, &mut Host, &mut allowance, 32).unwrap();
        assert!(reader.remaining().is_empty());
        let result = zirql::evaluate_expression(
            &query,
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
            Value::Constructed(V::Array(vec![row("beta"), row("alpha"), row("beta")]))
        );
        let field = Located::new(
            ExpressionKind::Field {
                value: Box::new(Located::new(ExpressionKind::Binding("m".into()), 3..4)),
                field: Located::new("add".into(), 5..8),
            },
            3..8,
        );
        assert_eq!(
            zirql::native_expression_kind::<(), _>(
                &field,
                &Host,
                &Kind::Subjects,
                &BTreeMap::from([("m".into(), Kind::Capture(Arc::new(schema.clone())))]),
                &mut allowance
            )
            .unwrap(),
            Kind::Subjects
        );
        let bindings = Bindings::new([(
            Located::new("m".into(), 0..1),
            rows.into_iter().next().unwrap(),
        )])
        .unwrap();
        assert_eq!(
            zirql::evaluate_expression(
                &field,
                &Host,
                Value::Subjects(vec![]),
                &mut (),
                &bindings,
                &mut allowance
            )
            .unwrap(),
            Value::Subjects(vec!["beta"])
        );
        let projection = parse(
            r#"project(m, {"name": m.add | names | one})"#,
            &mut allowance,
        );
        let empty = Value::Captures {
            rows: vec![],
            schema: Arc::new(schema),
        };
        assert_eq!(
            zirql::evaluate_projection(
                &projection,
                &Host,
                empty,
                &mut (),
                &Bindings::default(),
                &mut allowance
            )
            .unwrap(),
            V::Array(vec![])
        );
        // Plain constructed records still use the scalar field representation.
        let scalar = zirql::read_value_expression(
            &mut zirql::ArgumentReader::new("r.name", 0).unwrap(),
            &mut Host,
            &mut allowance,
            32,
        )
        .unwrap();
        assert!(matches!(scalar.value, K::Field { .. }));
        // Match is contextual: the legacy bare binding remains readable.
        let bare = zirql::read_native_expression(
            &mut zirql::ArgumentReader::new("match | count", 0).unwrap(),
            &mut Host,
            &mut allowance,
            32,
        )
        .unwrap();
        let names = BTreeMap::from([("match".into(), Kind::Subjects)]);
        assert_eq!(
            zirql::native_expression_kind::<(), _>(
                &bare,
                &Host,
                &Kind::Subjects,
                &names,
                &mut allowance
            )
            .unwrap(),
            Kind::Count
        );
        zirql::dispose_expression(query);
        zirql::dispose_expression(bare);
        zirql::dispose_expression(field);
        zirql::dispose_match(matching);
    }

    #[test]
    fn dependent_matches_refuse_scope_types_effects_missing_values_and_unknown() {
        let matching =
            parse_match("match { from outer in input; from inner in input; bind saved = outer; }");
        let mut allowance = budget();
        let rows = zirql::evaluate_match(
            &matching,
            &Host,
            Value::Subjects(vec!["beta", "alpha"]),
            &mut (),
            &Bindings::default(),
            &mut allowance,
        )
        .unwrap();
        let pairs = rows
            .iter()
            .map(|row| {
                let Value::Capture(record) = row else {
                    panic!("native capture")
                };
                (
                    record.fields[0].1 == Value::Subjects(vec!["beta"]),
                    record.fields[1].1 == Value::Subjects(vec!["beta"]),
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(
            pairs,
            [(true, true), (true, false), (false, true), (false, false)]
        );
        // A later source sees the first singleton rather than the original input.
        let dependent = parse_match("match { from outer in input; from inner in outer; }");
        assert_eq!(
            zirql::evaluate_match(
                &dependent,
                &Host,
                Value::Subjects(vec!["beta", "alpha"]),
                &mut (),
                &Bindings::default(),
                &mut allowance
            )
            .unwrap()
            .len(),
            2
        );
        for source in [
            "match { from x in x; }",
            "match { from x in later; bind later = input; }",
            "match { from x in input; bind x = input; }",
            "match { from input in input; }",
            "match { from x in input; where false; bind bad = input | count; }",
            "match { from x in input; where false; bind bad = input | edit; }",
        ] {
            let matching = parse_match(source);
            assert!(
                zirql::evaluate_match(
                    &matching,
                    &Host,
                    Value::Subjects(vec![]),
                    &mut (),
                    &Bindings::default(),
                    &mut allowance
                )
                .is_err()
            );
            zirql::dispose_match(matching);
        }
        let ambient = parse_match("match { from f in input; }");
        let bindings =
            Bindings::new([(Located::new("f".into(), 0..1), Value::Subjects(vec![]))]).unwrap();
        assert!(
            zirql::evaluate_match(
                &ambient,
                &Host,
                Value::Subjects(vec![]),
                &mut (),
                &bindings,
                &mut allowance
            )
            .is_err()
        );
        let required = parse_match("match { bind x = input; }");
        for input in [vec![], vec!["a", "b"]] {
            let error = zirql::evaluate_match(
                &required,
                &Host,
                Value::Subjects(input),
                &mut (),
                &Bindings::default(),
                &mut allowance,
            )
            .unwrap_err();
            assert!(
                matches!(error, EvaluationError::Core(QueryError { message, .. }) if message == "required capture needs exactly one value")
            );
        }
        let unknown_source = "match { from x in input; where not unknown; }";
        let unknown = parse_match(unknown_source);
        let error = zirql::evaluate_match(
            &unknown,
            &Host,
            Value::Subjects(vec!["a"]),
            &mut (),
            &Bindings::default(),
            &mut allowance,
        )
        .unwrap_err();
        let start = unknown_source.find("not").unwrap();
        assert_eq!(
            error,
            EvaluationError::Core(
                QueryError::query(
                    start..start + "not unknown".len(),
                    "required match guard is Unknown"
                )
                .with_category(QueryErrorCategory::UnknownPredicate)
            )
        );
        let mut reader = zirql::ArgumentReader::new(
            r#"match { from x in input; } | project(m, {"bad": m.missing | names})"#,
            0,
        )
        .unwrap();
        let invalid_field =
            zirql::read_native_expression(&mut reader, &mut Host, &mut allowance, 32).unwrap();
        assert!(
            matches!(zirql::native_expression_kind::<(),_>(&invalid_field, &Host, &Kind::Subjects, &BTreeMap::new(), &mut allowance), Err(QueryError { message, .. }) if message == "unknown native capture field")
        );
        zirql::dispose_expression(invalid_field);
        for matching in [matching, dependent, ambient, required, unknown] {
            zirql::dispose_match(matching);
        }
    }

    #[test]
    fn match_candidates_before_false_guards_share_work_result_and_nesting_limits() {
        let matching =
            parse_match("match { from outer in input; from inner in input; where false; }");
        let input = || Value::Subjects(vec!["a"; 10]);
        let mut low = Budget::for_program(ProgramLimits {
            work: 300,
            results: 10_000,
            nesting: 32,
            stream_items: 100,
        });
        let error = zirql::evaluate_match(
            &matching,
            &Host,
            input(),
            &mut (),
            &Bindings::default(),
            &mut low,
        )
        .unwrap_err();
        assert!(
            matches!(error, EvaluationError::Core(QueryError { message, .. }) if message == "work limit exceeded")
        );
        let mut allowance = budget();
        assert!(
            zirql::evaluate_match(
                &matching,
                &Host,
                input(),
                &mut (),
                &Bindings::default(),
                &mut allowance
            )
            .unwrap()
            .is_empty()
        );
        let mut shallow = Budget::for_program(ProgramLimits {
            nesting: 2,
            ..super::limits()
        });
        assert!(
            zirql::evaluate_match(
                &matching,
                &Host,
                input(),
                &mut (),
                &Bindings::default(),
                &mut shallow
            )
            .is_err()
        );
        let produces = parse_match("match { from x in input; }");
        let mut scarce = Budget::for_program(ProgramLimits {
            results: 2,
            ..super::limits()
        });
        assert!(
            matches!(zirql::evaluate_match(&produces, &Host, Value::Subjects(vec!["a"]), &mut (), &Bindings::default(), &mut scarce), Err(EvaluationError::Core(QueryError { message, .. })) if message == "result limit exceeded")
        );
        let mut legacy = Budget::new(zirql::Limits {
            work: 1000,
            items: 100,
            nesting: 32,
        });
        assert!(
            matches!(zirql::evaluate_match(&produces, &Host, input(), &mut (), &Bindings::default(), &mut legacy), Err(EvaluationError::Core(QueryError { message, .. })) if message == "native matching requires aggregate result limits")
        );
        zirql::dispose_match(matching);
        zirql::dispose_match(produces);
    }

    #[test]
    fn native_rows_reader_preflight_and_lazy_requirement_share_the_value_runner() {
        let mut allowance = budget();
        let bindings = Bindings::new([
            (
                Located::new("keyed".into(), 0..0),
                Value::Constructed(V::KeyedArray {
                    axes: vec![zirql::KeyAxis {
                        key_type: zirql::ValueKeyType::Integer,
                        keys: vec![zirql::ValueKey::Integer(9), zirql::ValueKey::Integer(3)],
                    }],
                    entries: vec![
                        zirql::KeyedEntry {
                            key: vec![zirql::ValueKey::Integer(9)],
                            value: V::Integer(90),
                        },
                        zirql::KeyedEntry {
                            key: vec![zirql::ValueKey::Integer(3)],
                            value: V::Integer(30),
                        },
                    ],
                }),
            ),
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
            (
                Located::new("if".into(), 0..0),
                Value::Constructed(V::Boolean(true)),
            ),
        ])
        .unwrap();
        for (source, expected) in [
            (
                "entries(zip_exact(keyed, keyed))",
                V::Array(vec![
                    V::Record(vec![
                        ("key".into(), V::Integer(9)),
                        (
                            "value".into(),
                            V::Tuple(vec![V::Integer(90), V::Integer(90)]),
                        ),
                    ]),
                    V::Record(vec![
                        ("key".into(), V::Integer(3)),
                        (
                            "value".into(),
                            V::Tuple(vec![V::Integer(30), V::Integer(30)]),
                        ),
                    ]),
                ]),
            ),
            ("f | names | one", V::Array(vec![V::String("saved".into())])),
            (
                "project(w, {\"n\": w | count})",
                V::Array(vec![V::Record(vec![("n".into(), V::Integer(1))])]),
            ),
            ("project", V::Array(vec![V::String("legacy".into())])),
            ("if", V::Boolean(true)),
            ("if and true", V::Boolean(true)),
            ("if true { if } else { false }", V::Boolean(true)),
            ("if if { 1 } else { 0 }", V::Integer(1)),
            ("f | project", V::Array(vec![V::String("legacy".into())])),
            ("one(f | names)", V::String("beta".into())),
            (
                "group_by(project(w, {\"n\": w | count}), \"n\")",
                V::Array(vec![V::Record(vec![
                    ("key".into(), V::Integer(1)),
                    (
                        "items".into(),
                        V::Array(vec![V::Record(vec![("n".into(), V::Integer(1))])]),
                    ),
                ])]),
            ),
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
            if let K::GroupBy { field, .. } = &expression.value {
                assert_eq!(&source[field.range.clone()], "\"n\"");
            }
            zirql::dispose_native_value(expression);
        }
        for (source, expected) in [
            (
                "lookup({\"caption\": null}, \"caption\") default \"untitled\"",
                V::String("untitled".into()),
            ),
            (
                "has_key({\"caption\": null}, \"caption\")",
                V::Boolean(true),
            ),
            ("has_key({}, \"caption\")", V::Boolean(false)),
            ("absent(lookup([], 1))", V::Boolean(true)),
            ("occurs(false)", V::Boolean(true)),
            ("<> default 3", V::Integer(3)),
            ("null default 3 + 2", V::Integer(5)),
            (
                "one(f | names) default one(f | fail | names)",
                V::String("beta".into()),
            ),
        ] {
            let mut reader = zirql::ArgumentReader::new(source, 0).unwrap();
            let expression =
                zirql::read_value_expression(&mut reader, &mut SavedNames, &mut allowance, 32)
                    .unwrap();
            assert!(reader.remaining().is_empty());
            let result = zirql::evaluate_native_value(
                &expression,
                &Host,
                Value::Subjects(vec!["beta"]),
                &mut (),
                &bindings,
                &mut allowance,
            )
            .unwrap();
            assert_eq!(result, expected);
            zirql::dispose_native_value(expression);
        }
        let source = "null default one(f | fail | names)";
        let expression = zirql::read_value_expression(
            &mut zirql::ArgumentReader::new(source, 0).unwrap(),
            &mut SavedNames,
            &mut allowance,
            32,
        )
        .unwrap();
        let error = zirql::evaluate_native_value(
            &expression,
            &Host,
            Value::Subjects(vec!["beta"]),
            &mut (),
            &bindings,
            &mut allowance,
        )
        .unwrap_err();
        assert_eq!(
            error,
            EvaluationError::Host(QueryError::source(20..21, "native failure"))
        );
        zirql::dispose_native_value(expression);
        let mut reader = zirql::ArgumentReader::new("  null", 0).unwrap();
        let absence =
            zirql::read_value_expression(&mut reader, &mut Host, &mut allowance, 32).unwrap();
        assert_eq!(absence.range, 2..6);
        assert!(matches!(absence.value, K::Literal(V::Absent)));
        for source in [
            "entries()",
            "entries([], [])",
            "zip_exact([])",
            "zip_exact([], [], [])",
            "group_by([], 1)",
            "group_by([])",
            "group_by([], \"n\", \"m\")",
        ] {
            let mut reader = zirql::ArgumentReader::new(source, 0).unwrap();
            assert!(
                zirql::read_value_expression(&mut reader, &mut Host, &mut allowance, 32).is_err()
            );
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
        let mut reader =
            zirql::ArgumentReader::new("[names for names in [1] if names == 1]", 0).unwrap();
        let expression =
            zirql::read_value_expression(&mut reader, &mut Host, &mut allowance, 32).unwrap();
        assert_eq!(
            zirql::evaluate_native_value(
                &expression,
                &Host,
                Value::Subjects(vec!["alpha"]),
                &mut (),
                &Bindings::default(),
                &mut allowance,
            )
            .unwrap(),
            V::Array(vec![V::Integer(1)])
        );
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

#[test]
fn shared_reader_releases_native_values_on_errors() {
    // An abort must fail the parent assertion without killing the test harness.
    const CHILD: &str = "ZIRQL_NATIVE_CLEANUP_CHILD";
    if std::env::var_os(CHILD).is_none() {
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "shared_reader_releases_native_values_on_errors",
                "--nocapture",
            ])
            .env(CHILD, "1")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        return;
    }

    // The host holds only a weak handle; accepted set operands retain its payload.
    #[derive(Default)]
    struct ReaderHost(std::cell::RefCell<std::rc::Weak<()>>);
    impl zirql::ValueReaderHost for ReaderHost {
        type Stage = ();
        type SetOperator = Rc<()>;
        type Emitter = ();
        type Predicate = Rc<()>;
        fn is_binding(&self, _: &str) -> bool {
            false
        }
        fn is_stage(&self, _: &str) -> bool {
            false
        }
        fn parse_stage(
            &mut self,
            reader: &mut zirql::ArgumentReader<'_>,
            _: &mut Budget,
        ) -> Result<ExpressionStep<(), ()>, QueryError> {
            Err(reader.error("no domain stages"))
        }
        fn parse_predicate_leaf(
            &mut self,
            reader: &mut zirql::ArgumentReader<'_>,
            _: &mut Budget,
        ) -> Result<Rc<()>, QueryError> {
            reader.word()?;
            let retained = self.0.borrow().upgrade().unwrap_or_else(|| Rc::new(()));
            *self.0.borrow_mut() = Rc::downgrade(&retained);
            Ok(retained)
        }
        fn filter(&mut self, _: zirql::Predicate<Rc<()>>, _: Range<usize>) {}
        fn parse_path_edge(&mut self, _: Located<&str>, _: &mut Budget) -> Result<(), QueryError> {
            Ok(())
        }
        fn projection(
            &mut self,
            projection: zirql::Projection<zirql::ReadExpression<Self>>,
            _: Range<usize>,
        ) {
            for (_, value) in projection.fields {
                zirql::dispose_native_value(value);
            }
        }
        fn set_operator(&self, word: &str) -> Option<Rc<()>> {
            if word != "union" {
                return None;
            }
            let retained = self.0.borrow().upgrade().unwrap_or_else(|| Rc::new(()));
            *self.0.borrow_mut() = Rc::downgrade(&retained);
            Some(retained)
        }
    }
    let generous = ProgramLimits {
        work: 100_000_000,
        results: 100_000_000,
        nesting: 64,
        stream_items: 1,
    };
    let prefix = format!("input{}", " union input".repeat(100_000));
    let mut host = ReaderHost::default();
    let source = "paths(edge, to=not leaf or leaf, max_depth=0)";
    let mut reader = zirql::ArgumentReader::new(source, 0).unwrap();
    let paths = zirql::read_paths(
        &mut reader,
        &mut host,
        &mut Budget::for_program(generous),
        64,
    )
    .unwrap();
    assert!(host.0.borrow().upgrade().is_some());
    zirql::dispose_paths(paths);
    assert!(host.0.borrow().upgrade().is_none());
    let source = "paths(edge, to=not leaf or leaf, max_depth=-1)";
    let mut reader = zirql::ArgumentReader::new(source, 0).unwrap();
    assert!(
        zirql::read_paths(
            &mut reader,
            &mut host,
            &mut Budget::for_program(generous),
            64
        )
        .is_err()
    );
    assert!(host.0.borrow().upgrade().is_none());
    let mut reader = zirql::ArgumentReader::new(&prefix, 0).unwrap();
    let value = zirql::read_value_expression(
        &mut reader,
        &mut host,
        &mut Budget::for_program(generous),
        64,
    )
    .unwrap();
    assert_eq!(reader.position(), prefix.len());
    assert!(host.0.borrow().upgrade().is_some());
    zirql::dispose_native_value(value);
    assert!(host.0.borrow().upgrade().is_none());
    let mut reader = zirql::ArgumentReader::new(&prefix, 0).unwrap();
    let native = zirql::read_native_expression(
        &mut reader,
        &mut host,
        &mut Budget::for_program(generous),
        64,
    )
    .unwrap();
    assert_eq!(reader.position(), prefix.len());
    assert!(host.0.borrow().upgrade().is_some());
    zirql::dispose_expression(native);
    assert!(host.0.borrow().upgrade().is_none());

    for (source, message) in [
        (format!("{prefix} +"), "expected a value expression"),
        (format!("{prefix} union"), "expected an identifier"),
        (format!("[{prefix}"), "expected ']'"),
        (format!("entries({prefix}"), "expected ')'"),
        (
            format!("zip_exact({prefix},"),
            "expected a value expression",
        ),
        (format!("zip_exact(input, {prefix}"), "expected ')'"),
        (format!("{{\"x\": {prefix}"), "expected '}'"),
        (format!("require(true, {prefix},"), "expected '\"'"),
        (
            format!("[{prefix} for x in input if"),
            "expected a value expression",
        ),
    ] {
        let mut reader = zirql::ArgumentReader::new(&source, 0).unwrap();
        let error = zirql::read_value_expression(
            &mut reader,
            &mut host,
            &mut Budget::for_program(generous),
            64,
        )
        .unwrap_err();
        assert_eq!(error.location, Location::Query(source.len()..source.len()));
        assert_eq!(error.message, message);
        assert!(host.0.borrow().upgrade().is_none());
    }
    let source = format!("project(x, {{\"x\": {prefix}}}");
    let mut reader = zirql::ArgumentReader::new(&source, 0).unwrap();
    let error = zirql::read_projection(
        &mut reader,
        &mut host,
        &mut Budget::for_program(generous),
        64,
    )
    .unwrap_err();
    assert_eq!(error.location, Location::Query(source.len()..source.len()));
    assert_eq!(error.message, "expected ')'");
    assert!(host.0.borrow().upgrade().is_none());

    // Public remaining allowances locate refusals after native children exist:
    // binary node admission, array insertion, and comprehension clause insertion.
    for (source, missing_results, work_refusal) in [
        (format!("{prefix} + 1"), 1, true),
        (format!("[{prefix}]"), 2, false),
        (format!("[0 for x in {prefix}]"), 3, false),
    ] {
        let mut allowance = Budget::for_program(generous);
        let mut reader = zirql::ArgumentReader::new(&source, 0).unwrap();
        let value =
            zirql::read_value_expression(&mut reader, &mut host, &mut allowance, 64).unwrap();
        let work = generous.work - allowance.remaining_work();
        let results = generous.results - allowance.remaining_results().unwrap();
        zirql::dispose_native_value(value);
        assert!(host.0.borrow().upgrade().is_none());
        let mut scarce = Budget::for_program(ProgramLimits {
            results: results - missing_results,
            ..generous
        });
        let mut reader = zirql::ArgumentReader::new(&source, 0).unwrap();
        let error =
            zirql::read_value_expression(&mut reader, &mut host, &mut scarce, 64).unwrap_err();
        let position = source.len() - usize::from(missing_results > 1);
        assert_eq!(error.location, Location::Query(position..position));
        assert_eq!(error.message, "result limit exceeded");
        assert_eq!(scarce.remaining_results(), Some(0));
        assert!(host.0.borrow().upgrade().is_none());
        if work_refusal {
            let mut scarce = Budget::for_program(ProgramLimits {
                work: work - 1,
                ..generous
            });
            let mut reader = zirql::ArgumentReader::new(&source, 0).unwrap();
            let error =
                zirql::read_value_expression(&mut reader, &mut host, &mut scarce, 64).unwrap_err();
            assert_eq!(error.location, Location::Query(source.len()..source.len()));
            assert_eq!(error.message, "work limit exceeded");
            assert_eq!(scarce.remaining_work(), 0);
            assert!(host.0.borrow().upgrade().is_none());
        }
    }
}
