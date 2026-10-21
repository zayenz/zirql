mod program;
pub use program::*;

use std::ops::Range;

use crate::{
    ArgumentReader, Budget, ConstructedValue as V, Expression, ExpressionKind, ExpressionStep,
    Located, Location, MatchClause, NativeMap, NativeMatch, NativeOrder, NativeOrdering,
    NativePaths, ParameterDeclaration, ParameterType, Predicate, PredicateArgument, PredicateKind,
    Projection, QueryError, SortSelector, ValueBinaryOperator as B, ValueConstructor,
    ValueExpression, ValueExpressionKind as K, ValueExtremum, ValueGenerator, ValueReducer,
    ValueUnaryOperator as U,
};

pub type ReadExpression<H> = Expression<
    <H as ValueReaderHost>::Stage,
    <H as ValueReaderHost>::SetOperator,
    <H as ValueReaderHost>::Emitter,
>;

/// One shared pipeline step with this reader's opaque native payload types.
pub type ReadStep<H> = ExpressionStep<
    <H as ValueReaderHost>::Stage,
    <H as ValueReaderHost>::Emitter,
    <H as ValueReaderHost>::SetOperator,
>;

/// How an ordinary field value continues into a pipeline.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FieldPipelineSource {
    /// Read a dotted name as native capture fields before native stages.
    NativeCapture,
    /// Apply common constructed collection stages to the selected value.
    Constructed,
    /// Pass the selected value to the host's declared explicit-source argument.
    ValueArgument,
}

/// Registered domain grammar leaves. Shared readers own values, projection,
/// native composition and Boolean predicate wrappers; hooks consume one leaf.
pub trait ValueReaderHost {
    type Stage;
    type SetOperator;
    type Emitter;

    fn is_binding(&self, name: &str) -> bool;
    /// Whether a known binding contains a constructed value. Unknown and native
    /// bindings keep native pipeline routing. This does not classify local names.
    fn is_constructed_binding(&self, _name: &str) -> bool {
        false
    }
    fn is_stage(&self, name: &str) -> bool;
    fn parse_stage(
        &mut self,
        reader: &mut ArgumentReader<'_>,
        budget: &mut Budget,
    ) -> Result<ReadStep<Self>, QueryError>;
    fn parse_predicate_leaf(
        &mut self,
        reader: &mut ArgumentReader<'_>,
        budget: &mut Budget,
    ) -> Result<Self::Stage, QueryError>;
    /// Parse one domain stage in the current lexical scope. An adapter that
    /// starts a nested common value reader carries these borrowed names into
    /// that reader's binding facts. This exposes names, never caller values.
    /// `nesting` is the remaining allowance for argument expressions. Forward
    /// it with the same budget when reentering a shared reader.
    fn parse_stage_in_scope(
        &mut self,
        reader: &mut ArgumentReader<'_>,
        _locals: &[&str],
        budget: &mut Budget,
        _nesting: usize,
    ) -> Result<ReadStep<Self>, QueryError> {
        self.parse_stage(reader, budget)
    }
    /// Preserve the same lexical names when a domain predicate has value
    /// arguments of its own. Forward the supplied remaining nesting allowance
    /// with the same budget when reentering a shared reader.
    fn parse_predicate_leaf_in_scope(
        &mut self,
        reader: &mut ArgumentReader<'_>,
        _locals: &[&str],
        budget: &mut Budget,
        _nesting: usize,
    ) -> Result<Self::Stage, QueryError> {
        self.parse_predicate_leaf(reader, budget)
    }
    /// Parse an existing domain leaf inside a predicate argument. The optional
    /// pattern selects names while `_` retains the whole subject. Hosts with dynamic
    /// value arguments can retain each through `PredicateArgument::bind_value`.
    /// Argument readers forward `nesting` and the same budget.
    fn parse_predicate_leaf_in_argument(
        &mut self,
        reader: &mut ArgumentReader<'_>,
        _argument: Option<&crate::Pattern>,
        locals: &[&str],
        budget: &mut Budget,
        nesting: usize,
    ) -> Result<Self::Stage, QueryError> {
        self.parse_predicate_leaf_in_scope(reader, locals, budget, nesting)
    }
    /// Admit a host's constructed filter spelling before native predicate parsing.
    /// Return `None` without consuming input to keep the ordinary native filter.
    /// Nested value readers use the supplied remaining `nesting` and budget.
    fn parse_constructed_filter_in_scope(
        &mut self,
        _reader: &mut ArgumentReader<'_>,
        _locals: &[&str],
        _budget: &mut Budget,
        _nesting: usize,
    ) -> Result<Option<ReadStep<Self>>, QueryError> {
        Ok(None)
    }
    /// Recognize a domain predicate leaf without consuming syntax or budget.
    /// Lexical names use value syntax; an explicit leaf call with `(` remains
    /// available in the same scope. Other bodies use shared expression syntax.
    fn is_predicate_leaf(&self, _name: &str) -> bool {
        false
    }
    /// Retain a shared scoped Boolean body as one native predicate leaf.
    /// Expose its bound expression through the host's value-argument hooks.
    /// Explicit native predicate headers also use this hook for their mandatory
    /// pattern check before any Boolean body leaf runs.
    fn predicate_expression(
        &mut self,
        predicate: PredicateArgument<ReadExpression<Self>>,
        range: Range<usize>,
    ) -> Result<Self::Stage, QueryError>
    where
        Self: Sized,
    {
        if let Some(pattern) = predicate.argument {
            crate::dispose_pattern(pattern);
        }
        dispose_native_value(*predicate.body);
        Err(QueryError::query(
            range,
            "predicate expressions are unsupported by this host",
        ))
    }
    /// Map one located edge identifier to an existing typed native primitive.
    /// The shared reader consumes the identifier; this hook consumes no syntax.
    fn parse_path_edge(
        &mut self,
        edge: Located<&str>,
        _budget: &mut Budget,
    ) -> Result<Self::Stage, QueryError> {
        Err(QueryError::query(
            edge.range,
            "native paths are unsupported by this host",
        ))
    }
    /// Retain native values instead of requiring an immediate scalar projection.
    /// Function-bearing frontends opt in; ordinary readers preserve their path.
    fn retains_native_values(&self) -> bool {
        false
    }

    /// Select field continuation for ordinary value expressions, including
    /// grouped fields. The default retains native capture pipelines.
    /// Constructed-only hosts choose [`FieldPipelineSource::Constructed`]. Hosts
    /// using the active prepared argument runner choose [`FieldPipelineSource::ValueArgument`]. This is independent
    /// of whether native results are retained by [`Self::retains_native_values`].
    /// Explicit native readers keep their native field postfixes.
    fn field_pipeline_source(&self) -> FieldPipelineSource {
        FieldPipelineSource::NativeCapture
    }

    /// Read one recognized domain call after its semantic callee name.
    /// The cursor is at `(`. `None` must leave it unchanged so the shared
    /// reader can read an ordinary function call. This hook owns only a domain
    /// signature, such as `enum_member(TypeIdentifier, "member")`, never a
    /// generic expression. Expression-valued arguments reenter
    /// [`read_value_expression_in_scope`] with these locals and remaining nesting.
    fn parse_domain_call(
        &mut self,
        _callee: Located<&str>,
        _reader: &mut ArgumentReader<'_>,
        _locals: &[&str],
        _budget: &mut Budget,
        _nesting: usize,
    ) -> Result<Option<ValueExpression<ReadExpression<Self>>>, QueryError>
    where
        Self: Sized,
    {
        Ok(None)
    }

    fn set_operator(&self, word: &str) -> Option<Self::SetOperator>;
}

/// Common statement prefixes are contextual, preserving legacy saved names.
#[derive(Debug, PartialEq, Eq)]
pub enum CommonStatement {
    Let(Located<String>),
    Report,
    Param(ParameterDeclaration),
}

/// Read one located pattern for any shared binding or value-match position.
/// Native structure is inspected only later through explicit host views.
///
/// # Errors
/// Malformed patterns, invalid scalar literals and parse limits are located.
pub fn read_pattern(
    reader: &mut ArgumentReader<'_>,
    budget: &mut Budget,
    nesting: usize,
) -> Result<crate::Pattern, QueryError> {
    PatternReader {
        reader,
        nesting: nesting
            .min(crate::MAX_NESTING)
            .min(budget.remaining_nesting()),
        budget,
    }
    .pattern(0)
}

/// Read contextual `let pattern =`, leaving the initializer to the frontend.
/// `absent` tests absence, and `present(pattern)` tests a present value, just
/// as in parameters, predicates, generators and value matches.
/// Non-declarations leave the cursor and allowance unchanged.
///
/// # Errors
/// Malformed recognized declarations and exhausted allowances are located.
pub fn read_pattern_let(
    reader: &mut ArgumentReader<'_>,
    budget: &mut Budget,
    nesting: usize,
) -> Result<Option<crate::Pattern>, QueryError> {
    let nesting = nesting
        .min(crate::MAX_NESTING)
        .min(budget.remaining_nesting());
    let mut probe = reader.at(reader.position())?;
    if !probe.keyword("let")
        || !probe
            .remaining()
            .starts_with(|ch: char| ch.is_whitespace() || matches!(ch, '#' | '%' | '/'))
    {
        return Ok(None);
    }
    probe.skip_trivia()?;
    let scan_start = probe.position();
    let mut stack = ScanDelimiters::new();
    let mut declaration = false;
    while let Some(ch) = probe.remaining().chars().next() {
        if probe.protected_span()? {
            continue;
        }
        match ch {
            '(' | '[' | '{' => {
                if stack.len() >= nesting.min(crate::MAX_NESTING) {
                    return Err(probe.error("pattern nesting limit exceeded"));
                }
                stack.push(match ch {
                    '(' => ')',
                    '[' => ']',
                    _ => '}',
                });
            }
            ')' | ']' | '}' if !stack.is_empty() => {
                if stack.pop() != Some(ch) {
                    return Err(probe.error("mismatched pattern delimiter"));
                }
            }
            ';' | '}' if stack.is_empty() => break,
            '=' if stack.is_empty() => {
                declaration =
                    !(probe.remaining().starts_with("==") || probe.remaining().starts_with("=>"));
                break;
            }
            _ => {}
        }
        probe.advance(ch.len_utf8())?;
    }
    if !declaration {
        return Ok(None);
    }
    budget.charge(
        probe.position() - scan_start,
        Location::Query(scan_start..probe.position()),
    )?;
    reader.keyword("let");
    let pattern = PatternGuard(Some(
        PatternReader {
            reader,
            budget,
            nesting: nesting.min(crate::MAX_NESTING),
        }
        .pattern(0)?,
    ));
    reader.expect('=')?;
    Ok(Some(pattern.into_pattern()))
}

/// Pattern parameters retain their whole type and original pattern coordinates.
#[derive(Debug, PartialEq)]
pub struct UnresolvedPatternFunctionParameter {
    pub pattern: crate::Pattern,
    pub declaration: Located<SignatureType>,
}
/// Reusable parsed pattern function, with local bindings in its lexical block.
#[derive(Debug, PartialEq)]
pub struct UnresolvedPatternFunctionDeclaration<Q> {
    pub name: Located<String>,
    pub parameters: Vec<Located<UnresolvedPatternFunctionParameter>>,
    pub result: Located<SignatureType>,
    pub value: ValueExpression<Q>,
}
/// Request-owned types borrowing the original parameter patterns.
#[derive(Debug, PartialEq)]
pub struct ResolvedPatternFunctionSignature<'a, K> {
    pub name: &'a Located<String>,
    pub parameters: Vec<Located<crate::PatternParameterRef<'a, K>>>,
    pub result: Located<crate::DeclaredType<K>>,
}
impl<Q> UnresolvedPatternFunctionDeclaration<Q> {
    /// Resolve small headers against the current host without copying bodies or patterns.
    ///
    /// # Errors
    /// Invalid types, host refusals and exhausted allowances retain type ranges.
    pub fn resolve_signature<K>(
        &self,
        budget: &mut Budget,
        mut resolve: impl FnMut(
            Located<&str>,
            &mut Budget,
        ) -> Result<crate::DeclaredType<K>, QueryError>,
    ) -> Result<ResolvedPatternFunctionSignature<'_, K>, QueryError> {
        let mut parameters = Vec::new();
        for parameter in &self.parameters {
            budget.charge_results(1, Location::Query(parameter.range.clone()))?;
            let p = &parameter.value;
            parameters.push(Located::new(
                crate::PatternParameterRef {
                    pattern: &p.pattern,
                    declaration: p.declaration.value.resolve(
                        budget,
                        &p.declaration.range,
                        &mut resolve,
                    )?,
                },
                parameter.range.clone(),
            ));
        }
        Ok(ResolvedPatternFunctionSignature {
            name: &self.name,
            parameters,
            result: Located::new(
                self.result
                    .value
                    .resolve(budget, &self.result.range, &mut resolve)?,
                self.result.range.clone(),
            ),
        })
    }

    /// Borrow the original body with a request-resolved pattern header.
    pub fn with_signature<'a, K>(
        &'a self,
        signature: &'a ResolvedPatternFunctionSignature<'a, K>,
    ) -> crate::PatternFunctionDeclarationRef<'a, Q, K> {
        crate::PatternFunctionDeclarationRef {
            name: signature.name,
            parameters: &signature.parameters,
            result: &signature.result,
            value: &self.value,
        }
    }
}

/// Common type syntax before a host has resolved nominal/native names.
/// Nested locations and named leaves retain their original query coordinates.
#[derive(Debug, PartialEq)]
pub enum SignatureType {
    Value,
    Boolean,
    Integer,
    Float,
    String,
    Named(Located<String>),
    Optional(Box<Located<Self>>),
    Sequence(Box<Located<Self>>),
    Tuple(Vec<Located<Self>>),
    Record(Vec<(Located<String>, Located<Self>)>),
    Dictionary(Box<Located<Self>>, Box<Located<Self>>),
    Set(Box<Located<Self>>),
    Keyed(Box<Located<Self>>),
    Queue(Box<Located<Self>>),
    IdentitySet(Box<Located<Self>>),
}
#[derive(Debug, PartialEq)]
pub struct UnresolvedFunctionParameter {
    pub name: Located<String>,
    pub declaration: SignatureType,
}
/// Parsed body trees and signature syntax, independent of model ownership.
#[derive(Debug, PartialEq)]
pub struct UnresolvedFunctionDeclaration<Q> {
    pub name: Located<String>,
    pub parameters: Vec<Located<UnresolvedFunctionParameter>>,
    pub result: Located<SignatureType>,
    pub bindings: Vec<(Located<String>, ValueExpression<Q>)>,
    pub value: ValueExpression<Q>,
}
/// Small request-owned metadata, separate from immutable parsed body trees.
#[derive(Debug, PartialEq)]
pub struct ResolvedFunctionSignature<K> {
    pub name: Located<String>,
    pub parameters: Vec<Located<crate::FunctionParameter<K>>>,
    pub result: Located<crate::DeclaredType<K>>,
}
impl<Q> UnresolvedFunctionDeclaration<Q> {
    /// Resolve named leaves against the current host owner. Set element names
    /// must resolve to real nominal identities, never a native kind tag.
    ///
    /// # Errors
    /// Host refusals retain named-leaf ranges; unsupported sets and limits fail.
    pub fn resolve_signature<K>(
        &self,
        budget: &mut Budget,
        mut resolve: impl FnMut(
            Located<&str>,
            &mut Budget,
        ) -> Result<crate::DeclaredType<K>, QueryError>,
    ) -> Result<ResolvedFunctionSignature<K>, QueryError> {
        let mut parameters = Vec::new();
        for parameter in &self.parameters {
            let p = &parameter.value;
            budget.charge(
                p.name.value.len() + 1,
                Location::Query(p.name.range.clone()),
            )?;
            budget.charge_results(1, Location::Query(parameter.range.clone()))?;
            parameters.push(Located::new(
                crate::FunctionParameter {
                    name: p.name.clone(),
                    declaration: p
                        .declaration
                        .resolve(budget, &parameter.range, &mut resolve)?,
                },
                parameter.range.clone(),
            ));
        }
        budget.charge(
            self.name.value.len() + 1,
            Location::Query(self.name.range.clone()),
        )?;
        Ok(ResolvedFunctionSignature {
            name: self.name.clone(),
            parameters,
            result: Located::new(
                self.result
                    .value
                    .resolve(budget, &self.result.range, &mut resolve)?,
                self.result.range.clone(),
            ),
        })
    }

    /// Borrow this body's trees with a header resolved for the current request.
    pub fn with_signature<'a, K>(
        &'a self,
        signature: &'a ResolvedFunctionSignature<K>,
    ) -> crate::FunctionDeclarationRef<'a, Q, K> {
        crate::FunctionDeclarationRef {
            name: &signature.name,
            parameters: &signature.parameters,
            result: &signature.result,
            bindings: &self.bindings,
            value: &self.value,
        }
    }
}
impl SignatureType {
    pub(crate) fn resolve<K>(
        &self,
        budget: &mut Budget,
        range: &Range<usize>,
        resolve: &mut impl FnMut(
            Located<&str>,
            &mut Budget,
        ) -> Result<crate::DeclaredType<K>, QueryError>,
    ) -> Result<crate::DeclaredType<K>, QueryError> {
        use crate::{DeclaredType as D, SetElementType as S};
        let location = Location::Query(range.clone());
        budget.within_evaluation(1, location.clone(), |budget| {
            budget.charge(1, location.clone())?;
            budget.charge_results(1, location.clone())?;
            let child = |value: &Located<Self>, budget: &mut Budget, resolve: &mut _| {
                value.value.resolve(budget, &value.range, resolve)
            };
            Ok(match self {
                Self::Value => D::Value,
                Self::Boolean => D::Boolean,
                Self::Integer => D::Integer,
                Self::Float => D::Float,
                Self::String => D::String,
                Self::Named(name) => resolve(
                    Located::new(name.value.as_str(), name.range.clone()),
                    budget,
                )?,
                Self::Optional(value) => D::Optional(Box::new(child(value, budget, resolve)?)),
                Self::Sequence(value) => D::Sequence(Box::new(child(value, budget, resolve)?)),
                Self::Keyed(value) => D::Keyed(Box::new(child(value, budget, resolve)?)),
                Self::Queue(value) => D::Queue(Box::new(child(value, budget, resolve)?)),
                Self::IdentitySet(value) => match child(value, budget, resolve)? {
                    D::Native(kind) => D::IdentitySet(kind),
                    _ => {
                        return Err(QueryError::query(
                            value.range.clone(),
                            "identity_set requires a native element type",
                        ));
                    }
                },
                Self::Dictionary(key, value) => D::Dictionary(
                    Box::new(child(key, budget, resolve)?),
                    Box::new(child(value, budget, resolve)?),
                ),
                Self::Tuple(values) => D::Tuple(
                    values
                        .iter()
                        .map(|value| child(value, budget, resolve))
                        .collect::<Result<_, _>>()?,
                ),
                Self::Record(fields) => {
                    let mut resolved = Vec::new();
                    for (label, value) in fields {
                        budget.charge(label.value.len() + 1, location.clone())?;
                        resolved.push((label.value.clone(), child(value, budget, resolve)?));
                    }
                    D::Record(resolved)
                }
                Self::Set(value) => D::Set(match child(value, budget, resolve)? {
                    D::Value => S::Value,
                    D::Boolean => S::Boolean,
                    D::Integer => S::Integer,
                    D::Float => S::Float,
                    D::String => S::String,
                    D::Member(id) => S::Member(id),
                    _ => {
                        return Err(QueryError::query(
                            value.range.clone(),
                            "unsupported set element type",
                        ));
                    }
                }),
            })
        })
    }
}

