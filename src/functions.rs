//! Request-local named pure functions. Frontends supply located declarations;
//! calls use the ordinary value evaluator and retain native host values.
use crate::value::execution::{self, Body, Data, Value};
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
    pub declarations: &'a [FunctionDeclaration<Q, K>],
}
impl<Q, K, H: ValueQueryHost<Q>> ValueQueryHost<Q> for Resolver<'_, '_, Q, K, H> {
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
fn register<Q, K>(
    declarations: &[FunctionDeclaration<Q, K>],
    budget: &mut Budget,
    mut conflict: impl FnMut(&str) -> bool,
) -> Result<(), QueryError> {
    for (index, d) in declarations.iter().enumerate() {
        let location = Location::Query(d.name.range.clone());
        budget.charge(d.name.value.len() + 1, location.clone())?;
        if d.name.value == "_" || builtin(&d.name.value) || conflict(&d.name.value) {
            return Err(QueryError::query(
                d.name.range.clone(),
                "function conflicts with a reserved operation name",
            ));
        }
        for previous in &declarations[..index] {
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
        for (index, p) in d.parameters.iter().enumerate() {
            budget_name(&p.value.name.value, budget, &p.value.name.range)?;
            if p.value.name.value == "_" {
                return Err(QueryError::query(
                    p.value.name.range.clone(),
                    "discard name cannot be bound",
                ));
            }
            for previous in &d.parameters[..index] {
                budget.charge(
                    1 + p
                        .value
                        .name
                        .value
                        .len()
                        .min(previous.value.name.value.len()),
                    Location::Query(p.value.name.range.clone()),
                )?;
                if p.value.name.value == previous.value.name.value {
                    return Err(QueryError::query(
                        p.value.name.range.clone(),
                        "binding is already defined",
                    ));
                }
            }
            crate::runtime_types::validate_declaration(
                &p.value.declaration,
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
        "one"
            | "occurs"
            | "absent"
            | "lookup"
            | "has_key"
            | "index_set"
            | "entries"
            | "zip_exact"
            | "length"
            | "sum"
            | "forall"
            | "exists"
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
    register(declarations, budget, |_| false)?;
    crate::value::run_functions(declarations, entry, bindings, budget, limits)
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
}
type Kind<C, H> = <H as ProjectionHost<C>>::Kind;
type Query<C, H> = NativeExpression<C, H>;
type HostValue<'a, C, H> = Value<'a, <H as ProgramHost<C>>::Value, Kind<C, H>>;
type Bodies<'a, C, H> = [Body<'a, Query<C, H>, Kind<C, H>>];
type Declaration<C, H> = FunctionDeclaration<Query<C, H>, Kind<C, H>>;
struct Operand<K> {
    native: crate::projection::prepared::PreparedNative,
    names: Vec<String>,
    kind: K,
}
struct Preflight<'h, 'a, C, H: FunctionHost<C>> {
    host: &'h H,
    declarations: &'a [Declaration<C, H>],
    scope: BTreeMap<String, Kind<C, H>>,
    relative: Option<Kind<C, H>>,
    prepared: BTreeMap<usize, Operand<Kind<C, H>>>,
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
                    self.host.constructed_kind(copy_value_types(
                        types,
                        budget,
                        Location::Query(query.range.clone()),
                    )?),
                );
            }
        }
        if self.relative.is_none() {
            no_ambient_input::<C, H>(query, budget)?;
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
        let arguments = crate::projection::declared_arguments::<C, H>(query, self.host, budget)?;
        for (argument, relative) in arguments {
            let saved = std::mem::replace(
                &mut self.scope,
                crate::projection::copy_scope::<C, H>(&scope, budget, &query.range)?,
            );
            let old = self.relative.replace(relative);
            let mut facts = BTreeMap::new();
            for (name, kind) in &scope {
                budget.charge(name.len() + 1, Location::Query(query.range.clone()))?;
                facts.insert(
                    name.clone(),
                    self.host
                        .scalar_types(kind, budget, &query.range)?
                        .unwrap_or_else(|| vec![ValueType::Unestablished]),
                );
            }
            let result = query_value_types(argument, &facts, self, budget);
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
                ValueExpressionKind::NativeValue(query) => Some(
                    self.prepared
                        .get(&(query.as_ref() as *const _ as usize))
                        .expect("prepared native let")
                        .kind
                        .clone(),
                ),
                ValueExpressionKind::Name(name) => self.scope.get(name).cloned(),
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
            ExpressionKind::Sequence { input, .. } => pending.push(input),
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

/// One submitted function request, bound to its exact host context and budget.
/// Preparation checks all signatures and bodies; evaluation also prepares the
/// entry before executing any native operation. The request is consumed once.
pub struct FunctionRequest<'a, 'r, C, H: FunctionHost<C>> {
    declarations: &'a [Declaration<C, H>],
    bodies: Vec<Body<'a, Query<C, H>, Kind<C, H>>>,
    preflight: Preflight<'r, 'a, C, H>,
    context: &'r mut C,
    budget: &'r mut Budget,
    limits: FunctionLimits,
}
impl<'a, 'r, C, H: FunctionHost<C>> FunctionRequest<'a, 'r, C, H> {
    /// Register signatures, then prepare every body in lexical scope.
    ///
    /// # Errors
    /// Invalid declarations, missing names, effects, emitters and allowances
    /// fail before any body executes, including unused functions.
    pub fn prepare(
        declarations: &'a [Declaration<C, H>],
        host: &'r H,
        context: &'r mut C,
        budget: &'r mut Budget,
        limits: FunctionLimits,
    ) -> Result<Self, FunctionError<EvaluationError<H::Error>>> {
        register(declarations, budget, |name| host.is_operation_name(name))?;
        let mut preflight = Preflight {
            host,
            declarations,
            scope: BTreeMap::new(),
            relative: None,
            prepared: BTreeMap::new(),
        };
        crate::projection::admit_slots::<Body<'_, Query<C, H>, Kind<C, H>>>(
            declarations.len(),
            budget,
            &(0..0),
        )?;
        let mut bodies = Vec::with_capacity(declarations.len());
        for declaration in declarations {
            preflight.scope.clear();
            for p in &declaration.parameters {
                budget.charge(
                    p.value.name.value.len() + 1,
                    Location::Query(p.range.clone()),
                )?;
                preflight.scope.insert(
                    p.value.name.value.clone(),
                    host.declaration_kind(&p.value.declaration, budget, &p.range)?,
                );
            }
            bodies.push(crate::value::prepare_function_body(
                declaration,
                &mut preflight,
                budget,
            )?);
        }
        Ok(Self {
            declarations,
            bodies,
            preflight,
            context,
            budget,
            limits,
        })
    }
    /// Evaluate with explicit native/constructed inputs and return the host's
    /// native value. Function bodies cannot capture these entry bindings.
    ///
    /// # Errors
    /// Whole argument/result admission retains native failures and responsible
    /// ranges. Failure releases active frames without refunding spending.
    pub fn evaluate(
        mut self,
        entry: &ValueExpression<Query<C, H>>,
        bindings: &Bindings<H::Value>,
    ) -> Result<H::Value, FunctionError<EvaluationError<H::Error>>> {
        self.preflight.scope.clear();
        let mut facts = BTreeMap::new();
        for (name, value) in bindings.iter() {
            let kind = self.preflight.host.kind(value, self.budget, &entry.range)?;
            budget_name(name, self.budget, &entry.range)?;
            facts.insert(
                name.to_owned(),
                self.preflight
                    .host
                    .scalar_types(&kind, self.budget, &entry.range)?
                    .unwrap_or_else(|| vec![ValueType::Unestablished]),
            );
            self.preflight.scope.insert(name.to_owned(), kind);
        }
        query_value_types(entry, &facts, &mut self.preflight, self.budget)?;
        let mut preparation = Preparation::new();
        let entry = preparation.resolve(entry, &mut self.preflight, self.budget)?;
        let mut calls = CallTrace::default();
        let mut runtime = Runtime {
            host: self.preflight.host,
            context: self.context,
            bindings,
            contracts: &BTreeMap::new(),
            prepared: &self.preflight.prepared,
            relative: None,
            limit: self.limits.active_depth,
            calls: &mut calls,
        };
        let location = Location::Query(entry.range.clone());
        let result = execution::execute(
            &entry,
            &preparation.names,
            &self.bodies,
            &mut runtime,
            self.budget,
            self.limits.active_depth,
            0,
        )
        .and_then(|value| runtime.finish_value(value, self.budget, &location));
        result.map_err(|error| FunctionError {
            error,
            omitted_calls: calls.omitted,
            calls: calls
                .frames
                .into_iter()
                .map(|(id, range)| FunctionCall {
                    function: self.declarations[id].name.value.chars().take(64).collect(),
                    range,
                })
                .collect(),
        })
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
    relative: Option<&'r H::Value>,
    limit: usize,
    calls: &'r mut CallTrace,
}
impl<C, H: FunctionHost<C>> Runtime<'_, '_, C, H> {
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
                data: Data::Native(value),
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
            Data::Native(value) => match std::sync::Arc::try_unwrap(value) {
                Ok(value) => Ok(value),
                Err(value) => self.host.copy_value(&value, self.context, budget, range),
            },
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
        lexical: bool,
        projected: bool,
        budget: &mut Budget,
        range: &Range<usize>,
    ) -> Result<HostValue<'a, C, H>, Self::Error> {
        let operand = self
            .prepared
            .get(&(query as *const _ as usize))
            .expect("request native operand");
        let mut values = BTreeMap::new();
        let mut contracts = BTreeMap::new();
        if !lexical {
            for (name, _) in self.bindings.iter() {
                budget_name(name, budget, range)?;
                values.insert(name.to_owned(), self.bindings.shared(name, range.clone())?);
            }
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
                Data::Native(owner) => std::sync::Arc::clone(owner),
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
        let bindings = Bindings::from_shared(values);
        crate::projection::admit_slots::<&H::Value>(operand.names.len(), budget, range)?;
        let outer = operand
            .names
            .iter()
            .map(|name| bindings.get(name, range.clone()))
            .collect::<Result<Vec<_>, _>>()?;
        let input = if lexical {
            None
        } else {
            self.relative
                .map(|value| self.host.copy_value(value, self.context, budget, range))
                .transpose()?
        };
        let host = self.host;
        let prepared = self.prepared;
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
        if projected {
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
struct Arguments<'r, 'a, C, H: FunctionHost<C>> {
    host: &'r H,
    context: &'r mut C,
    bindings: &'r Bindings<H::Value>,
    contracts: &'r BTreeMap<&'a str, &'a DeclaredType<Kind<C, H>>>,
    prepared: &'r BTreeMap<usize, Operand<Kind<C, H>>>,
    bodies: &'a Bodies<'a, C, H>,
    arguments: &'a [PreparedArgument<'a, Query<C, H>>],
    declared: &'r [usize],
    depth: usize,
    limit: usize,
    calls: &'r mut CallTrace,
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
            bindings: self.bindings,
            contracts: self.contracts,
            prepared: self.prepared,
            relative: Some(&input),
            limit: self.limit,
            calls: self.calls,
        };
        let value = execution::execute(
            &argument.expression,
            &argument.names,
            self.bodies,
            &mut runtime,
            budget,
            self.limit,
            self.depth,
        )?;
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
