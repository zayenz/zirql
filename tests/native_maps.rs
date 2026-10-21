use std::ops::Range;

use zirql::{
    Bindings, Budget, EvaluationError, ExpressionStep, Limits, Location, MapSelection,
    NativeOrderKey, ProgramHost, ProgramLimits, QueryError, QueryErrorCategory, ValueReaderHost,
};

#[derive(Debug, PartialEq)]
enum Stage {
    Names,
    Missing,
    Many,
    Wrong,
    Keep,
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
    Map(Vec<(String, Value<'a>)>),
}
#[derive(Default)]
struct Words {
    value_binding: bool,
}
impl ValueReaderHost for Words {
    type Stage = Stage;
    type SetOperator = ();
    type Emitter = ();

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
        if name.value == "reverse" {
            return Ok(ExpressionStep::Collection(zirql::Stage::Reverse));
        }
        if matches!(name.value, "head" | "tail") {
            reader.expect('(')?;
            let count = reader.unsigned_integer()?.value;
            reader.expect(')')?;
            return Ok(ExpressionStep::Collection(if name.value == "head" {
                zirql::Stage::Head(count)
            } else {
                zirql::Stage::Tail(count)
            }));
        }
        Ok(ExpressionStep::Stage(match name.value {
            "names" => Stage::Names,
            "missing" => Stage::Missing,
            "many" => Stage::Many,
            "wrong" => Stage::Wrong,
            "keep" => Stage::Keep,
            _ => return Err(QueryError::query(name.range, "unknown word stage")),
        }))
    }
    fn parse_predicate_leaf(
        &mut self,
        reader: &mut zirql::ArgumentReader<'_>,
        _: &mut Budget,
    ) -> Result<Stage, QueryError> {
        Err(reader.error("no predicates"))
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
        _: &mut &'a str,
        _: &Bindings<Value<'a>>,
        budget: &mut Budget,
        range: &Range<usize>,
    ) -> Result<Value<'a>, EvaluationError<QueryError>> {
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
#[derive(Clone, PartialEq)]
enum Kind {
    Rows,
    Strings,
    Map,
    Constructed,
}
impl<'a> zirql::ProjectionHost<&'a str> for Words {
    type Kind = Kind;
    fn kind(
        &self,
        value: &Value<'a>,
        _: &mut Budget,
        _: &Range<usize>,
    ) -> Result<Kind, QueryError> {
        Ok(match value {
            Value::Rows(_) => Kind::Rows,
            Value::Strings(_) => Kind::Strings,
            Value::Map(_) => Kind::Map,
        })
    }
    fn scalar_types(
        &self,
        kind: &Kind,
        _: &mut Budget,
        _: &Range<usize>,
    ) -> Result<Option<Vec<zirql::ValueType>>, QueryError> {
        Ok(matches!(kind, Kind::Rows | Kind::Strings).then(|| {
            vec![zirql::ValueType::Array(zirql::ArrayType::Elements(vec![
                zirql::ValueType::String,
            ]))]
        }))
    }
    fn constructed_kind(&self, _: Vec<zirql::ValueType>) -> Kind {
        Kind::Constructed
    }
    fn stage_kind(
        &self,
        stage: &Stage,
        input: &Kind,
        _: &std::collections::BTreeMap<String, Kind>,
        _: &mut Budget,
        range: &Range<usize>,
    ) -> Result<Kind, QueryError> {
        if *input != Kind::Rows {
            return Err(QueryError::query(range.clone(), "words required"));
        }
        Ok(match stage {
            Stage::Names | Stage::Missing | Stage::Many => Kind::Strings,
            Stage::Wrong | Stage::Keep => Kind::Rows,
        })
    }
    fn is_pure(&self, _: &Stage) -> bool {
        true
    }
    fn nested_queries(
        &self,
        _: &Stage,
        _: &mut Budget,
        _: &Range<usize>,
    ) -> Result<zirql::NativeOperands<&'a str, Self>, QueryError> {
        Ok(vec![])
    }
    fn subjects(
        &self,
        input: Value<'a>,
        _: &mut &'a str,
        budget: &mut Budget,
        range: &Range<usize>,
    ) -> Result<Vec<Value<'a>>, EvaluationError<QueryError>> {
        let Value::Rows(rows) = input else {
            return Err(QueryError::query(range.clone(), "words required").into());
        };
        budget.charge_results(rows.len(), Location::Query(range.clone()))?;
        Ok(rows.into_iter().map(|row| Value::Rows(vec![row])).collect())
    }
    fn project_value(
        &self,
        value: Value<'a>,
        _: &mut &'a str,
        budget: &mut Budget,
        range: &Range<usize>,
    ) -> Result<zirql::ConstructedValue, EvaluationError<QueryError>> {
        let location = Location::Query(range.clone());
        let strings = match value {
            Value::Rows(rows) => {
                budget.charge_results(rows.len() + 1, location.clone())?;
                let mut strings = Vec::with_capacity(rows.len());
                for row in rows {
                    budget.charge(row.name.len(), location.clone())?;
                    strings.push(zirql::ConstructedValue::String(row.name.to_owned()));
                }
                strings
            }
            Value::Strings(strings) => {
                budget.charge_results(strings.len() + 1, location)?;
                strings
                    .into_iter()
                    .map(zirql::ConstructedValue::String)
                    .collect()
            }
            Value::Map(_) => {
                return Err(QueryError::query(
                    range.clone(),
                    "explicit dictionary projection required",
                )
                .into());
            }
        };
        Ok(zirql::ConstructedValue::Array(strings))
    }
    fn lift_constructed(
        &self,
        _: zirql::ConstructedValue,
        _: Vec<zirql::ValueType>,
        _: &mut &'a str,
        _: &mut Budget,
        range: &Range<usize>,
    ) -> Result<Value<'a>, EvaluationError<QueryError>> {
        Err(QueryError::query(range.clone(), "native-only fixture").into())
    }
    fn mapping_kind(
        &self,
        _: &Kind,
        _: &Kind,
        _: &mut Budget,
        _: &Range<usize>,
    ) -> Result<Kind, QueryError> {
        Ok(Kind::Map)
    }
    fn mapping_keys(
        &self,
        value: Value<'a>,
        _: &mut &'a str,
        _: &mut Budget,
        _: &Range<usize>,
    ) -> Result<Option<Vec<String>>, EvaluationError<QueryError>> {
        Ok(if let Value::Strings(keys) = value {
            Some(keys)
        } else {
            None
        })
    }
    fn construct_map(
        &self,
        entries: Vec<(String, Value<'a>)>,
        _: &mut &'a str,
        _: &mut Budget,
        _: &Range<usize>,
    ) -> Result<Value<'a>, EvaluationError<QueryError>> {
        Ok(Value::Map(entries))
    }
    fn is_map_kind(&self, kind: &Kind) -> bool {
        matches!(kind, Kind::Map)
    }
    fn map_sort_kind(
        &self,
        input: &Kind,
        _: &mut Budget,
        range: &Range<usize>,
    ) -> Result<Kind, QueryError> {
        if matches!(input, Kind::Map) {
            Ok(input.clone())
        } else {
            Err(QueryError::query(range.clone(), "dictionary required"))
        }
    }
    fn map_entries(
        &self,
        value: Value<'a>,
        _: &mut &'a str,
        _: &mut Budget,
        range: &Range<usize>,
    ) -> Result<Vec<(String, Value<'a>)>, EvaluationError<QueryError>> {
        let Value::Map(entries) = value else {
            return Err(QueryError::query(range.clone(), "dictionary required").into());
        };
        Ok(entries)
    }
    fn ordering_key(
        &self,
        value: Value<'a>,
        _: &mut &'a str,
        _: &mut Budget,
        range: &Range<usize>,
    ) -> Result<zirql::NativeOrderKey, EvaluationError<QueryError>> {
        let Value::Rows(rows) = value else {
            return Err(EvaluationError::Host(QueryError::query(
                range.clone(),
                "scalar projection required",
            )));
        };
        Ok(zirql::NativeOrderKey::Count(rows[0].source.start))
    }
    fn collection_kind(
        &self,
        _: &zirql::Stage<()>,
        input: &Kind,
        _: &mut Budget,
        _: &Range<usize>,
    ) -> Result<Kind, QueryError> {
        Ok(input.clone())
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
    zirql::evaluate_native_expression(
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
fn common_map_reader_and_constructor_retain_native_rows_and_insertion_order() {
    // Rows also admit scalar projection; map bodies must retain their native owners.
    let source = String::from("βeta alpha");
    let beta = row(&source[..5], 0, false);
    let alpha = row(&source[6..], 6, false);
    let Value::Map(entries) =
        run("map_by(names, keep)", vec![beta.clone(), alpha.clone()]).unwrap()
    else {
        panic!("map")
    };
    assert_eq!(
        entries
            .iter()
            .map(|(key, _)| key.as_str())
            .collect::<Vec<_>>(),
        ["βeta", "alpha"]
    );
    assert_eq!(entries[1].1, Value::Rows(vec![alpha]));
    let Value::Rows(rows) = &entries[0].1 else {
        panic!("native rows")
    };
    assert_eq!(rows, &[beta]);
    assert_eq!(rows[0].name.as_ptr(), source.as_ptr());
    assert_eq!(
        run("map_by(names, keep)", vec![]).unwrap(),
        Value::Map(Vec::new())
    );
    assert!(
        matches!(run("map_by(names, map_by(names, keep))", vec![row("a", 0, false)]).unwrap(), Value::Map(map) if matches!(&map[0].1, Value::Map(nested) if matches!(&nested[0].1, Value::Rows(_))))
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
fn native_dictionaries_keep_owners_and_order_through_saved_selection() {
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
    let Value::Map(entries) = &ranked else {
        panic!("dictionary")
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
    let Value::Map(selected) = zirql::evaluate_native_expression(
        &query,
        &Words::default(),
        Value::Rows(vec![]),
        &mut "",
        &bindings,
        &mut budget,
    )
    .unwrap() else {
        panic!("dictionary")
    };
    assert_eq!(selected[0].0, "alpha");
    let Value::Rows(rows) = &selected[0].1 else {
        panic!("native rows")
    };
    assert_eq!(rows[0].source, 5..10);
    assert_eq!(rows[0].name.as_ptr(), source[5..].as_ptr());
    let Value::Map(saved) = bindings.get("saved", 0..5).unwrap() else {
        panic!("saved dictionary")
    };
    assert_eq!(saved[0].0, "zeta");
    assert_eq!(saved.len(), 3);
    assert!(
        matches!(run("map_by(names, map_by(names, keep)) | sort_by(value)", vec![row("a", 0, false)]), Err(EvaluationError::Host(error)) if error.message == "scalar projection required")
    );
}

fn descriptor_sort(
    entries: Vec<(&str, NativeOrderKey)>,
    budget: &mut Budget,
) -> Result<Vec<(String, NativeOrderKey)>, EvaluationError<()>> {
    zirql::sort_map_by_value(
        entries
            .into_iter()
            .map(|(key, value)| (key.to_owned(), value))
            .collect(),
        budget,
        Location::Query(8..22),
        |key, budget| {
            Ok(match key {
                NativeOrderKey::Count(value) => NativeOrderKey::Count(*value),
                NativeOrderKey::Text(value) => {
                    budget.charge(value.len(), Location::Query(8..22))?;
                    budget.charge_results(value.len(), Location::Query(8..22))?;
                    NativeOrderKey::Text(value.clone())
                }
                NativeOrderKey::Value(value) => NativeOrderKey::Value(
                    zirql::copy_constructed_value(value, budget, Location::Query(8..22))?,
                ),
                NativeOrderKey::Json(value) => {
                    fn units(value: &serde_json::Value) -> usize {
                        match value {
                            serde_json::Value::String(value) => 1 + value.len(),
                            serde_json::Value::Array(values) => {
                                1 + values.iter().map(units).sum::<usize>()
                            }
                            serde_json::Value::Object(values) => {
                                1 + values
                                    .iter()
                                    .map(|(key, value)| key.len() + units(value))
                                    .sum::<usize>()
                            }
                            _ => 1,
                        }
                    }
                    let units = units(value);
                    budget.charge(units, Location::Query(8..22))?;
                    budget.charge_results(units, Location::Query(8..22))?;
                    NativeOrderKey::Json(value.clone())
                }
            })
        },
    )
}

#[test]
fn dictionary_sorting_keeps_exact_integer_domain_and_current_ties() {
    use zirql::{ConstructedValue as V, NativeOrderKey as K};
    let mut budget = Budget::for_program(limits());
    let entries = descriptor_sort(
        vec![
            ("unsigned", K::Json(serde_json::json!(u64::MAX))),
            ("float", K::Value(V::Float(-1.5))),
            (
                "adjacent-b",
                K::Json(serde_json::json!(9_007_199_254_740_994u64)),
            ),
            ("signed-max", K::Value(V::Integer(i64::MAX))),
            (
                "adjacent-a",
                K::Json(serde_json::json!(9_007_199_254_740_993u64)),
            ),
            ("negative", K::Value(V::Integer(-1))),
            ("absent", K::Value(V::Absent)),
            ("false", K::Value(V::Boolean(false))),
            ("compound-b", K::Json(serde_json::json!([u64::MAX]))),
            ("compound-a", K::Value(V::Array(vec![V::Integer(i64::MAX)]))),
        ],
        &mut budget,
    )
    .unwrap();
    assert_eq!(
        entries
            .iter()
            .map(|(key, _)| key.as_str())
            .collect::<Vec<_>>(),
        [
            "absent",
            "false",
            "negative",
            "adjacent-a",
            "adjacent-b",
            "signed-max",
            "unsigned",
            "float",
            "compound-a",
            "compound-b"
        ]
    );
    let mut entries = descriptor_sort(
        vec![
            ("z", K::Value(V::Integer(2))),
            ("a", K::Count(2)),
            ("m", K::Json(serde_json::json!(1))),
        ],
        &mut budget,
    )
    .unwrap();
    assert_eq!(
        entries
            .iter()
            .map(|(key, _)| key.as_str())
            .collect::<Vec<_>>(),
        ["m", "z", "a"]
    );
    zirql::select_map(
        &mut entries,
        MapSelection::Reverse,
        &mut budget,
        Location::Query(8..22),
    )
    .unwrap();
    let entries =
        zirql::sort_map_by_value(entries, &mut budget, Location::Query(8..22), |key, _| {
            Ok::<_, EvaluationError<()>>(match key {
                K::Count(value) => K::Count(*value),
                K::Value(V::Integer(value)) => K::Value(V::Integer(*value)),
                K::Json(value) => K::Json(value.clone()),
                _ => unreachable!("integer descriptors"),
            })
        })
        .unwrap();
    assert_eq!(
        entries
            .iter()
            .map(|(key, _)| key.as_str())
            .collect::<Vec<_>>(),
        ["m", "a", "z"]
    );
    assert!(descriptor_sort(vec![], &mut budget).unwrap().is_empty());
}

#[test]
fn map_refusals_keep_locations_and_legacy_work_limits() {
    use zirql::{ConstructedValue as V, NativeOrderKey as K};
    let error = descriptor_sort(
        vec![("a", K::Value(V::Float(f64::INFINITY)))],
        &mut Budget::for_program(limits()),
    )
    .unwrap_err();
    assert!(
        matches!(error, EvaluationError::Core(error) if error.location == Location::Query(8..22) && error.category == QueryErrorCategory::Other)
    );
    let error = descriptor_sort(
        vec![("a", K::Value(V::Absent)), ("b", K::Value(V::Absent))],
        &mut Budget::new(zirql::Limits {
            work: 2,
            items: 16,
            nesting: 16,
        }),
    )
    .unwrap_err();
    assert!(
        matches!(error, EvaluationError::Core(error) if error.location == Location::Query(8..22) && error.category == QueryErrorCategory::WorkLimit)
    );
    let error = descriptor_sort(
        vec![("a", K::Value(V::Absent))],
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
    let error = zirql::select_map(
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
        matches!(&steps[0].value, ExpressionStep::Ordering(order) if matches!(&order.selector.as_ref().unwrap().query().value, zirql::ExpressionKind::Sequence { input, steps } if steps.is_empty() && matches!(&input.value, zirql::ExpressionKind::Binding(name) if name == "value")))
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
        matches!(&steps[0].value, ExpressionStep::Ordering(order) if matches!(&order.selector.as_ref().unwrap().query().value, zirql::ExpressionKind::Sequence { input, steps } if steps.is_empty() && matches!(&input.value, zirql::ExpressionKind::Binding(name) if name == "value")))
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
    assert!(matches!(&steps[0].value, ExpressionStep::Ordering(_)));
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
    assert_eq!(map[0].1, "a");
    assert_eq!(budget.remaining_work(), 0);
}
