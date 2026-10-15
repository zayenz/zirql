use std::{collections::BTreeMap, ops::Range};

use zirql::{
    Bindings, Budget, EvaluationError, Expression, ExpressionStep, Limits, Location, MapSelection,
    NativeMap, NativeOrdering, ProgramHost, ProgramLimits, QueryError, QueryErrorCategory,
    ScalarMapKey, ValueReaderHost,
};

type Query = Expression<Stage, (), ()>;
#[derive(Debug, PartialEq, Eq)]
enum Stage {
    Names,
    Missing,
    Many,
    Wrong,
    Keep,
    Rank,
    Select(MapSelection),
    Order(Box<NativeOrdering<Query>>),
    Map(Box<NativeMap<Query>>),
}
#[derive(Clone, Debug, PartialEq, Eq)]
struct Row<'a> {
    name: &'a str,
    source: Range<usize>,
    fails: bool,
}
#[derive(Clone, Debug, PartialEq, Eq)]
enum Value<'a> {
    Rows(Vec<Row<'a>>),
    Strings(Vec<String>),
    Map(BTreeMap<String, Value<'a>>),
    Ranked(Vec<(String, Value<'a>)>),
}
#[derive(Default)]
struct Words {
    value_binding: bool,
}
impl ValueReaderHost for Words {
    type Stage = Stage;
    type SetOperator = ();
    type Emitter = ();
    type Predicate = ();
    fn is_binding(&self, name: &str) -> bool {
        name == "saved" || self.value_binding && name == "value"
    }
    fn is_stage(&self, name: &str) -> bool {
        matches!(
            name,
            "names" | "missing" | "many" | "wrong" | "keep" | "reverse" | "head" | "tail"
        )
    }
    fn parse_stage(
        &mut self,
        reader: &mut zirql::ArgumentReader<'_>,
        _: &mut Budget,
    ) -> Result<ExpressionStep<Stage, ()>, QueryError> {
        let name = reader.word()?;
        Ok(ExpressionStep::Stage(match name.value {
            "names" => Stage::Names,
            "missing" => Stage::Missing,
            "many" => Stage::Many,
            "wrong" => Stage::Wrong,
            "keep" => Stage::Keep,
            "reverse" => Stage::Select(MapSelection::Reverse),
            "head" | "tail" => {
                reader.expect('(')?;
                let count = reader.unsigned_integer()?.value;
                reader.expect(')')?;
                Stage::Select(if name.value == "head" {
                    MapSelection::Head(count)
                } else {
                    MapSelection::Tail(count)
                })
            }
            _ => return Err(QueryError::query(name.range, "unknown word stage")),
        }))
    }
    fn parse_predicate_leaf(
        &mut self,
        reader: &mut zirql::ArgumentReader<'_>,
        _: &mut Budget,
    ) -> Result<(), QueryError> {
        Err(reader.error("no predicates"))
    }
    fn filter(&mut self, _: zirql::Predicate<()>, _: Range<usize>) -> Stage {
        unreachable!()
    }
    fn projection(&mut self, _: zirql::Projection<Query>, _: Range<usize>) -> Stage {
        unreachable!()
    }
    fn mapping(&mut self, map: NativeMap<Query>, _: Range<usize>) -> Result<Stage, QueryError> {
        Ok(Stage::Map(Box::new(map)))
    }
    fn map_ranking(&mut self, _: Range<usize>) -> Result<Stage, QueryError> {
        Ok(Stage::Rank)
    }
    fn ordering(
        &mut self,
        ordering: NativeOrdering<Query>,
        _: Range<usize>,
    ) -> Result<Stage, QueryError> {
        Ok(Stage::Order(Box::new(ordering)))
    }
    fn set_operator(&self, _: &str) -> Option<()> {
        None
    }
}
impl<'a> ProgramHost<&'a str> for Words {
    type Value = Value<'a>;
    type Stage = Stage;
    type SetOperator = ();
    type Emitter = ();
    type Error = QueryError;
    fn copy_value(
        &self,
        value: &Value<'a>,
        _: &mut &'a str,
        budget: &mut Budget,
        range: &Range<usize>,
    ) -> Result<Value<'a>, EvaluationError<QueryError>> {
        fn admit(
            value: &Value<'_>,
            budget: &mut Budget,
            range: &Range<usize>,
        ) -> Result<(), QueryError> {
            let count = match value {
                Value::Rows(rows) => rows.len(),
                Value::Strings(strings) => strings.len(),
                Value::Map(entries) => entries.len(),
                Value::Ranked(entries) => entries.len(),
            };
            budget.charge(count + 1, Location::Query(range.clone()))?;
            budget.charge_results(count + 1, Location::Query(range.clone()))?;
            match value {
                Value::Rows(_) => {}
                Value::Strings(strings) => {
                    for string in strings {
                        budget.charge(string.len(), Location::Query(range.clone()))?;
                        budget.charge_results(string.len(), Location::Query(range.clone()))?;
                    }
                }
                Value::Map(entries) => {
                    for (key, value) in entries {
                        budget.charge(key.len(), Location::Query(range.clone()))?;
                        budget.charge_results(key.len(), Location::Query(range.clone()))?;
                        admit(value, budget, range)?;
                    }
                }
                Value::Ranked(entries) => {
                    for (key, value) in entries {
                        budget.charge(key.len(), Location::Query(range.clone()))?;
                        budget.charge_results(key.len(), Location::Query(range.clone()))?;
                        admit(value, budget, range)?;
                    }
                }
            }
            Ok(())
        }
        admit(value, budget, range)?;
        Ok(value.clone())
    }
    fn evaluate_stage(
        &self,
        stage: &Stage,
        value: Value<'a>,
        context: &mut &'a str,
        bindings: &Bindings<Value<'a>>,
        budget: &mut Budget,
        range: &Range<usize>,
    ) -> Result<Value<'a>, EvaluationError<QueryError>> {
        let value = match (stage, value) {
            (Stage::Rank, value) => {
                let entries = match value {
                    Value::Map(entries) => {
                        let count = entries.len();
                        budget.charge(count + 1, Location::Query(range.clone()))?;
                        budget.charge_results(count + 1, Location::Query(range.clone()))?;
                        entries.into_iter().collect()
                    }
                    Value::Ranked(entries) => entries,
                    _ => return Err(QueryError::query(range.clone(), "map required").into()),
                };
                return Ok(Value::Ranked(zirql::rank_map_by_value(
                    entries,
                    budget,
                    Location::Query(range.clone()),
                    |value, _| {
                        let Value::Rows(rows) = value else {
                            return Err(EvaluationError::Host(QueryError::query(
                                range.clone(),
                                "scalar projection required",
                            )));
                        };
                        Ok(ScalarMapKey::Unsigned(rows[0].source.start as u64))
                    },
                )?));
            }
            (Stage::Select(selection), Value::Ranked(mut entries)) => {
                zirql::select_ranked_map(
                    &mut entries,
                    *selection,
                    budget,
                    Location::Query(range.clone()),
                )?;
                return Ok(Value::Ranked(entries));
            }
            (_, value) => value,
        };
        let Value::Rows(rows) = value else {
            return Err(QueryError::query(range.clone(), "words required").into());
        };
        Ok(match stage {
            Stage::Names => {
                let mut names = Vec::new();
                for row in rows {
                    budget.charge(row.name.len(), Location::Query(range.clone()))?;
                    budget.charge_results(row.name.len() + 1, Location::Query(range.clone()))?;
                    names.push(row.name.to_owned());
                }
                Value::Strings(names)
            }
            Stage::Missing => Value::Strings(vec![]),
            Stage::Many => Value::Strings(vec!["a".into(), "b".into()]),
            Stage::Wrong => Value::Rows(rows),
            Stage::Keep => {
                if rows.iter().any(|row| row.fails) {
                    return Err(EvaluationError::Host(QueryError::query(
                        range.clone(),
                        "value failed",
                    )));
                }
                Value::Rows(rows)
            }
            Stage::Rank | Stage::Select(_) | Stage::Order(_) => unreachable!(),
            Stage::Map(map) => Value::Map(zirql::map_native_by(
                rows,
                context,
                budget,
                Location::Query(range.clone()),
                |row, context, budget| {
                    let key = zirql::evaluate_expression(
                        &map.key,
                        self,
                        Value::Rows(vec![row.clone()]),
                        context,
                        bindings,
                        budget,
                    )?;
                    Ok(if let Value::Strings(keys) = key {
                        Some(keys)
                    } else {
                        None
                    })
                },
                |row, context, budget| {
                    zirql::evaluate_expression(
                        &map.value,
                        self,
                        Value::Rows(vec![row]),
                        context,
                        bindings,
                        budget,
                    )
                },
            )?),
        })
    }
    fn combine(
        &self,
        _: &(),
        _: Value<'a>,
        _: Value<'a>,
        _: &mut &'a str,
        _: &mut Budget,
        _: &Range<usize>,
    ) -> Result<Value<'a>, EvaluationError<QueryError>> {
        unreachable!()
    }
    fn construct_record(
        &self,
        _: Vec<(String, Value<'a>)>,
        _: &mut &'a str,
        _: &mut Budget,
        _: &Range<usize>,
    ) -> Result<Value<'a>, EvaluationError<QueryError>> {
        unreachable!()
    }
    fn emit(
        &self,
        _: &(),
        _: &Value<'a>,
        _: &mut &'a str,
        _: &mut Budget,
        _: &Range<usize>,
    ) -> Result<(), EvaluationError<QueryError>> {
        unreachable!()
    }
}
fn limits() -> ProgramLimits {
    ProgramLimits {
        work: 100_000,
        results: 10_000,
        stream_items: 16,
        nesting: 16,
    }
}
fn run<'a>(text: &str, rows: Vec<Row<'a>>) -> Result<Value<'a>, EvaluationError<QueryError>> {
    let mut budget = Budget::for_program(limits());
    let query = zirql::read_native_expression(
        &mut zirql::ArgumentReader::new(text, 0)?,
        &mut Words::default(),
        &mut budget,
        16,
    )?;
    zirql::evaluate_expression(
        &query,
        &Words::default(),
        Value::Rows(rows),
        &mut "",
        &Bindings::default(),
        &mut budget,
    )
}
fn row(name: &str, offset: usize, fails: bool) -> Row<'_> {
    Row {
        name,
        source: offset..offset + name.len(),
        fails,
    }
}
#[test]
fn common_map_reader_and_constructor_retain_native_rows_and_lexical_keys() {
    let source = String::from("βeta alpha");
    let beta = row(&source[..5], 0, false);
    let alpha = row(&source[6..], 6, false);
    let Value::Map(entries) =
        run("map_by(names, keep)", vec![beta.clone(), alpha.clone()]).unwrap()
    else {
        panic!("map")
    };
    assert_eq!(
        entries.keys().map(String::as_str).collect::<Vec<_>>(),
        ["alpha", "βeta"]
    );
    assert_eq!(entries["alpha"], Value::Rows(vec![alpha]));
    let Value::Rows(rows) = &entries["βeta"] else {
        panic!("native rows")
    };
    assert_eq!(rows, &[beta]);
    assert_eq!(rows[0].name.as_ptr(), source.as_ptr());
    assert_eq!(
        run("map_by(names, keep)", vec![]).unwrap(),
        Value::Map(BTreeMap::new())
    );
    assert!(
        matches!(run("map_by(names, map_by(names, keep))", vec![row("a", 0, false)]).unwrap(), Value::Map(map) if matches!(&map["a"], Value::Map(nested) if matches!(&nested["a"], Value::Rows(_))))
    );
}
#[test]
fn key_failures_and_input_order_precede_later_values() {
    for text in [
        "map_by(missing, keep)",
        "map_by(many, keep)",
        "map_by(wrong, keep)",
    ] {
        let EvaluationError::Core(error) = run(text, vec![row("a", 0, true)]).unwrap_err() else {
            panic!("core key failure")
        };
        assert_eq!(error.message, "map_by key must produce exactly one string");
        assert_eq!(error.location, Location::Query(0..text.len()));
    }
    let error = run(
        "map_by(names, keep)",
        vec![row("a", 0, false), row("a", 2, true)],
    )
    .unwrap_err();
    assert!(
        matches!(error, EvaluationError::Core(error) if error.message.contains("duplicate key `a`"))
    );
    let error = run(
        "map_by(names, keep)",
        vec![row("z", 0, true), row("a", 2, false), row("a", 4, false)],
    )
    .unwrap_err();
    assert!(matches!(error, EvaluationError::Host(error) if error.message == "value failed"));
}
#[test]
fn callable_map_storage_uses_the_active_located_allowance() {
    let mut budget = Budget::for_program(ProgramLimits {
        results: 1,
        ..limits()
    });
    let error = zirql::map_native_by(
        vec!["a"],
        &mut (),
        &mut budget,
        Location::Query(8..29),
        |key, _, _| Ok::<_, EvaluationError<()>>(Some(vec![(*key).into()])),
        |value, _, _| Ok(value),
    )
    .unwrap_err();
    assert!(
        matches!(error, EvaluationError::Core(error) if error.category == QueryErrorCategory::ResultLimit && error.location == Location::Query(8..29))
    );
    assert_eq!(budget.remaining_results(), Some(0));
}

