//! Request-local named pure functions. Frontends supply located declarations;
//! calls use the ordinary value evaluator and retain native host values.
use crate::Pattern;
use crate::value::execution::{self, Body, Data, NativeStep, NativeView, Value};
use crate::value::pattern::PatternRef;
use crate::value::prepared::{Preparation, PreparedArgument, RuntimeValue};
use crate::{
    Bindings, Budget, ConstructedRead, ConstructedValue, DeclaredType, EvaluationError, Expression,
    ExpressionKind, Located, Location, NativeArguments, NativeExpression, ProgramHost,
    ProjectionHost, QueryError, RuntimeTypeHost, ValueExpression, ValueExpressionKind,
    ValueQueryHost, ValueType, admit_value, constructed_value_type, copy_constructed_value,
    copy_value_types, dispose_native_value, native_expression_kind, query_value_types,
};
use std::{collections::BTreeMap, fmt, ops::Range};

/// A whole argument is admitted before its immutable name is installed.
#[derive(Debug, PartialEq)]
pub struct FunctionParameter<K> {
    pub name: Located<String>,
    pub declaration: DeclaredType<K>,
}
/// Ordered local bindings and one final pure value. Signatures register before
/// any body is inspected, permitting direct and mutual recursion.
#[derive(Debug, PartialEq)]
pub struct FunctionDeclaration<Q = (), K = ()> {
    pub name: Located<String>,
    pub parameters: Vec<Located<FunctionParameter<K>>>,
    pub result: Located<DeclaredType<K>>,
    pub bindings: Vec<(Located<String>, ValueExpression<Q>)>,
    pub value: ValueExpression<Q>,
}
/// A resolved header borrowing an immutable body. A host can resolve nominal
/// types for its current request without cloning or reparsing expression trees.
pub struct FunctionDeclarationRef<'a, Q = (), K = ()> {
    pub name: &'a Located<String>,
    pub parameters: &'a [Located<FunctionParameter<K>>],
    pub result: &'a Located<DeclaredType<K>>,
    pub bindings: &'a [(Located<String>, ValueExpression<Q>)],
    pub value: &'a ValueExpression<Q>,
}
impl<Q, K> Copy for FunctionDeclarationRef<'_, Q, K> {}
impl<Q, K> Clone for FunctionDeclarationRef<'_, Q, K> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<Q, K> FunctionDeclaration<Q, K> {
    pub fn as_borrowed(&self) -> FunctionDeclarationRef<'_, Q, K> {
        FunctionDeclarationRef {
            name: &self.name,
            parameters: &self.parameters,
            result: &self.result,
            bindings: &self.bindings,
            value: &self.value,
        }
    }
}
/// A whole typed argument destructured only after all argument admission.
#[derive(Debug, PartialEq)]
pub struct PatternParameter<K> {
    pub pattern: Pattern,
    pub declaration: DeclaredType<K>,
}

/// Pattern-capable declarations use lexical `Block` values for local lets.
/// Existing name-only declarations remain accepted by their original APIs.
#[derive(Debug, PartialEq)]
pub struct PatternFunctionDeclaration<Q = (), K = ()> {
    pub name: Located<String>,
    pub parameters: Vec<Located<PatternParameter<K>>>,
    pub result: Located<DeclaredType<K>>,
    pub value: ValueExpression<Q>,
}

pub(crate) enum Parameters<'a, K> {
    Names(&'a [Located<FunctionParameter<K>>]),
    Patterns(&'a [Located<PatternParameter<K>>]),
}
impl<K> Copy for Parameters<'_, K> {}
impl<K> Clone for Parameters<'_, K> {
    fn clone(&self) -> Self {
        *self
    }
}
pub(crate) struct ParameterRef<'a, K> {
    pub pattern: PatternRef<'a>,
    pub declaration: &'a DeclaredType<K>,
    pub range: &'a Range<usize>,
}
impl<'a, K> Parameters<'a, K> {
    pub fn len(self) -> usize {
        match self {
            Self::Names(v) => v.len(),
            Self::Patterns(v) => v.len(),
        }
    }
    pub fn iter(self) -> impl Iterator<Item = ParameterRef<'a, K>> {
        (0..self.len()).map(move |index| match self {
            Self::Names(v) => ParameterRef {
                pattern: PatternRef::Name(&v[index].value.name),
                declaration: &v[index].value.declaration,
                range: &v[index].range,
            },
            Self::Patterns(v) => ParameterRef {
                pattern: PatternRef::Pattern(&v[index].value.pattern),
                declaration: &v[index].value.declaration,
                range: &v[index].range,
            },
        })
    }
}
pub(crate) struct DeclarationRef<'a, Q, K> {
    pub name: &'a Located<String>,
    pub parameters: Parameters<'a, K>,
    pub result: &'a Located<DeclaredType<K>>,
    pub bindings: &'a [(Located<String>, ValueExpression<Q>)],
    pub value: &'a ValueExpression<Q>,
}
impl<Q, K> Copy for DeclarationRef<'_, Q, K> {}
impl<Q, K> Clone for DeclarationRef<'_, Q, K> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<'a, Q, K> From<FunctionDeclarationRef<'a, Q, K>> for DeclarationRef<'a, Q, K> {
    fn from(d: FunctionDeclarationRef<'a, Q, K>) -> Self {
        Self {
            name: d.name,
            parameters: Parameters::Names(d.parameters),
            result: d.result,
            bindings: d.bindings,
            value: d.value,
        }
    }
}
impl<Q, K> PatternFunctionDeclaration<Q, K> {
    pub(crate) fn as_borrowed(&self) -> DeclarationRef<'_, Q, K> {
        DeclarationRef {
            name: &self.name,
            parameters: Parameters::Patterns(&self.parameters),
            result: &self.result,
            bindings: &[],
            value: &self.value,
        }
    }
}

/// The maximum simultaneously active user-function activations. Compatible
/// tail calls reuse one activation. Expression nesting and host reentry have
/// separate existing guards in [`Budget`]. Every call still spends work.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FunctionLimits {
    pub active_depth: usize,
}
/// A bounded active-call location; completed calls are never retained.
#[derive(Debug, PartialEq, Eq)]
pub struct FunctionCall {
    pub function: String,
    pub range: Range<usize>,
}
/// Original failure and at most sixteen active function locations (names are
/// capped at 64 characters), innermost
/// first. A tail activation records its current call rather than its history.
#[derive(Debug, PartialEq, Eq)]
pub struct FunctionError<E> {
    pub error: E,
    pub calls: Vec<FunctionCall>,
    pub omitted_calls: usize,
}
impl<E: fmt::Display> fmt::Display for FunctionError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.error.fmt(f)
    }
}
impl<E: std::error::Error + 'static> std::error::Error for FunctionError<E> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.error)
    }
}
impl<E: From<QueryError>> From<QueryError> for FunctionError<E> {
    fn from(error: QueryError) -> Self {
        Self {
            error: error.into(),
            calls: Vec::new(),
            omitted_calls: 0,
        }
    }
}

#[derive(Default)]
pub(crate) struct CallTrace {
    pub frames: Vec<(usize, Range<usize>)>,
    pub omitted: usize,
}
impl CallTrace {
    pub fn append(&mut self, calls: &[(usize, Range<usize>)], omitted: usize) {
        let keep = calls.len().min(16 - self.frames.len());
        self.frames.extend_from_slice(&calls[..keep]);
        self.omitted = self
            .omitted
            .saturating_add(omitted)
            .saturating_add(calls.len() - keep);
    }
}

