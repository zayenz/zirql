//! A constructed-value host for the documentation's word-analysis examples.
//!
//! Count unordered neighbouring word sets into a dictionary:
//! cargo run --locked --example words -- '[{words[i], words[i + 1]} | i in index_set(words) where i < length(words)] | tally'
use zirql::{
    Budget, ConstructedBinding, ConstructedProgram, ConstructedValue, DeclaredType, FunctionLimits,
    Location, ParameterInputs, ProgramLimits,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let query = std::env::args()
        .nth(1)
        .unwrap_or_else(|| r#"words | windows(2) | map(_[1] ++ " " ++ _[2])"#.into());
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
    let words = ConstructedBinding::new(
        words,
        DeclaredType::Sequence(Box::new(DeclaredType::String)),
        &mut budget,
        location,
    )?;
    let program = ConstructedProgram::read(&query, &["words"], &mut budget, 64)?;
    let result = program.evaluate(
        &[("words", &words)],
        Some(&words),
        ParameterInputs::Cli(&[]),
        FunctionLimits { active_depth: 32 },
        &mut budget,
    )?;
    println!("{result:?}");
    Ok(())
}