#[test]
fn ranked_native_maps_keep_owners_and_order_through_saved_selection() {
    let source = String::from("zeta alpha beta");
    let rows = vec![
        row(&source[5..10], 5, false),
        row(&source[11..], 11, false),
        row(&source[..4], 0, false),
    ];
    // Words has no `value` stage: the common reader lowers this selector.
    let ranked = run(
        "map_by(names, keep) | sort_by( (# selector\n value) )",
        rows,
    )
    .unwrap();
    let Value::Ranked(entries) = &ranked else {
        panic!("ranked map")
    };
    assert_eq!(
        entries
            .iter()
            .map(|(key, _)| key.as_str())
            .collect::<Vec<_>>(),
        ["zeta", "alpha", "beta"]
    );
    let bindings = Bindings::new([(zirql::Located::new("saved".into(), 0..5), ranked)]).unwrap();
    let text = "saved | reverse | head(2) | tail(1)";
    let mut budget = Budget::for_program(limits());
    let query = zirql::read_native_expression(
        &mut zirql::ArgumentReader::new(text, 0).unwrap(),
        &mut Words::default(),
        &mut budget,
        16,
    )
    .unwrap();
    let Value::Ranked(selected) = zirql::evaluate_expression(
        &query,
        &Words::default(),
        Value::Rows(vec![]),
        &mut "",
        &bindings,
        &mut budget,
    )
    .unwrap() else {
        panic!("ranked map")
    };
    assert_eq!(selected[0].0, "alpha");
    let Value::Rows(rows) = &selected[0].1 else {
        panic!("native rows")
    };
    assert_eq!(rows[0].source, 5..10);
    assert_eq!(rows[0].name.as_ptr(), source[5..].as_ptr());
    let Value::Ranked(saved) = bindings.get("saved", 0..5).unwrap() else {
        panic!("saved ranked map")
    };
    assert_eq!(saved[0].0, "zeta");
    assert_eq!(saved.len(), 3);
    assert!(
        matches!(run("map_by(names, map_by(names, keep)) | sort_by(value)", vec![row("a", 0, false)]), Err(EvaluationError::Host(error)) if error.message == "scalar projection required")
    );
}

