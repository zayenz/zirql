//! CSV rows as constructed Zirql values, with named inputs and buffered output.
//!
//! [Query examples](https://zayenz.se/zirql/guides/csv/) show the inputs and results.
//! [Build the CSV host](https://zayenz.se/zirql/guides/csv-host/) explains this adapter.
use std::{
    fmt,
    fs::File,
    io::{self, Read, Write},
    path::PathBuf,
};
use zirql::{
    Budget, ConstructedBinding, ConstructedProgram, ConstructedValue as V, DeclaredType,
    FunctionLimits, JsonLayout, JsonRepresentation, Location, ParameterInputs, ProgramLimits,
    QueryError, render_json, render_markdown,
};

const MAX_INPUT_BYTES: usize = 64 * 1024;
const LIMITS: ProgramLimits = ProgramLimits {
    work: 1_000_000,
    results: 100_000,
    nesting: 64,
    stream_items: 1_000,
};

#[derive(Debug)]
struct CsvError {
    context: String,
    error: QueryError,
}
impl fmt::Display for CsvError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.context, self.error)
    }
}
impl std::error::Error for CsvError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.error)
    }
}

struct InputSpec {
    name: Option<String>,
    path: PathBuf,
}
impl InputSpec {
    fn parse(argument: String) -> Self {
        match argument.split_once('=') {
            Some((name, path)) => Self {
                name: Some(name.into()),
                path: path.into(),
            },
            None => Self {
                name: None,
                path: argument.into(),
            },
        }
    }
    fn label(&self) -> String {
        format!(
            "table {} ({})",
            self.name.as_deref().unwrap_or("rows"),
            self.path.display()
        )
    }
}

#[derive(Clone, Copy)]
enum OutputFormat {
    Json,
    Markdown,
}

struct Arguments {
    format: OutputFormat,
    inputs: Vec<InputSpec>,
    parameters: Vec<String>,
}

fn parse_arguments(arguments: impl Iterator<Item = String>) -> Result<Arguments, String> {
    let mut format = OutputFormat::Json;
    let mut inputs = Vec::new();
    let mut parameters = Vec::new();
    let mut arguments = arguments;
    while let Some(argument) = arguments.next() {
        if argument == "--format" {
            format = match arguments.next().as_deref() {
                Some("json") => OutputFormat::Json,
                Some("markdown") => OutputFormat::Markdown,
                _ => return Err("--format requires json or markdown".into()),
            };
        } else if argument == "--param" {
            parameters.push(arguments.next().ok_or("--param requires name=value")?);
        } else {
            inputs.push(InputSpec::parse(argument));
        }
    }
    Ok(Arguments {
        format,
        inputs,
        parameters,
    })
}

fn main() {
    let mut arguments = std::env::args().skip(1);
    let Some(query) = arguments.next() else {
        eprintln!(
            "usage: cargo run --example csv -- 'QUERY' [--format json|markdown] [--param name=value] FILE.csv | name=FILE.csv ..."
        );
        std::process::exit(1);
    };
    let Arguments {
        format,
        inputs,
        parameters,
    } = match parse_arguments(arguments) {
        Ok(arguments) => arguments,
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(1);
        }
    };
    match run(&query, &inputs, &parameters, format, LIMITS) {
        Ok(output) => {
            if let Err(error) = io::stdout().lock().write_all(&output) {
                eprintln!("writing output: {error}");
                std::process::exit(1);
            }
        }
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(1);
        }
    }
}

