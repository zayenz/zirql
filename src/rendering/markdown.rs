use std::collections::HashSet;

use super::{
    JsonLayout, JsonRepresentation, OutputBuffer, failure, render_json, render_json_value,
};
use crate::value::allocation;
use crate::{Budget, ConstructedValue as V, Location, QueryError, constructed_value_type};

/// Render constructed data as a complete UTF-8 Markdown buffer.
///
/// Scalars use a paragraph; ordered collections use a Value table. Records and
/// dictionaries use Key/Value tables, or keyed rows when every value is a record.
/// Collections of records use the first-seen union of fields as columns. Missing
/// and absent cells are blank. Nested cells and structural keys use compact
/// ordinary JSON, including tagged nominal members and explicit keyed arrays.
/// Empty collections and maps render as `_No entries._`.
///
/// ```
/// use zirql::{Budget, ConstructedValue as V, Location, ProgramLimits, render_markdown};
/// let mut budget = Budget::for_program(ProgramLimits {
///     work: 10_000, results: 10_000, nesting: 64, stream_items: 100,
/// });
/// let rows = V::Array(vec![V::Record(vec![("count".into(), V::Integer(3))])]);
/// let bytes = render_markdown(&rows, &mut budget, Location::Query(0..5))?;
/// assert_eq!(bytes, b"\n| count |\n| --- |\n| 3 |\n\n");
/// # Ok::<(), zirql::QueryError>(())
/// ```
///
/// Rendering is an explicit boundary after evaluation. Hosts project native
/// leaves before this call. Computational queues and identity sets are outside
/// [`crate::ConstructedValue`] and cannot be rendered through this API.
///
/// # Errors
/// Checks public values, table expansion, scalar conversion and escaped output
/// under the supplied request budget and location. Known allocations are checked
/// before growth. A failure spends its budget but returns no partial output.
pub fn render_markdown(
    value: &crate::ConstructedValue,
    budget: &mut Budget,
    location: Location,
) -> Result<Vec<u8>, QueryError> {
    constructed_value_type(value, budget, location.clone())?;
    let mut markdown = Markdown {
        output: OutputBuffer {
            bytes: Vec::new(),
            budget,
            location: &location,
            error: None,
        },
    };
    markdown.render(Data::Constructed(value.data()))?;
    Ok(markdown.output.bytes)
}

/// Render a mixed report through the same native projection boundary as JSON.
///
/// Constructed nodes use Markdown data layouts regardless of their host's JSON
/// representation. Native descriptors remain ordinary JSON data. Hosts retain
/// ownership checks and computational-value refusals in [`super::RenderHost`].
///
/// ```
/// use zirql::{Budget, ConstructedValue, JsonRepresentation, Location, ProgramLimits,
///             RenderContext, RenderHost, RenderView, render_report_markdown};
/// struct Host;
/// impl RenderHost for Host {
///     type Value = ConstructedValue;
///     type Error = std::convert::Infallible;
///     fn view<'a>(&mut self, value: &'a ConstructedValue, _: RenderContext,
///         _: &mut Budget, _: &Location) -> Result<RenderView<'a, ConstructedValue>, Self::Error> {
///         Ok(RenderView::Constructed(value, JsonRepresentation::Tagged))
///     }
/// }
/// let mut budget = Budget::for_program(ProgramLimits {
///     work: 10_000, results: 10_000, nesting: 64, stream_items: 100,
/// });
/// let bytes = render_report_markdown(&mut Host, &ConstructedValue::Integer(3),
///     &mut budget, Location::Query(0..5))?;
/// assert_eq!(bytes, b"\n3\n\n");
/// # Ok::<(), zirql::EvaluationError<std::convert::Infallible>>(())
/// ```
///
/// # Errors
/// Returns located budget/data errors or host projection errors. The request's
/// budget covers adaptation, table expansion and output; no partial bytes escape.
pub fn render_report_markdown<H: super::RenderHost>(
    host: &mut H,
    value: &H::Value,
    budget: &mut Budget,
    location: Location,
) -> Result<Vec<u8>, crate::EvaluationError<H::Error>> {
    let report = report(host, value, super::RenderContext::Value, budget, &location)?;
    let mut markdown = Markdown {
        output: OutputBuffer {
            bytes: Vec::new(),
            budget,
            location: &location,
            error: None,
        },
    };
    markdown
        .render(Data::Report(&report))
        .map_err(crate::EvaluationError::Core)?;
    Ok(markdown.output.bytes)
}