pub(crate) struct Resolver<'h, 'a, Q, K, H> {
    pub queries: &'h mut H,
    pub declarations: &'a [DeclarationRef<'a, Q, K>],
}
impl<Q, K: PartialEq, H: ValueQueryHost<Q>> ValueQueryHost<Q> for Resolver<'_, '_, Q, K, H> {
    type Error = H::Error;
    fn function(
        &mut self,
        name: &Located<String>,
        budget: &mut Budget,
    ) -> Result<(usize, usize), QueryError> {
        for (id, d) in self.declarations.iter().enumerate() {
            budget.charge(
                1 + name.value.len().min(d.name.value.len()),
                Location::Query(name.range.clone()),
            )?;
            if name.value == d.name.value {
                return Ok((id, d.parameters.len()));
            }
        }
        Err(QueryError::query(
            name.range.clone(),
            format!("undefined function: {}", name.value),
        ))
    }
    fn function_types(
        &mut self,
        name: &Located<String>,
        budget: &mut Budget,
    ) -> Result<Vec<ValueType>, QueryError> {
        let (id, _) = self.function(name, budget)?;
        crate::value::function_facts(
            &self.declarations[id].result.value,
            budget,
            &Location::Query(name.range.clone()),
        )
    }
    fn accumulator_function(
        &mut self,
        step: &Located<String>,
        budget: &mut Budget,
    ) -> Result<(usize, Vec<ValueType>), QueryError> {
        let (id, _) = self.function(step, budget)?;
        let state = accumulator_state(&self.declarations[id], step, budget)?;
        Ok((
            id,
            crate::value::function_facts(state, budget, &Location::Query(step.range.clone()))?,
        ))
    }
    fn query_types(
        &mut self,
        q: &Q,
        l: &[(&str, Vec<ValueType>)],
        b: &mut Budget,
        r: &Range<usize>,
    ) -> Result<Vec<ValueType>, QueryError> {
        self.queries.query_types(q, l, b, r)
    }
    fn native_value_types(
        &mut self,
        q: &Q,
        l: &[(&str, Vec<ValueType>)],
        b: &mut Budget,
        r: &Range<usize>,
    ) -> Result<Vec<ValueType>, QueryError> {
        self.queries.native_value_types(q, l, b, r)
    }
    fn query_arguments<'q>(
        &self,
        q: &'q Q,
        b: &mut Budget,
        r: &Range<usize>,
    ) -> Result<Vec<&'q ValueExpression<Q>>, QueryError> {
        self.queries.query_arguments(q, b, r)
    }
    fn evaluate_query(
        &mut self,
        q: &Q,
        l: &[(&str, ConstructedValue)],
        b: &mut Budget,
    ) -> Result<ConstructedValue, Self::Error> {
        self.queries.evaluate_query(q, l, b)
    }
    fn read_binding(
        &mut self,
        n: &str,
        r: &Range<usize>,
        b: &mut Budget,
    ) -> Result<ConstructedValue, Self::Error> {
        self.queries.read_binding(n, r, b)
    }
}
fn accumulator_state<'a, Q, K: PartialEq>(
    declaration: &DeclarationRef<'a, Q, K>,
    step: &Located<String>,
    budget: &mut Budget,
) -> Result<&'a DeclaredType<K>, QueryError> {
    let location = Location::Query(step.range.clone());
    if declaration.parameters.len() != 2 {
        return Err(QueryError::query(
            step.range.clone(),
            "accumulator step needs two parameters",
        ));
    }
    let state = declaration
        .parameters
        .iter()
        .next()
        .expect("two parameters")
        .declaration;
    if !crate::runtime_types::compatible_state(state, &declaration.result.value, budget, &location)?
    {
        return Err(QueryError::query(
            step.range.clone(),
            "accumulator result is incompatible with state",
        ));
    }
    Ok(state)
}

fn register<'a, Q: 'a, K: 'a>(
    declarations: impl Clone + Iterator<Item = DeclarationRef<'a, Q, K>>,
    budget: &mut Budget,
    mut conflict: impl FnMut(&str) -> bool,
) -> Result<(), QueryError> {
    for (index, d) in declarations.clone().enumerate() {
        let location = Location::Query(d.name.range.clone());
        budget.charge(d.name.value.len() + 1, location.clone())?;
        if d.name.value == "_" || builtin(&d.name.value) || conflict(&d.name.value) {
            return Err(QueryError::query(
                d.name.range.clone(),
                "function conflicts with a reserved operation name",
            ));
        }
        for previous in declarations.clone().take(index) {
            budget.charge(
                1 + d.name.value.len().min(previous.name.value.len()),
                location.clone(),
            )?;
            if d.name.value == previous.name.value {
                return Err(QueryError::query(
                    d.name.range.clone(),
                    "function is already defined",
                ));
            }
        }
        for p in d.parameters.iter() {
            crate::runtime_types::validate_declaration(
                p.declaration,
                budget,
                &Location::Query(p.range.clone()),
            )?;
        }
        crate::runtime_types::validate_declaration(
            &d.result.value,
            budget,
            &Location::Query(d.result.range.clone()),
        )?;
    }
    Ok(())
}
fn builtin(name: &str) -> bool {
    matches!(
        name,
        "fold"
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
            | "starts_with"
            | "ends_with"
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
            | "require"
            | "project"
            | "match"
            | "paths"
            | "default"
            | "if"
    )
}

/// Execute constructed functions without a native host. The same runner powers
/// [`FunctionRequest`]; declarations do not capture surrounding bindings.
///
/// ```
/// use zirql::{Bindings, Budget, ConstructedValue as V, DeclaredType as D,
///     FunctionDeclaration, FunctionParameter, FunctionLimits, Located,
///     ProgramLimits, ValueExpression as E, ValueExpressionKind as K,
///     ValueBinaryOperator as B, evaluate_functions};
/// fn e(value: K) -> E { Located::new(value, 0..1) }
/// fn name(value: &str) -> E { e(K::Name(value.into())) }
/// fn int(value: i64) -> E { e(K::Literal(V::Integer(value))) }
/// fn op(operator: B, left: E, right: E) -> E {
///     e(K::Binary { operator, left: Box::new(left), right: Box::new(right) })
/// }
/// fn call(arguments: Vec<E>) -> E {
///     e(K::Call { function: Located::new("acc".into(), 0..1), arguments })
/// }
/// let parameters = ["n", "total"].map(|name| Located::new(
///     FunctionParameter { name: Located::new(name.into(), 0..1),
///                         declaration: D::Integer }, 0..1));
/// let functions = [FunctionDeclaration {
///     name: Located::new("acc".into(), 0..1),
///     parameters: Vec::from(parameters), result: Located::new(D::Integer, 0..1),
///     bindings: vec![],
///     value: e(K::Conditional {
///         branches: vec![(op(B::Equal, name("n"), int(0)), name("total"))],
///         otherwise: Box::new(call(vec![op(B::Subtract, name("n"), int(1)),
///                                      op(B::Add, name("total"), name("n"))])),
///     }),
/// }];
/// let mut budget = Budget::for_program(ProgramLimits {
///     work: 1_000_000, results: 100_000, nesting: 64, stream_items: 100,
/// });
/// let value = evaluate_functions(&functions, &call(vec![int(100), int(0)]),
///     &Bindings::default(), &mut budget, FunctionLimits { active_depth: 1 })?;
/// assert_eq!(value.data(), &V::Integer(5050));
/// # Ok::<(), zirql::FunctionError<zirql::QueryError>>(())
/// ```
///
/// # Errors
/// Preflight checks every body. Runtime contracts, active depth and request
/// allowances fail at the responsible expression and preserve spent work.
pub fn evaluate_functions(
    declarations: &[FunctionDeclaration],
    entry: &ValueExpression,
    bindings: &Bindings<ConstructedValue>,
    budget: &mut Budget,
    limits: FunctionLimits,
) -> Result<ConstructedValue, FunctionError<QueryError>> {
    register(
        declarations.iter().map(|d| d.as_borrowed().into()),
        budget,
        |_| false,
    )?;
    crate::value::run_functions(declarations, entry, bindings, budget, limits)
}

/// Execute pattern-capable constructed functions through the ordinary runner.
///
/// # Errors
/// Every declaration is preflighted. Whole argument admission precedes atomic
/// parameter matching; mismatches fail at the parameter pattern.
pub fn evaluate_pattern_functions(
    declarations: &[PatternFunctionDeclaration],
    entry: &ValueExpression,
    bindings: &Bindings<ConstructedValue>,
    budget: &mut Budget,
    limits: FunctionLimits,
) -> Result<ConstructedValue, FunctionError<QueryError>> {
    register(
        declarations
            .iter()
            .map(PatternFunctionDeclaration::as_borrowed),
        budget,
        |_| false,
    )?;
    let mut retained = Vec::new();
    for d in declarations {
        crate::projection::admit_slots::<DeclarationRef<'_, (), ()>>(1, budget, &d.name.range)?;
        retained.push(d.as_borrowed());
    }
    crate::value::run_function_refs(&retained, entry, bindings, budget, limits)
}

