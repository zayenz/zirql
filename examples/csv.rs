//! CSV rows as constructed Zirql values, with named inputs and buffered JSON.
use std::{
    collections::BTreeMap,
    fmt,
    fs::File,
    io::{self, Read, Write},
    ops::Range,
    path::PathBuf,
};
use zirql::{
    ArgumentReader, ArrayType, Budget, ConstructedValue as V, DeclaredType, ExpressionStep,
    Located, Location, Predicate, ProgramLimits, Projection, QueryError, ReadExpression,
    ValueExpression, ValueExpressionKind as K, ValueQueryHost, ValueReaderHost, ValueType,
    copy_constructed_value, declare_constructed, dispose_native_value, dispose_pattern,
    evaluate_query_value, pattern_names, read_pattern_let, read_value_expression,
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

fn main() {
    let mut arguments = std::env::args().skip(1);
    let Some(query) = arguments.next() else {
        eprintln!("usage: cargo run --example csv -- 'QUERY' FILE.csv | name=FILE.csv ...");
        std::process::exit(1);
    };
    let inputs: Vec<_> = arguments.map(InputSpec::parse).collect();
    match run(&query, &inputs, LIMITS) {
        Ok(output) => {
            if let Err(error) = io::stdout().lock().write_all(&output) {
                eprintln!("writing JSON: {error}");
                std::process::exit(1);
            }
        }
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(1);
        }
    }
}

fn run(query: &str, inputs: &[InputSpec], limits: ProgramLimits) -> Result<Vec<u8>, CsvError> {
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
    let mut host = CsvHost {
        tables,
        inputs,
        let_names: Vec::new(),
    };
    let mut types = BTreeMap::new();
    for (index, input) in inputs.iter().enumerate() {
        if let Some(name) = &input.name {
            budget
                .charge(name.len() + 1, Location::Query(0..query.len()))
                .map_err(query_error)?;
            types.insert(
                name.clone(),
                host.tables[index]
                    .types(&mut budget, Location::Query(0..query.len()))
                    .map_err(query_error)?,
            );
        }
    }
    budget
        .charge(5, Location::Query(0..query.len()))
        .map_err(query_error)?;
    let rows_types = if host.tables.len() == 1 {
        host.tables[0]
            .types(&mut budget, Location::Query(0..query.len()))
            .map_err(query_error)?
    } else {
        vec![ValueType::Unestablished]
    };
    types.insert("rows".into(), rows_types);
    let expression =
        read_program(query, &mut host, &mut budget, limits.nesting).map_err(query_error)?;
    let result = evaluate_query_value(&expression, &types, &mut host, &mut budget);
    dispose_native_value(expression);
    let result = result.map_err(query_error)?;
    let mut output = Vec::new();
    let location = Location::Query(0..query.len());
    write_json(&result, &mut output, &mut budget, &location).map_err(query_error)?;
    append(&mut output, b"\n", &mut budget, &location).map_err(query_error)?;
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
    let mut bytes = name.bytes();
    matches!(bytes.next(), Some(b'a'..=b'z' | b'A'..=b'Z' | b'_'))
        && bytes.all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
        && !matches!(
            name,
            "_" | "rows"
                | "input"
                | "true"
                | "false"
                | "null"
                | "absent"
                | "let"
                | "report"
                | "param"
                | "fn"
                | "for"
                | "in"
                | "if"
                | "else"
                | "then"
                | "end"
                | "match"
                | "case"
                | "where"
                | "bind"
                | "from"
                | "and"
                | "or"
                | "not"
                | "mod"
                | "div"
                | "default"
                | "loop"
                | "continue"
                | "break"
                | "return"
        )
}

#[derive(Debug)]
struct Table {
    headers: Vec<String>,
    rows: V,
}
impl Table {
    fn types(&self, budget: &mut Budget, location: Location) -> Result<Vec<ValueType>, QueryError> {
        budget.charge(self.headers.len() + 3, location.clone())?;
        let mut fields = Vec::new();
        let mut declaration = Vec::new();
        for header in &self.headers {
            budget.charge(header.len() * 2 + 2, location.clone())?;
            fields.push((header.clone(), ValueType::String));
            declaration.push((header.clone(), DeclaredType::String));
        }
        // A declared header schema rejects unknown columns even with no rows.
        let row = ValueType::Declared {
            actual: Box::new(ValueType::Record(fields)),
            declaration: DeclaredType::Record(declaration),
        };
        Ok(vec![ValueType::Array(ArrayType::Elements(vec![row]))])
    }
}

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
    let rows = declare_constructed(
        V::Array(rows),
        DeclaredType::Array(Box::new(DeclaredType::Record(declaration))),
        budget,
        header_location,
    )?;
    Ok(Table { headers, rows })
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