// Borrow constructed data; own only the native projections and container slots.
// In particular, tagged JSON never determines a constructed node's table shape.
enum Report<'a> {
    Constructed(&'a V),
    Projected(serde_json::Value),
    Sequence(Vec<Report<'a>>),
    Record(Vec<(&'a str, Report<'a>)>),
}

fn report<'a, H: super::RenderHost>(
    host: &mut H,
    value: &'a H::Value,
    context: super::RenderContext,
    budget: &mut Budget,
    location: &Location,
) -> Result<Report<'a>, crate::EvaluationError<H::Error>> {
    use super::RenderView;
    use crate::EvaluationError::{Core, Host};
    budget.within_evaluation(1, location.clone(), |budget| {
        budget.charge(1, location.clone()).map_err(Core)?;
        Ok(
            match host.view(value, context, budget, location).map_err(Host)? {
                RenderView::Value(value) => return report(host, value, context, budget, location),
                RenderView::Constructed(value, _) => {
                    constructed_value_type(value, budget, location.clone()).map_err(Core)?;
                    Report::Constructed(value)
                }
                RenderView::Projected(value) => {
                    super::visit_json(&value, budget, location).map_err(Core)?;
                    Report::Projected(value)
                }
                RenderView::Sequence(values) => {
                    slots::<Report<'_>>(values.len(), budget, location).map_err(Core)?;
                    let mut output = Vec::with_capacity(values.len());
                    for value in values {
                        output.push(report(
                            host,
                            value,
                            super::RenderContext::Value,
                            budget,
                            location,
                        )?);
                    }
                    Report::Sequence(output)
                }
                RenderView::Record(fields) => {
                    report_record(host, fields, super::RenderContext::Value, budget, location)?
                }
                RenderView::Capture(capture) => report_record(
                    host,
                    &capture.fields,
                    super::RenderContext::CaptureField,
                    budget,
                    location,
                )?,
                RenderView::Captures(captures) => {
                    slots::<Report<'_>>(captures.len(), budget, location).map_err(Core)?;
                    let mut output = Vec::with_capacity(captures.len());
                    for capture in captures {
                        output.push(budget.within_evaluation(1, location.clone(), |budget| {
                            report_record(
                                host,
                                &capture.fields,
                                super::RenderContext::CaptureField,
                                budget,
                                location,
                            )
                        })?);
                    }
                    Report::Sequence(output)
                }
            },
        )
    })
}

fn slots<T>(length: usize, budget: &mut Budget, location: &Location) -> Result<(), QueryError> {
    allocation::<T>(length, location)?;
    budget.charge(length.saturating_add(1), location.clone())?;
    budget.charge_results(length.saturating_add(1), location.clone())
}

fn report_record<'a, H: super::RenderHost>(
    host: &mut H,
    fields: &'a [(String, H::Value)],
    context: super::RenderContext,
    budget: &mut Budget,
    location: &Location,
) -> Result<Report<'a>, crate::EvaluationError<H::Error>> {
    slots::<(&str, Report<'_>)>(fields.len(), budget, location)
        .map_err(crate::EvaluationError::Core)?;
    let mut output = Vec::with_capacity(fields.len());
    for (label, value) in fields {
        output.push((
            label.as_str(),
            report(host, value, context, budget, location)?,
        ));
    }
    Ok(Report::Record(output))
}

