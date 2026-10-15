use std::{collections::BTreeMap, ops::Range};

use zirql::{
    Bindings, Budget, EvaluationError, Expression, ExpressionStep, Location, NativeMap,
    ProgramHost, ProgramLimits, QueryError, QueryErrorCategory, ValueReaderHost,
};

type Query = Expression<Stage, (), ()>;
#[derive(Debug, PartialEq, Eq)]
enum Stage {
    Names,
    Missing,
    Many,
    Wrong,
    Value,
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
}
struct Words;
impl ValueReaderHost for Words {
    type Stage = Stage;
    type SetOperator = ();
    type Emitter = ();
    type Predicate = ();
    fn is_binding(&self, _: &str) -> bool {
        false
    }
    fn is_stage(&self, name: &str) -> bool {
        matches!(name, "names" | "missing" | "many" | "wrong" | "value")
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
            "value" => Stage::Value,
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
        budget.charge_results(1, Location::Query(range.clone()))?;
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
            Stage::Value => {
                if rows.iter().any(|row| row.fails) {
                    return Err(EvaluationError::Host(QueryError::query(
                        range.clone(),
                        "value failed",
                    )));
                }
                Value::Rows(rows)
            }
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
        &mut Words,
        &mut budget,
        16,
    )?;
    zirql::evaluate_expression(
        &query,
        &Words,
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
        run("map_by(names, value)", vec![beta.clone(), alpha.clone()]).unwrap()
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
        run("map_by(names, value)", vec![]).unwrap(),
        Value::Map(BTreeMap::new())
    );
    assert!(
        matches!(run("map_by(names, map_by(names, value))", vec![row("a", 0, false)]).unwrap(), Value::Map(map) if matches!(&map["a"], Value::Map(nested) if matches!(&nested["a"], Value::Rows(_))))
    );
}
#[test]
fn key_failures_and_input_order_precede_later_values() {
    for text in [
        "map_by(missing, value)",
        "map_by(many, value)",
        "map_by(wrong, value)",
    ] {
        let EvaluationError::Core(error) = run(text, vec![row("a", 0, true)]).unwrap_err() else {
            panic!("core key failure")
        };
        assert_eq!(error.message, "map_by key must produce exactly one string");
        assert_eq!(error.location, Location::Query(0..text.len()));
    }
    let error = run(
        "map_by(names, value)",
        vec![row("a", 0, false), row("a", 2, true)],
    )
    .unwrap_err();
    assert!(
        matches!(error, EvaluationError::Core(error) if error.message.contains("duplicate key `a`"))
    );
    let error = run(
        "map_by(names, value)",
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