struct CsvHost<'a> {
    tables: Vec<Table>,
    inputs: &'a [InputSpec],
    let_names: Vec<String>,
}
impl CsvHost<'_> {
    fn implicit(&self, range: &Range<usize>) -> Result<&Table, QueryError> {
        if self.tables.len() == 1 {
            return Ok(&self.tables[0]);
        }
        Err(QueryError::query(
            range.clone(),
            format!(
                "implicit CSV input is ambiguous; choose a named table: {}",
                self.inputs
                    .iter()
                    .filter_map(|input| input.name.as_deref())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        ))
    }
}
impl ValueReaderHost for CsvHost<'_> {
    type Stage = ();
    type SetOperator = ();
    type Emitter = ();
    type Predicate = ();
    fn field_pipeline_source(&self) -> zirql::FieldPipelineSource {
        zirql::FieldPipelineSource::Constructed
    }
    fn is_binding(&self, name: &str) -> bool {
        name == "rows"
            || self
                .inputs
                .iter()
                .any(|input| input.name.as_deref() == Some(name))
            || self.let_names.iter().any(|binding| binding == name)
    }
    fn is_constructed_binding(&self, name: &str) -> bool {
        self.is_binding(name)
    }

    fn is_stage(&self, _: &str) -> bool {
        false
    }
    fn parse_stage(
        &mut self,
        reader: &mut ArgumentReader<'_>,
        _: &mut Budget,
    ) -> Result<ExpressionStep<(), ()>, QueryError> {
        Err(reader
            .error("unknown CSV table or unavailable native stage; use a named table or rows"))
    }
    fn parse_predicate_leaf(
        &mut self,
        reader: &mut ArgumentReader<'_>,
        _: &mut Budget,
    ) -> Result<(), QueryError> {
        Err(reader.error("use a scoped row expression in filter"))
    }
    fn filter(&mut self, _: Predicate<()>, _: Range<usize>) {}
    fn projection(&mut self, _: Projection<ReadExpression<Self>>, _: Range<usize>) {}
    fn set_operator(&self, _: &str) -> Option<()> {
        None
    }
}
impl ValueQueryHost<ReadExpression<Self>> for CsvHost<'_> {
    type Error = QueryError;
    fn current_input_types(
        &mut self,
        range: &Range<usize>,
        budget: &mut Budget,
    ) -> Result<Vec<ValueType>, QueryError> {
        self.implicit(range)?
            .types(budget, Location::Query(range.clone()))
    }
    fn read_current_input(
        &mut self,
        range: &Range<usize>,
        budget: &mut Budget,
    ) -> Result<V, QueryError> {
        copy_constructed_value(
            &self.implicit(range)?.rows,
            budget,
            Location::Query(range.clone()),
        )
    }
    fn query_types(
        &mut self,
        _: &ReadExpression<Self>,
        _: &[(&str, Vec<ValueType>)],
        _: &mut Budget,
        range: &Range<usize>,
    ) -> Result<Vec<ValueType>, QueryError> {
        Err(QueryError::query(
            range.clone(),
            "native queries are unavailable in the CSV host",
        ))
    }
    fn evaluate_query(
        &mut self,
        query: &ReadExpression<Self>,
        _: &[(&str, V)],
        _: &mut Budget,
    ) -> Result<V, QueryError> {
        Err(QueryError::query(
            query.range.clone(),
            "native queries are unavailable in the CSV host",
        ))
    }
    fn read_binding(
        &mut self,
        name: &str,
        range: &Range<usize>,
        budget: &mut Budget,
    ) -> Result<V, QueryError> {
        let table = if name == "rows" {
            self.implicit(range)?
        } else {
            self.inputs
                .iter()
                .position(|input| input.name.as_deref() == Some(name))
                .map(|index| &self.tables[index])
                .ok_or_else(|| {
                    QueryError::query(range.clone(), format!("unknown CSV table {name:?}"))
                })?
        };
        copy_constructed_value(&table.rows, budget, Location::Query(range.clone()))
    }
    fn validate_name(
        &mut self,
        name: &str,
        range: &Range<usize>,
        _: &mut Budget,
    ) -> Result<(), QueryError> {
        if name == "rows" {
            self.implicit(range)?;
        }
        Ok(())
    }
}