fn scalar_rank(
    entries: Vec<(&str, ScalarMapKey)>,
    budget: &mut Budget,
) -> Result<Vec<(String, ScalarMapKey)>, EvaluationError<()>> {
    zirql::rank_map_by_value(
        entries
            .into_iter()
            .map(|(key, value)| (key.to_owned(), value))
            .collect(),
        budget,
        Location::Query(8..22),
        |key, budget| {
            Ok(match key {
                ScalarMapKey::Null => ScalarMapKey::Null,
                ScalarMapKey::Boolean(value) => ScalarMapKey::Boolean(*value),
                ScalarMapKey::Integer(value) => ScalarMapKey::Integer(*value),
                ScalarMapKey::Unsigned(value) => ScalarMapKey::Unsigned(*value),
                ScalarMapKey::Float(value) => ScalarMapKey::Float(*value),
                ScalarMapKey::String(value) => {
                    budget.charge(value.len(), Location::Query(8..22))?;
                    budget.charge_results(value.len(), Location::Query(8..22))?;
                    ScalarMapKey::String(value.clone())
                }
            })
        },
    )
}

#[test]
fn scalar_map_ranking_keeps_exact_numbers_categories_and_lexical_ties() {
    use ScalarMapKey as K;
    let mut budget = Budget::for_program(limits());
    let numbers = scalar_rank(
        vec![
            ("unsigned", K::Unsigned(u64::MAX)),
            ("beyond", K::Float(18_446_744_073_709_551_616.0)),
            ("integer", K::Integer(9_007_199_254_740_993)),
            ("float", K::Float(9_007_199_254_740_992.0)),
            ("negative", K::Integer(-1)),
            ("fraction", K::Float(-1.5)),
        ],
        &mut budget,
    )
    .unwrap();
    assert_eq!(
        numbers
            .iter()
            .map(|(key, _)| key.as_str())
            .collect::<Vec<_>>(),
        [
            "fraction", "negative", "float", "integer", "unsigned", "beyond"
        ]
    );
    let mut nulls = scalar_rank(vec![("z", K::Null), ("a", K::Null)], &mut budget).unwrap();
    assert_eq!(nulls[0].0, "a");
    zirql::select_ranked_map(
        &mut nulls,
        MapSelection::Reverse,
        &mut budget,
        Location::Query(8..22),
    )
    .unwrap();
    assert_eq!(nulls[0].0, "z");
    let reranked = zirql::rank_map_by_value(nulls, &mut budget, Location::Query(8..22), |_, _| {
        Ok::<_, EvaluationError<()>>(K::Null)
    })
    .unwrap();
    assert_eq!(reranked[0].0, "a");
    let booleans = scalar_rank(
        vec![("true", K::Boolean(true)), ("false", K::Boolean(false))],
        &mut budget,
    )
    .unwrap();
    assert_eq!(booleans[0].0, "false");
    let mut strings = scalar_rank(
        vec![
            ("first", K::String("β".into())),
            ("last", K::String("a".into())),
        ],
        &mut budget,
    )
    .unwrap();
    assert_eq!(strings[0].0, "last");
    zirql::select_ranked_map(
        &mut strings,
        MapSelection::Tail(usize::MAX),
        &mut budget,
        Location::Query(8..22),
    )
    .unwrap();
    assert_eq!(strings.len(), 2);
    zirql::select_ranked_map(
        &mut strings,
        MapSelection::Head(0),
        &mut budget,
        Location::Query(8..22),
    )
    .unwrap();
    assert!(strings.is_empty());
    assert!(scalar_rank(vec![], &mut budget).unwrap().is_empty());
}

