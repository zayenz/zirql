//! Common declarations and lexical statement progression.
use super::*;
use crate::{Pattern, Statement};
use std::{borrow::Cow, collections::HashSet};

/// One common statement with this host's native payloads.
pub type ReadStatement<H> = Statement<
    <H as ValueReaderHost>::Stage,
    <H as ValueReaderHost>::SetOperator,
    <H as ValueReaderHost>::Emitter,
>;
/// A complete program with this host's native payloads.
pub type ProgramRead<H> = ReadProgram<
    <H as ValueReaderHost>::Stage,
    <H as ValueReaderHost>::SetOperator,
    <H as ValueReaderHost>::Emitter,
>;

/// Why an expression is being read. Effects and output remain host decisions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProgramExpression {
    Binding,
    Report,
    Evaluate,
    Suppressed,
}

/// A parameter's written position among executable statements.
#[derive(Debug, PartialEq, Eq)]
pub struct ReadParameter {
    pub declaration: ParameterDeclaration,
    pub position: usize,
}

/// Parsed syntax, without preparation, execution or rendered output.
#[derive(Debug, PartialEq)]
pub struct ReadProgram<S, O, E> {
    pub functions: Vec<UnresolvedPatternFunctionDeclaration<Expression<S, O, E>>>,
    pub parameters: Vec<ReadParameter>,
    pub statements: Vec<Located<Statement<S, O, E>>>,
    pub range: Range<usize>,
}

/// Native syntax, admission and output decisions used by [`read_program`].
/// Each expression callback consumes one expression and receives only names
/// visible at that source position. Function bodies have no ambient names.
pub trait ProgramReaderHost: ValueReaderHost {
    /// Register one borrowed forward name before any function body is read.
    fn function_name(
        &mut self,
        _name: Located<&str>,
        _budget: &mut Budget,
    ) -> Result<(), QueryError> {
        Ok(())
    }
    fn function_scope(&mut self, _active: bool) {}
    fn admit_functions(
        &mut self,
        _functions: &mut [UnresolvedPatternFunctionDeclaration<ReadExpression<Self>>],
        _budget: &mut Budget,
    ) -> Result<(), QueryError>
    where
        Self: Sized,
    {
        Ok(())
    }
    /// Apply host binding-name restrictions; common duplicates are checked first.
    fn admit_name(&mut self, _name: Located<&str>, _parameter: bool) -> Result<(), QueryError> {
        Ok(())
    }
    /// Select the host's native-result admission for the recognized pattern.
    fn binding_pattern(&mut self, _pattern: &Pattern) {}
    /// Recognize only host binding syntax, such as a legacy `name =` header.
    fn read_binding(
        &mut self,
        _reader: &mut ArgumentReader<'_>,
        _budget: &mut Budget,
    ) -> Result<Option<Pattern>, QueryError> {
        Ok(None)
    }
    /// Recognize a host evaluation prefix such as `do`.
    fn read_output_prefix(
        &mut self,
        _reader: &mut ArgumentReader<'_>,
    ) -> Option<ProgramExpression> {
        None
    }
    /// Read one domain statement with its own effect-specific terminator.
    fn read_host_statement(
        &mut self,
        _reader: &mut ArgumentReader<'_>,
        _names: &[&str],
        _budget: &mut Budget,
        _nesting: usize,
    ) -> Result<Option<Vec<Located<ReadStatement<Self>>>>, QueryError>
    where
        Self: Sized,
    {
        Ok(None)
    }
    fn read_program_expression(
        &mut self,
        reader: &mut ArgumentReader<'_>,
        names: &[&str],
        purpose: ProgramExpression,
        budget: &mut Budget,
        nesting: usize,
    ) -> Result<ReadExpression<Self>, QueryError>;
    fn parameter_statement(
        &mut self,
        declaration: &ParameterDeclaration,
        _index: usize,
        _budget: &mut Budget,
    ) -> Result<Option<Located<ReadStatement<Self>>>, QueryError> {
        _budget.charge(
            declaration.name.value.len(),
            Location::Query(declaration.name.range.clone()),
        )?;
        Ok(Some(Located::new(
            Statement::Parameter {
                declaration: declaration.clone(),
            },
            declaration.name.range.clone(),
        )))
    }
    fn binding_statement(
        &mut self,
        pattern: Pattern,
        _names: Vec<Located<String>>,
        expression: ReadExpression<Self>,
        _budget: &mut Budget,
    ) -> Result<ReadStatement<Self>, QueryError> {
        if let crate::PatternKind::Name(name) = pattern.value {
            Ok(Statement::Bind { name, expression })
        } else {
            let range = expression.range.clone();
            let value = Located::new(K::NativeValue(Box::new(expression)), range);
            Ok(Statement::BindPattern {
                pattern: pattern.into(),
                value: value.into(),
            })
        }
    }

