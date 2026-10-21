use std::collections::HashSet;

use super::{JsonLayout, JsonRepresentation, OutputBuffer, failure, render_json};
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
    markdown.render(value.data())?;
    Ok(markdown.output.bytes)
}

#[derive(Clone, Copy)]
enum Entries<'a> {
    Record(&'a [(String, V)]),
    Dictionary(&'a [(V, V)]),
}

impl<'a> Entries<'a> {
    fn len(self) -> usize {
        match self {
            Self::Record(fields) => fields.len(),
            Self::Dictionary(entries) => entries.len(),
        }
    }

    fn entry(self, index: usize) -> (Key<'a>, &'a V) {
        match self {
            Self::Record(fields) => (Key::Label(&fields[index].0), &fields[index].1),
            Self::Dictionary(entries) => (Key::Value(&entries[index].0), &entries[index].1),
        }
    }
}

enum Key<'a> {
    Label(&'a str),
    Value(&'a V),
}

enum Rows<'a> {
    Sequence(&'a [V]),
    Keyed(Entries<'a>),
}

impl<'a> Rows<'a> {
    fn len(&self) -> usize {
        match self {
            Self::Sequence(values) => values.len(),
            Self::Keyed(entries) => entries.len(),
        }
    }

    fn row(&self, index: usize) -> (Option<Key<'a>>, &'a [(String, V)]) {
        let (key, value) = match self {
            Self::Sequence(values) => (None, &values[index]),
            Self::Keyed(entries) => {
                let (key, value) = entries.entry(index);
                (Some(key), value)
            }
        };
        let V::Record(fields) = value.data() else {
            unreachable!("record rows were checked before table rendering")
        };
        (key, fields)
    }
}

struct Markdown<'a> {
    output: OutputBuffer<'a>,
}

impl Markdown<'_> {
    fn render(&mut self, value: &V) -> Result<(), QueryError> {
        match value {
            V::Record(fields) => self.map(Entries::Record(fields)),
            V::Dictionary(entries) => self.map(Entries::Dictionary(entries)),
            V::Array(values) | V::Tuple(values) => self.sequence(values),
            V::Set { elements, .. } => self.sequence(elements),
            _ => {
                self.push("\n")?;
                if let V::String(text) = value {
                    self.paragraph(text)?;
                } else {
                    self.cell(value)?;
                }
                self.push("\n\n")
            }
        }
    }

    fn sequence(&mut self, values: &[V]) -> Result<(), QueryError> {
        if values.is_empty() {
            return self.empty();
        }
        self.work(values.len())?;
        if values
            .iter()
            .all(|value| matches!(value.data(), V::Record(_)))
        {
            return self.records(Rows::Sequence(values));
        }
        self.values(values)
    }

    fn values(&mut self, values: &[V]) -> Result<(), QueryError> {
        self.admit_table(values.len(), 1)?;
        self.header(["Value"])?;
        for value in values {
            self.push("| ")?;
            self.cell(value)?;
            self.push(" |\n")?;
        }
        self.push("\n")
    }

    fn map(&mut self, entries: Entries<'_>) -> Result<(), QueryError> {
        if entries.len() == 0 {
            return self.empty();
        }
        self.work(entries.len())?;
        if (0..entries.len()).all(|index| matches!(entries.entry(index).1.data(), V::Record(_))) {
            return self.records(Rows::Keyed(entries));
        }
        self.admit_table(entries.len(), 2)?;
        self.header(["Key", "Value"])?;
        for index in 0..entries.len() {
            let (key, value) = entries.entry(index);
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
        for index in 0..rows.len() {
            for (label, _) in rows.row(index).1 {
                self.work(label.len().saturating_add(1))?;
                if !seen.contains(label.as_str()) {
                    let count = columns.len() + 1;
                    self.table_cells(rows.len(), count + key_columns)?;
                    allocation::<&str>(count, self.output.location)?;
                    self.admit(2)?;
                    seen.insert(label.as_str());
                    columns.push(label.as_str());
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
        for index in 0..rows.len() {
            let (key, fields) = rows.row(index);
            self.push("|")?;
            if let Some(key) = key {
                self.push(" ")?;
                self.key(key)?;
                self.push(" |")?;
            }
            for column in &columns {
                self.push(" ")?;
                for (label, value) in fields {
                    self.work(label.len().min(column.len()) + 1)?;
                    if label == column {
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

    fn cell(&mut self, value: &V) -> Result<(), QueryError> {
        match value.data() {
            V::Absent => Ok(()),
            V::String(text) => self.escape(text),
            value => {
                let bytes = render_json(
                    value,
                    JsonRepresentation::Ordinary,
                    JsonLayout::Compact,
                    self.output.budget,
                    self.output.location.clone(),
                )?;
                self.escape(std::str::from_utf8(&bytes).expect("JSON output is UTF-8"))
            }
        }
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