/// Missing concrete function hooks over the existing projection/admission APIs.
/// Existing hosts opt in only when they expose named functions.
pub trait FunctionHost<C>:
    ProjectionHost<C> + RuntimeTypeHost<C, Kind = <Self as ProjectionHost<C>>::Kind>
{
    /// Map an explicit signature to the host's existing preflight kinds.
    fn declaration_kind(
        &self,
        declaration: &DeclaredType<<Self as ProjectionHost<C>>::Kind>,
        budget: &mut Budget,
        range: &Range<usize>,
    ) -> Result<<Self as ProjectionHost<C>>::Kind, QueryError>;
    /// Recognize registered domain operation names, including effectful ones.
    fn is_operation_name(&self, name: &str) -> bool;

    /// Enumerate one eager positional native container in order, preserving
    /// native identity and coordinates. Reject keyed/non-positional storage and
    /// incompatible empty element metadata. Charge scans and output allocation.
    fn sequence_items(
        &self,
        _value: &Self::Value,
        _element: &DeclaredType<<Self as ProjectionHost<C>>::Kind>,
        _context: &mut C,
        _budget: &mut Budget,
        range: &Range<usize>,
    ) -> Result<Vec<Self::Value>, EvaluationError<Self::Error>> {
        Err(QueryError::query(range.clone(), "native positional sequences are unsupported").into())
    }

    /// Construct a positional sequence of immutable native scan snapshots.
    /// Preserve the supplied element contract even for empty output, retaining
    /// each value's identity/coordinates and charging host allocation.
    fn construct_sequence(
        &self,
        _values: Vec<Self::Value>,
        _element: &DeclaredType<<Self as ProjectionHost<C>>::Kind>,
        _context: &mut C,
        _budget: &mut Budget,
        range: &Range<usize>,
    ) -> Result<Self::Value, EvaluationError<Self::Error>> {
        Err(QueryError::query(range.clone(), "native scan construction is unsupported").into())
    }

    /// Successful native admission remains valid for an unchanged immutable
    /// value and equivalent contract throughout this request. Opt in only when
    /// ownership, coordinates, element contents and handle validity cannot
    /// change during execution. Fresh values still receive whole admission.
    /// Hosts with mutable native state retain admission at every boundary.
    fn retains_native_admission(&self) -> bool {
        false
    }
}
type Kind<C, H> = <H as ProjectionHost<C>>::Kind;
type Query<C, H> = NativeExpression<C, H>;
type HostValue<'a, C, H> = Value<'a, <H as ProgramHost<C>>::Value, Kind<C, H>>;
type Bodies<'a, C, H> = [Body<'a, Query<C, H>, Kind<C, H>>];
type Declaration<C, H> = FunctionDeclaration<Query<C, H>, Kind<C, H>>;
type PatternDeclaration<C, H> = PatternFunctionDeclaration<Query<C, H>, Kind<C, H>>;
struct LexicalKinds<K> {
    scope: BTreeMap<String, K>,
    generators: BTreeMap<String, K>,
}
struct Operand<K> {
    native: crate::projection::prepared::PreparedNative,
    names: Vec<String>,
    kind: K,
}
struct Preflight<'h, 'a, C, H: FunctionHost<C>> {
    host: &'h H,
    declarations: Vec<DeclarationRef<'a, Query<C, H>, Kind<C, H>>>,
    scope: BTreeMap<String, Kind<C, H>>,
    generator_kinds: BTreeMap<String, Kind<C, H>>,
    relative: Option<Kind<C, H>>,
    prepared: BTreeMap<usize, Operand<Kind<C, H>>>,
    scopes: Vec<LexicalKinds<Kind<C, H>>>,
}
impl<C, H: FunctionHost<C>> Preflight<'_, '_, C, H> {
    fn native(
        &mut self,
        query: &Query<C, H>,
        locals: &[(&str, Vec<ValueType>)],
        budget: &mut Budget,
        projected: bool,
    ) -> Result<Vec<ValueType>, QueryError> {
        let mut scope = crate::projection::copy_scope::<C, H>(&self.scope, budget, &query.range)?;
        for (name, types) in locals {
            if !scope.contains_key(*name) {
                budget.charge(name.len() + 1, Location::Query(query.range.clone()))?;
                scope.insert(
                    (*name).to_owned(),
                    match self.generator_kinds.get(*name) {
                        Some(kind) => kind.clone(),
                        None => self.host.constructed_kind(copy_value_types(
                            types,
                            budget,
                            Location::Query(query.range.clone()),
                        )?),
                    },
                );
            }
        }
        if self.relative.is_none() {
            no_ambient_input::<C, H>(query, self.host, budget)?;
        }
        let input = self
            .relative
            .clone()
            .unwrap_or_else(|| self.host.constructed_kind(vec![ValueType::Unestablished]));
        let kind = native_expression_kind(query, self.host, &input, &scope, budget)?;
        crate::projection::admit_slots::<String>(scope.len(), budget, &query.range)?;
        let names = scope.keys().map(String::as_str).collect::<Vec<_>>();
        let native = crate::projection::prepared::PreparedNative::resolve::<C, H>(
            query,
            self.host,
            &names,
            None,
            &[],
            budget,
        )?;
        let names = scope
            .keys()
            .map(|name| {
                budget.charge(name.len(), Location::Query(query.range.clone()))?;
                Ok(name.clone())
            })
            .collect::<Result<_, QueryError>>()?;
        budget.charge(1, Location::Query(query.range.clone()))?;
        self.prepared.insert(
            query as *const _ as usize,
            Operand {
                native,
                names,
                kind: kind.clone(),
            },
        );
        let arguments =
            crate::projection::scoped_arguments::<C, H>(query, self.host, &input, &scope, budget)?;
        for argument in arguments {
            let saved = std::mem::replace(
                &mut self.scope,
                crate::projection::copy_scope::<C, H>(&scope, budget, &query.range)?,
            );
            for (name, kind) in argument.locals {
                budget_name(&name.value, budget, &name.range)?;
                if self.scope.insert(name.value, kind).is_some() {
                    return Err(QueryError::query(name.range, "binding is already defined"));
                }
            }
            let old = std::mem::replace(&mut self.relative, argument.relative);
            let mut facts = BTreeMap::new();
            for (name, kind) in &self.scope {
                budget.charge(name.len() + 1, Location::Query(query.range.clone()))?;
                facts.insert(
                    name.clone(),
                    self.host
                        .scalar_types(kind, budget, &query.range)?
                        .unwrap_or_else(|| vec![ValueType::Unestablished]),
                );
            }
            let result = query_value_types(argument.expression, &facts, self, budget);
            self.scope = saved;
            self.relative = old;
            result?;
        }
        match self.host.scalar_types(&kind, budget, &query.range)? {
            Some(types) => Ok(types),
            None if !projected => Ok(vec![ValueType::Unestablished]),
            None => Err(QueryError::query(
                query.range.clone(),
                "native handles require an explicit scalar projection",
            )),
        }
    }
}
impl<C, H: FunctionHost<C>> Preflight<'_, '_, C, H> {
    // Retain already-known native kinds through the same selected-value forms.
    // Unknown scalar/collection facts continue through ordinary value analysis.
    fn pattern_kind(
        &mut self,
        pattern: PatternRef<'_>,
        kind: Option<Kind<C, H>>,
        declaration: Option<&DeclaredType<Kind<C, H>>>,
        budget: &mut Budget,
    ) -> Result<(), QueryError> {
        let range = pattern.range();
        budget.within_evaluation(1, Location::Query(range.clone()), |budget| {
            budget.charge(1, Location::Query(range.clone()))?;
            let node = match pattern {
                PatternRef::Name(name) => {
                    if let Some(kind) = kind {
                        budget_name(&name.value, budget, &name.range)?;
                        self.scope.insert(name.value.clone(), kind);
                    }
                    return Ok(());
                }
                PatternRef::Pattern(p) => &p.value,
            };
            match node {
                crate::PatternKind::Name(name) => {
                    if let Some(kind) = kind {
                        budget_name(&name.value, budget, &name.range)?;
                        self.scope.insert(name.value.clone(), kind);
                    }
                }
                crate::PatternKind::Present(child) => {
                    let declaration = match declaration {
                        Some(DeclaredType::Optional(child)) => Some(child.as_ref()),
                        other => other,
                    };
                    let kind = declaration
                        .map(|d| self.host.declaration_kind(d, budget, range))
                        .transpose()?
                        .or(kind);
                    self.pattern_kind(PatternRef::Pattern(child), kind, declaration, budget)?;
                }
                crate::PatternKind::Tuple(children) => {
                    let declaration = declaration.map(|mut d| {
                        while let DeclaredType::Optional(child) = d {
                            d = child;
                        }
                        d
                    });
                    for (index, child) in children.iter().enumerate() {
                        let d = match declaration {
                            Some(DeclaredType::Tuple(v)) => v.get(index),
                            _ => None,
                        };
                        let k = match d {
                            Some(d) => Some(self.host.declaration_kind(d, budget, range)?),
                            None => kind
                                .as_ref()
                                .and_then(|k| self.host.tuple_view_kind(k))
                                .and_then(|v| v.get(index))
                                .cloned(),
                        };
                        self.pattern_kind(PatternRef::Pattern(child), k, d, budget)?;
                    }
                }
                crate::PatternKind::Record { fields, .. } => {
                    let declaration = declaration.map(|mut d| {
                        while let DeclaredType::Optional(child) = d {
                            d = child;
                        }
                        d
                    });
                    for (label, child) in fields {
                        let d = match declaration {
                            Some(DeclaredType::Record(v)) => {
                                let mut found = None;
                                for (name, d) in v {
                                    budget.charge(
                                        1 + name.len().min(label.value.len()),
                                        Location::Query(label.range.clone()),
                                    )?;
                                    if name == &label.value {
                                        found = Some(d);
                                        break;
                                    }
                                }
                                found
                            }
                            _ => None,
                        };
                        let k = if let Some(d) = d {
                            Some(self.host.declaration_kind(d, budget, &label.range)?)
                        } else {
                            let mut found = None;
                            if let Some(fields) =
                                kind.as_ref().and_then(|k| self.host.capture_record_kind(k))
                            {
                                for (name, k) in &fields.fields {
                                    budget.charge(
                                        1 + name.len().min(label.value.len()),
                                        Location::Query(label.range.clone()),
                                    )?;
                                    if name == &label.value {
                                        found = Some(k.clone());
                                        break;
                                    }
                                }
                            }
                            found
                        };
                        self.pattern_kind(PatternRef::Pattern(child), k, d, budget)?;
                    }
                }
                crate::PatternKind::Wildcard
                | crate::PatternKind::Literal(_)
                | crate::PatternKind::Absent => {}
            }
            Ok(())
        })
    }