fn run(
    query: &str,
    inputs: &[InputSpec],
    parameters: &[String],
    format: OutputFormat,
    limits: ProgramLimits,
) -> Result<Vec<u8>, CsvError> {
    let context = || {
        format!(
            "query {query:?}; inputs: {}",
            inputs
                .iter()
                .map(InputSpec::label)
                .collect::<Vec<_>>()
                .join(", ")
        )
    };
    let query_error = |error| CsvError {
        context: context(),
        error,
    };
    let mut budget = Budget::for_program(limits);
    admit_inputs(inputs, &mut budget).map_err(query_error)?;
    let mut total_bytes = 0;
    let mut tables = Vec::new();
    for input in inputs {
        let location = Location::Source(0..0);
        budget.charge(1, location.clone()).map_err(query_error)?;
        budget.charge_results(1, location).map_err(query_error)?;
        let table = load_table(input, &mut total_bytes, &mut budget).map_err(|error| CsvError {
            context: input.label(),
            error,
        })?;
        tables.push(table);
    }
    let mut named = Vec::new();
    let mut names = vec!["rows"];
    for (input, table) in inputs.iter().zip(&tables) {
        if let Some(name) = input.name.as_deref() {
            budget
                .charge(name.len() + 1, Location::Query(0..query.len()))
                .map_err(query_error)?;
            named.push((name, table));
            names.push(name);
        }
    }
    let current = if tables.len() == 1 {
        named.push(("rows", &tables[0]));
        Some(&tables[0])
    } else {
        None
    };
    let program = ConstructedProgram::read(query, &names, &mut budget, limits.nesting)
        .map_err(query_error)?;
    let result = program
        .evaluate(
            &named,
            current,
            ParameterInputs::Cli(parameters),
            FunctionLimits { active_depth: 32 },
            &mut budget,
        )
        .map_err(|failure| {
            let mut error = failure.error;
            if tables.len() > 1
                && (error.message == "constructed current input is unavailable"
                    || error.message == "undefined binding: rows")
            {
                error.message = format!(
                    "implicit CSV input is ambiguous; choose a named table: {}",
                    inputs
                        .iter()
                        .filter_map(|input| input.name.as_deref())
                        .collect::<Vec<_>>()
                        .join(", ")
                );
            }
            let mut context = context();
            for call in failure.calls {
                context.push_str(&format!("; in {} at {:?}", call.function, call.range));
            }
            CsvError { context, error }
        })?;
    let location = Location::Query(0..query.len());
    let output = match format {
        OutputFormat::Json => {
            let mut output = render_json(
                &result,
                JsonRepresentation::Ordinary,
                JsonLayout::Compact,
                &mut budget,
                location.clone(),
            )
            .map_err(query_error)?;
            budget.charge(1, location.clone()).map_err(query_error)?;
            budget.charge_results(1, location).map_err(query_error)?;
            output.push(b'\n');
            output
        }
        OutputFormat::Markdown => {
            render_markdown(&result, &mut budget, location).map_err(query_error)?
        }
    };
    Ok(output)
}

fn admit_inputs(inputs: &[InputSpec], budget: &mut Budget) -> Result<(), QueryError> {
    let location = Location::Query(0..0);
    budget.check_items(inputs.len(), location.clone())?;
    if inputs.is_empty() {
        return Err(QueryError::query(
            0..0,
            "provide one CSV file or named CSV files",
        ));
    }
    let named = inputs[0].name.is_some();
    if inputs.iter().any(|input| input.name.is_some() != named) || (!named && inputs.len() != 1) {
        return Err(QueryError::query(
            0..0,
            "use exactly one unnamed file, or only named files",
        ));
    }
    for (index, input) in inputs.iter().enumerate() {
        budget.charge(input.path.as_os_str().len() + 1, location.clone())?;
        if let Some(name) = &input.name {
            budget.charge(name.len() + index + 1, location.clone())?;
            if !is_table_name(name) {
                return Err(QueryError::query(
                    0..0,
                    format!("invalid or reserved table name {name:?}"),
                ));
            }
            if inputs[..index]
                .iter()
                .any(|other| other.name.as_ref() == Some(name))
            {
                return Err(QueryError::query(
                    0..0,
                    format!("duplicate table name {name:?}"),
                ));
            }
        }
    }
    Ok(())
}