    fn implicit_output(&mut self, _purpose: ProgramExpression) -> Option<Self::Emitter> {
        None
    }
    fn finish_program(
        &mut self,
        _reader: &ArgumentReader<'_>,
        _budget: &mut Budget,
    ) -> Result<(), QueryError> {
        Ok(())
    }
}

/// Read common functions, parameters, pattern lets and ordered evaluations.
/// Forward function names are available before any body; parameters and bindings
/// become visible only after their declaration. Host callbacks parse native
/// syntax and preserve domain admission without executing effects.
///
/// # Errors
/// Malformed declarations, duplicate names, trailing input and exhausted limits
/// retain their original byte locations. Partial trees are disposed iteratively.
pub fn read_program<H: ProgramReaderHost>(
    source: &str,
    initial_names: &[&str],
    host: &mut H,
    budget: &mut Budget,
    nesting: usize,
) -> Result<ProgramRead<H>, QueryError> {
    read_program_into(source, initial_names, host, budget, nesting, &mut None)
}

/// Read the current constructed-value subset: pattern lets and one final value.
/// The expression callback returns a single core value step; this entry assembles
/// its existing `Block` form without introducing a second statement reader.
///
/// # Errors
/// Common program errors and native or multiple-result expressions are located.
pub fn read_value_program<H: ProgramReaderHost>(
    source: &str,
    initial_names: &[&str],
    host: &mut H,
    budget: &mut Budget,
    nesting: usize,
) -> Result<ValueExpression<ReadExpression<H>>, QueryError> {
    let mut values = Some(ValueProgram {
        bindings: Vec::new(),
        value: None,
    });
    let parsed = read_program_into(source, initial_names, host, budget, nesting, &mut values);
    if let Err(error) = parsed {
        if let Some(values) = values {
            for (pattern, value) in values.bindings {
                crate::dispose_pattern(pattern);
                dispose_native_value(value);
            }
            if let Some(value) = values.value {
                dispose_native_value(value);
            }
        }
        return Err(error);
    }
    let values = values.expect("value program destination");
    let Some(value) = values.value else {
        for (pattern, value) in values.bindings {
            crate::dispose_pattern(pattern);
            dispose_native_value(value);
        }
        return Err(QueryError::query(
            0..source.len(),
            "expected a final value expression",
        ));
    };
    if values.bindings.is_empty() {
        return Ok(value);
    }
    Ok(Located::new(
        K::Block {
            bindings: values.bindings,
            value: Box::new(value),
        },
        0..source.len(),
    ))
}

