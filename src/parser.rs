use crate::{
    Located, MAX_NESTING, Pipeline, Predicate, PredicateKind, QueryError, Stage, check_depth,
};

/// The host reads its leaf name and arguments from the original query text.
/// Common helpers skip whitespace and comments, returning original UTF-8 byte ranges.
/// `source`, `remaining`, `position` and `advance` support host-specific forms.
pub struct ArgumentReader<'a> {
    source: &'a str,
    position: usize,
    end: usize,
}

impl<'a> ArgumentReader<'a> {
    /// Start at an original source byte boundary.
    pub fn new(source: &'a str, position: usize) -> Result<Self, QueryError> {
        if !source.is_char_boundary(position) {
            return Err(QueryError::query(
                position..position,
                "reader position must be an in-bounds UTF-8 boundary",
            ));
        }
        Ok(Self {
            source,
            position,
            end: source.len(),
        })
    }

    pub fn source(&self) -> &'a str {
        self.source
    }

    pub fn remaining(&self) -> &'a str {
        &self.source[self.position..self.end]
    }

    pub fn position(&self) -> usize {
        self.position
    }

    /// Consume an exact byte length, without skipping whitespace. The end must
    /// be an in-bounds UTF-8 boundary; failed advances leave the cursor unchanged.
    pub fn advance(&mut self, bytes: usize) -> Result<(), QueryError> {
        let Some(end) = self.position.checked_add(bytes) else {
            return Err(self.error("argument extends beyond query"));
        };
        if end > self.end || !self.source.is_char_boundary(end) {
            return Err(self.error("argument must end at a UTF-8 byte boundary"));
        }
        self.position = end;
        Ok(())
    }

    /// Skip whitespace, `#`/`%` line comments and nonnested block comments.
    /// A malformed block comment remains at the cursor for a checked read.
    pub fn skip_whitespace(&mut self) {
        let _ = self.skip_trivia();
    }

    pub(crate) fn skip_trivia(&mut self) -> Result<(), QueryError> {
        loop {
            let rest = self.remaining();
            let Some(ch) = rest.chars().next() else {
                return Ok(());
            };
            if ch.is_whitespace() {
                self.position += ch.len_utf8();
            } else if matches!(ch, '#' | '%') {
                self.position += rest.find('\n').unwrap_or(rest.len());
            } else if let Some(comment) = rest.strip_prefix("/*") {
                let Some(end) = comment.find("*/") else {
                    return Err(QueryError::query(
                        self.position..self.end,
                        "unterminated block comment",
                    ));
                };
                self.position += end + 4;
            } else {
                return Ok(());
            }
        }
    }

    /// Read an ASCII identifier or a nonempty single-quoted identifier.
    /// Quoted names return their logical identity without the quotes; the range
    /// includes the complete original spelling. Quoted keywords remain names.
    pub fn word(&mut self) -> Result<Located<&'a str>, QueryError> {
        self.skip_trivia()?;
        let start = self.position;
        if self.remaining().starts_with('\'') {
            self.advance(1)?;
            let Some(end) = self.remaining().find('\'') else {
                return Err(QueryError::query(
                    start..self.end,
                    "unterminated quoted identifier",
                ));
            };
            if self.remaining()[..end].contains(['\r', '\n', '\0']) {
                return Err(QueryError::query(
                    start..self.position + end,
                    "invalid quoted identifier",
                ));
            }
            if end == 0 {
                return Err(QueryError::query(
                    start..self.position + 1,
                    "expected a nonempty quoted identifier",
                ));
            }
            self.advance(end + 1)?;
            return Ok(Located::new(
                &self.source[start + 1..self.position - 1],
                start..self.position,
            ));
        }
        if !self
            .remaining()
            .bytes()
            .next()
            .is_some_and(|b| b.is_ascii_alphabetic() || b == b'_')
        {
            return Err(self.error("expected an identifier"));
        }
        while self
            .remaining()
            .bytes()
            .next()
            .is_some_and(|b| b.is_ascii_alphanumeric() || b == b'_')
        {
            self.position += 1;
        }
        Ok(Located::new(
            &self.source[start..self.position],
            start..self.position,
        ))
    }

    pub(crate) fn at(&self, position: usize) -> Result<Self, QueryError> {
        let mut reader = Self::new(self.source, position)?;
        if position > self.end {
            return Err(self.error("reader position exceeds expression boundary"));
        }
        reader.end = self.end;
        Ok(reader)
    }
    pub(crate) fn end(&self) -> usize {
        self.end
    }
    pub(crate) fn set_end(&mut self, end: usize) {
        debug_assert!(self.position <= end && self.source.is_char_boundary(end));
        self.end = end;
    }

    /// Skip a quote or comment while inspecting delimiters. No source is rewritten.
    pub(crate) fn protected_span(&mut self) -> Result<bool, QueryError> {
        let start = self.position;
        if self.remaining().starts_with(['#', '%']) || self.remaining().starts_with("/*") {
            self.skip_trivia()?;
            return Ok(true);
        }
        let Some(quote @ ('\'' | '"')) = self.remaining().chars().next() else {
            return Ok(false);
        };
        self.advance(1)?;
        let mut escaped = false;
        while let Some(ch) = self.remaining().chars().next() {
            self.advance(ch.len_utf8())?;
            if quote == '\'' && matches!(ch, '\r' | '\n' | '\0') {
                return Err(QueryError::query(
                    start..self.position,
                    "invalid quoted identifier",
                ));
            }
            if ch == quote && !escaped {
                return Ok(true);
            }
            escaped = quote == '"' && ch == '\\' && !escaped;
        }
        Err(QueryError::query(
            start..self.position,
            if quote == '"' {
                "unterminated string"
            } else {
                "unterminated quoted identifier"
            },
        ))
    }

    pub(crate) fn number_text(&mut self) -> Result<Located<&'a str>, QueryError> {
        self.skip_trivia()?;
        let start = self.position;
        if self.remaining().starts_with(['+', '-']) {
            self.advance(1)?;
        }
        let radix = self.remaining().starts_with("0x")
            || self.remaining().starts_with("0X")
            || self.remaining().starts_with("0o");
        let mut previous = b' ';
        while let Some(byte) = self.remaining().bytes().next() {
            if byte.is_ascii_digit()
                || (radix && byte.is_ascii_alphabetic())
                || matches!(byte, b'e' | b'E')
                || (byte == b'.' && !self.remaining().starts_with(".."))
                || (!radix && matches!(byte, b'+' | b'-') && matches!(previous, b'e' | b'E'))
            {
                self.advance(1)?;
                previous = byte;
            } else {
                break;
            }
        }
        Ok(Located::new(
            &self.source[start..self.position],
            start..self.position,
        ))
    }

    pub fn unsigned_integer(&mut self) -> Result<Located<usize>, QueryError> {
        self.skip_whitespace();
        let start = self.position;
        while self
            .remaining()
            .bytes()
            .next()
            .is_some_and(|byte| byte.is_ascii_digit())
        {
            self.position += 1;
        }
        let range = start..self.position;
        let value = self.source[range.clone()].parse().map_err(|_| {
            QueryError::query(
                range.clone(),
                "expected a non-negative integer fitting usize",
            )
        })?;
        Ok(Located::new(value, range))
    }

    /// Read a double-quoted UTF-8 string with JSON escapes, including Unicode.
    /// Literal newline, carriage return and tab characters are also retained.
    pub fn quoted_string(&mut self) -> Result<Located<String>, QueryError> {
        self.skip_trivia()?;
        let start = self.position;
        if !self.remaining().starts_with('"') {
            return Err(self.error("expected '\"'"));
        }
        self.protected_span()?;
        let range = start..self.position;
        let text = self.source[range.clone()]
            .replace('\n', "\\n")
            .replace('\r', "\\r")
            .replace('\t', "\\t");
        let value = serde_json::from_str(&text).map_err(|_| {
            QueryError::query(
                range.clone(),
                "invalid JSON string escape or Unicode sequence",
            )
        })?;
        Ok(Located::new(value, range))
    }

    pub fn expect(&mut self, character: char) -> Result<(), QueryError> {
        if self.consume(character) {
            return Ok(());
        }
        Err(self.error(format!("expected '{character}'")))
    }

    pub fn consume(&mut self, character: char) -> bool {
        self.skip_whitespace();
        if self.remaining().starts_with(character) {
            self.position += character.len_utf8();
            return true;
        }
        false
    }

    pub fn error(&self, message: impl Into<String>) -> QueryError {
        let end = self.position + self.remaining().chars().next().map_or(0, char::len_utf8);
        QueryError::query(self.position..end, message)
    }

    pub fn keyword(&mut self, keyword: &str) -> bool {
        self.skip_whitespace();
        let remaining = self.remaining();
        if remaining.starts_with(keyword)
            && !remaining[keyword.len()..]
                .bytes()
                .next()
                .is_some_and(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
        {
            self.position += keyword.len();
            return true;
        }
        false
    }
}