fn is_table_name(name: &str) -> bool {
    zirql::is_unquoted_binding_name(name) && !zirql::is_common_keyword(name) && name != "rows"
}

type Table = ConstructedBinding;

fn load_table(
    input: &InputSpec,
    total: &mut usize,
    budget: &mut Budget,
) -> Result<Table, QueryError> {
    let mut file = File::open(&input.path)
        .map_err(|error| QueryError::source(0..0, format!("opening CSV: {error}")))?;
    let mut bytes = Vec::new();
    let mut chunk = [0; 4096];
    loop {
        let remaining = MAX_INPUT_BYTES - *total;
        let count = file
            .read(&mut chunk[..4096.min(remaining + 1)])
            .map_err(|error| {
                QueryError::source(bytes.len()..bytes.len(), format!("reading CSV: {error}"))
            })?;
        if count == 0 {
            break;
        }
        let location = Location::Source(bytes.len()..bytes.len() + count);
        budget.charge(count, location.clone())?;
        if count > remaining {
            return Err(QueryError::source(
                bytes.len()..bytes.len() + count,
                "CSV inputs exceed the aggregate 64 KiB byte limit",
            ));
        }
        *total += count;
        bytes.extend_from_slice(&chunk[..count]);
    }
    decode_table(&bytes, budget)
}

fn decode_table(bytes: &[u8], budget: &mut Budget) -> Result<Table, QueryError> {
    // The byte cap also bounds the decoder's temporary record buffers.
    budget.charge(bytes.len() + 1, Location::Source(0..bytes.len()))?;
    let mut reader = csv::ReaderBuilder::new().from_reader(bytes);
    let header_record = reader.headers().map_err(csv_error)?;
    let header_location = csv_location(header_record.position());
    budget.check_items(header_record.len(), header_location.clone())?;
    budget.charge(header_record.len() + 1, header_location.clone())?;
    if header_record.is_empty() {
        return Err(QueryError::source(
            0..0,
            "CSV header is empty (record 0, line 1, byte 0)",
        ));
    }
    let mut headers: Vec<String> = Vec::new();
    let mut declaration = Vec::new();
    for header in header_record {
        budget.charge(header.len() + headers.len() + 1, header_location.clone())?;
        if header.is_empty() || headers.iter().any(|previous| previous == header) {
            return Err(QueryError {
                location: header_location.clone(),
                message: format!(
                    "empty or duplicate CSV header {header:?} ({})",
                    csv_position(header_record.position())
                ),
                category: zirql::QueryErrorCategory::Other,
            });
        }
        budget.charge(header.len() + 1, header_location.clone())?;
        declaration.push((header.to_owned(), DeclaredType::String));
        headers.push(header.to_owned());
    }
    budget.charge_results(1, header_location.clone())?;
    let mut rows = Vec::new();
    for record in reader.records() {
        let record = record.map_err(csv_error)?;
        let location = csv_location(record.position());
        budget.check_items(rows.len() + 1, location.clone())?;
        budget.charge(record.len() + 1, location.clone())?;
        budget.charge_results(record.len() + 1, location.clone())?;
        let mut fields = Vec::new();
        for (header, cell) in headers.iter().zip(record.iter()) {
            let size = header
                .len()
                .checked_add(cell.len())
                .and_then(|size| size.checked_add(1))
                .ok_or_else(|| QueryError::source(0..0, "CSV field size overflow"))?;
            budget.charge(size, location.clone())?;
            fields.push((header.clone(), V::String(cell.to_owned())));
        }
        rows.push(V::Record(fields));
    }
    ConstructedBinding::new(
        V::Array(rows),
        DeclaredType::Sequence(Box::new(DeclaredType::Record(declaration))),
        budget,
        header_location,
    )
}