struct ValueProgram<Q> {
    bindings: Vec<(Pattern, ValueExpression<Q>)>,
    value: Option<ValueExpression<Q>>,
}
fn program_value<S, O, E>(
    expression: Expression<S, O, E>,
) -> Result<ValueExpression<Expression<S, O, E>>, QueryError> {
    let range = expression.range;
    let expression = match expression.value {
        ExpressionKind::Sequence { input, mut steps } => {
            if matches!(input.value, ExpressionKind::Input)
                && steps.len() == 1
                && matches!(steps[0].value, ExpressionStep::Value(_))
            {
                let ExpressionStep::Value(value) = steps.pop().expect("one value step").value
                else {
                    unreachable!("checked value step")
                };
                return std::sync::Arc::try_unwrap(value).map_err(|_| {
                    QueryError::query(range, "value program requires an owned value expression")
                });
            }
            ExpressionKind::Sequence { input, steps }
        }
        expression => expression,
    };
    crate::dispose_expression(Located::new(expression, range.clone()));
    Err(QueryError::query(
        range,
        "native expressions are unavailable in a value program",
    ))
}

fn read_program_into<'s, H: ProgramReaderHost>(
    source: &'s str,
    initial_names: &'s [&'s str],
    host: &mut H,
    budget: &mut Budget,
    nesting: usize,
    values: &mut Option<ValueProgram<ReadExpression<H>>>,
) -> Result<ProgramRead<H>, QueryError> {
    let mut result = ProgramGuard(Some(ReadProgram {
        functions: Vec::new(),
        parameters: Vec::new(),
        statements: Vec::new(),
        range: 0..source.len(),
    }));
    let headers = function_headers(source, host, budget, nesting)?;
    if !headers.is_empty() && values.is_some() {
        return Err(QueryError::query(
            headers[0].name.range.clone(),
            "expected a value expression; function declarations are unavailable in this host",
        ));
    }
    let mut names = Cow::Borrowed(initial_names);
    admit_slots::<&str>(initial_names.len(), budget, &(0..0))?;
    let mut defined: HashSet<&str> = initial_names.iter().copied().collect();
    for header in &headers {
        define_name(header.name.value, &header.name.range, &mut defined, budget)?;
        host.admit_name(header.name.clone(), false)?;
        host.function_name(header.name.clone(), budget)?;
    }
    read_functions(
        source,
        &headers,
        host,
        budget,
        nesting,
        &mut result.program().functions,
    )?;
    let mut reader = ArgumentReader::new(source, 0)?;
    let mut declarations = headers.iter().peekable();
    loop {
        reader.skip_trivia()?;
        if reader.remaining().is_empty() {
            break;
        }
        if let Some(header) = declarations.peek()
            && header.range.start == reader.position()
        {
            reader.advance(header.range.len())?;
            declarations.next();
            continue;
        }
        let start = reader.position();
        if let Some(statements) = host.read_host_statement(&mut reader, &names, budget, nesting)? {
            if reader.position() == start {
                return Err(reader.error("host statement parser did not consume a statement"));
            }
            if let Err(error) = admit_slots::<Located<ReadStatement<H>>>(
                statements.len(),
                budget,
                &(start..reader.position()),
            ) {
                dispose_statements(statements);
                return Err(error);
            }
            result.program().statements.extend(statements);
            continue;
        }
        let mut probe = reader.at(start)?;
        if let Some(CommonStatement::Param(declaration)) =
            read_statement_prefix(&mut probe, budget)?
        {
            if values.is_some() {
                return Err(probe.error("parameter declarations are unavailable in this host"));
            }
            define_name(
                crate::parser::identifier_text(source, declaration.name.range.clone()),
                &declaration.name.range,
                &mut defined,
                budget,
            )?;
            host.admit_name(
                Located::new(
                    declaration.name.value.as_str(),
                    declaration.name.range.clone(),
                ),
                true,
            )?;
            probe.expect(';')?;
            reader.advance(probe.position() - start)?;
            let position = result.program().statements.len();
            let index = result.program().parameters.len();
            if let Some(statement) = host.parameter_statement(&declaration, index, budget)? {
                if let Err(error) =
                    admit_slots::<Located<ReadStatement<H>>>(1, budget, &statement.range)
                {
                    dispose_statement(statement.value);
                    return Err(error);
                }
                result.program().statements.push(statement);
            }
            introduce_name(&declaration.name, source, &mut names, budget)?;
            admit_slots::<ReadParameter>(1, budget, &declaration.name.range)?;
            result.program().parameters.push(ReadParameter {
                declaration,
                position,
            });
            continue;
        }
        let pattern = match host.read_binding(&mut reader, budget)? {
            Some(pattern) => Some(pattern),
            None => read_pattern_let(&mut reader, budget, nesting)?,
        };
        let pattern = PatternGuard(pattern);
        let binding_names = if let Some(pattern) = &pattern.0 {
            let mut selected = Vec::new();
            for name in crate::pattern_names(pattern, budget)? {
                define_name(
                    crate::parser::identifier_text(source, name.range.clone()),
                    &name.range,
                    &mut defined,
                    budget,
                )?;
                host.admit_name(Located::new(name.value.as_str(), name.range.clone()), false)?;
                budget
                    .charge_program_work(name.value.len(), Location::Query(name.range.clone()))?;
                admit_slots::<Located<String>>(1, budget, &name.range)?;
                selected.push(name.clone());
            }
            selected
        } else {
            Vec::new()
        };
        if let Some(pattern) = &pattern.0 {
            host.binding_pattern(pattern);
        }
        let purpose = if pattern.0.is_some() {
            ProgramExpression::Binding
        } else if let Some(purpose) = host.read_output_prefix(&mut reader) {
            purpose
        } else {
            let mut report = reader.at(reader.position())?;
            if matches!(
                read_statement_prefix(&mut report, budget)?,
                Some(CommonStatement::Report)
            ) {
                if values.is_some() {
                    return Err(report.error("report statements are unavailable in this host"));
                }
                reader.advance(report.position() - reader.position())?;
                ProgramExpression::Report
            } else {
                ProgramExpression::Evaluate
            }
        };
        let expression = ExpressionGuard(Some(host.read_program_expression(
            &mut reader,
            &names,
            purpose,
            budget,
            nesting,
        )?));
        reader.skip_trivia()?;
        if purpose == ProgramExpression::Binding || purpose == ProgramExpression::Suppressed {
            reader.expect(';')?;
        } else if values.is_some() {
            if !reader.remaining().is_empty() {
                return Err(reader.error("expected the end of the expression"));
            }
        } else if !reader.consume(';') && !reader.remaining().is_empty() {
            return Err(reader.error("expected ';' or end of program"));
        }
        if let Some(values) = values {
            if pattern.0.is_some() {
                admit_slots::<(Pattern, ValueExpression<ReadExpression<H>>)>(
                    1,
                    budget,
                    &(start..reader.position()),
                )?;
            }
            let value = program_value(expression.into_expression())?;
            if pattern.0.is_some() {
                values.bindings.push((pattern.into_pattern(), value));
                for name in &binding_names {
                    introduce_name(name, source, &mut names, budget)?;
                }
            } else if values.value.is_some() {
                dispose_native_value(value);
                return Err(reader.error("expected one final value expression"));
            } else {
                values.value = Some(value);
            }
            continue;
        }
        admit_slots::<Located<ReadStatement<H>>>(1, budget, &(start..reader.position()))?;
        let statement = if pattern.0.is_some() {
            let pattern = pattern.into_pattern();
            for name in &binding_names {
                introduce_name(name, source, &mut names, budget)?;
            }
            host.binding_statement(pattern, binding_names, expression.into_expression(), budget)?
        } else {
            Statement::Evaluate {
                expression: expression.into_expression(),
                implicit_output: host.implicit_output(purpose),
            }
        };
        result
            .program()
            .statements
            .push(Located::new(statement, start..reader.position()));
    }
    host.finish_program(&reader, budget)?;
    Ok(result.0.take().expect("completed program"))
}