#[test]
fn scalar_map_refusals_keep_locations_and_legacy_work_limits() {
    use ScalarMapKey as K;
    for entries in [
        vec![("a", K::Null), ("b", K::Boolean(false))],
        vec![("a", K::Float(f64::INFINITY))],
    ] {
        let error = scalar_rank(entries, &mut Budget::for_program(limits())).unwrap_err();
        assert!(
            matches!(error, EvaluationError::Core(error) if error.location == Location::Query(8..22) && error.category == QueryErrorCategory::Other)
        );
    }
    let key = "a".repeat(100);
    let error = scalar_rank(
        vec![(&key, K::Null), (&format!("{key}b"), K::Null)],
        &mut Budget::new(zirql::Limits {
            work: 12,
            items: 16,
            nesting: 16,
        }),
    )
    .unwrap_err();
    assert!(
        matches!(error, EvaluationError::Core(error) if error.location == Location::Query(8..22) && error.category == QueryErrorCategory::WorkLimit)
    );
    let error = scalar_rank(
        vec![("a", K::Null)],
        &mut Budget::for_program(ProgramLimits {
            results: 1,
            ..limits()
        }),
    )
    .unwrap_err();
    assert!(
        matches!(error, EvaluationError::Core(error) if error.location == Location::Query(8..22) && error.category == QueryErrorCategory::ResultLimit)
    );
    let mut entries = vec![("a".into(), ())];
    let error = zirql::select_ranked_map(
        &mut entries,
        MapSelection::Reverse,
        &mut Budget::new(zirql::Limits {
            work: 0,
            items: 16,
            nesting: 16,
        }),
        Location::Source(3..7),
    )
    .unwrap_err();
    assert_eq!(error.location, Location::Source(3..7));
    assert_eq!(entries, vec![("a".into(), ())]);
}