#[derive(Clone, Copy)]
enum Data<'a> {
    Constructed(&'a V),
    Json(&'a serde_json::Value),
    Report(&'a Report<'a>),
}
impl<'a> Data<'a> {
    fn data(self) -> Self {
        match self {
            Self::Constructed(value) => Self::Constructed(value.data()),
            Self::Report(Report::Constructed(value)) => Self::Constructed(value.data()),
            Self::Report(Report::Projected(value)) => Self::Json(value),
            value => value,
        }
    }
    fn text(self) -> Option<&'a str> {
        match self.data() {
            Self::Constructed(V::String(text)) => Some(text),
            Self::Json(serde_json::Value::String(text)) => Some(text),
            _ => None,
        }
    }
    fn is_absent(self) -> bool {
        matches!(
            self.data(),
            Self::Constructed(V::Absent) | Self::Json(serde_json::Value::Null)
        )
    }
    fn sequence(self) -> Option<Sequence<'a>> {
        match self.data() {
            Self::Constructed(V::Array(values) | V::Tuple(values)) => {
                Some(Sequence::Constructed(values))
            }
            Self::Constructed(V::Set { elements, .. }) => Some(Sequence::Constructed(elements)),
            Self::Json(serde_json::Value::Array(values)) => Some(Sequence::Json(values)),
            Self::Report(Report::Sequence(values)) => Some(Sequence::Report(values)),
            _ => None,
        }
    }
    fn entries(self) -> Option<Entries<'a>> {
        match self.data() {
            Self::Constructed(V::Record(fields)) => Some(Entries::Record(fields)),
            Self::Constructed(V::Dictionary(entries)) => Some(Entries::Dictionary(entries)),
            Self::Json(serde_json::Value::Object(fields)) => Some(Entries::Json(fields)),
            Self::Report(Report::Record(fields)) => Some(Entries::Report(fields)),
            _ => None,
        }
    }
    fn json(
        self,
        budget: &mut Budget,
        location: &Location,
    ) -> Result<serde_json::Value, QueryError> {
        use serde_json::Value;
        budget.within_evaluation(1, location.clone(), |budget| match self.data() {
            Self::Constructed(value) => super::to_json_value(
                value,
                JsonRepresentation::Ordinary,
                budget,
                location.clone(),
            ),
            Self::Json(value) => copy_json(value, budget, location),
            Self::Report(Report::Sequence(values)) => {
                let mut output = super::JsonConversion { budget, location }.array(values.len())?;
                for value in values {
                    output.push(Self::Report(value).json(budget, location)?);
                }
                Ok(Value::Array(output))
            }
            Self::Report(Report::Record(fields)) => {
                let mut output = super::JsonConversion { budget, location }.object(fields.len())?;
                for (label, value) in fields {
                    let value = Self::Report(value).json(budget, location)?;
                    super::JsonConversion { budget, location }.insert(&mut output, label, value)?;
                }
                Ok(Value::Object(output))
            }
            _ => unreachable!("transparent report nodes were unwrapped"),
        })
    }
}

fn copy_json(
    value: &serde_json::Value,
    budget: &mut Budget,
    location: &Location,
) -> Result<serde_json::Value, QueryError> {
    use serde_json::Value;
    budget.within_evaluation(1, location.clone(), |budget| match value {
        Value::Array(values) => {
            let mut output = super::JsonConversion { budget, location }.array(values.len())?;
            for value in values {
                output.push(copy_json(value, budget, location)?);
            }
            Ok(Value::Array(output))
        }
        Value::Object(fields) => {
            let mut output = super::JsonConversion { budget, location }.object(fields.len())?;
            for (label, value) in fields {
                let value = copy_json(value, budget, location)?;
                super::JsonConversion { budget, location }.insert(&mut output, label, value)?;
            }
            Ok(Value::Object(output))
        }
        Value::String(text) => super::JsonConversion { budget, location }.string(text),
        value => {
            super::JsonConversion { budget, location }.admit(1)?;
            Ok(value.clone())
        }
    })
}

