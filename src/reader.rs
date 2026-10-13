use std::ops::Range;

use crate::{
    ArgumentReader, Budget, ConstructedValue as V, Expression, ExpressionKind, ExpressionStep,
    Located, Location, Predicate, PredicateKind, Projection, QueryError, ValueBinaryOperator as B,
    ValueConstructor, ValueExpression, ValueExpressionKind as K, ValueGenerator, ValueReducer,
    ValueUnaryOperator as U,
};

pub type ReadExpression<H> = Expression<
    <H as ValueReaderHost>::Stage,
    <H as ValueReaderHost>::SetOperator,
    <H as ValueReaderHost>::Emitter,
>;

/// Registered domain grammar leaves. Shared readers own values, projection,
/// native composition and Boolean predicate wrappers; hooks consume one leaf.
pub trait ValueReaderHost {
    type Stage;
    type SetOperator;
    type Emitter;
    type Predicate;

    fn is_binding(&self, name: &str) -> bool;
    fn is_stage(&self, name: &str) -> bool;
    fn parse_stage(
        &mut self,
        reader: &mut ArgumentReader<'_>,
        budget: &mut Budget,
    ) -> Result<ExpressionStep<Self::Stage, Self::Emitter>, QueryError>;
    fn parse_predicate_leaf(
        &mut self,
        reader: &mut ArgumentReader<'_>,
        budget: &mut Budget,
    ) -> Result<Self::Predicate, QueryError>;
    fn filter(&mut self, predicate: Predicate<Self::Predicate>, range: Range<usize>)
    -> Self::Stage;
    fn projection(
        &mut self,
        projection: Projection<ReadExpression<Self>>,
        range: Range<usize>,
    ) -> Self::Stage
    where
        Self: Sized;
    fn set_operator(&self, word: &str) -> Option<Self::SetOperator>;
}

/// Common statement prefixes are contextual, preserving legacy saved names.
#[derive(Debug, PartialEq, Eq)]
pub enum CommonStatement {
    Let(Located<String>),
    Report,
}

/// Recognize `let name =` and `report expression` without reserving bare names.
/// Failure to recognize leaves the original cursor unchanged.
///
/// # Errors
/// Invalid source boundaries or admitted name metadata fail at the query.
pub fn read_statement_prefix(
    reader: &mut ArgumentReader<'_>,
    budget: &mut Budget,
) -> Result<Option<CommonStatement>, QueryError> {
    let mut probe = ArgumentReader::new(reader.source(), reader.position())?;
    let Ok(word) = probe.word() else {
        return Ok(None);
    };
    let result = if word.value == "let" {
        let Ok(name) = probe.word() else {
            return Ok(None);
        };
        if !probe.consume('=') {
            return Ok(None);
        }
        budget.charge(name.value.len() + 1, Location::Query(name.range.clone()))?;
        CommonStatement::Let(Located::new(name.value.to_owned(), name.range))
    } else if word.value == "report" {
        probe.skip_whitespace();
        if probe.remaining().is_empty() || probe.remaining().starts_with(['=', '|', ';']) {
            return Ok(None);
        }
        CommonStatement::Report
    } else {
        return Ok(None);
    };
    reader.advance(probe.position() - reader.position())?;
    Ok(Some(result))
}

/// Read a value using original byte offsets and one parsing allowance.
/// Native operands remain normalized trees and execute only through the runner.
///
/// # Errors
/// Unsupported syntax, invalid literals, nesting and allocation refusals are located.
pub fn read_value_expression<H: ValueReaderHost>(
    reader: &mut ArgumentReader<'_>,
    host: &mut H,
    budget: &mut Budget,
    nesting: usize,
) -> Result<ValueExpression<ReadExpression<H>>, QueryError> {
    Reader {
        reader,
        host,
        budget,
        nesting: nesting.min(crate::MAX_NESTING),
    }
    .expression(0, 0)
}