/// Recognize `let name =`, `report expression` and `param name: type` without
/// reserving bare names. A parameter declaration leaves its required `;` for
/// the host frontend, which introduces the name at this written position.
/// Failure to recognize leaves the original cursor unchanged.
///
/// # Errors
/// Malformed declarations, unsupported parameter types, invalid source boundaries
/// or exhausted name-metadata allowances fail at the original query range.
pub fn read_statement_prefix(
    reader: &mut ArgumentReader<'_>,
    budget: &mut Budget,
) -> Result<Option<CommonStatement>, QueryError> {
    let mut probe = reader.at(reader.position())?;
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
    } else if word.value == "param" {
        probe.skip_whitespace();
        if probe.remaining().is_empty() || probe.remaining().starts_with(['=', '|', ';', '(']) {
            return Ok(None);
        }
        let name = probe.word()?;
        if !probe.consume(':') {
            return Ok(None);
        }
        let ty = probe.word()?;
        let value = match ty.value {
            "int" => ParameterType::Int,
            "string" => ParameterType::String,
            "bool" => ParameterType::Bool,
            _ => {
                return Err(QueryError::query(
                    ty.range,
                    "parameter type must be int, string or bool",
                ));
            }
        };
        budget.charge(
            probe.position() - word.range.start,
            Location::Query(name.range.clone()),
        )?;
        budget.charge_results(1, Location::Query(name.range.clone()))?;
        CommonStatement::Param(ParameterDeclaration {
            name: Located::new(name.value.to_owned(), name.range),
            ty: Located::new(value, ty.range),
        })
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

/// Read a contextual `fn name(args) -> Type { lets; value }` declaration.
/// A non-declaration leaves the cursor and allowance unchanged. The resolver
/// recognizes only host/nominal names; all common type grammar stays here.
/// Signatures can be collected before preparing any body.
///
/// # Errors
/// Malformed signatures/bodies, unknown host types and limits retain byte ranges.
pub fn read_function_declaration<H: ValueReaderHost, T>(
    reader: &mut ArgumentReader<'_>,
    host: &mut H,
    budget: &mut Budget,
    nesting: usize,
    mut resolve_type: impl FnMut(
        Located<&str>,
        &mut Budget,
    ) -> Result<crate::DeclaredType<T>, QueryError>,
) -> Result<Option<crate::FunctionDeclaration<ReadExpression<H>, T>>, QueryError> {
    let Some(function) = read_unresolved_function_declaration(reader, host, budget, nesting)?
    else {
        return Ok(None);
    };
    let mut guard = FunctionGuard(Some(function));
    let signature = guard
        .0
        .as_ref()
        .expect("retained function")
        .resolve_signature(budget, &mut resolve_type)?;
    let function = guard.0.take().expect("retained function");
    Ok(Some(crate::FunctionDeclaration {
        name: signature.name,
        parameters: signature.parameters,
        result: signature.result,
        bindings: function.bindings,
        value: function.value,
    }))
}

/// Read common function declarations while retaining located host/nominal
/// names. Resolve their small headers once against the current request's owner;
/// expression trees remain reusable. Domain calls use
/// [`ValueReaderHost::parse_domain_call`].
///
/// # Errors
/// Malformed declarations, unsupported type structure and limits are located.
pub fn read_unresolved_function_declaration<H: ValueReaderHost>(
    reader: &mut ArgumentReader<'_>,
    host: &mut H,
    budget: &mut Budget,
    nesting: usize,
) -> Result<Option<UnresolvedFunctionDeclaration<ReadExpression<H>>>, QueryError> {
    let mut probe = reader.at(reader.position())?;
    if !probe.keyword("fn") || probe.word().is_err() || !probe.consume('(') {
        return Ok(None);
    }
    let mut parser = Reader {
        reader,
        host,
        nesting: nesting
            .min(crate::MAX_NESTING)
            .min(budget.remaining_nesting()),
        budget,
        locals: Vec::new(),
    };
    parser.reader.keyword("fn");
    let name = parser.name()?;
    parser.reader.expect('(')?;
    let mut parameters = Vec::new();
    if !parser.reader.consume(')') {
        loop {
            let parameter = parser.name()?;
            parser.reader.expect(':')?;
            let declaration = parser.signature(0)?;
            let range = parameter.range.start..parser.reader.position();
            parser.slots::<UnresolvedFunctionParameter>(1)?;
            parser.slots::<&str>(1)?;
            parser.locals.push(crate::parser::identifier_text(
                parser.reader.source(),
                parameter.range.clone(),
            ));
            parameters.push(Located::new(
                UnresolvedFunctionParameter {
                    name: parameter,
                    declaration: declaration.value,
                },
                range,
            ));
            if !parser.reader.consume(',') {
                parser.reader.expect(')')?;
                break;
            }
        }
    }
    parser.reader.expect('-')?;
    parser.reader.expect('>')?;
    let result = parser.signature(0)?;
    parser.reader.expect('{')?;
    let mut guard = FunctionGuard(Some(UnresolvedFunctionDeclaration {
        name,
        parameters,
        result,
        bindings: Vec::new(),
        value: Located::new(K::Literal(V::Absent), 0..0),
    }));
    while let Some(CommonStatement::Let(name)) = {
        let mut probe = parser.reader.at(parser.reader.position())?;
        if probe.keyword("let") && probe.word().is_ok() && probe.consume('=') {
            read_statement_prefix(parser.reader, parser.budget)?
        } else {
            None
        }
    } {
        let value = ValueGuard::new(parser.function_value()?);
        parser.reader.expect(';')?;
        parser.slots::<(Located<String>, ValueExpression<ReadExpression<H>>)>(1)?;
        parser.slots::<&str>(1)?;
        parser.locals.push(crate::parser::identifier_text(
            parser.reader.source(),
            name.range.clone(),
        ));
        guard
            .0
            .as_mut()
            .expect("retained function")
            .bindings
            .push((name, value.into_value()));
    }
    let value = ValueGuard::new(parser.function_value()?);
    parser.reader.expect('}')?;
    let mut function = guard.0.take().expect("retained function");
    function.value = value.into_value();
    Ok(Some(function))
}

struct FunctionGuard<S, O, E>(Option<UnresolvedFunctionDeclaration<Expression<S, O, E>>>);
impl<S, O, E> Drop for FunctionGuard<S, O, E> {
    fn drop(&mut self) {
        if let Some(function) = self.0.take() {
            for (_, value) in function.bindings {
                dispose_native_value(value);
            }
            dispose_native_value(function.value);
        }
    }
}

/// Release parsed function bodies through the same iterative value disposal
/// used by the common reader, including native expression wrappers.
pub fn dispose_unresolved_functions<S, O, E>(
    functions: Vec<UnresolvedFunctionDeclaration<Expression<S, O, E>>>,
) {
    for function in functions {
        drop(FunctionGuard(Some(function)));
    }
}

/// Read a contextual pattern function using the shared signature and value grammar.
/// Local lets lower to `Block`; the original name-only readers remain available.
///
/// # Errors
/// Malformed declarations, patterns, expressions and limits retain source ranges.
pub fn read_unresolved_pattern_function_declaration<H: ValueReaderHost>(
    reader: &mut ArgumentReader<'_>,
    host: &mut H,
    budget: &mut Budget,
    nesting: usize,
) -> Result<Option<UnresolvedPatternFunctionDeclaration<ReadExpression<H>>>, QueryError> {
    let mut probe = reader.at(reader.position())?;
    if !probe.keyword("fn") || probe.word().is_err() || !probe.consume('(') {
        return Ok(None);
    }
    let mut parser = Reader {
        reader,
        host,
        nesting: nesting
            .min(crate::MAX_NESTING)
            .min(budget.remaining_nesting()),
        budget,
        locals: Vec::new(),
    };
    parser.reader.keyword("fn");
    let name = parser.name()?;
    parser.reader.expect('(')?;
    let mut guard = PatternFunctionGuard(Some(UnresolvedPatternFunctionDeclaration {
        name,
        parameters: Vec::new(),
        result: Located::new(SignatureType::Boolean, 0..0),
        value: Located::new(K::Literal(V::Absent), 0..0),
    }));
    if !parser.reader.consume(')') {
        loop {
            let pattern = PatternGuard(Some(parser.pattern(0)?));
            parser.reader.expect(':')?;
            let declaration = parser.signature(0)?;
            parser.add_pattern_locals(pattern.pattern())?;
            parser.slots::<Located<UnresolvedPatternFunctionParameter>>(1)?;
            let range = pattern.pattern().range.start..declaration.range.end;
            guard
                .0
                .as_mut()
                .expect("retained function")
                .parameters
                .push(Located::new(
                    UnresolvedPatternFunctionParameter {
                        pattern: pattern.into_pattern(),
                        declaration,
                    },
                    range,
                ));
            if !parser.reader.consume(',') {
                parser.reader.expect(')')?;
                break;
            }
        }
    }
    parser.reader.expect('-')?;
    parser.reader.expect('>')?;
    guard.0.as_mut().expect("retained function").result = parser.signature(0)?;
    guard.0.as_mut().expect("retained function").value = parser.block(0)?;
    Ok(guard.0.take())
}

/// Read and resolve a pattern function for a host with request-independent kinds.
///
/// # Errors
/// The unresolved reader's errors and located type-resolution refusals propagate.
pub fn read_pattern_function_declaration<H: ValueReaderHost, T>(
    reader: &mut ArgumentReader<'_>,
    host: &mut H,
    budget: &mut Budget,
    nesting: usize,
    resolve_type: impl FnMut(Located<&str>, &mut Budget) -> Result<crate::DeclaredType<T>, QueryError>,
) -> Result<Option<crate::PatternFunctionDeclaration<ReadExpression<H>, T>>, QueryError> {
    let Some(function) =
        read_unresolved_pattern_function_declaration(reader, host, budget, nesting)?
    else {
        return Ok(None);
    };
    let mut guard = PatternFunctionGuard(Some(function));
    let signature = guard
        .0
        .as_ref()
        .expect("retained function")
        .resolve_signature(budget, resolve_type)?;
    budget.charge_results(
        signature.parameters.len(),
        Location::Query(signature.name.range.clone()),
    )?;
    let declarations = signature
        .parameters
        .into_iter()
        .map(|p| p.value.declaration)
        .collect::<Vec<_>>();
    let result = signature.result;
    let function = guard.0.take().expect("retained function");
    Ok(Some(crate::PatternFunctionDeclaration {
        name: function.name,
        parameters: function
            .parameters
            .into_iter()
            .zip(declarations)
            .map(|(p, declaration)| {
                Located::new(
                    crate::PatternParameter {
                        pattern: p.value.pattern,
                        declaration,
                    },
                    p.range,
                )
            })
            .collect(),
        result,
        value: function.value,
    }))
}

struct PatternFunctionGuard<S, O, E>(
    Option<UnresolvedPatternFunctionDeclaration<Expression<S, O, E>>>,
);
impl<S, O, E> Drop for PatternFunctionGuard<S, O, E> {
    fn drop(&mut self) {
        if let Some(function) = self.0.take() {
            for parameter in function.parameters {
                crate::dispose_pattern(parameter.value.pattern);
            }
            dispose_native_value(function.value);
        }
    }
}
/// Release reusable pattern function bodies and patterns iteratively.
pub fn dispose_unresolved_pattern_functions<S, O, E>(
    functions: Vec<UnresolvedPatternFunctionDeclaration<Expression<S, O, E>>>,
) {
    for function in functions {
        drop(PatternFunctionGuard(Some(function)));
    }
}

/// Read a value using original byte offsets and one parsing allowance.
/// Native operands remain normalized trees and execute only through the runner.
/// The caller owns a successful tree; use [`dispose_native_value`] to release
/// shared wrappers and flat native operands without recursive drop.
///
/// # Errors
/// Unsupported syntax, invalid literals, nesting and allocation refusals are located.
pub fn read_value_expression<H: ValueReaderHost>(
    reader: &mut ArgumentReader<'_>,
    host: &mut H,
    budget: &mut Budget,
    nesting: usize,
) -> Result<ValueExpression<ReadExpression<H>>, QueryError> {
    read_value_expression_in_scope(reader, host, &[], budget, nesting)
}

/// Read a shared expression with borrowed lexical names from a host argument.
/// Names classify values before domain stage names; returned trees own names
/// and host payloads. The caller supplies one request budget and the remaining
/// nesting allowance. Reentry shares the active budget depth and cannot reset
/// an enclosing reader's allowance. Contextual effects and whole-tree admission
/// remain host rules.
///
/// # Errors
/// Malformed syntax, literals and exhausted allowances retain original byte ranges.
pub fn read_value_expression_in_scope<H: ValueReaderHost>(
    reader: &mut ArgumentReader<'_>,
    host: &mut H,
    locals: &[&str],
    budget: &mut Budget,
    nesting: usize,
) -> Result<ValueExpression<ReadExpression<H>>, QueryError> {
    let locals = copy_reader_locals(locals, budget, reader.position())?;
    Reader {
        reader,
        host,
        nesting: nesting
            .min(crate::MAX_NESTING)
            .min(budget.remaining_nesting()),
        budget,
        locals,
    }
    .expression(0, 0)
}

/// Read ordinary call arguments after the callee and opening `(` were consumed.
/// Uses the same lexical scope, source ranges, parser and budget as value reading.
/// Leaves any following pipeline suffix for the calling stage reader.
///
/// # Errors
/// Invalid arguments, separators and exhausted allowances retain their ranges.
pub fn read_function_call_in_scope<H: ValueReaderHost>(
    reader: &mut ArgumentReader<'_>,
    name: Located<String>,
    host: &mut H,
    locals: &[&str],
    budget: &mut Budget,
    nesting: usize,
) -> Result<ValueExpression<ReadExpression<H>>, QueryError> {
    let locals = copy_reader_locals(locals, budget, reader.position())?;
    Reader {
        reader,
        host,
        nesting: nesting
            .min(crate::MAX_NESTING)
            .min(budget.remaining_nesting()),
        budget,
        locals,
    }
    .call_arguments(name, 0)
}

/// Whether a name is one of the fixed common value calls. Native operation
/// names, native ordering argument shapes and declared functions remain host facts.
pub fn is_common_value_call(name: &str) -> bool {
    matches!(
        name,
        "lookup"
            | "has_key"
            | "group_by"
            | "windows"
            | "runs_by"
            | "align"
            | "sort"
            | "sort_by"
            | "min"
            | "max"
            | "min_by"
            | "max_by"
            | "zip_exact"
            | "contains"
            | "starts_with"
            | "ends_with"
            | "parse_int"
            | "parse_float"
            | "to_string"
            | "require"
            | "length"
            | "sum"
            | "forall"
            | "exists"
            | "occurs"
            | "absent"
            | "entries"
            | "index_set"
            | "one"
    )
}

/// Whether an unquoted name has the common ASCII identifier spelling.
/// Keywords are contextual: hosts apply their binding or input-name policy
/// separately. Single-quoted identities must not be checked as unquoted text.
pub fn is_unquoted_binding_name(name: &str) -> bool {
    let mut bytes = name.bytes();
    bytes
        .next()
        .is_some_and(|b| b.is_ascii_alphabetic() || b == b'_')
        && bytes.all(|b| b.is_ascii_alphanumeric() || b == b'_')
}

/// Common syntax words. This is useful for external names such as CSV tables;
/// it is not a universal reserved-name set for lexical bindings.
pub fn is_common_keyword(name: &str) -> bool {
    matches!(
        name,
        "_" | "input"
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

/// Common names that conflict with function declarations. Lexical bindings
/// can still use contextual operation names; native operations remain host facts.
pub fn is_common_operation_name(name: &str) -> bool {
    matches!(
        name,
        "queue"
            | "pop_front"
            | "push_back_all"
            | "identity_set"
            | "insert"
            | "fold"
            | "scan"
            | "one"
            | "occurs"
            | "absent"
            | "lookup"
            | "has_key"
            | "index_set"
            | "entries"
            | "zip_exact"
            | "contains"
            | "filter"
            | "fixpoint"
            | "where_exists"
            | "starts_with"
            | "ends_with"
            | "parse_int"
            | "parse_float"
            | "to_string"
            | "length"
            | "sum"
            | "forall"
            | "exists"
            | "sort"
            | "sort_by"
            | "min"
            | "max"
            | "min_by"
            | "max_by"
            | "group_by"
            | "windows"
            | "runs_by"
            | "align"
            | "require"
            | "project"
            | "match"
            | "paths"
            | "map_by"
            | "default"
            | "if"
    )
}

/// The syntax of one constructed collection suffix, before reading its payload.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValueSuffixKind {
    Filter,
    Map,
    FlatMap,
    Windows,
    RunsBy,
    Count,
    Head,
    Tail,
    Reverse,
    Check,
    Sort,
    Unique,
    Tally,
}
impl ValueSuffixKind {
    /// Classify a common suffix spelling. Native-only stages return `None`.
    pub fn named(name: &str) -> Option<Self> {
        Some(match name {
            "filter" => Self::Filter,
            "map" => Self::Map,
            "flat_map" => Self::FlatMap,
            "windows" => Self::Windows,
            "runs_by" => Self::RunsBy,
            "count" => Self::Count,
            "head" => Self::Head,
            "tail" => Self::Tail,
            "reverse" => Self::Reverse,
            "check" => Self::Check,
            "sort" => Self::Sort,
            "unique" => Self::Unique,
            "tally" => Self::Tally,
            _ => return None,
        })
    }
    pub fn is_terminal(self) -> bool {
        self == Self::Tally
    }
    /// Lower a suffix without arguments. Argument-bearing kinds need their
    /// parsed payload and return `None`.
    pub fn without_arguments<Q>(self) -> Option<ValueSuffix<Q>> {
        Some(match self {
            Self::Count => ValueSuffix::Count,
            Self::Reverse => ValueSuffix::Reverse,
            Self::Sort => ValueSuffix::Sort,
            Self::Unique => ValueSuffix::Unique,
            Self::Tally => ValueSuffix::Tally,
            _ => return None,
        })
    }
}

/// Recognize collection forms that require value reading: sets, expression
/// keys, indexed arrays, comprehensions and postfix access. Plain JSON-shaped
/// arrays and records are not sufficient on their own, so a host can retain its
/// legacy literal/interpolation policy. This allocation-free prediction is
/// bounded by the source length and the common nesting ceiling.
pub fn has_collection_syntax(source: &str) -> bool {
    let mut reader = ArgumentReader::new(source, 0).expect("source boundary");
    let mut containers = [b' '; crate::MAX_NESTING];
    let mut depth = 0usize;
    while !reader.remaining().is_empty() {
        reader.skip_whitespace();
        let Some(character) = reader.remaining().chars().next() else {
            break;
        };
        let start = reader.position();
        reader
            .advance(character.len_utf8())
            .expect("character boundary");
        match character {
            '"' | '\'' | '#' | '%' | '/'
                if character != '/' || reader.remaining().starts_with('*') =>
            {
                let mut span = ArgumentReader::new(source, start).expect("source boundary");
                if span.protected_span().is_err() {
                    return true;
                }
                reader
                    .advance(span.position() - reader.position())
                    .expect("span boundary");
                if character == '\'' && depth > 0 && containers[depth - 1] == b'k' {
                    return true;
                }
                if character == '"' && depth > 0 && containers[depth - 1] == b'k' {
                    reader.skip_whitespace();
                    if !reader.consume(':') {
                        return true;
                    }
                    containers[depth - 1] = b'{';
                }
            }
            '{' | '[' | '(' => {
                if depth == containers.len() {
                    return true;
                }
                containers[depth] = if character == '{' {
                    b'k'
                } else {
                    character as u8
                };
                depth += 1;
            }
            '}' | ']' | ')' => {
                depth = depth.saturating_sub(1);
                if character == '}' {
                    reader.skip_whitespace();
                    if reader.remaining().starts_with(['[', '.']) {
                        return true;
                    }
                }
                if character == ']' {
                    let mut probe =
                        ArgumentReader::new(source, reader.position()).expect("source boundary");
                    if probe.keyword("axes") && probe.consume('(') {
                        return true;
                    }
                }
            }
            ':' if depth > 0 && containers[depth - 1] == b'[' => return true,
            ',' if depth > 0 && containers[depth - 1] == b'{' => containers[depth - 1] = b'k',
            _ if depth > 0 && containers[depth - 1] == b'k' => return true,
            ch if ch.is_ascii_alphabetic() || ch == '_' => {
                let mut probe = ArgumentReader::new(source, start).expect("source boundary");
                let word = probe.word().expect("identifier start");
                reader
                    .advance(probe.position() - reader.position())
                    .expect("word boundary");
                if word.value == "set" && probe.consume('<') {
                    return true;
                }
                if word.value == "for" && depth > 0 && probe.word().is_ok() && probe.keyword("in") {
                    return true;
                }
            }
            _ => {}
        }
    }
    false
}

// Recognize `for binder in` at this array's own level. `[for]` stays a literal.
// The allocation-free admission scan ignores quoted strings and comments, so
// legacy names and text containing `for` retain their old parsing allowance.
/// Predict whether a source contains common value/program syntax before a
/// host allocates its own tokens. The predicate supplies native calls that also
/// require common value parsing. This allocation-free admission prediction
/// does not consume the request budget; recognized parsing charges that budget.
/// Malformed recognized forms remain on the common path for located diagnosis.
pub fn has_common_expression_syntax(source: &str, is_native_call: impl Fn(&str) -> bool) -> bool {
    if has_collection_syntax(source) {
        return true;
    }
    let bytes = source.as_bytes();
    let mut cursor = 0;
    let mut arrays = 0usize;
    let mut records = 0usize;
    let mut groups = 0usize;
    let mut statement_start = true;
    let mut previous_for = false;
    let mut before_previous_for = false;
    while cursor < bytes.len() {
        match bytes[cursor] {
            b'"' | b'\'' | b'#' | b'%' | b'/'
                if bytes[cursor] != b'/' || source[cursor..].starts_with("/*") =>
            {
                let comment = matches!(bytes[cursor], b'#' | b'%' | b'/');
                let mut span = ArgumentReader::new(source, cursor).expect("source boundary");
                if span.protected_span().is_err() {
                    return true;
                }
                cursor = span.position();
                if !comment {
                    previous_for = false;
                    before_previous_for = false;
                    statement_start = false;
                }
            }
            byte if byte.is_ascii_whitespace() => cursor += 1,
            b'a'..=b'z' | b'A'..=b'Z' | b'_' => {
                let start = cursor;
                cursor += 1;
                while bytes
                    .get(cursor)
                    .is_some_and(|b| b.is_ascii_alphanumeric() || *b == b'_')
                {
                    cursor += 1;
                }
                let word = &source[start..cursor];
                let mut lookahead =
                    ArgumentReader::new(source, cursor).expect("source byte boundary");
                lookahead.skip_whitespace();
                let rest = lookahead.remaining();
                if matches!(word, "queue" | "identity_set") && rest.starts_with('<') {
                    return true;
                }
                if is_native_call(word) && rest.starts_with('(')
                    || admission_ordering_call(source, start)
                    || is_common_value_call(word)
                        && !matches!(
                            word,
                            "sort" | "sort_by" | "min" | "max" | "min_by" | "max_by"
                        )
                        && rest.starts_with('(')
                    || word == "project" && rest.starts_with('(')
                {
                    return true;
                }
                if statement_start
                    && word == "report"
                    && !rest.is_empty()
                    && !rest.starts_with(['=', '|', ';'])
                {
                    return true;
                }
                if statement_start && word == "fn" {
                    let mut probe =
                        ArgumentReader::new(source, cursor).expect("source byte boundary");
                    if probe.word().is_ok() && probe.consume('(') {
                        return true;
                    }
                }
                if statement_start && word == "param" {
                    let mut probe =
                        ArgumentReader::new(source, cursor).expect("source byte boundary");
                    if probe.word().is_ok() && probe.consume(':') {
                        return true;
                    }
                }
                if statement_start && word == "let" {
                    let mut probe =
                        ArgumentReader::new(source, cursor).expect("source byte boundary");
                    if probe.word().is_ok() && probe.consume('=') {
                        return true;
                    }
                }
                if is_common_control_call(word) && rest.starts_with('(') {
                    return true;
                }

                if word == "default"
                    && !rest.starts_with([':', '='])
                    && source[..start]
                        .trim_end()
                        .chars()
                        .next_back()
                        .is_some_and(|c| {
                            !matches!(c, '[' | '(' | '{' | ',' | ':' | ';' | '=' | '|')
                        })
                {
                    return true;
                }
                if arrays > 0 && before_previous_for && word == "in" {
                    return true;
                }
                before_previous_for = previous_for;
                previous_for = word == "for";
                statement_start = false;
            }
            byte => {
                if byte == b'[' {
                    arrays += 1;
                } else if byte == b']' {
                    arrays = arrays.saturating_sub(1);
                }
                if byte == b'{' {
                    records += 1;
                } else if byte == b'}' {
                    records = records.saturating_sub(1);
                }
                if byte == b'(' {
                    groups += 1;
                } else if byte == b')' {
                    groups = groups.saturating_sub(1);
                }
                statement_start = byte == b';' && arrays == 0 && records == 0 && groups == 0;
                previous_for = false;
                before_previous_for = false;
                cursor += 1;
            }
        }
    }
    false
}

fn admission_ordering_call(source: &str, position: usize) -> bool {
    let reader = ArgumentReader::new(source, position).expect("source boundary");
    let mut budget = Budget::for_program(crate::ProgramLimits {
        work: usize::MAX,
        results: usize::MAX,
        nesting: crate::MAX_NESTING,
        stream_items: usize::MAX,
    });
    probe_ordering_call(&reader, &mut budget, crate::MAX_NESTING).unwrap_or(true)
}

// Small lookahead reads use the existing reader's protected spans, spelling and
// boundaries. They retain source slices and allocate no token stream.
fn syntax_token<'s>(reader: &mut ArgumentReader<'s>) -> Option<Located<&'s str>> {
    reader.skip_trivia().ok()?;
    let start = reader.position();
    let first = reader.remaining().chars().next()?;
    if first.is_ascii_alphabetic() || first == '_' || first == '\'' {
        return reader.word().ok();
    }
    if first == '"' {
        reader.protected_span().ok()?;
    } else if first.is_ascii_digit() {
        reader.number_text().ok()?;
    } else {
        reader.advance(first.len_utf8()).ok()?;
    }
    Some(Located::new(
        &reader.source()[start..reader.position()],
        start..reader.position(),
    ))
}

