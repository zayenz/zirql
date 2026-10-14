use std::ops::Range;

use crate::{
    ArgumentReader, Budget, ConstructedValue as V, Expression, ExpressionKind, ExpressionStep,
    Located, Location, MatchClause, NativeMatch, NativePaths, ParameterDeclaration, ParameterType,
    Predicate, PredicateKind, Projection, QueryError, ValueBinaryOperator as B, ValueConstructor,
    ValueExpression, ValueExpressionKind as K, ValueExtremum, ValueGenerator, ValueReducer,
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
    /// Parse one domain stage in the current lexical scope. An adapter that
    /// starts a nested common value reader carries these borrowed names into
    /// that reader's binding facts. This exposes names, never caller values.
    /// Existing hosts keep their ordinary stage parser.
    fn parse_stage_in_scope(
        &mut self,
        reader: &mut ArgumentReader<'_>,
        _locals: &[&str],
        budget: &mut Budget,
    ) -> Result<ExpressionStep<Self::Stage, Self::Emitter>, QueryError> {
        self.parse_stage(reader, budget)
    }
    /// Preserve the same lexical names when a domain predicate has value
    /// arguments of its own. Existing predicate parsers remain unchanged.
    fn parse_predicate_leaf_in_scope(
        &mut self,
        reader: &mut ArgumentReader<'_>,
        _locals: &[&str],
        budget: &mut Budget,
    ) -> Result<Self::Predicate, QueryError> {
        self.parse_predicate_leaf(reader, budget)
    }
    fn filter(&mut self, predicate: Predicate<Self::Predicate>, range: Range<usize>)
    -> Self::Stage;
    fn projection(
        &mut self,
        projection: Projection<ReadExpression<Self>>,
        range: Range<usize>,
    ) -> Self::Stage
    where
        Self: Sized;
    /// Wrap the shared match tree as a native stage. Supporting hosts retain
    /// this tree and delegate its type discovery and execution to Zirql.
    fn matching(
        &mut self,
        matching: NativeMatch<ReadExpression<Self>, Self::Predicate>,
        range: Range<usize>,
    ) -> Result<Self::Stage, QueryError>
    where
        Self: Sized,
    {
        crate::dispose_match(matching);
        Err(QueryError::query(
            range,
            "native matching is unsupported by this host",
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

    /// Lower a function's local initializer or final value using host syntax.
    /// The cursor and lexical names refer to the original source. Consume one
    /// expression, leaving its `;` or closing `}` to the common declaration
    /// reader. `None` selects the existing common value grammar.
    fn parse_function_value(
        &mut self,
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

    /// Lower an explicit value source for a native pipeline. The host retains
    /// this value as a declared native argument and evaluates it with the active
    /// function runner. The default preserves scalar-only frontends.
    fn value_source(
        &mut self,
        value: ValueExpression<ReadExpression<Self>>,
        range: Range<usize>,
    ) -> Result<Self::Stage, QueryError>
    where
        Self: Sized,
    {
        dispose_native_value(value);
        Err(QueryError::query(
            range,
            "native value sources are unsupported by this host",
        ))
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

/// Common type syntax before a host has resolved nominal/native names.
/// Nested locations and named leaves retain their original query coordinates.
#[derive(Debug, PartialEq)]
pub enum SignatureType {
    Boolean,
    Integer,
    Float,
    String,
    Named(Located<String>),
    Optional(Box<Located<Self>>),
    Sequence(Box<Located<Self>>),
    Tuple(Vec<Located<Self>>),
    Record(Vec<(Located<String>, Located<Self>)>),
    Set(Box<Located<Self>>),
    Keyed(Box<Located<Self>>),
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
    fn resolve<K>(
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
        budget.charge(1, location.clone())?;
        budget.charge_results(1, location.clone())?;
        let child = |value: &Located<Self>, budget: &mut Budget, resolve: &mut _| {
            value.value.resolve(budget, &value.range, resolve)
        };
        Ok(match self {
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
                D::Boolean => S::Boolean,
                D::Integer => S::Integer,
                D::Float => S::Float,
                D::Member(id) => S::Member(id),
                _ => {
                    return Err(QueryError::query(
                        value.range.clone(),
                        "unsupported set element type",
                    ));
                }
            }),
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
/// expression trees remain reusable. Host lowering uses
/// [`ValueReaderHost::parse_function_value`].
///
/// # Errors
/// Malformed declarations, unsupported type structure and limits are located.
pub fn read_unresolved_function_declaration<H: ValueReaderHost>(
    reader: &mut ArgumentReader<'_>,
    host: &mut H,
    budget: &mut Budget,
    nesting: usize,
) -> Result<Option<UnresolvedFunctionDeclaration<ReadExpression<H>>>, QueryError> {
    let mut probe = ArgumentReader::new(reader.source(), reader.position())?;
    if !probe.keyword("fn") || probe.word().is_err() || !probe.consume('(') {
        return Ok(None);
    }
    let mut parser = Reader {
        reader,
        host,
        budget,
        nesting: nesting.min(crate::MAX_NESTING),
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
            parser
                .locals
                .push(&parser.reader.source()[parameter.range.clone()]);
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
        let mut probe = ArgumentReader::new(parser.reader.source(), parser.reader.position())?;
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
        parser
            .locals
            .push(&parser.reader.source()[name.range.clone()]);
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
    Reader {
        reader,
        host,
        budget,
        nesting: nesting.min(crate::MAX_NESTING),
        locals: Vec::new(),
    }
    .expression(0, 0)
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
        budget,
        nesting: nesting.min(crate::MAX_NESTING),
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
) -> Result<NativeMatch<ReadExpression<H>, H::Predicate>, QueryError> {
    Reader {
        reader,
        host,
        budget,
        nesting: nesting.min(crate::MAX_NESTING),
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
) -> Result<NativePaths<H::Stage, H::Predicate>, QueryError> {
    Reader {
        reader,
        host,
        budget,
        nesting: nesting.min(crate::MAX_NESTING),
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
        budget,
        nesting: nesting.min(crate::MAX_NESTING),
        locals: Vec::new(),
    }
    .projection(0)
}

struct Reader<'a, 's, 'h, H> {
    reader: &'a mut ArgumentReader<'s>,
    host: &'h mut H,
    budget: &'h mut Budget,
    nesting: usize,
    locals: Vec<&'s str>,
}

impl<'s, H: ValueReaderHost> Reader<'_, 's, '_, H> {
    fn function_value(&mut self) -> Result<ValueExpression<ReadExpression<H>>, QueryError> {
        if let Some(value) =
            self.host
                .parse_function_value(self.reader, &self.locals, self.budget, self.nesting)?
        {
            return Ok(value);
        }
        self.expression(0, 0)
    }

    fn signature(&mut self, depth: usize) -> Result<Located<SignatureType>, QueryError> {
        use SignatureType as S;
        self.depth(depth)?;
        self.reader.skip_whitespace();
        let start = self.reader.position();
        let name = self.reader.word()?;
        self.budget
            .charge(name.value.len() + 1, Location::Query(name.range.clone()))?;
        let ty = match name.value {
            "int" => S::Integer,
            "float" => S::Float,
            "bool" => S::Boolean,
            "string" => S::String,
            "opt" => S::Optional(Box::new(self.signature(depth + 1)?)),
            "sequence" | "keyed" | "set" => {
                self.reader.expect('<')?;
                let element = self.signature(depth + 1)?;
                self.reader.expect('>')?;
                match name.value {
                    "sequence" => S::Sequence(Box::new(element)),
                    "keyed" => S::Keyed(Box::new(element)),
                    _ => {
                        if !matches!(
                            element.value,
                            S::Boolean | S::Integer | S::Float | S::Named(_)
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
                        self.reader.skip_whitespace();
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
        self.reader.skip_whitespace();
        let start = self.reader.position();
        let mut branches = Vec::new();
        loop {
            if !self.reader.keyword("if") {
                return Err(self.reader.error("expected `if`"));
            }
            let condition = ValueGuard::new(self.expression(0, depth + 1)?);
            self.reader.expect('{')?;
            let value = ValueGuard::new(self.expression(0, depth + 1)?);
            self.reader.expect('}')?;
            self.slots::<(
                ValueExpression<ReadExpression<H>>,
                ValueExpression<ReadExpression<H>>,
            )>(1)?;
            branches.push((condition.into_value(), value.into_value()));
            if !self.reader.keyword("else") {
                return Err(self.reader.error("conditional requires a final else"));
            }
            if !self.word_is("if") {
                break;
            }
        }
        let branches = ValueGuard::new(Located::new(
            K::Conditional {
                branches,
                otherwise: Box::new(Located::new(K::Literal(V::Absent), start..start)),
            },
            start..start,
        ));
        self.reader.expect('{')?;
        let otherwise = ValueGuard::new(self.expression(0, depth + 1)?);
        self.reader.expect('}')?;
        let K::Conditional { branches, .. } = branches.into_value().value else {
            unreachable!()
        };
        self.node(
            K::Conditional {
                branches,
                otherwise: Box::new(otherwise.into_value()),
            },
            start..self.reader.position(),
        )
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
    fn ordering_call(&mut self) -> Result<bool, QueryError> {
        let mut probe = ArgumentReader::new(self.reader.source(), self.reader.position())?;
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
        // Existing native *_by(selector) stages have one argument. Constructed
        // helpers have a top-level comma, independent of nested value syntax.
        let mut depth = 0usize;
        let (mut quoted, mut escaped, mut comment) = (false, false, false);
        for (offset, ch) in probe.remaining().char_indices() {
            let position = probe.position() + offset;
            self.budget
                .charge(1, Location::Query(position..position + ch.len_utf8()))?;
            if comment {
                comment = ch != '\n';
            } else if quoted {
                if !escaped && ch == '"' {
                    quoted = false;
                }
                escaped = !escaped && ch == '\\';
            } else {
                match ch {
                    '#' => comment = true,
                    '"' => quoted = true,
                    '(' | '[' | '{' => depth += 1,
                    ')' if depth == 0 => return Ok(false),
                    ')' | ']' | '}' => depth = depth.saturating_sub(1),
                    ',' if depth == 0 => return Ok(true),
                    _ => {}
                }
            }
        }
        Ok(false)
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
    fn match_call(&self) -> bool {
        let mut probe = ArgumentReader::new(self.reader.source(), self.reader.position())
            .expect("reader byte boundary");
        probe.word().is_ok_and(|word| word.value == "match") && probe.consume('{')
    }
    // A dotted scalar name stays a scalar field access unless it feeds a native
    // pipeline. Explicit native readers always admit the same postfix chain.
    fn field_pipeline(&self) -> bool {
        let mut probe = ArgumentReader::new(self.reader.source(), self.reader.position())
            .expect("reader byte boundary");
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
            self.reader.skip_whitespace();
            if minimum <= 10 && self.reader.consume('[') {
                let index = ValueGuard::new(self.expression(0, depth + 1)?);
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
                let field = self.name()?;
                let range = left.value().range.start..field.range.end;
                left = ValueGuard::new(self.node(
                    K::Field {
                        value: Box::new(left.into_value()),
                        field,
                    },
                    range,
                )?);
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
        Ok(left.into_value())
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
        if self.reader.remaining().starts_with("<>") {
            self.reader.advance(2)?;
            return self.node(K::Literal(V::Absent), start..self.reader.position());
        }
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
            if matches!(value.value, K::NativeQuery(_) | K::NativeValue(_)) {
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
        let local = self.is_local(word.value, &word.range)?;
        if word.value == "if" {
            // A saved name keeps its postfixes/operators. An adjacent expression
            // starts the conditional form even when that name is also in scope.
            let mut condition = ArgumentReader::new(self.reader.source(), probe.position())?;
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
        let ordering_call = self.ordering_call()?;
        if self.match_call()
            || self.field_pipeline()
            || (self.host.is_stage(word.value)
                && !ordering_call
                && ((!local && !self.host.is_binding(word.value)) || self.projection_call()))
            || (word.value == "input" && !local)
            || probe.remaining().starts_with('|')
        {
            return self.native(depth);
        }
        let name = self.name()?;
        let kind = match name.value.as_str() {
            "true" => K::Literal(V::Boolean(true)),
            "false" => K::Literal(V::Boolean(false)),
            "null" => K::Literal(V::Absent),
            "not" => K::Unary {
                operator: U::Not,
                operand: Box::new(self.expression(9, depth + 1)?),
            },
            _ if self.reader.consume('(') => {
                if !matches!(
                    name.value.as_str(),
                    "lookup"
                        | "has_key"
                        | "group_by"
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
                        | "require"
                        | "length"
                        | "sum"
                        | "forall"
                        | "exists"
                        | "occurs"
                        | "absent"
                        | "entries"
                        | "one"
                ) {
                    let mut arguments =
                        ValueGuard::new(Located::new(K::Array(Vec::new()), start..start));
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
                    if self.reader.consume('|') {
                        let range = call.range.clone();
                        let stage = self.host.value_source(call, range.clone())?;
                        let source = Located::new(
                            ExpressionKind::Sequence {
                                input: Box::new(Located::new(ExpressionKind::Input, start..start)),
                                steps: vec![Located::new(
                                    ExpressionStep::Stage(stage),
                                    range.clone(),
                                )],
                            },
                            range,
                        );
                        return self.native_tail(source, depth, true);
                    }
                    return Ok(call);
                }
                let value = ValueGuard::new(self.expression(0, depth + 1)?);
                if matches!(name.value.as_str(), "lookup" | "has_key") {
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
                    self.reader.expect(',')?;
                    let right = ValueGuard::new(self.expression(0, depth + 1)?);
                    self.reader.expect(')')?;
                    K::Binary {
                        operator: match name.value.as_str() {
                            "contains" => B::Contains,
                            "starts_with" => B::StartsWith,
                            "ends_with" => B::EndsWith,
                            _ => unreachable!("string matching call"),
                        },
                        left: Box::new(value.into_value()),
                        right: Box::new(right.into_value()),
                    }
                } else if matches!(
                    name.value.as_str(),
                    "sort" | "sort_by" | "min" | "max" | "min_by" | "max_by"
                ) {
                    let field = if name.value.ends_with("_by") {
                        self.reader.expect(',')?;
                        Some(self.string()?)
                    } else {
                        None
                    };
                    if matches!(name.value.as_str(), "sort" | "sort_by") {
                        let descending = self.descending_option()?;
                        self.reader.expect(')')?;
                        K::Sort {
                            array: Box::new(value.into_value()),
                            field,
                            descending,
                        }
                    } else {
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
        self.node(kind, start..self.reader.position())
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
            self.reader.skip_whitespace();
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
        let start = self.reader.position();
        self.reader.expect('[')?;
        if self.reader.consume(']') {
            return self.node(K::Array(Vec::new()), start..self.reader.position());
        }
        let scope = self.locals.len();
        let binders = self.array_binders()?;
        self.slots::<&str>(binders.len())?;
        self.locals.extend(binders);
        let head = self.expression(0, depth + 1);
        self.locals.truncate(scope);
        let head = ValueGuard::new(head?);
        if self.word_is("for") {
            let mut comprehension = ValueGuard::new(Located::new(
                K::Comprehension {
                    constructor: ValueConstructor::Array,
                    clauses: Vec::new(),
                    head: Box::new(head.into_value()),
                },
                start..start,
            ));
            let result = (|| {
                while self.reader.keyword("for") {
                    let clause_start = self.reader.position();
                    let name = self.name()?;
                    if !self.reader.keyword("in") {
                        return Err(self
                            .reader
                            .error("expected `in` after comprehension binder"));
                    }
                    let source = ValueGuard::new(self.expression(0, depth + 1)?);
                    if name.value != "_" {
                        self.slots::<&str>(1)?;
                        self.locals.push(&self.reader.source()[name.range.clone()]);
                    }
                    let filter = if self.reader.keyword("if") {
                        Some(ValueGuard::new(self.expression(0, depth + 1)?))
                    } else {
                        None
                    };
                    let end = filter
                        .as_ref()
                        .map_or(source.value().range.end, |value| value.value().range.end);
                    self.slots::<Located<ValueGenerator<ReadExpression<H>>>>(1)?;
                    self.slots::<Located<Option<String>>>(1)?;
                    let K::Comprehension { clauses, .. } = &mut comprehension.value_mut().value
                    else {
                        unreachable!("retained comprehension")
                    };
                    clauses.push(Located::new(
                        ValueGenerator::In {
                            binders: vec![Located::new(
                                (name.value != "_").then_some(name.value),
                                name.range,
                            )],
                            source: source.into_value(),
                            filter: filter.map(ValueGuard::into_value),
                        },
                        clause_start..end,
                    ));
                    self.depth(depth + clauses.len())?;
                }
                self.reader.expect(']')?;
                self.node(
                    comprehension.into_value().value,
                    start..self.reader.position(),
                )
            })();
            self.locals.truncate(scope);
            return result;
        }
        self.slots::<ValueExpression<ReadExpression<H>>>(1)?;
        let mut array = ValueGuard::new(Located::new(
            K::Array(vec![head.into_value()]),
            start..start,
        ));
        while self.reader.consume(',') {
            let value = ValueGuard::new(self.expression(0, depth + 1)?);
            self.slots::<ValueExpression<ReadExpression<H>>>(1)?;
            let K::Array(values) = &mut array.value_mut().value else {
                unreachable!("retained array values")
            };
            values.push(value.into_value());
        }
        self.reader.expect(']')?;
        self.node(array.into_value().value, start..self.reader.position())
    }
    // The head precedes its declarations. Inspect only this array's own level;
    // the normal parser still owns grammar and diagnostics.
    fn array_binders(&mut self) -> Result<Vec<&'s str>, QueryError> {
        let source = self.reader.source();
        let mut probe = ArgumentReader::new(source, self.reader.position())?;
        let mut names = Vec::new();
        let mut nested = 0usize;
        let mut quoted = false;
        let mut escaped = false;
        let mut comment = false;
        while let Some(character) = probe.remaining().chars().next() {
            let start = probe.position();
            self.budget.charge(
                character.len_utf8(),
                Location::Query(start..start + character.len_utf8()),
            )?;
            if comment {
                comment = character != '\n';
            } else if quoted {
                if character == '"' && !escaped {
                    quoted = false;
                }
                escaped = character == '\\' && !escaped;
            } else {
                match character {
                    '"' => quoted = true,
                    '#' => comment = true,
                    '(' | '[' | '{' => nested += 1,
                    ']' if nested == 0 => break,
                    ')' | ']' | '}' => nested = nested.saturating_sub(1),
                    character
                        if nested == 0 && (character.is_ascii_alphabetic() || character == '_') =>
                    {
                        let word = probe.word()?;
                        self.budget.charge(
                            word.range.len().saturating_sub(1),
                            Location::Query(word.range.clone()),
                        )?;
                        if word.value == "for" {
                            let mut next = ArgumentReader::new(source, probe.position())?;
                            let name = next.word().ok();
                            let generator = next.keyword("in");
                            self.budget.charge(
                                next.position() - probe.position(),
                                Location::Query(probe.position()..next.position()),
                            )?;
                            if let Some(name) = name.filter(|name| generator && name.value != "_") {
                                self.slots::<&str>(1)?;
                                names.push(name.value);
                            }
                        }
                        continue;
                    }
                    _ => {}
                }
            }
            probe.advance(character.len_utf8())?;
        }
        Ok(names)
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
        self.reader.skip_whitespace();
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
            self.slots::<Located<ExpressionStep<H::Stage, H::Emitter>>>(1)?;
            let stage = self.host.matching(
                matching.0.take().expect("native match source"),
                range.clone(),
            )?;
            Located::new(
                ExpressionKind::Sequence {
                    input: Box::new(Located::new(ExpressionKind::Input, start..start)),
                    steps: vec![Located::new(ExpressionStep::Stage(stage), range.clone())],
                },
                range,
            )
        } else if self.reader.consume('(') {
            let value = ValueGuard::new(self.expression(0, depth + 1)?);
            self.reader.expect(')')?;
            let range = start..self.reader.position();
            if !matches!(value.value().value, K::NativeQuery(_) | K::NativeValue(_)) {
                self.reader.skip_whitespace();
                if self.reader.remaining().starts_with('|') {
                    return Err(self.reader.error("pipeline input requires a native query"));
                }
                return Ok(Located::new(value.into_value().value, range));
            }
            let (K::NativeQuery(query) | K::NativeValue(query)) = value.into_value().value else {
                unreachable!("native group checked")
            };
            Located::new(ExpressionKind::Group(query), range)
        } else {
            let mut probe = ArgumentReader::new(self.reader.source(), start)?;
            let first = probe.word()?;
            probe.skip_whitespace();
            let projection_call = self.projection_call();
            let binding =
                self.is_local(first.value, &first.range)? || self.host.is_binding(first.value);
            if first.value == "input" && !binding {
                self.reader.advance(probe.position() - start)?;
                Located::new(ExpressionKind::Input, first.range)
            } else if !projection_call && (binding || !self.host.is_stage(first.value)) {
                self.reader.advance(probe.position() - start)?;
                self.budget
                    .charge(first.value.len(), Location::Query(first.range.clone()))?;
                Located::new(ExpressionKind::Binding(first.value.to_owned()), first.range)
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
                self.host
                    .parse_stage_in_scope(self.reader, &self.locals, self.budget)?
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
    fn matching(
        &mut self,
        depth: usize,
    ) -> Result<NativeMatch<ReadExpression<H>, H::Predicate>, QueryError> {
        self.depth(depth)?;
        self.reader.skip_whitespace();
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
                self.reader.skip_whitespace();
                let clause_start = self.reader.position();
                self.slots::<Located<MatchClause<ReadExpression<H>, H::Predicate>>>(1)?;
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
                        let local = &self.reader.source()[name.range.clone()];
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
                            predicate: self.predicate_or(depth + 1)?,
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
    fn paths(&mut self, depth: usize) -> Result<NativePaths<H::Stage, H::Predicate>, QueryError> {
        self.depth(depth)?;
        self.reader.skip_whitespace();
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
        let mut target = PredicateGuard(Some(self.predicate_or(depth + 1)?));
        self.reader.expect(',')?;
        if !self.reader.keyword("max_depth") {
            return Err(self.reader.error("expected `max_depth` edge bound"));
        }
        self.reader.expect('=')?;
        let max_depth = self.reader.unsigned_integer()?;
        self.reader.expect(')')?;
        self.slots::<NativePaths<H::Stage, H::Predicate>>(1)?;
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
            PredicateKind::Leaf(self.host.parse_predicate_leaf_in_scope(
                self.reader,
                &self.locals,
                self.budget,
            )?)
        };
        self.slots::<Predicate<H::Predicate>>(1)?;
        Ok(Located::new(value, start..self.reader.position()))
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
    let mut pending = vec![value];
    while let Some(value) = pending.pop() {
        let clauses = match value.value {
            K::Literal(_) | K::Name(_) => continue,
            K::NativeQuery(query) | K::NativeValue(query) => {
                crate::dispose_expression(*query);
                continue;
            }
            K::One(child)
            | K::Occurs(child)
            | K::Absent(child)
            | K::IndexSet(child)
            | K::Entries(child)
            | K::Unary { operand: child, .. }
            | K::Field { value: child, .. }
            | K::TupleAccess { value: child, .. }
            | K::GroupBy { rows: child, .. }
            | K::Reduce { array: child, .. }
            | K::Sort { array: child, .. }
            | K::Extremum { array: child, .. } => {
                pending.push(*child);
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
            | K::ZipExact { left, right }
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
            | K::Tuple(values)
            | K::Set(values)
            | K::Call {
                arguments: values, ..
            } => {
                pending.extend(values);
                continue;
            }
            K::Record(fields) => {
                pending.extend(fields.into_iter().map(|(_, value)| value));
                continue;
            }
            K::KeyedArray { entries, .. } => {
                for (key, value) in entries {
                    pending.push(key);
                    pending.push(value);
                }
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
            K::KeyedComprehension {
                clauses,
                key,
                head,
                empty_axis_sources,
            } => {
                pending.push(*key);
                pending.push(*head);
                pending.extend(empty_axis_sources.into_iter().flatten());
                clauses
            }
        };
        for clause in clauses {
            let (source, filter) = match clause.value {
                ValueGenerator::In { source, filter, .. }
                | ValueGenerator::Bind {
                    value: source,
                    filter,
                    ..
                } => (source, filter),
            };
            pending.push(source);
            pending.extend(filter);
        }
    }
}

struct MatchGuard<S, O, E, P>(Option<NativeMatch<Expression<S, O, E>, P>>);
impl<S, O, E, P> Drop for MatchGuard<S, O, E, P> {
    fn drop(&mut self) {
        if let Some(matching) = self.0.take() {
            crate::dispose_match(matching);
        }
    }
}