/// Read `project(subject, {static quoted fields})` with shared value fields.
///
/// # Errors
/// Malformed syntax, literals and parse-budget exhaustion retain source offsets.
pub fn read_projection<H: ValueReaderHost>(
    reader: &mut ArgumentReader<'_>,
    host: &mut H,
    budget: &mut Budget,
    nesting: usize,
) -> Result<Projection<ReadExpression<H>>, QueryError> {
    Reader {
        reader,
        host,
        budget,
        nesting: nesting.min(crate::MAX_NESTING),
    }
    .projection(0)
}

struct Reader<'a, 's, 'h, H> {
    reader: &'a mut ArgumentReader<'s>,
    host: &'h mut H,
    budget: &'h mut Budget,
    nesting: usize,
}

impl<H: ValueReaderHost> Reader<'_, '_, '_, H> {
    fn location(&self) -> Location {
        Location::Query(self.reader.position()..self.reader.position())
    }
    fn slots<T>(&mut self, count: usize) -> Result<(), QueryError> {
        if count
            .checked_mul(std::mem::size_of::<T>())
            .is_none_or(|size| size > isize::MAX as usize)
        {
            return Err(self.reader.error("value allocation size overflow"));
        }
        self.budget.charge(count, self.location())?;
        self.budget.charge_results(count, self.location())
    }
    fn node(
        &mut self,
        value: K<ReadExpression<H>>,
        range: Range<usize>,
    ) -> Result<ValueExpression<ReadExpression<H>>, QueryError> {
        self.slots::<ValueExpression<ReadExpression<H>>>(1)?;
        Ok(Located::new(value, range))
    }
    fn depth(&self, depth: usize) -> Result<(), QueryError> {
        if depth >= self.nesting {
            return Err(self.reader.error("value nesting limit exceeded"));
        }
        Ok(())
    }
    fn name(&mut self) -> Result<Located<String>, QueryError> {
        let name = self.reader.word()?;
        self.budget
            .charge(name.value.len(), Location::Query(name.range.clone()))?;
        Ok(Located::new(name.value.to_owned(), name.range))
    }
    fn string(&mut self) -> Result<Located<String>, QueryError> {
        self.reader.skip_whitespace();
        let start = self.reader.position();
        self.reader.expect('"')?;
        let mut escaped = false;
        while let Some(ch) = self.reader.remaining().chars().next() {
            self.reader.advance(ch.len_utf8())?;
            if ch == '"' && !escaped {
                let range = start..self.reader.position();
                let text = &self.reader.source()[range.clone()];
                self.budget.charge(
                    text.len()
                        .checked_mul(6)
                        .ok_or_else(|| self.reader.error("value allocation size overflow"))?,
                    Location::Query(range.clone()),
                )?;
                let normalized = text
                    .replace('\n', "\\n")
                    .replace('\r', "\\r")
                    .replace('\t', "\\t");
                let value = serde_json::from_str(&normalized).map_err(|_| {
                    QueryError::query(
                        range.clone(),
                        "invalid JSON string escape or Unicode sequence",
                    )
                })?;
                return Ok(Located::new(value, range));
            }
            escaped = ch == '\\' && !escaped;
        }
        Err(QueryError::query(
            start..self.reader.position(),
            "unterminated string",
        ))
    }
    fn word_is(&self, word: &str) -> bool {
        let rest = self.reader.remaining().trim_start();
        rest.starts_with(word)
            && !rest[word.len()..]
                .bytes()
                .next()
                .is_some_and(|b| b.is_ascii_alphanumeric() || b == b'_')
    }
    fn projection_call(&self) -> bool {
        let mut probe = ArgumentReader::new(self.reader.source(), self.reader.position())
            .expect("reader byte boundary");
        probe
            .word()
            .is_ok_and(|word| word.value == "project" && self.host.is_stage(word.value))
            && probe.consume('(')
    }
    fn expression(
        &mut self,
        minimum: u8,
        depth: usize,
    ) -> Result<ValueExpression<ReadExpression<H>>, QueryError> {
        self.depth(depth)?;
        let mut left = self.atom(depth)?;
        let mut chain = 0;
        loop {
            self.reader.skip_whitespace();
            if minimum <= 10 && self.reader.consume('[') {
                let index = self.expression(0, depth + 1)?;
                self.reader.expect(']')?;
                let range = left.range.start..self.reader.position();
                left = self.node(
                    K::ArrayAccess {
                        array: Box::new(left),
                        index: Box::new(index),
                    },
                    range,
                )?;
            } else if minimum <= 10
                && self.reader.remaining().starts_with('.')
                && !self.reader.remaining().starts_with("..")
            {
                self.reader.advance(1)?;
                let field = self.name()?;
                let range = left.range.start..field.range.end;
                left = self.node(
                    K::Field {
                        value: Box::new(left),
                        field,
                    },
                    range,
                )?;
            } else {
                let rest = self.reader.remaining();
                let operator = [
                    ("or", 1, Some(B::Or)),
                    ("and", 2, Some(B::And)),
                    ("==", 3, Some(B::Equal)),
                    ("!=", 3, Some(B::NotEqual)),
                    ("<=", 4, Some(B::LessEqual)),
                    (">=", 4, Some(B::GreaterEqual)),
                    ("<", 4, Some(B::Less)),
                    (">", 4, Some(B::Greater)),
                    ("=", 3, Some(B::Equal)),
                    ("++", 5, Some(B::Concatenate)),
                    ("..", 6, None),
                    ("+", 7, Some(B::Add)),
                    ("-", 7, Some(B::Subtract)),
                    ("*", 8, Some(B::Multiply)),
                    ("/", 8, Some(B::Divide)),
                    ("div", 8, Some(B::IntegerDivide)),
                    ("mod", 8, Some(B::Modulo)),
                ]
                .into_iter()
                .find(|(word, _, _)| {
                    rest.starts_with(word)
                        && (!word.as_bytes()[0].is_ascii_alphabetic() || self.word_is(word))
                });
                let Some((word, precedence, operator)) = operator else {
                    break;
                };
                if precedence < minimum {
                    break;
                }
                self.reader.advance(word.len())?;
                let right = self.expression(precedence + 1, depth + chain + 1)?;
                let range = left.range.start..right.range.end;
                left = self.node(
                    match operator {
                        Some(operator) => K::Binary {
                            operator,
                            left: Box::new(left),
                            right: Box::new(right),
                        },
                        None => K::Range {
                            lower: Box::new(left),
                            upper: Box::new(right),
                        },
                    },
                    range,
                )?;
            }
            chain += 1;
            self.depth(depth + chain)?;
        }
        Ok(left)
    }
    fn atom(&mut self, depth: usize) -> Result<ValueExpression<ReadExpression<H>>, QueryError> {
        self.reader.skip_whitespace();
        let start = self.reader.position();
        let first = self
            .reader
            .remaining()
            .chars()
            .next()
            .ok_or_else(|| self.reader.error("expected a value expression"))?;
        if first == '[' {
            return self.array(depth);
        }
        if first == '{' {
            let fields = self.fields(depth)?;
            return self.node(K::Record(fields), start..self.reader.position());
        }
        if first == '"' {
            let string = self.string()?;
            return self.node(K::Literal(V::String(string.value)), string.range);
        }
        if first == '(' {
            let mut value = self.native_pipeline(depth)?;
            if matches!(value.value, K::NativeQuery(_)) {
                value = self.native_sets(value, depth)?;
            }
            return Ok(value);
        }
        if first.is_ascii_digit() || matches!(first, '+' | '-') {
            let negative = first == '-';
            if matches!(first, '+' | '-') {
                self.reader.advance(1)?;
                self.reader.skip_whitespace();
            }
            if !self
                .reader
                .remaining()
                .bytes()
                .next()
                .is_some_and(|b| b.is_ascii_digit())
            {
                let operand = self.expression(9, depth + 1)?;
                let range = start..operand.range.end;
                return self.node(
                    K::Unary {
                        operator: if negative { U::Negative } else { U::Positive },
                        operand: Box::new(operand),
                    },
                    range,
                );
            }
            let digits_start = self.reader.position();
            let mut previous = b' ';
            while let Some(byte) = self.reader.remaining().bytes().next() {
                if byte.is_ascii_digit()
                    || matches!(byte, b'e' | b'E')
                    || (byte == b'.' && !self.reader.remaining().starts_with(".."))
                    || (matches!(byte, b'+' | b'-') && matches!(previous, b'e' | b'E'))
                {
                    self.reader.advance(1)?;
                    previous = byte;
                } else {
                    break;
                }
            }
            let digits = &self.reader.source()[digits_start..self.reader.position()];
            self.budget.charge(digits.len() + 1, self.location())?;
            let spelling = if negative {
                format!("-{digits}")
            } else {
                digits.to_owned()
            };
            let value = number(&spelling, start..self.reader.position())?;
            return self.node(K::Literal(value), start..self.reader.position());
        }
        let mut probe = ArgumentReader::new(self.reader.source(), start)?;
        let word = probe.word()?;
        probe.skip_whitespace();
        if (self.host.is_stage(word.value)
            && (!self.host.is_binding(word.value) || self.projection_call()))
            || word.value == "input"
            || probe.remaining().starts_with('|')
        {
            return self.native(depth);
        }
        let name = self.name()?;
        let kind = match name.value.as_str() {
            "true" => K::Literal(V::Boolean(true)),
            "false" => K::Literal(V::Boolean(false)),
            "null" => {
                return Err(QueryError::query(
                    name.range,
                    "null is unsupported in constructed values",
                ));
            }
            "not" => K::Unary {
                operator: U::Not,
                operand: Box::new(self.expression(9, depth + 1)?),
            },
            _ if self.reader.consume('(') => {
                let value = self.expression(0, depth + 1)?;
                if name.value == "require" {
                    self.reader.expect(',')?;
                    let result = self.expression(0, depth + 1)?;
                    self.reader.expect(',')?;
                    let message = self.string()?;
                    self.reader.expect(')')?;
                    K::Require {
                        condition: Box::new(value),
                        value: Box::new(result),
                        message,
                    }
                } else {
                    self.reader.expect(')')?;
                    let operator = match name.value.as_str() {
                        "length" => ValueReducer::Length,
                        "sum" => ValueReducer::Sum,
                        "forall" => ValueReducer::Forall,
                        "exists" => ValueReducer::Exists,
                        "one" => {
                            return self
                                .node(K::One(Box::new(value)), start..self.reader.position());
                        }
                        _ => return Err(QueryError::query(name.range, "unsupported value call")),
                    };
                    K::Reduce {
                        operator,
                        array: Box::new(value),
                    }
                }
            }
            _ => K::Name(name.value),
        };
        self.node(kind, start..self.reader.position())
    }
    fn fields(
        &mut self,
        depth: usize,
    ) -> Result<crate::ProjectionFields<ReadExpression<H>>, QueryError> {
        self.reader.expect('{')?;
        let mut fields = Vec::new();
        if self.reader.consume('}') {
            return Ok(fields);
        }
        loop {
            let label = self.string()?;
            self.reader.expect(':')?;
            let value = self.expression(0, depth + 1)?;
            self.slots::<(Located<String>, ValueExpression<ReadExpression<H>>)>(1)?;
            fields.push((label, value));
            if !self.reader.consume(',') {
                self.reader.expect('}')?;
                break;
            }
            self.reader.skip_whitespace();
            if self.reader.remaining().starts_with('}') {
                return Err(self.reader.error("trailing comma in record fields"));
            }
        }
        Ok(fields)
    }
    fn array(&mut self, depth: usize) -> Result<ValueExpression<ReadExpression<H>>, QueryError> {
        let start = self.reader.position();
        self.reader.expect('[')?;
        let mut values = Vec::new();
        if self.reader.consume(']') {
            return self.node(K::Array(values), start..self.reader.position());
        }
        let head = self.expression(0, depth + 1)?;
        if self.word_is("for") {
            let mut clauses = Vec::new();
            while self.reader.keyword("for") {
                let clause_start = self.reader.position();
                let name = self.name()?;
                if !self.reader.keyword("in") {
                    return Err(self
                        .reader
                        .error("expected `in` after comprehension binder"));
                }
                let source = self.expression(0, depth + 1)?;
                let filter = if self.reader.keyword("if") {
                    Some(self.expression(0, depth + 1)?)
                } else {
                    None
                };
                let end = filter
                    .as_ref()
                    .map_or(source.range.end, |value| value.range.end);
                self.slots::<Located<ValueGenerator<ReadExpression<H>>>>(1)?;
                self.slots::<Located<Option<String>>>(1)?;
                clauses.push(Located::new(
                    ValueGenerator::In {
                        binders: vec![Located::new(
                            (name.value != "_").then_some(name.value),
                            name.range,
                        )],
                        source,
                        filter,
                    },
                    clause_start..end,
                ));
                self.depth(depth + clauses.len())?;
            }
            self.reader.expect(']')?;
            return self.node(
                K::Comprehension {
                    constructor: ValueConstructor::Array,
                    clauses,
                    head: Box::new(head),
                },
                start..self.reader.position(),
            );
        }
        self.slots::<ValueExpression<ReadExpression<H>>>(1)?;
        values.push(head);
        while self.reader.consume(',') {
            let value = self.expression(0, depth + 1)?;
            self.slots::<ValueExpression<ReadExpression<H>>>(1)?;
            values.push(value);
        }
        self.reader.expect(']')?;
        self.node(K::Array(values), start..self.reader.position())
    }
    fn projection(&mut self, depth: usize) -> Result<Projection<ReadExpression<H>>, QueryError> {
        self.depth(depth)?;
        if !self.reader.keyword("project") {
            return Err(self.reader.error("expected `project`"));
        }
        self.reader.expect('(')?;
        let subject = self.name()?;
        self.reader.expect(',')?;
        let fields = self.fields(depth + 1)?;
        self.reader.expect(')')?;
        Ok(Projection { subject, fields })
    }
    fn native(&mut self, depth: usize) -> Result<ValueExpression<ReadExpression<H>>, QueryError> {
        let value = self.native_pipeline(depth)?;
        self.native_sets(value, depth)
    }
    fn native_sets(
        &mut self,
        mut value: ValueExpression<ReadExpression<H>>,
        depth: usize,
    ) -> Result<ValueExpression<ReadExpression<H>>, QueryError> {
        loop {
            self.reader.skip_whitespace();
            let mut probe = ArgumentReader::new(self.reader.source(), self.reader.position())?;
            let Ok(word) = probe.word() else {
                break;
            };
            let Some(operator) = self.host.set_operator(word.value) else {
                break;
            };
            self.reader
                .advance(probe.position() - self.reader.position())?;
            let right = match self.native_pipeline(depth) {
                Ok(right) => right,
                Err(error) => {
                    dispose_native_value(value);
                    return Err(error);
                }
            };
            let range = value.range.start..right.range.end;
            if !matches!(value.value, K::NativeQuery(_))
                || !matches!(right.value, K::NativeQuery(_))
            {
                dispose_native_value(value);
                dispose_native_value(right);
                return Err(QueryError::query(
                    range,
                    "set operands require native selections",
                ));
            }
            let (K::NativeQuery(left), K::NativeQuery(right)) = (value.value, right.value) else {
                unreachable!("native set operands checked")
            };
            let guard = NativeGuard(Some(Located::new(
                ExpressionKind::Set {
                    operator,
                    left,
                    right,
                },
                range.clone(),
            )));
            self.slots::<ReadExpression<H>>(1)?;
            self.slots::<ValueExpression<ReadExpression<H>>>(1)?;
            let mut guard = guard;
            value = Located::new(
                K::NativeQuery(Box::new(
                    guard.0.take().expect("retained native expression"),
                )),
                range,
            );
        }
        Ok(value)
    }
    fn native_pipeline(
        &mut self,
        depth: usize,
    ) -> Result<ValueExpression<ReadExpression<H>>, QueryError> {
        self.depth(depth)?;
        self.reader.skip_whitespace();
        let start = self.reader.position();
        self.slots::<ReadExpression<H>>(2)?;
        let source = if self.reader.consume('(') {
            let value = self.expression(0, depth + 1)?;
            if let Err(error) = self.reader.expect(')') {
                dispose_native_value(value);
                return Err(error);
            }
            let range = start..self.reader.position();
            match value.value {
                K::NativeQuery(query) => Located::new(ExpressionKind::Group(query), range),
                kind => {
                    self.reader.skip_whitespace();
                    if self.reader.remaining().starts_with('|') {
                        return Err(self.reader.error("pipeline input requires a native query"));
                    }
                    return Ok(Located::new(kind, range));
                }
            }
        } else {
            let mut probe = ArgumentReader::new(self.reader.source(), start)?;
            let first = probe.word()?;
            probe.skip_whitespace();
            let projection_call = self.projection_call();
            if first.value == "input" {
                self.reader.advance(probe.position() - start)?;
                Located::new(ExpressionKind::Input, first.range)
            } else if !projection_call
                && (self.host.is_binding(first.value) || !self.host.is_stage(first.value))
            {
                self.reader.advance(probe.position() - start)?;
                self.budget
                    .charge(first.value.len(), Location::Query(first.range.clone()))?;
                Located::new(ExpressionKind::Binding(first.value.to_owned()), first.range)
            } else {
                Located::new(ExpressionKind::Input, start..start)
            }
        };
        let mut source = NativeGuard(Some(source));
        let mut steps = Vec::new();
        let starts_stage = matches!(
            source.0.as_ref().expect("native source").value,
            ExpressionKind::Input
        ) && source.0.as_ref().expect("native source").range.is_empty();
        let mut more = starts_stage || self.reader.consume('|');
        while more {
            self.reader.skip_whitespace();
            let stage_start = self.reader.position();
            if !self.host.is_binding("one") && self.reader.keyword("one") {
                let range = start..self.reader.position();
                let query = Located::new(
                    ExpressionKind::Sequence {
                        input: Box::new(source.0.take().expect("native source")),
                        steps,
                    },
                    range.clone(),
                );
                let mut query = NativeGuard(Some(query));
                self.slots::<ValueExpression<ReadExpression<H>>>(2)?;
                let value = Located::new(
                    K::NativeQuery(Box::new(query.0.take().expect("native extraction source"))),
                    range,
                );
                return Ok(Located::new(
                    K::One(Box::new(value)),
                    stage_start..self.reader.position(),
                ));
            }
            let step = if self.reader.keyword("filter") {
                self.reader.expect('(')?;
                let predicate = self.predicate_or(depth + 1)?;
                self.reader.expect(')')?;
                ExpressionStep::Stage(
                    self.host
                        .filter(predicate, stage_start..self.reader.position()),
                )
            } else if self.projection_call() {
                let projection = self.projection(depth + 1)?;
                ExpressionStep::Stage(
                    self.host
                        .projection(projection, stage_start..self.reader.position()),
                )
            } else {
                self.host.parse_stage(self.reader, self.budget)?
            };
            if self.reader.position() == stage_start {
                return Err(self
                    .reader
                    .error("host stage parser did not consume a stage"));
            }
            self.slots::<Located<ExpressionStep<H::Stage, H::Emitter>>>(1)?;
            steps.push(Located::new(step, stage_start..self.reader.position()));
            more = self.reader.consume('|');
        }
        let range = start..self.reader.position();
        let query = Located::new(
            ExpressionKind::Sequence {
                input: Box::new(source.0.take().expect("native source")),
                steps,
            },
            range.clone(),
        );
        let mut query = NativeGuard(Some(query));
        self.slots::<ValueExpression<ReadExpression<H>>>(1)?;
        Ok(Located::new(
            K::NativeQuery(Box::new(query.0.take().expect("native pipeline"))),
            range,
        ))
    }
    fn predicate_or(&mut self, depth: usize) -> Result<Predicate<H::Predicate>, QueryError> {
        let first = self.predicate_and(depth)?;
        self.slots::<Predicate<H::Predicate>>(1)?;
        let mut predicates = vec![first];
        while self.reader.keyword("or") {
            self.slots::<Predicate<H::Predicate>>(1)?;
            predicates.push(self.predicate_and(depth)?);
        }
        Ok(combine(predicates, PredicateKind::Or))
    }
    fn predicate_and(&mut self, depth: usize) -> Result<Predicate<H::Predicate>, QueryError> {
        let first = self.predicate_atom(depth)?;
        self.slots::<Predicate<H::Predicate>>(1)?;
        let mut predicates = vec![first];
        while self.reader.keyword("and") {
            self.slots::<Predicate<H::Predicate>>(1)?;
            predicates.push(self.predicate_atom(depth)?);
        }
        Ok(combine(predicates, PredicateKind::And))
    }
    fn predicate_atom(&mut self, depth: usize) -> Result<Predicate<H::Predicate>, QueryError> {
        self.depth(depth)?;
        self.reader.skip_whitespace();
        let start = self.reader.position();
        let value = if self.reader.keyword("not") {
            PredicateKind::Not(Box::new(self.predicate_atom(depth + 1)?))
        } else if self.reader.consume('(') {
            let predicate = self.predicate_or(depth + 1)?;
            self.reader.expect(')')?;
            PredicateKind::Group(Box::new(predicate))
        } else {
            PredicateKind::Leaf(self.host.parse_predicate_leaf(self.reader, self.budget)?)
        };
        self.slots::<Predicate<H::Predicate>>(1)?;
        Ok(Located::new(value, start..self.reader.position()))
    }
}