    fn local_kind(
        &mut self,
        value: &ValueExpression<Query<C, H>>,
        budget: &mut Budget,
    ) -> Result<Option<Kind<C, H>>, QueryError> {
        budget.within_evaluation(1, Location::Query(value.range.clone()), |budget| {
            budget.charge(1, Location::Query(value.range.clone()))?;
            Ok(match &value.value {
                ValueExpressionKind::Call { function, .. } => {
                    let (id, _) = self.function(function, budget)?;
                    Some(self.host.declaration_kind(
                        &self.declarations[id].result.value,
                        budget,
                        &value.range,
                    )?)
                }
                ValueExpressionKind::Fold { step, .. } => {
                    let (id, _) = self.function(step, budget)?;
                    let state = accumulator_state(&self.declarations[id], step, budget)?;
                    Some(self.host.declaration_kind(state, budget, &value.range)?)
                }
                ValueExpressionKind::Scan { step, .. } => {
                    let (id, _) = self.function(step, budget)?;
                    let state = accumulator_state(&self.declarations[id], step, budget)?;
                    let sequence =
                        DeclaredType::Sequence(Box::new(crate::runtime_types::copy_declaration(
                            state,
                            budget,
                            &Location::Query(value.range.clone()),
                        )?));
                    Some(
                        self.host
                            .declaration_kind(&sequence, budget, &value.range)?,
                    )
                }
                ValueExpressionKind::NativeValue(query) => Some(
                    self.prepared
                        .get(&(query.as_ref() as *const _ as usize))
                        .expect("prepared native let")
                        .kind
                        .clone(),
                ),
                ValueExpressionKind::Name(name) => self
                    .scope
                    .get(name)
                    .or_else(|| self.generator_kinds.get(name))
                    .cloned(),
                ValueExpressionKind::Block { bindings, value } => {
                    self.enter_scope(budget, &value.range)?;
                    let result = (|| {
                        for (pattern, source) in bindings {
                            let kind = self.local_kind(source, budget)?;
                            self.pattern_kind(PatternRef::Pattern(pattern), kind, None, budget)?;
                        }
                        self.local_kind(value, budget)
                    })();
                    self.leave_scope();
                    result?
                }
                ValueExpressionKind::ValueMatch { subject, arms } => {
                    let subject = self.local_kind(subject, budget)?;
                    let mut selected = None;
                    for (index, arm) in arms.iter().enumerate() {
                        self.enter_scope(budget, &arm.pattern.range)?;
                        let result = (|| {
                            self.pattern_kind(
                                PatternRef::Pattern(&arm.pattern),
                                subject.clone(),
                                None,
                                budget,
                            )?;
                            self.local_kind(&arm.value, budget)
                        })();
                        self.leave_scope();
                        let kind = result?;
                        if index > 0 && selected != kind {
                            return Ok(None);
                        }
                        selected = kind;
                    }
                    selected
                }
                ValueExpressionKind::Require { value, .. } => self.local_kind(value, budget)?,
                ValueExpressionKind::Conditional {
                    branches,
                    otherwise,
                } => {
                    let kind = self.local_kind(otherwise, budget)?;
                    for (_, value) in branches {
                        if self.local_kind(value, budget)? != kind {
                            return Ok(None);
                        }
                    }
                    kind
                }
                ValueExpressionKind::Default { value, fallback } => {
                    let right = self.local_kind(fallback, budget)?;
                    if matches!(
                        value.value,
                        ValueExpressionKind::Literal(ConstructedValue::Absent)
                    ) || self.local_kind(value, budget)? == right
                    {
                        right
                    } else {
                        None
                    }
                }
                _ => None,
            })
        })
    }
}

fn no_ambient_input<C, H: ProjectionHost<C>>(
    query: &Query<C, H>,
    host: &H,
    budget: &mut Budget,
) -> Result<(), QueryError> {
    let mut pending = vec![query];
    while let Some(query) = pending.pop() {
        budget.charge(1, Location::Query(query.range.clone()))?;
        let count = match &query.value {
            ExpressionKind::Set { .. } => 2,
            ExpressionKind::Record(fields) => fields.len(),
            ExpressionKind::Binding(_) | ExpressionKind::Input => 0,
            _ => 1,
        };
        crate::projection::admit_slots::<&Query<C, H>>(count, budget, &query.range)?;
        match &query.value {
            ExpressionKind::Input => {
                return Err(QueryError::query(
                    query.range.clone(),
                    "functions require explicit native inputs",
                ));
            }
            ExpressionKind::Group(value) | ExpressionKind::Field { value, .. } => {
                pending.push(value)
            }
            ExpressionKind::Sequence { input, steps } => {
                let explicit = steps.first().is_some_and(|step| match &step.value {
                    crate::ExpressionStep::Stage(stage) => host.is_explicit_source(stage),
                    crate::ExpressionStep::Emit { .. } => false,
                });
                if !explicit {
                    pending.push(input);
                }
            }
            ExpressionKind::Set { left, right, .. } => {
                pending.push(right);
                pending.push(left);
            }
            ExpressionKind::Record(fields) => pending.extend(fields.iter().map(|(_, v)| v)),
            ExpressionKind::Binding(_) => {}
        }
    }
    Ok(())
}
impl<C, H: FunctionHost<C>> ValueQueryHost<Query<C, H>> for Preflight<'_, '_, C, H> {
    type Error = EvaluationError<H::Error>;
    fn function(
        &mut self,
        name: &Located<String>,
        budget: &mut Budget,
    ) -> Result<(usize, usize), QueryError> {
        for (id, d) in self.declarations.iter().enumerate() {
            budget.charge(
                1 + name.value.len().min(d.name.value.len()),
                Location::Query(name.range.clone()),
            )?;
            if name.value == d.name.value {
                return Ok((id, d.parameters.len()));
            }
        }
        Err(QueryError::query(
            name.range.clone(),
            format!("undefined function: {}", name.value),
        ))
    }
    fn function_types(
        &mut self,
        name: &Located<String>,
        budget: &mut Budget,
    ) -> Result<Vec<ValueType>, QueryError> {
        let (id, _) = self.function(name, budget)?;
        crate::value::function_facts(
            &self.declarations[id].result.value,
            budget,
            &Location::Query(name.range.clone()),
        )
    }
    fn accumulator_function(
        &mut self,
        step: &Located<String>,
        budget: &mut Budget,
    ) -> Result<(usize, Vec<ValueType>), QueryError> {
        let (id, _) = self.function(step, budget)?;
        let state = accumulator_state(&self.declarations[id], step, budget)?;
        Ok((
            id,
            crate::value::function_facts(state, budget, &Location::Query(step.range.clone()))?,
        ))
    }
    fn query_types(
        &mut self,
        q: &Query<C, H>,
        l: &[(&str, Vec<ValueType>)],
        b: &mut Budget,
        _: &Range<usize>,
    ) -> Result<Vec<ValueType>, QueryError> {
        self.native(q, l, b, true)
    }
    fn native_value_types(
        &mut self,
        q: &Query<C, H>,
        l: &[(&str, Vec<ValueType>)],
        b: &mut Budget,
        _: &Range<usize>,
    ) -> Result<Vec<ValueType>, QueryError> {
        self.native(q, l, b, false)
    }
    fn enter_scope(&mut self, budget: &mut Budget, range: &Range<usize>) -> Result<(), QueryError> {
        let scope = crate::projection::copy_scope::<C, H>(&self.scope, budget, range)?;
        let generators =
            crate::projection::copy_scope::<C, H>(&self.generator_kinds, budget, range)?;
        crate::projection::admit_slots::<LexicalKinds<Kind<C, H>>>(1, budget, range)?;
        self.scopes.push(LexicalKinds { scope, generators });
        Ok(())
    }
    fn leave_scope(&mut self) {
        let saved = self.scopes.pop().expect("entered lexical scope");
        self.scope = saved.scope;
        self.generator_kinds = saved.generators;
    }
    fn prepare_pattern(
        &mut self,
        pattern: &Pattern,
        source: &ValueExpression<Query<C, H>>,
        budget: &mut Budget,
    ) -> Result<(), QueryError> {
        let kind = self.local_kind(source, budget)?;
        self.pattern_kind(PatternRef::Pattern(pattern), kind, None, budget)
    }
    fn prepare_local(
        &mut self,
        name: &Located<String>,
        value: &ValueExpression<Query<C, H>>,
        types: &[ValueType],
        budget: &mut Budget,
    ) -> Result<(), QueryError> {
        let kind = match self.local_kind(value, budget)? {
            Some(kind) => kind,
            None => self.host.constructed_kind(copy_value_types(
                types,
                budget,
                Location::Query(name.range.clone()),
            )?),
        };
        budget.charge(name.value.len() + 1, Location::Query(name.range.clone()))?;
        self.scope.insert(name.value.clone(), kind);
        Ok(())
    }
    fn prepare_generator(
        &mut self,
        binder: &Located<Option<String>>,
        source: &ValueExpression<Query<C, H>>,
        types: &[ValueType],
        budget: &mut Budget,
    ) -> Result<(), QueryError> {
        if let Some(name) = &binder.value {
            let kind = match self.local_kind(source, budget)? {
                Some(kind)
                    if self
                        .host
                        .scalar_types(&kind, budget, &source.range)?
                        .is_none() =>
                {
                    self.host.subject_kind(&kind, budget, &source.range)?
                }
                _ => self.host.constructed_kind(copy_value_types(
                    types,
                    budget,
                    Location::Query(binder.range.clone()),
                )?),
            };
            budget_name(name, budget, &binder.range)?;
            self.generator_kinds.insert(name.clone(), kind);
        }
        Ok(())
    }
    fn query_arguments<'q>(
        &self,
        query: &'q Query<C, H>,
        budget: &mut Budget,
        range: &Range<usize>,
    ) -> Result<Vec<&'q ValueExpression<Query<C, H>>>, QueryError> {
        let arguments = crate::projection::declared_arguments::<C, H>(query, self.host, budget)?;
        crate::projection::admit_slots::<&ValueExpression<Query<C, H>>>(
            arguments.len(),
            budget,
            range,
        )?;
        Ok(arguments.into_iter().map(|(query, _)| query).collect())
    }
    fn evaluate_query(
        &mut self,
        _: &Query<C, H>,
        _: &[(&str, ConstructedValue)],
        _: &mut Budget,
    ) -> Result<ConstructedValue, Self::Error> {
        unreachable!("preflight executes no native operations")
    }
    fn read_binding(
        &mut self,
        _: &str,
        _: &Range<usize>,
        _: &mut Budget,
    ) -> Result<ConstructedValue, Self::Error> {
        unreachable!("preflight reads no values")
    }
}