pub(crate) fn identifier_text(source: &str, range: std::ops::Range<usize>) -> &str {
    let text = &source[range];
    text.strip_prefix('\'')
        .and_then(|s| s.strip_suffix('\''))
        .unwrap_or(text)
}

/// Parse filter(predicate), head(nonnegative integer), unique, tally and count,
/// separated by pipes. Precedence is not, and, or. Parentheses group predicates. The host
/// parser consumes one whole leaf and validates its arguments; it must be
/// read-only. Boolean words and core stages are reserved in common syntax.
pub fn parse_pipeline<L>(
    source: &str,
    mut parse_leaf: impl FnMut(&mut ArgumentReader<'_>) -> Result<L, QueryError>,
    nesting_limit: usize,
) -> Result<Pipeline<L>, QueryError> {
    let mut parser = Parser {
        reader: ArgumentReader {
            source,
            position: 0,
            end: source.len(),
        },
        parse_leaf: &mut parse_leaf,
        nesting_limit: nesting_limit.min(MAX_NESTING),
    };
    check_depth(1, parser.nesting_limit, &(0..source.len()))?;
    let mut stages = Vec::new();
    parser.reader.skip_whitespace();
    while !parser.reader.remaining().is_empty() {
        let name = parser.reader.word()?;
        let value = match name.value {
            "filter" => {
                parser.reader.expect('(')?;
                let predicate = parser.or(2)?;
                parser.reader.expect(')')?;
                Stage::Filter(predicate)
            }
            "head" | "tail" => {
                parser.reader.expect('(')?;
                let count = parser.reader.unsigned_integer()?;
                parser.reader.expect(')')?;
                if name.value == "head" {
                    Stage::Head(count.value)
                } else {
                    Stage::Tail(count.value)
                }
            }
            "reverse" => Stage::Reverse,
            "check" => {
                parser.reader.skip_whitespace();
                let (mut expected, mut message) = (None, None);
                if parser.reader.remaining().starts_with('(') {
                    parser.reader.expect('(')?;
                    parser.reader.skip_whitespace();
                    if !parser.reader.remaining().starts_with('"') {
                        expected = Some(parser.reader.unsigned_integer()?.value);
                        parser.reader.skip_whitespace();
                        if parser.reader.remaining().starts_with(',') {
                            parser.reader.expect(',')?;
                            message = Some(parser.reader.quoted_string()?.value);
                        }
                    } else {
                        message = Some(parser.reader.quoted_string()?.value);
                    }
                    parser.reader.expect(')')?;
                }
                Stage::Check { expected, message }
            }
            "count" => Stage::Count,
            "unique" => Stage::Unique,
            "tally" => Stage::Tally,
            _ => return Err(QueryError::query(name.range, "unsupported pipeline stage")),
        };
        stages.push(Located::new(
            value,
            name.range.start..parser.reader.position,
        ));
        parser.reader.skip_whitespace();
        if parser.reader.remaining().is_empty() {
            break;
        }
        parser.reader.expect('|')?;
        parser.reader.skip_whitespace();
        if parser.reader.remaining().is_empty() {
            return Err(parser.reader.error("expected a pipeline stage after '|'"));
        }
    }
    Pipeline::new(stages, 0..source.len(), nesting_limit)
}

struct Parser<'a, F> {
    reader: ArgumentReader<'a>,
    parse_leaf: F,
    nesting_limit: usize,
}