fn combine<P>(
    mut children: Vec<Predicate<P>>,
    kind: impl FnOnce(Vec<Predicate<P>>) -> PredicateKind<P>,
) -> Predicate<P> {
    if children.len() == 1 {
        return children.pop().expect("one predicate");
    }
    let range = children[0].range.start..children.last().expect("Boolean children").range.end;
    Located::new(kind(children), range)
}

fn number(text: &str, range: Range<usize>) -> Result<V, QueryError> {
    if !text.contains(['.', 'e', 'E']) {
        return text
            .parse::<i64>()
            .map(V::Integer)
            .map_err(|_| QueryError::query(range, "integer literal exceeds signed 64-bit range"));
    }
    let value = text
        .parse::<f64>()
        .map_err(|_| QueryError::query(range.clone(), "invalid decimal float"))?;
    if !value.is_finite() {
        return Err(QueryError::query(range, "nonfinite float literal"));
    }
    if value == 0.0
        && text
            .split(['e', 'E'])
            .next()
            .expect("mantissa")
            .bytes()
            .any(|b| matches!(b, b'1'..=b'9'))
    {
        return Err(QueryError::query(
            range,
            "nonzero decimal literal rounded to zero",
        ));
    }
    Ok(V::Float(value))
}

struct NativeGuard<S, O, E>(Option<Expression<S, O, E>>);
impl<S, O, E> Drop for NativeGuard<S, O, E> {
    fn drop(&mut self) {
        if let Some(expression) = self.0.take() {
            crate::dispose_expression(expression);
        }
    }
}
fn dispose_native_value<S, O, E>(value: ValueExpression<Expression<S, O, E>>) {
    if let K::NativeQuery(query) = value.value {
        crate::dispose_expression(*query);
    }
}