/// Resolved functions and native operands shared by one submitted program.
/// Preparation borrows immutable source trees and retains no execution results.
/// The host must use this unit only with its current request context.
pub struct PreparedFunctions<'a, 'h, C, H: FunctionHost<C>> {
    bodies: Vec<Body<'a, Query<C, H>, Kind<C, H>>>,
    preflight: Preflight<'h, 'a, C, H>,
    body_prepared: Vec<BTreeMap<usize, Operand<Kind<C, H>>>>,
    identity: std::sync::Arc<()>,
}

/// One written value, resolved once in its exact lexical and relative scope.
/// Execute it through the same [`PreparedFunctions`] that prepared it.
pub struct PreparedFunctionValue<'a, Q, K = ()> {
    kind: K,
    prepared: BTreeMap<usize, Operand<K>>,
    value: crate::value::prepared::PreparedValue<'a, Q>,
    names: Vec<&'a str>,
    identity: std::sync::Arc<()>,
}

impl<Q, K> PreparedFunctionValue<'_, Q, K> {
    /// Preflight kind for the written value; runtime whole-type admission still
    /// checks actual arguments and results.
    pub fn kind(&self) -> &K {
        &self.kind
    }
}

impl<'a, 'h, C, H: FunctionHost<C>> PreparedFunctions<'a, 'h, C, H> {
    /// Register signatures and prepare every body, including unused functions.
    ///
    /// # Errors
    /// Invalid declarations, scope, effects and allowances fail before execution.
    pub fn prepare(
        declarations: impl IntoIterator<Item = &'a Declaration<C, H>>,
        host: &'h H,
        budget: &mut Budget,
    ) -> Result<Self, FunctionError<EvaluationError<H::Error>>> {
        Self::prepare_borrowed(
            declarations
                .into_iter()
                .map(FunctionDeclaration::as_borrowed),
            host,
            budget,
        )
    }

    /// Prepare request-resolved headers that borrow existing body trees.
    /// Scope, purity, type and allowance checks are identical to [`Self::prepare`].
    ///
    /// # Errors
    /// Invalid declarations or written bodies fail before execution.
    pub fn prepare_borrowed(
        declarations: impl IntoIterator<Item = FunctionDeclarationRef<'a, Query<C, H>, Kind<C, H>>>,
        host: &'h H,
        budget: &mut Budget,
    ) -> Result<Self, FunctionError<EvaluationError<H::Error>>> {
        Self::prepare_headers(
            declarations.into_iter().map(DeclarationRef::from),
            host,
            budget,
        )
    }

    /// Prepare native functions with destructured whole typed parameters.
    ///
    /// # Errors
    /// Scope, views, purity and contracts are checked before execution.
    pub fn prepare_patterns(
        declarations: impl IntoIterator<Item = &'a PatternFunctionDeclaration<Query<C, H>, Kind<C, H>>>,
        host: &'h H,
        budget: &mut Budget,
    ) -> Result<Self, FunctionError<EvaluationError<H::Error>>> {
        Self::prepare_headers(
            declarations
                .into_iter()
                .map(PatternFunctionDeclaration::as_borrowed),
            host,
            budget,
        )
    }