fn expression_boundary(reader: &ArgumentReader<'_>) -> usize {
    let mut probe = reader.at(reader.position()).expect("reader boundary");
    let mut depth = 0usize;
    while let Some(token) = syntax_token(&mut probe) {
        match token.value {
            "(" | "[" | "{" => depth += 1,
            ")" | "]" | "}" if depth == 0 => return token.range.start,
            ")" | "]" | "}" => depth -= 1,
            "|" | ";" if depth == 0 => return token.range.start,
            _ => {}
        }
    }
    reader.end()
}

const VALUE_OPERATORS: &[(&str, u8, Option<B>)] = &[
    ("or", 1, Some(B::Or)),
    ("\\/", 1, Some(B::Or)),
    ("and", 2, Some(B::And)),
    ("/\\", 2, Some(B::And)),
    ("==", 3, Some(B::Equal)),
    ("!=", 3, Some(B::NotEqual)),
    ("<=", 4, Some(B::LessEqual)),
    (">=", 4, Some(B::GreaterEqual)),
    ("in", 4, Some(B::In)),
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
];

/// Whether a value continues with a postfix or infix operator. Pipeline stages
/// are classified separately by [`ValueSuffixKind`].
pub fn is_value_continuation(token: &str) -> bool {
    matches!(token, "[" | "." | "!" | "default")
        || VALUE_OPERATORS.iter().any(|(word, _, _)| *word == token)
}

/// Whether the initial (possibly grouped) name selects a field. Hosts that
/// retain saved native values leave its later pipeline routing to their facts.
pub fn has_named_value_postfix(reader: &ArgumentReader<'_>) -> bool {
    let mut probe = reader.at(reader.position()).expect("reader boundary");
    let Some(name) = std::iter::from_fn(|| syntax_token(&mut probe)).find(|t| t.value != "(")
    else {
        return false;
    };
    if !is_unquoted_binding_name(name.value) {
        return false;
    }
    std::iter::from_fn(|| syntax_token(&mut probe))
        .find(|t| t.value != ")")
        .is_some_and(|t| t.value == ".")
}

/// Recognize common value syntax at the current expression boundary without
/// consuming the reader. The host first applies native-stage precedence using
/// its binding facts and [`probe_ordering_call`]. Plain JSON-shaped literals
/// remain available to a legacy host. Checked ordering lookahead shares the
/// supplied request budget and remaining nesting allowance.
///
/// # Errors
/// Malformed ordering delimiters and exhausted lookahead allowances are located.
pub fn starts_common_expression(
    reader: &ArgumentReader<'_>,
    budget: &mut Budget,
    nesting: usize,
) -> Result<bool, QueryError> {
    let mut head = reader.at(reader.position())?;
    let mut groups = 0usize;
    let first = loop {
        match syntax_token(&mut head) {
            Some(token) if token.value == "(" => groups += 1,
            token => break token,
        }
    };
    if first.as_ref().is_some_and(|t| {
        t.value
            .starts_with(|ch: char| ch.is_ascii_digit() || ch == '"')
            || matches!(t.value, "+" | "-" | "true" | "false" | "null" | "not")
    }) {
        return Ok(true);
    }
    let end = expression_boundary(reader);
    if has_collection_syntax(&reader.source()[reader.position()..end]) {
        return Ok(true);
    }
    let mut probe = reader.at(reader.position())?;
    let array_head = {
        let mut start = probe.at(probe.position())?;
        syntax_token(&mut start).is_some_and(|t| t.value == "[")
    };
    let mut depth = 0usize;
    let mut previous = "[";
    while let Some(token) = syntax_token(&mut probe) {
        let word = token.value;
        match word {
            "[" if depth == 0 && is_unquoted_binding_name(previous) => return Ok(true),
            "[" | "{" | "(" => depth += 1,
            "]" | "}" | ")" => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Ok(
                        syntax_token(&mut probe).is_some_and(|t| is_value_continuation(t.value))
                    );
                }
            }
            "|" if array_head && depth == 1 => {
                let mut tail = probe.at(probe.position())?;
                while let Some(next) = syntax_token(&mut tail) {
                    if matches!(next.value, ";") {
                        break;
                    }
                    if next.value == "in" {
                        return Ok(true);
                    }
                }
            }
            "for" if array_head && depth == 1 && !matches!(previous, "[" | ",") => return Ok(true),
            "|" | ";" if depth == 0 => return Ok(false),
            "." if depth > 0 && depth <= groups => {}
            "," if depth > 0 && depth <= groups => return Ok(true),
            "default" if !matches!(previous, "[" | "(" | "{" | "," | ":" | ";" | "=" | "|") => {
                return Ok(true);
            }
            word if depth <= groups && is_value_continuation(word) => {
                return Ok(true);
            }
            word if is_common_control_call(word) => {
                let mut start = reader.at(token.range.start)?;
                start.word()?;
                if start.consume('(')
                    || matches!(word, "queue" | "identity_set") && start.consume('<')
                {
                    return Ok(true);
                }
            }
            word if is_common_value_call(word) => {
                let mut start = reader.at(token.range.start)?;
                if matches!(
                    word,
                    "sort" | "sort_by" | "min" | "max" | "min_by" | "max_by"
                ) {
                    if probe_ordering_call(&start, budget, nesting)? {
                        return Ok(true);
                    }
                } else {
                    start.word()?;
                    if start.consume('(') {
                        return Ok(true);
                    }
                }
            }
            _ => {}
        }
        previous = word;
    }
    Ok(false)
}

/// Common state/control calls that require the shared value reader. Native
/// primitive names remain inputs to host recognition.
pub fn is_common_control_call(name: &str) -> bool {
    matches!(
        name,
        "match"
            | "loop"
            | "fold"
            | "scan"
            | "queue"
            | "pop_front"
            | "push_back_all"
            | "identity_set"
            | "insert"
    )
}

/// Recognize a dotted native capture followed by a pipeline. Hosts apply their
/// field-source policy separately; no cursor or budget is consumed.
pub fn has_native_field_pipeline(reader: &ArgumentReader<'_>) -> bool {
    let mut probe = reader.at(reader.position()).expect("reader boundary");
    if probe.word().is_err() || !probe.consume('.') {
        return false;
    }
    loop {
        if probe.word().is_err() {
            return false;
        }
        if !probe.consume('.') {
            return probe.consume('|');
        }
    }
}

/// One supported constructed collection suffix. Hosts can retain this syntax
/// until their existing type facts distinguish constructed from native input.
#[derive(Debug, PartialEq)]
pub enum ValueSuffix<Q = ()> {
    Filter(PredicateArgument<Q>),
    Map(PredicateArgument<Q>),
    FlatMap(PredicateArgument<Q>),
    Windows(Box<ValueExpression<Q>>),
    RunsBy(Located<String>),
    Count,
    Head(Box<ValueExpression<Q>>),
    Tail(Box<ValueExpression<Q>>),
    Reverse,
    Check {
        operand: Option<Box<ValueExpression<Q>>>,
        message: Option<Box<ValueExpression<Q>>>,
    },
    Sort,
    Unique,
    Tally,
}

/// Lower one suffix to the shared value tree, preserving the stage's location.
/// Parsing and tree-allocation allowances remain with the caller.
///
/// # Errors
/// A suffix after a terminal tally is refused at the suffix range.
pub fn lower_value_suffix<S, O, E>(
    value: ValueExpression<Expression<S, O, E>>,
    stage: Located<ValueSuffix<Expression<S, O, E>>>,
) -> Result<ValueExpression<Expression<S, O, E>>, QueryError> {
    if matches!(value.value, K::Tally(_)) {
        dispose_native_value(value);
        match stage.value {
            ValueSuffix::Filter(predicate)
            | ValueSuffix::Map(predicate)
            | ValueSuffix::FlatMap(predicate) => {
                if let Some(pattern) = predicate.argument {
                    crate::dispose_pattern(pattern);
                }
                dispose_native_value(*predicate.body);
            }
            ValueSuffix::Windows(size) | ValueSuffix::Head(size) | ValueSuffix::Tail(size) => {
                dispose_native_value(*size)
            }
            ValueSuffix::Check { operand, message } => {
                if let Some(v) = operand {
                    dispose_native_value(*v);
                }
                if let Some(v) = message {
                    dispose_native_value(*v);
                }
            }
            _ => {}
        }
        return Err(QueryError::query(
            stage.range,
            "tally must be the terminal stage",
        ));
    }
    let kind = match stage.value {
        ValueSuffix::Filter(predicate) => K::Filter {
            predicate,
            array: Box::new(value),
        },
        ValueSuffix::Map(argument) => K::Map {
            argument,
            array: Box::new(value),
        },
        ValueSuffix::FlatMap(argument) => K::FlatMap {
            argument,
            array: Box::new(value),
        },
        ValueSuffix::Windows(size) => K::Windows {
            items: Box::new(value),
            size,
        },
        ValueSuffix::RunsBy(field) => K::RunsBy {
            rows: Box::new(value),
            field,
        },
        ValueSuffix::Count => K::Reduce {
            operator: ValueReducer::Length,
            array: Box::new(value),
        },
        ValueSuffix::Head(count) => K::Head {
            collection: Box::new(value),
            count,
        },
        ValueSuffix::Tail(count) => K::Tail {
            collection: Box::new(value),
            count,
        },
        ValueSuffix::Reverse => K::Reverse(Box::new(value)),
        ValueSuffix::Check { operand, message } => K::Check {
            collection: Box::new(value),
            operand,
            message,
        },
        ValueSuffix::Sort => K::Sort {
            array: Box::new(value),
            selector: None,
            descending: false,
        },
        ValueSuffix::Unique => K::Unique(Box::new(value)),
        ValueSuffix::Tally => K::Tally(Box::new(value)),
    };
    Ok(Located::new(kind, stage.range))
}

/// Continue a lowered value with common constructed collection stages.
/// Lexical names and host syntax remain available inside scoped stage bodies.
///
/// # Errors
/// Invalid stages, predicate syntax and exhausted allowances retain query ranges.
pub fn read_value_suffix<H: ValueReaderHost>(
    reader: &mut ArgumentReader<'_>,
    host: &mut H,
    value: ValueExpression<ReadExpression<H>>,
    locals: &[&str],
    budget: &mut Budget,
    nesting: usize,
) -> Result<ValueExpression<ReadExpression<H>>, QueryError> {
    let value = ValueGuard::new(value);
    let locals = copy_reader_locals(locals, budget, reader.position())?;
    let mut parser = Reader {
        reader,
        host,
        nesting: nesting
            .min(crate::MAX_NESTING)
            .min(budget.remaining_nesting()),
        budget,
        locals,
    };
    parser.value_suffix(value.into_value(), 0)
}

fn copy_reader_locals<'l>(
    locals: &[&'l str],
    budget: &mut Budget,
    position: usize,
) -> Result<Vec<&'l str>, QueryError> {
    let location = Location::Query(position..position);
    crate::value::allocation::<&str>(locals.len(), &location)?;
    budget.charge(locals.len(), location.clone())?;
    budget.charge_results(locals.len(), location)?;
    Ok(locals.to_vec())
}

/// Read the parenthesized scoped body after a constructed `filter` keyword.
///
/// # Errors
/// Invalid expressions, malformed delimiters and parsing limits are located.
pub fn read_constructed_filter<H: ValueReaderHost>(
    reader: &mut ArgumentReader<'_>,
    host: &mut H,
    locals: &[&str],
    budget: &mut Budget,
    nesting: usize,
) -> Result<PredicateArgument<ReadExpression<H>>, QueryError> {
    reader.expect('(')?;
    let mut predicate = PredicateArgumentGuard(Some(read_predicate_argument(
        reader, host, locals, budget, nesting,
    )?));
    reader.expect(')')?;
    Ok(predicate.0.take().expect("retained predicate argument"))
}

/// Read `pattern => expression` or an expression using the implicit `_` subject.
/// The caller supplies its closing delimiter and the subject used at execution.
/// This returns a value-only body. Use [`read_native_predicate_argument`] for
/// Boolean trees containing host-native predicate leaves.
///
/// # Errors
/// Invalid bodies and exhausted allowances retain their original query offsets.
pub fn read_predicate_argument<H: ValueReaderHost>(
    reader: &mut ArgumentReader<'_>,
    host: &mut H,
    locals: &[&str],
    budget: &mut Budget,
    nesting: usize,
) -> Result<PredicateArgument<ReadExpression<H>>, QueryError> {
    let locals = copy_reader_locals(locals, budget, reader.position())?;
    let mut parser = Reader {
        reader,
        host,
        nesting: nesting
            .min(crate::MAX_NESTING)
            .min(budget.remaining_nesting()),
        budget,
        locals,
    };
    parser.predicate_argument(0)
}

/// Read a native predicate tree with shared Boolean expressions and host leaves.
/// An optional `pattern =>` header binds its names alongside the implicit `_`
/// subject. The caller supplies the closing delimiter and execution subject.
/// Forward the enclosing lexical names, request budget and remaining nesting
/// allowance when reading a host argument at its original source cursor.
///
/// # Errors
/// Invalid predicates, unsupported headers and exhausted allowances retain
/// their original query offsets.
pub fn read_native_predicate_argument<H: ValueReaderHost>(
    reader: &mut ArgumentReader<'_>,
    host: &mut H,
    locals: &[&str],
    budget: &mut Budget,
    nesting: usize,
) -> Result<Predicate<H::Stage>, QueryError> {
    let locals = copy_reader_locals(locals, budget, reader.position())?;
    let mut parser = Reader {
        reader,
        host,
        nesting: nesting
            .min(crate::MAX_NESTING)
            .min(budget.remaining_nesting()),
        budget,
        locals,
    };
    parser.native_predicate_argument(0)
}

/// Read a normalized native expression, including capture field postfixes.
/// Domain stage hooks own native primitive arguments; common composition stays shared.
///
/// # Errors
/// Scalar expressions, malformed syntax and exhausted allowances are located.
pub fn read_native_expression<H: ValueReaderHost>(
    reader: &mut ArgumentReader<'_>,
    host: &mut H,
    budget: &mut Budget,
    nesting: usize,
) -> Result<ReadExpression<H>, QueryError> {
    Reader {
        reader,
        host,
        nesting: nesting
            .min(crate::MAX_NESTING)
            .min(budget.remaining_nesting()),
        budget,
        locals: Vec::new(),
    }
    .native_expression(0)
}

/// Read contextual `match { from name in query; bind name = query; where guard; }`.
/// The reader retains ordered clauses and original query ranges; execution is separate.
///
/// # Errors
/// Unsupported/malformed clauses and nesting or allocation exhaustion fail at source offsets.
pub fn read_match<H: ValueReaderHost>(
    reader: &mut ArgumentReader<'_>,
    host: &mut H,
    budget: &mut Budget,
    nesting: usize,
) -> Result<NativeMatch<crate::NativeBody<ReadExpression<H>>, H::Stage>, QueryError> {
    Reader {
        reader,
        host,
        nesting: nesting
            .min(crate::MAX_NESTING)
            .min(budget.remaining_nesting()),
        budget,
        locals: Vec::new(),
    }
    .matching(0)
}

/// Read `paths(edge, to=predicate, max_depth=n)` using common Boolean rules.
/// The edge is one registered name and the required bound is a literal usize.
///
/// # Errors
/// Unsupported edges, malformed arguments, negative/overflowing bounds and
/// exhausted parse allowances fail at their original query ranges.
pub fn read_paths<H: ValueReaderHost>(
    reader: &mut ArgumentReader<'_>,
    host: &mut H,
    budget: &mut Budget,
    nesting: usize,
) -> Result<NativePaths<H::Stage, H::Stage>, QueryError> {
    Reader {
        reader,
        host,
        nesting: nesting
            .min(crate::MAX_NESTING)
            .min(budget.remaining_nesting()),
        budget,
        locals: Vec::new(),
    }
    .paths(0)
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
        nesting: nesting
            .min(crate::MAX_NESTING)
            .min(budget.remaining_nesting()),
        budget,
        locals: Vec::new(),
    }
    .projection(0)
}

struct PatternGuard(Option<crate::Pattern>);
impl PatternGuard {
    fn pattern(&self) -> &crate::Pattern {
        self.0.as_ref().expect("retained pattern")
    }
    fn into_pattern(mut self) -> crate::Pattern {
        self.0.take().expect("retained pattern")
    }
}
impl Drop for PatternGuard {
    fn drop(&mut self) {
        if let Some(pattern) = self.0.take() {
            crate::dispose_pattern(pattern);
        }
    }
}
struct PatternReader<'a, 's, 'b> {
    reader: &'a mut ArgumentReader<'s>,
    budget: &'b mut Budget,
    nesting: usize,
}
impl PatternReader<'_, '_, '_> {
    fn slot(&mut self) -> Result<(), QueryError> {
        let location = Location::Query(self.reader.position()..self.reader.position());
        self.budget.charge(1, location.clone())?;
        self.budget.charge_results(1, location)
    }
    fn pattern(&mut self, depth: usize) -> Result<crate::Pattern, QueryError> {
        use crate::PatternKind as P;
        if depth >= self.nesting {
            return Err(self.reader.error("pattern nesting limit exceeded"));
        }
        self.reader.skip_trivia()?;
        let start = self.reader.position();
        self.slot()?;
        let kind = if self.reader.consume('(') {
            let mut guard = PatternGuard(Some(Located::new(P::Tuple(Vec::new()), start..start)));
            if !self.reader.consume(')') {
                loop {
                    let child = PatternGuard(Some(self.pattern(depth + 1)?));
                    self.slot()?;
                    let P::Tuple(children) = &mut guard.0.as_mut().expect("retained tuple").value
                    else {
                        unreachable!()
                    };
                    children.push(child.into_pattern());
                    if !self.reader.consume(',') {
                        self.reader.expect(')')?;
                        break;
                    }
                    if self.reader.consume(')') {
                        break;
                    }
                }
            }
            guard.into_pattern().value
        } else if self.reader.consume('[') {
            let mut guard = PatternGuard(Some(Located::new(
                P::List {
                    elements: Vec::new(),
                    ignore_rest: false,
                },
                start..start,
            )));
            if !self.reader.consume(']') {
                loop {
                    self.reader.skip_trivia()?;
                    if self.reader.remaining().starts_with("..") {
                        self.reader.advance(2)?;
                        let P::List { ignore_rest, .. } =
                            &mut guard.0.as_mut().expect("retained list").value
                        else {
                            unreachable!()
                        };
                        *ignore_rest = true;
                        self.reader.expect(']')?;
                        break;
                    }
                    let child = PatternGuard(Some(self.pattern(depth + 1)?));
                    self.slot()?;
                    let P::List { elements, .. } =
                        &mut guard.0.as_mut().expect("retained list").value
                    else {
                        unreachable!()
                    };
                    elements.push(child.into_pattern());
                    if !self.reader.consume(',') {
                        self.reader.expect(']')?;
                        break;
                    }
                    if self.reader.consume(']') {
                        break;
                    }
                }
            }
            guard.into_pattern().value
        } else if self.reader.consume('{') {
            let mut guard = PatternGuard(Some(Located::new(
                P::Record {
                    fields: Vec::new(),
                    ignore_rest: false,
                },
                start..start,
            )));
            if !self.reader.consume('}') {
                loop {
                    self.reader.skip_trivia()?;
                    if self.reader.remaining().starts_with("..") {
                        self.reader.advance(2)?;
                        let P::Record { ignore_rest, .. } =
                            &mut guard.0.as_mut().expect("retained record").value
                        else {
                            unreachable!()
                        };
                        *ignore_rest = true;
                        self.reader.expect('}')?;
                        break;
                    }
                    let quoted = self.reader.remaining().starts_with('"');
                    let label = if quoted {
                        read_string(self.reader, self.budget)?
                    } else {
                        let name = self.reader.word()?;
                        self.budget
                            .charge(name.value.len() + 1, Location::Query(name.range.clone()))?;
                        Located::new(name.value.to_owned(), name.range)
                    };
                    let child = if self.reader.consume(':') {
                        self.pattern(depth + 1)?
                    } else {
                        if quoted {
                            return Err(self
                                .reader
                                .error("quoted pattern fields require `: pattern`"));
                        }
                        self.slot()?;
                        let kind = if label.value == "_" {
                            P::Wildcard
                        } else {
                            P::Name(label.clone())
                        };
                        Located::new(kind, label.range.clone())
                    };
                    let child = PatternGuard(Some(child));
                    self.slot()?;
                    let P::Record { fields, .. } =
                        &mut guard.0.as_mut().expect("retained record").value
                    else {
                        unreachable!()
                    };
                    fields.push((label, child.into_pattern()));
                    if !self.reader.consume(',') {
                        self.reader.expect('}')?;
                        break;
                    }
                    if self.reader.consume('}') {
                        break;
                    }
                }
            }
            guard.into_pattern().value
        } else if self.reader.remaining().starts_with('"') {
            P::Literal(V::String(read_string(self.reader, self.budget)?.value))
        } else if self
            .reader
            .remaining()
            .bytes()
            .next()
            .is_some_and(|b| b.is_ascii_digit() || matches!(b, b'+' | b'-'))
        {
            let spelling = self.reader.number_text()?;
            let range = spelling.range;
            self.budget
                .charge(range.len() + 1, Location::Query(range.clone()))?;
            P::Literal(number(&self.reader.source()[range.clone()], range)?)
        } else {
            let name = self.reader.word()?;
            self.budget
                .charge(name.value.len() + 1, Location::Query(name.range.clone()))?;
            match name.value {
                _ if self.reader.source()[name.range.start..].starts_with('\'') => {
                    P::Name(Located::new(name.value.to_owned(), name.range))
                }
                "_" => P::Wildcard,
                "true" => P::Literal(V::Boolean(true)),
                "false" => P::Literal(V::Boolean(false)),
                "absent" => P::Absent,
                "present" if self.reader.consume('(') => {
                    let child = PatternGuard(Some(self.pattern(depth + 1)?));
                    self.reader.expect(')')?;
                    P::Present(Box::new(child.into_pattern()))
                }
                _ => P::Name(Located::new(name.value.to_owned(), name.range)),
            }
        };
        Ok(Located::new(kind, start..self.reader.position()))
    }
}