fn read_program(
    query: &str,
    host: &mut CsvHost<'_>,
    budget: &mut Budget,
    nesting: usize,
) -> Result<ValueExpression<zirql::Expression<(), (), ()>>, QueryError> {
    let mut reader = ArgumentReader::new(query, 0)?;
    let mut bindings = Vec::new();
    let result = (|| {
        while let Some(pattern) = read_pattern_let(&mut reader, budget, nesting)? {
            let value = match read_value_expression(&mut reader, host, budget, nesting) {
                Ok(value) => value,
                Err(error) => {
                    dispose_pattern(pattern);
                    return Err(error);
                }
            };
            let names = match pattern_names(&pattern, budget) {
                Ok(names) => names,
                Err(error) => {
                    dispose_pattern(pattern);
                    dispose_native_value(value);
                    return Err(error);
                }
            };
            for name in names {
                if let Err(error) =
                    budget.charge(name.value.len() + 2, Location::Query(name.range.clone()))
                {
                    dispose_pattern(pattern);
                    dispose_native_value(value);
                    return Err(error);
                }
                host.let_names.push(name.value.clone());
            }
            bindings.push((pattern, value));
            if !reader.consume(';') {
                return Err(reader.error("expected ';' after a let binding"));
            }
        }
        let value = read_value_expression(&mut reader, host, budget, nesting)?;
        reader.skip_whitespace();
        if !reader.remaining().is_empty() {
            dispose_native_value(value);
            return Err(reader.error("expected the end of the expression"));
        }
        Ok(value)
    })();
    match result {
        Ok(value) if bindings.is_empty() => Ok(value),
        Ok(value) => Ok(Located::new(
            K::Block {
                bindings,
                value: Box::new(value),
            },
            0..query.len(),
        )),
        Err(error) => {
            for (pattern, value) in bindings {
                dispose_pattern(pattern);
                dispose_native_value(value);
            }
            Err(error)
        }
    }
}