#[derive(Clone, Copy)]
enum Sequence<'a> {
    Constructed(&'a [V]),
    Json(&'a [serde_json::Value]),
    Report(&'a [Report<'a>]),
}
impl<'a> Sequence<'a> {
    fn len(self) -> usize {
        match self {
            Self::Constructed(values) => values.len(),
            Self::Json(values) => values.len(),
            Self::Report(values) => values.len(),
        }
    }
    fn item(self, index: usize) -> Data<'a> {
        match self {
            Self::Constructed(values) => Data::Constructed(&values[index]),
            Self::Json(values) => Data::Json(&values[index]),
            Self::Report(values) => Data::Report(&values[index]),
        }
    }
}

#[derive(Clone, Copy)]
enum Entries<'a> {
    Record(&'a [(String, V)]),
    Dictionary(&'a [(V, V)]),
    Json(&'a serde_json::Map<String, serde_json::Value>),
    Report(&'a [(&'a str, Report<'a>)]),
}
impl<'a> Entries<'a> {
    fn len(self) -> usize {
        match self {
            Self::Record(fields) => fields.len(),
            Self::Dictionary(entries) => entries.len(),
            Self::Json(fields) => fields.len(),
            Self::Report(fields) => fields.len(),
        }
    }
    fn iter(self) -> EntryIter<'a> {
        match self {
            Self::Record(fields) => EntryIter::Record(fields.iter()),
            Self::Dictionary(entries) => EntryIter::Dictionary(entries.iter()),
            Self::Json(fields) => EntryIter::Json(fields.iter()),
            Self::Report(fields) => EntryIter::Report(fields.iter()),
        }
    }
    fn fields(self) -> impl Iterator<Item = (&'a str, Data<'a>)> {
        self.iter().map(|(key, value)| {
            let Key::Label(label) = key else {
                unreachable!("record field")
            };
            (label, value)
        })
    }
    fn is_record(self) -> bool {
        !matches!(self, Self::Dictionary(_))
    }
}

enum EntryIter<'a> {
    Record(std::slice::Iter<'a, (String, V)>),
    Dictionary(std::slice::Iter<'a, (V, V)>),
    Json(serde_json::map::Iter<'a>),
    Report(std::slice::Iter<'a, (&'a str, Report<'a>)>),
}
impl<'a> Iterator for EntryIter<'a> {
    type Item = (Key<'a>, Data<'a>);
    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Record(fields) => fields
                .next()
                .map(|(key, value)| (Key::Label(key), Data::Constructed(value))),
            Self::Dictionary(entries) => entries
                .next()
                .map(|(key, value)| (Key::Value(Data::Constructed(key)), Data::Constructed(value))),
            Self::Json(fields) => fields
                .next()
                .map(|(key, value)| (Key::Label(key), Data::Json(value))),
            Self::Report(fields) => fields
                .next()
                .map(|(key, value)| (Key::Label(key), Data::Report(value))),
        }
    }
}

enum Key<'a> {
    Label(&'a str),
    Value(Data<'a>),
}
enum Rows<'a> {
    Sequence(Sequence<'a>),
    Keyed(Entries<'a>),
}
impl<'a> Rows<'a> {
    fn len(&self) -> usize {
        match self {
            Self::Sequence(values) => values.len(),
            Self::Keyed(entries) => entries.len(),
        }
    }
    fn iter(&self) -> RowIter<'a> {
        match self {
            Self::Sequence(values) => RowIter::Sequence(*values, 0),
            Self::Keyed(entries) => RowIter::Keyed(entries.iter()),
        }
    }
}
enum RowIter<'a> {
    Sequence(Sequence<'a>, usize),
    Keyed(EntryIter<'a>),
}
impl<'a> Iterator for RowIter<'a> {
    type Item = (Option<Key<'a>>, Entries<'a>);
    fn next(&mut self) -> Option<Self::Item> {
        let (key, value) = match self {
            Self::Sequence(values, index) => {
                if *index == values.len() {
                    return None;
                }
                let value = values.item(*index);
                *index += 1;
                (None, value)
            }
            Self::Keyed(entries) => {
                let (key, value) = entries.next()?;
                (Some(key), value)
            }
        };
        Some((
            key,
            value
                .entries()
                .expect("record rows were checked before table rendering"),
        ))
    }
}

struct Markdown<'a> {
    output: OutputBuffer<'a>,
}

impl Markdown<'_> {
    fn render(&mut self, value: Data<'_>) -> Result<(), QueryError> {
        if let Some(entries) = value.entries() {
            return self.map(entries);
        }
        if let Some(values) = value.sequence() {
            return self.sequence(values);
        }
        self.push("\n")?;
        if let Some(text) = value.text() {
            self.paragraph(text)?;
        } else {
            self.cell(value)?;
        }
        self.push("\n\n")
    }

    fn sequence(&mut self, values: Sequence<'_>) -> Result<(), QueryError> {
        if values.len() == 0 {
            return self.empty();
        }
        self.work(values.len())?;
        if (0..values.len())
            .all(|index| values.item(index).entries().is_some_and(Entries::is_record))
        {
            return self.records(Rows::Sequence(values));
        }
        self.values(values)
    }

    fn values(&mut self, values: Sequence<'_>) -> Result<(), QueryError> {
        self.admit_table(values.len(), 1)?;
        self.header(["Value"])?;
        for index in 0..values.len() {
            self.push("| ")?;
            self.cell(values.item(index))?;
            self.push(" |\n")?;
        }
        self.push("\n")
    }

    fn map(&mut self, entries: Entries<'_>) -> Result<(), QueryError> {
        if entries.len() == 0 {
            return self.empty();
        }
        self.work(entries.len())?;
        if entries
            .iter()
            .all(|(_, value)| value.entries().is_some_and(Entries::is_record))
        {
            return self.records(Rows::Keyed(entries));
        }
        self.admit_table(entries.len(), 2)?;
        self.header(["Key", "Value"])?;
        for (key, value) in entries.iter() {
            self.push("| ")?;
            self.key(key)?;
            self.push(" | ")?;
            self.cell(value)?;
            self.push(" |\n")?;
        }
        self.push("\n")
    }

    fn records(&mut self, rows: Rows<'_>) -> Result<(), QueryError> {
        let keyed = matches!(rows, Rows::Keyed(_));
        let key_columns = usize::from(keyed);
        let mut columns = Vec::new();
        let mut seen = HashSet::new();
        for (_, fields) in rows.iter() {
            for (label, _) in fields.fields() {
                self.work(label.len().saturating_add(1))?;
                if !seen.contains(label) {
                    let count = columns.len() + 1;
                    self.table_cells(rows.len(), count + key_columns)?;
                    allocation::<&str>(count, self.output.location)?;
                    self.admit(2)?;
                    seen.insert(label);
                    columns.push(label);
                }
            }
        }
        // A sequence of empty records still has rows to display.
        if !keyed && columns.is_empty() {
            let Rows::Sequence(values) = rows else {
                unreachable!()
            };
            return self.values(values);
        }
        self.admit_table(rows.len(), columns.len() + key_columns)?;
        self.header(
            keyed
                .then_some("Key")
                .into_iter()
                .chain(columns.iter().copied()),
        )?;
        for (key, fields) in rows.iter() {
            self.push("|")?;
            if let Some(key) = key {
                self.push(" ")?;
                self.key(key)?;
                self.push(" |")?;
            }
            for column in &columns {
                self.push(" ")?;
                for (label, value) in fields.fields() {
                    self.work(label.len().min(column.len()) + 1)?;
                    if label == *column {
                        self.cell(value)?;
                        break;
                    }
                }
                self.push(" |")?;
            }
            self.push("\n")?;
        }
        self.push("\n")
    }

    fn key(&mut self, key: Key<'_>) -> Result<(), QueryError> {
        match key {
            Key::Label(label) => self.escape(label),
            Key::Value(value) => self.cell(value),
        }
    }

    fn cell(&mut self, value: Data<'_>) -> Result<(), QueryError> {
        if value.is_absent() {
            return Ok(());
        }
        if let Some(text) = value.text() {
            return self.escape(text);
        }
        let bytes = match value.data() {
            Data::Constructed(value) => render_json(
                value,
                JsonRepresentation::Ordinary,
                JsonLayout::Compact,
                self.output.budget,
                self.output.location.clone(),
            )?,
            Data::Json(value) => render_json_value(
                value,
                JsonLayout::Compact,
                self.output.budget,
                self.output.location.clone(),
            )?,
            value => {
                let json = value.json(self.output.budget, self.output.location)?;
                render_json_value(
                    &json,
                    JsonLayout::Compact,
                    self.output.budget,
                    self.output.location.clone(),
                )?
            }
        };
        self.escape(std::str::from_utf8(&bytes).expect("JSON output is UTF-8"))
    }

    fn header<'a>(&mut self, columns: impl IntoIterator<Item = &'a str>) -> Result<(), QueryError> {
        self.push("\n| ")?;
        let mut count = 0;
        for column in columns {
            if count > 0 {
                self.push(" | ")?;
            }
            self.escape(column)?;
            count += 1;
        }
        self.push(" |\n| ")?;
        for index in 0..count {
            if index > 0 {
                self.push(" | ")?;
            }
            self.push("---")?;
        }
        self.push(" |\n")
    }

    fn table_cells(&self, rows: usize, columns: usize) -> Result<usize, QueryError> {
        let cells = rows
            .checked_add(1)
            .and_then(|rows| rows.checked_mul(columns))
            .ok_or_else(|| failure(self.output.location, "Markdown table size overflow"))?;
        self.output
            .budget
            .check_items(cells, self.output.location.clone())?;
        Ok(cells)
    }

    fn admit_table(&mut self, rows: usize, columns: usize) -> Result<(), QueryError> {
        let cells = self.table_cells(rows, columns)?;
        self.admit(cells)
    }

    fn work(&mut self, units: usize) -> Result<(), QueryError> {
        self.output
            .budget
            .charge(units, self.output.location.clone())
    }

    fn admit(&mut self, units: usize) -> Result<(), QueryError> {
        self.work(units)?;
        self.output
            .budget
            .charge_results(units, self.output.location.clone())
    }

    fn empty(&mut self) -> Result<(), QueryError> {
        self.push("\n_No entries._\n\n")
    }

    fn push(&mut self, text: &str) -> Result<(), QueryError> {
        self.output.append(text.as_bytes())
    }

    fn paragraph(&mut self, text: &str) -> Result<(), QueryError> {
        self.work(text.len())?;
        let content = text.trim_start_matches([' ', '\t']);
        for ch in text[..text.len() - content.len()].chars() {
            self.push(if ch == ' ' { "&#32;" } else { "&#9;" })?;
        }
        let mut start = 0;
        for (index, ch) in content.char_indices() {
            if matches!(ch, '#' | '+' | '-' | '.' | ')' | '=') {
                self.escape(&content[start..index])?;
                self.push("\\")?;
                self.push(ch.encode_utf8(&mut [0; 4]))?;
                start = index + ch.len_utf8();
            }
        }
        self.escape(&content[start..])
    }

    fn escape(&mut self, text: &str) -> Result<(), QueryError> {
        let mut chars = text.chars().peekable();
        while let Some(ch) = chars.next() {
            match ch {
                '&' => self.push("&amp;")?,
                '<' => self.push("&lt;")?,
                '>' => self.push("&gt;")?,
                '\r' => {
                    if chars.peek() == Some(&'\n') {
                        chars.next();
                    }
                    self.push("<br>")?;
                }
                '\n' => self.push("<br>")?,
                '\\' | '|' | '`' | '*' | '_' | '[' | ']' | '~' => {
                    self.push("\\")?;
                    self.push(ch.encode_utf8(&mut [0; 4]))?;
                }
                _ => self.push(ch.encode_utf8(&mut [0; 4]))?,
            }
        }
        Ok(())
    }
}
