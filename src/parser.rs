use crate::{
    Located, MAX_NESTING, Pipeline, Predicate, PredicateKind, QueryError, Stage, check_depth,
};

/// The host reads its leaf name and arguments from the original query text.
/// Common helpers skip whitespace and return original UTF-8 byte ranges.
/// `source`, `remaining`, `position` and `advance` support host-specific forms.
pub struct ArgumentReader<'a> {
    source: &'a str,
    position: usize,
}

impl<'a> ArgumentReader<'a> {
    pub fn source(&self) -> &'a str {
        self.source
    }

    pub fn remaining(&self) -> &'a str {
        &self.source[self.position..]
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
        if !self.source.is_char_boundary(end) {
            return Err(self.error("argument must end at a UTF-8 byte boundary"));
        }
        self.position = end;
        Ok(())
    }

    pub fn skip_whitespace(&mut self) {
        while let Some(character) = self.remaining().chars().next() {
            if !character.is_whitespace() {
                break;
            }
            self.position += character.len_utf8();
        }
    }

    /// Read an ASCII identifier: a letter or underscore, then letters, digits
    /// or underscores. A host may read other names through `remaining/advance`.
    pub fn word(&mut self) -> Result<Located<&'a str>, QueryError> {
        self.skip_whitespace();
        let start = self.position;
        if !self
            .remaining()
            .bytes()
            .next()
            .is_some_and(|byte| byte.is_ascii_alphabetic() || byte == b'_')
        {
            return Err(self.error("expected an identifier"));
        }
        while self
            .remaining()
            .bytes()
            .next()
            .is_some_and(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
        {
            self.position += 1;
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

    /// Read a double-quoted UTF-8 string with \" \\ \n \r and \t escapes.
    pub fn quoted_string(&mut self) -> Result<Located<String>, QueryError> {
        self.skip_whitespace();
        let start = self.position;
        self.expect('"')?;
        let mut value = String::new();
        while let Some(character) = self.remaining().chars().next() {
            self.position += character.len_utf8();
            match character {
                '"' => return Ok(Located::new(value, start..self.position)),
                '\\' => {
                    let escape_start = self.position - 1;
                    let Some(escape) = self.remaining().chars().next() else {
                        break;
                    };
                    self.position += escape.len_utf8();
                    value.push(match escape {
                        '"' => '"',
                        '\\' => '\\',
                        'n' => '\n',
                        'r' => '\r',
                        't' => '\t',
                        _ => {
                            return Err(QueryError::query(
                                escape_start..self.position,
                                "unsupported string escape",
                            ));
                        }
                    });
                }
                _ => value.push(character),
            }
        }
        Err(QueryError::query(
            start..self.position,
            "unterminated string",
        ))
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

    fn keyword(&mut self, keyword: &str) -> bool {
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

/// Parse filter(predicate), head(nonnegative integer) and count, separated by
/// pipes. Precedence is not, and, or. Parentheses group predicates. The host
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
            "head" => {
                parser.reader.expect('(')?;
                let count = parser.reader.unsigned_integer()?;
                parser.reader.expect(')')?;
                Stage::Head(count.value)
            }
            "count" => Stage::Count,
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
    fn or(&mut self, depth: usize) -> Result<Predicate<L>, QueryError> {
        let mut children = vec![self.and(depth)?];
        while self.reader.keyword("or") {
            children.push(self.and(depth)?);
        }
        Ok(combine(children, PredicateKind::Or))
    }

    fn and(&mut self, depth: usize) -> Result<Predicate<L>, QueryError> {
        let mut children = vec![self.unary(depth)?];
        while self.reader.keyword("and") {
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