fn read_functions<H: ProgramReaderHost>(
    source: &str,
    headers: &[FunctionHeader],
    host: &mut H,
    budget: &mut Budget,
    nesting: usize,
    functions: &mut Vec<UnresolvedPatternFunctionDeclaration<ReadExpression<H>>>,
) -> Result<(), QueryError> {
    host.function_scope(true);
    let admission = (|| {
        for header in headers {
            admit_slots::<UnresolvedPatternFunctionDeclaration<ReadExpression<H>>>(
                1,
                budget,
                &header.range,
            )?;
            let mut reader = ArgumentReader::new(source, header.range.start)?;
            let function =
                read_unresolved_pattern_function_declaration(&mut reader, host, budget, nesting)?
                    .ok_or_else(|| reader.error("expected a function declaration"))?;
            functions.push(function);
        }
        host.admit_functions(functions, budget)
    })();
    host.function_scope(false);
    admission
}

fn admit_slots<T>(
    count: usize,
    budget: &mut Budget,
    range: &Range<usize>,
) -> Result<(), QueryError> {
    if count
        .checked_mul(std::mem::size_of::<T>())
        .is_none_or(|size| size > isize::MAX as usize)
    {
        return Err(QueryError::query(
            range.clone(),
            "program allocation size overflow",
        ));
    }
    budget.charge_program_work(count, Location::Query(range.clone()))?;
    budget.charge_results(count, Location::Query(range.clone()))
}
fn define_name<'a>(
    name: &'a str,
    range: &Range<usize>,
    defined: &mut HashSet<&'a str>,
    budget: &mut Budget,
) -> Result<(), QueryError> {
    budget.charge_program_work(1, Location::Query(range.clone()))?;
    if defined.contains(name) {
        return Err(QueryError::query(
            range.clone(),
            "binding name is reserved or already defined",
        ));
    }
    admit_slots::<&str>(1, budget, range)?;
    defined.insert(name);
    Ok(())
}
fn introduce_name<'a>(
    name: &Located<String>,
    source: &'a str,
    names: &mut Cow<'a, [&'a str]>,
    budget: &mut Budget,
) -> Result<(), QueryError> {
    let copied = if matches!(names, Cow::Borrowed(_)) {
        names.len()
    } else {
        0
    };
    admit_slots::<&str>(copied + 1, budget, &name.range)?;
    names
        .to_mut()
        .push(crate::parser::identifier_text(source, name.range.clone()));
    Ok(())
}
struct FunctionHeader<'a> {
    name: Located<&'a str>,
    range: Range<usize>,
}
fn function_headers<'s, H: ValueReaderHost>(
    source: &'s str,
    host: &mut H,
    budget: &mut Budget,
    nesting: usize,
) -> Result<Vec<FunctionHeader<'s>>, QueryError> {
    budget.charge_program_work(source.len(), Location::Query(0..source.len()))?;
    let mut reader = ArgumentReader::new(source, 0)?;
    let mut headers = Vec::new();
    let mut delimiters = ScanDelimiters::new();
    while !reader.remaining().is_empty() {
        reader.skip_trivia()?;
        if reader.remaining().is_empty() {
            break;
        }
        if reader.protected_span()? {
            continue;
        }
        if delimiters.is_empty() {
            let start = reader.position();
            let mut probe = reader.at(start)?;
            if probe.keyword("fn")
                && let Ok(name) = probe.word()
                && probe.consume('(')
            {
                let mut stack = ScanDelimiters::closing(')');
                while !stack.is_empty() && !probe.remaining().is_empty() {
                    if probe.protected_span()? {
                        continue;
                    }
                    let ch = probe.remaining().chars().next().expect("nonempty source");
                    scan_delimiter(&mut stack, ch, nesting, &probe)?;
                    probe.advance(ch.len_utf8())?;
                }
                probe.expect('-')?;
                probe.expect('>')?;
                let mut signature_reader = Reader {
                    reader: &mut probe,
                    host,
                    budget,
                    nesting,
                    locals: Vec::new(),
                };
                let _signature = signature_reader.signature(0)?;
                probe.expect('{')?;
                stack.push('}');
                while !stack.is_empty() && !probe.remaining().is_empty() {
                    if probe.protected_span()? {
                        continue;
                    }
                    let ch = probe.remaining().chars().next().expect("nonempty source");
                    scan_delimiter(&mut stack, ch, nesting, &probe)?;
                    probe.advance(ch.len_utf8())?;
                }
                if !stack.is_empty() {
                    return Err(probe.error("expected a complete function body"));
                }
                admit_slots::<FunctionHeader<'_>>(1, budget, &name.range)?;
                reader.advance(probe.position() - start)?;
                headers.push(FunctionHeader {
                    name,
                    range: start..reader.position(),
                });
                continue;
            }
        }
        if reader
            .remaining()
            .starts_with(|ch: char| ch.is_ascii_alphabetic() || ch == '_')
        {
            reader.word()?;
            continue;
        }
        let ch = reader.remaining().chars().next().expect("nonempty source");
        match ch {
            '(' | '[' | '{' => {
                if delimiters.len() >= nesting.min(crate::MAX_NESTING) {
                    return Err(reader.error("program nesting limit exceeded"));
                }
                delimiters.push(match ch {
                    '(' => ')',
                    '[' => ']',
                    _ => '}',
                });
            }
            ')' | ']' | '}' => {
                delimiters.pop();
            }
            _ => {}
        }
        reader.advance(ch.len_utf8())?;
    }
    Ok(headers)
}
fn scan_delimiter(
    stack: &mut ScanDelimiters,
    ch: char,
    nesting: usize,
    reader: &ArgumentReader<'_>,
) -> Result<(), QueryError> {
    match ch {
        '(' | '[' | '{' => {
            if stack.len() >= nesting.min(crate::MAX_NESTING) {
                return Err(reader.error("program nesting limit exceeded"));
            }
            stack.push(match ch {
                '(' => ')',
                '[' => ']',
                _ => '}',
            });
        }
        ')' | ']' | '}' if stack.pop() != Some(ch) => {
            return Err(reader.error("mismatched function delimiter"));
        }
        _ => {}
    }
    Ok(())
}

