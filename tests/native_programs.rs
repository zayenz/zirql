use std::{ops::Range, rc::Rc};

use zirql::{
    Bindings, Budget, DeclaredType, EmissionPolicy, EvaluationError, Expression, ExpressionKind,
    ExpressionStep, Host, Located, Location, NativeInput, NativeValue, OperationSignature,
    Pipeline, PredicateKind, Program, ProgramHost, ProgramLimits, ProjectionHost, QueryError,
    QueryErrorCategory, RuntimeTypeHost, Stage, Statement, Truth, checked_invoke, evaluate,
    evaluate_expression, evaluate_program,
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
fn native_sets_keep_identity_and_host_candidate_order() {
    use zirql::{NativeSetOperator as Operator, combine_native_set};

    #[derive(Debug, PartialEq, Eq, Hash)]
    struct Handle {
        owner: u8,
        index: u8,
    }
    let handle = |index| Handle { owner: 7, index };
    for (operator, expected) in [
        (Operator::Union, vec![1, 2, 3]),
        (Operator::Intersect, vec![2]),
        (Operator::Except, vec![3]),
    ] {
        let result = combine_native_set::<_, QueryError>(
            operator,
            [3, 2, 3].into_iter().map(handle),
            [1, 2].into_iter().map(handle),
            (0..20).map(|index| Ok(handle(index))),
            &mut Budget::for_program(ProgramLimits {
                stream_items: 3,
                ..limits()
            }),
            Location::Query(20..30),
            |_, _| Ok(()),
        )
        .unwrap();
        assert_eq!(result, expected.into_iter().map(handle).collect::<Vec<_>>());
    }
    let left = ["beta", "alpha", "beta"];
    let right = ["gamma", "alpha", "gamma"];
    for (word, expected) in [
        ("union", vec!["beta", "alpha", "gamma"]),
        ("intersect", vec!["alpha"]),
        ("except", vec!["beta"]),
    ] {
        let result = combine_native_set::<_, QueryError>(
            Operator::from_keyword(word).unwrap(),
            left.into_iter(),
            right.into_iter(),
            left.into_iter().chain(right).map(Ok),
            &mut Budget::for_program(limits()),
            Location::Query(20..30),
            |text, budget| Ok(budget.charge(text.len(), Location::Query(20..30))?),
        )
        .unwrap();
        assert_eq!(result, expected);
    }
    assert_eq!(Operator::from_keyword("difference"), None);
    assert_eq!(Operator::from_keyword("Union"), None);
    let empty = combine_native_set::<u8, QueryError>(
        Operator::Union,
        [].into_iter(),
        [].into_iter(),
        [],
        &mut Budget::for_program(limits()),
        Location::Query(20..30),
        |_, _| Ok(()),
    )
    .unwrap();
    assert!(empty.is_empty());
}

#[test]
fn native_sets_keep_located_limits_and_candidate_failures() {
    use zirql::{NativeSetOperator, combine_native_set};

    for (mut budget, category) in [
        (
            Budget::new(zirql::Limits {
                work: 0,
                items: 8,
                nesting: 16,
            }),
            QueryErrorCategory::WorkLimit,
        ),
        (
            Budget::for_program(ProgramLimits {
                results: 0,
                ..limits()
            }),
            QueryErrorCategory::ResultLimit,
        ),
        (
            Budget::for_program(ProgramLimits {
                stream_items: 1,
                ..limits()
            }),
            QueryErrorCategory::ItemLimit,
        ),
    ] {
        let error = combine_native_set::<_, QueryError>(
            NativeSetOperator::Union,
            [1, 2].into_iter(),
            [].into_iter(),
            [Ok(1), Ok(2)],
            &mut budget,
            Location::Query(20..30),
            |_, _| Ok(()),
        )
        .unwrap_err();
        let EvaluationError::Core(error) = error else {
            panic!("core error")
        };
        assert_eq!(error.location, Location::Query(20..30));
        assert_eq!(error.category, category);
    }
    let failure = QueryError::source(40..42, "foreign candidate");
    let error = combine_native_set::<u8, QueryError>(
        NativeSetOperator::Except,
        [].into_iter(),
        [].into_iter(),
        [Err(EvaluationError::Host(failure.clone()))],
        &mut Budget::for_program(limits()),
        Location::Query(20..30),
        |_, _| Ok(()),
    )
    .unwrap_err();
    assert_eq!(error, EvaluationError::Host(failure));
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
    side: u64,
    text: &'a str,
    range: Range<usize>,
}

#[derive(Debug)]
enum SourceValue<'a> {
    Constructed(zirql::ConstructedValue),
    Computational(std::sync::Arc<zirql::ComputationalValue<SourceValue<'a>, SourceKind>>),
    Words(Vec<Word<'a>>),
    Count(usize),
    Record(zirql::NativeCapture<SourceValue<'a>>),
    Tuple(Vec<SourceValue<'a>>),
    Set(Vec<SourceValue<'a>>),
    List(Vec<SourceValue<'a>>),
    Dictionary(Vec<(SourceValue<'a>, SourceValue<'a>)>),
    Sequence {
        values: Vec<SourceValue<'a>>,
        element: DeclaredType<SourceKind>,
    },
}

// Computational values have no language equality operation. This fixture
// compares retained wrapper identity only; native/data comparisons keep their
// existing structural meaning.
impl PartialEq for SourceValue<'_> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Constructed(a), Self::Constructed(b)) => a == b,
            (Self::Computational(a), Self::Computational(b)) => std::sync::Arc::ptr_eq(a, b),
            (Self::Words(a), Self::Words(b)) => a == b,
            (Self::Count(a), Self::Count(b)) => a == b,
            (Self::Record(a), Self::Record(b)) => a == b,
            (Self::Tuple(a), Self::Tuple(b))
            | (Self::Set(a), Self::Set(b))
            | (Self::List(a), Self::List(b)) => a == b,
            (Self::Dictionary(a), Self::Dictionary(b)) => a == b,
            (
                Self::Sequence {
                    values: a,
                    element: ae,
                },
                Self::Sequence {
                    values: b,
                    element: be,
                },
            ) => a == b && ae == be,
            _ => false,
        }
    }
}

type SourceExpression = Expression<SourceStage, (), &'static str>;

#[derive(Debug, PartialEq, Eq)]
enum SourceStage {
    Order(Box<zirql::NativeOrdering<SourceExpression>>),
    FixedPoint(Box<SourceExpression>),
    Exists(Box<SourceExpression>),
    Neighbors,
    Same,
    Invalidate,
    Prefix(&'static str),
    Pipeline(Pipeline<()>),
    Select(Box<SourceExpression>),
    Fail,
    UnknownGuard,
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
            SourceValue::Constructed(v) => {
                SourceValue::Constructed(zirql::copy_constructed_value(v, budget, location)?)
            }
            SourceValue::Computational(v) => {
                zirql::admit_computational(
                    value,
                    &v.declaration(budget, &location)?,
                    self,
                    _context,
                    budget,
                    location.clone(),
                )?;
                budget.charge(1, location.clone())?;
                budget.charge_results(1, location.clone())?;
                SourceValue::Computational(std::sync::Arc::new(
                    v.share(self, _context, budget, location)?,
                ))
            }
            SourceValue::Words(words) => {
                budget.check_items(words.len(), location.clone())?;
                budget.charge(words.len(), location.clone())?;
                budget.charge_results(words.len(), location)?;
                SourceValue::Words(
                    words
                        .iter()
                        .map(|word| Word {
                            side: word.side,
                            text: word.text,
                            range: word.range.clone(),
                        })
                        .collect(),
                )
            }
            SourceValue::Sequence { values, element } => {
                budget.charge_results(values.len() + 1, location.clone())?;
                SourceValue::Sequence {
                    values: values
                        .iter()
                        .map(|v| self.copy_value(v, _context, budget, range))
                        .collect::<Result<_, _>>()?,
                    element: element.clone(),
                }
            }
            SourceValue::Count(count) => {
                budget.charge_results(1, location)?;
                SourceValue::Count(*count)
            }
            SourceValue::Record(record) => {
                let fields = &record.fields;
                budget.charge_results(1 + fields.len(), location)?;
                let mut copied = Vec::with_capacity(fields.len());
                for (label, value) in fields {
                    budget.charge(label.len(), Location::Query(range.clone()))?;
                    copied.push((
                        label.clone(),
                        self.copy_value(value, _context, budget, range)?,
                    ));
                }
                SourceValue::Record(zirql::NativeCapture { fields: copied })
            }
            SourceValue::Dictionary(entries) => {
                budget.charge_results(1 + entries.len(), location)?;
                SourceValue::Dictionary(
                    entries
                        .iter()
                        .map(|(k, v)| {
                            Ok((
                                self.copy_value(k, _context, budget, range)?,
                                self.copy_value(v, _context, budget, range)?,
                            ))
                        })
                        .collect::<Result<_, EvaluationError<SourceFailure>>>()?,
                )
            }
            SourceValue::List(values) => {
                budget.charge_results(1 + values.len(), location)?;
                SourceValue::List(
                    values
                        .iter()
                        .map(|v| self.copy_value(v, _context, budget, range))
                        .collect::<Result<_, _>>()?,
                )
            }
            SourceValue::Set(values) => {
                budget.charge_results(1 + values.len(), location)?;
                SourceValue::Set(
                    values
                        .iter()
                        .map(|v| self.copy_value(v, _context, budget, range))
                        .collect::<Result<_, _>>()?,
                )
            }
            SourceValue::Tuple(values) => {
                budget.charge_results(values.len() + 1, location)?;
                SourceValue::Tuple(
                    values
                        .iter()
                        .map(|v| self.copy_value(v, _context, budget, range))
                        .collect::<Result<_, _>>()?,
                )
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
            SourceStage::Order(ordering) => {
                let SourceValue::Words(words) = value else {
                    return Err(
                        QueryError::query(range.clone(), "expected source selections").into(),
                    );
                };
                let words = zirql::order_native_by(
                    words,
                    ordering.order,
                    budget,
                    Location::Query(range.clone()),
                    |word, budget| {
                        let output = if let Some(selector) = &ordering.selector {
                            let input = self.copy_value(
                                &SourceValue::Words(vec![Word {
                                    side: word.side,
                                    text: word.text,
                                    range: word.range.clone(),
                                }]),
                                context,
                                budget,
                                range,
                            )?;
                            evaluate_expression(selector, self, input, context, bindings, budget)?
                        } else {
                            SourceValue::Words(vec![Word {
                                side: word.side,
                                text: word.text,
                                range: word.range.clone(),
                            }])
                        };
                        match output {
                            SourceValue::Count(count) => Ok(zirql::NativeOrderKey::Count(count)),
                            SourceValue::Words(words) if words.len() == 1 => {
                                budget
                                    .charge(words[0].text.len(), Location::Query(range.clone()))?;
                                budget.charge_results(1, Location::Query(range.clone()))?;
                                Ok(zirql::NativeOrderKey::Text(words[0].text.to_owned()))
                            }
                            _ => Err(QueryError::query(
                                range.clone(),
                                "expected one word or count",
                            )
                            .into()),
                        }
                    },
                )?;
                Ok(SourceValue::Words(words))
            }
            SourceStage::Exists(body) => {
                let SourceValue::Words(words) = value else {
                    return Err(QueryError::query(range.clone(), "expected words").into());
                };
                Ok(SourceValue::Words(zirql::filter_native_exists(
                    words,
                    budget,
                    Location::Query(range.clone()),
                    |word, budget| {
                        let input = self.copy_value(
                            &SourceValue::Words(vec![Word {
                                side: word.side,
                                text: word.text,
                                range: word.range.clone(),
                            }]),
                            context,
                            budget,
                            range,
                        )?;
                        match evaluate_expression(body, self, input, context, bindings, budget)? {
                            SourceValue::Words(words) => Ok(words),
                            _ => Err(QueryError::query(range.clone(), "expected words").into()),
                        }
                    },
                )?))
            }
            SourceStage::FixedPoint(body) => zirql::evaluate_native_fixed_point(
                value,
                context,
                budget,
                Location::Query(range.clone()),
                |current, context, budget| {
                    let input = self.copy_value(current, context, budget, range)?;
                    evaluate_expression(body, self, input, context, bindings, budget)
                },
                |current, context, budget| self.copy_value(current, context, budget, range),
                |left, right, _, budget| {
                    let (SourceValue::Words(left), SourceValue::Words(right)) = (left, right)
                    else {
                        return Err(QueryError::query(range.clone(), "expected words").into());
                    };
                    zirql::ordered_native_equal(
                        left,
                        right,
                        budget,
                        Location::Query(range.clone()),
                        |left, right, _| Ok(left == right),
                    )
                },
            ),
            SourceStage::Invalidate => {
                context.source = &context.source[1..];
                Ok(SourceValue::Constructed(zirql::ConstructedValue::Boolean(
                    true,
                )))
            }
            SourceStage::Same => {
                let a = bindings.get("candidate", range.clone())?;
                let b = bindings.get("target", range.clone())?;
                let a_id = <Self as zirql::FunctionHost<_>>::native_identity(
                    self,
                    a,
                    &SourceKind::Words,
                    context,
                    budget,
                    range,
                )?;
                let b_id = <Self as zirql::FunctionHost<_>>::native_identity(
                    self,
                    b,
                    &SourceKind::Words,
                    context,
                    budget,
                    range,
                )?;
                budget.charge(4, Location::Query(range.clone()))?;
                Ok(SourceValue::Constructed(zirql::ConstructedValue::Boolean(
                    a_id == b_id,
                )))
            }
            SourceStage::Neighbors => {
                let SourceValue::Words(words) = value else {
                    return Err(QueryError::query(range.clone(), "expected node").into());
                };
                let [word] = words.as_slice() else {
                    return Err(QueryError::query(range.clone(), "expected one node").into());
                };
                let offsets: &[usize] = match word.text {
                    "A" => &[2, 4],
                    "B" => &[6],
                    "C" => &[2],
                    "D" => &[0],
                    "E" => &[],
                    _ => return Err(QueryError::query(range.clone(), "unknown graph node").into()),
                };
                budget.charge(offsets.len() + 1, Location::Query(range.clone()))?;
                budget.charge_results(offsets.len() + 1, Location::Query(range.clone()))?;
                Ok(SourceValue::Sequence {
                    values: offsets
                        .iter()
                        .map(|&i| {
                            SourceValue::Words(vec![Word {
                                side: word.side,
                                text: &context.source[i..i + 1],
                                range: i..i + 1,
                            }])
                        })
                        .collect(),
                    element: DeclaredType::Native(SourceKind::Words),
                })
            }
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
            SourceStage::UnknownGuard => Err(QueryError::query(
                range.clone(),
                "native guard remained Unknown",
            )
            .with_category(QueryErrorCategory::UnknownPredicate)
            .into()),
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
        Ok(SourceValue::Record(zirql::NativeCapture { fields }))
    }

    fn capture_record<'v>(
        &self,
        value: &'v SourceValue<'a>,
    ) -> Option<&'v zirql::NativeCapture<SourceValue<'a>>> {
        match value {
            SourceValue::Record(v) => Some(v),
            _ => None,
        }
    }
    fn tuple_view<'v>(&self, value: &'v SourceValue<'a>) -> Option<&'v [SourceValue<'a>]> {
        match value {
            SourceValue::Tuple(v) => Some(v),
            _ => None,
        }
    }

    fn positional_view<'v>(&self, value: &'v SourceValue<'a>) -> Option<&'v [SourceValue<'a>]> {
        match value {
            SourceValue::Sequence { values, .. } | SourceValue::List(values) => Some(values),
            _ => None,
        }
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
        words.push(Word {
            side: 0,
            text,
            range,
        });
    }
    SourceValue::Words(words)
}

#[derive(Clone, Debug, PartialEq)]
enum SourceKind {
    Constructed(std::sync::Arc<Vec<zirql::ValueType>>),
    Computational(Box<DeclaredType<SourceKind>>),
    Words,
    Count,
    Record(zirql::NativeCapture<SourceKind>),
    Tuple(Vec<SourceKind>),
    Set,
    List,
    Dictionary,
    Sequence(Box<DeclaredType<SourceKind>>),
}
fn source_declaration(kind: &SourceKind) -> DeclaredType<SourceKind> {
    match kind {
        SourceKind::Constructed(types) => match types.as_slice() {
            [zirql::ValueType::Integer] => DeclaredType::Integer,
            [zirql::ValueType::Boolean] => DeclaredType::Boolean,
            [zirql::ValueType::Float] => DeclaredType::Float,
            [zirql::ValueType::String] => DeclaredType::String,
            _ => panic!("unsupported fixture scalar declaration"),
        },
        SourceKind::Record(record) => DeclaredType::Record(
            record
                .fields
                .iter()
                .map(|(s, k)| (s.clone(), source_declaration(k)))
                .collect(),
        ),
        SourceKind::Sequence(element) => DeclaredType::Sequence(element.clone()),
        SourceKind::Computational(d) => *d.clone(),
        SourceKind::Tuple(children) => {
            DeclaredType::Tuple(children.iter().map(source_declaration).collect())
        }
        other => DeclaredType::Native(other.clone()),
    }
}

impl<'a> RuntimeTypeHost<SourceContext<'a>> for SourceHost {
    type Kind = SourceKind;
    fn constructed_value<'v>(
        &self,
        value: &'v SourceValue<'a>,
    ) -> Option<&'v zirql::ConstructedValue> {
        match value {
            SourceValue::Constructed(v) => Some(v),
            _ => None,
        }
    }

    fn admit_native(
        &self,
        value: &SourceValue<'a>,
        expected: &DeclaredType<SourceKind>,
        context: &mut SourceContext<'a>,
        budget: &mut Budget,
        location: Location,
    ) -> Result<(), EvaluationError<SourceFailure>> {
        if zirql::admit_computational(value, expected, self, context, budget, location.clone())? {
            return Ok(());
        }
        if let SourceValue::Constructed(v) = value {
            return Ok(zirql::admit_constructed(v, expected, budget, location)?);
        }
        if let DeclaredType::Optional(d) = expected {
            return self.admit_native(value, d, context, budget, location);
        }
        match (expected, value) {
            (
                DeclaredType::Native(
                    kind @ (SourceKind::Record(_) | SourceKind::Tuple(_) | SourceKind::Sequence(_)),
                ),
                _,
            ) => {
                return self.admit_native(
                    value,
                    &source_declaration(kind),
                    context,
                    budget,
                    location,
                );
            }
            (DeclaredType::Sequence(ty), SourceValue::Sequence { values, element })
                if ty.as_ref() == element =>
            {
                for value in values {
                    self.admit_native(value, ty, context, budget, location.clone())?;
                }
                return Ok(());
            }
            (DeclaredType::Tuple(types), SourceValue::Tuple(values))
                if types.len() == values.len() =>
            {
                for (value, ty) in values.iter().zip(types) {
                    self.admit_native(value, ty, context, budget, location.clone())?;
                }
                return Ok(());
            }
            (DeclaredType::Record(types), SourceValue::Record(values))
                if types.len() == values.fields.len() =>
            {
                for (label, ty) in types {
                    let value = values.get(&Located::new(label.as_str(), 0..1), budget)?;
                    self.admit_native(value, ty, context, budget, location.clone())?;
                }
                return Ok(());
            }
            (DeclaredType::Native(SourceKind::Set), SourceValue::Set(values))
            | (DeclaredType::Native(SourceKind::List), SourceValue::List(values)) => {
                for value in values {
                    let kind = zirql::ProjectionHost::kind(
                        self,
                        value,
                        budget,
                        match &location {
                            Location::Query(r) | Location::Source(r) => r,
                        },
                    )?;
                    self.admit_native(
                        value,
                        &source_declaration(&kind),
                        context,
                        budget,
                        location.clone(),
                    )?;
                }
                return Ok(());
            }
            (DeclaredType::Native(SourceKind::Dictionary), SourceValue::Dictionary(entries)) => {
                for (key, value) in entries {
                    for value in [key, value] {
                        let kind = zirql::ProjectionHost::kind(
                            self,
                            value,
                            budget,
                            match &location {
                                Location::Query(r) | Location::Source(r) => r,
                            },
                        )?;
                        self.admit_native(
                            value,
                            &source_declaration(&kind),
                            context,
                            budget,
                            location.clone(),
                        )?;
                    }
                }
                return Ok(());
            }
            (DeclaredType::Dictionary(key_type, value_type), SourceValue::Dictionary(entries)) => {
                for (key, value) in entries {
                    self.admit_native(key, key_type, context, budget, location.clone())?;
                    self.admit_native(value, value_type, context, budget, location.clone())?;
                }
                return Ok(());
            }
            (DeclaredType::Native(SourceKind::Count), SourceValue::Count(_)) => return Ok(()),
            _ => {}
        }
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
    fn set_kind(
        &self,
        left: &SourceKind,
        right: &SourceKind,
        _: &mut Budget,
        range: &Range<usize>,
    ) -> Result<SourceKind, QueryError> {
        if left != right || !matches!(left, SourceKind::Words) {
            return Err(QueryError::query(
                range.clone(),
                "sets require word identities",
            ));
        }
        Ok(left.clone())
    }

    fn kind(
        &self,
        value: &SourceValue<'a>,
        budget: &mut Budget,
        range: &Range<usize>,
    ) -> Result<SourceKind, QueryError> {
        budget.charge(1, Location::Query(range.clone()))?;
        Ok(match value {
            SourceValue::Constructed(v) => {
                SourceKind::Constructed(std::sync::Arc::new(vec![zirql::constructed_value_type(
                    v,
                    budget,
                    Location::Query(range.clone()),
                )?]))
            }
            SourceValue::Computational(v) => SourceKind::Computational(Box::new(
                v.declaration(budget, &Location::Query(range.clone()))?,
            )),
            SourceValue::Sequence { element, .. } => {
                SourceKind::Sequence(Box::new(element.clone()))
            }
            SourceValue::Set(_) => SourceKind::Set,
            SourceValue::List(_) => SourceKind::List,
            SourceValue::Dictionary(_) => SourceKind::Dictionary,
            SourceValue::Words(_) => SourceKind::Words,
            SourceValue::Count(_) => SourceKind::Count,
            SourceValue::Record(record) => SourceKind::Record(zirql::NativeCapture {
                fields: record
                    .fields
                    .iter()
                    .map(|(s, v)| Ok((s.clone(), self.kind(v, budget, range)?)))
                    .collect::<Result<_, QueryError>>()?,
            }),
            SourceValue::Tuple(values) => SourceKind::Tuple(
                values
                    .iter()
                    .map(|v| self.kind(v, budget, range))
                    .collect::<Result<_, _>>()?,
            ),
        })
    }
    fn scalar_types(
        &self,
        kind: &SourceKind,
        budget: &mut Budget,
        range: &Range<usize>,
    ) -> Result<Option<Vec<zirql::ValueType>>, QueryError> {
        match kind {
            SourceKind::Constructed(types) => Ok(Some(zirql::copy_value_types(
                types,
                budget,
                Location::Query(range.clone()),
            )?)),
            _ => Ok(None),
        }
    }
    fn capture_record_kind<'k>(
        &self,
        kind: &'k SourceKind,
    ) -> Option<&'k zirql::NativeCapture<SourceKind>> {
        match kind {
            SourceKind::Record(v) => Some(v),
            _ => None,
        }
    }
    fn tuple_view_kind<'k>(&self, kind: &'k SourceKind) -> Option<&'k [SourceKind]> {
        match kind {
            SourceKind::Tuple(v) => Some(v),
            _ => None,
        }
    }
    fn positional_view_kind(
        &self,
        kind: &SourceKind,
        budget: &mut Budget,
        range: &Range<usize>,
    ) -> Result<Option<SourceKind>, QueryError> {
        match kind {
            SourceKind::Sequence(element) => {
                <Self as zirql::FunctionHost<_>>::declaration_kind(self, element, budget, range)
                    .map(Some)
            }
            _ => Ok(None),
        }
    }
    fn pattern_constructed<'v>(
        &self,
        value: &'v SourceValue<'a>,
    ) -> Option<&'v zirql::ConstructedValue> {
        <Self as zirql::RuntimeTypeHost<_>>::constructed_value(self, value)
    }
    fn constructed_kind(&self, types: Vec<zirql::ValueType>) -> SourceKind {
        SourceKind::Constructed(std::sync::Arc::new(types))
    }
    fn stage_kind(
        &self,
        stage: &SourceStage,
        _: &SourceKind,
        _: &std::collections::BTreeMap<String, SourceKind>,
        _: &mut Budget,
        _: &Range<usize>,
    ) -> Result<SourceKind, QueryError> {
        Ok(match stage {
            SourceStage::Neighbors => {
                SourceKind::Sequence(Box::new(DeclaredType::Native(SourceKind::Words)))
            }
            SourceStage::Same | SourceStage::Invalidate => {
                self.constructed_kind(vec![zirql::ValueType::Boolean])
            }
            _ => SourceKind::Words,
        })
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
    fn subject_kind(
        &self,
        kind: &SourceKind,
        budget: &mut Budget,
        range: &Range<usize>,
    ) -> Result<SourceKind, QueryError> {
        match kind {
            SourceKind::Sequence(element) => {
                <Self as zirql::FunctionHost<_>>::declaration_kind(self, element, budget, range)
            }
            _ => Ok(kind.clone()),
        }
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
        value: SourceValue<'a>,
        _: &mut SourceContext<'a>,
        budget: &mut Budget,
        r: &Range<usize>,
    ) -> Result<zirql::ConstructedValue, EvaluationError<SourceFailure>> {
        match value {
            SourceValue::Constructed(v) => Ok(v),
            SourceValue::Computational(_) => Err(runtime_refusal(
                "unsupported computational value rendering",
                budget,
                Location::Query(r.clone()),
            )?
            .into()),
            _ => Err(QueryError::query(r.clone(), "native-only fixture").into()),
        }
    }
    fn lift_constructed(
        &self,
        value: zirql::ConstructedValue,
        _: Vec<zirql::ValueType>,
        _: &mut SourceContext<'a>,
        _: &mut Budget,
        _r: &Range<usize>,
    ) -> Result<SourceValue<'a>, EvaluationError<SourceFailure>> {
        Ok(SourceValue::Constructed(value))
    }
}
impl<'a> zirql::FunctionHost<SourceContext<'a>> for SourceHost {
    fn construct_list(
        &self,
        values: Vec<SourceValue<'a>>,
        context: &mut SourceContext<'a>,
        budget: &mut Budget,
        range: &Range<usize>,
    ) -> Result<SourceValue<'a>, EvaluationError<SourceFailure>> {
        let kinds = values
            .iter()
            .map(|value| zirql::ProjectionHost::kind(self, value, budget, range))
            .collect::<Result<Vec<_>, _>>()?;
        if let Some(first) = kinds.first()
            && kinds.iter().all(|kind| kind == first)
        {
            return self.construct_sequence(
                values,
                &source_declaration(first),
                context,
                budget,
                range,
            );
        }
        budget.charge_results(1, Location::Query(range.clone()))?;
        Ok(SourceValue::List(values))
    }

    fn set_view<'v>(&self, value: &'v SourceValue<'a>) -> Option<&'v [SourceValue<'a>]> {
        match value {
            SourceValue::Set(values) => Some(values),
            _ => None,
        }
    }
    fn dictionary_view<'v>(
        &self,
        value: &'v SourceValue<'a>,
    ) -> Option<&'v [(SourceValue<'a>, SourceValue<'a>)]> {
        match value {
            SourceValue::Dictionary(entries) => Some(entries),
            _ => None,
        }
    }
    fn construct_set(
        &self,
        values: Vec<SourceValue<'a>>,
        _: &mut SourceContext<'a>,
        budget: &mut Budget,
        range: &Range<usize>,
    ) -> Result<SourceValue<'a>, EvaluationError<SourceFailure>> {
        budget.charge_results(1, Location::Query(range.clone()))?;
        Ok(SourceValue::Set(values))
    }
    fn construct_dictionary(
        &self,
        entries: Vec<(SourceValue<'a>, SourceValue<'a>)>,
        _: &mut SourceContext<'a>,
        budget: &mut Budget,
        range: &Range<usize>,
    ) -> Result<SourceValue<'a>, EvaluationError<SourceFailure>> {
        budget.charge_results(1, Location::Query(range.clone()))?;
        Ok(SourceValue::Dictionary(entries))
    }

    fn prepare_computational(
        &self,
        d: Option<&DeclaredType<SourceKind>>,
        budget: &mut Budget,
        range: &Range<usize>,
    ) -> Result<(), QueryError> {
        budget.charge(1, Location::Query(range.clone()))?;
        if let Some(d) = d {
            self.declaration_kind(d, budget, range)?;
        }
        Ok(())
    }
    fn retains_native_admission(&self) -> bool {
        true
    }
    fn computational_value<'v>(
        &self,
        value: &'v SourceValue<'a>,
    ) -> Option<&'v zirql::ComputationalValue<SourceValue<'a>, SourceKind>> {
        match value {
            SourceValue::Computational(v) => Some(v),
            _ => None,
        }
    }
    fn lift_computational(
        &self,
        value: zirql::ComputationalValue<SourceValue<'a>, SourceKind>,
        _: &mut SourceContext<'a>,
        budget: &mut Budget,
        range: &Range<usize>,
    ) -> Result<SourceValue<'a>, EvaluationError<SourceFailure>> {
        budget.charge(1, Location::Query(range.clone()))?;
        budget.charge_results(1, Location::Query(range.clone()))?;
        Ok(SourceValue::Computational(std::sync::Arc::new(value)))
    }
    fn construct_tuple(
        &self,
        values: Vec<SourceValue<'a>>,
        _: &mut SourceContext<'a>,
        budget: &mut Budget,
        range: &Range<usize>,
    ) -> Result<SourceValue<'a>, EvaluationError<SourceFailure>> {
        budget.charge(values.len() + 1, Location::Query(range.clone()))?;
        budget.charge_results(values.len() + 1, Location::Query(range.clone()))?;
        Ok(SourceValue::Tuple(values))
    }
    fn resolve_type_name(
        &self,
        name: Located<&str>,
        budget: &mut Budget,
    ) -> Result<DeclaredType<SourceKind>, QueryError> {
        budget.charge(name.value.len() + 1, Location::Query(name.range.clone()))?;
        if name.value == "Word" {
            Ok(DeclaredType::Native(SourceKind::Words))
        } else {
            Err(QueryError::query(name.range, "unknown native type"))
        }
    }
    fn worklist_context(
        &self,
        context: &mut SourceContext<'a>,
        budget: &mut Budget,
        range: &Range<usize>,
    ) -> Result<zirql::NativeContextId, EvaluationError<SourceFailure>> {
        budget.charge(1, Location::Query(range.clone()))?;
        Ok(zirql::NativeContextId([
            context.source.as_ptr() as usize as u64,
            context.source.len() as u64,
            0,
        ]))
    }
    fn native_identity(
        &self,
        value: &SourceValue<'a>,
        _: &SourceKind,
        _: &mut SourceContext<'a>,
        budget: &mut Budget,
        range: &Range<usize>,
    ) -> Result<zirql::NativeIdentity, EvaluationError<SourceFailure>> {
        budget.charge(4, Location::Query(range.clone()))?;
        let SourceValue::Words(words) = value else {
            return Err(QueryError::query(range.clone(), "expected node").into());
        };
        let [word] = words.as_slice() else {
            return Err(QueryError::query(range.clone(), "expected one node").into());
        };
        Ok(zirql::NativeIdentity([
            word.text.as_ptr() as usize as u64 - word.range.start as u64,
            word.side,
            word.range.start as u64,
            word.range.end as u64,
        ]))
    }

    fn sequence_items(
        &self,
        value: &SourceValue<'a>,
        element: &DeclaredType<SourceKind>,
        context: &mut SourceContext<'a>,
        budget: &mut Budget,
        range: &Range<usize>,
    ) -> Result<Vec<SourceValue<'a>>, EvaluationError<SourceFailure>> {
        let SourceValue::Sequence {
            values,
            element: actual,
        } = value
        else {
            return Err(QueryError::query(range.clone(), "expected native sequence").into());
        };
        if element != actual {
            return Err(
                QueryError::query(range.clone(), "wrong native element declaration").into(),
            );
        }
        budget.check_items(values.len(), Location::Query(range.clone()))?;
        values
            .iter()
            .map(|v| self.copy_value(v, context, budget, range))
            .collect()
    }
    fn construct_sequence(
        &self,
        values: Vec<SourceValue<'a>>,
        element: &DeclaredType<SourceKind>,
        _: &mut SourceContext<'a>,
        budget: &mut Budget,
        range: &Range<usize>,
    ) -> Result<SourceValue<'a>, EvaluationError<SourceFailure>> {
        budget.charge(1 + values.len(), Location::Query(range.clone()))?;
        budget.charge_results(1, Location::Query(range.clone()))?;
        Ok(SourceValue::Sequence {
            values,
            element: element.clone(),
        })
    }

    fn declaration_kind(
        &self,
        declaration: &DeclaredType<SourceKind>,
        budget: &mut Budget,
        range: &Range<usize>,
    ) -> Result<SourceKind, QueryError> {
        budget.charge(1, Location::Query(range.clone()))?;
        Ok(match declaration {
            DeclaredType::Boolean => self.constructed_kind(vec![zirql::ValueType::Boolean]),
            DeclaredType::Integer => self.constructed_kind(vec![zirql::ValueType::Integer]),
            DeclaredType::Float => self.constructed_kind(vec![zirql::ValueType::Float]),
            DeclaredType::String => self.constructed_kind(vec![zirql::ValueType::String]),
            DeclaredType::Native(kind) => kind.clone(),
            DeclaredType::Sequence(element) => SourceKind::Sequence(element.clone()),
            d @ (DeclaredType::Queue(_) | DeclaredType::IdentitySet(_)) => {
                SourceKind::Computational(Box::new(d.clone()))
            }
            DeclaredType::Optional(d) => self.declaration_kind(d, budget, range)?,
            DeclaredType::Tuple(children) => SourceKind::Tuple(
                children
                    .iter()
                    .map(|d| self.declaration_kind(d, budget, range))
                    .collect::<Result<_, _>>()?,
            ),
            DeclaredType::Record(fields) => SourceKind::Record(zirql::NativeCapture {
                fields: fields
                    .iter()
                    .map(|(s, d)| Ok((s.clone(), self.declaration_kind(d, budget, range)?)))
                    .collect::<Result<_, QueryError>>()?,
            }),
            _ => SourceKind::Words,
        })
    }
    fn is_operation_name(&self, name: &str) -> bool {
        name == "prefix"
    }
}
#[test]
fn named_native_accumulators_keep_handles_empty_schema_and_failed_context() {
    use zirql::{
        FunctionDeclaration, FunctionLimits, FunctionParameter, FunctionRequest,
        ValueExpressionKind as K,
    };
    let source = String::from("alpha beta");
    let foreign = String::from("alpha beta");
    let mut context = SourceContext {
        source: &source,
        prefix: "",
        emissions: vec![],
    };
    let mut budget = Budget::for_program(ProgramLimits {
        work: 1_000_000,
        results: 100_000,
        ..limits()
    });
    let element = DeclaredType::Native(SourceKind::Words);
    let mut step = FunctionDeclaration {
        name: at("last".into(), 1),
        parameters: ["state", "item"]
            .into_iter()
            .map(|name| {
                at(
                    FunctionParameter {
                        name: at(name.into(), 2),
                        declaration: element.clone(),
                    },
                    2,
                )
            })
            .collect(),
        result: at(element.clone(), 3),
        bindings: vec![],
        value: at(K::Name("item".into()), 4),
    };
    let native = |name: &str| {
        at(
            K::NativeValue(Box::new(at(ExpressionKind::Binding(name.into()), 12))),
            12,
        )
    };
    let entry = |scan, items: &str| {
        at(
            if scan {
                K::Scan {
                    items: Box::new(native(items)),
                    initial: Box::new(native("initial")),
                    step: at("last".into(), 15),
                }
            } else {
                K::Fold {
                    items: Box::new(native(items)),
                    initial: Box::new(native("initial")),
                    step: at("last".into(), 15),
                }
            },
            11,
        )
    };
    let words = |range: Range<usize>| {
        SourceValue::Words(vec![Word {
            side: 0,
            text: &source[range.clone()],
            range,
        }])
    };
    let bindings = Bindings::new([
        (
            at("items".into(), 0),
            SourceValue::Sequence {
                values: vec![words(0..5), words(6..10)],
                element: element.clone(),
            },
        ),
        (
            at("empty".into(), 0),
            SourceValue::Sequence {
                values: vec![],
                element: element.clone(),
            },
        ),
        (
            at("wrong_empty".into(), 0),
            SourceValue::Sequence {
                values: vec![],
                element: DeclaredType::Native(SourceKind::Count),
            },
        ),
        (at("initial".into(), 0), source_input(&source, &mut budget)),
    ])
    .unwrap();
    for scan in [false, true] {
        let result = FunctionRequest::prepare(
            std::slice::from_ref(&step),
            &SourceHost,
            &mut context,
            &mut budget,
            FunctionLimits { active_depth: 1 },
        )
        .unwrap()
        .evaluate(&entry(scan, "items"), &bindings)
        .unwrap();
        let states = if scan {
            let SourceValue::Sequence {
                values,
                element: actual,
            } = result
            else {
                panic!("native scan")
            };
            assert_eq!(actual, element);
            values
        } else {
            vec![result]
        };
        for (index, value) in states.iter().enumerate() {
            let range = if scan && index == 0 { 0..5 } else { 6..10 };
            let SourceValue::Words(words) = value else {
                panic!("native state")
            };
            assert_eq!(words[0].range, range);
            assert_eq!(words[0].text.as_ptr(), source[range.clone()].as_ptr());
        }
    }
    let empty = FunctionRequest::prepare(
        std::slice::from_ref(&step),
        &SourceHost,
        &mut context,
        &mut budget,
        FunctionLimits { active_depth: 1 },
    )
    .unwrap()
    .evaluate(&entry(true, "empty"), &bindings)
    .unwrap();
    assert!(
        matches!(empty, SourceValue::Sequence { values, element: actual } if values.is_empty() && actual == element)
    );
    assert!(
        FunctionRequest::prepare(
            std::slice::from_ref(&step),
            &SourceHost,
            &mut context,
            &mut budget,
            FunctionLimits { active_depth: 1 }
        )
        .unwrap()
        .evaluate(&entry(false, "wrong_empty"), &bindings)
        .is_err()
    );
    context.source = &foreign;
    let error = FunctionRequest::prepare(
        std::slice::from_ref(&step),
        &SourceHost,
        &mut context,
        &mut budget,
        FunctionLimits { active_depth: 1 },
    )
    .unwrap()
    .evaluate(&entry(false, "items"), &bindings)
    .unwrap_err();
    assert!(matches!(
        error.error,
        EvaluationError::Host(SourceFailure::Source(_))
    ));
    context.source = &source;
    FunctionRequest::prepare(
        std::slice::from_ref(&step),
        &SourceHost,
        &mut context,
        &mut budget,
        FunctionLimits { active_depth: 1 },
    )
    .unwrap()
    .evaluate(&entry(false, "empty"), &bindings)
    .unwrap();
    step.result = at(DeclaredType::Native(SourceKind::Count), 3);
    let incompatible = FunctionRequest::prepare(
        std::slice::from_ref(&step),
        &SourceHost,
        &mut context,
        &mut budget,
        FunctionLimits { active_depth: 1 },
    )
    .unwrap()
    .evaluate(&entry(false, "empty"), &bindings)
    .unwrap_err();
    assert!(
        matches!(incompatible.error, EvaluationError::Core(ref error)
        if error.location == Location::Query(15..16))
    );
    step.result = at(element.clone(), 3);
    step.value = at(
        K::NativeValue(Box::new(sequence(
            at(ExpressionKind::Binding("item".into()), 40),
            vec![at(ExpressionStep::Stage(SourceStage::Fail), 41)],
        ))),
        40,
    );
    let failed = FunctionRequest::prepare(
        std::slice::from_ref(&step),
        &SourceHost,
        &mut context,
        &mut budget,
        FunctionLimits { active_depth: 1 },
    )
    .unwrap()
    .evaluate(&entry(true, "items"), &bindings)
    .unwrap_err();
    assert!(
        matches!(failed.error, EvaluationError::Host(SourceFailure::Source(ref range))
        if *range == (0..source.len()))
    );
    assert_eq!(failed.calls[0].function, "last");
    assert_eq!(failed.calls[0].range, 15..16);
    FunctionRequest::prepare(
        std::slice::from_ref(&step),
        &SourceHost,
        &mut context,
        &mut budget,
        FunctionLimits { active_depth: 1 },
    )
    .unwrap()
    .evaluate(&entry(false, "empty"), &bindings)
    .unwrap();
    let SourceValue::Words(initial) = bindings.get("initial", 0..1).unwrap() else {
        panic!("initial alias")
    };
    assert_eq!(initial.len(), 2);
    assert_eq!(initial[0].text.as_ptr(), source.as_ptr());
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
                side: 0,
                text: &source[6..11],
                range: 6..11
            },
            Word {
                side: 0,
                text: &source[12..17],
                range: 12..17
            },
        ]
    );
    assert_eq!(words[0].text.as_ptr(), source[6..11].as_ptr());
    let SourceValue::Record(record) = &context.emissions[2].1 else {
        panic!("expected native report")
    };
    let fields = &record.fields;
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
        SignatureQuery {
            declaration: Box<zirql::PatternFunctionDeclaration<Query, Kind>>,
            value: Box<zirql::ValueExpression<Query>>,
        },
        Argument {
            value: Box<zirql::ValueExpression<Query>>,
            selected: bool,
        },
        Fail,
        Edit,
        Saved(String),
        Projection(Box<Projection<Query>>),
        Match(Box<NativeMatch<Query, Guard>>),
        Paths(Box<NativePaths<Stage, Guard>>),
        Neighbors,
    }
    #[derive(Debug, PartialEq)]
    enum Guard {
        True,
        False,
        Unknown,
        Expression(Arc<zirql::ValueExpression<Query>>),
        Count(Arc<zirql::ValueExpression<Query>>),
    }
    impl Guard {
        fn truth(&self) -> Truth {
            match self {
                Self::True => Truth::True,
                Self::False => Truth::False,
                Self::Unknown => Truth::Unknown,
                Self::Expression(_) | Self::Count(_) => {
                    unreachable!("expression guard uses the prepared runner")
                }
            }
        }
    }
    struct Host;

    fn guard_values(predicate: &Predicate<Guard>) -> Vec<&zirql::ValueExpression<Query>> {
        let mut values = Vec::new();
        let mut pending = vec![predicate];
        while let Some(predicate) = pending.pop() {
            match &predicate.value {
                zirql::PredicateKind::Leaf(Guard::Expression(body) | Guard::Count(body)) => {
                    values.push(body.as_ref())
                }
                zirql::PredicateKind::Leaf(_) => {}
                zirql::PredicateKind::Not(body) | zirql::PredicateKind::Group(body) => {
                    pending.push(body)
                }
                zirql::PredicateKind::And(bodies) | zirql::PredicateKind::Or(bodies) => {
                    pending.extend(bodies)
                }
            }
        }
        values
    }
    fn match_argument(source: &Query) -> Option<&zirql::ValueExpression<Query>> {
        if let ExpressionKind::Sequence { steps, .. } = &source.value
            && let [step] = steps.as_slice()
            && let ExpressionStep::Stage(Stage::NativeArgument(value)) = &step.value
        {
            return Some(value);
        }
        None
    }

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
                Stage::SignatureQuery { .. } => unreachable!("signature-only preparation control"),
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
                        Value::Captures { rows, .. } => rows.len(),
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
            if let Stage::Projection(projection) = stage {
                budget.charge(projection.fields.len(), Location::Query(range.clone()))?;
                return Ok(projection
                    .fields
                    .iter()
                    .map(|(_, value)| (value, Kind::Subjects))
                    .collect());
            }
            if let Stage::Match(matching) = stage {
                let mut values = Vec::new();
                for clause in &matching.clauses {
                    let source = match &clause.value {
                        zirql::MatchClause::From { source, .. }
                        | zirql::MatchClause::Bind { value: source, .. } => source,
                        zirql::MatchClause::Where { predicate } => {
                            values.extend(
                                guard_values(predicate)
                                    .into_iter()
                                    .map(|body| (body, Kind::Subjects)),
                            );
                            continue;
                        }
                    };
                    if let Some(value) = match_argument(source) {
                        values.push((value, Kind::Subjects));
                    }
                }
                budget.charge(values.len(), Location::Query(range.clone()))?;
                return Ok(values);
            }
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
        fn scoped_value_arguments<'s>(
            &self,
            stage: &'s Stage,
            input: &Kind,
            _: &BTreeMap<String, Kind>,
            budget: &mut Budget,
            range: &Range<usize>,
        ) -> Result<Vec<zirql::ScopedNativeArgument<'s, Query, Kind>>, QueryError> {
            if let Stage::Projection(projection) = stage {
                let kind = self.subject_kind(input, budget, range)?;
                budget.charge(projection.fields.len(), Location::Query(range.clone()))?;
                return Ok(projection
                    .fields
                    .iter()
                    .map(|(_, expression)| zirql::ScopedNativeArgument {
                        expression,
                        relative: Some(kind.clone()),
                        locals: vec![(projection.subject.clone(), kind.clone())],
                    })
                    .collect());
            }
            if let Stage::Match(matching) = stage {
                let mut locals: Vec<(Located<String>, Kind)> = Vec::new();
                let mut arguments = Vec::new();
                for clause in &matching.clauses {
                    let (name, source) = match &clause.value {
                        zirql::MatchClause::From { name, source }
                        | zirql::MatchClause::Bind {
                            name,
                            value: source,
                        } => (name, source),
                        zirql::MatchClause::Where { predicate } => {
                            for expression in guard_values(predicate) {
                                arguments.push(zirql::ScopedNativeArgument {
                                    expression,
                                    relative: Some(input.clone()),
                                    locals: locals.clone(),
                                });
                            }
                            continue;
                        }
                    };
                    if let Some(expression) = match_argument(source) {
                        budget.charge(locals.len() + 1, Location::Query(source.range.clone()))?;
                        arguments.push(zirql::ScopedNativeArgument {
                            expression,
                            relative: None,
                            locals: locals.clone(),
                        });
                    }
                    locals.push((name.clone(), Kind::Subjects));
                }
                return Ok(arguments);
            }
            Ok(self
                .value_arguments(stage, budget, range)?
                .into_iter()
                .map(|(expression, kind)| zirql::ScopedNativeArgument {
                    expression,
                    relative: (!self.is_explicit_source(stage)).then_some(kind),
                    locals: vec![],
                })
                .collect())
        }
        fn evaluate_stage_with_arguments(
            &self,
            stage: &Stage,
            input: Value,
            arguments: &mut dyn zirql::NativeArguments<(), Self>,
            budget: &mut Budget,
            range: &Range<usize>,
        ) -> Result<Value, EvaluationError<QueryError>> {
            if let Stage::Projection(projection) = stage {
                return arguments
                    .evaluate_projection(projection, input, budget)
                    .map(|result| Value::Constructed(result.value));
            }
            if let Stage::Match(matching) = stage {
                let (context, bindings) = arguments.parts();
                let input_kind = self.kind(&input, budget, range)?;
                let mut scope = BTreeMap::new();
                for (name, value) in bindings.iter() {
                    scope.insert(name.to_owned(), self.kind(value, budget, range)?);
                }
                let schema = Arc::new(zirql::match_types(
                    matching,
                    self,
                    &input_kind,
                    &scope,
                    budget,
                )?);
                let _ = context;
                let rows =
                    zirql::evaluate_match_with_arguments(matching, self, input, arguments, budget)?;
                return Ok(Value::Captures { rows, schema });
            }
            if let Stage::Paths(paths) = stage {
                let input_kind = self.kind(&input, budget, range)?;
                let schema = Arc::new(zirql::path_types(
                    paths,
                    self,
                    &input_kind,
                    &BTreeMap::new(),
                    budget,
                )?);
                let rows =
                    zirql::evaluate_paths_with_arguments(paths, self, input, arguments, budget)?;
                budget.charge_results(1, Location::Query(range.clone()))?;
                return Ok(Value::Captures { rows, schema });
            }
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
                Stage::SignatureQuery { declaration, value } => {
                    let parameters = declaration
                        .parameters
                        .iter()
                        .map(|parameter| {
                            Located::new(
                                zirql::PatternParameterRef {
                                    pattern: &parameter.value.pattern,
                                    declaration: parameter.value.declaration.clone(),
                                },
                                parameter.range.clone(),
                            )
                        })
                        .collect::<Vec<_>>();
                    zirql::pattern_function_value_kind_borrowed(
                        [zirql::PatternFunctionDeclarationRef {
                            name: &declaration.name,
                            parameters: &parameters,
                            result: &declaration.result,
                            value: &declaration.value,
                        }],
                        self,
                        value,
                        scope,
                        Some(input.clone()),
                        budget,
                    )
                }
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
                Stage::Count
                    if matches!(input, Kind::Subjects | Kind::Strings | Kind::Captures(_)) =>
                {
                    Ok(Kind::Count)
                }
                Stage::Fail | Stage::Edit => Ok(input.clone()),
                _ => Err(QueryError::query(
                    range.clone(),
                    "incompatible native stage",
                )),
            }
        }
        fn is_explicit_source(&self, stage: &Stage) -> bool {
            matches!(
                stage,
                Stage::NativeArgument(_) | Stage::Match(_) | Stage::SignatureQuery { .. }
            )
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
        fn retains_native_admission(&self) -> bool {
            true
        }
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
    fn shared_reader_pairs_generators_scopes_and_existing_expression_forms() {
        let bindings = Bindings::new([(
            super::at("xs".into(), 0),
            Value::Constructed(V::Array(vec![V::Integer(1), V::Integer(2), V::Integer(3)])),
        )])
        .unwrap();
        let selected = V::Array(vec![V::Integer(4), V::Integer(6)]);
        for (common, alternate, expected) in [
            (
                "[x*2 for x in xs if x>1]",
                "[x*2 | x in xs where x>1]",
                selected,
            ),
            (
                "[(names,count,total) for names,count in [1,2] for total in [names+count] if total>2]",
                "[(names,count,total) | names,count in [1,2], total = names+count where total>2]",
                V::Array(vec![
                    V::Tuple(vec![V::Integer(1), V::Integer(2), V::Integer(3)]),
                    V::Tuple(vec![V::Integer(2), V::Integer(1), V::Integer(3)]),
                    V::Tuple(vec![V::Integer(2), V::Integer(2), V::Integer(4)]),
                ]),
            ),
            (
                "[a+b for (a,b) in [(1,2),(3,4)]]",
                "[a+b | [a,b,..] in [[1,2,99],[3,4]]]",
                V::Array(vec![V::Integer(3), V::Integer(7)]),
            ),
            (
                "{x mod 2 for x in xs}",
                "{x mod 2 | x in xs}",
                V::Set {
                    element_type: Some(zirql::SetElementType::Integer),
                    elements: vec![V::Integer(1), V::Integer(0)].into(),
                },
            ),
            (
                r#"{label:n for (label,n) in [("b",2),("a",1)]}"#,
                r#"{label:n | (label,n) in [("b",2),("a",1)]}"#,
                V::Record(vec![
                    ("b".into(), V::Integer(2)),
                    ("a".into(), V::Integer(1)),
                ]),
            ),
            (
                "index_set([x:x for x in (x:set<int>()).x])",
                "index_set([x:x | x in (x:set<int>()).x])",
                V::Set {
                    element_type: Some(zirql::SetElementType::Integer),
                    elements: vec![].into(),
                },
            ),
        ] {
            for text in [common, alternate] {
                let mut budget = function_budget();
                let mut reader = zirql::ArgumentReader::new(text, 0).unwrap();
                let value =
                    zirql::read_value_expression(&mut reader, &mut Host, &mut budget, 64).unwrap();
                assert!(reader.remaining().trim().is_empty(), "{text}");
                let result = zirql::evaluate_native_value(
                    &value,
                    &Host,
                    Value::Subjects(vec![]),
                    &mut (),
                    &bindings,
                    &mut budget,
                )
                .unwrap_or_else(|error| panic!("{text}: {error}"));
                assert_eq!(result, expected, "{text}");
                zirql::dispose_native_value(value);
            }
        }
        let text = r#"% quoted names retain logical identity
            [(label: 'if', value: if x == 2 /\ true then [|2:1:|4:42,41|3:32,31|][3,1]
              elseif false \/ false then 1 div 0 else [3:7,8][4] endif)
             | (x,'if') in [(2,"a"),(1,"b")] /* generator trivia */] | filter(_.value > 0)"#;
        let mut budget = function_budget();
        let mut reader = zirql::ArgumentReader::new(text, 0).unwrap();
        let value = zirql::read_value_expression(&mut reader, &mut Host, &mut budget, 64).unwrap();
        assert!(reader.remaining().is_empty());
        let result = zirql::evaluate_native_value(
            &value,
            &Host,
            Value::Subjects(vec![]),
            &mut (),
            &Bindings::default(),
            &mut budget,
        )
        .unwrap();
        assert_eq!(
            result,
            V::Array(vec![
                V::Record(vec![
                    ("label".into(), V::String("a".into())),
                    ("value".into(), V::Integer(31))
                ]),
                V::Record(vec![
                    ("label".into(), V::String("b".into())),
                    ("value".into(), V::Integer(8))
                ])
            ])
        );
        zirql::dispose_native_value(value);
        let text = "[(names | count) | names in source] | count";
        let bindings = Bindings::new([(
            super::at("source".into(), 0),
            Value::Subjects(vec!["a", "b"]),
        )])
        .unwrap();
        let mut reader = zirql::ArgumentReader::new(text, 0).unwrap();
        let value = zirql::read_value_expression(&mut reader, &mut Host, &mut budget, 64).unwrap();
        assert!(reader.remaining().is_empty());
        let result = zirql::FunctionRequest::prepare(
            &[],
            &Host,
            &mut (),
            &mut budget,
            zirql::FunctionLimits { active_depth: 1 },
        )
        .unwrap()
        .evaluate(&value, &bindings)
        .unwrap();
        assert_eq!(result, Value::Constructed(V::Integer(2)));
        zirql::dispose_native_value(value);
    }

    #[test]
    fn scoped_reader_domain_calls_keep_names_numeric_bounds_and_full_admission() {
        use zirql::{DeclaredType as D, FunctionLimits, FunctionRequest};
        let text = "fn 'bump'('if': int) -> int { let 'count' = 'if' + 0o1; 'count' } 'bump'(2)";
        let mut reader = zirql::ArgumentReader::new(text, 0).unwrap();
        let mut budget = function_budget();
        let declaration =
            zirql::read_function_declaration(&mut reader, &mut Host, &mut budget, 64, |name, _| {
                Err::<D<Kind>, _>(QueryError::query(name.range, "unknown type"))
            })
            .unwrap()
            .unwrap();
        assert_eq!(declaration.parameters[0].value.name.value, "if");
        assert_eq!(
            &text[declaration.parameters[0].value.name.range.clone()],
            "'if'"
        );
        let value = zirql::read_value_expression(&mut reader, &mut Host, &mut budget, 64).unwrap();
        assert_eq!(
            FunctionRequest::prepare(
                &[declaration],
                &Host,
                &mut (),
                &mut budget,
                FunctionLimits { active_depth: 1 }
            )
            .unwrap()
            .evaluate(&value, &Bindings::default())
            .unwrap(),
            Value::Constructed(V::Declared {
                value: Box::new(V::Integer(3)),
                declaration: D::Integer
            })
        );
        zirql::dispose_native_value(value);
        let text = r#"(low: -0x8000000000000000, high: +0X7FFFFFFFFFFFFFFF, octal: 0o17, member: enum_member(Colour,"red"), matched: match (-0x8000000000000000) { -0x8000000000000000 => true, _ => false }, arithmetic: (0x1e+2,0x1e-2,0X1E+2,0x1d+2), exponent: 1e+2+1e-2)"#;
        let mut reader = zirql::ArgumentReader::new(text, 0).unwrap();
        let value = zirql::read_value_expression(&mut reader, &mut Host, &mut budget, 64).unwrap();
        let result = zirql::evaluate_native_value(
            &value,
            &Host,
            Value::Subjects(vec![]),
            &mut (),
            &Bindings::default(),
            &mut budget,
        )
        .unwrap();
        let V::Record(fields) = result else {
            panic!("record")
        };
        assert_eq!(fields[0].1, V::Integer(i64::MIN));
        assert_eq!(fields[1].1, V::Integer(i64::MAX));
        assert_eq!(fields[2].1, V::Integer(15));
        assert!(
            matches!(&fields[3].1, V::Member(member) if member.type_id == zirql::NominalTypeId(7) && member.member_name == "red")
        );
        assert_eq!(fields[4].1, V::Boolean(true));
        assert_eq!(
            fields[5].1,
            V::Tuple(vec![
                V::Integer(32),
                V::Integer(28),
                V::Integer(32),
                V::Integer(31)
            ])
        );
        assert_eq!(fields[6].1, V::Float(100.01));
        zirql::dispose_native_value(value);
        // An ordinary domain argument uses the scoped reader, even when its
        // lexical name is also a registered native stage.
        let text = "native(names) | count";
        let mut reader = zirql::ArgumentReader::new(text, 0).unwrap();
        let value = zirql::read_value_expression_in_scope(
            &mut reader,
            &mut Host,
            &["names"],
            &mut budget,
            64,
        )
        .unwrap();
        let bindings = Bindings::new([(
            super::at("names".into(), 0),
            Value::Subjects(vec!["a", "b"]),
        )])
        .unwrap();
        let result = FunctionRequest::prepare(
            &[],
            &Host,
            &mut (),
            &mut budget,
            FunctionLimits { active_depth: 1 },
        )
        .unwrap()
        .evaluate(&value, &bindings)
        .unwrap();
        assert!(matches!(
            result,
            Value::Count(2) | Value::Constructed(V::Integer(2))
        ));
        zirql::dispose_native_value(value);
        for text in [
            "fn unused() -> int { let hidden = edit; 1 }",
            "fn unused() -> int { if true then 1 else edit endif }",
        ] {
            let mut reader = zirql::ArgumentReader::new(text, 0).unwrap();
            let declaration = zirql::read_function_declaration(
                &mut reader,
                &mut Host,
                &mut budget,
                64,
                |name, _| Err::<D<Kind>, _>(QueryError::query(name.range, "unknown type")),
            )
            .unwrap()
            .unwrap();
            assert!(
                FunctionRequest::prepare(
                    &[declaration],
                    &Host,
                    &mut (),
                    &mut budget,
                    FunctionLimits { active_depth: 1 }
                )
                .is_err()
            );
        }
    }

    #[test]
    fn native_argument_readers_preserve_the_enclosing_nesting_allowance() {
        let mut budget = Budget::for_program(ProgramLimits {
            nesting: 8,
            ..super::limits()
        });
        let text = "native(native(f)) | count";
        let mut reader = zirql::ArgumentReader::new(text, 0).unwrap();
        let value = zirql::read_value_expression(&mut reader, &mut Host, &mut budget, 4).unwrap();
        assert!(reader.remaining().is_empty());
        assert_eq!(value.range, 0..text.len());
        zirql::dispose_native_value(value);

        let text = "native(native(native(native(f)))) | count";
        let mut reader = zirql::ArgumentReader::new(text, 0).unwrap();
        let error =
            zirql::read_value_expression(&mut reader, &mut Host, &mut budget, 4).unwrap_err();
        assert_eq!(error.location, Location::Query(28..29));
        assert!(error.message.contains("nesting"));
        let text = "f | filter(count_is(native(native(f)) | count))";
        let mut reader = zirql::ArgumentReader::new(text, 0).unwrap();
        let error =
            zirql::read_value_expression(&mut reader, &mut Host, &mut budget, 4).unwrap_err();
        assert!(
            matches!(error.location,Location::Query(ref range) if range.start < range.end && range.end <= text.len())
        );
        assert!(error.message.contains("nesting"));
        // A refused callback restores the allowance for the next reader.
        let text = "native(native(f)) | count";
        let value = zirql::read_value_expression(
            &mut zirql::ArgumentReader::new(text, 0).unwrap(),
            &mut Host,
            &mut budget,
            4,
        )
        .unwrap();
        zirql::dispose_native_value(value);
    }

    #[test]
    fn shared_reader_refuses_scope_shape_and_malformed_alternate_syntax_with_locations() {
        for text in [
            "[(x,y) | x in [y], y in [1]]",
            "[x | x in 3]",
            "[x | x in [1], x in [2]]",
        ] {
            let mut budget = function_budget();
            let mut reader = zirql::ArgumentReader::new(text, 0).unwrap();
            let value =
                zirql::read_value_expression(&mut reader, &mut Host, &mut budget, 64).unwrap();
            let error = zirql::evaluate_native_value(
                &value,
                &Host,
                Value::Subjects(vec![]),
                &mut (),
                &Bindings::default(),
                &mut budget,
            )
            .unwrap_err();
            assert!(matches!(
                error,
                EvaluationError::Core(QueryError {
                    location: Location::Query(_),
                    ..
                })
            ));
            zirql::dispose_native_value(value);
        }
        for text in [
            "[x for x = 1]",
            "[x | x in [1] where ]",
            "[|1,2|3|]",
            "[||]",
            "[9223372036854775807:1,2]",
            "0x1.0p2",
            "['bad]",
            "[1 /* unfinished]",
            "[(1,2]]",
        ] {
            let mut budget = function_budget();
            let mut reader = zirql::ArgumentReader::new(text, 0).unwrap();
            let error =
                zirql::read_value_expression(&mut reader, &mut Host, &mut budget, 64).unwrap_err();
            assert!(
                matches!(error.location, Location::Query(ref range) if range.start <= range.end && range.end <= text.len()),
                "{text}: {error}"
            );
        }
        let text = "[x*2 | x in [1,2,3] where x>1]";
        let mut budget = Budget::for_program(ProgramLimits {
            work: 8,
            ..super::limits()
        });
        let error = zirql::read_value_expression(
            &mut zirql::ArgumentReader::new(text, 0).unwrap(),
            &mut Host,
            &mut budget,
            64,
        )
        .unwrap_err();
        assert_eq!(error.category, QueryErrorCategory::WorkLimit);
    }

    #[test]
    fn named_accumulators_use_prepared_native_arguments_and_refuse_impure_steps() {
        use zirql::{DeclaredType as D, FunctionLimits, FunctionRequest, ValueBinaryOperator as B};
        let mut add = native_function(
            "add",
            function_value(K::Binary {
                operator: B::Add,
                left: Box::new(fn_name("n")),
                right: Box::new(fn_name("xs")),
            }),
        );
        add.parameters[1].value.declaration = D::Integer;
        add.result = super::at(D::Integer, 4);
        let scan = function_value(K::Scan {
            items: Box::new(function_value(K::Array(vec![fn_int(2), fn_int(3)]))),
            initial: Box::new(fn_int(0)),
            step: super::at("add".into(), 70),
        });
        let mut apply = native_function(
            "apply",
            function_value(K::NativeQuery(Box::new(native_operand(Stage::Argument {
                value: Box::new(scan),
                selected: true,
            })))),
        );
        apply.result = super::at(D::Integer, 4);
        let functions = [add, apply];
        let bindings =
            Bindings::new([(super::at("input".into(), 0), Value::Subjects(vec!["alpha"]))])
                .unwrap();
        let entry = function_call(
            "apply",
            vec![
                fn_int(0),
                function_value(K::NativeValue(Box::new(super::at(
                    ExpressionKind::Binding("input".into()),
                    61,
                )))),
            ],
        );
        let mut budget = Budget::for_program(ProgramLimits {
            work: 1_000_000,
            results: 100_000,
            nesting: 64,
            stream_items: 100,
        });
        let result = FunctionRequest::prepare(
            &functions,
            &Host,
            &mut (),
            &mut budget,
            FunctionLimits { active_depth: 2 },
        )
        .unwrap()
        .evaluate(&entry, &bindings)
        .unwrap();
        assert!(matches!(result, Value::Constructed(value) if value.data() == &V::Integer(4)));
        let impure = native_function(
            "impure",
            function_value(K::NativeValue(Box::new(native_operand(Stage::Edit)))),
        );
        let empty = function_value(K::Fold {
            items: Box::new(function_value(K::Array(vec![]))),
            initial: Box::new(fn_int(0)),
            step: super::at("impure".into(), 70),
        });
        let functions = [impure];
        assert!(
            FunctionRequest::prepare(
                &functions,
                &Host,
                &mut (),
                &mut budget,
                FunctionLimits { active_depth: 1 }
            )
            .and_then(|request| request.evaluate(&empty, &bindings))
            .is_err()
        );
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
            function_value(K::Loop {
                states: vec![(super::at("loop_subject".into(), 9), fn_name("xs"))],
                body: Box::new(function_value(K::Break(Box::new(function_value(
                    K::NativeValue(Box::new(super::at(
                        ExpressionKind::Binding("loop_subject".into()),
                        20,
                    ))),
                ))))),
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
        let capture = Bindings::new([(
            super::at("row".into(), 60),
            Value::Capture(NativeCapture {
                fields: vec![("selected".into(), Value::Subjects(vec!["alpha"]))],
            }),
        )])
        .unwrap();
        let field = function_value(K::Field {
            value: Box::new(function_value(K::NativeValue(Box::new(super::at(
                ExpressionKind::Binding("row".into()),
                61,
            ))))),
            field: super::at("selected".into(), 62),
        });
        let kinds = BTreeMap::from([(
            "row".into(),
            Kind::Capture(Arc::new(NativeCapture {
                fields: vec![("selected".into(), Kind::Subjects)],
            })),
        )]);
        assert_eq!(
            zirql::pattern_function_value_kind(
                std::iter::empty(),
                &Host,
                &field,
                &kinds,
                None,
                &mut budget,
            )
            .unwrap(),
            Kind::Subjects
        );
        let result = FunctionRequest::prepare(
            &functions,
            &Host,
            &mut (),
            &mut budget,
            FunctionLimits { active_depth: 1 },
        )
        .unwrap()
        .evaluate(&field, &capture)
        .unwrap();
        assert!(matches!(result,Value::Subjects(rows) if rows==vec!["alpha"]));
        let mut items = native_function(
            "items",
            function_value(K::Loop {
                states: vec![(super::at("tick".into(), 9), fn_int(0))],
                body: Box::new(function_value(K::Conditional {
                    branches: vec![(
                        fn_binary(B::Equal, fn_name("tick"), fn_int(1)),
                        function_value(K::Break(Box::new(function_value(K::Array(vec![fn_int(
                            1,
                        )]))))),
                    )],
                    otherwise: Box::new(function_value(K::Continue(vec![fn_int(1)]))),
                })),
            }),
        );
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
        for name in ["names", "sort", "sort_by", "min", "max", "min_by", "max_by"] {
            let functions = [native_function(name, fn_name("xs"))];
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
    }
    #[test]
    fn parsed_string_helpers_share_prepared_native_arguments() {
        let source = r#"[contains("Ångström", "ström"), starts_with("Ångström", "ång"), ends_with("Ångström", ""), contains("é", "é"), starts_with("", ""), contains(one(source | names), "et")]"#;
        let mut allowance = function_budget();
        let mut reader = zirql::ArgumentReader::new(source, 0).unwrap();
        let expression =
            zirql::read_value_expression(&mut reader, &mut Host, &mut allowance, 64).unwrap();
        assert!(reader.remaining().is_empty());
        let mut functions = zirql::PreparedFunctions::prepare(&[], &Host, &mut allowance).unwrap();
        let scope = BTreeMap::from([("source".into(), Kind::Subjects)]);
        let prepared = functions
            .prepare_value(&expression, &scope, None, &mut allowance)
            .unwrap();
        let bindings =
            Bindings::new([(super::at("source".into(), 0), Value::Subjects(vec!["beta"]))])
                .unwrap();
        let result = functions
            .evaluate(
                &prepared,
                &bindings,
                None,
                &mut (),
                &mut allowance,
                zirql::FunctionLimits { active_depth: 1 },
            )
            .unwrap();
        assert_eq!(
            result,
            Value::Constructed(V::Array(
                vec![true, false, true, false, true, true]
                    .into_iter()
                    .map(V::Boolean)
                    .collect()
            ))
        );
        zirql::dispose_native_value(expression);
    }

    #[test]
    fn prepared_set_tallies_retain_empty_shape_and_signature_types() {
        use zirql::{FunctionLimits, PreparedFunctions};
        let mut allowance = function_budget();
        let mut reader = zirql::ArgumentReader::new(
            r#"fn tally_sets(xs: sequence<set<string>>) -> dict<set<string>, int> { (xs) | tally }"#,
            0,
        ).unwrap();
        let function = zirql::read_function_declaration(
            &mut reader,
            &mut Host,
            &mut allowance,
            64,
            |name, _| {
                Err::<zirql::DeclaredType<Kind>, _>(QueryError::query(
                    name.range,
                    "unexpected type",
                ))
            },
        )
        .unwrap()
        .unwrap();
        let mut functions = PreparedFunctions::prepare([&function], &Host, &mut allowance).unwrap();
        let source = r#"[tally_sets([]), ([set<string>() for x in []] | tally), ([{"ash", "apple"}, {"apple", "ash"}] | tally)]"#;
        let mut reader = zirql::ArgumentReader::new(source, 0).unwrap();
        let expression =
            zirql::read_value_expression(&mut reader, &mut Host, &mut allowance, 64).unwrap();
        let prepared = functions
            .prepare_value(&expression, &BTreeMap::new(), None, &mut allowance)
            .unwrap();
        let result = functions
            .evaluate(
                &prepared,
                &Bindings::default(),
                None,
                &mut (),
                &mut allowance,
                FunctionLimits { active_depth: 1 },
            )
            .unwrap();
        let Value::Constructed(V::Array(rows)) = result else {
            panic!("constructed rows")
        };
        assert!(
            matches!(rows[0].data(), V::Dictionary(values) if values.is_empty()),
            "{:?}",
            rows[0]
        );
        assert_eq!(rows[1], V::Dictionary(vec![].into()));
        assert_eq!(
            rows[2],
            V::Dictionary(
                vec![(
                    V::Set {
                        element_type: Some(zirql::SetElementType::String),
                        elements: vec![V::String("ash".into()), V::String("apple".into())].into()
                    },
                    V::Integer(2)
                )]
                .into()
            )
        );
        zirql::dispose_native_value(expression);
    }

    #[test]
    fn parsed_functions_share_preparation_native_arguments_and_generator_kinds() {
        use zirql::{DeclaredType as D, FunctionLimits, PreparedFunctions};
        let source = r#"
            fn odd(n: int) -> bool { let next = n - 1; if n == 0 { false } else { even(next) } }
            fn even(n: int) -> bool { if n == 0 { true } else if n == 1 { false } else { odd(n - 1) } }
            fn identity(xs: Subjects) -> Subjects { let alias = xs; alias }
            fn counts(xs: sequence<Subjects>) -> int { sum([identity(item) | count for item in xs]) }
            fn ordered(xs: Subjects) -> int { native(if (min(sort([2, 1, 2, 1], descending=true)) default 0) == 1 { xs } else { identity(xs) }) | count }
            fn parameter(if: int) -> int { let saved = if; if saved == 0 { if } else { saved } }
            fn local() -> int { let if = 3; if if == 3 { if } else { 0 } }
            fn forwarded(if: Subjects) -> int { native(if true { if } else { if }) | count }
            fn forwarded_local(xs: Subjects) -> int { let if = xs; native(if) | count }
            fn make_words() -> sequence<string> { ["ash", "apple", "ash"] }
            fn distinct_words() -> sequence<string> { (make_words()) | unique }
            fn matching_words(xs: sequence<string>) -> int { (xs) | filter(starts_with("a")) | count }
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
        let mut functions = PreparedFunctions::prepare_borrowed(
            declarations[..2]
                .iter()
                .chain(&declarations[2..])
                .map(zirql::FunctionDeclaration::as_borrowed),
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
        let mut reader =
            zirql::ArgumentReader::new(r#"matching_words(["ash", "birch", "apple"])"#, 0).unwrap();
        let matching =
            zirql::read_value_expression(&mut reader, &mut Host, &mut budget, 64).unwrap();
        let prepared = functions
            .prepare_value(&matching, &BTreeMap::new(), None, &mut budget)
            .unwrap();
        assert!(
            matches!(functions.evaluate(&prepared, &Bindings::default(), None, &mut (), &mut budget, limits).unwrap(), Value::Constructed(value) if value.data() == &V::Integer(2))
        );
        let mut reader = zirql::ArgumentReader::new("(make_words()) | sort | unique", 0).unwrap();
        let piped = zirql::read_value_expression(&mut reader, &mut Host, &mut budget, 64).unwrap();
        let typed = function_value(K::Unique(Box::new(function_value(K::Sort {
            array: Box::new(function_call("make_words", vec![])),
            selector: None,
            descending: false,
        }))));
        let prepared = functions
            .prepare_value(&piped, &BTreeMap::new(), None, &mut budget)
            .unwrap();
        let expected = functions
            .prepare_value(&typed, &BTreeMap::new(), None, &mut budget)
            .unwrap();
        let left = functions
            .evaluate(
                &prepared,
                &Bindings::default(),
                None,
                &mut (),
                &mut budget,
                limits,
            )
            .unwrap();
        let right = functions
            .evaluate(
                &expected,
                &Bindings::default(),
                None,
                &mut (),
                &mut budget,
                limits,
            )
            .unwrap();
        assert_eq!(left, right);
        assert!(
            matches!(left, Value::Constructed(ref value) if value.data() == &V::Array(vec![V::String("apple".into()), V::String("ash".into())]))
        );
        let mut reader = zirql::ArgumentReader::new("distinct_words()", 0).unwrap();
        let distinct =
            zirql::read_value_expression(&mut reader, &mut Host, &mut budget, 64).unwrap();
        let distinct = functions
            .prepare_value(&distinct, &BTreeMap::new(), None, &mut budget)
            .unwrap();
        let result = functions
            .evaluate(
                &distinct,
                &Bindings::default(),
                None,
                &mut (),
                &mut budget,
                limits,
            )
            .unwrap();
        assert!(
            matches!(result, Value::Constructed(ref value) if value.data() == &V::Array(vec![V::String("ash".into()), V::String("apple".into())]))
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
        let mut reader = zirql::ArgumentReader::new(
            "forwarded(source) + forwarded_local(source) + ordered(source)",
            0,
        )
        .unwrap();
        let forwarded =
            zirql::read_value_expression(&mut reader, &mut Host, &mut budget, 64).unwrap();
        let forwarded = functions
            .prepare_value(&forwarded, &scope, None, &mut budget)
            .unwrap();
        assert_eq!(
            functions
                .evaluate(&forwarded, &bindings, None, &mut (), &mut budget, limits)
                .unwrap(),
            Value::Constructed(V::Integer(9))
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
        assert_eq!(paths.target.value, zirql::PredicateKind::Leaf(Guard::True));
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
    fn signature_kind_callbacks_borrow_bodies_without_preparing_them_again() {
        use zirql::{DeclaredType as D, PatternFunctionDeclaration, PreparedFunctions};
        let declaration = || PatternFunctionDeclaration {
            name: super::at("header_only".into(), 1),
            parameters: vec![],
            result: super::at(D::Native(Kind::Subjects), 2),
            value: function_value(K::Name("invalid_unused_body".into())),
        };
        let callback = function_value(K::NativeValue(Box::new(native_operand(
            Stage::SignatureQuery {
                declaration: Box::new(declaration()),
                value: Box::new(function_call("header_only", vec![])),
            },
        ))));
        let outer = [native_function("outer", callback)];
        let mut budget = function_budget();
        // This enters the host's kind callback while the outer body's complete
        // preparation is active. The callback needs only the inner signature.
        assert!(PreparedFunctions::prepare(&outer, &Host, &mut budget).is_ok());
        let owned = declaration();
        let borrowed = || zirql::PatternFunctionDeclarationRef {
            name: &owned.name,
            parameters: &[],
            result: &owned.result,
            value: &owned.value,
        };
        let entry = function_call("header_only", vec![]);
        let scope = BTreeMap::new();
        assert_eq!(
            zirql::pattern_function_value_kind([&owned], &Host, &entry, &scope, None, &mut budget)
                .unwrap(),
            zirql::pattern_function_value_kind_borrowed(
                [borrowed()],
                &Host,
                &entry,
                &scope,
                None,
                &mut budget,
            )
            .unwrap()
        );
        // Signature discovery never substitutes for the complete request check.
        assert!(PreparedFunctions::prepare_patterns([&owned], &Host, &mut budget).is_err());
        assert!(
            PreparedFunctions::prepare_patterns_borrowed([borrowed()], &Host, &mut budget).is_err()
        );
        let text = "{row: (label, _), \"cost\": present(amount), ..}";
        let mut reader = zirql::ArgumentReader::new(text, 0).unwrap();
        let pattern = zirql::read_pattern(&mut reader, &mut budget, 64).unwrap();
        let names = zirql::pattern_names(&pattern, &mut budget).unwrap();
        assert_eq!(
            names
                .iter()
                .map(|name| name.value.as_str())
                .collect::<Vec<_>>(),
            ["label", "amount"]
        );
        for name in names {
            assert_eq!(&text[name.range.clone()], name.value);
        }
        let mut scarce = Budget::for_program(ProgramLimits {
            work: 1,
            ..super::limits()
        });
        assert!(zirql::pattern_names(&pattern, &mut scarce).is_err());
    }

    #[test]
    fn present_patterns_keep_refined_callback_facts_and_opaque_native_kinds() {
        use zirql::{
            DeclaredType as D, PatternFunctionDeclaration, PatternKind as P, ValueMatchArm,
        };
        let binding = |name: &str| {
            function_value(K::NativeValue(Box::new(super::at(
                ExpressionKind::Binding(name.into()),
                9,
            ))))
        };
        let callback = |value| {
            function_value(K::NativeValue(Box::new(native_operand(
                Stage::SignatureQuery {
                    declaration: Box::new(PatternFunctionDeclaration {
                        name: super::at("header_only".into(), 1),
                        parameters: vec![],
                        result: super::at(D::Native(Kind::Subjects), 2),
                        value: function_value(K::Name("invalid_unused_body".into())),
                    }),
                    value: Box::new(value),
                },
            ))))
        };
        let matching = |guard, value, fallback| {
            function_value(K::ValueMatch {
                subject: Box::new(binding("optional")),
                arms: vec![
                    ValueMatchArm {
                        pattern: super::at(
                            P::Present(Box::new(super::at(
                                P::Name(super::at("cost".into(), 5)),
                                5,
                            ))),
                            4,
                        ),
                        guard,
                        value,
                    },
                    ValueMatchArm {
                        pattern: super::at(P::Wildcard, 6),
                        guard: None,
                        value: fallback,
                    },
                ],
            })
        };
        let number = |value| function_value(K::Literal(V::Integer(value)));
        let entry = matching(
            Some(callback(function_value(K::Binary {
                operator: zirql::ValueBinaryOperator::Greater,
                left: Box::new(function_value(K::Name("cost".into()))),
                right: Box::new(number(0)),
            }))),
            function_value(K::Binary {
                operator: zirql::ValueBinaryOperator::Add,
                left: Box::new(function_value(K::Name("cost".into()))),
                right: Box::new(number(1)),
            }),
            number(0),
        );
        let mut budget = function_budget();
        let scope = BTreeMap::from([
            ("xs".into(), Kind::Subjects),
            (
                "optional".into(),
                Kind::Constructed(Arc::new(vec![ValueType::Integer, ValueType::Absent])),
            ),
        ]);
        let Kind::Constructed(types) =
            zirql::pattern_function_value_kind([], &Host, &entry, &scope, None, &mut budget)
                .unwrap()
        else {
            panic!("constructed integer match result");
        };
        assert!(!types.is_empty() && types.iter().all(|ty| *ty == ValueType::Integer));
        let native = matching(None, callback(binding("cost")), binding("optional"));
        let scope = BTreeMap::from([
            ("xs".into(), Kind::Subjects),
            ("optional".into(), Kind::Subjects),
        ]);
        assert_eq!(
            zirql::pattern_function_value_kind([], &Host, &native, &scope, None, &mut budget)
                .unwrap(),
            Kind::Subjects
        );
    }

    #[test]
    fn parsed_pattern_functions_blocks_matches_and_borrowed_headers_share_execution() {
        use zirql::{DeclaredType as D, FunctionLimits, PreparedFunctions};
        let source = r#"
            fn report((label, {cost: (_, cost), ..}): tuple<string, record {cost: tuple<string, int>, ignored: int}>) -> record {label: string, total: int} {
                if cost > 0 { let answer = cost + 1; {"label": label, "total": answer} }
                else { {"label": label, "total": 0} }
            }
            fn pair_sum([left,right,..]: sequence<int>) -> int { left + right }
            fn positive((left,right): tuple<int,int>) -> bool { left + right > 0 }
            fn absent_only(absent: opt int) -> int { 1 }
            fn classify(value: opt int) -> int {
                match (value) {
                    absent => 0,
                    present(cost) if cost < 0 => require(false, 0, "negative"),
                    present(cost) if cost > 100 => 1,
                    present(cost) => { let answer = cost + 1; answer },
                    _ => -1
                }
            }
            fn tally((n, total): tuple<int, int>) -> int {
                match (n) { 0 => total, _ => { let next = (n - 1, total + n); if true { tally(next) } else { 0 } } }
            }
        "#;
        let mut budget = function_budget();
        let mut reader = zirql::ArgumentReader::new(source, 0).unwrap();
        let mut parsed = Vec::new();
        while let Some(function) = zirql::read_unresolved_pattern_function_declaration(
            &mut reader,
            &mut Host,
            &mut budget,
            64,
        )
        .unwrap()
        {
            parsed.push(function);
        }
        reader.skip_whitespace();
        assert!(reader.remaining().is_empty());
        let headers = parsed
            .iter()
            .map(|function| {
                function
                    .resolve_signature(&mut budget, |name, _| {
                        Err::<D<Kind>, _>(QueryError::query(name.range, "unexpected named type"))
                    })
                    .unwrap()
            })
            .collect::<Vec<_>>();
        assert_eq!(
            &source[headers[0].parameters[0].value.pattern.range.clone()],
            "(label, {cost: (_, cost), ..})"
        );
        let limits = FunctionLimits { active_depth: 1 };
        for (text, expected) in [
            (
                r#"report(("alpha", {"cost": ("unused", 4), "ignored": 99}))"#,
                V::Record(vec![
                    (
                        "label".into(),
                        V::Declared {
                            value: Box::new(V::String("alpha".into())),
                            declaration: D::String,
                        },
                    ),
                    ("total".into(), V::Integer(5)),
                ]),
            ),
            ("pair_sum([3,4,99])", V::Integer(7)),
            (
                "pair_sum([length([(1,2),(0,0)] | filter((a,b) => positive((a,b)) and _.1 == a)),0])",
                V::Integer(1),
            ),
            ("absent_only(null)", V::Integer(1)),
            ("classify(null)", V::Integer(0)),
            ("classify(4)", V::Integer(5)),
            ("tally((100, 0))", V::Integer(5050)),
        ] {
            let mut reader = zirql::ArgumentReader::new(text, 0).unwrap();
            let entry =
                zirql::read_value_expression(&mut reader, &mut Host, &mut budget, 64).unwrap();
            assert!(reader.remaining().is_empty());
            let mut functions = PreparedFunctions::prepare_patterns_borrowed(
                parsed
                    .iter()
                    .zip(&headers)
                    .map(|(body, header)| body.with_signature(header)),
                &Host,
                &mut budget,
            )
            .unwrap();
            let prepared = functions
                .prepare_value(&entry, &BTreeMap::new(), None, &mut budget)
                .unwrap();
            let result = functions
                .evaluate(
                    &prepared,
                    &Bindings::default(),
                    None,
                    &mut (),
                    &mut budget,
                    limits,
                )
                .unwrap();
            let Value::Constructed(V::Declared { value, .. }) = result else {
                panic!("declared function result");
            };
            assert_eq!(*value, expected);
        }
        let text = "absent_only(1)";
        let entry = zirql::read_value_expression(
            &mut zirql::ArgumentReader::new(text, 0).unwrap(),
            &mut Host,
            &mut budget,
            64,
        )
        .unwrap();
        let mut functions = PreparedFunctions::prepare_patterns_borrowed(
            parsed
                .iter()
                .zip(&headers)
                .map(|(body, header)| body.with_signature(header)),
            &Host,
            &mut budget,
        )
        .unwrap();
        let prepared = functions
            .prepare_value(&entry, &BTreeMap::new(), None, &mut budget)
            .unwrap();
        let error = functions
            .evaluate(
                &prepared,
                &Bindings::default(),
                None,
                &mut (),
                &mut budget,
                limits,
            )
            .unwrap_err();
        assert!(
            matches!(error.error, EvaluationError::Core(QueryError { location: Location::Query(ref range), .. }) if &source[range.clone()] == "absent")
        );
        drop(functions);
        zirql::dispose_native_value(entry);
        let text = r#"match (2) { 1 => {"kind": 0}, _ => {"kind": 2} }"#;
        let mut reader = zirql::ArgumentReader::new(text, 0).unwrap();
        let entry = zirql::read_value_expression(&mut reader, &mut Host, &mut budget, 64).unwrap();
        let mut functions = PreparedFunctions::prepare_patterns_borrowed(
            parsed
                .iter()
                .zip(&headers)
                .map(|(body, header)| body.with_signature(header)),
            &Host,
            &mut budget,
        )
        .unwrap();
        let prepared = functions
            .prepare_value(&entry, &BTreeMap::new(), None, &mut budget)
            .unwrap();
        assert_eq!(
            functions
                .evaluate(
                    &prepared,
                    &Bindings::default(),
                    None,
                    &mut (),
                    &mut budget,
                    limits
                )
                .unwrap(),
            Value::Constructed(V::Record(vec![("kind".into(), V::Integer(2))]))
        );
        drop(functions);
        drop(headers);
        zirql::dispose_unresolved_pattern_functions(parsed);
    }

    #[test]
    fn parsed_patterns_preserve_contextual_names_and_preflight_refusals() {
        use zirql::{FunctionLimits, PreparedFunctions};
        let mut budget = function_budget();
        let mut reader = zirql::ArgumentReader::new("fn contextual(if: int, match: int, value: int, present: int) -> int { let let = if; let fn = match; let param = value; let report = present; let + fn + param + report }", 0).unwrap();
        let function = zirql::read_pattern_function_declaration(
            &mut reader,
            &mut Host,
            &mut budget,
            64,
            |name, _| {
                Err::<zirql::DeclaredType<Kind>, _>(QueryError::query(
                    name.range,
                    "unexpected type",
                ))
            },
        )
        .unwrap()
        .unwrap();
        // Keep the old name-only API available beside pattern declarations.
        let mut legacy = zirql::ArgumentReader::new(
            "fn legacy(value: int) -> int { let saved = value; saved }",
            0,
        )
        .unwrap();
        let old =
            zirql::read_function_declaration(&mut legacy, &mut Host, &mut budget, 64, |name, _| {
                Err::<zirql::DeclaredType<Kind>, _>(QueryError::query(
                    name.range,
                    "unexpected type",
                ))
            })
            .unwrap()
            .unwrap();
        let mut functions =
            PreparedFunctions::prepare_patterns([&function], &Host, &mut budget).unwrap();
        let mut reader = zirql::ArgumentReader::new("contextual(1, 2, 3, 4)", 0).unwrap();
        let entry = zirql::read_value_expression(&mut reader, &mut Host, &mut budget, 64).unwrap();
        let prepared = functions
            .prepare_value(&entry, &BTreeMap::new(), None, &mut budget)
            .unwrap();
        assert!(
            matches!(functions.evaluate(&prepared, &Bindings::default(), None, &mut (), &mut budget, FunctionLimits { active_depth: 1 }).unwrap(), Value::Constructed(V::Declared { value, .. }) if *value == V::Integer(10))
        );
        for text in [
            "match (0) { _ => 0, missing => 1 }",
            "match (0) { x => x }",
            "if true { 1 } else { let (x, x) = (1, 2); x }",
            "if true { let x = 1; x } else { x }",
            "match (0) { _ => 1, x => edit }",
        ] {
            let mut reader = zirql::ArgumentReader::new(text, 0).unwrap();
            let entry =
                zirql::read_value_expression(&mut reader, &mut Host, &mut budget, 64).unwrap();
            let mut empty = PreparedFunctions::prepare([], &Host, &mut budget).unwrap();
            assert!(
                empty
                    .prepare_value(&entry, &BTreeMap::new(), None, &mut budget)
                    .is_err(),
                "{text}"
            );
        }
        for text in [
            "match (0) { x if 1 => x, _ => 2 }",
            "if true { let (x, y) = (1,); x } else { 0 }",
        ] {
            let mut reader = zirql::ArgumentReader::new(text, 0).unwrap();
            let entry =
                zirql::read_value_expression(&mut reader, &mut Host, &mut budget, 64).unwrap();
            let mut empty = PreparedFunctions::prepare([], &Host, &mut budget).unwrap();
            let prepared = empty.prepare_value(&entry, &BTreeMap::new(), None, &mut budget);
            if let Ok(prepared) = prepared {
                assert!(
                    empty
                        .evaluate(
                            &prepared,
                            &Bindings::default(),
                            None,
                            &mut (),
                            &mut budget,
                            FunctionLimits { active_depth: 1 }
                        )
                        .is_err()
                );
            }
        }
        zirql::dispose_functions(vec![old]);
        zirql::dispose_pattern_functions(vec![function]);
    }

    #[test]
    fn request_resolved_headers_borrow_host_lowered_bodies() {
        use zirql::{DeclaredType as D, FunctionLimits, PreparedFunctions};
        let source = "fn saved(xs: sequence<set<Colour>>) -> sequence<set<Colour>> { let local = xs; local }";
        let mut budget = function_budget();
        let mut reader = zirql::ArgumentReader::new(source, 0).unwrap();
        let parsed =
            zirql::read_unresolved_function_declaration(&mut reader, &mut Host, &mut budget, 64)
                .unwrap()
                .unwrap();
        assert_eq!(reader.position(), source.len());
        assert_eq!(&source[parsed.bindings[0].1.range.clone()], "xs");
        assert_eq!(&source[parsed.value.range.clone()], "local");
        let header = parsed
            .resolve_signature(&mut budget, |name, _| {
                assert_eq!(&source[name.range], "Colour");
                Ok::<_, QueryError>(D::<Kind>::Member(zirql::NominalTypeId(7)))
            })
            .unwrap();
        assert_eq!(
            header.result.value,
            D::Sequence(Box::new(D::Set(zirql::SetElementType::Member(
                zirql::NominalTypeId(7)
            ))))
        );
        let entry = function_call(
            "saved",
            vec![function_value(K::Array(vec![function_value(K::Literal(
                V::Set {
                    element_type: Some(zirql::SetElementType::Member(zirql::NominalTypeId(7))),
                    elements: vec![].into(),
                },
            ))]))],
        );
        let mut prepared = PreparedFunctions::prepare_borrowed(
            [parsed.with_signature(&header)],
            &Host,
            &mut budget,
        )
        .unwrap();
        let entry = prepared
            .prepare_value(&entry, &BTreeMap::new(), None, &mut budget)
            .unwrap();
        let result = prepared
            .evaluate(
                &entry,
                &Bindings::default(),
                None,
                &mut (),
                &mut budget,
                FunctionLimits { active_depth: 1 },
            )
            .unwrap();
        assert!(matches!(result, Value::Constructed(_)));
        let error = parsed
            .resolve_signature(&mut budget, |name, _| {
                Err::<D<Kind>, _>(QueryError::query(name.range, "unresolved current owner"))
            })
            .unwrap_err();
        assert_eq!(
            error.location,
            Location::Query(source.find("Colour").unwrap()..source.find("Colour").unwrap() + 6)
        );
        assert!(
            parsed
                .resolve_signature(&mut budget, |_, _| Ok(D::Native(Kind::Subjects)))
                .is_err()
        );
        drop(prepared);
        zirql::dispose_unresolved_functions(vec![parsed]);
    }

    #[test]
    fn dependent_match_arguments_keep_clause_scope_active_calls_and_guard_laziness() {
        use zirql::{DeclaredType as D, FunctionLimits, FunctionRequest, MatchClause as C};
        let make = |guard, source_name: &str| {
            let mut identity = native_function("identity", fn_name("xs"));
            identity.parameters.remove(0);
            let mut clauses = vec![super::at(
                C::From {
                    name: super::at("first".into(), 10),
                    source: super::at(ExpressionKind::Binding("xs".into()), 11),
                },
                10,
            )];
            if let Some(guard) = guard {
                clauses.push(super::at(
                    C::Where {
                        predicate: super::at(zirql::PredicateKind::Leaf(guard), 12),
                    },
                    12,
                ));
            }
            clauses.push(super::at(
                C::Bind {
                    name: super::at("second".into(), 13),
                    value: super::sequence(
                        super::at(ExpressionKind::Input, 14),
                        vec![super::at(
                            ExpressionStep::Stage(Stage::NativeArgument(Arc::new(function_call(
                                "identity",
                                vec![fn_name(source_name)],
                            )))),
                            14,
                        )],
                    ),
                },
                13,
            ));
            let source = function_value(K::NativeValue(Box::new(super::sequence(
                super::at(ExpressionKind::Input, 15),
                vec![super::at(
                    ExpressionStep::Stage(Stage::Match(Box::new(NativeMatch {
                        clauses,
                        range: 10..20,
                    }))),
                    15,
                )],
            ))));
            let values = function_value(K::Comprehension {
                constructor: zirql::ValueConstructor::Array,
                clauses: vec![super::at(
                    zirql::ValueGenerator::In {
                        case: false,
                        binders: vec![super::at(
                            zirql::PatternKind::Name(super::at("row".into(), 21)),
                            21,
                        )],
                        source,
                        filter: None,
                    },
                    21,
                )],
                head: Box::new(fn_int(1)),
            });
            let mut capture = native_function(
                "capture",
                function_value(K::Reduce {
                    operator: zirql::ValueReducer::Sum,
                    array: Box::new(values),
                }),
            );
            capture.result = super::at(D::Integer, 4);
            vec![identity, capture]
        };
        let entry = function_call(
            "capture",
            vec![
                fn_int(0),
                function_value(K::NativeValue(Box::new(super::at(
                    ExpressionKind::Binding("input".into()),
                    30,
                )))),
            ],
        );
        let bindings = Bindings::new([(
            super::at("input".into(), 31),
            Value::Subjects(vec!["alpha", "beta"]),
        )])
        .unwrap();
        let declarations = make(None, "first");
        let mut budget = function_budget();
        let result = FunctionRequest::prepare(
            &declarations,
            &Host,
            &mut (),
            &mut budget,
            FunctionLimits { active_depth: 2 },
        )
        .unwrap()
        .evaluate(&entry, &bindings)
        .unwrap();
        assert!(matches!(result,Value::Constructed(value) if value.data()==&V::Integer(2)));
        let error = FunctionRequest::prepare(
            &declarations,
            &Host,
            &mut (),
            &mut budget,
            FunctionLimits { active_depth: 1 },
        )
        .unwrap()
        .evaluate(&entry, &bindings)
        .unwrap_err();
        assert!(error.to_string().contains("active function depth"));
        let skipped = make(Some(Guard::False), "first");
        let result = FunctionRequest::prepare(
            &skipped,
            &Host,
            &mut (),
            &mut budget,
            FunctionLimits { active_depth: 1 },
        )
        .unwrap()
        .evaluate(&entry, &bindings)
        .unwrap();
        assert!(matches!(result,Value::Constructed(value) if value.data()==&V::Integer(0)));
        let unknown = make(Some(Guard::Unknown), "first");
        let error = FunctionRequest::prepare(
            &unknown,
            &Host,
            &mut (),
            &mut budget,
            FunctionLimits { active_depth: 2 },
        )
        .unwrap()
        .evaluate(&entry, &bindings)
        .unwrap_err();
        assert!(
            matches!(error.error,EvaluationError::Core(ref error) if error.category==QueryErrorCategory::UnknownPredicate)
        );
        let invalid = make(None, "later");
        assert!(
            FunctionRequest::prepare(
                &invalid,
                &Host,
                &mut (),
                &mut budget,
                FunctionLimits { active_depth: 2 }
            )
            .is_err()
        );
        let good = FunctionRequest::prepare(
            &declarations,
            &Host,
            &mut (),
            &mut budget,
            FunctionLimits { active_depth: 2 },
        )
        .unwrap()
        .evaluate(&entry, &bindings);
        assert!(good.is_ok());
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
                "fn f(x: set<tuple<int>>) -> int { 0 }",
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
        type Predicate = Guard;
        fn field_pipeline_source(&self) -> zirql::FieldPipelineSource {
            zirql::FieldPipelineSource::ValueArgument
        }
        fn value_source(
            &mut self,
            value: zirql::ValueExpression<Query>,
            range: Range<usize>,
        ) -> Result<Stage, QueryError> {
            Host.value_source(value, range)
        }
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
            Host.parse_stage(reader, budget)
        }
        fn parse_stage_in_scope(
            &mut self,
            reader: &mut zirql::ArgumentReader<'_>,
            _: &[&str],
            budget: &mut Budget,
            nesting: usize,
        ) -> Result<ExpressionStep<Stage, ()>, QueryError> {
            Host.parse_stage_in_scope(reader, self.0, budget, nesting)
        }
        fn parse_predicate_leaf_in_argument(
            &mut self,
            reader: &mut zirql::ArgumentReader<'_>,
            argument: Option<&zirql::Pattern>,
            locals: &[&str],
            budget: &mut Budget,
            nesting: usize,
        ) -> Result<Guard, QueryError> {
            let mut scope = self.0.to_vec();
            for name in locals {
                if !scope.contains(name) {
                    scope.push(name);
                }
            }
            Host.parse_predicate_leaf_in_argument(reader, argument, &scope, budget, nesting)
        }
        fn predicate_expression(
            &mut self,
            predicate: zirql::PredicateArgument<Query>,
            range: Range<usize>,
        ) -> Result<Guard, QueryError> {
            Ok(Guard::Expression(Arc::new(predicate.bind(Located::new(
                K::NativeValue(Box::new(Located::new(ExpressionKind::Input, range.clone()))),
                range,
            )))))
        }
        fn is_predicate_leaf(&self, name: &str) -> bool {
            matches!(name, "true" | "false" | "unknown" | "bound" | "count_is")
        }
        fn parse_predicate_leaf(
            &mut self,
            reader: &mut zirql::ArgumentReader<'_>,
            budget: &mut Budget,
        ) -> Result<Guard, QueryError> {
            Host.parse_predicate_leaf(reader, budget)
        }
        fn filter(&mut self, _: Predicate<Guard>, _: Range<usize>) -> Stage {
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
        type Predicate = Guard;
        fn parse_domain_call(
            &mut self,
            callee: Located<&str>,
            reader: &mut zirql::ArgumentReader<'_>,
            _: &[&str],
            budget: &mut Budget,
            _: usize,
        ) -> Result<Option<zirql::ValueExpression<Query>>, QueryError> {
            if callee.value != "enum_member" {
                return Ok(None);
            }
            reader.expect('(')?;
            let ty = reader.word()?;
            reader.expect(',')?;
            let member = reader.quoted_string()?;
            reader.expect(')')?;
            let range = callee.range.start..reader.position();
            budget.charge(range.len(), Location::Query(range.clone()))?;
            budget.charge_results(1, Location::Query(range.clone()))?;
            if ty.value != "Colour" || member.value != "red" {
                return Err(QueryError::query(range, "unknown nominal member"));
            }
            Ok(Some(Located::new(
                K::Literal(V::Member(zirql::NominalMember {
                    type_id: zirql::NominalTypeId(7),
                    member_id: zirql::NominalMemberId(1),
                    declaration_order: 0,
                    type_name: ty.value.into(),
                    member_name: member.value,
                })),
                range,
            )))
        }
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
            nesting: usize,
        ) -> Result<ExpressionStep<Stage, ()>, QueryError> {
            let mut probe = zirql::ArgumentReader::new(reader.source(), reader.position())?;
            if probe.keyword("native") {
                reader.word()?;
                reader.expect('(')?;
                let value = zirql::read_value_expression_in_scope(
                    reader, &mut Host, locals, budget, nesting,
                )?;
                reader.expect(')')?;
                return Ok(ExpressionStep::Stage(Stage::NativeArgument(Arc::new(
                    value,
                ))));
            }
            let mut probe = zirql::ArgumentReader::new(reader.source(), reader.position())?;
            if probe.keyword("paths") {
                return zirql::read_paths(reader, &mut ArgumentScope(locals), budget, nesting)
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
            nesting: usize,
        ) -> Result<Guard, QueryError> {
            let mut probe = zirql::ArgumentReader::new(reader.source(), reader.position())?;
            if probe.keyword("bound") {
                reader.word()?;
                reader.expect('(')?;
                let value = zirql::read_value_expression_in_scope(
                    reader, &mut Host, locals, budget, nesting,
                )?;
                reader.expect(')')?;
                if matches!(value.value,K::Name(ref name) if locals.contains(&name.as_str())) {
                    return Ok(Guard::True);
                }
                return Err(QueryError::query(value.range, "guard needs a lexical name"));
            }
            self.parse_predicate_leaf(reader, budget)
        }
        fn parse_predicate_leaf_in_argument(
            &mut self,
            reader: &mut zirql::ArgumentReader<'_>,
            argument: Option<&zirql::Pattern>,
            locals: &[&str],
            budget: &mut Budget,
            nesting: usize,
        ) -> Result<Guard, QueryError> {
            if !reader.keyword("count_is") {
                return self.parse_predicate_leaf_in_scope(reader, locals, budget, nesting);
            }
            reader.expect('(')?;
            let value =
                zirql::read_value_expression_in_scope(reader, &mut Host, locals, budget, nesting)?;
            reader.expect(')')?;
            let range = value.range.clone();
            let subject = Located::new(
                K::NativeValue(Box::new(Located::new(ExpressionKind::Input, range.clone()))),
                range,
            );
            Ok(Guard::Count(Arc::new(
                zirql::PredicateArgument {
                    argument: argument
                        .map(|p| zirql::copy_pattern(p, budget))
                        .transpose()?,
                    body: Box::new(value),
                }
                .bind_value(subject),
            )))
        }
        fn predicate_expression(
            &mut self,
            predicate: zirql::PredicateArgument<Query>,
            range: Range<usize>,
        ) -> Result<Guard, QueryError> {
            Ok(Guard::Expression(Arc::new(predicate.bind(Located::new(
                K::NativeValue(Box::new(Located::new(ExpressionKind::Input, range.clone()))),
                range,
            )))))
        }
        fn is_predicate_leaf(&self, name: &str) -> bool {
            matches!(name, "true" | "false" | "unknown" | "bound" | "count_is")
        }
        fn parse_predicate_leaf(
            &mut self,
            reader: &mut zirql::ArgumentReader<'_>,
            _: &mut Budget,
        ) -> Result<Guard, QueryError> {
            let word = reader.word()?;
            match word.value {
                "true" => Ok(Guard::True),
                "false" => Ok(Guard::False),
                "unknown" => Ok(Guard::Unknown),
                _ => Err(QueryError::query(word.range, "unsupported guard")),
            }
        }
        fn filter(&mut self, _: Predicate<Guard>, _: Range<usize>) -> Stage {
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
            matching: NativeMatch<ReadExpression<Self>, Guard>,
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
        type Guard = Guard;
        fn evaluate_match_source_with_arguments(
            &self,
            source: &Query,
            input: Value,
            bindings: &Bindings<Value>,
            arguments: &mut dyn zirql::NativeArguments<(), Self>,
            budget: &mut Budget,
        ) -> Result<Value, EvaluationError<QueryError>> {
            if let Some(value) = match_argument(source) {
                let selected = arguments.evaluate_match_value(value, input, bindings, budget)?;
                return self.copy_value(
                    selected.value(),
                    arguments.parts().0,
                    budget,
                    &source.range,
                );
            }
            zirql::evaluate_expression(source, self, input, arguments.parts().0, bindings, budget)
        }
        fn validate_guard_leaf(
            &self,
            _: &Guard,
            _: &Kind,
            _: &BTreeMap<String, Kind>,
            _: &mut Budget,
            _: &Range<usize>,
        ) -> Result<(), QueryError> {
            Ok(())
        }
        fn evaluate_guard_leaf_with_arguments(
            &self,
            guard: &Guard,
            input: &Value,
            bindings: &Bindings<Value>,
            arguments: &mut dyn zirql::NativeArguments<(), Self>,
            budget: &mut Budget,
            range: &Range<usize>,
        ) -> Result<Truth, EvaluationError<QueryError>> {
            if let Guard::Count(expression) = guard {
                let relative = self.copy_value(input, arguments.parts().0, budget, range)?;
                let value =
                    arguments.evaluate_match_value(expression, relative, bindings, budget)?;
                let value = self.copy_value(value.value(), arguments.parts().0, budget, range)?;
                let value = self.project_value(value, arguments.parts().0, budget, range)?;
                let V::Integer(expected) = value.data() else {
                    return Err(
                        QueryError::query(range.clone(), "count_is needs an integer").into(),
                    );
                };
                let Value::Subjects(subjects) = input else {
                    unreachable!()
                };
                return Ok(if *expected == i64::try_from(subjects.len()).unwrap() {
                    Truth::True
                } else {
                    Truth::False
                });
            }
            if let Guard::Expression(expression) = guard {
                let input = self.copy_value(input, arguments.parts().0, budget, range)?;
                return Ok(
                    if arguments.evaluate_predicate(expression, input, Some(bindings), budget)? {
                        Truth::True
                    } else {
                        Truth::False
                    },
                );
            }
            Ok(guard.truth())
        }
        fn evaluate_guard_leaf(
            &self,
            guard: &Guard,
            _: &Value,
            _: &mut (),
            _: &Bindings<Value>,
            _: &mut Budget,
            _: &Range<usize>,
        ) -> Result<Truth, EvaluationError<QueryError>> {
            Ok(guard.truth())
        }
    }
    impl PathHost<()> for Host {
        type Target = Guard;
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
            _: &Guard,
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
            target: &Guard,
            _: &Value,
            _: &mut (),
            _: &Bindings<Value>,
            _: &mut Budget,
            _: &Range<usize>,
        ) -> Result<Truth, EvaluationError<QueryError>> {
            Ok(target.truth())
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
        type Predicate = Guard;
        fn is_binding(&self, name: &str) -> bool {
            matches!(name, "f" | "one" | "project" | "if" | "words")
        }
        fn is_constructed_binding(&self, name: &str) -> bool {
            name == "words"
        }
        fn is_stage(&self, name: &str) -> bool {
            Host.is_stage(name)
                || matches!(
                    name,
                    "sort" | "min" | "max" | "sort_by" | "min_by" | "max_by"
                )
        }
        fn ordering(
            &mut self,
            ordering: zirql::NativeOrdering<Query>,
            _: Range<usize>,
        ) -> Result<Stage, QueryError> {
            if let Some(selector) = ordering.selector {
                zirql::dispose_expression(selector);
            }
            Ok(Stage::Saved("one".into()))
        }
        fn parse_stage(
            &mut self,
            reader: &mut zirql::ArgumentReader<'_>,
            budget: &mut Budget,
        ) -> Result<ExpressionStep<Stage, ()>, QueryError> {
            let mut probe = zirql::ArgumentReader::new(reader.source(), reader.position())?;
            let name = probe.word()?;
            if matches!(
                name.value,
                "sort" | "min" | "max" | "sort_by" | "min_by" | "max_by"
            ) {
                reader.word()?;
                if name.value.ends_with("_by") {
                    reader.expect('(')?;
                    let selector = reader.word()?;
                    assert_eq!(selector.value, "name");
                    reader.expect(')')?;
                }
                budget.charge(
                    reader.position() - name.range.start,
                    Location::Query(name.range),
                )?;
                Ok(ExpressionStep::Stage(Stage::Saved("one".into())))
            } else if matches!(name.value, "one" | "project") {
                budget.charge(name.value.len(), Location::Query(name.range.clone()))?;
                reader.advance(probe.position() - reader.position())?;
                Ok(ExpressionStep::Stage(Stage::Saved(name.value.to_owned())))
            } else {
                Host.parse_stage(reader, budget)
            }
        }
        fn parse_predicate_leaf_in_argument(
            &mut self,
            reader: &mut zirql::ArgumentReader<'_>,
            argument: Option<&zirql::Pattern>,
            locals: &[&str],
            budget: &mut Budget,
            nesting: usize,
        ) -> Result<Guard, QueryError> {
            Host.parse_predicate_leaf_in_argument(reader, argument, locals, budget, nesting)
        }
        fn predicate_expression(
            &mut self,
            predicate: zirql::PredicateArgument<Query>,
            range: Range<usize>,
        ) -> Result<Guard, QueryError> {
            Ok(Guard::Expression(Arc::new(predicate.bind(Located::new(
                K::NativeValue(Box::new(Located::new(ExpressionKind::Input, range.clone()))),
                range,
            )))))
        }
        fn is_predicate_leaf(&self, name: &str) -> bool {
            matches!(name, "true" | "false" | "unknown" | "bound" | "count_is")
        }
        fn parse_predicate_leaf(
            &mut self,
            reader: &mut zirql::ArgumentReader<'_>,
            budget: &mut Budget,
        ) -> Result<Guard, QueryError> {
            Host.parse_predicate_leaf(reader, budget)
        }
        fn filter(&mut self, predicate: Predicate<Guard>, range: Range<usize>) -> Stage {
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

    fn parse_match(source: &str) -> NativeMatch<Query, Guard> {
        let mut reader = zirql::ArgumentReader::new(source, 0).unwrap();
        let matching = zirql::read_match(&mut reader, &mut Host, &mut budget(), 32).unwrap();
        assert!(reader.remaining().is_empty());
        matching
    }

    fn parse_paths(source: &str) -> NativePaths<Stage, Guard> {
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
                    case: false,
                    binders: vec![Located::new(
                        zirql::PatternKind::Name(Located::new("m".into(), 0..1)),
                        0..1,
                    )],
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
                    case: false,
                    binders: vec![Located::new(
                        zirql::PatternKind::Name(Located::new("n".into(), 0..1)),
                        0..1,
                    )],
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
        use zirql::{DeclaredType as D, FunctionLimits, PreparedFunctions};
        let source = "project(w, {\"name\": prefix ++ one(w | names), \"pairs\": [n ++ suffix for n in w | names for suffix in [\"!\", \"?\"]]})";
        let projection = parse(source, &mut budget());
        let prepared_projection = parse(
            &source.replace("[\"!\", \"?\"]", "suffixes()"),
            &mut budget(),
        );
        let suffixes = zirql::FunctionDeclaration {
            name: super::at("suffixes".into(), 1),
            parameters: vec![],
            result: super::at(D::Sequence(Box::new(D::String)), 2),
            bindings: vec![],
            value: function_value(K::Array(vec![
                function_value(K::Literal(V::String("!".into()))),
                function_value(K::Literal(V::String("?".into()))),
            ])),
        };
        let mut functions = PreparedFunctions::prepare([&suffixes], &Host, &mut budget()).unwrap();
        let scope = BTreeMap::from([
            ("w".into(), Kind::Subjects),
            (
                "prefix".into(),
                Host.constructed_kind(vec![ValueType::String]),
            ),
        ]);
        let fields = prepared_projection
            .fields
            .iter()
            .map(|(_, value)| {
                functions
                    .prepare_value(value, &scope, Some(Kind::Subjects), &mut budget())
                    .unwrap()
            })
            .collect::<Vec<_>>();
        let fields = fields.iter().collect::<Vec<_>>();
        let bindings = Bindings::new([(
            super::at("prefix".into(), 0),
            Value::Constructed(V::String("row:".into())),
        )])
        .unwrap();
        let limits = FunctionLimits { active_depth: 2 };
        let mut first = None;
        for subjects in [vec!["alpha", "beta", "alpha"], vec!["other"], vec![]] {
            let result = zirql::evaluate_projection_with_types(
                &projection,
                &Host,
                Value::Subjects(subjects.clone()),
                &mut (),
                &bindings,
                &mut budget(),
            )
            .unwrap();
            let prepared = functions
                .evaluate_projection(
                    &prepared_projection,
                    &fields,
                    Value::Subjects(subjects.clone()),
                    &bindings,
                    &mut (),
                    &mut budget(),
                    limits,
                )
                .unwrap();
            assert_eq!(prepared.value, result.value);
            assert_eq!(
                prepared.types,
                vec![ValueType::Array(ArrayType::Elements(vec![
                    ValueType::Record(vec![
                        ("name".into(), ValueType::String),
                        (
                            "pairs".into(),
                            ValueType::Array(ArrayType::Elements(vec![ValueType::String]))
                        ),
                    ])
                ]))]
            );
            let expected = V::Array(
                subjects
                    .iter()
                    .map(|subject| {
                        V::Record(vec![
                            ("name".into(), V::String(format!("row:{subject}"))),
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
        let reversed = [fields[1], fields[0]];
        let other = PreparedFunctions::prepare([&suffixes], &Host, &mut budget()).unwrap();
        for (unit, entries) in [
            (&functions, reversed.as_slice()),
            (&functions, &fields[..1]),
            (&other, fields.as_slice()),
        ] {
            assert!(
                unit.evaluate_projection(
                    &prepared_projection,
                    entries,
                    Value::Subjects(vec![]),
                    &bindings,
                    &mut (),
                    &mut budget(),
                    limits
                )
                .is_err()
            );
        }
        let shadowed = bindings
            .with_local(
                super::at("w".into(), 0),
                Value::Subjects(vec![]),
                &mut budget(),
            )
            .unwrap();
        assert!(
            functions
                .evaluate_projection(
                    &prepared_projection,
                    &fields,
                    Value::Subjects(vec![]),
                    &shadowed,
                    &mut (),
                    &mut budget(),
                    limits
                )
                .is_err()
        );
        for source in ["project(w, {\"a\": 1, \"a\": 2})", "project(w, {\"a\": w})"] {
            let projection = parse(source, &mut budget());
            let mut functions =
                PreparedFunctions::prepare([&suffixes], &Host, &mut budget()).unwrap();
            let entries = projection
                .fields
                .iter()
                .map(|(_, value)| {
                    functions
                        .prepare_value(value, &scope, Some(Kind::Subjects), &mut budget())
                        .unwrap()
                })
                .collect::<Vec<_>>();
            assert!(
                functions
                    .evaluate_projection(
                        &projection,
                        &entries.iter().collect::<Vec<_>>(),
                        Value::Subjects(vec![]),
                        &bindings,
                        &mut (),
                        &mut budget(),
                        limits
                    )
                    .is_err()
            );
        }
        let invalid = parse(
            "project(w, {\"a\": if true {1} else {missing()}})",
            &mut budget(),
        );
        assert!(
            functions
                .prepare_value(
                    &invalid.fields[0].1,
                    &scope,
                    Some(Kind::Subjects),
                    &mut budget()
                )
                .is_err()
        );
        let mut broken = native_function(
            "broken",
            fn_binary(
                zirql::ValueBinaryOperator::IntegerDivide,
                fn_int(1),
                fn_int(0),
            ),
        );
        broken.parameters.clear();
        broken.result = super::at(D::Integer, 4);
        let failure = parse("project(w, {\"a\": broken()})", &mut budget());
        let mut functions = PreparedFunctions::prepare([&broken], &Host, &mut budget()).unwrap();
        let entry = functions
            .prepare_value(
                &failure.fields[0].1,
                &scope,
                Some(Kind::Subjects),
                &mut budget(),
            )
            .unwrap();
        let error = functions
            .evaluate_projection(
                &failure,
                &[&entry],
                Value::Subjects(vec!["alpha"]),
                &bindings,
                &mut (),
                &mut budget(),
                limits,
            )
            .unwrap_err();
        assert_eq!(error.calls[0].function, "broken");
        assert!(
            matches!(error.error, EvaluationError::Core(QueryError { location: Location::Query(range), .. }) if range == broken.value.range)
        );
    }

    #[test]
    fn function_projections_keep_active_locals_nested_rows_and_empty_types() {
        use zirql::{DeclaredType as D, FunctionLimits, PreparedFunctions};
        let projection = parse(
            r#"project(row, {"name": one(row | names), "offset": offset, "nested": row | project(inner, {"name": one(inner | names), "n": n})})"#,
            &mut function_budget(),
        );
        let value = function_value(K::NativeQuery(Box::new(native_operand(Stage::Projection(
            Box::new(projection),
        )))));
        let mut make = native_function("make", value);
        make.bindings.push((
            super::at("offset".into(), 5),
            fn_binary(zirql::ValueBinaryOperator::Add, fn_name("n"), fn_int(1)),
        ));
        let nested = D::Sequence(Box::new(D::Record(vec![
            ("name".into(), D::String),
            ("n".into(), D::Integer),
        ])));
        make.result = super::at(
            D::Sequence(Box::new(D::Record(vec![
                ("name".into(), D::String),
                ("offset".into(), D::Integer),
                ("nested".into(), nested),
            ]))),
            4,
        );
        let call = function_call("make", vec![fn_int(7), fn_name("source")]);
        let mut functions =
            PreparedFunctions::prepare([&make], &Host, &mut function_budget()).unwrap();
        let entry = functions
            .prepare_value(
                &call,
                &BTreeMap::from([("source".into(), Kind::Subjects)]),
                None,
                &mut function_budget(),
            )
            .unwrap();
        for subjects in [vec!["alpha", "beta", "alpha"], vec![]] {
            let bindings = Bindings::new([(
                super::at("source".into(), 0),
                Value::Subjects(subjects.clone()),
            )])
            .unwrap();
            let value = functions
                .evaluate(
                    &entry,
                    &bindings,
                    None,
                    &mut (),
                    &mut function_budget(),
                    FunctionLimits { active_depth: 2 },
                )
                .unwrap();
            let value = Host
                .project_value(value, &mut (), &mut function_budget(), &(0..1))
                .unwrap();
            assert_eq!(
                value.data(),
                &V::Array(
                    subjects
                        .iter()
                        .map(|name| V::Record(vec![
                            ("name".into(), V::String((*name).into())),
                            ("offset".into(), V::Integer(8)),
                            (
                                "nested".into(),
                                V::Array(vec![V::Record(vec![
                                    ("name".into(), V::String((*name).into())),
                                    (
                                        "n".into(),
                                        V::Declared {
                                            value: Box::new(V::Integer(7)),
                                            declaration: D::Integer,
                                        }
                                    ),
                                ])])
                            ),
                        ]))
                        .collect()
                )
            );
        }
        let scope = BTreeMap::from([("xs".into(), Kind::Subjects)]);
        let projection = parse(r#"project(row, {"z": 1})"#, &mut function_budget());
        let value = function_value(K::NativeQuery(Box::new(native_operand(Stage::Projection(
            Box::new(projection),
        )))));
        let entry = functions
            .prepare_value(&value, &scope, None, &mut function_budget())
            .unwrap();
        let expected = vec![ValueType::Array(ArrayType::Elements(vec![
            ValueType::Record(vec![("z".into(), ValueType::Integer)]),
        ]))];
        assert_eq!(*entry.kind(), Host.constructed_kind(expected));
        let bindings =
            Bindings::new([(super::at("xs".into(), 0), Value::Subjects(vec![]))]).unwrap();
        assert_eq!(
            functions
                .evaluate(
                    &entry,
                    &bindings,
                    None,
                    &mut (),
                    &mut function_budget(),
                    FunctionLimits { active_depth: 2 }
                )
                .unwrap(),
            Value::Constructed(V::Array(vec![]))
        );
        for source in [
            r#"project(row, {"z": row})"#,
            r#"project(row, {"z": missing})"#,
        ] {
            let projection = parse(source, &mut function_budget());
            let value = function_value(K::NativeQuery(Box::new(native_operand(
                Stage::Projection(Box::new(projection)),
            ))));
            let mut functions =
                PreparedFunctions::prepare([&make], &Host, &mut function_budget()).unwrap();
            assert!(
                functions
                    .prepare_value(&value, &scope, None, &mut function_budget())
                    .is_err()
            );
        }
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
                Located::new(zirql::PredicateKind::Leaf(Guard::True), 0..1),
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
    fn field_value_arguments_feed_native_stages_with_report_results() {
        use zirql::{FunctionLimits, FunctionRequest};
        let bindings = Bindings::new([(
            Located::new("m".into(), 0..1),
            Value::Capture(NativeCapture {
                fields: vec![("add".into(), Value::Subjects(vec!["alpha"]))],
            }),
        )])
        .unwrap();
        for (text, expected) in [
            ("m.add | names | one", V::String("alpha".into())),
            ("(m.add) | names | one", V::String("alpha".into())),
            (
                "m.add | names union (m.add | names)",
                V::Array(vec![V::String("alpha".into())]),
            ),
        ] {
            let mut allowance = budget();
            let mut reader = zirql::ArgumentReader::new(text, 0).unwrap();
            let value = zirql::read_value_expression(
                &mut reader,
                &mut ArgumentScope(&["m"]),
                &mut allowance,
                32,
            )
            .unwrap();
            assert!(reader.remaining().is_empty());
            let result = FunctionRequest::prepare(
                &[],
                &Host,
                &mut (),
                &mut allowance,
                FunctionLimits { active_depth: 1 },
            )
            .unwrap()
            .evaluate(&value, &bindings)
            .unwrap();
            assert_eq!(result, Value::Constructed(expected));
            zirql::dispose_native_value(value);
        }
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
    fn constructed_word_pipelines_share_the_value_runner_and_preserve_native_routes() {
        let bindings = Bindings::new([(
            super::at("words".into(), 0),
            Value::Constructed(V::Array(
                ["ash", "apple", "birch", "apple"]
                    .into_iter()
                    .map(|word| V::String(word.into()))
                    .collect(),
            )),
        )])
        .unwrap();
        let selected = || {
            V::Array(
                ["ash", "apple", "apple"]
                    .into_iter()
                    .map(|s| V::String(s.into()))
                    .collect(),
            )
        };
        for (source, expected) in [
            (r#"[w for w in words if starts_with(w, "a")]"#, selected()),
            (
                r#"["ash", "apple", "birch", "apple"] | filter(starts_with("a"))"#,
                selected(),
            ),
            (
                r#"["ash", "apple", "birch", "apple"] | filter(starts_with("a")) | count"#,
                V::Integer(3),
            ),
            (
                r#"[w for w in words if starts_with(w, "a")] | unique"#,
                V::Array(vec![V::String("ash".into()), V::String("apple".into())]),
            ),
            (
                r#"[w for w in words if starts_with(w, "a")] | sort | unique"#,
                V::Array(vec![V::String("apple".into()), V::String("ash".into())]),
            ),
            (
                r#"[w for w in words if starts_with(w, "a")] | tally"#,
                V::Record(vec![
                    ("ash".into(), V::Integer(1)),
                    ("apple".into(), V::Integer(2)),
                ]),
            ),
            (
                "words | unique",
                V::Array(
                    ["ash", "apple", "birch"]
                        .into_iter()
                        .map(|s| V::String(s.into()))
                        .collect(),
                ),
            ),
            (
                r#"null default ["ash", "ash"] | tally"#,
                V::Record(vec![("ash".into(), V::Integer(2))]),
            ),
            (
                r#"if true { ["ash", "ash"] } else { ["birch"] } | unique"#,
                V::Array(vec![V::String("ash".into())]),
            ),
        ] {
            let mut allowance = budget();
            let mut reader = zirql::ArgumentReader::new(source, 0).unwrap();
            let value =
                zirql::read_value_expression(&mut reader, &mut SavedNames, &mut allowance, 32)
                    .unwrap();
            assert!(reader.remaining().trim().is_empty(), "{source}");
            assert_eq!(
                zirql::evaluate_native_value(
                    &value,
                    &Host,
                    Value::Subjects(vec![]),
                    &mut (),
                    &bindings,
                    &mut allowance
                )
                .unwrap(),
                expected
            );
            zirql::dispose_native_value(value);
            if let Some(pipe) = source.find(" |") {
                let prefix = zirql::read_value_expression(
                    &mut zirql::ArgumentReader::new(&source[..pipe], 0).unwrap(),
                    &mut SavedNames,
                    &mut allowance,
                    32,
                )
                .unwrap();
                let value = zirql::read_value_suffix(
                    &mut zirql::ArgumentReader::new(source, pipe).unwrap(),
                    &mut SavedNames,
                    prefix,
                    &[],
                    &mut allowance,
                    32,
                )
                .unwrap();
                assert_eq!(
                    zirql::evaluate_native_value(
                        &value,
                        &Host,
                        Value::Subjects(vec![]),
                        &mut (),
                        &bindings,
                        &mut allowance,
                    )
                    .unwrap(),
                    expected
                );
                zirql::dispose_native_value(value);
            }
        }
        for (source, stage) in [("[] | tally | sort", "sort"), ("[] | mystery", "mystery")] {
            let mut allowance = budget();
            let error = zirql::read_value_expression(
                &mut zirql::ArgumentReader::new(source, 0).unwrap(),
                &mut SavedNames,
                &mut allowance,
                32,
            )
            .unwrap_err();
            let prefix = zirql::read_value_expression(
                &mut zirql::ArgumentReader::new("[]", 0).unwrap(),
                &mut SavedNames,
                &mut allowance,
                32,
            )
            .unwrap();
            let suffix_error = zirql::read_value_suffix(
                &mut zirql::ArgumentReader::new(source, 2).unwrap(),
                &mut SavedNames,
                prefix,
                &[],
                &mut allowance,
                32,
            )
            .unwrap_err();
            assert_eq!(error, suffix_error);
            let Location::Query(range) = error.location else {
                panic!("query range")
            };
            assert_eq!(&source[range], stage);
        }
    }

    #[test]
    fn common_reader_index_sets_and_tuple_postfixes_use_shared_values() {
        use zirql::{
            KeyAxis, NominalMember, NominalMemberId, NominalTypeId, SetElementType, ValueKey,
            ValueKeyType,
        };
        assert!(!zirql::is_common_value_call("axes"));
        let member_type = NominalTypeId(7);
        let member = |id, order| NominalMember {
            type_id: member_type,
            member_id: NominalMemberId(id),
            declaration_order: order,
            type_name: "Guests".into(),
            member_name: format!("member{id}"),
        };
        let axis = || KeyAxis {
            key_type: ValueKeyType::Member(member_type),
            keys: vec![
                ValueKey::Member(member(22, 4)),
                ValueKey::Member(member(11, 1)),
            ],
        };
        let keyed = |axes| {
            Value::Constructed(V::KeyedArray {
                axes,
                entries: vec![],
            })
        };
        let bindings = Bindings::new([
            (
                Located::new("guests".into(), 0..0),
                Value::Constructed(V::Array(vec![
                    V::Member(member(22, 4)),
                    V::Member(member(11, 1)),
                ])),
            ),
            (
                Located::new("empty_guests".into(), 0..0),
                Value::Constructed(V::Set {
                    element_type: Some(SetElementType::Member(member_type)),
                    elements: vec![].into(),
                }),
            ),
            (Located::new("keyed".into(), 0..0), keyed(vec![axis()])),
            (
                Located::new("empty".into(), 0..0),
                keyed(vec![KeyAxis {
                    key_type: ValueKeyType::Member(member_type),
                    keys: vec![],
                }]),
            ),
            (
                Located::new("matrix".into(), 0..0),
                keyed(vec![axis(), axis()]),
            ),
        ])
        .unwrap();
        let integer_axis = |keys: &[i64]| KeyAxis {
            key_type: ValueKeyType::Integer,
            keys: keys.iter().copied().map(ValueKey::Integer).collect(),
        };
        let sparse = || V::KeyedArray {
            axes: vec![axis(), integer_axis(&[8, 1, 4])],
            entries: vec![zirql::KeyedEntry {
                key: vec![ValueKey::Member(member(11, 1)), ValueKey::Integer(8)],
                value: V::Integer(8),
            }],
        };
        for (source, expected) in [
            ("[(guests[2], 8): 8] axes(guests, [8, 1, 4])", sparse()),
            (
                "[(g, j): j for g in guests if g == guests[2] for j in [1, 8] if j == 8] axes(guests, [8, 1, 4])",
                sparse(),
            ),
            (
                "[] axes(empty_guests, [8, 1])",
                V::KeyedArray {
                    axes: vec![
                        KeyAxis {
                            key_type: ValueKeyType::Member(member_type),
                            keys: vec![],
                        },
                        integer_axis(&[8, 1]),
                    ],
                    entries: vec![],
                },
            ),
            (
                "[i: i for i in [1] if false] axes([5, 2, 9])",
                V::KeyedArray {
                    axes: vec![integer_axis(&[5, 2, 9])],
                    entries: vec![],
                },
            ),
            (
                "[] axes(set<int>(), [8, 1])",
                V::KeyedArray {
                    axes: vec![integer_axis(&[]), integer_axis(&[8, 1])],
                    entries: vec![],
                },
            ),
            (
                "[(i, j): j for i in set<int>() for j in [8, 1]] axes(set<int>(), [8, 1])",
                V::KeyedArray {
                    axes: vec![integer_axis(&[]), integer_axis(&[8, 1])],
                    entries: vec![],
                },
            ),
            (
                "index_set([2: 20, 5: 50] axes([5, 2, 9]))",
                V::Set {
                    element_type: Some(SetElementType::Integer),
                    elements: vec![V::Integer(5), V::Integer(2), V::Integer(9)].into(),
                },
            ),
            (
                "entries([2: 20, 5: 50] axes([5, 2, 9]))",
                V::Array(vec![
                    V::Record(vec![
                        ("key".into(), V::Integer(5)),
                        ("value".into(), V::Integer(50)),
                    ]),
                    V::Record(vec![
                        ("key".into(), V::Integer(2)),
                        ("value".into(), V::Integer(20)),
                    ]),
                ]),
            ),
            ("[2: 20, 5: 50] axes([5, 2, 9])[2]", V::Integer(20)),
            ("entries([5: 50, 2: 20]) | count", V::Integer(2)),
            (
                "index_set([5: 50, 2: 20])",
                V::Set {
                    element_type: Some(SetElementType::Integer),
                    elements: vec![V::Integer(2), V::Integer(5)].into(),
                },
            ),
            ("has_key([2: 20] axes([5, 2, 9]), 9)", V::Boolean(false)),
            ("[]", V::Array(vec![])),
            (
                "index_set([8, 5])",
                V::Set {
                    element_type: Some(SetElementType::Integer),
                    elements: vec![V::Integer(1), V::Integer(2)].into(),
                },
            ),
            (
                "index_set([])",
                V::Set {
                    element_type: Some(SetElementType::Integer),
                    elements: vec![].into(),
                },
            ),
            (
                "index_set(keyed)",
                V::Set {
                    element_type: Some(SetElementType::Member(member_type)),
                    elements: vec![V::Member(member(22, 4)), V::Member(member(11, 1))].into(),
                },
            ),
            (
                "index_set(empty)",
                V::Set {
                    element_type: Some(SetElementType::Member(member_type)),
                    elements: vec![].into(),
                },
            ),
            (
                r#"({"rows": [((3, 7), 9)]}, 0).1.rows[1].1.2 + 1"#,
                V::Integer(8),
            ),
            (
                "(1, 2).2..4",
                V::Set {
                    element_type: Some(SetElementType::Integer),
                    elements: vec![V::Integer(2), V::Integer(3), V::Integer(4)].into(),
                },
            ),
            ("(1.5, 2.5).1 + 0.5", V::Float(2.0)),
        ] {
            let mut allowance = budget();
            let mut reader = zirql::ArgumentReader::new(source, 0).unwrap();
            let expression =
                zirql::read_value_expression(&mut reader, &mut SavedNames, &mut allowance, 32)
                    .unwrap();
            assert!(reader.remaining().is_empty(), "{source}");
            let result = zirql::evaluate_native_value(
                &expression,
                &Host,
                Value::Subjects(vec![]),
                &mut (),
                &bindings,
                &mut allowance,
            )
            .unwrap();
            assert_eq!(result, expected, "{source}");
            zirql::dispose_native_value(expression);
        }
        for (source, message) in [
            ("[7: 0] axes([1])", "absent from"),
            ("[2: 0, 2: 1] axes([2])", "duplicate"),
            ("[1: 0] axes([1], [2])", "rank"),
            ("[1: 0] axes([1, 1])", "duplicate"),
            ("[1: 0] axes([\"x\"])", "integer"),
            ("[1: 0] axes([])", "unestablished"),
            ("index_set(matrix)", "rank one"),
            ("index_set(1)", "needs an array"),
            ("(1, 2).0", "missing collection position"),
            ("(1, 2).3", "missing collection position"),
            ("[1, 2].1", "needs a tuple"),
            ("if true { 7 } else { index_set(1) }", "needs an array"),
        ] {
            let mut allowance = budget();
            let mut reader = zirql::ArgumentReader::new(source, 0).unwrap();
            let expression =
                zirql::read_value_expression(&mut reader, &mut SavedNames, &mut allowance, 32)
                    .unwrap();
            let error = zirql::evaluate_native_value(
                &expression,
                &Host,
                Value::Subjects(vec![]),
                &mut (),
                &bindings,
                &mut allowance,
            )
            .unwrap_err();
            let EvaluationError::Core(error) = error else {
                panic!("shared error")
            };
            assert!(error.message.contains(message), "{source}: {error}");
            let Location::Query(range) = error.location else {
                panic!("query location")
            };
            let expected = if source == "[7: 0] axes([1])" {
                "7"
            } else if source == "[2: 0, 2: 1] axes([2])" {
                "2"
            } else if source == "[1: 0] axes([1], [2])" {
                "1"
            } else if source == "[1: 0] axes([1, 1])" {
                "[1, 1]"
            } else if source == "[1: 0] axes([\"x\"])" {
                "[\"x\"]"
            } else if source == "[1: 0] axes([])" {
                "[]"
            } else if source.starts_with("if ") {
                "index_set(1)"
            } else {
                source
            };
            assert_eq!(&source[range], expected);
            zirql::dispose_native_value(expression);
        }
        for source in [
            "[] axes()",
            "[] axes([1]) axes([2])",
            "[1] axes([1])",
            "{} axes([1])",
            "index_set()",
            "index_set([], [])",
            "(1, 2).-1",
            "(1, 2).999999999999999999999999999999",
        ] {
            let mut allowance = budget();
            let mut reader = zirql::ArgumentReader::new(source, 0).unwrap();
            let error =
                zirql::read_value_expression(&mut reader, &mut SavedNames, &mut allowance, 32)
                    .unwrap_err();
            assert!(matches!(error.location, Location::Query(_)));
        }
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
                "index_set(keyed)",
                V::Set {
                    element_type: Some(zirql::SetElementType::Integer),
                    elements: vec![V::Integer(9), V::Integer(3)].into(),
                },
            ),
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
            (
                "sort([2, 1, 2, 1])",
                V::Array(vec![
                    V::Integer(1),
                    V::Integer(1),
                    V::Integer(2),
                    V::Integer(2),
                ]),
            ),
            (
                "sort([2, 1, 2, 1], descending=true)",
                V::Array(vec![
                    V::Integer(2),
                    V::Integer(2),
                    V::Integer(1),
                    V::Integer(1),
                ]),
            ),
            ("min([2, 1, 2, 1])", V::Integer(1)),
            ("max([]) default 7", V::Integer(7)),
            (
                "sort_by([{\"id\": \"A\", \"score\": 2}, {\"id\": \"B\", \"score\": 1}], \"score\", descending=false)",
                V::Array(vec![
                    V::Record(vec![
                        ("id".into(), V::String("B".into())),
                        ("score".into(), V::Integer(1)),
                    ]),
                    V::Record(vec![
                        ("id".into(), V::String("A".into())),
                        ("score".into(), V::Integer(2)),
                    ]),
                ]),
            ),
            (
                "min_by([{\"score\": 2}, {\"score\": 1}], \"score\")",
                V::Record(vec![("score".into(), V::Integer(1))]),
            ),
            ("max_by([], \"score\")", V::Absent),
            ("sort", V::Array(vec![V::String("saved".into())])),
            ("sort_by(name)", V::Array(vec![V::String("saved".into())])),
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
            "sort()",
            "sort([], descending=1)",
            "sort([], descending=true, descending=false)",
            "sort([], wrong=true)",
            "sort_by([], 1)",
            "sort_by([])",
            "min([], descending=true)",
            "max_by([], \"score\", true)",
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
    #[test]
    fn public_native_predicate_reader_keeps_scope_offsets_and_reentry_limits() {
        let source = "root(candidate => count_is((candidate | count)) and (_ | count) == limit)";
        let mut allowance = budget();
        let mut reader = zirql::ArgumentReader::new(source, 5).unwrap();
        let target = zirql::read_native_predicate_argument(
            &mut reader,
            &mut Host,
            &["limit"],
            &mut allowance,
            32,
        )
        .unwrap();
        assert_eq!(reader.remaining(), ")");
        assert_eq!(target.range, 5..source.len() - 1);
        let matching = NativeMatch {
            clauses: vec![Located::new(
                zirql::MatchClause::Where { predicate: target },
                5..source.len() - 1,
            )],
            range: 0..source.len(),
        };
        let value = function_value(K::NativeQuery(Box::new(super::sequence(
            native_operand(Stage::Match(Box::new(matching))),
            vec![super::at(ExpressionStep::Stage(Stage::Count), 0)],
        ))));
        let bindings = Bindings::new([
            (
                Located::new("limit".into(), 0..0),
                Value::Constructed(V::Integer(1)),
            ),
            (
                Located::new("xs".into(), 0..0),
                Value::Subjects(vec!["alpha"]),
            ),
        ])
        .unwrap();
        let result = zirql::FunctionRequest::prepare(
            &[],
            &Host,
            &mut (),
            &mut allowance,
            zirql::FunctionLimits { active_depth: 1 },
        )
        .unwrap()
        .evaluate(&value, &bindings)
        .unwrap();
        assert!(matches!(result, Value::Constructed(value) if value.data() == &V::Integer(1)));
        zirql::dispose_native_value(value);

        let source = "root(count_is(1) and (_ | count) == )";
        let error = zirql::read_native_predicate_argument(
            &mut zirql::ArgumentReader::new(source, 5).unwrap(),
            &mut Host,
            &[],
            &mut allowance,
            32,
        )
        .unwrap_err();
        assert_eq!(
            error.location,
            Location::Query(source.len() - 1..source.len())
        );

        let mut allowance = Budget::for_program(ProgramLimits {
            nesting: 3,
            ..super::limits()
        });
        let source = "root(count_is(native(native(f)) | count))";
        let error = zirql::read_native_predicate_argument(
            &mut zirql::ArgumentReader::new(source, 5).unwrap(),
            &mut Host,
            &[],
            &mut allowance,
            32,
        )
        .unwrap_err();
        assert!(error.message.contains("nesting"));
        assert_eq!(error.location, Location::Query(27..28));
        let mut reader = zirql::ArgumentReader::new("root(true)", 5).unwrap();
        let predicate =
            zirql::read_native_predicate_argument(&mut reader, &mut Host, &[], &mut allowance, 32)
                .unwrap();
        assert_eq!(predicate.value, zirql::PredicateKind::Leaf(Guard::True));
    }

    #[test]
    fn native_predicate_headers_are_admitted_before_bare_boolean_leaves() {
        use zirql::{DeclaredType as D, FunctionLimits, FunctionRequest};
        let run = |source: &str| {
            let mut allowance = function_budget();
            let declaration = zirql::read_function_declaration(
                &mut zirql::ArgumentReader::new(source, 0).unwrap(),
                &mut Host,
                &mut allowance,
                64,
                |name, _| match name.value {
                    "Subjects" => Ok(D::Native(Kind::Subjects)),
                    _ => Err(QueryError::query(name.range, "unknown native type")),
                },
            )
            .unwrap()
            .unwrap();
            let entry = function_call(
                "select",
                vec![function_value(K::NativeValue(Box::new(Located::new(
                    ExpressionKind::Binding("original".into()),
                    0..1,
                ))))],
            );
            let bindings = Bindings::new([(
                super::at("original".into(), 0),
                Value::Subjects(vec!["alpha", "beta"]),
            )])
            .unwrap();
            let declarations = [declaration];
            let result = FunctionRequest::prepare(
                &declarations,
                &Host,
                &mut (),
                &mut allowance,
                FunctionLimits { active_depth: 1 },
            )
            .and_then(|request| request.evaluate(&entry, &bindings));
            zirql::dispose_functions(Vec::from(declarations));
            zirql::dispose_native_value(entry);
            result
        };
        for body in ["true", "false", "unknown"] {
            let source = format!(
                "fn select(xs: Subjects) -> int {{ (xs | match {{ from row in input; where [a,b] => {body}; }} | count) }}"
            );
            let error = run(&source).unwrap_err();
            assert!(
                matches!(error.error, EvaluationError::Core(QueryError { location: Location::Query(ref range), ref message, .. }) if &source[range.clone()] == "[a,b]" && message.contains("pattern does not match")),
                "{error:?}"
            );
        }
        let source = "fn select(xs: Subjects) -> int { (xs | match { from row in input; where row => true; } | count) }";
        let error = run(source).unwrap_err();
        assert!(
            matches!(error.error, EvaluationError::Core(QueryError { location: Location::Query(ref range), .. }) if &source[range.clone()] == "row")
        );
        for (header, expected) in [("whole => true", 2), ("_ => false and unknown", 0)] {
            let source = format!(
                "fn select(xs: Subjects) -> int {{ (xs | match {{ from row in input; where {header}; }} | count) }}"
            );
            assert!(
                matches!(run(&source).unwrap(), Value::Constructed(value) if value.data() == &V::Integer(expected))
            );
        }
    }

    #[test]
    fn scoped_native_arguments_keep_body_kinds_and_nearest_subject() {
        use zirql::{FunctionLimits, PatternKind as P, PredicateArgument, PreparedFunctions};
        let named = |name: &str| function_value(K::Name(name.into()));
        let scoped = |name: Option<&str>, subject, body| {
            PredicateArgument {
                argument: name.map(|name| super::at(P::Name(super::at(name.into(), 5)), 5)),
                body: Box::new(body),
            }
            .bind_value(subject)
        };
        let scope = BTreeMap::from([
            ("xs".into(), Kind::Subjects),
            ("capture".into(), Kind::Subjects),
        ]);
        let bindings = Bindings::new([
            (super::at("xs".into(), 0), Value::Subjects(vec!["alpha"])),
            (
                super::at("capture".into(), 0),
                Value::Subjects(vec!["beta"]),
            ),
        ])
        .unwrap();
        let raw = named("xs");
        let inner = scoped(
            None,
            function_value(K::Literal(V::Boolean(true))),
            function_value(K::PredicateSubject),
        );
        let inner_kind = scoped(Some("whole"), named("xs"), inner);
        let nested = scoped(
            Some("whole"),
            named("xs"),
            function_value(K::Block {
                bindings: vec![(
                    super::at(P::Wildcard, 6),
                    scoped(None, named("capture"), function_value(K::PredicateSubject)),
                )],
                value: Box::new(function_value(K::PredicateSubject)),
            }),
        );
        let argument = Arc::new(scoped(Some("candidate"), named("xs"), named("candidate")));
        let bridged = function_value(K::NativeValue(Box::new(native_operand(
            Stage::NativeArgument(Arc::clone(&argument)),
        ))));
        let mut budget = function_budget();
        let mut functions = PreparedFunctions::prepare_patterns([], &Host, &mut budget).unwrap();
        for entry in [&raw, &nested, argument.as_ref(), &bridged] {
            assert_eq!(
                functions
                    .value_kind(entry, &scope, None, &mut budget)
                    .unwrap(),
                Kind::Subjects
            );
            let prepared = functions
                .prepare_value(entry, &scope, None, &mut budget)
                .unwrap();
            assert_eq!(
                functions
                    .evaluate(
                        &prepared,
                        &bindings,
                        None,
                        &mut (),
                        &mut budget,
                        FunctionLimits { active_depth: 1 }
                    )
                    .unwrap(),
                Value::Subjects(vec!["alpha"])
            );
        }
        assert_eq!(
            functions
                .value_kind(&inner_kind, &scope, None, &mut budget)
                .unwrap(),
            Kind::Constructed(Arc::new(vec![ValueType::Boolean]))
        );

        // Preparation refuses incompatible native operands before enumeration.
        let invalid = scoped(
            Some("candidate"),
            named("xs"),
            function_value(K::NativeValue(Box::new(super::sequence(
                Located::new(ExpressionKind::Binding("candidate".into()), 10..19),
                vec![
                    Located::new(ExpressionStep::Stage(Stage::Names), 20..25),
                    Located::new(ExpressionStep::Stage(Stage::Names), 26..31),
                ],
            )))),
        );
        let error = functions
            .value_kind(
                &invalid,
                &BTreeMap::from([("xs".into(), Kind::Subjects)]),
                None,
                &mut budget,
            )
            .unwrap_err();
        assert!(
            matches!(error.error, EvaluationError::Core(QueryError { location: Location::Query(ref range), .. }) if *range == (26..31))
        );
        let prepared = functions
            .prepare_value(&nested, &scope, None, &mut budget)
            .unwrap();
        assert_eq!(
            functions
                .evaluate(
                    &prepared,
                    &bindings,
                    None,
                    &mut (),
                    &mut budget,
                    FunctionLimits { active_depth: 1 }
                )
                .unwrap(),
            Value::Subjects(vec!["alpha"])
        );
    }

    #[test]
    fn scoped_match_predicates_keep_original_input_captures_and_active_functions() {
        use zirql::{DeclaredType as D, FunctionLimits, FunctionRequest};
        let source = r#"fn wanted(n: int) -> bool { n == 2 }
            fn select(xs: Subjects) -> int {
                (xs | match { from row in input; where whole => count_is((_ | count)) and count_is((whole | count)) and count_is((row | count) + 1) and wanted((native(_) | count)) and ((whole | count) == 2 and count_is(2)) and (row | count) == 1 and (_ | count) == 2; } | count)
            }
            fn colliding(xs: Subjects) -> int {
                (xs | match { from row in input; where bound => (bound | count) == 2 and bound | count == 2 and bound(row); } | count)
            }
            select(native(input))"#;
        // The match executes inside select, including its call to wanted.
        let mut allowance = function_budget();
        let mut reader = zirql::ArgumentReader::new(source, 0).unwrap();
        let mut functions = Vec::new();
        for _ in 0..3 {
            functions.push(
                zirql::read_function_declaration(
                    &mut reader,
                    &mut Host,
                    &mut allowance,
                    64,
                    |name, _| {
                        if name.value == "Subjects" {
                            Ok(D::Native(Kind::Subjects))
                        } else {
                            Err(QueryError::query(name.range, "unknown type"))
                        }
                    },
                )
                .unwrap()
                .unwrap(),
            );
        }
        let entry = function_call(
            "select",
            vec![function_value(K::NativeValue(Box::new(Located::new(
                ExpressionKind::Binding("original".into()),
                0..1,
            ))))],
        );
        let bindings = Bindings::new([(
            super::at("original".into(), 0),
            Value::Subjects(vec!["alpha", "beta"]),
        )])
        .unwrap();
        let result = FunctionRequest::prepare(
            &functions,
            &Host,
            &mut (),
            &mut allowance,
            FunctionLimits { active_depth: 3 },
        )
        .unwrap()
        .evaluate(&entry, &bindings)
        .unwrap();
        assert!(matches!(result, Value::Constructed(value) if value.data() == &V::Integer(2)));
        let colliding = function_call(
            "colliding",
            vec![function_value(K::NativeValue(Box::new(Located::new(
                ExpressionKind::Binding("original".into()),
                0..1,
            ))))],
        );
        let result = FunctionRequest::prepare(
            &functions,
            &Host,
            &mut (),
            &mut allowance,
            FunctionLimits { active_depth: 3 },
        )
        .unwrap()
        .evaluate(&colliding, &bindings)
        .unwrap();
        assert!(matches!(result, Value::Constructed(value) if value.data() == &V::Integer(2)));
        zirql::dispose_native_value(colliding);
        let source = r#"["aa", "b", "cc"] | filter(s => wanted(if s == "b" { 1 } else { 2 }))"#;
        let selected = zirql::read_value_expression(
            &mut zirql::ArgumentReader::new(source, 0).unwrap(),
            &mut Host,
            &mut allowance,
            64,
        )
        .unwrap();
        let result = FunctionRequest::prepare(
            &functions,
            &Host,
            &mut (),
            &mut allowance,
            FunctionLimits { active_depth: 3 },
        )
        .unwrap()
        .evaluate(&selected, &Bindings::default())
        .unwrap();
        assert!(
            matches!(result, Value::Constructed(value) if value.data() == &V::Array(vec![V::String("aa".into()), V::String("cc".into())]))
        );
        zirql::dispose_native_value(selected);
        let native_source = function_value(K::Filter {
            array: Box::new(function_value(K::NativeValue(Box::new(Located::new(
                ExpressionKind::Binding("original".into()),
                0..1,
            ))))),
            predicate: zirql::PredicateArgument {
                argument: None,
                body: Box::new(function_value(K::Literal(V::Boolean(true)))),
            },
        });
        let empty =
            Bindings::new([(super::at("original".into(), 0), Value::Subjects(vec![]))]).unwrap();
        assert!(
            FunctionRequest::prepare(
                &functions,
                &Host,
                &mut (),
                &mut allowance,
                FunctionLimits { active_depth: 3 }
            )
            .unwrap()
            .evaluate(&native_source, &empty)
            .is_err()
        );
        zirql::dispose_native_value(native_source);
        let source = "[] | filter(false and (edit | count) == 0)";
        let impure = zirql::read_value_expression(
            &mut zirql::ArgumentReader::new(source, 0).unwrap(),
            &mut Host,
            &mut allowance,
            64,
        )
        .unwrap();
        assert!(
            FunctionRequest::prepare(
                &functions,
                &Host,
                &mut (),
                &mut allowance,
                FunctionLimits { active_depth: 3 }
            )
            .unwrap()
            .evaluate(&impure, &Bindings::default())
            .is_err()
        );
        zirql::dispose_native_value(impure);
        zirql::dispose_functions(functions);
        zirql::dispose_native_value(entry);

        // Ordinary diff/scalar hosts can bind the same native subject without
        // implementing FunctionHost or creating a function session.
        let predicate = zirql::read_predicate_argument(
            &mut zirql::ArgumentReader::new(
                "candidate => (candidate | count) == 2 and (_ | count) == 2",
                0,
            )
            .unwrap(),
            &mut Host,
            &[],
            &mut allowance,
            64,
        )
        .unwrap();
        assert!(
            zirql::evaluate_native_predicate(
                &predicate,
                &Host,
                Value::Subjects(vec!["alpha", "beta"]),
                &mut (),
                &Bindings::default(),
                &mut allowance
            )
            .unwrap()
        );
        zirql::dispose_native_value(*predicate.body);
        let argument = zirql::read_predicate_argument(
            &mut zirql::ArgumentReader::new("candidate => (candidate | names | one)", 0).unwrap(),
            &mut Host,
            &[],
            &mut allowance,
            64,
        )
        .unwrap();
        assert_eq!(
            zirql::evaluate_native_predicate_value(
                &argument,
                &Host,
                Value::Subjects(vec!["alpha"]),
                &mut (),
                &Bindings::default(),
                &mut allowance
            )
            .unwrap()
            .value,
            V::String("alpha".into())
        );
        zirql::dispose_native_value(*argument.body);
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
        fn is_predicate_leaf(&self, _: &str) -> bool {
            true
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
    for source in [
        format!("fixpoint({prefix})"),
        format!("fixpoint({prefix}"),
        format!("where_exists({prefix})"),
        format!("where_exists({prefix}"),
    ] {
        let error = zirql::read_native_expression(
            &mut zirql::ArgumentReader::new(&source, 0).unwrap(),
            &mut host,
            &mut Budget::for_program(generous),
            64,
        )
        .unwrap_err();
        assert!(matches!(error.location, Location::Query(_)));
        assert!(host.0.borrow().upgrade().is_none());
    }
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
    let mut value = value;
    for _ in 0..20_000 {
        value = at(
            zirql::ValueExpressionKind::Loop {
                states: vec![(at("state".into(), 0), value)],
                body: Box::new(at(
                    zirql::ValueExpressionKind::Break(Box::new(at(
                        zirql::ValueExpressionKind::Continue(vec![]),
                        0,
                    ))),
                    0,
                )),
            },
            0,
        );
    }
    zirql::dispose_native_value(value);
    assert!(host.0.borrow().upgrade().is_none());
    let mut reader = zirql::ArgumentReader::new(&prefix, 0).unwrap();
    let guard = zirql::read_value_expression(
        &mut reader,
        &mut host,
        &mut Budget::for_program(generous),
        64,
    )
    .unwrap();
    let mut pattern = at(zirql::PatternKind::Wildcard, 0);
    for _ in 0..20_000 {
        pattern = at(zirql::PatternKind::Present(Box::new(pattern)), 0);
    }
    // Refused callable trees can contain deep patterns and native guard/body
    // edges even though no text frontend accepts their syntax yet.
    zirql::dispose_native_value(at(
        zirql::ValueExpressionKind::ValueMatch {
            subject: Box::new(at(
                zirql::ValueExpressionKind::Literal(zirql::ConstructedValue::Absent),
                0,
            )),
            arms: vec![zirql::ValueMatchArm {
                pattern,
                guard: Some(guard),
                value: at(
                    zirql::ValueExpressionKind::Block {
                        bindings: vec![(
                            at(zirql::PatternKind::Wildcard, 0),
                            at(
                                zirql::ValueExpressionKind::Literal(
                                    zirql::ConstructedValue::Absent,
                                ),
                                0,
                            ),
                        )],
                        value: Box::new(at(
                            zirql::ValueExpressionKind::Literal(zirql::ConstructedValue::Absent),
                            0,
                        )),
                    },
                    0,
                ),
            }],
        },
        0,
    ));
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
        (format!("[{prefix}] | unique |"), "expected an identifier"),
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

#[test]
fn native_pattern_views_keep_child_handles_coordinates_and_guard_failures() {
    use zirql::{
        FunctionLimits, FunctionRequest, PatternFunctionDeclaration, PatternKind as P,
        PatternParameter, ValueExpressionKind as K, ValueMatchArm,
    };
    let p = |kind| at(kind, 70);
    let name = |s: &str| p(P::Name(at(s.to_owned(), 71)));
    let native_name = |s: &str| at(K::Name(s.to_owned()), 40);
    let record_pattern = p(P::Record {
        fields: vec![(
            at("selected".into(), 73),
            p(P::Tuple(vec![name("first"), p(P::Wildcard)])),
        )],
        ignore_rest: true,
    });
    let value_match = |guard| {
        at(
            K::ValueMatch {
                subject: Box::new(at(K::Literal(zirql::ConstructedValue::Absent), 41)),
                arms: vec![
                    ValueMatchArm {
                        pattern: p(P::Absent),
                        guard,
                        value: native_name("first"),
                    },
                    ValueMatchArm {
                        pattern: p(P::Wildcard),
                        guard: None,
                        value: native_name("first"),
                    },
                ],
            },
            42,
        )
    };
    let body = |guard| {
        at(
            K::Block {
                bindings: vec![(
                    p(P::Record {
                        fields: vec![(
                            at("selected".into(), 73),
                            p(P::Tuple(vec![name("first"), p(P::Wildcard)])),
                        )],
                        ignore_rest: true,
                    }),
                    native_name("report"),
                )],
                value: Box::new(value_match(guard)),
            },
            43,
        )
    };
    let report_type = || {
        DeclaredType::Record(vec![
            (
                "selected".into(),
                DeclaredType::Tuple(vec![
                    DeclaredType::Native(SourceKind::Words),
                    DeclaredType::Native(SourceKind::Words),
                ]),
            ),
            ("ignored".into(), DeclaredType::Native(SourceKind::Words)),
        ])
    };
    let function = |value| PatternFunctionDeclaration {
        name: at("select".into(), 1),
        parameters: vec![at(
            PatternParameter {
                pattern: name("report"),
                declaration: report_type(),
            },
            2,
        )],
        result: at(DeclaredType::Native(SourceKind::Words), 3),
        value,
    };
    let source = String::from("alpha beta");
    let foreign = String::from("alpha beta");
    let mut context = SourceContext {
        source: &source,
        prefix: "",
        emissions: vec![],
    };
    let words = |range: Range<usize>| {
        SourceValue::Words(vec![Word {
            side: 0,
            text: &source[range.clone()],
            range,
        }])
    };
    let bindings = Bindings::new([(
        at("input".into(), 1),
        SourceValue::Record(zirql::NativeCapture {
            fields: vec![
                ("ignored".into(), words(6..10)),
                (
                    "selected".into(),
                    SourceValue::Tuple(vec![words(0..5), words(6..10)]),
                ),
            ],
        }),
    )])
    .unwrap();
    let entry = at(
        K::Call {
            function: at("select".into(), 30),
            arguments: vec![at(
                K::NativeValue(Box::new(at(ExpressionKind::Binding("input".into()), 31))),
                31,
            )],
        },
        30,
    );
    let mut budget = Budget::for_program(ProgramLimits {
        work: 1_000_000,
        results: 10_000,
        nesting: 64,
        ..limits()
    });
    let functions = [function(body(None))];
    let result = FunctionRequest::prepare_patterns(
        &functions,
        &SourceHost,
        &mut context,
        &mut budget,
        FunctionLimits { active_depth: 1 },
    )
    .unwrap()
    .evaluate(&entry, &bindings)
    .unwrap();
    let SourceValue::Words(selected) = result else {
        panic!("native words");
    };
    assert_eq!(
        selected,
        vec![Word {
            side: 0,
            text: "alpha",
            range: 0..5
        }]
    );
    assert_eq!(selected[0].text.as_ptr(), source.as_ptr());
    // Matching the same whole native record directly also uses the exposed
    // child kinds in its guard/body, without a function-parameter declaration.
    let functions = [function(at(
        K::ValueMatch {
            subject: Box::new(native_name("report")),
            arms: vec![
                ValueMatchArm {
                    pattern: record_pattern,
                    guard: None,
                    value: native_name("first"),
                },
                ValueMatchArm {
                    pattern: p(P::Wildcard),
                    guard: None,
                    value: at(
                        K::NativeValue(Box::new(at(ExpressionKind::Binding("report".into()), 50))),
                        50,
                    ),
                },
            ],
        },
        44,
    ))];
    // The fallback has the wrong result contract but is never selected.
    let result = FunctionRequest::prepare_patterns(
        &functions,
        &SourceHost,
        &mut context,
        &mut budget,
        FunctionLimits { active_depth: 1 },
    )
    .unwrap()
    .evaluate(&entry, &bindings)
    .unwrap();
    assert!(matches!(result, SourceValue::Words(ref v) if v[0].text.as_ptr() == source.as_ptr()));
    let guard = at(
        K::NativeValue(Box::new(sequence(
            at(ExpressionKind::Binding("first".into()), 60),
            vec![at(ExpressionStep::Stage(SourceStage::Fail), 61)],
        ))),
        60,
    );
    let functions = [function(body(Some(guard)))];
    let error = FunctionRequest::prepare_patterns(
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
        EvaluationError::Host(SourceFailure::Source(0..source.len()))
    );
    let unknown = at(
        K::NativeValue(Box::new(sequence(
            at(ExpressionKind::Binding("first".into()), 60),
            vec![at(ExpressionStep::Stage(SourceStage::UnknownGuard), 61)],
        ))),
        60,
    );
    let unknown_functions = [function(body(Some(unknown)))];
    let error = FunctionRequest::prepare_patterns(
        &unknown_functions,
        &SourceHost,
        &mut context,
        &mut budget,
        FunctionLimits { active_depth: 1 },
    )
    .unwrap()
    .evaluate(&entry, &bindings)
    .unwrap_err();
    assert!(
        matches!(error.error, EvaluationError::Core(ref e) if e.category == QueryErrorCategory::UnknownPredicate && e.location == Location::Query(61..62))
    );
    context.source = &foreign;
    let error = FunctionRequest::prepare_patterns(
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
}

#[test]
fn loop_native_state_keeps_borrowed_identity_aliases_and_failure_cleanup() {
    use zirql::{
        ConstructedValue as V, FunctionLimits, FunctionRequest, ValueBinaryOperator as B,
        ValueExpressionKind as K,
    };
    fn scalar(value: i64) -> zirql::ValueExpression<SourceExpression> {
        at(K::Literal(V::Integer(value)), 82)
    }
    fn name(value: &str) -> zirql::ValueExpression<SourceExpression> {
        at(K::Name(value.into()), 83)
    }
    fn native(value: &str) -> zirql::ValueExpression<SourceExpression> {
        at(
            K::NativeValue(Box::new(at(ExpressionKind::Binding(value.into()), 84))),
            84,
        )
    }
    fn loop_value(
        next: zirql::ValueExpression<SourceExpression>,
        last: zirql::ValueExpression<SourceExpression>,
    ) -> zirql::ValueExpression<SourceExpression> {
        at(
            K::Loop {
                states: vec![
                    (at("held".into(), 80), native("source")),
                    (at("n".into(), 81), scalar(0)),
                ],
                body: Box::new(at(
                    K::Conditional {
                        branches: vec![(
                            at(
                                K::Binary {
                                    operator: B::Equal,
                                    left: Box::new(name("n")),
                                    right: Box::new(scalar(2)),
                                },
                                85,
                            ),
                            at(K::Break(Box::new(name("held"))), 86),
                        )],
                        otherwise: Box::new(at(K::Continue(vec![next, last]), 87)),
                    },
                    85,
                )),
            },
            80,
        )
    }
    let source = String::from("alpha beta");
    let mut context = SourceContext {
        source: &source,
        prefix: "",
        emissions: Vec::new(),
    };
    let mut budget = Budget::for_program(ProgramLimits {
        work: 1_000_000,
        results: 100_000,
        ..limits()
    });
    let bindings = Bindings::new([
        (at("source".into(), 1), source_input(&source, &mut budget)),
        (at("wrong".into(), 2), SourceValue::Count(1)),
    ])
    .unwrap();
    let alias = bindings.get("source", 1..2).unwrap();
    let next_n = || {
        at(
            K::Binary {
                operator: B::Add,
                left: Box::new(name("n")),
                right: Box::new(scalar(1)),
            },
            88,
        )
    };
    let entry = loop_value(name("held"), next_n());
    let result = FunctionRequest::prepare(
        &[],
        &SourceHost,
        &mut context,
        &mut budget,
        FunctionLimits { active_depth: 1 },
    )
    .unwrap()
    .evaluate(&entry, &bindings)
    .unwrap();
    let SourceValue::Words(words) = result else {
        panic!("native loop result")
    };
    assert_eq!(words[0].text.as_ptr(), source.as_ptr());
    assert_eq!(words[1].range, 6..10);
    let failing = loop_value(
        name("held"),
        at(
            K::Binary {
                operator: B::IntegerDivide,
                left: Box::new(scalar(1)),
                right: Box::new(scalar(0)),
            },
            90,
        ),
    );
    let error = FunctionRequest::prepare(
        &[],
        &SourceHost,
        &mut context,
        &mut budget,
        FunctionLimits { active_depth: 1 },
    )
    .unwrap()
    .evaluate(&failing, &bindings)
    .unwrap_err();
    assert!(
        matches!(error.error, EvaluationError::Core(QueryError { location: Location::Query(r), .. }) if r == (90..91))
    );
    let wrong = loop_value(name("wrong"), next_n());
    let error = FunctionRequest::prepare(
        &[],
        &SourceHost,
        &mut context,
        &mut budget,
        FunctionLimits { active_depth: 1 },
    )
    .unwrap()
    .evaluate(&wrong, &bindings)
    .unwrap_err();
    assert!(
        matches!(error.error, EvaluationError::Core(ref e) if e.message.contains("borrowed words"))
    );
    assert_eq!(bindings.get("source", 1..2).unwrap(), alias);
    assert!(
        FunctionRequest::prepare(
            &[],
            &SourceHost,
            &mut context,
            &mut budget,
            FunctionLimits { active_depth: 1 }
        )
        .unwrap()
        .evaluate(&entry, &bindings)
        .is_ok()
    );
    let SourceValue::Words(words) = alias else {
        panic!("retained alias")
    };
    assert_eq!(words[0].text.as_ptr(), source.as_ptr());
    assert_eq!(words[1].range, 6..10);
    zirql::dispose_native_value(entry);
    zirql::dispose_native_value(failing);
    zirql::dispose_native_value(wrong);
}

#[test]
fn prepared_binding_and_program_capture_fields_keep_native_values() {
    use std::collections::BTreeMap;
    use zirql::{FunctionLimits, PreparedFunctions, ProjectionHost, ValueExpressionKind as K};
    fn eq_capable<T: Eq>() {}
    eq_capable::<Program<(), (), ()>>();
    let source = String::from("alpha beta");
    let mut context = SourceContext {
        source: &source,
        prefix: "",
        emissions: Vec::new(),
    };
    let words = |range: Range<usize>| {
        SourceValue::Words(vec![Word {
            text: &source[range.clone()],
            range,
            side: 0,
        }])
    };
    let record = SourceValue::Record(zirql::NativeCapture {
        fields: vec![
            (
                "selected".into(),
                SourceValue::Tuple(vec![words(0..5), words(6..10)]),
            ),
            ("ignored".into(), words(6..10)),
        ],
    });
    let mut budget = Budget::for_program(ProgramLimits {
        work: 100_000,
        results: 10_000,
        nesting: 64,
        ..limits()
    });
    let mut reader = zirql::ArgumentReader::new("{selected: (first, _), ..}", 0).unwrap();
    let original = zirql::read_pattern(&mut reader, &mut budget, 64).unwrap();
    let pattern = zirql::copy_pattern(&original, &mut budget).unwrap();
    assert_eq!(pattern, original);
    let mut scarce = Budget::for_program(ProgramLimits {
        work: 1,
        ..limits()
    });
    assert!(zirql::copy_pattern(&original, &mut scarce).is_err());
    let initializer = at(
        K::NativeValue(Box::new(at(ExpressionKind::Binding("input".into()), 20))),
        20,
    );
    let scope = BTreeMap::from([(
        "input".into(),
        SourceHost.kind(&record, &mut budget, &(20..21)).unwrap(),
    )]);
    let bindings = Bindings::new([(at("input".into(), 1), record)]).unwrap();
    let mut functions = PreparedFunctions::prepare([], &SourceHost, &mut budget).unwrap();
    let mut block_reader = zirql::ArgumentReader::new("{selected: (first, _), ..}", 0).unwrap();
    let block_pattern = zirql::read_pattern(&mut block_reader, &mut budget, 64).unwrap();
    let block = at(
        K::Block {
            bindings: vec![(
                block_pattern,
                at(
                    K::NativeValue(Box::new(at(ExpressionKind::Binding("input".into()), 20))),
                    20,
                ),
            )],
            value: Box::new(at(K::Name("first".into()), 25)),
        },
        20,
    );
    assert_eq!(
        functions
            .value_kind(&block, &scope, None, &mut budget)
            .unwrap(),
        SourceKind::Words
    );
    let prepared = functions
        .prepare_binding(&pattern, &initializer, &scope, None, &mut budget)
        .unwrap();
    let names = prepared
        .bindings()
        .map(|(name, _)| name.clone())
        .collect::<Vec<_>>();
    assert_eq!(names[0].value, "first");
    assert_eq!(prepared.bindings().next().unwrap().1, &SourceKind::Words);
    let capture = functions
        .evaluate_binding(
            &prepared,
            &bindings,
            None,
            &mut context,
            &mut budget,
            FunctionLimits { active_depth: 1 },
        )
        .unwrap();
    let program = Program::new(
        vec![at(
            Statement::BindFields {
                names,
                expression: at(ExpressionKind::Input, 30),
            },
            30,
        )],
        0..40,
    );
    let installed = evaluate_program(
        &program,
        &SourceHost,
        SourceValue::Record(capture),
        &mut context,
        Bindings::default(),
        &mut budget,
    )
    .unwrap();
    let SourceValue::Words(selected) = installed.get("first", 30..31).unwrap() else {
        panic!("native field");
    };
    assert_eq!(selected[0].range, 0..5);
    assert_eq!(selected[0].text.as_ptr(), source.as_ptr());
    let collision = BTreeMap::from([
        ("first".into(), SourceKind::Words),
        ("input".into(), scope["input"].clone()),
    ]);
    assert!(
        functions
            .prepare_binding(&pattern, &initializer, &collision, None, &mut budget)
            .is_err()
    );
    let missing = Program::new(
        vec![at(
            Statement::BindFields {
                names: vec![at("first".into(), 1), at("missing".into(), 2)],
                expression: at(ExpressionKind::Input, 30),
            },
            30,
        )],
        0..40,
    );
    let capture = SourceValue::Record(zirql::NativeCapture {
        fields: vec![("first".into(), words(0..5))],
    });
    assert!(
        evaluate_program(
            &missing,
            &SourceHost,
            capture,
            &mut context,
            Bindings::default(),
            &mut budget
        )
        .is_err()
    );
}

fn worklist_value(
    value: zirql::ValueExpressionKind<SourceExpression>,
) -> zirql::ValueExpression<SourceExpression> {
    at(value, 70)
}
fn worklist_name(name: &str) -> zirql::ValueExpression<SourceExpression> {
    worklist_value(zirql::ValueExpressionKind::Name(name.into()))
}
fn word_type() -> Located<zirql::SignatureType> {
    at(zirql::SignatureType::Named(at("Word".into(), 71)), 71)
}
fn word_at(source: &str, range: Range<usize>, side: u64) -> SourceValue<'_> {
    SourceValue::Words(vec![Word {
        text: &source[range.clone()],
        range,
        side,
    }])
}
fn worklist_query(
    binding: &str,
    stage: SourceStage,
    projected: bool,
) -> zirql::ValueExpression<SourceExpression> {
    let query = sequence(
        at(ExpressionKind::Binding(binding.into()), 72),
        vec![at(ExpressionStep::Stage(stage), 73)],
    );
    worklist_value(if projected {
        zirql::ValueExpressionKind::NativeQuery(Box::new(query))
    } else {
        zirql::ValueExpressionKind::NativeValue(Box::new(query))
    })
}
fn graph_functions() -> Vec<zirql::FunctionDeclaration<SourceExpression, SourceKind>> {
    use zirql::{
        ConstructedValue as V, PatternKind as P, ValueBinaryOperator as B, ValueExpressionKind as K,
    };
    let v = worklist_value;
    let n = worklist_name;
    let boolean = |b| v(K::Literal(V::Boolean(b)));
    let integer = |i| v(K::Literal(V::Integer(i)));
    let exit = |found| {
        v(K::Break(Box::new(v(K::Tuple(vec![
            boolean(found),
            n("expanded"),
        ])))))
    };
    let body = v(K::ValueMatch {
        subject: Box::new(v(K::PopFront(Box::new(n("todo"))))),
        arms: vec![
            zirql::ValueMatchArm {
                pattern: at(
                    P::Present(Box::new(at(
                        P::Tuple(vec![
                            at(P::Name(at("candidate".into(), 74)), 74),
                            at(P::Name(at("rest".into(), 75)), 75),
                        ]),
                        74,
                    ))),
                    74,
                ),
                guard: None,
                value: v(K::Conditional {
                    branches: vec![(
                        worklist_query("candidate", SourceStage::Same, true),
                        exit(true),
                    )],
                    otherwise: Box::new(v(K::Conditional {
                        branches: vec![(
                            v(K::Binary {
                                operator: B::In,
                                left: Box::new(n("candidate")),
                                right: Box::new(n("seen")),
                            }),
                            v(K::Continue(vec![n("rest"), n("seen"), n("expanded")])),
                        )],
                        otherwise: Box::new(v(K::Continue(vec![
                            v(K::PushBackAll {
                                queue: Box::new(n("rest")),
                                items: Box::new(worklist_query(
                                    "candidate",
                                    SourceStage::Neighbors,
                                    false,
                                )),
                            }),
                            v(K::Insert {
                                set: Box::new(n("seen")),
                                item: Box::new(n("candidate")),
                            }),
                            v(K::Binary {
                                operator: B::Add,
                                left: Box::new(n("expanded")),
                                right: Box::new(integer(1)),
                            }),
                        ]))),
                    })),
                }),
            },
            zirql::ValueMatchArm {
                pattern: at(P::Wildcard, 76),
                guard: None,
                value: exit(false),
            },
        ],
    });
    let parameters = || {
        ["start", "target"]
            .into_iter()
            .map(|name| {
                at(
                    zirql::FunctionParameter {
                        name: at(name.into(), 77),
                        declaration: DeclaredType::Native(SourceKind::Words),
                    },
                    77,
                )
            })
            .collect()
    };
    vec![
        zirql::FunctionDeclaration {
            name: at("walk".into(), 78),
            parameters: parameters(),
            bindings: vec![],
            result: at(
                DeclaredType::Tuple(vec![DeclaredType::Boolean, DeclaredType::Integer]),
                78,
            ),
            value: v(K::Loop {
                states: vec![
                    (
                        at("todo".into(), 79),
                        v(K::Queue {
                            items: Box::new(v(K::Array(vec![n("start")]))),
                            element: None,
                        }),
                    ),
                    (
                        at("seen".into(), 80),
                        v(K::IdentitySet {
                            element: word_type(),
                        }),
                    ),
                    (at("expanded".into(), 81), integer(0)),
                ],
                body: Box::new(body),
            }),
        },
        zirql::FunctionDeclaration {
            name: at("reaches".into(), 82),
            parameters: parameters(),
            bindings: vec![],
            result: at(DeclaredType::Boolean, 82),
            value: v(K::TupleAccess {
                value: Box::new(v(K::Call {
                    function: at("walk".into(), 83),
                    arguments: vec![n("start"), n("target")],
                })),
                position: at(1, 84),
            }),
        },
    ]
}
fn worklist_limits() -> ProgramLimits {
    ProgramLimits {
        work: 10_000_000,
        results: 1_000_000,
        nesting: 64,
        stream_items: 10_000,
    }
}

#[test]
fn callable_native_worklist_reachability_handles_cycles_duplicates_and_zero_edges() {
    use zirql::{ConstructedValue as V, FunctionLimits, FunctionRequest, ValueExpressionKind as K};
    let source = String::from("A B C D E");
    let functions = graph_functions();
    for (function, target, expected) in [
        ("reaches", 6, V::Boolean(true)),
        ("reaches", 8, V::Boolean(false)),
        ("reaches", 0, V::Boolean(true)),
        ("walk", 8, V::Tuple(vec![V::Boolean(false), V::Integer(4)])),
    ] {
        let mut context = SourceContext {
            source: &source,
            prefix: "",
            emissions: vec![],
        };
        let mut budget = Budget::for_program(worklist_limits());
        let bindings = Bindings::new([
            (at("start".into(), 1), word_at(&source, 0..1, 0)),
            (
                at("target".into(), 2),
                word_at(&source, target..target + 1, 0),
            ),
        ])
        .unwrap();
        let entry = worklist_value(K::Call {
            function: at(function.into(), 85),
            arguments: vec![worklist_name("start"), worklist_name("target")],
        });
        let result = FunctionRequest::prepare(
            &functions,
            &SourceHost,
            &mut context,
            &mut budget,
            FunctionLimits { active_depth: 2 },
        )
        .unwrap()
        .evaluate(&entry, &bindings)
        .unwrap();
        let SourceValue::Constructed(actual) = result else {
            panic!("constructed reachability");
        };
        match (actual.data(), &expected) {
            (V::Tuple(a), V::Tuple(b)) => {
                assert_eq!(a.len(), b.len());
                for (a, b) in a.iter().zip(b) {
                    assert_eq!(a.data(), b.data());
                }
            }
            (a, b) => assert_eq!(a, b),
        }
    }
}

#[test]
fn native_collections_keep_fifo_contracts_identities_aliases_and_current_context() {
    use zirql::{
        ComputationalValue as W, ConstructedValue as V, FunctionHost, FunctionLimits,
        FunctionRequest, NativeIdentitySet, NativeQueue, ValueExpressionKind as K,
    };
    let source = String::from("A B C D E A");
    let foreign = String::from("A B C D E A");
    let mut context = SourceContext {
        source: &source,
        prefix: "",
        emissions: vec![],
    };
    let mut budget = Budget::for_program(worklist_limits());
    let location = Location::Query(90..91);
    let element = DeclaredType::Native(SourceKind::Words);
    let queue = NativeQueue::new(
        vec![word_at(&source, 0..1, 0), word_at(&source, 2..3, 0)],
        element.clone(),
        &SourceHost,
        &mut context,
        &mut budget,
        location.clone(),
    )
    .unwrap();
    let alias = queue
        .share(&SourceHost, &mut context, &mut budget, location.clone())
        .unwrap();
    let queue = queue
        .push_back_all(
            vec![word_at(&source, 4..5, 0)],
            &SourceHost,
            &mut context,
            &mut budget,
            location.clone(),
        )
        .unwrap();
    let mut queue = queue;
    for offset in [0, 2, 4] {
        let (item, rest) = queue
            .pop_front(&SourceHost, &mut context, &mut budget, location.clone())
            .unwrap()
            .unwrap();
        assert_eq!(item.as_ref(), &word_at(&source, offset..offset + 1, 0));
        queue = rest;
    }
    assert!(
        queue
            .pop_front(&SourceHost, &mut context, &mut budget, location.clone())
            .unwrap()
            .is_none()
    );
    assert_eq!(alias.len(), 2);
    let mut seen = NativeIdentitySet::new(
        SourceKind::Words,
        &SourceHost,
        &mut context,
        &mut budget,
        location.clone(),
    )
    .unwrap();
    seen = seen
        .insert(
            word_at(&source, 0..1, 0),
            &SourceHost,
            &mut context,
            &mut budget,
            location.clone(),
        )
        .unwrap();
    let earlier = seen
        .share(&SourceHost, &mut context, &mut budget, location.clone())
        .unwrap();
    seen = seen
        .insert(
            word_at(&source, 0..1, 0),
            &SourceHost,
            &mut context,
            &mut budget,
            location.clone(),
        )
        .unwrap();
    assert_eq!(seen.len(), 1);
    assert!(
        !seen
            .contains(
                &word_at(&source, 10..11, 0),
                &SourceHost,
                &mut context,
                &mut budget,
                location.clone()
            )
            .unwrap()
    );
    assert!(
        !seen
            .contains(
                &word_at(&source, 0..1, 1),
                &SourceHost,
                &mut context,
                &mut budget,
                location.clone()
            )
            .unwrap()
    );
    seen = seen
        .insert(
            word_at(&source, 10..11, 0),
            &SourceHost,
            &mut context,
            &mut budget,
            location.clone(),
        )
        .unwrap();
    assert_eq!(seen.len(), 2);
    assert_eq!(earlier.len(), 1);
    let wrapper = SourceHost
        .lift_computational(W::Queue(alias), &mut context, &mut budget, &(90..91))
        .unwrap();
    let declaration = DeclaredType::Queue(Box::new(element.clone()));
    zirql::admit_value(
        &wrapper,
        &declaration,
        &SourceHost,
        &mut context,
        &mut budget,
        location.clone(),
    )
    .unwrap();
    let wrong = DeclaredType::Queue(Box::new(DeclaredType::Integer));
    assert!(
        zirql::admit_value(
            &wrapper,
            &wrong,
            &SourceHost,
            &mut context,
            &mut budget,
            location.clone()
        )
        .is_err()
    );
    assert!(
        <SourceHost as zirql::ProjectionHost<_>>::project_value(
            &SourceHost,
            SourceHost
                .copy_value(&wrapper, &mut context, &mut budget, &(90..91))
                .unwrap(),
            &mut context,
            &mut budget,
            &(90..91)
        )
        .is_err()
    );
    context.source = &foreign;
    let error = zirql::admit_value(
        &wrapper,
        &declaration,
        &SourceHost,
        &mut context,
        &mut budget,
        location.clone(),
    )
    .unwrap_err();
    assert!(
        matches!(error, EvaluationError::Core(ref e) if e.location == location && e.message.contains("context"))
    );
    context.source = &source;
    let empty = worklist_value(K::Queue {
        items: Box::new(worklist_value(K::Array(vec![]))),
        element: Some(word_type()),
    });
    let entry = worklist_value(K::PopFront(Box::new(empty)));
    let result = FunctionRequest::prepare(
        &[],
        &SourceHost,
        &mut context,
        &mut budget,
        FunctionLimits { active_depth: 4 },
    )
    .unwrap()
    .evaluate(&entry, &Bindings::default())
    .unwrap();
    assert!(matches!(result, SourceValue::Constructed(V::Absent)));
    // A constructed queue item extracted from a native pop tuple stays usable
    // by ordinary scalar operations.
    let item = worklist_value(K::Binary {
        operator: zirql::ValueBinaryOperator::Add,
        left: Box::new(worklist_value(K::Literal(V::Integer(19)))),
        right: Box::new(worklist_value(K::Literal(V::Integer(21)))),
    });
    let scalar_queue = worklist_value(K::Queue {
        items: Box::new(worklist_value(K::Array(vec![item]))),
        element: None,
    });
    let scalar_pop = worklist_value(K::Block {
        bindings: vec![(
            at(
                zirql::PatternKind::Present(Box::new(at(
                    zirql::PatternKind::Tuple(vec![
                        at(zirql::PatternKind::Name(at("number".into(), 95)), 95),
                        at(zirql::PatternKind::Wildcard, 96),
                    ]),
                    95,
                ))),
                95,
            ),
            worklist_value(K::PopFront(Box::new(scalar_queue))),
        )],
        value: Box::new(worklist_value(K::Binary {
            operator: zirql::ValueBinaryOperator::Add,
            left: Box::new(worklist_name("number")),
            right: Box::new(worklist_value(K::Literal(V::Integer(2)))),
        })),
    });
    let result = FunctionRequest::prepare(
        &[],
        &SourceHost,
        &mut context,
        &mut budget,
        FunctionLimits { active_depth: 1 },
    )
    .unwrap()
    .evaluate(&scalar_pop, &Bindings::default())
    .unwrap();
    assert!(matches!(result, SourceValue::Constructed(V::Integer(42))));
    let retained = worklist_value(K::Block {
        bindings: vec![(
            at(zirql::PatternKind::Name(at("held".into(), 97)), 97),
            worklist_value(K::Queue {
                items: Box::new(worklist_value(K::Array(vec![]))),
                element: Some(word_type()),
            }),
        )],
        value: Box::new(worklist_name("held")),
    });
    let result = FunctionRequest::prepare(
        &[],
        &SourceHost,
        &mut context,
        &mut budget,
        FunctionLimits { active_depth: 1 },
    )
    .unwrap()
    .evaluate(&retained, &Bindings::default())
    .unwrap();
    assert!(
        matches!(result, SourceValue::Computational(c) if matches!(c.as_ref(), W::Queue(q) if q.is_empty()))
    );
}

#[test]
fn queue_inference_uses_selected_input_and_refuses_untyped_empty_input() {
    use zirql::{
        ComputationalValue as W, ConstructedValue as V, FunctionLimits, FunctionRequest,
        ValueExpressionKind as K,
    };
    let mut context = SourceContext {
        source: "",
        prefix: "",
        emissions: vec![],
    };
    let mut budget = Budget::for_program(worklist_limits());
    let selected_queue = |selected| {
        worklist_value(K::Queue {
            items: Box::new(worklist_value(K::Conditional {
                branches: vec![(
                    worklist_value(K::Literal(V::Boolean(false))),
                    worklist_value(K::Array(vec![worklist_value(K::Literal(V::Integer(1)))])),
                )],
                otherwise: Box::new(worklist_value(K::Array(selected))),
            })),
            element: None,
        })
    };
    let selected = selected_queue(vec![worklist_value(K::Literal(V::String("chosen".into())))]);
    let result = FunctionRequest::prepare(
        &[],
        &SourceHost,
        &mut context,
        &mut budget,
        FunctionLimits { active_depth: 1 },
    )
    .unwrap()
    .evaluate(&selected, &Bindings::default())
    .unwrap();
    let SourceValue::Computational(value) = result else {
        panic!("queue");
    };
    let W::Queue(queue) = value.as_ref() else {
        panic!("queue");
    };
    assert_eq!(queue.element(), &DeclaredType::String);
    let (item, _) = queue
        .share(
            &SourceHost,
            &mut context,
            &mut budget,
            Location::Query(90..91),
        )
        .unwrap()
        .pop_front(
            &SourceHost,
            &mut context,
            &mut budget,
            Location::Query(90..91),
        )
        .unwrap()
        .unwrap();
    assert!(matches!(item.as_ref(), SourceValue::Constructed(V::String(s)) if s == "chosen"));
    let empty = selected_queue(vec![]);
    let error = FunctionRequest::prepare(
        &[],
        &SourceHost,
        &mut context,
        &mut budget,
        FunctionLimits { active_depth: 1 },
    )
    .unwrap()
    .evaluate(&empty, &Bindings::default())
    .unwrap_err();
    assert!(
        matches!(error.error, EvaluationError::Core(e) if e.message.contains("empty queue needs"))
    );
}

#[test]
fn native_constructed_children_remain_shared_until_owned_output() {
    use zirql::{
        ConstructedValue as V, FunctionLimits, FunctionRequest, PatternKind as P,
        ValueExpressionKind as K,
    };
    let bindings = Bindings::new([
        (
            at("ignored".into(), 1),
            SourceValue::Tuple(vec![SourceValue::Constructed(V::String(
                "x".repeat(65_536),
            ))]),
        ),
        (
            at("partial".into(), 2),
            SourceValue::Tuple(vec![SourceValue::Constructed(V::Record(vec![
                (
                    "payload".into(),
                    V::Record(vec![
                        ("small".into(), V::Integer(40)),
                        (
                            "ignored".into(),
                            V::Array(vec![V::String("x".repeat(65_536))]),
                        ),
                    ]),
                ),
                ("other".into(), V::String("x".repeat(65_536))),
            ]))]),
        ),
    ])
    .unwrap();
    let mut context = SourceContext {
        source: "",
        prefix: "",
        emissions: vec![],
    };
    let mut evaluate = |entry: &zirql::ValueExpression<SourceExpression>| {
        let mut budget = Budget::for_program(ProgramLimits {
            work: 10_000,
            results: 10_000,
            nesting: 64,
            stream_items: 100,
        });
        FunctionRequest::prepare(
            &[],
            &SourceHost,
            &mut context,
            &mut budget,
            FunctionLimits { active_depth: 4 },
        )
        .unwrap()
        .evaluate(entry, &bindings)
    };
    let ignored = worklist_value(K::Block {
        bindings: vec![(
            at(P::Tuple(vec![at(P::Wildcard, 92)]), 91),
            worklist_name("ignored"),
        )],
        value: Box::new(worklist_value(K::Literal(V::Boolean(true)))),
    });
    assert!(matches!(
        evaluate(&ignored).unwrap(),
        SourceValue::Constructed(V::Boolean(true))
    ));
    let selected = |name| {
        worklist_value(K::TupleAccess {
            value: Box::new(worklist_name(name)),
            position: at(1, 93),
        })
    };
    let partial = worklist_value(K::Block {
        bindings: vec![(
            at(
                P::Record {
                    fields: vec![(
                        at("payload".into(), 94),
                        at(
                            P::Record {
                                fields: vec![(
                                    at("small".into(), 95),
                                    at(P::Name(at("number".into(), 96)), 96),
                                )],
                                ignore_rest: true,
                            },
                            94,
                        ),
                    )],
                    ignore_rest: true,
                },
                94,
            ),
            selected("partial"),
        )],
        value: Box::new(worklist_value(K::Binary {
            operator: zirql::ValueBinaryOperator::Add,
            left: Box::new(worklist_name("number")),
            right: Box::new(worklist_value(K::Literal(V::Integer(2)))),
        })),
    });
    assert!(matches!(
        evaluate(&partial).unwrap(),
        SourceValue::Constructed(V::Integer(42))
    ));
    let field = |value, label: &str| {
        worklist_value(K::ArrayAccess {
            array: Box::new(value),
            index: Box::new(worklist_value(K::Literal(V::String(label.into())))),
        })
    };
    let computed = worklist_value(K::Binary {
        operator: zirql::ValueBinaryOperator::Add,
        left: Box::new(field(field(selected("partial"), "payload"), "small")),
        right: Box::new(worklist_value(K::Literal(V::Integer(2)))),
    });
    assert!(matches!(
        evaluate(&computed).unwrap(),
        SourceValue::Constructed(V::Integer(42))
    ));
    // Returning an owned large selected value still spends its copy allowance.
    let error = evaluate(&selected("ignored")).unwrap_err();
    assert!(
        matches!(error.error, EvaluationError::Core(e) if e.category == QueryErrorCategory::WorkLimit)
    );
    let step = zirql::FunctionDeclaration {
        name: at("selected_scalar".into(), 1),
        parameters: vec![
            at(
                zirql::FunctionParameter {
                    name: at("state".into(), 2),
                    declaration: DeclaredType::Integer,
                },
                2,
            ),
            at(
                zirql::FunctionParameter {
                    name: at("item".into(), 3),
                    declaration: DeclaredType::Tuple(vec![DeclaredType::Integer]),
                },
                3,
            ),
        ],
        result: at(DeclaredType::Integer, 4),
        bindings: vec![],
        value: selected("item"),
    };
    let bindings = Bindings::new([(
        at("items".into(), 5),
        SourceValue::Sequence {
            values: vec![SourceValue::Tuple(vec![SourceValue::Constructed(
                V::Integer(42),
            )])],
            element: DeclaredType::Tuple(vec![DeclaredType::Integer]),
        },
    )])
    .unwrap();
    let scan = worklist_value(K::Scan {
        items: Box::new(worklist_name("items")),
        initial: Box::new(worklist_value(K::Literal(V::Integer(0)))),
        step: at("selected_scalar".into(), 6),
    });
    let mut budget = Budget::for_program(worklist_limits());
    let result = FunctionRequest::prepare(
        &[step],
        &SourceHost,
        &mut context,
        &mut budget,
        FunctionLimits { active_depth: 2 },
    )
    .unwrap()
    .evaluate(&scan, &bindings)
    .unwrap();
    let SourceValue::Constructed(value) = result else {
        panic!("constructed scalar scan");
    };
    let V::Array(items) = value.data() else {
        panic!("scan sequence");
    };
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].data(), &V::Integer(42));
}

#[test]
fn prepared_binding_and_kind_queries_keep_worklist_contracts() {
    use std::collections::BTreeMap;
    use zirql::{
        ComputationalValue as W, ConstructedValue as V, FunctionLimits, PreparedFunctions,
        ValueExpressionKind as K,
    };
    let queue = |value| {
        worklist_value(K::Queue {
            items: Box::new(worklist_value(K::Array(vec![worklist_value(K::Literal(
                value,
            ))]))),
            element: None,
        })
    };
    let integers = queue(V::Integer(1));
    let strings = queue(V::String("chosen".into()));
    let pattern = at(zirql::PatternKind::Name(at("held".into(), 90)), 90);
    let scope = BTreeMap::new();
    let bindings = Bindings::default();
    let mut context = SourceContext {
        source: "",
        prefix: "",
        emissions: vec![],
    };
    let mut budget = Budget::for_program(worklist_limits());
    let mut functions = PreparedFunctions::prepare([], &SourceHost, &mut budget).unwrap();
    let binding = functions
        .prepare_binding(&pattern, &integers, &scope, None, &mut budget)
        .unwrap();
    assert_eq!(
        functions
            .value_kind(&strings, &scope, None, &mut budget)
            .unwrap(),
        SourceKind::Computational(Box::new(DeclaredType::Queue(Box::new(
            DeclaredType::String
        ))))
    );
    let strings = functions
        .prepare_value(&strings, &scope, None, &mut budget)
        .unwrap();
    let capture = functions
        .evaluate_binding(
            &binding,
            &bindings,
            None,
            &mut context,
            &mut budget,
            FunctionLimits { active_depth: 1 },
        )
        .unwrap();
    let SourceValue::Computational(value) = &capture.fields[0].1 else {
        panic!("queue");
    };
    assert!(
        matches!(value.as_ref(), W::Queue(q) if q.element() == &DeclaredType::Integer && q.len() == 1)
    );
    let value = functions
        .evaluate(
            &strings,
            &bindings,
            None,
            &mut context,
            &mut budget,
            FunctionLimits { active_depth: 1 },
        )
        .unwrap();
    assert!(
        matches!(value, SourceValue::Computational(c) if matches!(c.as_ref(), W::Queue(q) if q.element() == &DeclaredType::String && q.len() == 1))
    );
}

#[test]
fn prepared_worklists_keep_contextual_empty_optional_and_scan_snapshots() {
    use zirql::{
        ConstructedValue as V, FunctionDeclaration as F, FunctionLimits, FunctionParameter as P,
        FunctionRequest, ValueExpressionKind as K,
    };
    let source = String::from("A B C");
    let mut context = SourceContext {
        source: &source,
        prefix: "",
        emissions: vec![],
    };
    let mut budget = Budget::for_program(worklist_limits());
    let v = worklist_value;
    let n = worklist_name;
    let word = || DeclaredType::Native(SourceKind::Words);
    let queue = || DeclaredType::Queue(Box::new(word()));
    let pop = || DeclaredType::Optional(Box::new(DeclaredType::Tuple(vec![word(), queue()])));
    let parameter = |name: &str, d| {
        at(
            P {
                name: at(name.into(), 1),
                declaration: d,
            },
            1,
        )
    };
    let functions = vec![
        F {
            name: at("empty".into(), 1),
            parameters: vec![],
            result: at(queue(), 2),
            bindings: vec![],
            value: v(K::Queue {
                items: Box::new(v(K::Array(vec![]))),
                element: None,
            }),
        },
        F {
            name: at("peek".into(), 3),
            parameters: vec![parameter("q", queue())],
            result: at(pop(), 4),
            bindings: vec![],
            value: v(K::PopFront(Box::new(n("q")))),
        },
        F {
            name: at("enqueue".into(), 5),
            parameters: vec![parameter("q", queue()), parameter("item", word())],
            result: at(queue(), 6),
            bindings: vec![],
            value: v(K::PushBackAll {
                queue: Box::new(n("q")),
                items: Box::new(v(K::Array(vec![n("item")]))),
            }),
        },
    ];
    let empty_call = v(K::Call {
        function: at("empty".into(), 9),
        arguments: vec![],
    });
    let empty_pop = v(K::Call {
        function: at("peek".into(), 9),
        arguments: vec![v(K::Queue {
            items: Box::new(v(K::Array(vec![]))),
            element: None,
        })],
    });
    let absent = FunctionRequest::prepare(
        &functions,
        &SourceHost,
        &mut context,
        &mut budget,
        FunctionLimits { active_depth: 2 },
    )
    .unwrap()
    .evaluate(&empty_pop, &Bindings::default())
    .unwrap();
    assert!(matches!(absent, SourceValue::Constructed(V::Absent)));
    let empty = FunctionRequest::prepare(
        &functions,
        &SourceHost,
        &mut context,
        &mut budget,
        FunctionLimits { active_depth: 2 },
    )
    .unwrap()
    .evaluate(&empty_call, &Bindings::default())
    .unwrap();
    assert!(
        matches!(&empty, SourceValue::Computational(c) if matches!(c.as_ref(), zirql::ComputationalValue::Queue(q) if q.is_empty()))
    );
    let bindings = Bindings::new([
        (
            at("items".into(), 10),
            SourceValue::Sequence {
                values: vec![word_at(&source, 2..3, 0), word_at(&source, 4..5, 0)],
                element: word(),
            },
        ),
        (at("start".into(), 11), word_at(&source, 0..1, 0)),
    ])
    .unwrap();
    let scan = v(K::Scan {
        items: Box::new(n("items")),
        initial: Box::new(v(K::Queue {
            items: Box::new(v(K::Array(vec![n("start")]))),
            element: None,
        })),
        step: at("enqueue".into(), 12),
    });
    let scan = FunctionRequest::prepare(
        &functions,
        &SourceHost,
        &mut context,
        &mut budget,
        FunctionLimits { active_depth: 1 },
    )
    .unwrap()
    .evaluate(&scan, &bindings)
    .unwrap();
    let SourceValue::Sequence { values, element } = scan else {
        panic!("native scan");
    };
    assert_eq!(element, queue());
    assert_eq!(values.len(), 2);
    for (value, expected) in values.into_iter().zip([vec![0, 2], vec![0, 2, 4]]) {
        let SourceValue::Computational(c) = value else {
            panic!("queue snapshot");
        };
        let zirql::ComputationalValue::Queue(q) = c.as_ref() else {
            panic!("queue");
        };
        let mut q = q
            .share(
                &SourceHost,
                &mut context,
                &mut budget,
                Location::Query(13..14),
            )
            .unwrap();
        for i in expected {
            let (item, rest) = q
                .pop_front(
                    &SourceHost,
                    &mut context,
                    &mut budget,
                    Location::Query(13..14),
                )
                .unwrap()
                .unwrap();
            assert_eq!(item.as_ref(), &word_at(&source, i..i + 1, 0));
            q = rest;
        }
        assert!(q.is_empty());
    }
}

#[test]
fn worklist_refusals_keep_locations_budget_and_retained_admission_context() {
    use zirql::{
        ConstructedValue as V, FunctionHost, FunctionLimits, FunctionRequest,
        ValueExpressionKind as K,
    };
    let source = String::from("A B");
    let mut context = SourceContext {
        source: &source,
        prefix: "",
        emissions: vec![],
    };
    let mut budget = Budget::for_program(worklist_limits());
    let missing = worklist_value(K::Queue {
        items: Box::new(worklist_value(K::Array(vec![]))),
        element: None,
    });
    let scalar_set = worklist_value(K::IdentitySet {
        element: at(zirql::SignatureType::Integer, 91),
    });
    for entry in [&missing, &scalar_set] {
        let e = FunctionRequest::prepare(
            &[],
            &SourceHost,
            &mut context,
            &mut budget,
            FunctionLimits { active_depth: 1 },
        )
        .unwrap()
        .evaluate(entry, &Bindings::default())
        .unwrap_err();
        assert!(
            matches!(e.error, EvaluationError::Core(ref e) if matches!(e.location, Location::Query(_)))
        );
    }
    let empty = zirql::NativeQueue::new(
        vec![],
        DeclaredType::Native(SourceKind::Words),
        &SourceHost,
        &mut context,
        &mut budget,
        Location::Query(92..93),
    )
    .unwrap();
    let bindings = Bindings::new([(
        at("input".into(), 1),
        SourceHost
            .lift_computational(
                zirql::ComputationalValue::Queue(empty),
                &mut context,
                &mut budget,
                &(92..93),
            )
            .unwrap(),
    )])
    .unwrap();
    let d = DeclaredType::Queue(Box::new(DeclaredType::Native(SourceKind::Words)));
    let function = zirql::FunctionDeclaration {
        name: at("invalidate".into(), 2),
        parameters: vec![at(
            zirql::FunctionParameter {
                name: at("q".into(), 3),
                declaration: d.clone(),
            },
            3,
        )],
        result: at(d, 4),
        bindings: vec![(
            at("changed".into(), 5),
            worklist_query("q", SourceStage::Invalidate, true),
        )],
        value: worklist_name("q"),
    };
    let entry = worklist_value(K::Call {
        function: at("invalidate".into(), 6),
        arguments: vec![worklist_name("input")],
    });
    let e = FunctionRequest::prepare(
        &[function],
        &SourceHost,
        &mut context,
        &mut budget,
        FunctionLimits { active_depth: 1 },
    )
    .unwrap()
    .evaluate(&entry, &bindings)
    .unwrap_err();
    assert!(
        matches!(e.error, EvaluationError::Core(ref e) if e.location == Location::Query(70..71) && e.message.contains("context"))
    );
    context.source = &source;
    let location = Location::Query(94..95);
    let mut small = Budget::for_program(ProgramLimits {
        results: 1,
        ..worklist_limits()
    });
    let e = zirql::NativeQueue::new(
        vec![word_at(&source, 0..1, 0)],
        DeclaredType::Native(SourceKind::Words),
        &SourceHost,
        &mut context,
        &mut small,
        location.clone(),
    )
    .unwrap_err();
    assert!(
        matches!(e, EvaluationError::Core(ref e) if e.location == location && e.category == QueryErrorCategory::ResultLimit)
    );
    let mut small = Budget::for_program(ProgramLimits {
        work: 1,
        ..worklist_limits()
    });
    let e = zirql::NativeIdentitySet::new(
        SourceKind::Words,
        &SourceHost,
        &mut context,
        &mut small,
        location.clone(),
    )
    .unwrap_err();
    assert!(
        matches!(e, EvaluationError::Core(ref e) if e.location == location && e.category == QueryErrorCategory::WorkLimit)
    );
    let okay = worklist_value(K::Literal(V::Boolean(true)));
    assert!(matches!(
        FunctionRequest::prepare(
            &[],
            &SourceHost,
            &mut context,
            &mut budget,
            FunctionLimits { active_depth: 1 }
        )
        .unwrap()
        .evaluate(&okay, &Bindings::default())
        .unwrap(),
        SourceValue::Constructed(V::Boolean(true))
    ));
}

#[test]
fn common_reader_executes_loops_named_reducers_and_native_worklist_calls() {
    use zirql::{ConstructedValue as V, FunctionLimits, FunctionRequest, ValueExpressionKind as K};
    struct ReaderHost;
    impl zirql::ValueReaderHost for ReaderHost {
        type Stage = SourceStage;
        type SetOperator = ();
        type Emitter = &'static str;
        type Predicate = Truth;
        fn is_binding(&self, _: &str) -> bool {
            false
        }
        fn is_stage(&self, _: &str) -> bool {
            false
        }
        fn retains_native_values(&self) -> bool {
            true
        }
        fn set_operator(&self, _: &str) -> Option<()> {
            None
        }

        fn parse_stage(
            &mut self,
            reader: &mut zirql::ArgumentReader<'_>,
            _: &mut Budget,
        ) -> Result<ExpressionStep<SourceStage, &'static str>, QueryError> {
            Err(reader.error("no domain stages"))
        }
        fn is_predicate_leaf(&self, _: &str) -> bool {
            true
        }
        fn parse_predicate_leaf(
            &mut self,
            reader: &mut zirql::ArgumentReader<'_>,
            _: &mut Budget,
        ) -> Result<Truth, QueryError> {
            Err(reader.error("no domain predicates"))
        }
        fn filter(&mut self, _: zirql::Predicate<Truth>, _: Range<usize>) -> SourceStage {
            unreachable!("no domain predicates")
        }
        fn projection(
            &mut self,
            _: zirql::Projection<SourceExpression>,
            _: Range<usize>,
        ) -> SourceStage {
            unreachable!("no domain projections")
        }
    }
    let text = r#"
fn add((count, total): tuple<int,int>, item: int) -> tuple<int,int> {
    (count + 1, total + item)
}
fn append(state: queue<Word>, item: Word) -> queue<Word> {
    push_back_all(state, [item])
}
{
  "fold": fold([2,3,5], (0,0), add),
  "scan": scan([2,3,5], (0,0), add),
  "walk": loop (pending=queue<Word>(nodes), seen=identity_set<Word>(), count=0) {
    match (pop_front(pending)) {
      present((current, rest)) => {
        if current in seen { continue (rest, seen, count); }
        else { continue (rest, insert(seen,current), count+1); }
      },
      _ => break count
    }
  }
}
"#;
    let source = String::from("A B C");
    let mut context = SourceContext {
        source: &source,
        prefix: "",
        emissions: vec![],
    };
    let mut budget = Budget::for_program(ProgramLimits {
        work: 1_000_000,
        results: 100_000,
        stream_items: 100,
        nesting: 64,
    });
    let bindings = Bindings::new([(
        at("nodes".into(), 0),
        SourceValue::Sequence {
            values: vec![
                word_at(&source, 0..1, 0),
                word_at(&source, 2..3, 0),
                word_at(&source, 0..1, 0),
                word_at(&source, 4..5, 0),
            ],
            element: DeclaredType::Native(SourceKind::Words),
        },
    )])
    .unwrap();
    let mut reader = zirql::ArgumentReader::new(text, 0).unwrap();
    let mut declarations = Vec::new();
    while let Some(declaration) = zirql::read_pattern_function_declaration(
        &mut reader,
        &mut ReaderHost,
        &mut budget,
        64,
        |name, _| {
            if name.value == "Word" {
                Ok(DeclaredType::Native(SourceKind::Words))
            } else {
                Err(QueryError::query(name.range, "unknown native type"))
            }
        },
    )
    .unwrap()
    {
        declarations.push(declaration);
    }
    let entry =
        zirql::read_value_expression(&mut reader, &mut ReaderHost, &mut budget, 64).unwrap();
    assert!(reader.remaining().is_empty());
    let result = FunctionRequest::prepare_patterns(
        &declarations,
        &SourceHost,
        &mut context,
        &mut budget,
        FunctionLimits { active_depth: 1 },
    )
    .unwrap()
    .evaluate(&entry, &bindings)
    .unwrap();
    let SourceValue::Constructed(value) = result else {
        panic!("constructed report")
    };
    let V::Record(fields) = value.data() else {
        panic!("report fields")
    };
    assert_eq!(
        fields
            .iter()
            .map(|(name, _)| name.as_str())
            .collect::<Vec<_>>(),
        ["fold", "scan", "walk"]
    );
    assert_eq!(
        fields[0].1.data(),
        &V::Tuple(vec![V::Integer(3), V::Integer(10)])
    );
    let V::Array(states) = fields[1].1.data() else {
        panic!("scalar scan")
    };
    for (state, (count, total)) in states.iter().zip([(1, 2), (2, 5), (3, 10)]) {
        assert_eq!(
            state.data(),
            &V::Tuple(vec![V::Integer(count), V::Integer(total)])
        );
    }
    assert_eq!(states.len(), 3);
    assert_eq!(fields[2].1.data(), &V::Integer(3));
    zirql::dispose_native_value(entry);
    let text = "scan(nodes, queue<Word>([]), append)";
    let mut reader = zirql::ArgumentReader::new(text, 0).unwrap();
    let entry =
        zirql::read_value_expression(&mut reader, &mut ReaderHost, &mut budget, 64).unwrap();
    let result = FunctionRequest::prepare_patterns(
        &declarations,
        &SourceHost,
        &mut context,
        &mut budget,
        FunctionLimits { active_depth: 1 },
    )
    .unwrap()
    .evaluate(&entry, &bindings)
    .unwrap();
    let SourceValue::Sequence { values, .. } = result else {
        panic!("native scan snapshots")
    };
    assert_eq!(values.len(), 4);
    for (value, length) in [(&values[0], 1), (&values[3], 4)] {
        assert!(
            matches!(value, SourceValue::Computational(value) if matches!(value.as_ref(), zirql::ComputationalValue::Queue(queue) if queue.len()==length))
        );
    }
    zirql::dispose_native_value(entry);
    for text in ["queue", "loop", "fold"] {
        let mut reader = zirql::ArgumentReader::new(text, 0).unwrap();
        let entry =
            zirql::read_value_expression(&mut reader, &mut ReaderHost, &mut budget, 64).unwrap();
        assert!(matches!(&entry.value, K::Name(name) if name == text));
        zirql::dispose_native_value(entry);
    }
    let mut reader = zirql::ArgumentReader::new("queue < 3", 0).unwrap();
    let comparison =
        zirql::read_value_expression(&mut reader, &mut ReaderHost, &mut budget, 64).unwrap();
    assert!(
        matches!(&comparison.value, K::Binary { operator: zirql::ValueBinaryOperator::Less, left, .. } if matches!(&left.value, K::Name(name) if name=="queue"))
    );
    zirql::dispose_native_value(comparison);
    for text in ["fold([],0,add(1))", "identity_set<Word>(1)"] {
        let mut reader = zirql::ArgumentReader::new(text, 0).unwrap();
        let error = zirql::read_value_expression(&mut reader, &mut ReaderHost, &mut budget, 64)
            .unwrap_err();
        assert!(matches!(error.location, Location::Query(_)));
    }
    let mut reader = zirql::ArgumentReader::new("loop (n=0) { break n; }", 0).unwrap();
    let mut small = Budget::for_program(ProgramLimits {
        results: 1,
        ..limits()
    });
    let error =
        zirql::read_value_expression(&mut reader, &mut ReaderHost, &mut small, 64).unwrap_err();
    assert_eq!(error.category, QueryErrorCategory::ResultLimit);
}

#[test]
fn native_ordering_keeps_borrowed_coordinates_and_common_reader_semantics() {
    struct Reader;
    impl zirql::ValueReaderHost for Reader {
        type Stage = SourceStage;
        type SetOperator = ();
        type Emitter = &'static str;
        type Predicate = ();
        fn is_binding(&self, name: &str) -> bool {
            name == "saved"
        }
        fn is_stage(&self, name: &str) -> bool {
            name == "count"
        }
        fn parse_stage(
            &mut self,
            reader: &mut zirql::ArgumentReader<'_>,
            _: &mut Budget,
        ) -> Result<ExpressionStep<SourceStage, &'static str>, QueryError> {
            let word = reader.word()?;
            if word.value != "count" {
                return Err(QueryError::query(word.range, "unsupported stage"));
            }
            Ok(ExpressionStep::Stage(SourceStage::Pipeline(Pipeline::new(
                vec![Located::new(Stage::Count, word.range.clone())],
                word.range,
                16,
            )?)))
        }
        fn is_predicate_leaf(&self, _: &str) -> bool {
            true
        }
        fn parse_predicate_leaf(
            &mut self,
            reader: &mut zirql::ArgumentReader<'_>,
            _: &mut Budget,
        ) -> Result<(), QueryError> {
            Err(reader.error("unsupported predicate"))
        }
        fn filter(&mut self, _: zirql::Predicate<()>, _: Range<usize>) -> SourceStage {
            unreachable!()
        }
        fn projection(
            &mut self,
            _: zirql::Projection<SourceExpression>,
            _: Range<usize>,
        ) -> SourceStage {
            unreachable!()
        }
        fn ordering(
            &mut self,
            ordering: zirql::NativeOrdering<SourceExpression>,
            _: Range<usize>,
        ) -> Result<SourceStage, QueryError> {
            Ok(SourceStage::Order(Box::new(ordering)))
        }
        fn where_exists(
            &mut self,
            body: SourceExpression,
            _: Range<usize>,
        ) -> Result<SourceStage, QueryError> {
            Ok(SourceStage::Exists(Box::new(body)))
        }
        fn fixed_point(
            &mut self,
            body: SourceExpression,
            _: Range<usize>,
        ) -> Result<SourceStage, QueryError> {
            Ok(SourceStage::FixedPoint(Box::new(body)))
        }
        fn set_operator(&self, word: &str) -> Option<()> {
            zirql::NativeSetOperator::from_keyword(word).map(|_| ())
        }
    }
    let saved = zirql::read_native_expression(
        &mut zirql::ArgumentReader::new("(saved union saved) | count", 0).unwrap(),
        &mut Reader,
        &mut Budget::for_program(limits()),
        16,
    )
    .unwrap();
    let ExpressionKind::Sequence { input, .. } = &saved.value else {
        panic!("pipeline")
    };
    let ExpressionKind::Group(group) = &input.value else {
        panic!("group")
    };
    assert!(matches!(group.value, ExpressionKind::Set { .. }));
    zirql::dispose_expression(saved);
    let source = "fn selected(local: Word) -> Word { where_exists(local) }";
    let declaration = zirql::read_pattern_function_declaration(
        &mut zirql::ArgumentReader::new(source, 0).unwrap(),
        &mut Reader,
        &mut Budget::for_program(limits()),
        16,
        |_, _| Ok(DeclaredType::Native(SourceKind::Words)),
    )
    .unwrap()
    .unwrap();
    let zirql::ValueExpressionKind::NativeQuery(query) = &declaration.value.value else {
        panic!("native function body")
    };
    let ExpressionKind::Sequence { steps, .. } = &query.value else {
        panic!("existence pipeline")
    };
    let ExpressionStep::Stage(SourceStage::Exists(body)) = &steps[0].value else {
        panic!("retained relation")
    };
    let ExpressionKind::Sequence { input, .. } = &body.value else {
        panic!("relation input")
    };
    assert!(matches!(&input.value, ExpressionKind::Binding(name) if name == "local"));
    assert_eq!(&source[input.range.clone()], "local");
    let source = "βeta alpha alpha";
    let input = || {
        SourceValue::Words(vec![
            Word {
                side: 7,
                text: &source[0..5],
                range: 0..5,
            },
            Word {
                side: 7,
                text: &source[6..11],
                range: 6..11,
            },
            Word {
                side: 7,
                text: &source[12..17],
                range: 12..17,
            },
        ])
    };
    let mut context = SourceContext {
        source,
        prefix: "",
        emissions: vec![],
    };
    for (query, expected) in [
        ("where_exists(input)", vec![(0, 5), (6, 11), (12, 17)]),
        ("(where_exists(input))", vec![(0, 5), (6, 11), (12, 17)]),
        (
            "input | where_exists(where_exists(input))",
            vec![(0, 5), (6, 11), (12, 17)],
        ),
        ("fixpoint(input)", vec![(0, 5), (6, 11), (12, 17)]),
        ("input | fixpoint(input)", vec![(0, 5), (6, 11), (12, 17)]),
        ("fixpoint(fixpoint(input))", vec![(0, 5), (6, 11), (12, 17)]),
        ("sort", vec![(6, 11), (12, 17), (0, 5)]),
        ("sort_by(input)", vec![(6, 11), (12, 17), (0, 5)]),
        ("min", vec![(6, 11)]),
        ("max", vec![(0, 5)]),
        ("min_all", vec![(6, 11), (12, 17)]),
        ("max_all", vec![(0, 5)]),
        ("min_by(count)", vec![(0, 5)]),
        ("max_by(count)", vec![(0, 5)]),
        ("min_all_by(count)", vec![(0, 5), (6, 11), (12, 17)]),
        ("max_all_by(count)", vec![(0, 5), (6, 11), (12, 17)]),
    ] {
        let mut budget = Budget::for_program(limits());
        let expression = zirql::read_native_expression(
            &mut zirql::ArgumentReader::new(query, 0).unwrap(),
            &mut Reader,
            &mut budget,
            16,
        )
        .unwrap();
        let SourceValue::Words(words) = evaluate_expression(
            &expression,
            &SourceHost,
            input(),
            &mut context,
            &Bindings::default(),
            &mut budget,
        )
        .unwrap() else {
            panic!("words")
        };
        assert_eq!(
            words
                .iter()
                .map(|word| (word.range.start, word.range.end))
                .collect::<Vec<_>>(),
            expected
        );
        for word in words {
            assert_eq!(word.side, 7);
            assert_eq!(word.text.as_ptr(), source[word.range.clone()].as_ptr());
        }
    }
    for query in [
        "fixpoint(",
        "fixpoint(input",
        "fixpoint()",
        "where_exists(",
        "where_exists(input",
        "where_exists()",
    ] {
        assert!(
            zirql::read_native_expression(
                &mut zirql::ArgumentReader::new(query, 0).unwrap(),
                &mut Reader,
                &mut Budget::for_program(limits()),
                16,
            )
            .is_err()
        );
    }
    let typed = SourceStage::Order(Box::new(zirql::NativeOrdering {
        order: zirql::NativeOrder::MinAll,
        selector: None,
    }));
    let mut budget = Budget::for_program(limits());
    let SourceValue::Words(words) = ProgramHost::evaluate_stage(
        &SourceHost,
        &typed,
        input(),
        &mut context,
        &Bindings::default(),
        &mut budget,
        &(21..28),
    )
    .unwrap() else {
        panic!("words")
    };
    assert_eq!(
        words
            .iter()
            .map(|word| (word.range.start, word.range.end))
            .collect::<Vec<_>>(),
        [(6, 11), (12, 17)]
    );
    for (budget, category) in [
        (
            Budget::new(zirql::Limits {
                work: 2,
                items: 8,
                nesting: 16,
            }),
            QueryErrorCategory::WorkLimit,
        ),
        (
            Budget::for_program(ProgramLimits {
                results: 1,
                ..limits()
            }),
            QueryErrorCategory::ResultLimit,
        ),
    ] {
        let mut budget = budget;
        let error = ProgramHost::evaluate_stage(
            &SourceHost,
            &typed,
            input(),
            &mut context,
            &Bindings::default(),
            &mut budget,
            &(21..28),
        )
        .unwrap_err();
        assert!(
            matches!(error, EvaluationError::Core(QueryError { location: Location::Query(range), category: actual, .. }) if range == (21..28) && actual == category)
        );
    }
    let mut budget = Budget::for_program(limits());
    let ordered = zirql::order_native_by(
        vec!["text", "count", "tie"],
        zirql::NativeOrder::Sort,
        &mut budget,
        Location::Query(0..1),
        |item, _| {
            Ok::<_, EvaluationError<SourceFailure>>(if *item == "text" {
                zirql::NativeOrderKey::Text(String::new())
            } else {
                zirql::NativeOrderKey::Count(usize::MAX)
            })
        },
    )
    .unwrap();
    assert_eq!(ordered, ["count", "tie", "text"]);

    for (order, expected) in [
        (zirql::NativeOrder::Sort, vec![1, 2, 0]),
        (zirql::NativeOrder::Min, vec![1]),
        (zirql::NativeOrder::Max, vec![0]),
        (zirql::NativeOrder::MinAll, vec![1, 2]),
        (zirql::NativeOrder::MaxAll, vec![0]),
    ] {
        let ordered = zirql::order_native_by(
            vec![0, 1, 2],
            order,
            &mut Budget::for_program(limits()),
            Location::Query(21..28),
            |index, _| {
                Ok::<_, EvaluationError<SourceFailure>>(zirql::NativeOrderKey::Value(
                    zirql::ConstructedValue::Integer([7, 2, 2][*index]),
                ))
            },
        )
        .unwrap();
        assert_eq!(ordered, expected);
    }
    let member = |id, order, name: &str| {
        zirql::ConstructedValue::Member(zirql::NominalMember {
            type_id: zirql::NominalTypeId(9),
            member_id: zirql::NominalMemberId(id),
            declaration_order: order,
            type_name: "Size".into(),
            member_name: name.into(),
        })
    };
    for keys in [
        vec![
            zirql::ConstructedValue::Integer(1),
            zirql::ConstructedValue::Float(2.0),
        ],
        vec![
            zirql::ConstructedValue::Integer(1),
            zirql::ConstructedValue::Absent,
        ],
        vec![zirql::ConstructedValue::Boolean(true)],
        vec![zirql::ConstructedValue::Float(f64::INFINITY)],
        // Both contradictory members lose to the first key.
        vec![
            member(1, 0, "small"),
            member(2, 1, "large"),
            member(2, 2, "other"),
        ],
    ] {
        let error = zirql::order_native_by(
            keys,
            zirql::NativeOrder::Min,
            &mut Budget::for_program(limits()),
            Location::Query(21..28),
            |key, budget| {
                Ok::<_, EvaluationError<SourceFailure>>(zirql::NativeOrderKey::Value(
                    zirql::copy_constructed_value(key, budget, Location::Query(21..28))?,
                ))
            },
        )
        .unwrap_err();
        assert!(
            matches!(error, EvaluationError::Core(QueryError { location: Location::Query(range), .. }) if range == (21..28))
        );
    }
    for keys in [
        vec![
            zirql::ConstructedValue::Float(2.5),
            zirql::ConstructedValue::Float(-1.0),
        ],
        vec![
            zirql::ConstructedValue::String("z".into()),
            zirql::ConstructedValue::String("a".into()),
        ],
        vec![member(2, 1, "a"), member(1, 0, "z")],
    ] {
        let ordered = zirql::order_native_by(
            vec![0, 1],
            zirql::NativeOrder::Sort,
            &mut Budget::for_program(limits()),
            Location::Query(21..28),
            |index, budget| {
                Ok::<_, EvaluationError<SourceFailure>>(zirql::NativeOrderKey::Value(
                    zirql::copy_constructed_value(&keys[*index], budget, Location::Query(21..28))?,
                ))
            },
        )
        .unwrap();
        assert_eq!(ordered, [1, 0]);
    }
}

#[test]
fn native_fixed_points_replace_ordered_integer_selections_with_bounded_checkpoints() {
    fn copy(
        value: &[u8],
        _: &mut (),
        budget: &mut Budget,
    ) -> Result<Vec<u8>, EvaluationError<QueryError>> {
        let location = Location::Query(20..30);
        budget.check_items(value.len(), location.clone())?;
        budget.charge(value.len() + 1, location.clone())?;
        budget.charge_results(value.len() + 1, location)?;
        Ok(value.to_vec())
    }
    fn same(
        left: &[u8],
        right: &[u8],
        _: &mut (),
        budget: &mut Budget,
    ) -> Result<bool, EvaluationError<QueryError>> {
        zirql::ordered_native_equal(left, right, budget, Location::Query(20..30), |a, b, _| {
            Ok(a == b)
        })
    }
    fn run(
        values: Vec<u8>,
        budget: &mut Budget,
        mut next: impl FnMut(
            &[u8],
            &mut (),
            &mut Budget,
        ) -> Result<Vec<u8>, EvaluationError<QueryError>>,
    ) -> Result<Vec<u8>, EvaluationError<QueryError>> {
        zirql::evaluate_native_fixed_point(
            values,
            &mut (),
            budget,
            Location::Query(20..30),
            |value, context, budget| next(value, context, budget),
            |value, context, budget| copy(value, context, budget),
            |left, right, context, budget| same(left, right, context, budget),
        )
    }
    let unchanged = run(vec![2, 1, 1], &mut Budget::for_program(limits()), copy).unwrap();
    assert_eq!(unchanged, [2, 1, 1]);
    let empty = run(
        vec![2, 1, 1],
        &mut Budget::for_program(limits()),
        |current, _, budget| {
            budget.charge(1, Location::Query(20..30))?;
            Ok(current.get(1..).unwrap_or_default().to_vec())
        },
    )
    .unwrap();
    assert!(empty.is_empty());
    let cycle = run(
        vec![2, 1, 1],
        &mut Budget::for_program(limits()),
        |current, context, budget| {
            let mut result = copy(current, context, budget)?;
            result.reverse();
            Ok(result)
        },
    )
    .unwrap_err();
    assert!(
        matches!(cycle, EvaluationError::Core(QueryError { location: Location::Query(range), message, .. }) if range == (20..30) && message.contains("cycles"))
    );
    assert!(!same(&[1, 1], &[1], &mut (), &mut Budget::for_program(limits())).unwrap());
    assert!(
        !same(
            &[1, 2],
            &[2, 1],
            &mut (),
            &mut Budget::for_program(limits())
        )
        .unwrap()
    );
    for (mut budget, category) in [
        (
            Budget::new(zirql::Limits {
                work: 1,
                items: 8,
                nesting: 16,
            }),
            QueryErrorCategory::WorkLimit,
        ),
        (
            Budget::for_program(ProgramLimits {
                results: 1,
                ..limits()
            }),
            QueryErrorCategory::ResultLimit,
        ),
        (
            Budget::for_program(ProgramLimits {
                stream_items: 1,
                ..limits()
            }),
            QueryErrorCategory::ItemLimit,
        ),
    ] {
        let error = run(vec![1, 2], &mut budget, copy).unwrap_err();
        assert!(
            matches!(error, EvaluationError::Core(QueryError { location: Location::Query(range), category: actual, .. }) if range == (20..30) && actual == category)
        );
    }
    let mut budget = Budget::new(zirql::Limits {
        work: 0,
        items: 8,
        nesting: 16,
    });
    let error = zirql::ordered_native_equal::<u8, QueryError>(
        &[],
        &[],
        &mut budget,
        Location::Query(20..30),
        |a, b, _| Ok(a == b),
    )
    .unwrap_err();
    assert!(matches!(
        error,
        EvaluationError::Core(QueryError {
            category: QueryErrorCategory::WorkLimit,
            ..
        })
    ));
}

#[test]
fn shared_native_body_spellings_are_reserved_for_functions() {
    let value = || {
        at(
            zirql::ValueExpressionKind::Literal(zirql::ConstructedValue::Integer(0)),
            4,
        )
    };
    for name in ["fixpoint", "where_exists"] {
        let declaration = zirql::FunctionDeclaration {
            name: at(name.into(), 1),
            parameters: vec![],
            result: at(DeclaredType::Integer, 2),
            bindings: vec![],
            value: value(),
        };
        let error = zirql::evaluate_functions(
            &[declaration],
            &value(),
            &Bindings::default(),
            &mut Budget::for_program(limits()),
            zirql::FunctionLimits { active_depth: 1 },
        )
        .unwrap_err();
        assert!(error.to_string().contains("reserved operation"));
    }
}

#[test]
fn native_set_kind_admission_reaches_unused_value_branches() {
    use zirql::{ConstructedValue as V, ValueExpressionKind as K};
    let set = Located::new(
        ExpressionKind::Set {
            operator: (),
            left: Box::new(Located::new(
                ExpressionKind::Binding("count".into()),
                21..26,
            )),
            right: Box::new(Located::new(
                ExpressionKind::Binding("count".into()),
                33..38,
            )),
        },
        21..38,
    );
    let expression = Located::new(
        K::Conditional {
            branches: vec![(
                Located::new(K::Literal(V::Boolean(true)), 0..4),
                Located::new(K::Literal(V::Integer(1)), 5..6),
            )],
            otherwise: Box::new(Located::new(K::NativeQuery(Box::new(set)), 21..38)),
        },
        0..38,
    );
    let error = zirql::evaluate_native_value(
        &expression,
        &SourceHost,
        SourceValue::Words(vec![]),
        &mut SourceContext {
            source: "",
            prefix: "",
            emissions: vec![],
        },
        &Bindings::new(vec![(
            Located::new("count".into(), 0..1),
            SourceValue::Count(0),
        )])
        .unwrap(),
        &mut Budget::for_program(limits()),
    )
    .unwrap_err();
    assert!(
        matches!(&error, EvaluationError::Core(QueryError { location: Location::Query(range), message, .. }) if range == &(21..38) && message == "sets require word identities"),
        "{error:?}"
    );
}

#[test]
fn native_list_patterns_share_selected_owners_with_ordinary_predicates() {
    use zirql::{
        ConstructedValue as V, FunctionLimits, FunctionRequest, PatternKind as P,
        PredicateArgument, ValueExpressionKind as K,
    };
    let source = String::from("alpha beta");
    let mut context = SourceContext {
        source: &source,
        prefix: "",
        emissions: vec![],
    };
    let mut budget = Budget::for_program(worklist_limits());
    let subject = || SourceValue::Sequence {
        values: vec![word_at(&source, 0..5, 0), word_at(&source, 6..10, 0)],
        element: DeclaredType::Native(SourceKind::Words),
    };
    let prefix = || {
        at(
            P::List {
                elements: vec![at(P::Name(at("candidate".into(), 71)), 70)],
                ignore_rest: true,
            },
            70,
        )
    };
    let same = || {
        at(
            K::NativeQuery(Box::new(sequence(
                at(ExpressionKind::Binding("candidate".into()), 72),
                vec![at(ExpressionStep::Stage(SourceStage::Same), 72)],
            ))),
            72,
        )
    };
    let predicate = PredicateArgument {
        argument: Some(prefix()),
        body: Box::new(same()),
    };
    zirql::native_predicate_types(
        &predicate,
        &SourceHost,
        SourceKind::Sequence(Box::new(DeclaredType::Native(SourceKind::Words))),
        std::collections::BTreeMap::from([("target".into(), SourceKind::Words)]),
        &mut budget,
    )
    .unwrap();
    let captures = Bindings::new([(at("target".into(), 2), word_at(&source, 0..5, 0))]).unwrap();
    assert!(
        zirql::evaluate_native_predicate(
            &predicate,
            &SourceHost,
            subject(),
            &mut context,
            &captures,
            &mut budget
        )
        .unwrap()
    );
    let scalar = PredicateArgument {
        argument: Some(prefix()),
        body: Box::new(at(K::Name("candidate".into()), 72)),
    };
    assert_eq!(
        zirql::evaluate_native_predicate_value(
            &scalar,
            &SourceHost,
            SourceValue::Sequence {
                values: vec![SourceValue::Constructed(V::Integer(4))],
                element: DeclaredType::Integer
            },
            &mut context,
            &Bindings::default(),
            &mut budget
        )
        .unwrap()
        .value,
        V::Integer(4)
    );
    let bindings = Bindings::new([
        (at("input".into(), 1), subject()),
        (at("target".into(), 2), word_at(&source, 0..5, 0)),
    ])
    .unwrap();
    let entry = predicate.bind(at(
        K::NativeValue(Box::new(at(ExpressionKind::Binding("input".into()), 2))),
        2,
    ));
    let result = FunctionRequest::prepare_patterns(
        &[],
        &SourceHost,
        &mut context,
        &mut budget,
        FunctionLimits { active_depth: 1 },
    )
    .unwrap()
    .evaluate(&entry, &bindings)
    .unwrap();
    assert!(matches!(result, SourceValue::Constructed(v) if v.data() == &V::Boolean(true)));
    let function = zirql::PatternFunctionDeclaration {
        name: at("head".into(), 1),
        parameters: vec![at(
            zirql::PatternParameter {
                pattern: prefix(),
                declaration: DeclaredType::Sequence(Box::new(DeclaredType::Native(
                    SourceKind::Words,
                ))),
            },
            3,
        )],
        result: at(DeclaredType::Native(SourceKind::Words), 4),
        value: at(K::Name("candidate".into()), 5),
    };
    let call = at(
        K::Call {
            function: at("head".into(), 6),
            arguments: vec![at(
                K::NativeValue(Box::new(at(ExpressionKind::Binding("input".into()), 7))),
                7,
            )],
        },
        6,
    );
    let result = FunctionRequest::prepare_patterns(
        &[function],
        &SourceHost,
        &mut context,
        &mut budget,
        FunctionLimits { active_depth: 1 },
    )
    .unwrap()
    .evaluate(&call, &bindings)
    .unwrap();
    assert!(
        matches!(result, SourceValue::Words(ref values) if values == &[Word { side: 0, text: "alpha", range: 0..5 }] && values[0].text.as_ptr() == source.as_ptr())
    );
    let mismatch = PredicateArgument {
        argument: Some(at(
            P::List {
                elements: vec![
                    at(P::Wildcard, 81),
                    at(P::Wildcard, 82),
                    at(P::Wildcard, 83),
                ],
                ignore_rest: false,
            },
            80,
        )),
        body: Box::new(at(K::Literal(V::Boolean(true)), 81)),
    };
    let error = zirql::evaluate_native_predicate(
        &mismatch,
        &SourceHost,
        subject(),
        &mut context,
        &Bindings::default(),
        &mut budget,
    )
    .unwrap_err();
    assert!(
        matches!(error, EvaluationError::Core(QueryError { location: Location::Query(range), .. }) if range == (80..81))
    );
    zirql::dispose_native_value(entry);
    zirql::dispose_native_value(call);
}

#[test]
fn ordinary_native_sets_and_dictionary_keys_retain_identity_and_source() {
    use zirql::{ConstructedValue as V, FunctionLimits, FunctionRequest, ValueExpressionKind as K};
    let source = String::from("ash apple ash");
    let mut context = SourceContext {
        source: &source,
        prefix: "",
        emissions: vec![],
    };
    let mut budget = Budget::for_program(ProgramLimits {
        work: 1_000_000,
        results: 100_000,
        nesting: 64,
        stream_items: 100_000,
    });
    let bindings = Bindings::new([
        (at("a".into(), 1), word_at(&source, 0..3, 0)),
        (at("b".into(), 2), word_at(&source, 4..9, 0)),
        (at("another_ash".into(), 3), word_at(&source, 10..13, 0)),
        (
            at("path".into(), 4),
            SourceValue::Record(zirql::NativeCapture {
                fields: vec![("a".into(), word_at(&source, 0..3, 0))],
            }),
        ),
    ])
    .unwrap();
    let native = |name: &str| {
        at(
            K::NativeValue(Box::new(at(ExpressionKind::Binding(name.into()), 5))),
            5,
        )
    };
    let alias = || {
        at(
            K::NativeValue(Box::new(at(
                ExpressionKind::Field {
                    value: Box::new(at(ExpressionKind::Binding("path".into()), 6)),
                    field: at("a".into(), 7),
                },
                6,
            ))),
            6,
        )
    };
    let pair = || at(K::Set(vec![native("a"), native("b")]), 8);
    let expression = at(
        K::Tally(Box::new(at(
            K::Array(vec![
                pair(),
                at(K::Set(vec![native("b"), alias()]), 9),
                at(K::Set(vec![native("a"), alias()]), 10),
                at(K::Set(vec![native("another_ash")]), 11),
            ]),
            8,
        ))),
        8,
    );
    let result = FunctionRequest::prepare(
        &[],
        &SourceHost,
        &mut context,
        &mut budget,
        FunctionLimits { active_depth: 1 },
    )
    .unwrap()
    .evaluate(&expression, &bindings)
    .unwrap();
    let SourceValue::Dictionary(entries) = result else {
        panic!("expected ordinary native dictionary")
    };
    assert_eq!(entries.len(), 3);
    for ((key, count), expected) in entries.iter().zip([2, 1, 1]) {
        assert_eq!(count, &SourceValue::Constructed(V::Integer(expected)));
        let SourceValue::Set(values) = key else {
            panic!("set key")
        };
        for value in values {
            let SourceValue::Words(words) = value else {
                panic!("native word")
            };
            assert_eq!(
                words[0].text.as_ptr(),
                source[words[0].range.clone()].as_ptr()
            );
        }
    }
    assert!(
        !zirql::native_values_equal(
            &entries[1].0,
            &entries[2].0,
            &SourceHost,
            &mut context,
            &mut budget,
            Location::Query(0..1)
        )
        .unwrap()
    );
    let lookup = at(
        K::ArrayAccess {
            array: Box::new(at(K::Dictionary(vec![(pair(), native("a"))]), 12)),
            index: Box::new(at(K::Set(vec![native("b"), alias()]), 13)),
        },
        12,
    );
    let result = FunctionRequest::prepare(
        &[],
        &SourceHost,
        &mut context,
        &mut budget,
        FunctionLimits { active_depth: 1 },
    )
    .unwrap()
    .evaluate(&lookup, &bindings)
    .unwrap();
    assert!(
        matches!(result, SourceValue::Words(ref words) if words[0].range == (0..3) && words[0].text.as_ptr() == source.as_ptr())
    );
    let comprehension = at(
        K::Tally(Box::new(at(
            K::Comprehension {
                constructor: zirql::ValueConstructor::Array,
                clauses: vec![at(
                    zirql::ValueGenerator::In {
                        case: false,
                        binders: vec![at(zirql::PatternKind::Name(at("i".into(), 15)), 15)],
                        source: at(K::Literal(V::Array(vec![V::Integer(1), V::Integer(2)])), 15),
                        filter: None,
                    },
                    15,
                )],
                head: Box::new(pair()),
            },
            15,
        ))),
        15,
    );
    let result = FunctionRequest::prepare(
        &[],
        &SourceHost,
        &mut context,
        &mut budget,
        FunctionLimits { active_depth: 1 },
    )
    .unwrap()
    .evaluate(&comprehension, &bindings)
    .unwrap();
    assert!(
        matches!(result, SourceValue::Dictionary(ref entries) if entries.len() == 1 && entries[0].1 == SourceValue::Constructed(V::Integer(2)))
    );
    let native_dictionary_comprehension = at(
        K::DictionaryComprehension {
            clauses: vec![at(
                zirql::ValueGenerator::In {
                    case: false,
                    binders: vec![at(zirql::PatternKind::Name(at("node".into(), 15)), 15)],
                    source: pair(),
                    filter: None,
                },
                15,
            )],
            key: Box::new(at(K::Name("node".into()), 15)),
            head: Box::new(at(K::Name("node".into()), 15)),
        },
        15,
    );
    let result = FunctionRequest::prepare(
        &[],
        &SourceHost,
        &mut context,
        &mut budget,
        FunctionLimits { active_depth: 1 },
    )
    .unwrap()
    .evaluate(&native_dictionary_comprehension, &bindings)
    .unwrap();
    assert!(
        matches!(result, SourceValue::Dictionary(ref entries) if entries.len() == 2 && entries.iter().all(|(key, value)| key == value))
    );
    let mixed_list = at(
        K::Tally(Box::new(at(
            K::Array(vec![native("a"), at(K::Literal(V::Integer(1)), 16)]),
            16,
        ))),
        16,
    );
    let result = FunctionRequest::prepare(
        &[],
        &SourceHost,
        &mut context,
        &mut budget,
        FunctionLimits { active_depth: 1 },
    )
    .unwrap()
    .evaluate(&mixed_list, &bindings)
    .unwrap();
    assert!(matches!(result, SourceValue::Dictionary(ref entries) if entries.len() == 2));
    let mixed = at(
        K::Set(vec![
            native("a"),
            at(K::Literal(V::String("ash".into())), 14),
        ]),
        14,
    );
    let result = FunctionRequest::prepare(
        &[],
        &SourceHost,
        &mut context,
        &mut budget,
        FunctionLimits { active_depth: 1 },
    )
    .unwrap()
    .evaluate(&mixed, &bindings)
    .unwrap();
    assert!(matches!(result, SourceValue::Set(ref values) if values.len() == 2));
    assert!(
        zirql::admit_value(
            &word_at(&source, 0..3, 0),
            &DeclaredType::Value,
            &SourceHost,
            &mut context,
            &mut budget,
            Location::Query(0..1)
        )
        .is_err()
    );
    let queue = zirql::NativeQueue::new(
        vec![],
        DeclaredType::Native(SourceKind::Words),
        &SourceHost,
        &mut context,
        &mut budget,
        Location::Query(0..1),
    )
    .unwrap();
    let queue =
        SourceValue::Computational(std::sync::Arc::new(zirql::ComputationalValue::Queue(queue)));
    assert!(
        zirql::admit_value(
            &queue,
            &DeclaredType::Value,
            &SourceHost,
            &mut context,
            &mut budget,
            Location::Query(0..1)
        )
        .is_err()
    );
}

#[test]
fn nested_native_collections_have_structural_hashes_comparison_and_admission() {
    use zirql::ConstructedValue as V;
    let source = String::from("ash apple");
    let foreign = String::from("ash apple");
    let mut context = SourceContext {
        source: &source,
        prefix: "",
        emissions: vec![],
    };
    let mut budget = Budget::for_program(ProgramLimits {
        work: 1_000_000,
        results: 100_000,
        nesting: 64,
        stream_items: 100_000,
    });
    let location = Location::Query(0..10);
    let number = || SourceValue::Constructed(V::Integer(1));
    let tuple = || SourceValue::Tuple(vec![word_at(&source, 0..3, 0), number()]);
    let first = zirql::construct_native_dictionary(
        vec![
            (tuple(), word_at(&source, 4..9, 0)),
            (number(), word_at(&source, 0..3, 0)),
        ],
        &SourceHost,
        &mut context,
        &mut budget,
        location.clone(),
    )
    .unwrap();
    let second = zirql::construct_native_dictionary(
        vec![
            (number(), word_at(&source, 0..3, 0)),
            (tuple(), word_at(&source, 4..9, 0)),
        ],
        &SourceHost,
        &mut context,
        &mut budget,
        location.clone(),
    )
    .unwrap();
    assert!(
        zirql::native_values_equal(
            &first,
            &second,
            &SourceHost,
            &mut context,
            &mut budget,
            location.clone()
        )
        .unwrap()
    );
    assert_eq!(
        zirql::native_value_hash(
            &first,
            &SourceHost,
            &mut context,
            &mut budget,
            location.clone()
        )
        .unwrap(),
        zirql::native_value_hash(
            &second,
            &SourceHost,
            &mut context,
            &mut budget,
            location.clone()
        )
        .unwrap()
    );
    assert_eq!(
        zirql::native_values_compare(
            &first,
            &second,
            &SourceHost,
            &mut context,
            &mut budget,
            location.clone()
        )
        .unwrap(),
        std::cmp::Ordering::Equal
    );
    let set = zirql::construct_native_set(
        vec![first, second],
        &SourceHost,
        &mut context,
        &mut budget,
        location.clone(),
    )
    .unwrap();
    assert!(matches!(set, SourceValue::Set(ref values) if values.len() == 1));
    let positional = SourceValue::Tuple(vec![
        number(),
        SourceValue::Constructed(V::String("ash".into())),
    ]);
    let constructed =
        SourceValue::Constructed(V::Tuple(vec![V::Integer(1), V::String("ash".into())]));
    assert!(
        zirql::native_values_equal(
            &positional,
            &constructed,
            &SourceHost,
            &mut context,
            &mut budget,
            location.clone()
        )
        .unwrap()
    );
    assert_eq!(
        zirql::native_value_hash(
            &positional,
            &SourceHost,
            &mut context,
            &mut budget,
            location.clone()
        )
        .unwrap(),
        zirql::native_value_hash(
            &constructed,
            &SourceHost,
            &mut context,
            &mut budget,
            location.clone()
        )
        .unwrap()
    );
    assert_eq!(
        zirql::native_values_compare(
            &positional,
            &constructed,
            &SourceHost,
            &mut context,
            &mut budget,
            location.clone()
        )
        .unwrap(),
        std::cmp::Ordering::Equal
    );
    let foreign_value = zirql::construct_native_dictionary(
        vec![(number(), word_at(&foreign, 0..3, 0))],
        &SourceHost,
        &mut context,
        &mut budget,
        location.clone(),
    )
    .unwrap_err();
    assert!(matches!(
        foreign_value,
        EvaluationError::Host(SourceFailure::Source(_))
    ));
    let bad_data = SourceValue::Constructed(V::Float(f64::NAN));
    assert!(
        zirql::construct_native_set(
            vec![bad_data],
            &SourceHost,
            &mut context,
            &mut budget,
            location
        )
        .is_err()
    );
}

#[test]
fn constructed_dictionary_children_of_native_values_support_all_lookup_forms() {
    use zirql::{
        ConstructedValue as V, DeclaredType as D, FunctionLimits, FunctionRequest,
        ValueExpressionKind as K,
    };
    let source = String::from("ash apple");
    let mut context = SourceContext {
        source: &source,
        prefix: "",
        emissions: vec![],
    };
    let mut budget = Budget::for_program(ProgramLimits {
        work: 1_000_000,
        results: 100_000,
        nesting: 64,
        stream_items: 1_000,
    });
    let dictionary = zirql::declare_constructed(
        V::Dictionary(vec![(V::Integer(1), V::Integer(7))].into()),
        D::Dictionary(Box::new(D::Integer), Box::new(D::Integer)),
        &mut budget,
        Location::Query(0..1),
    )
    .unwrap();
    let bindings = Bindings::new([(
        at("t".into(), 0),
        SourceValue::Tuple(vec![SourceValue::Constructed(dictionary)]),
    )])
    .unwrap();
    let child = || {
        at(
            K::TupleAccess {
                value: Box::new(at(
                    K::NativeValue(Box::new(at(ExpressionKind::Binding("t".into()), 0))),
                    0,
                )),
                position: at(1, 0),
            },
            0,
        )
    };
    let key = || Box::new(at(K::Literal(V::Integer(1)), 0));
    let entry = at(
        K::Array(vec![
            at(
                K::ArrayAccess {
                    array: Box::new(child()),
                    index: key(),
                },
                0,
            ),
            at(
                K::Lookup {
                    value: Box::new(child()),
                    key: key(),
                },
                0,
            ),
            at(
                K::HasKey {
                    value: Box::new(child()),
                    key: key(),
                },
                0,
            ),
        ]),
        0,
    );
    let result = FunctionRequest::prepare(
        &[],
        &SourceHost,
        &mut context,
        &mut budget,
        FunctionLimits { active_depth: 1 },
    )
    .unwrap()
    .evaluate(&entry, &bindings)
    .unwrap();
    let SourceValue::Constructed(result) = result else {
        panic!("constructed lookup results")
    };
    let V::Array(values) = result.data() else {
        panic!("lookup result array")
    };
    assert_eq!(values[0].data(), &V::Integer(7));
    assert_eq!(values[1].data(), &V::Integer(7));
    assert_eq!(values[2].data(), &V::Boolean(true));
    zirql::dispose_native_value(entry);
}

#[test]
fn scoped_mapping_retains_native_handles_in_saved_nested_reports() {
    use zirql::{
        ConstructedValue as V, FunctionLimits, FunctionRequest, PatternKind as P,
        PredicateArgument, ValueExpressionKind as K,
    };
    let source = String::from("ash apple");
    let mut context = SourceContext {
        source: &source,
        prefix: "",
        emissions: vec![],
    };
    let mut budget = Budget::for_program(ProgramLimits {
        work: 1_000_000,
        results: 100_000,
        nesting: 64,
        stream_items: 100,
    });
    let bindings = Bindings::new([
        (
            at("nodes".into(), 1),
            SourceValue::Sequence {
                values: vec![word_at(&source, 4..9, 0), word_at(&source, 0..3, 0)],
                element: DeclaredType::Native(SourceKind::Words),
            },
        ),
        (
            at("empty".into(), 2),
            SourceValue::Sequence {
                values: vec![],
                element: DeclaredType::Native(SourceKind::Words),
            },
        ),
        (at("opaque".into(), 3), word_at(&source, 0..3, 0)),
    ])
    .unwrap();
    let v = worklist_value;
    let n = worklist_name;
    let native = |name: &str| {
        v(K::NativeValue(Box::new(at(
            ExpressionKind::Binding(name.into()),
            5,
        ))))
    };
    let field = |value, name: &str| {
        v(K::Field {
            value: Box::new(value),
            field: at(name.into(), 6),
        })
    };
    let subject = || v(K::PredicateSubject);
    let mapped = v(K::Map {
        array: Box::new(native("nodes")),
        argument: PredicateArgument {
            argument: None,
            body: Box::new(v(K::Record(vec![(at("native".into(), 7), subject())]))),
        },
    });
    let expression = v(K::Block {
        bindings: vec![(at(P::Name(at("report".into(), 10)), 10), mapped)],
        value: Box::new(v(K::FlatMap {
            array: Box::new(n("report")),
            argument: PredicateArgument {
                argument: Some(at(P::Name(at("row".into(), 11)), 11)),
                body: Box::new(v(K::Map {
                    array: Box::new(v(K::Array(vec![
                        v(K::Literal(V::Integer(2))),
                        v(K::Literal(V::Integer(1))),
                    ]))),
                    argument: PredicateArgument {
                        argument: None,
                        body: Box::new(v(K::Record(vec![
                            (at("original".into(), 12), field(n("row"), "native")),
                            (at("copy".into(), 13), subject()),
                        ]))),
                    },
                })),
            },
        })),
    });
    let result = FunctionRequest::prepare(
        &[],
        &SourceHost,
        &mut context,
        &mut budget,
        FunctionLimits { active_depth: 1 },
    )
    .unwrap()
    .evaluate(&expression, &bindings)
    .unwrap();
    let SourceValue::Sequence { values, .. } = result else {
        panic!("native report sequence")
    };
    assert_eq!(values.len(), 4);
    for (value, (range, copy)) in values
        .iter()
        .zip([(4..9, 2), (4..9, 1), (0..3, 2), (0..3, 1)])
    {
        let SourceValue::Record(record) = value else {
            panic!("native record")
        };
        let SourceValue::Words(words) = &record.fields[0].1 else {
            panic!("retained native word")
        };
        assert_eq!(words[0].range, range.clone());
        assert_eq!(words[0].text.as_ptr(), source[range].as_ptr());
        assert_eq!(words[0].side, 0);
        assert_eq!(
            record.fields[1].1,
            SourceValue::Constructed(V::Integer(copy))
        );
    }
    let invalid = v(K::Reduce {
        operator: zirql::ValueReducer::Length,
        array: Box::new(v(K::FlatMap {
            array: Box::new(native("empty")),
            argument: PredicateArgument {
                argument: None,
                body: Box::new(native("opaque")),
            },
        })),
    });
    let error = FunctionRequest::prepare(
        &[],
        &SourceHost,
        &mut context,
        &mut budget,
        FunctionLimits { active_depth: 1 },
    )
    .unwrap()
    .evaluate(&invalid, &bindings)
    .unwrap_err();
    assert!(
        matches!(&error.error, EvaluationError::Core(error) if error.message.contains("positional")),
        "{error:?}"
    );
    let invalid = v(K::Map {
        array: Box::new(native("opaque")),
        argument: PredicateArgument {
            argument: None,
            body: Box::new(subject()),
        },
    });
    let error = FunctionRequest::prepare(
        &[],
        &SourceHost,
        &mut context,
        &mut budget,
        FunctionLimits { active_depth: 1 },
    )
    .unwrap()
    .evaluate(&invalid, &bindings)
    .unwrap_err();
    assert!(
        matches!(&error.error, EvaluationError::Core(error) if error.message.contains("positional")),
        "{error:?}"
    );
    assert!(context.emissions.is_empty());
}

#[test]
fn case_generator_does_not_suppress_unknown_native_guards() {
    use zirql::{
        ConstructedValue as V, FunctionLimits, FunctionRequest, ValueConstructor,
        ValueExpressionKind as K, ValueGenerator,
    };
    let source = String::from("ash");
    let mut context = SourceContext {
        source: &source,
        prefix: "",
        emissions: vec![],
    };
    let bindings = Bindings::new([(
        at("input".into(), 1),
        SourceValue::Words(vec![Word {
            side: 0,
            text: &source,
            range: 0..3,
        }]),
    )])
    .unwrap();
    let expression = at(
        K::Comprehension {
            constructor: ValueConstructor::Array,
            clauses: vec![at(
                ValueGenerator::In {
                    case: true,
                    binders: vec![at(zirql::PatternKind::Literal(V::Integer(1)), 10)],
                    source: at(K::Literal(V::Array(vec![V::Integer(0), V::Integer(1)])), 11),
                    filter: Some(at(
                        K::NativeValue(Box::new(sequence(
                            at(ExpressionKind::Binding("input".into()), 60),
                            vec![at(ExpressionStep::Stage(SourceStage::UnknownGuard), 61)],
                        ))),
                        60,
                    )),
                },
                10,
            )],
            head: Box::new(at(K::Literal(V::Integer(1)), 12)),
        },
        10,
    );
    let mut budget = Budget::for_program(ProgramLimits {
        work: 1_000_000,
        results: 10_000,
        nesting: 64,
        ..limits()
    });
    let error = FunctionRequest::prepare(
        &[],
        &SourceHost,
        &mut context,
        &mut budget,
        FunctionLimits { active_depth: 1 },
    )
    .unwrap()
    .evaluate(&expression, &bindings)
    .unwrap_err();
    assert!(
        matches!(error.error, EvaluationError::Core(ref e) if e.category == QueryErrorCategory::UnknownPredicate && e.location == Location::Query(61..62))
    );
}

#[test]
fn sequence_neighbourhoods_retain_native_source_handles() {
    use zirql::{ConstructedValue as V, FunctionLimits, FunctionRequest, ValueExpressionKind as K};
    let source = String::from("ash apple birch");
    let mut context = SourceContext {
        source: &source,
        prefix: "",
        emissions: vec![],
    };
    let mut budget = Budget::for_program(ProgramLimits {
        work: 1_000_000,
        results: 100_000,
        nesting: 64,
        stream_items: 100,
    });
    let rows = [("same", 0..3), ("same", 4..9), ("other", 10..15)]
        .into_iter()
        .map(|(name, range)| {
            SourceValue::Record(zirql::NativeCapture {
                fields: vec![
                    (
                        "name".into(),
                        SourceValue::Constructed(V::String(name.into())),
                    ),
                    ("node".into(), word_at(&source, range, 0)),
                ],
            })
        })
        .collect();
    let bindings = Bindings::new([(
        at("rows".into(), 0),
        SourceValue::Sequence {
            values: rows,
            element: DeclaredType::Record(vec![
                ("name".into(), DeclaredType::String),
                ("node".into(), DeclaredType::Native(SourceKind::Words)),
            ]),
        },
    )])
    .unwrap();
    let native = || {
        worklist_value(K::NativeValue(Box::new(at(
            ExpressionKind::Binding("rows".into()),
            0,
        ))))
    };
    let window = worklist_value(K::Windows {
        items: Box::new(native()),
        size: Box::new(worklist_value(K::Literal(V::Integer(2)))),
    });
    let result = FunctionRequest::prepare(
        &[],
        &SourceHost,
        &mut context,
        &mut budget,
        FunctionLimits { active_depth: 1 },
    )
    .unwrap()
    .evaluate(&window, &bindings)
    .unwrap();
    let SourceValue::Sequence {
        values: windows, ..
    } = result
    else {
        panic!("native windows")
    };
    assert_eq!(windows.len(), 2);
    for (window, ranges) in windows.iter().zip([[0..3, 4..9], [4..9, 10..15]]) {
        let SourceValue::Sequence { values: rows, .. } = window else {
            panic!("window items")
        };
        for (row, range) in rows.iter().zip(ranges) {
            let SourceValue::Record(record) = row else {
                panic!("original row")
            };
            let SourceValue::Words(words) = &record.fields[1].1 else {
                panic!("native word")
            };
            assert_eq!(words[0].range, range.clone());
            assert_eq!(words[0].text.as_ptr(), source[range].as_ptr());
        }
    }
    let expression = worklist_value(K::RunsBy {
        rows: Box::new(native()),
        field: at("name".into(), 0),
    });
    let result = FunctionRequest::prepare(
        &[],
        &SourceHost,
        &mut context,
        &mut budget,
        FunctionLimits { active_depth: 1 },
    )
    .unwrap()
    .evaluate(&expression, &bindings)
    .unwrap();
    let SourceValue::Sequence { values: runs, .. } = result else {
        panic!("native runs")
    };
    assert_eq!(runs.len(), 2);
    let SourceValue::Record(record) = &runs[0] else {
        panic!("run record")
    };
    assert_eq!(
        record.fields[0].1,
        SourceValue::Constructed(V::String("same".into()))
    );
    let SourceValue::Sequence { values: rows, .. } = &record.fields[1].1 else {
        panic!("run items")
    };
    assert_eq!(rows.len(), 2);
    let SourceValue::Record(record) = &rows[1] else {
        panic!("original row")
    };
    let SourceValue::Words(words) = &record.fields[1].1 else {
        panic!("retained word")
    };
    assert_eq!(words[0].text.as_ptr(), source[4..9].as_ptr());
    assert_eq!(words[0].range, 4..9);
    zirql::dispose_native_value(window);
    zirql::dispose_native_value(expression);
}