impl<L, F> Parser<'_, F>
where
    F: FnMut(&mut ArgumentReader<'_>) -> Result<L, QueryError>,
{
    fn consume_operator(&mut self, operator: &str) -> bool {
        self.reader.skip_whitespace();
        if self.reader.remaining().starts_with(operator) {
            self.reader
                .advance(operator.len())
                .expect("operator is in input");
            true
        } else {
            false
        }
    }
    fn or(&mut self, depth: usize) -> Result<Predicate<L>, QueryError> {
        let mut children = vec![self.and(depth)?];
        while self.reader.keyword("or") || self.consume_operator("\\/") {
            children.push(self.and(depth)?);
        }
        Ok(combine(children, PredicateKind::Or))
    }

    fn and(&mut self, depth: usize) -> Result<Predicate<L>, QueryError> {
        let mut children = vec![self.unary(depth)?];
        while self.reader.keyword("and") || self.consume_operator("/\\") {
            children.push(self.unary(depth)?);
        }
        Ok(combine(children, PredicateKind::And))
    }

    fn unary(&mut self, depth: usize) -> Result<Predicate<L>, QueryError> {
        self.reader.skip_whitespace();
        let start = self.reader.position;
        check_depth(depth, self.nesting_limit, &(start..start))?;
        let value = if self.reader.keyword("not") {
            PredicateKind::Not(Box::new(self.unary(depth + 1)?))
        } else if self.reader.consume('(') {
            let child = self.or(depth + 1)?;
            self.reader.expect(')')?;
            PredicateKind::Group(Box::new(child))
        } else {
            let leaf = (self.parse_leaf)(&mut self.reader)?;
            if self.reader.position == start {
                return Err(self
                    .reader
                    .error("host leaf parser did not consume a predicate"));
            }
            PredicateKind::Leaf(leaf)
        };
        Ok(Located::new(value, start..self.reader.position))
    }
}

fn combine<L>(
    mut children: Vec<Predicate<L>>,
    kind: impl FnOnce(Vec<Predicate<L>>) -> PredicateKind<L>,
) -> Predicate<L> {
    if children.len() == 1 {
        return children.pop().expect("one predicate");
    }
    let range = children[0].range.start..children[children.len() - 1].range.end;
    Located::new(kind(children), range)
}