    fn prepare_headers(
        declarations: impl IntoIterator<Item = DeclarationRef<'a, Query<C, H>, Kind<C, H>>>,
        host: &'h H,
        budget: &mut Budget,
    ) -> Result<Self, FunctionError<EvaluationError<H::Error>>> {
        let mut retained = Vec::new();
        for declaration in declarations {
            crate::projection::admit_slots::<DeclarationRef<'_, Query<C, H>, Kind<C, H>>>(
                1,
                budget,
                &declaration.name.range,
            )?;
            retained.push(declaration);
        }
        register(retained.iter().copied(), budget, |name| {
            host.is_operation_name(name)
        })?;
        let mut preflight = Preflight {
            host,
            declarations: retained,
            scope: BTreeMap::new(),
            generator_kinds: BTreeMap::new(),
            relative: None,
            prepared: BTreeMap::new(),
            scopes: Vec::new(),
        };
        crate::projection::admit_slots::<Body<'_, Query<C, H>, Kind<C, H>>>(
            preflight.declarations.len(),
            budget,
            &(0..0),
        )?;
        let mut bodies = Vec::with_capacity(preflight.declarations.len());
        crate::projection::admit_slots::<BTreeMap<usize, Operand<Kind<C, H>>>>(
            preflight.declarations.len(),
            budget,
            &(0..0),
        )?;
        let mut body_prepared = Vec::with_capacity(preflight.declarations.len());
        for index in 0..preflight.declarations.len() {
            let declaration = preflight.declarations[index];
            preflight.scope.clear();
            preflight.generator_kinds.clear();
            for p in declaration.parameters.iter() {
                let kind = host.declaration_kind(p.declaration, budget, p.range)?;
                preflight.pattern_kind(p.pattern, Some(kind), Some(p.declaration), budget)?;
            }
            bodies.push(crate::value::prepare_function_body(
                declaration,
                &mut preflight,
                budget,
            )?);
            body_prepared.push(std::mem::take(&mut preflight.prepared));
        }
        preflight.scope.clear();
        budget.charge_results(1, Location::Query(0..0))?;
        Ok(Self {
            bodies,
            preflight,
            body_prepared,
            identity: std::sync::Arc::new(()),
        })
    }

    /// Resolve one written value without executing native operations. Call this
    /// for all program values before any statement runs. Functions cannot capture
    /// this entry's scope. Relative native input is explicit and optional.
    ///
    /// # Errors
    /// All written branches and embedded native arguments are checked.
    #[expect(
        clippy::type_complexity,
        reason = "The prepared entry retains the host's existing native expression and kind types"
    )]
    pub fn prepare_value(
        &mut self,
        entry: &'a ValueExpression<Query<C, H>>,
        scope: &BTreeMap<String, Kind<C, H>>,
        relative: Option<Kind<C, H>>,
        budget: &mut Budget,
    ) -> Result<
        PreparedFunctionValue<'a, Query<C, H>, Kind<C, H>>,
        FunctionError<EvaluationError<H::Error>>,
    > {
        self.preflight.scope = crate::projection::copy_scope::<C, H>(scope, budget, &entry.range)?;
        self.preflight.relative = relative;
        self.preflight.generator_kinds.clear();
        let mut facts = BTreeMap::new();
        for (name, kind) in scope {
            budget_name(name, budget, &entry.range)?;
            facts.insert(
                name.clone(),
                self.preflight
                    .host
                    .scalar_types(kind, budget, &entry.range)?
                    .unwrap_or_else(|| vec![ValueType::Unestablished]),
            );
        }
        let saved = std::mem::take(&mut self.preflight.prepared);
        let result = (|| {
            let types = query_value_types(entry, &facts, &mut self.preflight, budget)?;
            let kind = match self.preflight.local_kind(entry, budget)? {
                Some(kind) => kind,
                None => self.preflight.host.constructed_kind(types),
            };
            let mut preparation = Preparation::new();
            let value = preparation.resolve(entry, &mut self.preflight, budget)?;
            budget.charge(1, Location::Query(entry.range.clone()))?;
            Ok(PreparedFunctionValue {
                kind,
                value,
                names: preparation.names,
                prepared: std::mem::take(&mut self.preflight.prepared),
                identity: std::sync::Arc::clone(&self.identity),
            })
        })();
        self.preflight.prepared = saved;
        result
    }

    /// Execute a prepared value with current bindings, relative input and one
    /// cumulative allowance. Results retain native ownership through the host.
    ///
    /// # Errors
    /// Runtime contracts, active depth, host failures and allowances retain the
    /// responsible expression and bounded active call context.
    pub fn evaluate(
        &self,
        entry: &PreparedFunctionValue<'a, Query<C, H>, Kind<C, H>>,
        bindings: &Bindings<H::Value>,
        relative: Option<&H::Value>,
        context: &mut C,
        budget: &mut Budget,
        limits: FunctionLimits,
    ) -> Result<H::Value, FunctionError<EvaluationError<H::Error>>> {
        if !std::sync::Arc::ptr_eq(&self.identity, &entry.identity) {
            return Err(QueryError::query(
                entry.value.range.clone(),
                "value belongs to a different prepared function program",
            )
            .into());
        }
        let mut calls = CallTrace::default();
        let mut runtime = Runtime {
            host: self.preflight.host,
            context,
            bindings,
            contracts: &BTreeMap::new(),
            prepared: &entry.prepared,
            body_prepared: &self.body_prepared,
            source_function: None,
            relative,
            limit: limits.active_depth,
            calls: &mut calls,
        };
        let location = Location::Query(entry.value.range.clone());
        let result = execution::execute(
            &entry.value,
            &entry.names,
            &self.bodies,
            &mut runtime,
            budget,
            limits.active_depth,
            0,
        )
        .and_then(|value| runtime.finish_value(value, budget, &location));
        result.map_err(|error| FunctionError {
            error,
            omitted_calls: calls.omitted,
            calls: calls
                .frames
                .into_iter()
                .map(|(id, range)| FunctionCall {
                    function: self.preflight.declarations[id]
                        .name
                        .value
                        .chars()
                        .take(64)
                        .collect(),
                    range,
                })
                .collect(),
        })
    }
}

/// One-shot facade over [`PreparedFunctions`]. Existing callers retain their
/// context and budget borrowing contract.
pub struct FunctionRequest<'a, 'r, C, H: FunctionHost<C>> {
    functions: PreparedFunctions<'a, 'r, C, H>,
    context: &'r mut C,
    budget: &'r mut Budget,
    limits: FunctionLimits,
}
impl<'a, 'r, C, H: FunctionHost<C>> FunctionRequest<'a, 'r, C, H> {
    /// Prepare every declaration before evaluating the entry.
    ///
    /// # Errors
    /// Invalid declarations, scope, effects and allowances fail before execution.
    pub fn prepare(
        declarations: &'a [Declaration<C, H>],
        host: &'r H,
        context: &'r mut C,
        budget: &'r mut Budget,
        limits: FunctionLimits,
    ) -> Result<Self, FunctionError<EvaluationError<H::Error>>> {
        let functions = PreparedFunctions::prepare(declarations, host, budget)?;
        Ok(Self {
            functions,
            context,
            budget,
            limits,
        })
    }
    /// Prepare pattern-capable functions using the same request runner.
    ///
    /// # Errors
    /// Invalid declarations fail before any operation executes.
    pub fn prepare_patterns(
        declarations: &'a [PatternDeclaration<C, H>],
        host: &'r H,
        context: &'r mut C,
        budget: &'r mut Budget,
        limits: FunctionLimits,
    ) -> Result<Self, FunctionError<EvaluationError<H::Error>>> {
        let functions = PreparedFunctions::prepare_patterns(declarations, host, budget)?;
        Ok(Self {
            functions,
            context,
            budget,
            limits,
        })
    }

    /// Prepare and evaluate one entry with explicit input bindings.
    ///
    /// # Errors
    /// Runtime contracts and limits retain the original failure and bounded calls.
    pub fn evaluate(
        mut self,
        entry: &'a ValueExpression<Query<C, H>>,
        bindings: &Bindings<H::Value>,
    ) -> Result<H::Value, FunctionError<EvaluationError<H::Error>>> {
        let mut scope = BTreeMap::new();
        for (name, value) in bindings.iter() {
            budget_name(name, self.budget, &entry.range)?;
            scope.insert(
                name.to_owned(),
                self.functions
                    .preflight
                    .host
                    .kind(value, self.budget, &entry.range)?,
            );
        }
        let entry = self
            .functions
            .prepare_value(entry, &scope, None, self.budget)?;
        self.functions.evaluate(
            &entry,
            bindings,
            None,
            self.context,
            self.budget,
            self.limits,
        )
    }
}

fn budget_name(name: &str, budget: &mut Budget, range: &Range<usize>) -> Result<(), QueryError> {
    budget.charge(name.len() + 1, Location::Query(range.clone()))
}