fn csv_location(position: Option<&csv::Position>) -> Location {
    let byte = position
        .and_then(|position| usize::try_from(position.byte()).ok())
        .unwrap_or(0);
    Location::Source(byte..byte)
}
fn csv_error(error: csv::Error) -> QueryError {
    let location = csv_location(error.position());
    let position = csv_position(error.position());
    QueryError {
        location,
        message: format!("{position}: {error}"),
        category: zirql::QueryErrorCategory::Other,
    }
}
fn csv_position(position: Option<&csv::Position>) -> String {
    position
        .map(|position| {
            format!(
                "record {}, line {}, byte {}",
                position.record(),
                position.line(),
                position.byte()
            )
        })
        .unwrap_or_else(|| "unknown CSV position".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use zirql::QueryErrorCategory;

    const REPORT: &str = r#"let selected = people | filter(p => p.active == "yes");
let groups = [{"team": t.label, "members": [p.name for p in selected if p.team == t.id]} for t in teams];
[{"team": g.team, "members": sort(g.members), "count": length(g.members)} for g in groups]"#;
    const REPORT_JSON: &str = "[{\"team\":\"Runtime\",\"members\":[\"Barbara Liskov\"],\"count\":1},{\"team\":\"Compiler\",\"members\":[\"Ada, Lovelace\",\"Grace Hopper\"],\"count\":2},{\"team\":\"Operations\",\"members\":[],\"count\":0}]\n";

    fn people() -> Vec<InputSpec> {
        vec![InputSpec::parse("examples/csv/people.csv".into())]
    }
    fn named() -> Vec<InputSpec> {
        [
            "people=examples/csv/people.csv",
            "teams=examples/csv/teams.csv",
        ]
        .into_iter()
        .map(|argument| InputSpec::parse(argument.into()))
        .collect()
    }
    fn text(query: &str, inputs: &[InputSpec]) -> String {
        String::from_utf8(run(query, inputs, &[], OutputFormat::Json, LIMITS).unwrap()).unwrap()
    }
    fn refusal(query: &str, inputs: &[InputSpec], message: &str) -> CsvError {
        let error = run(query, inputs, &[], OutputFormat::Json, LIMITS).unwrap_err();
        assert!(error.to_string().contains(message), "{error}");
        assert!(error.context.contains(&format!("{query:?}")));
        assert!(matches!(error.error.location, Location::Query(_)));
        error
    }

    #[test]
    fn fixture_queries_keep_decoded_strings_saved_values_and_written_order() {
        assert_eq!(
            text(
                r#"[{"ash", "apple"}, {"apple", "birch"}, {"birch", "apple"}] | tally"#,
                &named(),
            ),
            concat!(
                r#"{"kind":"dictionary","entries":[{"key":["ash","apple"],"value":1},{"key":["apple","birch"],"value":2}]}"#,
                "\n",
            )
        );
        let fields = r#"[outer.items | filter(_ > 1) for outer in [{"items": [1, 2, 3]}]]"#;
        assert_eq!(text(fields, &people()), "[[2,3]]\n");
        assert_eq!(
            text(
                r#"[(outer.items) | filter(_ > 1) for outer in [{"items": [1, 2, 3]}]]"#,
                &people()
            ),
            text(fields, &people())
        );
        assert_eq!(
            text(
                r#"[{"items": [1, 2, 3], "minimum": 1}] | filter(outer => length(outer.items | filter(_ > outer.minimum)) == 2) | count"#,
                &people()
            ),
            "1\n"
        );
        assert_eq!(
            text("let xs = [1,2,3]; [x*2 for x in xs if x>1]", &people()),
            "[4,6]\n"
        );
        assert_eq!(
            text("let xs = [1,2,3]; [x*2 | x in xs where x>1]", &people()),
            "[4,6]\n"
        );
        assert_eq!(
            text(
                r#"let 'yes' = "yes"; [name | {name,active,..} in people where active == 'yes']"#,
                &named()
            ),
            text(r#"[p.name for p in people if p.active == "yes"]"#, &named())
        );
        let malformed = "let xs = [1]; [x | x in xs where ]";
        let error = refusal(malformed, &people(), "expected");
        assert_eq!(
            error.error.location,
            Location::Query(malformed.len() - 1..malformed.len())
        );
        assert_eq!(
            text(
                r#"[{"name": p.name, "note": p.note} for p in rows if p.active == "yes"]"#,
                &people()
            ),
            "[{\"name\":\"Grace Hopper\",\"note\":\"Said \\\"hello\\\"\"},{\"name\":\"Ada, Lovelace\",\"note\":\"First line\\nsecond line\"},{\"name\":\"Barbara Liskov\",\"note\":\"\"}]\n"
        );
        assert_eq!(
            text(r#"filter(_.active == "yes")"#, &people()),
            text(r#"[p for p in rows if p.active == "yes"]"#, &people())
        );
        assert_eq!(
            text(
                r#"let wanted = "yes"; people | filter(p => p.active == wanted)"#,
                &named()
            ),
            text(r#"people | filter(_.active == "yes")"#, &named())
        );
        assert_eq!(text("count", &people()), "4\n");
        assert_eq!(
            text("rows | count", &people()),
            text("(rows) | count", &people())
        );
        assert_eq!(text("people | count", &named()), "4\n");
        assert_eq!(
            text(
                "let numbers = [3, 1, 3]; numbers | sort | unique | count",
                &people()
            ),
            "2\n"
        );
        assert_eq!(
            text(
                r#"let selected = rows | filter(_.active == "yes"); selected | count"#,
                &people()
            ),
            "3\n"
        );
        assert_eq!(
            text(
                r#"let selected = people | filter(p => p.active == "yes"); selected | count"#,
                &named()
            ),
            text(
                r#"let selected = people | filter(p => p.active == "yes"); (selected) | count"#,
                &named()
            )
        );
        assert_eq!(
            text(
                r#"let [first, ..] = rows | filter({active: status, ..} => status == "yes" and _.active == status); let {name, ..} = first; name"#,
                &people()
            ),
            "\"Grace Hopper\"\n"
        );
        assert_eq!(
            text(
                r#"let [first, ..] = people | filter({active: status, ..} => status == "yes"); let {name, ..} = first; name"#,
                &named()
            ),
            "\"Grace Hopper\"\n"
        );
        let short = "let [a,b,..] = [1]; [a,b]";
        let error = refusal(short, &people(), "pattern did not match");
        assert_eq!(error.error.location, Location::Query(4..12));
        assert_eq!(text("fn f(x: int) -> int { x } f(1)", &people()), "1\n");
        assert_eq!(
            text(
                r#"/* fn hidden() -> int { 1 } */ let label = "fn inside() -> int { 2 }"; label"#,
                &people()
            ),
            "\"fn inside() -> int { 2 }\"\n"
        );
        let duplicate = "let chosen = 1; let chosen = 2; chosen";
        let error = refusal(duplicate, &people(), "already defined");
        assert_eq!(error.error.location, Location::Query(20..26));
        refusal(
            "let first = later; let later = 1; first",
            &people(),
            "later",
        );
        refusal("rows;", &people(), "final value");
        refusal("rows trailing", &people(), "expected");
        assert_eq!(text("rows | unique | count", &people()), "4\n");
        refusal(
            "rows | names",
            &people(),
            "unsupported constructed pipeline stage",
        );
        assert_eq!(text(REPORT, &named()), REPORT_JSON);
        assert_eq!(
            text(
                r#"let column = "name"; [lookup(rows[1], column), has_key(rows[1], column)]"#,
                &people()
            ),
            "[\"Grace Hopper\",true]\n"
        );
        assert_eq!(
            text(
                r#"let selected = people | filter(p => p.active == "yes"); let row = selected[1]; let column = "name"; {"row": row, "name": lookup(row, column), "present": has_key(row, column)}"#,
                &named()
            ),
            "{\"row\":{\"id\":\"2\",\"name\":\"Grace Hopper\",\"team\":\"compiler\",\"active\":\"yes\",\"note\":\"Said \\\"hello\\\"\"},\"name\":\"Grace Hopper\",\"present\":true}\n"
        );
        assert_eq!(
            text(
                r#"let row = {"name": "Ada"}; let column = "missing"; [lookup(row, column), has_key(row, column)]"#,
                &people()
            ),
            "[null,false]\n"
        );
        let query = r#"let column = "missing"; lookup(rows[1], column)"#;
        let error = refusal(query, &people(), "record label outside declared schema");
        assert_eq!(error.error.location, Location::Query(24..47));
        let query = r#"let row = people[1]; let column = "missing"; has_key(row, column)"#;
        let error = refusal(query, &named(), "record label outside declared schema");
        assert_eq!(error.error.location, Location::Query(45..65));
        assert_eq!(
            text("[p.note for p in rows if p.active == \"no\"]", &people()),
            "[\"01\"]\n"
        );
    }

    #[test]
    fn csv_admission_keeps_headers_exact_and_refuses_bad_records() {
        let mut budget = Budget::for_program(LIMITS);
        let error = decode_table(b"name,name\nAda,Grace\n", &mut budget).unwrap_err();
        assert!(error.message.contains("duplicate CSV header"));
        let error = decode_table(b"name,\nAda,Grace\n", &mut budget).unwrap_err();
        assert!(error.message.contains("empty or duplicate CSV header"));
        let error = decode_table(b"name,team\nAda\n", &mut budget).unwrap_err();
        assert!(error.message.contains("record 1, line 2, byte 10"));
        assert!(
            decode_table(b"", &mut budget)
                .unwrap_err()
                .message
                .contains("header is empty")
        );
        assert!(
            decode_table(b"name\n\xff\n", &mut budget)
                .unwrap_err()
                .message
                .contains("UTF-8")
        );

        let table = decode_table(b"full name, team\nAda,compiler\n", &mut budget).unwrap();
        let V::Array(rows) = table.value().data() else {
            panic!("CSV table is an array")
        };
        assert_eq!(
            rows,
            &vec![V::Record(vec![
                ("full name".into(), V::String("Ada".into())),
                (" team".into(), V::String("compiler".into()))
            ])]
        );
        let empty = decode_table(b"name,team\n", &mut budget).unwrap();
        let program = ConstructedProgram::read(
            "rows | filter(_.missing == \"yes\")",
            &["rows"],
            &mut budget,
            LIMITS.nesting,
        )
        .unwrap();
        let error = program
            .evaluate(
                &[("rows", &empty)],
                Some(&empty),
                ParameterInputs::Cli(&[]),
                FunctionLimits { active_depth: 32 },
                &mut budget,
            )
            .unwrap_err()
            .error;
        assert!(error.message.contains("declared schema"), "{error}");
    }

    #[test]
    fn table_scope_ambiguity_and_shared_limits_fail_with_query_context() {
        refusal(
            "[p.name for p in missing]",
            &named(),
            "undefined binding: missing",
        );
        refusal("[p.missing for p in rows]", &people(), "missing");
        refusal("count", &named(), "implicit CSV input is ambiguous");
        refusal("rows", &named(), "people, teams");
        refusal("let rows = []; rows", &people(), "binding");
        refusal("let x = []; let x = []; x", &people(), "binding");
        refusal(
            "let x = y; let y = []; x",
            &people(),
            "undefined binding: y",
        );
        refusal(
            "let selected = [p for p in rows]; p",
            &people(),
            "undefined binding: p",
        );
        let error = refusal("1..100001", &people(), "result limit exceeded");
        assert_eq!(error.error.category, QueryErrorCategory::ResultLimit);
        let names = ["rows", "alpha", "beta"];
        let source = format!("/* {} */ 1", "a".repeat(128));
        let error = ConstructedProgram::read(
            &source,
            &names,
            &mut Budget::for_program(ProgramLimits { work: 64, ..LIMITS }),
            LIMITS.nesting,
        )
        .err()
        .unwrap();
        assert_eq!(error.category, QueryErrorCategory::WorkLimit);
        let error = ConstructedProgram::read(
            "1",
            &names,
            &mut Budget::for_program(ProgramLimits {
                results: 2,
                ..LIMITS
            }),
            LIMITS.nesting,
        )
        .err()
        .unwrap();
        assert_eq!(error.category, QueryErrorCategory::ResultLimit);
        ConstructedProgram::read(
            &source,
            &names,
            &mut Budget::new(zirql::Limits {
                work: 64,
                items: LIMITS.stream_items,
                nesting: LIMITS.nesting,
            }),
            LIMITS.nesting,
        )
        .unwrap();
        let mut budget = Budget::for_program(LIMITS);
        assert!(admit_inputs(&[InputSpec::parse("rows=no.csv".into())], &mut budget).is_err());
        assert!(admit_inputs(&[InputSpec::parse("for=no.csv".into())], &mut budget).is_err());
        assert!(admit_inputs(&[InputSpec::parse("input=no.csv".into())], &mut budget).is_err());
        assert!(admit_inputs(&[InputSpec::parse("lookup=no.csv".into())], &mut budget).is_ok());
        assert!(
            admit_inputs(
                &[
                    InputSpec::parse("p=no.csv".into()),
                    InputSpec::parse("p=other.csv".into())
                ],
                &mut budget
            )
            .is_err()
        );
        assert!(
            admit_inputs(
                &[
                    InputSpec::parse("no.csv".into()),
                    InputSpec::parse("p=other.csv".into())
                ],
                &mut budget
            )
            .is_err()
        );
        assert!(
            admit_inputs(
                &[
                    InputSpec::parse("no.csv".into()),
                    InputSpec::parse("other.csv".into())
                ],
                &mut budget
            )
            .is_err()
        );
    }

    #[test]
    fn parameters_and_named_functions_keep_cli_inputs_separate() {
        let Arguments {
            format,
            inputs,
            parameters,
        } = parse_arguments(
            [
                "--param",
                "minimum=2",
                "orders=examples/csv/orders.csv",
                "--format",
                "markdown",
                "--param",
                "label=kept=exact",
            ]
            .into_iter()
            .map(String::from),
        )
        .unwrap();
        assert!(matches!(format, OutputFormat::Markdown));
        assert_eq!(inputs[0].name.as_deref(), Some("orders"));
        let query = "param minimum: int; param label: string; fn selected(xs: sequence<record {item: string, quantity: string, price: string}>, n: int) -> int { length([x for x in xs if parse_int(x.quantity) >= n]) } {\"count\": selected(orders, minimum), \"label\": label}";
        let output = run(query, &inputs, &parameters, OutputFormat::Json, LIMITS).unwrap();
        assert_eq!(
            String::from_utf8(output).unwrap(),
            "{\"count\":2,\"label\":\"kept=exact\"}\n"
        );
    }

    #[test]
    fn direct_keyed_output_keeps_unused_axis_keys() {
        let output = text("[2: 20, 5: 50] axes([5, 2, 9])", &people());
        let value: serde_json::Value = serde_json::from_str(&output).unwrap();
        assert_eq!(value["indexing"], "explicit");
        assert_eq!(
            value["axes"][0][2],
            serde_json::json!({"kind":"integer","value":9})
        );
        assert_eq!(value["entries"].as_array().unwrap().len(), 2);
        assert_eq!(value["entries"][0]["key"][0]["value"], 5);
        assert_eq!(
            value["entries"][1]["value"],
            serde_json::json!({"kind":"integer","value":20})
        );
        assert!(output.ends_with('\n'));
    }
}
