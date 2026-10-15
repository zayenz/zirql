//! A constructed-value host for the documentation's word-analysis examples.
use std::{collections::BTreeMap, ops::Range};
use zirql::{
    ArgumentReader, Budget, ConstructedValue, ExpressionStep, Location, Predicate, ProgramLimits,
    Projection, QueryError, ReadExpression, ValueQueryHost, ValueReaderHost, ValueType,
    constructed_value_type, copy_constructed_value, dispose_native_value, evaluate_query_value,
    read_value_expression,
};

struct Words {
    words: ConstructedValue,
}

// This host exposes one constructed binding. It registers no native operations.
impl ValueReaderHost for Words {
    type Stage = ();
    type SetOperator = ();
    type Emitter = ();
    type Predicate = ();

    fn is_binding(&self, name: &str) -> bool {
        name == "words"
    }

    fn is_stage(&self, _: &str) -> bool {
        false
    }

    fn parse_stage(
        &mut self,
        reader: &mut ArgumentReader<'_>,
        _: &mut Budget,
    ) -> Result<ExpressionStep<(), ()>, QueryError> {
        Err(reader.error("this word host has no native stages"))
    }

    fn parse_predicate_leaf(
        &mut self,
        reader: &mut ArgumentReader<'_>,
        _: &mut Budget,
    ) -> Result<(), QueryError> {
        Err(reader.error("use string expressions to select words"))
    }

    fn filter(&mut self, _: Predicate<()>, _: Range<usize>) {}

    fn projection(&mut self, _: Projection<ReadExpression<Self>>, _: Range<usize>) {}

    fn set_operator(&self, _: &str) -> Option<()> {
        None
    }
}

impl ValueQueryHost<ReadExpression<Self>> for Words {
    type Error = QueryError;

    fn query_types(
        &mut self,
        _: &ReadExpression<Self>,
        _: &[(&str, Vec<ValueType>)],
        _: &mut Budget,
        range: &Range<usize>,
    ) -> Result<Vec<ValueType>, QueryError> {
        Err(QueryError::query(
            range.clone(),
            "native queries are unavailable",
        ))
    }

    fn evaluate_query(
        &mut self,
        query: &ReadExpression<Self>,
        _: &[(&str, ConstructedValue)],
        _: &mut Budget,
    ) -> Result<ConstructedValue, QueryError> {
        Err(QueryError::query(
            query.range.clone(),
            "native queries are unavailable",
        ))
    }

    fn read_binding(
        &mut self,
        name: &str,
        range: &Range<usize>,
        budget: &mut Budget,
    ) -> Result<ConstructedValue, QueryError> {
        if name != "words" {
            return Err(QueryError::query(
                range.clone(),
                "unknown word-host binding",
            ));
        }
        copy_constructed_value(&self.words, budget, Location::Query(range.clone()))
    }
}

fn main() -> Result<(), QueryError> {
    let query = std::env::args().nth(1).unwrap_or_else(|| {
        r#"[words[i] ++ " " ++ words[i + 1]
 for i in index_set(words) if i < length(words)]"#
            .into()
    });
    let source = "ash apple birch apple";
    let mut budget = Budget::for_program(ProgramLimits {
        work: 1_000_000,
        results: 100_000,
        nesting: 64,
        stream_items: 1_000,
    });
    // Admit the scan, string bytes and array entries before allocating them.
    let location = Location::Source(0..source.len());
    budget.charge(source.len(), location.clone())?;
    let count = source.split_whitespace().count();
    budget.check_items(count, location.clone())?;
    budget.charge(source.len() + count, location.clone())?;
    budget.charge_results(1 + count, location.clone())?;
    let words = ConstructedValue::Array(
        source
            .split_whitespace()
            .map(|w| ConstructedValue::String(w.into()))
            .collect(),
    );
    let types = BTreeMap::from([(
        "words".into(),
        vec![constructed_value_type(&words, &mut budget, location)?],
    )]);
    let mut host = Words { words };
    let mut reader = ArgumentReader::new(&query, 0)?;
    let expression = read_value_expression(&mut reader, &mut host, &mut budget, 64)?;
    reader.skip_whitespace();
    let result = if reader.remaining().is_empty() {
        evaluate_query_value(&expression, &types, &mut host, &mut budget)
    } else {
        Err(reader.error("expected the end of the expression"))
    };
    dispose_native_value(expression);
    println!("{:#?}", result?);
    Ok(())
}