struct Runtime<'r, 'a, C, H: FunctionHost<C>> {
    host: &'r H,
    context: &'r mut C,
    bindings: &'r Bindings<H::Value>,
    contracts: &'r BTreeMap<&'a str, &'a DeclaredType<Kind<C, H>>>,
    prepared: &'r BTreeMap<usize, Operand<Kind<C, H>>>,
    body_prepared: &'r [BTreeMap<usize, Operand<Kind<C, H>>>],
    source_function: Option<usize>,
    relative: Option<&'r H::Value>,
    limit: usize,
    calls: &'r mut CallTrace,
}
impl<C, H: FunctionHost<C>> Runtime<'_, '_, C, H> {
    fn owned_native(
        &mut self,
        value: NativeView<H::Value>,
        budget: &mut Budget,
        range: &Range<usize>,
    ) -> Result<std::sync::Arc<H::Value>, EvaluationError<H::Error>> {
        if value.path.is_empty() {
            Ok(value.owner)
        } else {
            budget.charge(value.path.len(), Location::Query(range.clone()))?;
            let value = native_view::<C, H>(&value, self.host);
            Ok(std::sync::Arc::new(self.host.copy_value(
                value,
                self.context,
                budget,
                range,
            )?))
        }
    }
    fn retained<'a>(
        &mut self,
        value: std::sync::Arc<H::Value>,
        budget: &mut Budget,
        range: &Range<usize>,
    ) -> Result<HostValue<'a, C, H>, EvaluationError<H::Error>> {
        if let Some(constructed) = self.host.constructed_value(&value) {
            let value = match self
                .host
                .share_constructed(&value, self.context, budget, range)?
            {
                Some(owner) => RuntimeValue::shared(owner),
                None => RuntimeValue::Owned(copy_constructed_value(
                    constructed,
                    budget,
                    Location::Query(range.clone()),
                )?),
            };
            constructed_value_type(value.as_ref(), budget, Location::Query(range.clone()))?;
            Ok(Value::constructed(value))
        } else {
            Ok(Value {
                data: Data::Native(NativeView::new(value)),
                contract: None,
            })
        }
    }
    fn finish_value(
        &mut self,
        value: Value<'_, H::Value, Kind<C, H>>,
        budget: &mut Budget,
        location: &Location,
    ) -> Result<H::Value, EvaluationError<H::Error>> {
        let range = match location {
            Location::Query(r) | Location::Source(r) => r,
        };
        match value.data {
            Data::Native(value) => {
                let value = self.owned_native(value, budget, range)?;
                match std::sync::Arc::try_unwrap(value) {
                    Ok(value) => Ok(value),
                    Err(value) => self.host.copy_value(&value, self.context, budget, range),
                }
            }
            Data::Constructed(data) => {
                let value = Value {
                    data: Data::<H::Value>::Constructed(data),
                    contract: value.contract,
                }
                .materialize(budget, location)?;
                let types = vec![constructed_value_type(&value, budget, location.clone())?];
                self.host
                    .lift_constructed(value, types, self.context, budget, range)
            }
        }
    }
}
impl<'a, C, H: FunctionHost<C>> execution::Host<'a, Query<C, H>> for Runtime<'_, 'a, C, H> {
    type Native = H::Value;
    type Kind = Kind<C, H>;
    type Error = EvaluationError<H::Error>;
    fn inspect_pattern(
        &mut self,
        value: &HostValue<'a, C, H>,
        budget: &mut Budget,
        location: &Location,
    ) -> Result<(), Self::Error> {
        if value.contract.is_none()
            && let Data::Native(view) = &value.data
        {
            let range = match location {
                Location::Query(r) | Location::Source(r) => r,
            };
            budget.charge(view.path.len(), location.clone())?;
            let value = native_view::<C, H>(view, self.host);
            let kind = self.host.kind(value, budget, range)?;
            admit_value(
                value,
                &DeclaredType::Native(kind),
                self.host,
                self.context,
                budget,
                location.clone(),
            )?;
        }
        Ok(())
    }
    fn native_record<'v>(&self, value: &'v H::Value) -> Option<&'v crate::NativeCapture<H::Value>> {
        self.host.capture_record(value)
    }
    fn native_tuple<'v>(&self, value: &'v H::Value) -> Option<&'v [H::Value]> {
        self.host.tuple_view(value)
    }
    fn retains_native_admission(&self) -> bool {
        self.host.retains_native_admission()
    }
    fn outer(
        &mut self,
        _: usize,
        name: &str,
        budget: &mut Budget,
        range: &Range<usize>,
    ) -> Result<HostValue<'a, C, H>, Self::Error> {
        budget.charge(1, Location::Query(range.clone()))?;
        let mut value = self.retained(self.bindings.shared(name, range.clone())?, budget, range)?;
        value.contract = self.contracts.get(name).copied();
        Ok(value)
    }
    fn native(
        &mut self,
        query: &'a Query<C, H>,
        arguments: &'a [PreparedArgument<'a, Query<C, H>>],
        locals: &mut [(&'a str, HostValue<'a, C, H>)],
        bodies: &'a Bodies<'a, C, H>,
        depth: usize,
        function: Option<usize>,
        projected: bool,
        budget: &mut Budget,
        range: &Range<usize>,
    ) -> Result<HostValue<'a, C, H>, Self::Error> {
        let lexical = function.is_some();
        let source_function = function.or(self.source_function);
        let prepared = match source_function {
            Some(id) => &self.body_prepared[id],
            None => self.prepared,
        };
        let operand = prepared
            .get(&(query as *const _ as usize))
            .expect("request native operand");
        let mut values = BTreeMap::new();
        let mut contracts = BTreeMap::new();
        if !lexical {
            crate::projection::admit_slots::<(&str, &DeclaredType<Kind<C, H>>)>(
                self.contracts.len(),
                budget,
                range,
            )?;
            contracts.extend(
                self.contracts
                    .iter()
                    .map(|(&name, &contract)| (name, contract)),
            );
        }
        for (name, value) in locals {
            budget_name(name, budget, range)?;
            if let Some(contract) = value.contract {
                crate::projection::admit_slots::<(&str, &DeclaredType<Kind<C, H>>)>(
                    1, budget, range,
                )?;
                contracts.insert(*name, contract);
            } else {
                contracts.remove(*name);
            }
            let value = match &mut value.data {
                Data::Native(owner) => {
                    let owner = owner.share(budget, &Location::Query(range.clone()))?;
                    self.owned_native(owner, budget, range)?
                }
                Data::Constructed(data) => {
                    let value = data
                        .share(budget, &Location::Query(range.clone()))?
                        .into_shared(budget, &Location::Query(range.clone()))?;
                    let types = vec![constructed_value_type(
                        &value,
                        budget,
                        Location::Query(range.clone()),
                    )?];
                    std::sync::Arc::new(self.host.lift_shared_constructed(
                        value,
                        types,
                        self.context,
                        budget,
                        range,
                    )?)
                }
            };
            values.insert((*name).to_owned(), value);
        }
        let bindings = if lexical {
            Bindings::from_shared(values)
        } else {
            self.bindings.with_shared_values(values, budget, range)?
        };
        let outer = crate::projection::prepared::NativeOuter::Bindings {
            values: &bindings,
            names: &operand.names,
        };
        let input = if lexical {
            None
        } else {
            self.relative
                .map(|value| self.host.copy_value(value, self.context, budget, range))
                .transpose()?
        };
        let host = self.host;
        let prepared = self.prepared;
        let body_prepared = self.body_prepared;
        let limit = self.limit;
        let calls = &mut *self.calls;
        let value = operand.native.execute(
            query,
            host,
            input,
            self.context,
            &bindings,
            &outer,
            None,
            &[],
            &mut |stage, declared, input, context, bindings, budget, range| {
                let mut arguments = Arguments {
                    host,
                    context,
                    bindings,
                    contracts: &contracts,
                    prepared,
                    body_prepared,
                    source_function,
                    bodies,
                    arguments,
                    declared,
                    depth,
                    limit,
                    calls,
                };
                // Synchronous host callbacks retain the existing bounded reentry guard.
                budget.within_evaluation(0, Location::Query(range.clone()), |budget| {
                    host.evaluate_stage_with_arguments(stage, input, &mut arguments, budget, range)
                })
            },
            budget,
        )?;
        if projected || host.scalar_types(&operand.kind, budget, range)?.is_some() {
            let value = host.project_value(value, self.context, budget, range)?;
            constructed_value_type(&value, budget, Location::Query(range.clone()))?;
            Ok(Value::constructed(RuntimeValue::Owned(value)))
        } else {
            self.retained(std::sync::Arc::new(value), budget, range)
        }
    }
    fn admit_native(
        &mut self,
        value: &H::Value,
        expected: &DeclaredType<Kind<C, H>>,
        budget: &mut Budget,
        location: Location,
    ) -> Result<(), Self::Error> {
        admit_value(value, expected, self.host, self.context, budget, location)
    }
    fn sequence_items(
        &mut self,
        value: NativeView<H::Value>,
        element: &DeclaredType<Kind<C, H>>,
        budget: &mut Budget,
        location: &Location,
    ) -> Result<Vec<HostValue<'a, C, H>>, Self::Error> {
        let range = match location {
            Location::Query(r) | Location::Source(r) => r,
        };
        budget.charge(value.path.len(), location.clone())?;
        let source = native_view::<C, H>(&value, self.host);
        let values = self
            .host
            .sequence_items(source, element, self.context, budget, range)?;
        budget.check_items(values.len(), location.clone())?;
        crate::projection::admit_slots::<HostValue<'a, C, H>>(values.len(), budget, range)?;
        let mut output = Vec::with_capacity(values.len());
        for value in values {
            budget.charge(1, location.clone())?;
            budget.charge_results(1, location.clone())?;
            output.push(self.retained(std::sync::Arc::new(value), budget, range)?);
        }
        Ok(output)
    }
    fn finish_sequence(
        &mut self,
        values: Vec<HostValue<'a, C, H>>,
        element: &'a DeclaredType<Kind<C, H>>,
        budget: &mut Budget,
        location: &Location,
    ) -> Result<HostValue<'a, C, H>, Self::Error> {
        if !crate::runtime_types::contains_native(element, budget, location)? {
            return Ok(execution::constructed_sequence(
                values, element, budget, location,
            )?);
        }
        let range = match location {
            Location::Query(r) | Location::Source(r) => r,
        };
        crate::projection::admit_slots::<H::Value>(values.len(), budget, range)?;
        let mut output = Vec::with_capacity(values.len());
        for value in values {
            output.push(self.finish_value(value, budget, location)?);
        }
        let value = self
            .host
            .construct_sequence(output, element, self.context, budget, range)?;
        let expected = DeclaredType::Sequence(Box::new(crate::runtime_types::copy_declaration(
            element, budget, location,
        )?));
        admit_value(
            &value,
            &expected,
            self.host,
            self.context,
            budget,
            location.clone(),
        )?;
        self.retained(std::sync::Arc::new(value), budget, range)
    }
    fn subjects(
        &mut self,
        value: NativeView<H::Value>,
        budget: &mut Budget,
        location: &Location,
    ) -> Result<Vec<H::Value>, Self::Error> {
        let range = match location {
            Location::Query(range) | Location::Source(range) => range,
        };
        let value = self.owned_native(value, budget, range)?;
        let value = match std::sync::Arc::try_unwrap(value) {
            Ok(value) => value,
            Err(value) => self.host.copy_value(&value, self.context, budget, range)?,
        };
        self.host.subjects(value, self.context, budget, range)
    }
    fn check(
        &mut self,
        _: &ConstructedValue,
        _: &mut Budget,
        _: &Location,
    ) -> Result<(), QueryError> {
        Ok(())
    }
    fn failure(&mut self, calls: &[(usize, Range<usize>)], omitted: usize) {
        self.calls.append(calls, omitted);
    }
}
fn native_view<'v, C, H: ProgramHost<C>>(
    value: &'v NativeView<H::Value>,
    host: &H,
) -> &'v H::Value {
    let mut data = value.owner.as_ref();
    for step in &value.path {
        data = match step {
            NativeStep::Record(index) => {
                &host
                    .capture_record(data)
                    .expect("retained record view")
                    .fields[*index]
                    .1
            }
            NativeStep::Tuple(index) => {
                &host.tuple_view(data).expect("retained tuple view")[*index]
            }
        };
    }
    data
}