fn append(
    output: &mut Vec<u8>,
    bytes: &[u8],
    budget: &mut Budget,
    location: &Location,
) -> Result<(), QueryError> {
    budget.charge(bytes.len(), location.clone())?;
    output.extend_from_slice(bytes);
    Ok(())
}
struct JsonBuffer<'a> {
    bytes: &'a mut Vec<u8>,
    budget: &'a mut Budget,
    location: &'a Location,
    error: Option<QueryError>,
}
impl Write for JsonBuffer<'_> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if let Err(error) = append(self.bytes, bytes, self.budget, self.location) {
            self.error = Some(error);
            return Err(io::Error::other("JSON output allowance exhausted"));
        }
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
fn write_json(
    value: &V,
    output: &mut Vec<u8>,
    budget: &mut Budget,
    location: &Location,
) -> Result<(), QueryError> {
    budget.within_evaluation(1, location.clone(), |budget| {
        budget.charge(1, location.clone())?;
        budget.charge_results(1, location.clone())?;
        match value.data() {
            V::Array(values)
            | V::Tuple(values)
            | V::Set {
                elements: values, ..
            } => {
                append(output, b"[", budget, location)?;
                for (index, value) in values.iter().enumerate() {
                    if index != 0 {
                        append(output, b",", budget, location)?;
                    }
                    write_json(value, output, budget, location)?;
                }
                append(output, b"]", budget, location)
            }
            V::Record(fields) => {
                append(output, b"{", budget, location)?;
                for (index, (label, value)) in fields.iter().enumerate() {
                    if index != 0 {
                        append(output, b",", budget, location)?;
                    }
                    write_string(label, output, budget, location)?;
                    append(output, b":", budget, location)?;
                    write_json(value, output, budget, location)?;
                }
                append(output, b"}", budget, location)
            }
            V::Member(_) | V::KeyedArray { .. } => Err(QueryError::query(
                location_range(location),
                "CSV JSON output cannot represent nominal members or keyed-array shape",
            )),
            scalar => {
                let mut writer = JsonBuffer {
                    bytes: output,
                    budget,
                    location,
                    error: None,
                };
                let result = match scalar {
                    V::Absent => serde_json::to_writer(&mut writer, &()),
                    V::Boolean(value) => serde_json::to_writer(&mut writer, value),
                    V::Integer(value) => serde_json::to_writer(&mut writer, value),
                    V::Float(value) => serde_json::to_writer(&mut writer, value),
                    V::String(value) => serde_json::to_writer(&mut writer, value),
                    _ => unreachable!("containers and declared wrappers are handled above"),
                };
                result.map_err(|error| {
                    writer.error.unwrap_or_else(|| {
                        QueryError::query(
                            location_range(location),
                            format!("writing JSON: {error}"),
                        )
                    })
                })
            }
        }
    })
}
fn location_range(location: &Location) -> Range<usize> {
    match location {
        Location::Query(range) | Location::Source(range) => range.clone(),
    }
}
fn write_string(
    value: &str,
    output: &mut Vec<u8>,
    budget: &mut Budget,
    location: &Location,
) -> Result<(), QueryError> {
    let mut writer = JsonBuffer {
        bytes: output,
        budget,
        location,
        error: None,
    };
    serde_json::to_writer(&mut writer, value).map_err(|error| {
        writer.error.unwrap_or_else(|| {
            QueryError::query(
                location_range(location),
                format!("writing JSON string: {error}"),
            )
        })
    })
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
        String::from_utf8(run(query, inputs, LIMITS).unwrap()).unwrap()
    }
    fn refusal(query: &str, inputs: &[InputSpec], message: &str) -> CsvError {
        let error = run(query, inputs, LIMITS).unwrap_err();
        assert!(error.to_string().contains(message), "{error}");
        assert!(error.context.contains(&format!("{query:?}")));
        assert!(matches!(error.error.location, Location::Query(_)));
        error
    }

    #[test]
    fn fixture_queries_keep_decoded_strings_saved_values_and_written_order() {
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
        refusal("fn f(x: int) -> int { x } f(1)", &people(), "expected");
        let error = refusal("rows | unique", &people(), "unique");
        assert_eq!(error.error.location, Location::Query(7..13));
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
        let V::Array(rows) = table.rows.data() else {
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
        let inputs = people();
        let mut host = CsvHost {
            tables: vec![empty],
            inputs: &inputs,
            let_names: Vec::new(),
        };
        let types = BTreeMap::from([(
            "rows".into(),
            host.tables[0]
                .types(&mut budget, Location::Query(0..1))
                .unwrap(),
        )]);
        let expression = read_program(
            "rows | filter(_.missing == \"yes\")",
            &mut host,
            &mut budget,
            LIMITS.nesting,
        )
        .unwrap();
        let error = evaluate_query_value(&expression, &types, &mut host, &mut budget).unwrap_err();
        dispose_native_value(expression);
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
        let mut budget = Budget::for_program(LIMITS);
        assert!(admit_inputs(&[InputSpec::parse("rows=no.csv".into())], &mut budget).is_err());
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
    fn output_is_metered_and_cannot_flatten_keyed_shape() {
        let mut budget = Budget::for_program(ProgramLimits { work: 5, ..LIMITS });
        let mut bytes = Vec::new();
        let error = write_json(
            &V::String("needs escaping\n".into()),
            &mut bytes,
            &mut budget,
            &Location::Query(2..3),
        )
        .unwrap_err();
        assert_eq!(error.category, QueryErrorCategory::WorkLimit);
        assert_eq!(error.location, Location::Query(2..3));
        // run returns its buffer only after evaluation, rendering and newline succeed.
        let mut budget = Budget::for_program(LIMITS);
        let error = write_json(
            &V::KeyedArray {
                axes: vec![zirql::KeyAxis {
                    key_type: zirql::ValueKeyType::Integer,
                    keys: vec![],
                }],
                entries: vec![],
            },
            &mut bytes,
            &mut budget,
            &Location::Query(0..1),
        )
        .unwrap_err();
        assert!(error.message.contains("keyed-array shape"));
    }
}