#[test]
fn map_selector_keeps_binding_and_local_precedence() {
    let mut budget = Budget::for_program(limits());
    let query = zirql::read_native_expression(
        &mut zirql::ArgumentReader::new("sort_by(value)", 0).unwrap(),
        &mut Words {
            value_binding: true,
        },
        &mut budget,
        16,
    )
    .unwrap();
    let zirql::ExpressionKind::Sequence { steps, .. } = query.value else {
        panic!("sequence")
    };
    assert!(
        matches!(&steps[0].value, ExpressionStep::Stage(Stage::Order(order)) if matches!(&order.selector.as_ref().unwrap().value, zirql::ExpressionKind::Sequence { input, steps } if steps.is_empty() && matches!(&input.value, zirql::ExpressionKind::Binding(name) if name == "value")))
    );
    let expression = zirql::read_value_expression(
        &mut zirql::ArgumentReader::new("[input | sort_by(value) for value in [1]]", 0).unwrap(),
        &mut Words::default(),
        &mut budget,
        16,
    )
    .unwrap();
    let zirql::ValueExpressionKind::Comprehension { head, .. } = expression.value else {
        panic!("comprehension")
    };
    let zirql::ValueExpressionKind::NativeQuery(query) = head.value else {
        panic!("native head")
    };
    let zirql::ExpressionKind::Sequence { steps, .. } = query.value else {
        panic!("sequence")
    };
    assert!(
        matches!(&steps[0].value, ExpressionStep::Stage(Stage::Order(order)) if matches!(&order.selector.as_ref().unwrap().value, zirql::ExpressionKind::Sequence { input, steps } if steps.is_empty() && matches!(&input.value, zirql::ExpressionKind::Binding(name) if name == "value")))
    );
    assert!(run("value", vec![]).is_err());
    let query = zirql::read_native_expression(
        &mut zirql::ArgumentReader::new("min_by(value)", 0).unwrap(),
        &mut Words::default(),
        &mut budget,
        16,
    )
    .unwrap();
    let zirql::ExpressionKind::Sequence { steps, .. } = query.value else {
        panic!("sequence")
    };
    assert!(matches!(
        &steps[0].value,
        ExpressionStep::Stage(Stage::Order(_))
    ));
}

#[test]
fn callable_map_retains_legacy_work_and_enclosing_nesting_accounting() {
    let mut budget = Budget::new(Limits {
        work: 0,
        items: 1,
        nesting: 0,
    });
    let map = zirql::map_native_by(
        vec!["a"],
        &mut (),
        &mut budget,
        Location::Query(8..29),
        |key, _, _| Ok::<_, EvaluationError<()>>(Some(vec![(*key).into()])),
        |value, _, _| Ok(value),
    )
    .unwrap();
    assert_eq!(map["a"], "a");
    assert_eq!(budget.remaining_work(), 0);
}