struct ProgramGuard<S, O, E>(Option<ReadProgram<S, O, E>>);
impl<S, O, E> ProgramGuard<S, O, E> {
    fn program(&mut self) -> &mut ReadProgram<S, O, E> {
        self.0.as_mut().expect("retained program")
    }
}
impl<S, O, E> Drop for ProgramGuard<S, O, E> {
    fn drop(&mut self) {
        if let Some(program) = self.0.take() {
            dispose_unresolved_pattern_functions(program.functions);
            dispose_statements(program.statements);
        }
    }
}
fn dispose_statement<S, O, E>(statement: Statement<S, O, E>) {
    let expression = match statement {
        Statement::Bind { expression, .. }
        | Statement::BindFields { expression, .. }
        | Statement::Evaluate { expression, .. } => expression,
        Statement::Parameter { .. } => return,
        Statement::BindPattern { pattern, value } => {
            if let Ok(pattern) = std::sync::Arc::try_unwrap(pattern) {
                crate::dispose_pattern(pattern);
            }
            if let Ok(value) = std::sync::Arc::try_unwrap(value) {
                crate::dispose_native_value(value);
            }
            return;
        }
    };
    crate::dispose_expression(expression);
}
fn dispose_statements<S, O, E>(statements: Vec<Located<Statement<S, O, E>>>) {
    for statement in statements {
        dispose_statement(statement.value);
    }
}
struct ExpressionGuard<S, O, E>(Option<Expression<S, O, E>>);
impl<S, O, E> ExpressionGuard<S, O, E> {
    fn into_expression(mut self) -> Expression<S, O, E> {
        self.0.take().expect("retained expression")
    }
}
impl<S, O, E> Drop for ExpressionGuard<S, O, E> {
    fn drop(&mut self) {
        if let Some(expression) = self.0.take() {
            crate::dispose_expression(expression);
        }
    }
}