struct Arguments<'r, 'a, C, H: FunctionHost<C>> {
    host: &'r H,
    context: &'r mut C,
    bindings: &'r Bindings<H::Value>,
    contracts: &'r BTreeMap<&'a str, &'a DeclaredType<Kind<C, H>>>,
    prepared: &'r BTreeMap<usize, Operand<Kind<C, H>>>,
    body_prepared: &'r [BTreeMap<usize, Operand<Kind<C, H>>>],
    source_function: Option<usize>,
    bodies: &'a Bodies<'a, C, H>,
    arguments: &'a [PreparedArgument<'a, Query<C, H>>],
    declared: &'r [usize],
    depth: usize,
    limit: usize,
    calls: &'r mut CallTrace,
}
impl<'a, C, H: FunctionHost<C>> Arguments<'_, 'a, C, H> {
    fn run(
        &mut self,
        expression: &ValueExpression<Query<C, H>>,
        input: &H::Value,
        bindings: Option<&Bindings<H::Value>>,
        budget: &mut Budget,
    ) -> Result<HostValue<'a, C, H>, EvaluationError<H::Error>> {
        let id = expression as *const _ as usize;
        budget.charge(
            self.declared.len() + 1,
            Location::Query(expression.range.clone()),
        )?;
        if !self.declared.contains(&id) {
            return Err(QueryError::query(
                expression.range.clone(),
                "argument was not declared by this native stage",
            )
            .into());
        }
        let argument = self
            .arguments
            .iter()
            .find(|a| std::ptr::eq(a.source, expression))
            .ok_or_else(|| {
                QueryError::query(
                    expression.range.clone(),
                    "argument was not prepared in this request",
                )
            })?;
        let mut runtime = Runtime {
            host: self.host,
            context: self.context,
            bindings: bindings.unwrap_or(self.bindings),
            contracts: self.contracts,
            prepared: self.prepared,
            body_prepared: self.body_prepared,
            source_function: self.source_function,
            relative: Some(input),
            limit: self.limit,
            calls: self.calls,
        };
        execution::execute(
            &argument.expression,
            &argument.names,
            self.bodies,
            &mut runtime,
            budget,
            self.limit,
            self.depth,
        )
    }
}
impl<C, H: FunctionHost<C>> NativeArguments<C, H> for Arguments<'_, '_, C, H> {
    fn parts(&mut self) -> (&mut C, &Bindings<H::Value>) {
        (self.context, self.bindings)
    }
    fn evaluate(
        &mut self,
        expression: &ValueExpression<Query<C, H>>,
        input: H::Value,
        budget: &mut Budget,
    ) -> Result<ConstructedRead, EvaluationError<H::Error>> {
        let value = self.run(expression, &input, None, budget)?;
        let location = Location::Query(expression.range.clone());
        let declaration = if let Some(contract) = value.contract {
            if crate::runtime_types::contains_native(contract, budget, &location)? {
                return Err(QueryError::query(
                    expression.range.clone(),
                    "native argument requires an explicit scalar projection",
                )
                .into());
            }
            Some(crate::runtime_types::constructed_declaration(
                contract, budget, &location,
            )?)
        } else {
            None
        };
        let value = match value.data {
            Data::Constructed(value) => value,
            Data::Native(_) => {
                return Err(QueryError::query(
                    expression.range.clone(),
                    "native argument requires an explicit scalar projection",
                )
                .into());
            }
        };
        Ok(ConstructedRead::from_runtime(value, location, declaration))
    }
    fn evaluate_native(
        &mut self,
        expression: &ValueExpression<Query<C, H>>,
        input: H::Value,
        budget: &mut Budget,
    ) -> Result<crate::NativeRead<H::Value, Kind<C, H>>, EvaluationError<H::Error>> {
        let value = self.run(expression, &input, None, budget)?;
        self.native_read(value, budget, &expression.range)
    }
    fn evaluate_match_value(
        &mut self,
        expression: &ValueExpression<Query<C, H>>,
        input: H::Value,
        bindings: &Bindings<H::Value>,
        budget: &mut Budget,
    ) -> Result<crate::NativeRead<H::Value, Kind<C, H>>, EvaluationError<H::Error>> {
        let value = self.run(expression, &input, Some(bindings), budget)?;
        self.native_read(value, budget, &expression.range)
    }
}
impl<C, H: FunctionHost<C>> Arguments<'_, '_, C, H> {
    #[expect(
        clippy::type_complexity,
        reason = "The selected argument retains the adapter's native value, kind and error types"
    )]
    fn native_read(
        &mut self,
        value: HostValue<'_, C, H>,
        budget: &mut Budget,
        range: &Range<usize>,
    ) -> Result<crate::NativeRead<H::Value, Kind<C, H>>, EvaluationError<H::Error>> {
        let location = Location::Query(range.clone());
        let declaration = value
            .contract
            .map(|contract| crate::runtime_types::copy_declaration(contract, budget, &location))
            .transpose()?;
        let value = match value.data {
            Data::Native(owner) => {
                if owner.path.is_empty() {
                    owner.owner
                } else {
                    budget.charge(owner.path.len(), location.clone())?;
                    std::sync::Arc::new(self.host.copy_value(
                        native_view::<C, H>(&owner, self.host),
                        self.context,
                        budget,
                        range,
                    )?)
                }
            }
            Data::Constructed(data) => {
                let value = Value {
                    data: Data::<H::Value>::Constructed(data),
                    contract: value.contract,
                }
                .materialize(budget, &location)?;
                let types = vec![constructed_value_type(&value, budget, location)?];
                crate::projection::admit_slots::<H::Value>(1, budget, range)?;
                std::sync::Arc::new(self.host.lift_constructed(
                    value,
                    types,
                    self.context,
                    budget,
                    range,
                )?)
            }
        };
        Ok(crate::NativeRead { value, declaration })
    }
}

/// Release lowered native function trees without recursively dropping value
/// expression wrappers. Opaque stage payloads retain their host's disposal policy.
pub fn dispose_functions<S, O, E, K>(functions: Vec<FunctionDeclaration<Expression<S, O, E>, K>>) {
    for function in functions {
        for (_, value) in function.bindings {
            dispose_native_value(value);
        }
        dispose_native_value(function.value);
        for parameter in function.parameters {
            dispose_declaration(parameter.value.declaration);
        }
        dispose_declaration(function.result.value);
    }
}
/// Release native pattern-function trees and nested patterns iteratively.
pub fn dispose_pattern_functions<S, O, E, K>(
    functions: Vec<PatternFunctionDeclaration<Expression<S, O, E>, K>>,
) {
    for function in functions {
        dispose_native_value(function.value);
        for parameter in function.parameters {
            crate::dispose_pattern(parameter.value.pattern);
            dispose_declaration(parameter.value.declaration);
        }
        dispose_declaration(function.result.value);
    }
}

fn dispose_declaration<K>(declaration: DeclaredType<K>) {
    let mut pending = vec![declaration];
    while let Some(declaration) = pending.pop() {
        match declaration {
            DeclaredType::Optional(child)
            | DeclaredType::Array(child)
            | DeclaredType::Sequence(child)
            | DeclaredType::Keyed(child) => pending.push(*child),
            DeclaredType::Tuple(children) => pending.extend(children),
            DeclaredType::Record(fields) => {
                pending.extend(fields.into_iter().map(|(_, value)| value))
            }
            DeclaredType::Boolean
            | DeclaredType::Integer
            | DeclaredType::Float
            | DeclaredType::String
            | DeclaredType::Member(_)
            | DeclaredType::Set(_)
            | DeclaredType::Native(_) => {}
        }
    }
}