struct GeneratorHeader {
    patterns: Vec<PatternGuard>,
    binding: bool,
    case: bool,
}

fn generator_header(
    reader: &mut ArgumentReader<'_>,
    budget: &mut Budget,
    nesting: usize,
    alternate: bool,
) -> Result<GeneratorHeader, QueryError> {
    let case = reader.keyword("case");
    let mut patterns = Vec::new();
    loop {
        patterns.push(PatternGuard(Some(read_pattern(reader, budget, nesting)?)));
        if !reader.consume(',') {
            break;
        }
    }
    if reader.keyword("in") {
        return Ok(GeneratorHeader {
            patterns,
            binding: false,
            case,
        });
    }
    if alternate && patterns.len() == 1 && reader.consume('=') {
        return Ok(GeneratorHeader {
            patterns,
            binding: true,
            case,
        });
    }
    Err(reader.error(if alternate {
        "expected `in` or single-pattern `=` generator"
    } else {
        "expected `in` after comprehension binder"
    }))
}

fn typed_arguments_end(
    reader: &mut ArgumentReader<'_>,
    budget: &mut Budget,
    nesting: usize,
) -> Result<Option<usize>, QueryError> {
    if !reader.consume('<') {
        return Ok(None);
    }
    let mut depth = 1usize;
    while let Some(ch) = reader.remaining().chars().next() {
        let start = reader.position();
        if !reader.protected_span()? {
            reader.advance(ch.len_utf8())?;
            match ch {
                '(' | ')' | '[' | ']' | ';' | '|' => return Ok(None),
                '<' => {
                    depth += 1;
                    if depth >= nesting {
                        return Err(reader.error("value nesting limit exceeded"));
                    }
                }
                '>' => {
                    depth -= 1;
                    if depth == 0 {
                        let end = reader.position();
                        budget.charge(end - start, Location::Query(start..end))?;
                        return Ok(reader.consume('(').then_some(end));
                    }
                }
                _ => {}
            }
        }
        budget.charge(
            reader.position() - start,
            Location::Query(start..reader.position()),
        )?;
    }
    Ok(None)
}

/// Distinguish constructed ordering helpers from native selectors without
/// consuming the reader. `sort`, `min` and `max` take values; their `_by`
/// forms take values only when a comma occurs at their argument delimiter level.
/// Quotes, comments, source boundaries and byte coordinates are preserved.
/// Scanning uses the active budget and remaining nesting allowance.
///
/// # Errors
/// Malformed delimiters, unterminated input and exhausted allowances are located.
pub fn probe_ordering_call(
    reader: &ArgumentReader<'_>,
    budget: &mut Budget,
    nesting: usize,
) -> Result<bool, QueryError> {
    let mut probe = reader.at(reader.position())?;
    let Ok(name) = probe.word() else {
        return Ok(false);
    };
    if !matches!(
        name.value,
        "sort" | "min" | "max" | "sort_by" | "min_by" | "max_by"
    ) || !probe.consume('(')
    {
        return Ok(false);
    }
    if matches!(name.value, "sort" | "min" | "max") {
        return Ok(true);
    }
    // A top-level comma distinguishes constructed helpers from native selectors.
    let mut stack = ScanDelimiters::new();
    while !probe.remaining().is_empty() {
        if stack.is_empty() {
            if probe.remaining().starts_with(')') {
                return Ok(false);
            }
            if probe.remaining().starts_with(',') {
                return Ok(true);
            }
        }
        scan_step(
            &mut probe,
            &mut stack,
            budget,
            nesting
                .min(crate::MAX_NESTING)
                .min(budget.remaining_nesting()),
        )?;
    }
    Err(probe.error("unterminated ordering call"))
}

struct ScanDelimiters {
    closers: [char; crate::MAX_NESTING],
    len: usize,
}
impl ScanDelimiters {
    fn new() -> Self {
        Self {
            closers: [' '; crate::MAX_NESTING],
            len: 0,
        }
    }
    fn closing(closer: char) -> Self {
        let mut stack = Self::new();
        stack.push(closer);
        stack
    }
    fn len(&self) -> usize {
        self.len
    }
    fn is_empty(&self) -> bool {
        self.len == 0
    }
    fn last(&self) -> Option<&char> {
        self.len.checked_sub(1).map(|i| &self.closers[i])
    }
    fn push(&mut self, closer: char) {
        self.closers[self.len] = closer;
        self.len += 1;
    }
    fn pop(&mut self) -> Option<char> {
        self.len = self.len.checked_sub(1)?;
        Some(self.closers[self.len])
    }
}

fn scan_step(
    reader: &mut ArgumentReader<'_>,
    stack: &mut ScanDelimiters,
    budget: &mut Budget,
    nesting: usize,
) -> Result<(), QueryError> {
    let start = reader.position();
    if reader
        .remaining()
        .starts_with(|ch: char| ch.is_ascii_alphabetic() || ch == '_')
    {
        let mut probe = reader.at(start)?;
        let word = probe.word()?;
        if matches!(word.value, "set" | "queue" | "identity_set")
            && let Some(end) = typed_arguments_end(&mut probe, budget, nesting)?
        {
            reader.advance(end - start)?;
            return budget.charge(end - start, Location::Query(start..end));
        }
        reader.advance(word.range.end - start)?;
        return budget.charge(word.range.len(), Location::Query(word.range));
    }
    if !reader.protected_span()? {
        let ch = reader
            .remaining()
            .chars()
            .next()
            .ok_or_else(|| reader.error("unterminated delimiter"))?;
        match ch {
            '(' | '[' | '{' => {
                if stack.len() + 1 >= nesting {
                    return Err(reader.error("value nesting limit exceeded"));
                }
                budget.charge_results(1, Location::Query(start..start + ch.len_utf8()))?;
                stack.push(match ch {
                    '(' => ')',
                    '[' => ']',
                    _ => '}',
                });
            }
            ')' | ']' | '}' if stack.pop() != Some(ch) => {
                return Err(reader.error("mismatched delimiter"));
            }
            _ => {}
        }
        reader.advance(ch.len_utf8())?;
    }
    budget.charge(
        reader.position() - start,
        Location::Query(start..reader.position()),
    )
}

struct ComprehensionHead<'s> {
    separator: usize,
    alternate: bool,
    names: Vec<&'s str>,
}

struct Reader<'a, 's, 'h, 'l, H> {
    reader: &'a mut ArgumentReader<'s>,
    host: &'h mut H,
    budget: &'h mut Budget,
    nesting: usize,
    locals: Vec<&'l str>,
}

impl<'s: 'l, 'l, H: ValueReaderHost> Reader<'_, 's, '_, 'l, H> {
    fn host_syntax<T>(
        &mut self,
        depth: usize,
        read: impl FnOnce(
            &mut H,
            &mut ArgumentReader<'s>,
            &[&str],
            &mut Budget,
            usize,
        ) -> Result<T, QueryError>,
    ) -> Result<T, QueryError> {
        self.depth(depth)?;
        let nesting = self.nesting;
        self.budget
            .within_parsing(depth + 1, nesting, self.location(), |budget| {
                read(
                    self.host,
                    self.reader,
                    &self.locals,
                    budget,
                    nesting.saturating_sub(depth + 1),
                )
            })
    }

    fn function_value(&mut self) -> Result<ValueExpression<ReadExpression<H>>, QueryError> {
        self.function_value_at(0)
    }
    fn function_value_at(
        &mut self,
        depth: usize,
    ) -> Result<ValueExpression<ReadExpression<H>>, QueryError> {
        self.depth(depth)?;
        self.expression(0, depth)
    }

    fn pattern(&mut self, depth: usize) -> Result<crate::Pattern, QueryError> {
        PatternReader {
            reader: self.reader,
            budget: self.budget,
            nesting: self.nesting,
        }
        .pattern(depth)
    }
    fn add_pattern_locals(&mut self, pattern: &crate::Pattern) -> Result<(), QueryError> {
        for (name, _) in
            crate::value::pattern::facts(pattern, &[crate::ValueType::Unestablished], self.budget)?
        {
            self.slots::<&str>(1)?;
            self.locals.push(crate::parser::identifier_text(
                self.reader.source(),
                name.range.clone(),
            ));
        }
        Ok(())
    }
    fn block(&mut self, depth: usize) -> Result<ValueExpression<ReadExpression<H>>, QueryError> {
        self.depth(depth)?;
        self.reader.skip_trivia()?;
        let start = self.reader.position();
        self.reader.expect('{')?;
        let locals = self.locals.len();
        let result = (|| {
            let mut guard = ValueGuard::new(Located::new(
                K::Block {
                    bindings: Vec::new(),
                    value: Box::new(Located::new(K::Literal(V::Absent), start..start)),
                },
                start..start,
            ));
            while let Some(pattern) = read_pattern_let(
                self.reader,
                self.budget,
                self.nesting.saturating_sub(depth + 1),
            )? {
                let pattern = PatternGuard(Some(pattern));
                let value = ValueGuard::new(self.function_value_at(depth + 1)?);
                self.reader.expect(';')?;
                self.add_pattern_locals(pattern.pattern())?;
                self.slots::<(crate::Pattern, ValueExpression<ReadExpression<H>>)>(1)?;
                let K::Block { bindings, .. } = &mut guard.value_mut().value else {
                    unreachable!()
                };
                bindings.push((pattern.into_pattern(), value.into_value()));
            }
            let K::Block { bindings, .. } = &guard.value().value else {
                unreachable!()
            };
            let value =
                ValueGuard::new(self.function_value_at(depth + usize::from(!bindings.is_empty()))?);
            if matches!(value.value().value, K::Break(_) | K::Continue(_)) {
                self.reader.consume(';');
            }
            self.reader.expect('}')?;
            if bindings.is_empty() {
                return Ok(value.into_value());
            }
            let K::Block {
                value: final_value, ..
            } = &mut guard.value_mut().value
            else {
                unreachable!()
            };
            **final_value = value.into_value();
            guard.value_mut().range = start..self.reader.position();
            self.slots::<ValueExpression<ReadExpression<H>>>(1)?;
            Ok(guard.into_value())
        })();
        self.locals.truncate(locals);
        result
    }
    fn loop_value(
        &mut self,
        depth: usize,
    ) -> Result<ValueExpression<ReadExpression<H>>, QueryError> {
        self.depth(depth)?;
        let start = self.reader.position();
        self.reader.keyword("loop");
        self.reader.expect('(')?;
        let mut guard = ValueGuard::new(Located::new(
            K::Loop {
                states: Vec::new(),
                body: Box::new(Located::new(K::Literal(V::Absent), start..start)),
            },
            start..start,
        ));
        if !self.reader.consume(')') {
            loop {
                let name = self.name()?;
                self.reader.expect('=')?;
                let value = ValueGuard::new(self.function_value_at(depth + 1)?);
                self.slots::<(Located<String>, ValueExpression<ReadExpression<H>>)>(1)?;
                let K::Loop { states, .. } = &mut guard.value_mut().value else {
                    unreachable!()
                };
                states.push((name, value.into_value()));
                if !self.reader.consume(',') {
                    self.reader.expect(')')?;
                    break;
                }
            }
        }
        let locals = self.locals.len();
        let result = (|| {
            let K::Loop { states, .. } = &guard.value().value else {
                unreachable!()
            };
            for (name, _) in states {
                self.slots::<&str>(1)?;
                self.locals.push(crate::parser::identifier_text(
                    self.reader.source(),
                    name.range.clone(),
                ));
            }
            self.block(depth + 1)
        })();
        self.locals.truncate(locals);
        let body = ValueGuard::new(result?);
        let K::Loop { body: target, .. } = &mut guard.value_mut().value else {
            unreachable!()
        };
        **target = body.into_value();
        guard.value_mut().range = start..self.reader.position();
        self.slots::<ValueExpression<ReadExpression<H>>>(1)?;
        Ok(guard.into_value())
    }

    fn continue_value(
        &mut self,
        depth: usize,
    ) -> Result<ValueExpression<ReadExpression<H>>, QueryError> {
        let start = self.reader.position();
        self.reader.keyword("continue");
        self.reader.expect('(')?;
        let mut guard = ValueGuard::new(Located::new(K::Continue(Vec::new()), start..start));
        if !self.reader.consume(')') {
            loop {
                let value = ValueGuard::new(self.function_value_at(depth + 1)?);
                self.slots::<ValueExpression<ReadExpression<H>>>(1)?;
                let K::Continue(values) = &mut guard.value_mut().value else {
                    unreachable!()
                };
                values.push(value.into_value());
                if !self.reader.consume(',') {
                    self.reader.expect(')')?;
                    break;
                }
            }
        }
        guard.value_mut().range = start..self.reader.position();
        self.slots::<ValueExpression<ReadExpression<H>>>(1)?;
        Ok(guard.into_value())
    }

    // A saved `queue < bound` remains a comparison. Only a complete type
    // argument followed by `(` selects an explicitly typed constructor.
    fn typed_worklist_call(&mut self) -> Result<bool, QueryError> {
        let mut probe = self.reader.at(self.reader.position())?;
        probe.word()?;
        Ok(typed_arguments_end(&mut probe, self.budget, self.nesting)?.is_some())
    }

    fn worklist_call(
        &mut self,
        depth: usize,
    ) -> Result<ValueExpression<ReadExpression<H>>, QueryError> {
        let start = self.reader.position();
        let name = self.name()?;
        let element = if self.reader.consume('<') {
            let element = self.signature(depth + 1)?;
            self.reader.expect('>')?;
            Some(element)
        } else {
            None
        };
        self.reader.expect('(')?;
        if name.value == "identity_set" {
            let element = element.ok_or_else(|| {
                self.reader
                    .error("identity_set needs an explicit native element type")
            })?;
            self.reader.expect(')')?;
            return self.node(K::IdentitySet { element }, start..self.reader.position());
        }
        let left = ValueGuard::new(self.function_value_at(depth + 1)?);
        let kind = match name.value.as_str() {
            "queue" => {
                self.reader.expect(')')?;
                K::Queue {
                    items: Box::new(left.into_value()),
                    element,
                }
            }
            "pop_front" => {
                self.reader.expect(')')?;
                K::PopFront(Box::new(left.into_value()))
            }
            "push_back_all" | "insert" => {
                self.reader.expect(',')?;
                let right = ValueGuard::new(self.function_value_at(depth + 1)?);
                self.reader.expect(')')?;
                if name.value == "insert" {
                    K::Insert {
                        set: Box::new(left.into_value()),
                        item: Box::new(right.into_value()),
                    }
                } else {
                    K::PushBackAll {
                        queue: Box::new(left.into_value()),
                        items: Box::new(right.into_value()),
                    }
                }
            }
            "fold" | "scan" => {
                self.reader.expect(',')?;
                let initial = ValueGuard::new(self.function_value_at(depth + 1)?);
                self.reader.expect(',')?;
                let step = self.name()?;
                self.reader.expect(')')?;
                if name.value == "fold" {
                    K::Fold {
                        items: Box::new(left.into_value()),
                        initial: Box::new(initial.into_value()),
                        step,
                    }
                } else {
                    K::Scan {
                        items: Box::new(left.into_value()),
                        initial: Box::new(initial.into_value()),
                        step,
                    }
                }
            }
            _ => unreachable!("recognized shared worklist call"),
        };
        self.node(kind, start..self.reader.position())
    }

    fn arm_value(
        &mut self,
        depth: usize,
    ) -> Result<ValueExpression<ReadExpression<H>>, QueryError> {
        let mut probe = self.reader.at(self.reader.position())?;
        if probe.consume('{') {
            // A direct JSON record keeps its existing spelling in an arm.
            let mut record = probe.consume('}');
            if !record && probe.consume('"') {
                let mut escaped = false;
                while let Some(ch) = probe.remaining().chars().next() {
                    self.budget.charge(
                        1,
                        Location::Query(probe.position()..probe.position() + ch.len_utf8()),
                    )?;
                    probe.advance(ch.len_utf8())?;
                    if ch == '"' && !escaped {
                        record = probe.consume(':');
                        break;
                    }
                    escaped = ch == '\\' && !escaped;
                }
            }
            if !record {
                return self.block(depth);
            }
        }
        self.function_value_at(depth)
    }
    fn value_match(
        &mut self,
        depth: usize,
    ) -> Result<ValueExpression<ReadExpression<H>>, QueryError> {
        self.depth(depth)?;
        self.reader.skip_trivia()?;
        let start = self.reader.position();
        self.reader.keyword("match");
        self.reader.expect('(')?;
        let subject = ValueGuard::new(self.function_value_at(depth + 1)?);
        self.reader.expect(')')?;
        self.reader.expect('{')?;
        let mut guard = ValueGuard::new(Located::new(
            K::ValueMatch {
                subject: Box::new(subject.into_value()),
                arms: Vec::new(),
            },
            start..start,
        ));
        if !self.reader.consume('}') {
            loop {
                let pattern = PatternGuard(Some(self.pattern(depth + 1)?));
                let locals = self.locals.len();
                let result = (|| {
                    self.add_pattern_locals(pattern.pattern())?;
                    let condition = if self.reader.keyword("if") {
                        Some(ValueGuard::new(self.function_value_at(depth + 1)?))
                    } else {
                        None
                    };
                    self.reader.expect('=')?;
                    self.reader.expect('>')?;
                    let value = ValueGuard::new(self.arm_value(depth + 1)?);
                    self.slots::<crate::ValueMatchArm<ReadExpression<H>>>(1)?;
                    let K::ValueMatch { arms, .. } = &mut guard.value_mut().value else {
                        unreachable!()
                    };
                    arms.push(crate::ValueMatchArm {
                        pattern: pattern.into_pattern(),
                        guard: condition.map(ValueGuard::into_value),
                        value: value.into_value(),
                    });
                    Ok::<_, QueryError>(())
                })();
                self.locals.truncate(locals);
                result?;
                if !self.reader.consume(',') {
                    self.reader.expect('}')?;
                    break;
                }
                if self.reader.consume('}') {
                    break;
                }
            }
        }
        guard.value_mut().range = start..self.reader.position();
        self.slots::<ValueExpression<ReadExpression<H>>>(1)?;
        Ok(guard.into_value())
    }

    fn signature(&mut self, depth: usize) -> Result<Located<SignatureType>, QueryError> {
        use SignatureType as S;
        self.depth(depth)?;
        self.reader.skip_trivia()?;
        let start = self.reader.position();
        let name = self.reader.word()?;
        self.budget
            .charge(name.value.len() + 1, Location::Query(name.range.clone()))?;
        let quoted = self.reader.source()[name.range.start..].starts_with('\'');
        let ty = match name.value {
            _ if quoted => S::Named(Located::new(name.value.to_owned(), name.range)),
            "value" => S::Value,
            "int" => S::Integer,
            "float" => S::Float,
            "bool" => S::Boolean,
            "string" => S::String,
            "opt" => S::Optional(Box::new(self.signature(depth + 1)?)),
            "sequence" | "keyed" | "set" | "queue" | "identity_set" => {
                self.reader.expect('<')?;
                let element = self.signature(depth + 1)?;
                self.reader.expect('>')?;
                match name.value {
                    "sequence" => S::Sequence(Box::new(element)),
                    "keyed" => S::Keyed(Box::new(element)),
                    "queue" => S::Queue(Box::new(element)),
                    "identity_set" => S::IdentitySet(Box::new(element)),
                    _ => {
                        if !matches!(
                            element.value,
                            S::Value | S::Boolean | S::Integer | S::Float | S::String | S::Named(_)
                        ) {
                            return Err(QueryError::query(
                                element.range,
                                "unsupported set element type",
                            ));
                        }
                        S::Set(Box::new(element))
                    }
                }
            }
            "dict" => {
                self.reader.expect('<')?;
                let key = self.signature(depth + 1)?;
                self.reader.expect(',')?;
                let value = self.signature(depth + 1)?;
                self.reader.expect('>')?;
                S::Dictionary(Box::new(key), Box::new(value))
            }
            "tuple" => {
                self.reader.expect('<')?;
                let mut values = Vec::new();
                loop {
                    self.slots::<Located<S>>(1)?;
                    values.push(self.signature(depth + 1)?);
                    if !self.reader.consume(',') {
                        self.reader.expect('>')?;
                        break;
                    }
                }
                S::Tuple(values)
            }
            "record" => {
                self.reader.expect('{')?;
                let mut fields = Vec::new();
                if !self.reader.consume('}') {
                    loop {
                        self.reader.skip_trivia()?;
                        let label = if self.reader.remaining().starts_with('"') {
                            self.string()?
                        } else {
                            self.name()?
                        };
                        self.reader.expect(':')?;
                        let value = self.signature(depth + 1)?;
                        self.slots::<(Located<String>, Located<S>)>(1)?;
                        fields.push((label, value));
                        if !self.reader.consume(',') {
                            self.reader.expect('}')?;
                            break;
                        }
                    }
                }
                S::Record(fields)
            }
            _ => S::Named(Located::new(name.value.to_owned(), name.range)),
        };
        self.slots::<S>(1)?;
        Ok(Located::new(ty, start..self.reader.position()))
    }

    fn conditional(
        &mut self,
        depth: usize,
    ) -> Result<ValueExpression<ReadExpression<H>>, QueryError> {
        self.depth(depth)?;
        self.reader.skip_trivia()?;
        let start = self.reader.position();
        self.reader.keyword("if");
        let mut guard = ValueGuard::new(Located::new(
            K::Conditional {
                branches: Vec::new(),
                otherwise: Box::new(Located::new(K::Literal(V::Absent), start..start)),
            },
            start..start,
        ));
        let mut alternate = None;
        loop {
            let condition = ValueGuard::new(self.expression(0, depth + 1)?);
            let then = self.reader.keyword("then");
            if alternate.is_some_and(|alternate| alternate != then) {
                return Err(self
                    .reader
                    .error("conditional branches must use one spelling"));
            }
            alternate = Some(then);
            let value = ValueGuard::new(if then {
                self.expression(0, depth + 1)?
            } else {
                self.block(depth + 1)?
            });
            self.slots::<(
                ValueExpression<ReadExpression<H>>,
                ValueExpression<ReadExpression<H>>,
            )>(1)?;
            let K::Conditional { branches, .. } = &mut guard.value_mut().value else {
                unreachable!()
            };
            branches.push((condition.into_value(), value.into_value()));
            if then && self.reader.keyword("elseif") {
                continue;
            }
            if !self.reader.keyword("else") {
                return Err(self.reader.error("conditional requires a final else"));
            }
            if !then && self.reader.keyword("if") {
                continue;
            }
            break;
        }
        let otherwise = ValueGuard::new(if alternate == Some(true) {
            self.expression(0, depth + 1)?
        } else {
            self.block(depth + 1)?
        });
        if alternate == Some(true) && !self.reader.keyword("endif") {
            return Err(self.reader.error("expected `endif`"));
        }
        let K::Conditional {
            otherwise: final_value,
            ..
        } = &mut guard.value_mut().value
        else {
            unreachable!()
        };
        **final_value = otherwise.into_value();
        guard.value_mut().range = start..self.reader.position();
        self.slots::<ValueExpression<ReadExpression<H>>>(1)?;
        Ok(guard.into_value())
    }

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
        let value = ValueGuard::new(Located::new(value, range));
        self.slots::<ValueExpression<ReadExpression<H>>>(1)?;
        Ok(value.into_value())
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
        read_string(self.reader, self.budget)
    }
    fn word_is(&self, word: &str) -> bool {
        let mut probe = self
            .reader
            .at(self.reader.position())
            .expect("reader boundary");
        probe.set_end(self.reader.end());
        probe.keyword(word)
    }
    fn projection_call(&self) -> bool {
        let mut probe = self
            .reader
            .at(self.reader.position())
            .expect("reader byte boundary");
        probe
            .word()
            .is_ok_and(|word| word.value == "project" && self.host.is_stage(word.value))
            && probe.consume('(')
    }
    fn native_body_call(&self, name: &str) -> bool {
        let mut probe = self
            .reader
            .at(self.reader.position())
            .expect("reader byte boundary");
        probe.word().is_ok_and(|word| word.value == name) && probe.consume('(')
    }
    fn descending_option(&mut self) -> Result<bool, QueryError> {
        if !self.reader.consume(',') {
            return Ok(false);
        }
        let option = self.name()?;
        if option.value != "descending" {
            return Err(QueryError::query(
                option.range,
                "expected descending=true or descending=false",
            ));
        }
        self.reader.expect('=')?;
        let value = self.name()?;
        match value.value.as_str() {
            "true" => Ok(true),
            "false" => Ok(false),
            _ => Err(QueryError::query(
                value.range,
                "descending needs a literal Boolean",
            )),
        }
    }
    fn value_match_call(&mut self) -> Result<bool, QueryError> {
        let mut probe = self.reader.at(self.reader.position())?;
        if !probe.keyword("match") || !probe.consume('(') {
            return Ok(false);
        }
        let mut stack = ScanDelimiters::closing(')');
        while !probe.remaining().is_empty() {
            scan_step(&mut probe, &mut stack, self.budget, self.nesting)?;
            if stack.is_empty() {
                return Ok(probe.consume('{'));
            }
        }
        Err(probe.error("unterminated match subject"))
    }
    fn match_call(&self) -> bool {
        let mut probe = self
            .reader
            .at(self.reader.position())
            .expect("reader byte boundary");
        probe.word().is_ok_and(|word| word.value == "match") && probe.consume('{')
    }
    // Default readers retain their native capture postfix route. Value-capable
    // and constructed-only readers select fields through the value tree.
    fn field_pipeline(&self) -> bool {
        self.host.field_pipeline_source() == FieldPipelineSource::NativeCapture
            && has_native_field_pipeline(self.reader)
    }
    fn is_local(&mut self, name: &str, range: &Range<usize>) -> Result<bool, QueryError> {
        for local in self.locals.iter().rev() {
            self.budget.charge(
                1 + name.len().min(local.len()),
                Location::Query(range.clone()),
            )?;
            if *local == name {
                return Ok(true);
            }
        }
        Ok(false)
    }
    fn expression(
        &mut self,
        minimum: u8,
        depth: usize,
    ) -> Result<ValueExpression<ReadExpression<H>>, QueryError> {
        self.depth(depth)?;
        let mut left = ValueGuard::new(self.atom(depth)?);
        let mut chain = 0;
        loop {
            self.reader.skip_trivia()?;
            if minimum <= 10 && self.reader.keyword("with") {
                let start = left.value().range.start;
                let fields = self.update_fields(depth + chain + 1)?;
                left = ValueGuard::new(self.node(
                    K::With {
                        record: Box::new(left.into_value()),
                        fields,
                    },
                    start..self.reader.position(),
                )?);
            } else if minimum <= 10 && self.reader.consume('[') {
                let index = ValueGuard::new(self.expression(0, depth + 1)?);
                let index = if self.reader.consume(',') {
                    let start = index.value().range.start;
                    let mut tuple = ValueGuard::new(
                        self.node(K::Tuple(vec![index.into_value()]), start..start)?,
                    );
                    loop {
                        let part = ValueGuard::new(self.expression(0, depth + 1)?);
                        self.slots::<ValueExpression<ReadExpression<H>>>(1)?;
                        let K::Tuple(parts) = &mut tuple.value_mut().value else {
                            unreachable!()
                        };
                        parts.push(part.into_value());
                        if !self.reader.consume(',') {
                            break;
                        }
                    }
                    tuple.value_mut().range.end = self.reader.position();
                    tuple
                } else {
                    index
                };
                self.reader.expect(']')?;
                let range = left.value().range.start..self.reader.position();
                left = ValueGuard::new(self.node(
                    K::ArrayAccess {
                        array: Box::new(left.into_value()),
                        index: Box::new(index.into_value()),
                    },
                    range,
                )?);
            } else if minimum <= 10
                && self.reader.remaining().starts_with('.')
                && !self.reader.remaining().starts_with("..")
            {
                self.reader.advance(1)?;
                self.reader.skip_trivia()?;
                let start = left.value().range.start;
                let kind = if self
                    .reader
                    .remaining()
                    .bytes()
                    .next()
                    .is_some_and(|byte| byte.is_ascii_digit())
                {
                    let position = self.reader.unsigned_integer()?;
                    self.budget.charge(
                        position.range.len(),
                        Location::Query(position.range.clone()),
                    )?;
                    K::TupleAccess {
                        value: Box::new(left.into_value()),
                        position,
                    }
                } else {
                    let field = self.name()?;
                    K::Field {
                        value: Box::new(left.into_value()),
                        field,
                    }
                };
                left = ValueGuard::new(self.node(kind, start..self.reader.position())?);
            } else {
                if minimum <= 9 && self.word_is("default") {
                    self.reader.advance("default".len())?;
                    let right = self.expression(10, depth + chain + 1)?;
                    let range = left.value().range.start..right.range.end;
                    left = ValueGuard::new(self.node(
                        K::Default {
                            value: Box::new(left.into_value()),
                            fallback: Box::new(right),
                        },
                        range,
                    )?);
                    chain += 1;
                    self.depth(depth + chain)?;
                    continue;
                }
                let rest = self.reader.remaining();
                if rest.starts_with("=>") {
                    break;
                }
                let operator = VALUE_OPERATORS.iter().copied().find(|(word, _, _)| {
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
                let range = left.value().range.start..right.range.end;
                left = ValueGuard::new(self.node(
                    match operator {
                        Some(operator) => K::Binary {
                            operator,
                            left: Box::new(left.into_value()),
                            right: Box::new(right),
                        },
                        None => K::Range {
                            lower: Box::new(left.into_value()),
                            upper: Box::new(right),
                        },
                    },
                    range,
                )?);
            }
            chain += 1;
            self.depth(depth + chain)?;
        }
        if minimum == 0 {
            if matches!(left.value().value, K::Field { .. })
                && self.host.field_pipeline_source() == FieldPipelineSource::ValueArgument
                && self.reader.consume('|')
            {
                let value = self.value_pipeline(left.into_value(), depth + chain)?;
                return self.native_sets(value, depth + chain);
            }
            return self.value_suffix(left.into_value(), depth + chain);
        }
        Ok(left.into_value())
    }
    fn atom(&mut self, depth: usize) -> Result<ValueExpression<ReadExpression<H>>, QueryError> {
        self.reader.skip_trivia()?;
        let start = self.reader.position();
        let first = self
            .reader
            .remaining()
            .chars()
            .next()
            .ok_or_else(|| self.reader.error("expected a value expression"))?;
        if self.reader.remaining().starts_with("<>") {
            self.reader.advance(2)?;
            return self.node(K::Literal(V::Absent), start..self.reader.position());
        }
        if first == '[' {
            return self.array(depth);
        }
        if first == '{' {
            return self.collection(depth, true);
        }
        if first == '"' {
            let string = self.string()?;
            return self.node(K::Literal(V::String(string.value)), string.range);
        }
        if first == '(' {
            let mut value = self.native_pipeline(depth)?;
            if matches!(value.value, K::NativeQuery(_) | K::NativeValue(_)) {
                value = self.native_sets(value, depth)?;
            }
            return Ok(value);
        }
        if first.is_ascii_digit() || matches!(first, '+' | '-') {
            let negative = first == '-';
            if matches!(first, '+' | '-') {
                self.reader.advance(1)?;
                self.reader.skip_trivia()?;
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
            self.reader.number_text()?;
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
        let mut probe = self.reader.at(start)?;
        let word = probe.word()?;
        probe.skip_whitespace();
        if self.reader.source()[word.range.start..].starts_with('\'') {
            let name = self.name()?;
            return self.named_value(name, depth);
        }
        if word.value == "set"
            && probe.remaining().starts_with('<')
            && self.typed_worklist_call()?
        {
            self.reader.keyword("set");
            self.reader.expect('<')?;
            let element = self.name()?;
            let element_type = match element.value.as_str() {
                "value" => crate::SetElementType::Value,
                "int" => crate::SetElementType::Integer,
                "bool" => crate::SetElementType::Boolean,
                "float" => crate::SetElementType::Float,
                "string" => crate::SetElementType::String,
                _ => {
                    return Err(QueryError::query(
                        element.range,
                        "empty sets support value, int, bool, float or string",
                    ));
                }
            };
            self.reader.expect('>')?;
            self.reader.expect('(')?;
            self.reader.expect(')')?;
            return self.node(
                K::Literal(V::Set {
                    element_type: Some(element_type),
                    elements: Vec::new().into(),
                }),
                start..self.reader.position(),
            );
        }
        let local = self.is_local(word.value, &word.range)?;
        if word.value == "loop" && probe.remaining().starts_with('(') {
            return self.loop_value(depth);
        }
        if word.value == "continue" && probe.remaining().starts_with('(') {
            return self.continue_value(depth);
        }
        if word.value == "break"
            && !probe
                .remaining()
                .starts_with(['}', ')', ']', ',', ';', '|'])
            && !probe.remaining().is_empty()
        {
            self.reader.keyword("break");
            let value = ValueGuard::new(self.function_value_at(depth + 1)?);
            return self.node(
                K::Break(Box::new(value.into_value())),
                start..self.reader.position(),
            );
        }
        if matches!(word.value, "queue" | "identity_set")
            && (probe.remaining().starts_with('(')
                || probe.remaining().starts_with('<') && self.typed_worklist_call()?)
            || is_common_control_call(word.value)
                && !matches!(word.value, "loop" | "match" | "queue" | "identity_set")
                && probe.remaining().starts_with('(')
        {
            return self.worklist_call(depth);
        }
        if word.value == "if" {
            // A saved name keeps its postfixes/operators. An adjacent expression
            // starts the conditional form even when that name is also in scope.
            let mut condition = self.reader.at(probe.position())?;
            let starts_condition = condition.remaining().starts_with(['(', '"'])
                || condition
                    .remaining()
                    .bytes()
                    .next()
                    .is_some_and(|byte| byte.is_ascii_digit())
                || condition.word().is_ok_and(|word| {
                    !matches!(word.value, "and" | "or" | "in" | "div" | "mod" | "default")
                });
            if (!local && !self.host.is_binding("if")) || starts_condition {
                return self.conditional(depth);
            }
        }
        if word.value == "match" && self.value_match_call()? {
            return self.value_match(depth);
        }
        if !local && !self.host.is_binding(word.value) && !self.host.is_stage(word.value) {
            if matches!(word.value, "filter" | "map" | "flat_map")
                && probe.remaining().starts_with('(')
            {
                let name = self.name()?;
                self.reader.expect('(')?;
                let mut predicate =
                    PredicateArgumentGuard(Some(self.predicate_argument(depth + 1)?));
                self.reader.expect(')')?;
                let predicate = predicate.0.take().expect("retained predicate argument");
                self.depth(depth + 1)?;
                let input = self.node(K::CurrentInput, name.range)?;
                return self.node(
                    match name.value.as_str() {
                        "map" => K::Map {
                            array: Box::new(input),
                            argument: predicate,
                        },
                        "flat_map" => K::FlatMap {
                            array: Box::new(input),
                            argument: predicate,
                        },
                        _ => K::Filter {
                            array: Box::new(input),
                            predicate,
                        },
                    },
                    start..self.reader.position(),
                );
            }
            if word.value == "count" && !probe.remaining().starts_with('(') {
                let name = self.name()?;
                self.depth(depth + 1)?;
                let input = self.node(K::CurrentInput, name.range.clone())?;
                return self.node(
                    K::Reduce {
                        operator: ValueReducer::Length,
                        array: Box::new(input),
                    },
                    name.range,
                );
            }
        }
        if word.value == "_" && !self.field_pipeline() && !probe.remaining().starts_with('|') {
            let name = self.name()?;
            return self.node(K::PredicateSubject, name.range);
        }
        let ordering_call = probe_ordering_call(self.reader, self.budget, self.nesting)?;
        let mut next = self.reader.at(probe.position())?;
        let set_operand = (local || self.host.is_binding(word.value))
            && next
                .word()
                .is_ok_and(|next| self.host.set_operator(next.value).is_some());
        if set_operand
            || (matches!(word.value, "fixpoint" | "where_exists")
                && probe.remaining().starts_with('('))
            || self.match_call()
            || self.field_pipeline()
            || ((self.host.is_stage(word.value)
                || native_order(word.value).is_some()
                || word.value == "map_by")
                && !ordering_call
                && ((!local && !self.host.is_binding(word.value)) || self.projection_call()))
            || (word.value == "input" && !local)
            || (probe.remaining().starts_with('|')
                && !matches!(word.value, "true" | "false" | "null")
                && ((local
                    && self.host.field_pipeline_source() != FieldPipelineSource::Constructed)
                    || (!local && !self.host.is_constructed_binding(word.value))))
        {
            return self.native(depth);
        }
        let name = self.name()?;
        self.reader.skip_trivia()?;
        if self.reader.remaining().starts_with('(') && !is_common_value_call(&name.value) {
            return self.named_value(name, depth);
        }
        let name_end = name.range.end;
        let kind = match name.value.as_str() {
            "true" => K::Literal(V::Boolean(true)),
            "false" => K::Literal(V::Boolean(false)),
            "null" => K::Literal(V::Absent),
            "not" => K::Unary {
                operator: U::Not,
                operand: Box::new(self.expression(9, depth + 1)?),
            },
            _ if self.reader.consume('(') => {
                if !is_common_value_call(&name.value) {
                    return self.function_call(name, depth);
                }
                let value = ValueGuard::new(self.expression(0, depth + 1)?);
                if matches!(
                    name.value.as_str(),
                    "parse_int" | "parse_float" | "to_string"
                ) {
                    self.reader.expect(')')?;
                    K::Unary {
                        operator: match name.value.as_str() {
                            "parse_int" => U::ParseInt,
                            "parse_float" => U::ParseFloat,
                            _ => U::ToString,
                        },
                        operand: Box::new(value.into_value()),
                    }
                } else if matches!(name.value.as_str(), "lookup" | "has_key") {
                    self.reader.expect(',')?;
                    let key = ValueGuard::new(self.expression(0, depth + 1)?);
                    self.reader.expect(')')?;
                    if name.value == "lookup" {
                        K::Lookup {
                            value: Box::new(value.into_value()),
                            key: Box::new(key.into_value()),
                        }
                    } else {
                        K::HasKey {
                            value: Box::new(value.into_value()),
                            key: Box::new(key.into_value()),
                        }
                    }
                } else if matches!(
                    name.value.as_str(),
                    "contains" | "starts_with" | "ends_with"
                ) {
                    let compact = name.value == "starts_with"
                        && self.locals.contains(&"_")
                        && self.reader.consume(')');
                    let (left, right) = if compact {
                        (
                            self.node(K::PredicateSubject, name.range.clone())?,
                            value.into_value(),
                        )
                    } else {
                        self.reader.expect(',')?;
                        let right = self.expression(0, depth + 1)?;
                        self.reader.expect(')')?;
                        (value.into_value(), right)
                    };
                    K::Binary {
                        operator: match name.value.as_str() {
                            "contains" => B::Contains,
                            "starts_with" => B::StartsWith,
                            "ends_with" => B::EndsWith,
                            _ => unreachable!("string matching call"),
                        },
                        left: Box::new(left),
                        right: Box::new(right),
                    }
                } else if matches!(
                    name.value.as_str(),
                    "sort" | "sort_by" | "min" | "max" | "min_by" | "max_by"
                ) {
                    if matches!(name.value.as_str(), "sort" | "sort_by") {
                        let selector = if name.value == "sort_by" {
                            self.reader.expect(',')?;
                            if self.reader.keyword("value") {
                                let end = self.reader.position();
                                Some(Located::new(SortSelector::Value, end - "value".len()..end))
                            } else {
                                let field = self.string()?;
                                Some(Located::new(SortSelector::Field(field.value), field.range))
                            }
                        } else {
                            None
                        };
                        let descending = self.descending_option()?;
                        self.reader.expect(')')?;
                        K::Sort {
                            array: Box::new(value.into_value()),
                            selector,
                            descending,
                        }
                    } else {
                        let field = if name.value.ends_with("_by") {
                            self.reader.expect(',')?;
                            Some(self.string()?)
                        } else {
                            None
                        };
                        self.reader.expect(')')?;
                        K::Extremum {
                            operator: if name.value.starts_with("min") {
                                ValueExtremum::Min
                            } else {
                                ValueExtremum::Max
                            },
                            array: Box::new(value.into_value()),
                            field,
                        }
                    }
                } else if name.value == "windows" {
                    self.reader.expect(',')?;
                    let size = ValueGuard::new(self.expression(0, depth + 1)?);
                    self.reader.expect(')')?;
                    K::Windows {
                        items: Box::new(value.into_value()),
                        size: Box::new(size.into_value()),
                    }
                } else if name.value == "runs_by" {
                    self.reader.expect(',')?;
                    let field = self.string()?;
                    self.reader.expect(')')?;
                    K::RunsBy {
                        rows: Box::new(value.into_value()),
                        field,
                    }
                } else if name.value == "align" {
                    self.reader.expect(',')?;
                    let right = ValueGuard::new(self.expression(0, depth + 1)?);
                    self.reader.expect(',')?;
                    if !self.reader.keyword("by") {
                        return Err(self.reader.error("expected by=\"field\""));
                    }
                    self.reader.expect('=')?;
                    let field = self.string()?;
                    self.reader.expect(')')?;
                    K::Align {
                        left: Box::new(value.into_value()),
                        right: Box::new(right.into_value()),
                        field,
                    }
                } else if name.value == "group_by" {
                    self.reader.expect(',')?;
                    let field = self.string()?;
                    self.reader.expect(')')?;
                    K::GroupBy {
                        rows: Box::new(value.into_value()),
                        field,
                    }
                } else if name.value == "zip_exact" {
                    self.reader.expect(',')?;
                    let right = ValueGuard::new(self.expression(0, depth + 1)?);
                    self.reader.expect(')')?;
                    K::ZipExact {
                        left: Box::new(value.into_value()),
                        right: Box::new(right.into_value()),
                    }
                } else if name.value == "require" {
                    self.reader.expect(',')?;
                    let result = ValueGuard::new(self.expression(0, depth + 1)?);
                    self.reader.expect(',')?;
                    let message = self.string()?;
                    self.reader.expect(')')?;
                    K::Require {
                        condition: Box::new(value.into_value()),
                        value: Box::new(result.into_value()),
                        message,
                    }
                } else {
                    self.reader.expect(')')?;
                    let operator = match name.value.as_str() {
                        "length" => ValueReducer::Length,
                        "sum" => ValueReducer::Sum,
                        "forall" => ValueReducer::Forall,
                        "exists" => ValueReducer::Exists,
                        "occurs" | "absent" => {
                            let kind = if name.value == "occurs" {
                                K::Occurs(Box::new(value.into_value()))
                            } else {
                                K::Absent(Box::new(value.into_value()))
                            };
                            return self.node(kind, start..self.reader.position());
                        }
                        "index_set" => {
                            return self.node(
                                K::IndexSet(Box::new(value.into_value())),
                                start..self.reader.position(),
                            );
                        }
                        "entries" => {
                            return self.node(
                                K::Entries(Box::new(value.into_value())),
                                start..self.reader.position(),
                            );
                        }
                        "one" => {
                            return self.node(
                                K::One(Box::new(value.into_value())),
                                start..self.reader.position(),
                            );
                        }
                        _ => return Err(QueryError::query(name.range, "unsupported value call")),
                    };
                    K::Reduce {
                        operator,
                        array: Box::new(value.into_value()),
                    }
                }
            }
            _ => K::Name(name.value),
        };
        let end = if matches!(kind, K::Name(_)) {
            name_end
        } else {
            self.reader.position()
        };
        self.node(kind, start..end)
    }
    fn named_value(
        &mut self,
        name: Located<String>,
        depth: usize,
    ) -> Result<ValueExpression<ReadExpression<H>>, QueryError> {
        self.reader.skip_trivia()?;
        if !self.reader.remaining().starts_with('(') {
            let range = name.range.clone();
            return self.node(K::Name(name.value), range);
        }
        let position = self.reader.position();
        if let Some(value) = self.host_syntax(depth, |host, reader, locals, budget, nesting| {
            host.parse_domain_call(
                Located::new(name.value.as_str(), name.range.clone()),
                reader,
                locals,
                budget,
                nesting,
            )
        })? {
            return Ok(value);
        }
        if self.reader.position() != position {
            return Err(self.reader.error("unrecognized domain call consumed input"));
        }
        self.reader.expect('(')?;
        self.function_call(name, depth)
    }
    fn function_call(
        &mut self,
        name: Located<String>,
        depth: usize,
    ) -> Result<ValueExpression<ReadExpression<H>>, QueryError> {
        let call = self.call_arguments(name, depth)?;
        if self.reader.consume('|') {
            return self.value_pipeline(call, depth);
        }
        Ok(call)
    }
    fn call_arguments(
        &mut self,
        name: Located<String>,
        depth: usize,
    ) -> Result<ValueExpression<ReadExpression<H>>, QueryError> {
        let start = name.range.start;
        let mut arguments = ValueGuard::new(Located::new(K::Array(Vec::new()), start..start));
        if !self.reader.consume(')') {
            loop {
                let argument = ValueGuard::new(self.expression(0, depth + 1)?);
                self.slots::<ValueExpression<ReadExpression<H>>>(1)?;
                let K::Array(values) = &mut arguments.value_mut().value else {
                    unreachable!()
                };
                values.push(argument.into_value());
                if !self.reader.consume(',') {
                    self.reader.expect(')')?;
                    break;
                }
            }
        }
        let K::Array(arguments) = arguments.into_value().value else {
            unreachable!()
        };
        let call = self.node(
            K::Call {
                function: name,
                arguments,
            },
            start..self.reader.position(),
        )?;
        Ok(call)
    }
    fn value_pipeline(
        &mut self,
        value: ValueExpression<ReadExpression<H>>,
        depth: usize,
    ) -> Result<ValueExpression<ReadExpression<H>>, QueryError> {
        let value = ValueGuard::new(value);
        self.depth(depth)?;
        self.slots::<ReadExpression<H>>(2)?;
        self.slots::<Located<ExpressionStep<H::Stage, H::Emitter, H::SetOperator>>>(1)?;
        let range = value.value().range.clone();
        let start = range.start;
        let value = value.into_value();
        let source = Located::new(
            ExpressionKind::Sequence {
                input: Box::new(Located::new(ExpressionKind::Input, start..start)),
                steps: vec![Located::new(
                    ExpressionStep::Value(value.into()),
                    range.clone(),
                )],
            },
            range,
        );
        self.native_tail(source, depth, true)
    }
    fn update_fields(
        &mut self,
        depth: usize,
    ) -> Result<crate::ProjectionFields<ReadExpression<H>>, QueryError> {
        let start = self.reader.position();
        self.reader.expect('{')?;
        let mut record = ValueGuard::new(Located::new(K::Record(Vec::new()), start..start));
        if !self.reader.consume('}') {
            loop {
                self.reader.skip_trivia()?;
                let label = if self.reader.remaining().starts_with('"') {
                    self.string()?
                } else {
                    self.name()?
                };
                self.reader.expect(':')?;
                let value = ValueGuard::new(self.expression(0, depth + 1)?);
                self.slots::<(Located<String>, ValueExpression<ReadExpression<H>>)>(1)?;
                let K::Record(fields) = &mut record.value_mut().value else {
                    unreachable!("retained update fields")
                };
                fields.push((label, value.into_value()));
                if self.reader.consume('}') {
                    break;
                }
                self.reader.expect(',')?;
            }
        }
        let K::Record(fields) = record.into_value().value else {
            unreachable!("retained update fields")
        };
        Ok(fields)
    }
    fn fields(
        &mut self,
        depth: usize,
    ) -> Result<crate::ProjectionFields<ReadExpression<H>>, QueryError> {
        let start = self.reader.position();
        self.reader.expect('{')?;
        if self.reader.consume('}') {
            return Ok(Vec::new());
        }
        let mut record = ValueGuard::new(Located::new(K::Record(Vec::new()), start..start));
        loop {
            let label = self.string()?;
            self.reader.expect(':')?;
            let value = ValueGuard::new(self.expression(0, depth + 1)?);
            self.slots::<(Located<String>, ValueExpression<ReadExpression<H>>)>(1)?;
            let K::Record(fields) = &mut record.value_mut().value else {
                unreachable!("retained record fields")
            };
            fields.push((label, value.into_value()));
            if !self.reader.consume(',') {
                self.reader.expect('}')?;
                break;
            }
            self.reader.skip_trivia()?;
            if self.reader.remaining().starts_with('}') {
                return Err(self.reader.error("trailing comma in record fields"));
            }
        }
        let K::Record(fields) = record.into_value().value else {
            unreachable!("retained record fields")
        };
        Ok(fields)
    }
    fn array(&mut self, depth: usize) -> Result<ValueExpression<ReadExpression<H>>, QueryError> {
        self.collection(depth, false)
    }
    fn matrix(
        &mut self,
        start: usize,
        depth: usize,
    ) -> Result<ValueExpression<ReadExpression<H>>, QueryError> {
        let mut matrix = ValueGuard::new(Located::new(
            K::KeyedArray {
                axes: None,
                declared_axis_sources: None,
                entries: Vec::new(),
            },
            start..start,
        ));
        let mut columns: Option<Vec<Range<usize>>> = None;
        let mut width = None;
        let mut row = 0usize;
        while !self.reader.consume(']') {
            let segment_start = self.reader.position();
            let mut probe = self.reader.at(segment_start)?;
            probe.set_end(self.reader.end());
            let mut stack = ScanDelimiters::new();
            while !probe.remaining().is_empty()
                && !(stack.is_empty() && probe.remaining().starts_with('|'))
            {
                scan_step(
                    &mut probe,
                    &mut stack,
                    self.budget,
                    self.nesting.saturating_sub(depth + 1),
                )?;
            }
            if probe.remaining().is_empty() {
                return Err(probe.error("expected matrix row delimiter `|`"));
            }
            let segment_end = probe.position();
            let end = self.reader.end();
            self.reader.set_end(segment_end);
            let parsed = (|| {
                self.reader.skip_trivia()?;
                if self.reader.remaining().is_empty() {
                    return Err(self.reader.error("empty matrix requires established axes"));
                }
                let first = ValueGuard::new(self.expression(0, depth + 1)?);
                let keyed = self.reader.consume(':');
                let first_range = first.value().range.clone();
                let header = if keyed && row == 0 && columns.is_none() {
                    let mut probe = self.reader.at(self.reader.position())?;
                    probe.set_end(segment_end);
                    let mut stack = ScanDelimiters::new();
                    let mut colon = false;
                    while !probe.remaining().is_empty() {
                        if stack.is_empty() && probe.remaining().starts_with(':') {
                            colon = true;
                        }
                        scan_step(
                            &mut probe,
                            &mut stack,
                            self.budget,
                            self.nesting.saturating_sub(depth + 1),
                        )?;
                    }
                    self.reader.skip_trivia()?;
                    colon || self.reader.remaining().is_empty()
                } else {
                    false
                };
                if header {
                    self.slots::<Range<usize>>(1)?;
                    let mut keys = vec![first_range];
                    while !self.reader.remaining().is_empty() {
                        let key = ValueGuard::new(self.expression(0, depth + 1)?);
                        self.reader.expect(':')?;
                        self.slots::<Range<usize>>(1)?;
                        keys.push(key.value().range.clone());
                        self.reader.skip_trivia()?;
                    }
                    columns = Some(keys);
                    return Ok(());
                }
                row += 1;
                let row_key = keyed.then_some(first_range);
                let mut first = if keyed { None } else { Some(first) };
                let mut column = 0usize;
                loop {
                    let value = if let Some(value) = first.take() {
                        value
                    } else {
                        ValueGuard::new(self.expression(0, depth + 1)?)
                    };
                    let range = value.value().range.clone();
                    let row_key = if let Some(key) = &row_key {
                        self.expression_span(key.clone(), depth + 1)?
                    } else {
                        self.node(
                            K::Literal(V::Integer(
                                i64::try_from(row)
                                    .map_err(|_| self.reader.error("matrix row overflow"))?,
                            )),
                            range.clone(),
                        )?
                    };
                    let row_key = ValueGuard::new(row_key);
                    let column_key = if let Some(keys) = &columns {
                        let key = keys.get(column).ok_or_else(|| {
                            QueryError::query(range.clone(), "matrix has missing cells")
                        })?;
                        self.expression_span(key.clone(), depth + 1)?
                    } else {
                        self.node(
                            K::Literal(V::Integer(
                                i64::try_from(column + 1)
                                    .map_err(|_| self.reader.error("matrix column overflow"))?,
                            )),
                            range.clone(),
                        )?
                    };
                    let column_key = ValueGuard::new(column_key);
                    self.slots::<ValueExpression<ReadExpression<H>>>(2)?;
                    let key = ValueGuard::new(self.node(
                        K::Tuple(vec![row_key.into_value(), column_key.into_value()]),
                        range,
                    )?);
                    self.slots::<(
                        ValueExpression<ReadExpression<H>>,
                        ValueExpression<ReadExpression<H>>,
                    )>(1)?;
                    let K::KeyedArray { entries, .. } = &mut matrix.value_mut().value else {
                        unreachable!()
                    };
                    entries.push((key.into_value(), value.into_value()));
                    column += 1;
                    if !self.reader.consume(',') {
                        break;
                    }
                }
                self.reader.skip_trivia()?;
                if !self.reader.remaining().is_empty() {
                    return Err(self.reader.error("unexpected matrix row syntax"));
                }
                let expected =
                    *width.get_or_insert_with(|| columns.as_ref().map_or(column, Vec::len));
                if column != expected {
                    return Err(QueryError::query(
                        segment_start..segment_end,
                        "matrix has missing cells",
                    ));
                }
                Ok(())
            })();
            self.reader.set_end(end);
            parsed?;
            self.reader.expect('|')?;
        }
        if row == 0 {
            return Err(QueryError::query(
                start..self.reader.position(),
                "empty matrix requires established axes",
            ));
        }
        self.collection_axes(matrix.into_value().value, start, depth)
    }
    fn collection(
        &mut self,
        depth: usize,
        braces: bool,
    ) -> Result<ValueExpression<ReadExpression<H>>, QueryError> {
        let start = self.reader.position();
        let close = if braces { '}' } else { ']' };
        self.reader.expect(if braces { '{' } else { '[' })?;
        if !braces && self.reader.consume('|') {
            return self.matrix(start, depth);
        }
        if self.reader.consume(close) {
            let kind = if braces {
                K::Record(Vec::new())
            } else {
                K::Array(Vec::new())
            };
            return self.collection_axes(kind, start, depth);
        }
        let scope = self.locals.len();
        let discovery = self.collection_binders(close, depth + 1)?;
        if let Some(head) = &discovery {
            self.slots::<&str>(head.names.len())?;
            self.locals.extend(&head.names);
        }
        let end = self.reader.end();
        if let Some(head) = &discovery {
            self.reader.set_end(head.separator);
        }
        let parsed = (|| {
            let first = ValueGuard::new(self.expression(0, depth + 1)?);
            let pair = if self.reader.consume(':') {
                let head = ValueGuard::new(self.expression(0, depth + 1)?);
                (Some(first), head)
            } else {
                (None, first)
            };
            if discovery.is_some() {
                self.reader.skip_trivia()?;
                if !self.reader.remaining().is_empty() {
                    return Err(self.reader.error("unexpected comprehension head syntax"));
                }
            }
            Ok(pair)
        })();
        self.reader.set_end(end);
        self.locals.truncate(scope);
        let (key, head) = parsed?;
        if let Some(ComprehensionHead {
            alternate, names, ..
        }) = discovery
        {
            let mut comprehension = ValueGuard::new(Located::new(
                if let Some(key) = key {
                    if braces {
                        K::DictionaryComprehension {
                            clauses: Vec::new(),
                            key: Box::new(key.into_value()),
                            head: Box::new(head.into_value()),
                        }
                    } else {
                        K::KeyedComprehension {
                            clauses: Vec::new(),
                            key: Box::new(key.into_value()),
                            head: Box::new(head.into_value()),
                            declared_axis_sources: None,
                            empty_axis_sources: None,
                        }
                    }
                } else {
                    K::Comprehension {
                        constructor: if braces {
                            ValueConstructor::Set
                        } else {
                            ValueConstructor::Array
                        },
                        clauses: Vec::new(),
                        head: Box::new(head.into_value()),
                    }
                },
                start..start,
            ));
            let mut domains = Vec::new();
            let result = (|| {
                if alternate {
                    self.reader.expect('|')?;
                }
                loop {
                    if !alternate && !self.reader.keyword("for") {
                        return Err(self.reader.error("expected `for`"));
                    }
                    let clause_start = self.reader.position();
                    let GeneratorHeader {
                        mut patterns,
                        binding,
                        case,
                    } = generator_header(
                        self.reader,
                        self.budget,
                        self.nesting.saturating_sub(depth + 1),
                        alternate,
                    )?;
                    if binding && case {
                        return Err(self.reader.error("case requires an `in` generator"));
                    }
                    let source_start = self.reader.position();
                    let source = ValueGuard::new(self.expression(0, depth + 1)?);
                    let source_range = source_start..self.reader.position();
                    for pattern in &patterns {
                        self.add_pattern_locals(pattern.pattern())?;
                    }
                    let filter = if self.reader.keyword(if alternate { "where" } else { "if" }) {
                        Some(ValueGuard::new(self.expression(0, depth + 1)?))
                    } else {
                        None
                    };
                    let clause_end = self.reader.position();
                    if !binding {
                        for pattern in &patterns {
                            if let crate::PatternKind::Name(name) = &pattern.pattern().value {
                                self.slots::<(&str, Range<usize>)>(1)?;
                                domains.push((
                                    crate::parser::identifier_text(
                                        self.reader.source(),
                                        name.range.clone(),
                                    ),
                                    source_range.clone(),
                                ));
                            }
                        }
                    }
                    self.slots::<Located<ValueGenerator<ReadExpression<H>>>>(1)?;
                    let clause = if binding {
                        ValueGenerator::Bind {
                            binder: patterns.pop().expect("one binding pattern").into_pattern(),
                            value: source.into_value(),
                            filter: filter.map(ValueGuard::into_value),
                        }
                    } else {
                        ValueGenerator::In {
                            case,
                            binders: patterns
                                .into_iter()
                                .map(PatternGuard::into_pattern)
                                .collect(),
                            source: source.into_value(),
                            filter: filter.map(ValueGuard::into_value),
                        }
                    };
                    let clauses = match &mut comprehension.value_mut().value {
                        K::Comprehension { clauses, .. }
                        | K::DictionaryComprehension { clauses, .. }
                        | K::KeyedComprehension { clauses, .. } => clauses,
                        _ => unreachable!("retained comprehension"),
                    };
                    clauses.push(Located::new(clause, clause_start..clause_end));
                    self.depth(depth + clauses.len())?;
                    if alternate {
                        if !self.reader.consume(',') {
                            break;
                        }
                    } else if !self.word_is("for") {
                        break;
                    }
                }
                self.reader.expect(close)?;
                Ok(())
            })();
            self.locals.truncate(scope);
            result?;
            if !self.word_is("axes")
                && let K::KeyedComprehension { key, .. } = &comprehension.value().value
            {
                let sources = self.empty_axis_sources(key, &domains, &names, depth + 1)?;
                let K::KeyedComprehension {
                    empty_axis_sources, ..
                } = &mut comprehension.value_mut().value
                else {
                    unreachable!()
                };
                *empty_axis_sources = sources;
            }
            return self.collection_axes(comprehension.into_value().value, start, depth);
        }
        self.slots::<ValueExpression<ReadExpression<H>>>(if key.is_some() { 2 } else { 1 })?;
        let mut next_key = key.as_ref().and_then(|k| match k.value().value {
            K::Literal(V::Integer(n)) => Some(n),
            _ => None,
        });
        let mut collection = ValueGuard::new(Located::new(
            if let Some(key) = key {
                let entries = vec![(key.into_value(), head.into_value())];
                if braces {
                    K::Dictionary(entries)
                } else {
                    K::KeyedArray {
                        axes: None,
                        declared_axis_sources: None,
                        entries,
                    }
                }
            } else if braces {
                K::Set(vec![head.into_value()])
            } else {
                K::Array(vec![head.into_value()])
            },
            start..start,
        ));
        while self.reader.consume(',') {
            let first = ValueGuard::new(self.expression(0, depth + 1)?);
            match &mut collection.value_mut().value {
                K::Dictionary(entries) | K::KeyedArray { entries, .. } => {
                    let (key, value) = if self.reader.consume(':') {
                        next_key = match first.value().value {
                            K::Literal(V::Integer(n)) => Some(n),
                            _ => None,
                        };
                        (first, ValueGuard::new(self.expression(0, depth + 1)?))
                    } else if !braces {
                        let n = next_key.and_then(|n| n.checked_add(1)).ok_or_else(|| QueryError::query(first.value().range.clone(), "starting-key tail requires a written integer and checked successors"))?;
                        next_key = Some(n);
                        let key = ValueGuard::new(
                            self.node(K::Literal(V::Integer(n)), first.value().range.clone())?,
                        );
                        (key, first)
                    } else {
                        return Err(self.reader.error("expected ':' after dictionary key"));
                    };
                    self.slots::<(
                        ValueExpression<ReadExpression<H>>,
                        ValueExpression<ReadExpression<H>>,
                    )>(1)?;
                    entries.push((key.into_value(), value.into_value()));
                }
                K::Array(values) | K::Set(values) => {
                    self.slots::<ValueExpression<ReadExpression<H>>>(1)?;
                    values.push(first.into_value());
                }
                _ => unreachable!("retained collection"),
            }
        }
        self.reader.expect(close)?;
        if let K::Dictionary(entries) = &collection.value().value
            && entries
                .iter()
                .all(|(key, _)| matches!(key.value, K::Literal(V::String(_))))
        {
            self.slots::<(Located<String>, ValueExpression<ReadExpression<H>>)>(entries.len())?;
        }
        let kind = match collection.into_value().value {
            K::Dictionary(entries)
                if entries
                    .iter()
                    .all(|(key, _)| matches!(key.value, K::Literal(V::String(_)))) =>
            {
                K::Record(
                    entries
                        .into_iter()
                        .map(|(key, value)| {
                            let K::Literal(V::String(label)) = key.value else {
                                unreachable!("static string key")
                            };
                            (Located::new(label, key.range), value)
                        })
                        .collect(),
                )
            }
            kind => kind,
        };
        self.collection_axes(kind, start, depth)
    }
    fn expression_span(
        &mut self,
        range: Range<usize>,
        depth: usize,
    ) -> Result<ValueExpression<ReadExpression<H>>, QueryError> {
        let mut reader = self.reader.at(range.start)?;
        reader.set_end(range.end);
        let locals = copy_reader_locals(&self.locals, self.budget, range.start)?;
        let mut parser = Reader {
            reader: &mut reader,
            host: self.host,
            budget: self.budget,
            nesting: self.nesting,
            locals,
        };
        let value = ValueGuard::new(parser.expression(0, depth)?);
        parser.reader.skip_trivia()?;
        if !parser.reader.remaining().is_empty() {
            return Err(parser.reader.error("unexpected expression syntax"));
        }
        Ok(value.into_value())
    }
    fn empty_axis_sources(
        &mut self,
        key: &ValueExpression<ReadExpression<H>>,
        domains: &[(&str, Range<usize>)],
        names: &[&str],
        depth: usize,
    ) -> Result<Option<Vec<ValueExpression<ReadExpression<H>>>>, QueryError> {
        let components = match &key.value {
            K::Tuple(parts) => parts.as_slice(),
            _ => std::slice::from_ref(key),
        };
        let mut sources = ValueGuard::new(Located::new(K::Array(Vec::new()), key.range.clone()));
        for component in components {
            let K::Name(name) = &component.value else {
                return Ok(None);
            };
            self.budget
                .charge(domains.len(), Location::Query(component.range.clone()))?;
            let Some((_, range)) = domains.iter().find(|(binder, _)| *binder == name) else {
                return Ok(None);
            };
            let mut probe = self.reader.at(range.start)?;
            probe.set_end(range.end);
            let mut stack = ScanDelimiters::new();
            let mut previous = None;
            let mut field = false;
            while !probe.remaining().is_empty() {
                let start = probe.position();
                probe.skip_trivia()?;
                self.budget.charge(
                    probe.position() - start,
                    Location::Query(start..probe.position()),
                )?;
                let start = probe.position();
                let Some(ch) = probe.remaining().chars().next() else {
                    break;
                };
                if ch == '"' {
                    scan_step(
                        &mut probe,
                        &mut stack,
                        self.budget,
                        self.nesting.saturating_sub(depth),
                    )?;
                    field = false;
                    previous = Some('v');
                } else if ch.is_ascii_alphabetic() || ch == '_' || ch == '\'' {
                    let word = probe.word()?;
                    let mut next = probe.at(probe.position())?;
                    next.skip_trivia()?;
                    let label = stack.last() == Some(&')')
                        && matches!(previous, Some('(' | ','))
                        && next.remaining().starts_with(':');
                    let call = next.remaining().starts_with('(');
                    self.budget.charge(
                        probe.position() - start + names.len(),
                        Location::Query(start..probe.position()),
                    )?;
                    if !field && !label && !call && names.contains(&word.value) {
                        return Ok(None);
                    }
                    field = false;
                    previous = Some('v');
                } else {
                    scan_step(
                        &mut probe,
                        &mut stack,
                        self.budget,
                        self.nesting.saturating_sub(depth),
                    )?;
                    field = ch == '.' && previous != Some('.');
                    previous = Some(ch);
                }
            }
            let source = ValueGuard::new(self.expression_span(range.clone(), depth)?);
            self.slots::<ValueExpression<ReadExpression<H>>>(1)?;
            let K::Array(values) = &mut sources.value_mut().value else {
                unreachable!()
            };
            values.push(source.into_value());
        }
        let K::Array(values) = sources.into_value().value else {
            unreachable!()
        };
        Ok(Some(values))
    }
    fn collection_axes(
        &mut self,
        kind: K<ReadExpression<H>>,
        start: usize,
        depth: usize,
    ) -> Result<ValueExpression<ReadExpression<H>>, QueryError> {
        let mut collection = ValueGuard::new(Located::new(kind, start..self.reader.position()));
        if self.word_is("axes") {
            let axis_name = self.name()?;
            let kind = &mut collection.value_mut().value;
            if matches!(kind, K::Array(values) if values.is_empty()) {
                *kind = K::KeyedArray {
                    axes: None,
                    declared_axis_sources: None,
                    entries: Vec::new(),
                };
            }
            let sources = match kind {
                K::KeyedArray {
                    declared_axis_sources,
                    ..
                }
                | K::KeyedComprehension {
                    declared_axis_sources,
                    ..
                } => declared_axis_sources,
                _ => {
                    return Err(QueryError::query(
                        axis_name.range,
                        "axes requires an indexed bracket collection",
                    ));
                }
            };
            *sources = Some(Vec::new());
            self.reader.expect('(')?;
            if self.reader.consume(')') {
                return Err(QueryError::query(
                    axis_name.range,
                    "axes needs at least one source",
                ));
            }
            loop {
                let source = ValueGuard::new(self.expression(0, depth + 1)?);
                self.slots::<ValueExpression<ReadExpression<H>>>(1)?;
                let sources = match &mut collection.value_mut().value {
                    K::KeyedArray {
                        declared_axis_sources,
                        ..
                    }
                    | K::KeyedComprehension {
                        declared_axis_sources,
                        ..
                    } => declared_axis_sources.as_mut().expect("declared sources"),
                    _ => unreachable!("indexed collection"),
                };
                sources.push(source.into_value());
                if !self.reader.consume(',') {
                    self.reader.expect(')')?;
                    break;
                }
            }
            if self.word_is("axes") {
                let duplicate = self.name()?;
                return Err(QueryError::query(
                    duplicate.range,
                    "duplicate axes declaration",
                ));
            }
        }
        self.node(collection.into_value().value, start..self.reader.position())
    }
    // Discover only this collection's own generator separators. All pattern
    // syntax still goes through the shared reader, and every scan spends work.
    fn collection_binders(
        &mut self,
        close: char,
        depth: usize,
    ) -> Result<Option<ComprehensionHead<'s>>, QueryError> {
        let source = self.reader.source();
        let mut probe = ArgumentReader::new(source, self.reader.position())?;
        probe.set_end(self.reader.end());
        let mut stack = ScanDelimiters::new();
        let mut found: Option<(usize, bool)> = None;
        let mut names = Vec::new();
        loop {
            let trivia_start = probe.position();
            probe.skip_trivia()?;
            self.budget.charge(
                probe.position() - trivia_start,
                Location::Query(trivia_start..probe.position()),
            )?;
            let Some(ch) = probe.remaining().chars().next() else {
                break;
            };
            if stack.is_empty() {
                if ch == close {
                    break;
                }
                let separator = probe.position();
                let common = found.is_none_or(|(_, alternate)| !alternate) && probe.keyword("for");
                let pipe = found.is_none() && !common && probe.consume('|');
                let comma = found.is_some_and(|(_, alternate)| alternate) && probe.consume(',');
                if common || pipe || comma {
                    let alternate = found.map_or(pipe, |(_, alternate)| alternate);
                    let mut header = ArgumentReader::new(source, probe.position())?;
                    header.set_end(probe.end());
                    match generator_header(
                        &mut header,
                        self.budget,
                        self.nesting.saturating_sub(depth),
                        alternate,
                    ) {
                        Ok(GeneratorHeader { patterns, .. }) => {
                            self.budget.charge(
                                header.position() - probe.position(),
                                Location::Query(probe.position()..header.position()),
                            )?;
                            if found.is_none() {
                                found = Some((separator, alternate));
                            }
                            for pattern in patterns {
                                for name in crate::pattern_names(pattern.pattern(), self.budget)? {
                                    self.slots::<&str>(1)?;
                                    names.push(crate::parser::identifier_text(
                                        source,
                                        name.range.clone(),
                                    ));
                                }
                            }
                            probe.advance(header.position() - probe.position())?;
                            continue;
                        }
                        Err(error) if error.category != crate::QueryErrorCategory::Other => {
                            return Err(error);
                        }
                        Err(error) if found.is_some() && (common || comma) => return Err(error),
                        Err(_) => {}
                    }
                }
                if probe.position() != separator {
                    continue;
                }
            }
            scan_step(
                &mut probe,
                &mut stack,
                self.budget,
                self.nesting.saturating_sub(depth),
            )?;
        }
        if !stack.is_empty() {
            return Err(probe.error("unterminated collection delimiter"));
        }
        Ok(found.map(|(separator, alternate)| ComprehensionHead {
            separator,
            alternate,
            names,
        }))
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
        let fields = ValueGuard::new(Located::new(K::Record(fields), subject.range.clone()));
        self.reader.expect(')')?;
        let K::Record(fields) = fields.into_value().value else {
            unreachable!("retained projection fields")
        };
        Ok(Projection { subject, fields })
    }
    fn native(&mut self, depth: usize) -> Result<ValueExpression<ReadExpression<H>>, QueryError> {
        let value = self.native_pipeline(depth)?;
        self.native_sets(value, depth)
    }
    fn native_sets(
        &mut self,
        value: ValueExpression<ReadExpression<H>>,
        depth: usize,
    ) -> Result<ValueExpression<ReadExpression<H>>, QueryError> {
        let mut value = ValueGuard::new(value);
        loop {
            self.reader.skip_trivia()?;
            let mut probe = self.reader.at(self.reader.position())?;
            let Ok(word) = probe.word() else {
                break;
            };
            let Some(operator) = self.host.set_operator(word.value) else {
                break;
            };
            self.reader
                .advance(probe.position() - self.reader.position())?;
            let right = ValueGuard::new(self.native_pipeline(depth)?);
            let range = value.value().range.start..right.value().range.end;
            if !matches!(value.value().value, K::NativeQuery(_) | K::NativeValue(_))
                || !matches!(right.value().value, K::NativeQuery(_) | K::NativeValue(_))
            {
                return Err(QueryError::query(
                    range,
                    "set operands require native selections",
                ));
            }
            let (
                K::NativeQuery(left) | K::NativeValue(left),
                K::NativeQuery(right) | K::NativeValue(right),
            ) = (value.into_value().value, right.into_value().value)
            else {
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
            value = ValueGuard::new(Located::new(
                K::NativeQuery(Box::new(
                    guard.0.take().expect("retained native expression"),
                )),
                range,
            ));
        }
        Ok(value.into_value())
    }
    fn native_pipeline(
        &mut self,
        depth: usize,
    ) -> Result<ValueExpression<ReadExpression<H>>, QueryError> {
        self.depth(depth)?;
        self.reader.skip_trivia()?;
        let start = self.reader.position();
        self.slots::<ReadExpression<H>>(2)?;
        let source = if self.match_call() {
            let mut matching = MatchGuard(Some(self.matching(depth + 1)?));
            let range = matching
                .0
                .as_ref()
                .expect("native match source")
                .range
                .clone();
            self.slots::<Located<ExpressionStep<H::Stage, H::Emitter, H::SetOperator>>>(1)?;
            let stage =
                ExpressionStep::Match(matching.0.take().expect("native match source").into());
            Located::new(
                ExpressionKind::Sequence {
                    input: Box::new(Located::new(ExpressionKind::Input, start..start)),
                    steps: vec![Located::new(stage, range.clone())],
                },
                range,
            )
        } else if self.reader.consume('(') {
            let mut probe = self.reader.at(self.reader.position())?;
            probe.set_end(self.reader.end());
            probe.skip_trivia()?;
            let label = if probe.remaining().starts_with('"') {
                probe.quoted_string().map(|label| label.range)
            } else {
                probe.word().map(|label| label.range)
            };
            if label.is_ok() && probe.consume(':') {
                let mut record = ValueGuard::new(Located::new(K::Record(Vec::new()), start..start));
                loop {
                    self.reader.skip_trivia()?;
                    let label = if self.reader.remaining().starts_with('"') {
                        self.string()?
                    } else {
                        self.name()?
                    };
                    self.reader.expect(':')?;
                    let value = ValueGuard::new(self.expression(0, depth + 1)?);
                    self.slots::<(Located<String>, ValueExpression<ReadExpression<H>>)>(1)?;
                    let K::Record(fields) = &mut record.value_mut().value else {
                        unreachable!()
                    };
                    fields.push((label, value.into_value()));
                    if !self.reader.consume(',') {
                        self.reader.expect(')')?;
                        break;
                    }
                }
                record.value_mut().range = start..self.reader.position();
                self.slots::<ValueExpression<ReadExpression<H>>>(1)?;
                return Ok(record.into_value());
            }
            if self.reader.consume(')') {
                return self.node(K::Tuple(Vec::new()), start..self.reader.position());
            }
            let value = ValueGuard::new(self.expression(0, depth + 1)?);
            if self.reader.consume(',') {
                let mut tuple = ValueGuard::new(Located::new(
                    K::Tuple(vec![value.into_value()]),
                    start..start,
                ));
                self.slots::<ValueExpression<ReadExpression<H>>>(1)?;
                if !self.reader.consume(')') {
                    loop {
                        let child = ValueGuard::new(self.expression(0, depth + 1)?);
                        self.slots::<ValueExpression<ReadExpression<H>>>(1)?;
                        let K::Tuple(children) = &mut tuple.value_mut().value else {
                            unreachable!()
                        };
                        children.push(child.into_value());
                        if !self.reader.consume(',') {
                            self.reader.expect(')')?;
                            break;
                        }
                        if self.reader.consume(')') {
                            break;
                        }
                    }
                }
                tuple.value_mut().range = start..self.reader.position();
                self.slots::<ValueExpression<ReadExpression<H>>>(1)?;
                return Ok(tuple.into_value());
            }
            self.reader.expect(')')?;
            let range = start..self.reader.position();
            if !matches!(value.value().value, K::NativeQuery(_) | K::NativeValue(_)) {
                return Ok(Located::new(value.into_value().value, range));
            }
            let (K::NativeQuery(query) | K::NativeValue(query)) = value.into_value().value else {
                unreachable!("native group checked")
            };
            Located::new(ExpressionKind::Group(query), range)
        } else {
            let mut probe = self.reader.at(start)?;
            let first = probe.word()?;
            probe.skip_whitespace();
            let projection_call = self.projection_call();
            let native_body_call = matches!(first.value, "fixpoint" | "where_exists")
                && probe.remaining().starts_with('(');
            let binding =
                self.is_local(first.value, &first.range)? || self.host.is_binding(first.value);
            if first.value == "input" && !binding {
                self.reader.advance(probe.position() - start)?;
                Located::new(ExpressionKind::Input, first.range)
            } else if !projection_call
                && !native_body_call
                && (binding
                    || (!self.host.is_stage(first.value)
                        && native_order(first.value).is_none()
                        && first.value != "map_by"))
            {
                self.reader.advance(probe.position() - start)?;
                self.budget
                    .charge(first.value.len(), Location::Query(first.range.clone()))?;
                Located::new(
                    ExpressionKind::Binding(if first.value == "_" {
                        crate::value::PREDICATE_SUBJECT.to_owned()
                    } else {
                        first.value.to_owned()
                    }),
                    first.range,
                )
            } else {
                Located::new(ExpressionKind::Input, start..start)
            }
        };
        self.native_tail(source, depth, false)
    }
    fn native_tail(
        &mut self,
        source: ReadExpression<H>,
        depth: usize,
        forced: bool,
    ) -> Result<ValueExpression<ReadExpression<H>>, QueryError> {
        let start = source.range.start;
        let mut source = NativeGuard(Some(source));
        let mut postfixes = 0;
        while self.reader.consume('.') {
            self.depth(depth + postfixes + 1)?;
            let field = self.name()?;
            let range = start..field.range.end;
            self.slots::<ReadExpression<H>>(1)?;
            source.0 = Some(Located::new(
                ExpressionKind::Field {
                    value: Box::new(source.0.take().expect("native field source")),
                    field,
                },
                range,
            ));
            postfixes += 1;
        }
        let mut steps = Vec::new();
        let starts_stage = matches!(
            source.0.as_ref().expect("native source").value,
            ExpressionKind::Input
        ) && source.0.as_ref().expect("native source").range.is_empty();
        let mut more = forced || starts_stage || self.reader.consume('|');
        while more {
            self.reader.skip_trivia()?;
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
            let constructed_filter = if self.word_is("filter") {
                self.host_syntax(depth, |host, reader, locals, budget, nesting| {
                    host.parse_constructed_filter_in_scope(reader, locals, budget, nesting)
                })?
            } else {
                None
            };
            let step = if let Some(stage) = constructed_filter {
                stage
            } else if self.reader.keyword("filter") {
                self.reader.expect('(')?;
                let predicate = self.native_predicate_argument(depth + 1)?;
                self.reader.expect(')')?;
                ExpressionStep::Filter(predicate.into())
            } else if self.native_body_call("where_exists") {
                self.name()?;
                self.reader.expect('(')?;
                let mut body = NativeGuard(Some(self.native_expression(depth + 1)?));
                self.reader.expect(')')?;
                let body = body.0.take().expect("existence relation");
                let range = body.range.clone();
                ExpressionStep::WhereExists(crate::NativeBody::new(body, range).into())
            } else if self.native_body_call("fixpoint") {
                self.name()?;
                self.reader.expect('(')?;
                let mut body = NativeGuard(Some(self.native_expression(depth + 1)?));
                self.reader.expect(')')?;
                let body = body.0.take().expect("fixed-point body");
                let range = body.range.clone();
                ExpressionStep::FixedPoint(crate::NativeBody::new(body, range).into())
            } else if self.match_call() {
                let matching = self.matching(depth + 1)?;
                ExpressionStep::Match(matching.into())
            } else if self.projection_call() {
                let projection = self.projection(depth + 1)?;
                ExpressionStep::Projection(projection.into())
            } else if self.word_is("map_by") {
                let mapping = self.mapping(depth + 1)?;
                ExpressionStep::Mapping(
                    NativeMap {
                        key: crate::NativeBody::new(
                            mapping.key,
                            stage_start..self.reader.position(),
                        ),
                        value: crate::NativeBody::new(
                            mapping.value,
                            stage_start..self.reader.position(),
                        ),
                    }
                    .into(),
                )
            } else if let Some((order, keyed)) = {
                let mut probe = self.reader.at(stage_start)?;
                probe.word().ok().and_then(|word| native_order(word.value))
            } {
                self.name()?;
                let mut selector = NativeGuard(None);
                let map_value = if keyed {
                    self.reader.expect('(')?;
                    let map_value =
                        order == NativeOrder::Sort && self.map_value_selector(depth + 1)?;
                    if !map_value {
                        selector.0 = Some(self.native_expression(depth + 1)?);
                    }
                    self.reader.expect(')')?;
                    map_value
                } else {
                    false
                };
                if map_value {
                    ExpressionStep::MapSorting
                } else {
                    ExpressionStep::Ordering(
                        NativeOrdering {
                            order,
                            selector: selector.0.take().map(|query| {
                                let range = query.range.clone();
                                crate::NativeBody::new(query, range)
                            }),
                        }
                        .into(),
                    )
                }
            } else {
                self.host_syntax(depth, |host, reader, locals, budget, nesting| {
                    host.parse_stage_in_scope(reader, locals, budget, nesting)
                })?
            };
            if self.reader.position() == stage_start {
                return Err(self
                    .reader
                    .error("host stage parser did not consume a stage"));
            }
            self.slots::<Located<ExpressionStep<H::Stage, H::Emitter, H::SetOperator>>>(1)?;
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
        let query = Box::new(query.0.take().expect("native pipeline"));
        Ok(Located::new(
            if self.host.retains_native_values() {
                K::NativeValue(query)
            } else {
                K::NativeQuery(query)
            },
            range,
        ))
    }
    fn map_value_selector(&mut self, depth: usize) -> Result<bool, QueryError> {
        let start = self.reader.position();
        let mut probe = self.reader.at(start)?;
        let mut groups = 0;
        self.depth(depth)?;
        while probe.consume('(') {
            groups += 1;
            self.depth(depth + groups)?;
        }
        let Ok(name) = probe.word() else {
            return Ok(false);
        };
        if name.value != "value"
            || self.is_local(name.value, &name.range)?
            || self.host.is_binding(name.value)
        {
            return Ok(false);
        }
        for _ in 0..groups {
            if !probe.consume(')') {
                return Ok(false);
            }
        }
        let end = probe.position();
        if !probe.consume(')') {
            return Ok(false);
        }
        self.budget
            .charge(end - start, Location::Query(start..end))?;
        self.reader.advance(end - start)?;
        Ok(true)
    }
    fn native_expression(&mut self, depth: usize) -> Result<ReadExpression<H>, QueryError> {
        let value = self.native(depth)?;
        let range = value.range.clone();
        match value.value {
            K::NativeQuery(query) | K::NativeValue(query) => Ok(*query),
            kind => {
                dispose_native_value(Located::new(kind, range.clone()));
                Err(QueryError::query(range, "expected a native expression"))
            }
        }
    }
    fn mapping(&mut self, depth: usize) -> Result<NativeMap<ReadExpression<H>>, QueryError> {
        self.depth(depth)?;
        if !self.reader.keyword("map_by") {
            return Err(self.reader.error("expected `map_by`"));
        }
        self.reader.expect('(')?;
        let mut key = NativeGuard(Some(self.native_expression(depth + 1)?));
        self.reader.expect(',')?;
        let mut value = NativeGuard(Some(self.native_expression(depth + 1)?));
        self.reader.expect(')')?;
        Ok(NativeMap {
            key: key.0.take().expect("map key selector"),
            value: value.0.take().expect("map value selector"),
        })
    }
    fn matching(
        &mut self,
        depth: usize,
    ) -> Result<NativeMatch<crate::NativeBody<ReadExpression<H>>, H::Stage>, QueryError> {
        self.depth(depth)?;
        self.reader.skip_trivia()?;
        let start = self.reader.position();
        if !self.reader.keyword("match") {
            return Err(self.reader.error("expected `match`"));
        }
        self.reader.expect('{')?;
        let scope = self.locals.len();
        let mut guard = MatchGuard(Some(NativeMatch {
            clauses: Vec::new(),
            range: start..start,
        }));
        let result = (|| {
            while !self.reader.consume('}') {
                self.reader.skip_trivia()?;
                let clause_start = self.reader.position();
                self.slots::<Located<MatchClause<crate::NativeBody<ReadExpression<H>>, H::Stage>>>(
                    1,
                )?;
                let word = self.reader.word()?;
                let (clause, name) = match word.value {
                    "from" | "bind" => {
                        let name = self.name()?;
                        if word.value == "from" {
                            if !self.reader.keyword("in") {
                                return Err(self.reader.error("expected `in` after capture name"));
                            }
                        } else {
                            self.reader.expect('=')?;
                        }
                        let source = self.native_expression(depth + 1)?;
                        let range = source.range.clone();
                        let source = crate::NativeBody::new(source, range);
                        let local = crate::parser::identifier_text(
                            self.reader.source(),
                            name.range.clone(),
                        );
                        let clause = if word.value == "from" {
                            MatchClause::From { name, source }
                        } else {
                            MatchClause::Bind {
                                name,
                                value: source,
                            }
                        };
                        (clause, Some(local))
                    }
                    "where" => (
                        MatchClause::Where {
                            predicate: self.native_predicate_argument(depth + 1)?,
                        },
                        None,
                    ),
                    _ => {
                        return Err(QueryError::query(
                            word.range,
                            "expected from, bind or where clause",
                        ));
                    }
                };
                // Retain before checking the terminator so failure drops native
                // operands through the common iterative release path.
                guard
                    .0
                    .as_mut()
                    .expect("retained match")
                    .clauses
                    .push(Located::new(clause, clause_start..self.reader.position()));
                self.reader.expect(';')?;
                if let Some(name) = name {
                    self.slots::<&str>(1)?;
                    self.locals.push(name);
                }
                let matching = guard.0.as_ref().expect("retained match");
                self.depth(depth + matching.clauses.len())?;
            }
            let mut matching = guard.0.take().expect("retained match");
            matching.range = start..self.reader.position();
            Ok(matching)
        })();
        self.locals.truncate(scope);
        result
    }
    fn paths(&mut self, depth: usize) -> Result<NativePaths<H::Stage, H::Stage>, QueryError> {
        self.depth(depth)?;
        self.reader.skip_trivia()?;
        let start = self.reader.position();
        if !self.reader.keyword("paths") {
            return Err(self.reader.error("expected `paths`"));
        }
        self.reader.expect('(')?;
        let edge = self.reader.word()?;
        self.budget
            .charge(edge.value.len(), Location::Query(edge.range.clone()))?;
        let edge_range = edge.range.clone();
        let edge = self.host.parse_path_edge(edge, self.budget)?;
        self.reader.expect(',')?;
        if !self.reader.keyword("to") {
            return Err(self.reader.error("expected `to` target argument"));
        }
        self.reader.expect('=')?;
        let mut target = PredicateGuard(Some(self.native_predicate_argument(depth + 1)?));
        self.reader.expect(',')?;
        if !self.reader.keyword("max_depth") {
            return Err(self.reader.error("expected `max_depth` edge bound"));
        }
        self.reader.expect('=')?;
        let max_depth = self.reader.unsigned_integer()?;
        self.reader.expect(')')?;
        self.slots::<NativePaths<H::Stage, H::Stage>>(1)?;
        self.budget.charge(
            self.reader.position() - start,
            Location::Query(start..self.reader.position()),
        )?;
        Ok(NativePaths {
            edge: Located::new(edge, edge_range),
            target: target.0.take().expect("retained path target"),
            max_depth,
            range: start..self.reader.position(),
        })
    }
    fn predicate_pattern(&mut self, depth: usize) -> Result<Option<crate::Pattern>, QueryError> {
        let mut probe = self.reader.at(self.reader.position())?;
        probe.skip_whitespace();
        let mut stack = ScanDelimiters::new();
        while !probe.remaining().is_empty() {
            if stack.is_empty()
                && (probe.remaining().starts_with("=>")
                    || probe.remaining().starts_with([')', ']', '}', ',', ';']))
            {
                break;
            }
            scan_step(
                &mut probe,
                &mut stack,
                self.budget,
                self.nesting.saturating_sub(depth),
            )?;
        }
        probe.skip_whitespace();
        if !probe.remaining().starts_with("=>") {
            return Ok(None);
        }
        let pattern = PatternGuard(Some(self.pattern(depth + 1)?));
        self.reader.skip_trivia()?;
        if !self.reader.remaining().starts_with("=>") {
            return Err(self.reader.error("expected `=>` after predicate pattern"));
        }
        self.reader.advance(2)?;
        Ok(Some(pattern.into_pattern()))
    }
    fn predicate_argument(
        &mut self,
        depth: usize,
    ) -> Result<PredicateArgument<ReadExpression<H>>, QueryError> {
        self.depth(depth)?;
        let mut argument = PatternGuard(self.predicate_pattern(depth)?);
        let saved = self.locals.len();
        self.slots::<&str>(1)?;
        self.locals.push("_");
        if let Some(pattern) = &argument.0 {
            self.add_pattern_locals(pattern)?;
        }
        let result = self.expression(0, depth + 1);
        self.locals.truncate(saved);
        let body = result?;
        Ok(PredicateArgument {
            argument: argument.0.take(),
            body: Box::new(body),
        })
    }
    fn native_predicate_argument(
        &mut self,
        depth: usize,
    ) -> Result<Predicate<H::Stage>, QueryError> {
        let mut argument = PatternGuard(self.predicate_pattern(depth)?);
        let saved = self.locals.len();
        self.slots::<&str>(1)?;
        self.locals.push("_");
        if let Some(pattern) = &argument.0 {
            self.add_pattern_locals(pattern)?;
        }
        let result = (|| {
            let mut body = PredicateGuard(Some(self.predicate_or(depth, argument.0.as_ref())?));
            let Some(pattern) = &argument.0 else {
                return Ok(body.0.take().expect("retained predicate"));
            };
            let range = pattern.range.clone();
            self.slots::<ValueExpression<ReadExpression<H>>>(1)?;
            self.slots::<Predicate<H::Stage>>(3)?;
            // Admit the header before any native Boolean leaf, including leaves
            // that retain no value argument or bound name of their own.
            let admission = self.host.predicate_expression(
                PredicateArgument {
                    argument: argument.0.take(),
                    body: Box::new(Located::new(K::Literal(V::Boolean(true)), range.clone())),
                },
                range.clone(),
            )?;
            Ok(combine(
                vec![
                    Located::new(PredicateKind::Leaf(admission), range),
                    body.0.take().expect("retained predicate"),
                ],
                PredicateKind::And,
            ))
        })();
        self.locals.truncate(saved);
        result
    }
    fn value_suffix(
        &mut self,
        value: ValueExpression<ReadExpression<H>>,
        depth: usize,
    ) -> Result<ValueExpression<ReadExpression<H>>, QueryError> {
        let mut value = ValueGuard::new(value);
        let mut wrappers = 0;
        while self.reader.consume('|') {
            self.depth(depth + wrappers + 1)?;
            let stage = self.name()?;
            if matches!(value.value().value, K::Tally(_)) {
                return Err(QueryError::query(
                    stage.range,
                    "tally must be the terminal stage",
                ));
            }
            let suffix = match ValueSuffixKind::named(&stage.value) {
                Some(ValueSuffixKind::Filter | ValueSuffixKind::Map | ValueSuffixKind::FlatMap) => {
                    self.reader.expect('(')?;
                    let mut predicate = PredicateArgumentGuard(Some(
                        self.predicate_argument(depth + wrappers + 1)?,
                    ));
                    self.reader.expect(')')?;
                    let argument = predicate.0.take().expect("retained scoped argument");
                    match stage.value.as_str() {
                        "map" => ValueSuffix::Map(argument),
                        "flat_map" => ValueSuffix::FlatMap(argument),
                        _ => ValueSuffix::Filter(argument),
                    }
                }
                Some(kind @ (ValueSuffixKind::Head | ValueSuffixKind::Tail)) => {
                    self.reader.expect('(')?;
                    let count = ValueGuard::new(self.expression(0, depth + wrappers + 1)?);
                    self.reader.expect(')')?;
                    self.slots::<ValueExpression<ReadExpression<H>>>(1)?;
                    if kind == ValueSuffixKind::Head {
                        ValueSuffix::Head(Box::new(count.into_value()))
                    } else {
                        ValueSuffix::Tail(Box::new(count.into_value()))
                    }
                }
                Some(ValueSuffixKind::Check) => {
                    let mut operand = None;
                    let mut message = None;
                    if self.reader.consume('(') && !self.reader.consume(')') {
                        operand = Some(ValueGuard::new(self.expression(0, depth + wrappers + 1)?));
                        if self.reader.consume(',') {
                            message =
                                Some(ValueGuard::new(self.expression(0, depth + wrappers + 1)?));
                        }
                        self.reader.expect(')')?;
                    }
                    self.slots::<ValueExpression<ReadExpression<H>>>(
                        usize::from(operand.is_some()) + usize::from(message.is_some()),
                    )?;
                    ValueSuffix::Check {
                        operand: operand.map(|v| Box::new(v.into_value())),
                        message: message.map(|v| Box::new(v.into_value())),
                    }
                }
                Some(ValueSuffixKind::Windows) => {
                    self.reader.expect('(')?;
                    let size = ValueGuard::new(self.expression(0, depth + wrappers + 1)?);
                    self.reader.expect(')')?;
                    self.slots::<ValueExpression<ReadExpression<H>>>(1)?;
                    ValueSuffix::Windows(Box::new(size.into_value()))
                }
                Some(ValueSuffixKind::RunsBy) => {
                    self.reader.expect('(')?;
                    let field = self.string()?;
                    self.reader.expect(')')?;
                    ValueSuffix::RunsBy(field)
                }
                Some(kind) => kind.without_arguments().expect("argument-free suffix"),
                None => {
                    return Err(QueryError::query(
                        stage.range,
                        "unsupported constructed pipeline stage",
                    ));
                }
            };
            self.slots::<ValueExpression<ReadExpression<H>>>(1)?;
            value = ValueGuard::new(lower_value_suffix(
                value.into_value(),
                Located::new(suffix, stage.range.start..self.reader.position()),
            )?);
            wrappers += 1;
        }
        Ok(value.into_value())
    }
    fn symbol(&mut self, symbol: &str) -> Result<bool, QueryError> {
        self.reader.skip_trivia()?;
        if self.reader.remaining().starts_with(symbol) {
            self.reader.advance(symbol.len())?;
            Ok(true)
        } else {
            Ok(false)
        }
    }
    fn predicate_or(
        &mut self,
        depth: usize,
        argument: Option<&crate::Pattern>,
    ) -> Result<Predicate<H::Stage>, QueryError> {
        let first = self.predicate_and(depth, argument)?;
        self.slots::<Predicate<H::Stage>>(1)?;
        let mut predicates = vec![first];
        while self.reader.keyword("or") || self.symbol("\\/")? {
            self.slots::<Predicate<H::Stage>>(1)?;
            predicates.push(self.predicate_and(depth, argument)?);
        }
        Ok(combine(predicates, PredicateKind::Or))
    }
    fn predicate_and(
        &mut self,
        depth: usize,
        argument: Option<&crate::Pattern>,
    ) -> Result<Predicate<H::Stage>, QueryError> {
        let first = self.predicate_atom(depth, argument)?;
        self.slots::<Predicate<H::Stage>>(1)?;
        let mut predicates = vec![first];
        while self.reader.keyword("and") || self.symbol("/\\")? {
            self.slots::<Predicate<H::Stage>>(1)?;
            predicates.push(self.predicate_atom(depth, argument)?);
        }
        Ok(combine(predicates, PredicateKind::And))
    }
    fn predicate_group(&mut self, depth: usize) -> Result<bool, QueryError> {
        let mut probe = self.reader.at(self.reader.position())?;
        if !probe.consume('(') {
            return Ok(false);
        }
        // A following value operator/postfix keeps the parentheses with its operand.
        let mut stack = ScanDelimiters::closing(')');
        self.depth(depth + 1)?;
        while !probe.remaining().is_empty() {
            scan_step(
                &mut probe,
                &mut stack,
                self.budget,
                self.nesting.saturating_sub(depth),
            )?;
            if stack.is_empty() {
                let end = probe.position();
                probe.skip_trivia()?;
                self.budget.charge(
                    probe.position() - end,
                    Location::Query(end..probe.position()),
                )?;
                return Ok(probe.remaining().is_empty()
                    || probe.remaining().starts_with([')', ']', '}', ',', ';'])
                    || probe.remaining().starts_with("/\\")
                    || probe.remaining().starts_with("\\/")
                    || probe.keyword("and")
                    || probe.keyword("or"));
            }
        }
        Err(probe.error("unterminated predicate group"))
    }
    fn predicate_atom(
        &mut self,
        depth: usize,
        argument: Option<&crate::Pattern>,
    ) -> Result<Predicate<H::Stage>, QueryError> {
        self.depth(depth)?;
        self.reader.skip_trivia()?;
        let start = self.reader.position();
        let value = if self.reader.keyword("not") {
            PredicateKind::Not(Box::new(self.predicate_atom(depth + 1, argument)?))
        } else if self.predicate_group(depth)? && self.reader.consume('(') {
            let predicate = self.predicate_or(depth + 1, argument)?;
            self.reader.expect(')')?;
            PredicateKind::Group(Box::new(predicate))
        } else {
            let mut probe = self.reader.at(self.reader.position())?;
            let native_leaf = match probe.word() {
                Ok(word) if self.host.is_predicate_leaf(word.value) => {
                    probe.consume('(')
                        || (!self.is_local(word.value, &word.range)?
                            && !self.host.is_binding(word.value))
                }
                _ => false,
            };
            if native_leaf {
                PredicateKind::Leaf(self.host_syntax(
                    depth,
                    |host, reader, locals, budget, nesting| {
                        host.parse_predicate_leaf_in_argument(
                            reader, argument, locals, budget, nesting,
                        )
                    },
                )?)
            } else {
                let body = ValueGuard::new(self.expression(3, depth + 1)?);
                let range = body.value().range.clone();
                PredicateKind::Leaf(
                    self.host.predicate_expression(
                        PredicateArgument {
                            argument: argument
                                .map(|p| crate::copy_pattern(p, self.budget))
                                .transpose()?,
                            body: Box::new(body.into_value()),
                        },
                        range,
                    )?,
                )
            }
        };
        self.slots::<Predicate<H::Stage>>(1)?;
        Ok(Located::new(value, start..self.reader.position()))
    }
}

struct PredicateArgumentGuard<S, O, E>(Option<PredicateArgument<Expression<S, O, E>>>);
impl<S, O, E> Drop for PredicateArgumentGuard<S, O, E> {
    fn drop(&mut self) {
        if let Some(predicate) = self.0.take() {
            if let Some(pattern) = predicate.argument {
                crate::dispose_pattern(pattern);
            }
            dispose_native_value(*predicate.body);
        }
    }
}

struct PredicateGuard<P>(Option<Predicate<P>>);
impl<P> Drop for PredicateGuard<P> {
    fn drop(&mut self) {
        if let Some(predicate) = self.0.take() {
            crate::matching::dispose_predicate(predicate);
        }
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
    let unsigned = text.strip_prefix(['+', '-']).unwrap_or(text);
    if unsigned.starts_with("0x") || unsigned.starts_with("0X") || unsigned.starts_with("0o") {
        if unsigned.contains(['p', 'P', '.']) {
            return Err(QueryError::query(
                range,
                "hexadecimal floats are unsupported",
            ));
        }
        let radix = if unsigned.starts_with("0o") { 8 } else { 16 };
        let magnitude = u64::from_str_radix(&unsigned[2..], radix).map_err(|_| {
            QueryError::query(range.clone(), "invalid integer or signed 64-bit overflow")
        })?;
        let value = if text.starts_with('-') {
            if magnitude == 1u64 << 63 {
                i64::MIN
            } else {
                i64::try_from(magnitude)
                    .ok()
                    .and_then(i64::checked_neg)
                    .ok_or_else(|| {
                        QueryError::query(
                            range.clone(),
                            "integer literal exceeds signed 64-bit range",
                        )
                    })?
            }
        } else {
            i64::try_from(magnitude).map_err(|_| {
                QueryError::query(range.clone(), "integer literal exceeds signed 64-bit range")
            })?
        };
        return Ok(V::Integer(value));
    }
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
// Keep partially parsed shared values under iterative cleanup across fallible steps.
struct ValueGuard<S, O, E>(Option<ValueExpression<Expression<S, O, E>>>);
impl<S, O, E> ValueGuard<S, O, E> {
    fn new(value: ValueExpression<Expression<S, O, E>>) -> Self {
        Self(Some(value))
    }
    fn value(&self) -> &ValueExpression<Expression<S, O, E>> {
        self.0.as_ref().expect("retained value expression")
    }
    fn value_mut(&mut self) -> &mut ValueExpression<Expression<S, O, E>> {
        self.0.as_mut().expect("retained value expression")
    }
    fn into_value(mut self) -> ValueExpression<Expression<S, O, E>> {
        self.0.take().expect("retained value expression")
    }
}
impl<S, O, E> Drop for ValueGuard<S, O, E> {
    fn drop(&mut self) {
        if let Some(value) = self.0.take() {
            dispose_native_value(value);
        }
    }
}

/// Consume a shared value expression and release its wrappers and native trees
/// iteratively, including flat native set operands. Cleanup needs no budget.
/// Hosts remain responsible for trees inside opaque stage, set-operator,
/// emitter and predicate payloads, which this helper drops without inspecting.
pub fn dispose_native_value<S, O, E>(value: ValueExpression<Expression<S, O, E>>) {
    dispose_native_trees(vec![value], Vec::new());
}

pub(crate) fn dispose_native_trees<S, O, E>(
    mut pending: Vec<ValueExpression<Expression<S, O, E>>>,
    mut native: Vec<Expression<S, O, E>>,
) {
    while !pending.is_empty() || !native.is_empty() {
        if let Some(expression) = native.pop() {
            match expression.value {
                ExpressionKind::Input | ExpressionKind::Binding(_) => {}
                ExpressionKind::Group(child) | ExpressionKind::Field { value: child, .. } => {
                    native.push(*child)
                }
                ExpressionKind::Sequence { input, steps } => {
                    native.push(*input);
                    for step in steps {
                        crate::composition::dispose_step(step.value, &mut pending);
                    }
                }
                ExpressionKind::Set { left, right, .. } => {
                    native.push(*left);
                    native.push(*right);
                }
                ExpressionKind::Record(fields) => {
                    native.extend(fields.into_iter().map(|(_, value)| value))
                }
            }
            continue;
        }
        let value = pending.pop().expect("remaining value tree");
        let clauses = match value.value {
            K::Literal(value) => {
                crate::value::dispose_constructed_value(value);
                continue;
            }
            K::Name(_) | K::CurrentInput | K::PredicateSubject => continue,
            K::IdentitySet { element } => {
                dispose_signature(element.value);
                continue;
            }
            K::Queue { items, element } => {
                pending.push(*items);
                if let Some(element) = element {
                    dispose_signature(element.value);
                }
                continue;
            }
            K::PopFront(child) => {
                pending.push(*child);
                continue;
            }
            K::PushBackAll { queue, items } => {
                pending.push(*queue);
                pending.push(*items);
                continue;
            }
            K::Insert { set, item } => {
                pending.push(*set);
                pending.push(*item);
                continue;
            }
            K::NativeQuery(query) | K::NativeValue(query) => {
                native.push(*query);
                continue;
            }
            K::One(child)
            | K::Reverse(child)
            | K::Break(child)
            | K::Occurs(child)
            | K::Absent(child)
            | K::IndexSet(child)
            | K::Entries(child)
            | K::Unique(child)
            | K::Tally(child)
            | K::Unary { operand: child, .. }
            | K::Field { value: child, .. }
            | K::TupleAccess { value: child, .. }
            | K::RunsBy { rows: child, .. }
            | K::GroupBy { rows: child, .. }
            | K::Reduce { array: child, .. }
            | K::Sort { array: child, .. }
            | K::Extremum { array: child, .. } => {
                pending.push(*child);
                continue;
            }
            K::Head { collection, count } | K::Tail { collection, count } => {
                pending.push(*collection);
                pending.push(*count);
                continue;
            }
            K::Check {
                collection,
                operand,
                message,
            } => {
                pending.push(*collection);
                if let Some(v) = operand {
                    pending.push(*v);
                }
                if let Some(v) = message {
                    pending.push(*v);
                }
                continue;
            }
            K::Filter { array, predicate }
            | K::Map {
                array,
                argument: predicate,
            }
            | K::FlatMap {
                array,
                argument: predicate,
            }
            | K::PredicateScope {
                subject: array,
                predicate,
            }
            | K::SubjectScope {
                subject: array,
                argument: predicate,
            } => {
                pending.push(*array);
                if let Some(pattern) = predicate.argument {
                    crate::dispose_pattern(pattern);
                }
                pending.push(*predicate.body);
                continue;
            }
            K::Fold {
                items: left,
                initial: right,
                ..
            }
            | K::Scan {
                items: left,
                initial: right,
                ..
            }
            | K::Binary { left, right, .. }
            | K::Default {
                value: left,
                fallback: right,
            }
            | K::Lookup {
                value: left,
                key: right,
            }
            | K::HasKey {
                value: left,
                key: right,
            }
            | K::Windows {
                items: left,
                size: right,
            }
            | K::ZipExact { left, right }
            | K::Align { left, right, .. }
            | K::ArrayAccess {
                array: left,
                index: right,
            }
            | K::Range {
                lower: left,
                upper: right,
            }
            | K::Require {
                condition: left,
                value: right,
                ..
            } => {
                pending.push(*left);
                pending.push(*right);
                continue;
            }
            K::Array(values)
            | K::Continue(values)
            | K::Tuple(values)
            | K::Set(values)
            | K::Call {
                arguments: values, ..
            } => {
                pending.extend(values);
                continue;
            }
            K::With { record, fields } => {
                pending.push(*record);
                pending.extend(fields.into_iter().map(|(_, value)| value));
                continue;
            }
            K::Record(fields) => {
                pending.extend(fields.into_iter().map(|(_, value)| value));
                continue;
            }
            K::KeyedArray {
                entries,
                declared_axis_sources,
                ..
            } => {
                pending.extend(declared_axis_sources.into_iter().flatten());
                for (key, value) in entries {
                    pending.push(key);
                    pending.push(value);
                }
                continue;
            }
            K::Dictionary(entries) => {
                for (key, value) in entries {
                    pending.push(key);
                    pending.push(value);
                }
                continue;
            }
            K::Loop { states, body } => {
                pending.push(*body);
                pending.extend(states.into_iter().map(|(_, value)| value));
                continue;
            }
            K::Block { bindings, value } => {
                pending.push(*value);
                for (pattern, value) in bindings {
                    crate::value::pattern::dispose_pattern(pattern);
                    pending.push(value);
                }
                continue;
            }
            K::ValueMatch { subject, arms } => {
                pending.push(*subject);
                for arm in arms {
                    crate::value::pattern::dispose_pattern(arm.pattern);
                    pending.extend(arm.guard);
                    pending.push(arm.value);
                }
                continue;
            }
            K::Conditional {
                branches,
                otherwise,
            } => {
                pending.push(*otherwise);
                for (condition, value) in branches {
                    pending.push(condition);
                    pending.push(value);
                }
                continue;
            }
            K::Comprehension { clauses, head, .. } => {
                pending.push(*head);
                clauses
            }
            K::DictionaryComprehension { clauses, key, head } => {
                pending.push(*key);
                pending.push(*head);
                clauses
            }
            K::KeyedComprehension {
                clauses,
                key,
                head,
                declared_axis_sources,
                empty_axis_sources,
            } => {
                pending.extend(declared_axis_sources.into_iter().flatten());
                pending.push(*key);
                pending.push(*head);
                pending.extend(empty_axis_sources.into_iter().flatten());
                clauses
            }
        };
        for clause in clauses {
            let (source, filter) = match clause.value {
                ValueGenerator::In {
                    binders,
                    source,
                    filter,
                    ..
                } => {
                    for pattern in binders {
                        crate::dispose_pattern(pattern);
                    }
                    (source, filter)
                }
                ValueGenerator::Bind {
                    binder,
                    value,
                    filter,
                } => {
                    crate::dispose_pattern(binder);
                    (value, filter)
                }
            };
            pending.push(source);
            pending.extend(filter);
        }
    }
}

#[expect(
    clippy::type_complexity,
    reason = "The disposal guard retains the existing native match payload contract"
)]
struct MatchGuard<S, O, E, P>(Option<NativeMatch<crate::NativeBody<Expression<S, O, E>>, P>>);
impl<S, O, E, P> Drop for MatchGuard<S, O, E, P> {
    fn drop(&mut self) {
        if let Some(matching) = self.0.take() {
            crate::dispose_match(matching);
        }
    }
}

fn read_string(
    reader: &mut ArgumentReader<'_>,
    budget: &mut Budget,
) -> Result<Located<String>, QueryError> {
    let mut probe = reader.at(reader.position())?;
    probe.set_end(reader.end());
    probe.skip_trivia()?;
    let start = probe.position();
    if !probe.remaining().starts_with('"') {
        return Err(probe.error("expected '\"'"));
    }
    probe.protected_span()?;
    let range = start..probe.position();
    let work = range
        .len()
        .checked_mul(6)
        .ok_or_else(|| reader.error("value allocation size overflow"))?;
    budget.charge(work, Location::Query(range))?;
    reader.quoted_string()
}

fn dispose_signature(value: SignatureType) {
    let mut pending = vec![value];
    while let Some(value) = pending.pop() {
        match value {
            SignatureType::Optional(c)
            | SignatureType::Sequence(c)
            | SignatureType::Keyed(c)
            | SignatureType::Set(c)
            | SignatureType::Queue(c)
            | SignatureType::IdentitySet(c) => pending.push(c.value),
            SignatureType::Dictionary(key, value) => {
                pending.push(key.value);
                pending.push(value.value);
            }
            SignatureType::Tuple(c) => pending.extend(c.into_iter().map(|c| c.value)),
            SignatureType::Record(c) => pending.extend(c.into_iter().map(|(_, c)| c.value)),
            _ => {}
        }
    }
}

fn native_order(name: &str) -> Option<(NativeOrder, bool)> {
    Some(match name {
        "sort" => (NativeOrder::Sort, false),
        "min" => (NativeOrder::Min, false),
        "max" => (NativeOrder::Max, false),
        "min_all" => (NativeOrder::MinAll, false),
        "max_all" => (NativeOrder::MaxAll, false),
        "sort_by" => (NativeOrder::Sort, true),
        "min_by" => (NativeOrder::Min, true),
        "max_by" => (NativeOrder::Max, true),
        "min_all_by" => (NativeOrder::MinAll, true),
        "max_all_by" => (NativeOrder::MaxAll, true),
        _ => return None,
    })
}
