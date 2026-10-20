//! Request-local named pure functions. Frontends supply located declarations;
//! calls use the ordinary value evaluator and retain native host values.
use crate::Pattern;
use crate::value::execution::{self, Body, Data, NativeStep, NativeView, Value};
use crate::value::pattern::PatternRef;
use crate::value::prepared::{
    Preparation, PreparedArgument, PreparedKind, PreparedValue, RuntimeValue,
};
use crate::{
    Bindings, Budget, ConstructedRead, ConstructedValue, DeclaredType, EvaluationError, Expression,
    ExpressionKind, Located, Location, NativeArguments, NativeExpression, ProgramHost,
    ProjectionHost, QueryError, RuntimeTypeHost, ValueExpression, ValueExpressionKind,
    ValueQueryHost, ValueType, admit_value, constructed_value_type, copy_constructed_value,
    copy_value_types, dispose_native_value, native_expression_kind,
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

/// A request-resolved declaration borrowing its original parameter pattern.
#[derive(Debug, PartialEq)]
pub struct PatternParameterRef<'a, K> {
    pub pattern: &'a Pattern,
    pub declaration: DeclaredType<K>,
}

/// Request-owned headers borrowing reusable patterns and a body tree.
pub struct PatternFunctionDeclarationRef<'a, Q = (), K = ()> {
    pub name: &'a Located<String>,
    pub parameters: &'a [Located<PatternParameterRef<'a, K>>],
    pub result: &'a Located<DeclaredType<K>>,
    pub value: &'a ValueExpression<Q>,
}

pub(crate) enum Parameters<'a, K> {
    Names(&'a [Located<FunctionParameter<K>>]),
    Patterns(&'a [Located<PatternParameter<K>>]),
    BorrowedPatterns(&'a [Located<PatternParameterRef<'a, K>>]),
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
            Self::BorrowedPatterns(v) => v.len(),
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
            Self::BorrowedPatterns(v) => ParameterRef {
                pattern: PatternRef::Pattern(v[index].value.pattern),
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
impl<'a, Q, K> From<PatternFunctionDeclarationRef<'a, Q, K>> for DeclarationRef<'a, Q, K> {
    fn from(d: PatternFunctionDeclarationRef<'a, Q, K>) -> Self {
        Self {
            name: d.name,
            parameters: Parameters::BorrowedPatterns(d.parameters),
            result: d.result,
            bindings: &[],
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
            | "require"
            | "project"
            | "match"
            | "paths"
            | "map_by"
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
    /// Expose an ordinary host-owned set. Its elements retain native handles;
    /// iteration order is insertion order and equality uses membership.
    fn set_view<'v>(&self, _value: &'v Self::Value) -> Option<&'v [Self::Value]> {
        None
    }
    /// Expose an ordinary dictionary with arbitrary value keys.
    fn dictionary_view<'v>(
        &self,
        _value: &'v Self::Value,
    ) -> Option<&'v [(Self::Value, Self::Value)]> {
        None
    }
    /// Consult a retained native set index using ordinary structural identity.
    /// The outer `None` selects the shared temporary index; `Some(None)` means
    /// indexed absence and `Some(Some(i))` selects position `i` in `set_view`.
    /// Charge index work and resolve hash collisions by value equality. The
    /// runner admits the set and item in the current context before this call.
    fn set_position(
        &self,
        _set: &Self::Value,
        _item: &Self::Value,
        _context: &mut C,
        _budget: &mut Budget,
        _range: &Range<usize>,
    ) -> Result<Option<Option<usize>>, EvaluationError<Self::Error>> {
        Ok(None)
    }
    /// Consult a retained dictionary key index. The outer `None` selects the
    /// shared temporary index; `Some(None)` means indexed absence and
    /// `Some(Some(i))` selects entry `i` in `dictionary_view`. Use structural
    /// key identity, including native owners, and resolve hash collisions.
    /// The runner admits the dictionary and key before this call.
    fn dictionary_position(
        &self,
        _dictionary: &Self::Value,
        _key: &Self::Value,
        _context: &mut C,
        _budget: &mut Budget,
        _range: &Range<usize>,
    ) -> Result<Option<Option<usize>>, EvaluationError<Self::Error>> {
        Ok(None)
    }
    /// Retain an admitted set's values and native owners in insertion order.
    /// This is ordinary data; rendering remains an explicit host boundary.
    fn construct_set(
        &self,
        _values: Vec<Self::Value>,
        _context: &mut C,
        _budget: &mut Budget,
        range: &Range<usize>,
    ) -> Result<Self::Value, EvaluationError<Self::Error>> {
        Err(QueryError::query(range.clone(), "native set construction is unsupported").into())
    }
    /// Retain admitted dictionary entries and their native owners in insertion
    /// order. The shared runner checks duplicate keys before calling this hook.
    fn construct_dictionary(
        &self,
        _entries: Vec<(Self::Value, Self::Value)>,
        _context: &mut C,
        _budget: &mut Budget,
        range: &Range<usize>,
    ) -> Result<Self::Value, EvaluationError<Self::Error>> {
        Err(QueryError::query(
            range.clone(),
            "native dictionary construction is unsupported",
        )
        .into())
    }
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

    /// Construct an ordinary positional list. Hosts supporting mixed native
    /// kinds override this hook; the default retains existing homogeneous
    /// sequence adapters.
    fn construct_list(
        &self,
        values: Vec<Self::Value>,
        context: &mut C,
        budget: &mut Budget,
        range: &Range<usize>,
    ) -> Result<Self::Value, EvaluationError<Self::Error>>
    where
        Self: Sized,
    {
        let first = values.first().ok_or_else(|| {
            QueryError::query(range.clone(), "empty native list needs host support")
        })?;
        let declaration = if let Some(computational) = self.computational_value(first) {
            computational.declaration(budget, &Location::Query(range.clone()))?
        } else {
            match self.constructed_value(first) {
                Some(value) => {
                    let facts =
                        constructed_value_type(value, budget, Location::Query(range.clone()))?;
                    crate::worklists::fact_contract(
                        &facts,
                        budget,
                        &Location::Query(range.clone()),
                    )?
                    .ok_or_else(|| {
                        QueryError::query(
                            range.clone(),
                            "native list element contract is unestablished",
                        )
                    })?
                }
                None => DeclaredType::Native(self.kind(first, budget, range)?),
            }
        };
        for value in &values {
            if !crate::admit_computational(
                value,
                &declaration,
                self,
                context,
                budget,
                Location::Query(range.clone()),
            )? {
                admit_value(
                    value,
                    &declaration,
                    self,
                    context,
                    budget,
                    Location::Query(range.clone()),
                )?;
            }
        }
        self.construct_sequence(values, &declaration, context, budget, range)
    }

    /// Opt into computational constructors during all-branch preparation.
    /// `None` defers element inference to selected actual input. A known
    /// declaration is checked before execution or public factory allocation.
    fn prepare_computational(
        &self,
        _declaration: Option<&DeclaredType<<Self as ProjectionHost<C>>::Kind>>,
        _budget: &mut Budget,
        range: &Range<usize>,
    ) -> Result<(), QueryError> {
        Err(QueryError::query(
            range.clone(),
            "computational values are unsupported",
        ))
    }
    /// Borrow an opt-in computational wrapper without exposing storage.
    fn computational_value<'v>(
        &self,
        _value: &'v Self::Value,
    ) -> Option<&'v crate::ComputationalValue<Self::Value, <Self as ProjectionHost<C>>::Kind>> {
        None
    }
    /// Wrap a computational result. Copying this wrapper must share roots.
    fn lift_computational(
        &self,
        _value: crate::ComputationalValue<Self::Value, <Self as ProjectionHost<C>>::Kind>,
        _context: &mut C,
        _budget: &mut Budget,
        range: &Range<usize>,
    ) -> Result<Self::Value, EvaluationError<Self::Error>> {
        Err(QueryError::query(range.clone(), "computational values are unsupported").into())
    }
    /// Construct an explicit native/mixed positional tuple, preserving owners.
    fn construct_tuple(
        &self,
        _values: Vec<Self::Value>,
        _context: &mut C,
        _budget: &mut Budget,
        range: &Range<usize>,
    ) -> Result<Self::Value, EvaluationError<Self::Error>> {
        Err(QueryError::query(range.clone(), "native tuple construction is unsupported").into())
    }
    /// Resolve a named constructor type in the current immutable request owner.
    fn resolve_type_name(
        &self,
        name: Located<&str>,
        _budget: &mut Budget,
    ) -> Result<DeclaredType<<Self as ProjectionHost<C>>::Kind>, QueryError> {
        Err(QueryError::query(
            name.range,
            "native type resolution is unsupported",
        ))
    }
    /// Identify the immutable owner/snapshot, including its validity stamp.
    /// Change this token whenever earlier native admission can become invalid.
    fn worklist_context(
        &self,
        _context: &mut C,
        _budget: &mut Budget,
        range: &Range<usize>,
    ) -> Result<crate::NativeContextId, EvaluationError<Self::Error>> {
        Err(QueryError::query(range.clone(), "computational values are unsupported").into())
    }
    /// Return an exact canonical identity agreeing with native semantic equality.
    /// Include owner, diff side and object; never substitute a hash or label.
    fn native_identity(
        &self,
        _value: &Self::Value,
        _expected: &<Self as ProjectionHost<C>>::Kind,
        _context: &mut C,
        _budget: &mut Budget,
        range: &Range<usize>,
    ) -> Result<crate::NativeIdentity, EvaluationError<Self::Error>> {
        Err(QueryError::query(range.clone(), "native identity is unsupported").into())
    }
    /// Order admitted native leaves. The default orders canonical identities
    /// within one kind; hosts can provide an order between domain kinds.
    fn compare_native_values(
        &self,
        left: &Self::Value,
        right: &Self::Value,
        context: &mut C,
        budget: &mut Budget,
        range: &Range<usize>,
    ) -> Result<std::cmp::Ordering, EvaluationError<Self::Error>> {
        let left_kind = self.kind(left, budget, range)?;
        let right_kind = self.kind(right, budget, range)?;
        if left_kind != right_kind {
            return Err(QueryError::query(
                range.clone(),
                "ordering different native kinds requires a host comparison",
            )
            .into());
        }
        Ok(self
            .native_identity(left, &left_kind, context, budget, range)?
            .0
            .cmp(
                &self
                    .native_identity(right, &right_kind, context, budget, range)?
                    .0,
            ))
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
    contracts: BTreeMap<String, DeclaredType<K>>,
}
struct Operand<K> {
    native: crate::projection::prepared::PreparedNative,
    names: Vec<String>,
    kind: K,
    argument_kinds: BTreeMap<usize, K>,
}
struct Preflight<'h, 'a, C, H: FunctionHost<C>> {
    host: &'h H,
    declarations: Vec<DeclarationRef<'a, Query<C, H>, Kind<C, H>>>,
    scope: BTreeMap<String, Kind<C, H>>,
    generator_kinds: BTreeMap<String, Kind<C, H>>,
    relative: Option<Kind<C, H>>,
    prepared: BTreeMap<usize, Operand<Kind<C, H>>>,
    scopes: Vec<LexicalKinds<Kind<C, H>>>,
    contracts: BTreeMap<String, DeclaredType<Kind<C, H>>>,
    worklists: BTreeMap<usize, Option<DeclaredType<Kind<C, H>>>>,
    worklist_contracts: Vec<Option<DeclaredType<Kind<C, H>>>>,
    general_key_expressions: Vec<crate::value::GeneralKeyExpression>,
}
impl<'h, 'a, C, H: FunctionHost<C>> Preflight<'h, 'a, C, H> {
    fn new(
        declarations: impl IntoIterator<Item = DeclarationRef<'a, Query<C, H>, Kind<C, H>>>,
        host: &'h H,
        budget: &mut Budget,
    ) -> Result<Self, QueryError> {
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
        Ok(Self {
            host,
            declarations: retained,
            scope: BTreeMap::new(),
            generator_kinds: BTreeMap::new(),
            relative: None,
            prepared: BTreeMap::new(),
            scopes: Vec::new(),
            contracts: BTreeMap::new(),
            worklists: BTreeMap::new(),
            worklist_contracts: Vec::new(),
            general_key_expressions: Vec::new(),
        })
    }
}

/// Discover one value's kind from pattern declarations and outer kinds.
/// This registers and checks signatures, then uses ordinary common preflight for
/// the value, including lexical patterns and native arguments. It does not prepare
/// declaration bodies, so hosts can call it from a kind callback while complete
/// function preparation is already active.
///
/// Before executing a request, separately call [`PreparedFunctions::prepare_patterns`]
/// (or its borrowed-header counterpart) to check every body, including unused ones.
/// This query alone does not establish body purity or authorize execution.
///
/// # Errors
/// Invalid signatures, value scope/effects, host operations and limits are located.
pub fn pattern_function_value_kind<'a, C, H: FunctionHost<C>>(
    declarations: impl IntoIterator<Item = &'a PatternFunctionDeclaration<Query<C, H>, Kind<C, H>>>,
    host: &H,
    value: &ValueExpression<Query<C, H>>,
    scope: &BTreeMap<String, Kind<C, H>>,
    relative: Option<Kind<C, H>>,
    budget: &mut Budget,
) -> Result<Kind<C, H>, QueryError>
where
    Query<C, H>: 'a,
    Kind<C, H>: 'a,
{
    Preflight::new(
        declarations
            .into_iter()
            .map(PatternFunctionDeclaration::as_borrowed),
        host,
        budget,
    )?
    .entry_kind(value, scope, relative, budget)
}

/// Discover one value's kind from request-resolved borrowed pattern headers.
///
/// Like [`pattern_function_value_kind`], this checks signatures and the selected
/// value without preparing declaration bodies. Hosts can call it from a native
/// kind callback using their existing resolved headers and borrowed body trees.
/// Before execution, separately check every body with
/// [`PreparedFunctions::prepare_patterns_borrowed`], including unused declarations.
///
/// # Errors
/// Invalid signatures, value scope/effects, host operations and limits are located.
pub fn pattern_function_value_kind_borrowed<'a, C, H: FunctionHost<C>>(
    declarations: impl IntoIterator<Item = PatternFunctionDeclarationRef<'a, Query<C, H>, Kind<C, H>>>,
    host: &H,
    value: &ValueExpression<Query<C, H>>,
    scope: &BTreeMap<String, Kind<C, H>>,
    relative: Option<Kind<C, H>>,
    budget: &mut Budget,
) -> Result<Kind<C, H>, QueryError>
where
    Query<C, H>: 'a,
    Kind<C, H>: 'a,
{
    Preflight::new(
        declarations.into_iter().map(DeclarationRef::from),
        host,
        budget,
    )?
    .entry_kind(value, scope, relative, budget)
}

impl<C, H: FunctionHost<C>> Preflight<'_, '_, C, H> {
    fn value_types(
        &mut self,
        entry: &ValueExpression<Query<C, H>>,
        facts: &BTreeMap<String, Vec<ValueType>>,
        budget: &mut Budget,
    ) -> Result<Vec<ValueType>, QueryError> {
        let analysis = crate::value::analyze_query_value(entry, facts, self, budget)?;
        for tally in analysis.general_key_expressions {
            crate::projection::admit_slots::<crate::value::GeneralKeyExpression>(
                1,
                budget,
                &entry.range,
            )?;
            self.general_key_expressions.push(tally);
        }
        Ok(analysis.types)
    }

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
                argument_kinds: BTreeMap::new(),
            },
        );
        let arguments =
            crate::projection::scoped_arguments::<C, H>(query, self.host, &input, &scope, budget)?;
        let mut argument_kinds = BTreeMap::new();
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
            let result = (|| {
                let types = self.value_types(argument.expression, &facts, budget)?;
                let kind = self
                    .local_kind(argument.expression, budget)?
                    .unwrap_or_else(|| self.host.constructed_kind(types));
                crate::projection::admit_slots::<(usize, Kind<C, H>)>(
                    1,
                    budget,
                    &argument.expression.range,
                )?;
                argument_kinds.insert(argument.expression as *const _ as usize, kind);
                Ok::<_, QueryError>(())
            })();
            self.scope = saved;
            self.relative = old;
            result?;
        }
        self.prepared
            .get_mut(&(query as *const _ as usize))
            .expect("prepared native operand")
            .argument_kinds = argument_kinds;
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
    fn entry_kind(
        &mut self,
        entry: &ValueExpression<Query<C, H>>,
        scope: &BTreeMap<String, Kind<C, H>>,
        relative: Option<Kind<C, H>>,
        budget: &mut Budget,
    ) -> Result<Kind<C, H>, QueryError> {
        self.scope = crate::projection::copy_scope::<C, H>(scope, budget, &entry.range)?;
        self.relative = relative;
        self.generator_kinds.clear();
        self.contracts.clear();
        self.worklists.clear();
        let mut facts = BTreeMap::new();
        for (name, kind) in scope {
            budget_name(name, budget, &entry.range)?;
            facts.insert(
                name.clone(),
                self.host
                    .scalar_types(kind, budget, &entry.range)?
                    .unwrap_or_else(|| vec![ValueType::Unestablished]),
            );
        }
        let types = self.value_types(entry, &facts, budget)?;
        Ok(match self.local_kind(entry, budget)? {
            Some(kind) => kind,
            None => self.host.constructed_kind(types),
        })
    }

    fn expect(
        &mut self,
        value: &ValueExpression<Query<C, H>>,
        expected: &DeclaredType<Kind<C, H>>,
        budget: &mut Budget,
    ) -> Result<(), QueryError> {
        let location = Location::Query(value.range.clone());
        budget.within_evaluation(1, location.clone(), |budget| {
            budget.charge(1, location.clone())?;
            let mut d = expected;
            while let DeclaredType::Optional(c) = d {
                d = c;
            }
            match &value.value {
                ValueExpressionKind::Queue { .. } if matches!(d, DeclaredType::Queue(_)) => {
                    let id = value as *const _ as usize;
                    if let Some(Some(old)) = self.worklists.get(&id) {
                        if !crate::runtime_types::same_declaration(old, d, budget, &location)? {
                            return Err(QueryError::query(
                                value.range.clone(),
                                "incompatible queue contexts",
                            ));
                        }
                    } else {
                        self.worklists.insert(
                            id,
                            Some(crate::runtime_types::copy_declaration(
                                d, budget, &location,
                            )?),
                        );
                    }
                }
                ValueExpressionKind::Conditional {
                    branches,
                    otherwise,
                } => {
                    for (_, v) in branches {
                        self.expect(v, expected, budget)?;
                    }
                    self.expect(otherwise, expected, budget)?;
                }
                ValueExpressionKind::Block { value, .. }
                | ValueExpressionKind::Require { value, .. }
                | ValueExpressionKind::Break(value) => self.expect(value, expected, budget)?,
                ValueExpressionKind::ValueMatch { arms, .. } => {
                    for arm in arms {
                        self.expect(&arm.value, expected, budget)?;
                    }
                }
                ValueExpressionKind::Default { value, fallback } => {
                    self.expect(value, expected, budget)?;
                    self.expect(fallback, expected, budget)?;
                }
                ValueExpressionKind::Tuple(values) => {
                    if let DeclaredType::Tuple(types) = d {
                        for (v, d) in values.iter().zip(types) {
                            self.expect(v, d, budget)?;
                        }
                    }
                }
                ValueExpressionKind::Array(values) => {
                    if let DeclaredType::Array(d) | DeclaredType::Sequence(d) = d {
                        for v in values {
                            self.expect(v, d, budget)?;
                        }
                    }
                }
                ValueExpressionKind::Loop { body, .. } => self.expect(body, expected, budget)?,
                _ => {}
            }
            Ok(())
        })
    }
    fn expect_next(
        &mut self,
        value: &ValueExpression<Query<C, H>>,
        states: &[Option<DeclaredType<Kind<C, H>>>],
        budget: &mut Budget,
    ) -> Result<(), QueryError> {
        budget.within_evaluation(1, Location::Query(value.range.clone()), |budget| {
            budget.charge(1, Location::Query(value.range.clone()))?;
            match &value.value {
                ValueExpressionKind::Continue(values) => {
                    for (v, d) in values.iter().zip(states) {
                        if let Some(d) = d {
                            self.expect(v, d, budget)?;
                        }
                    }
                }
                ValueExpressionKind::Block { value, .. } => {
                    self.expect_next(value, states, budget)?
                }
                ValueExpressionKind::Conditional {
                    branches,
                    otherwise,
                } => {
                    for (_, v) in branches {
                        self.expect_next(v, states, budget)?;
                    }
                    self.expect_next(otherwise, states, budget)?;
                }
                ValueExpressionKind::ValueMatch { arms, .. } => {
                    for arm in arms {
                        self.expect_next(&arm.value, states, budget)?;
                    }
                }
                _ => {}
            }
            Ok(())
        })
    }
    fn value_declaration(
        &mut self,
        value: &ValueExpression<Query<C, H>>,
        budget: &mut Budget,
    ) -> Result<Option<DeclaredType<Kind<C, H>>>, QueryError> {
        let location = Location::Query(value.range.clone());
        budget.within_evaluation(1, location.clone(), |budget| {
            budget.charge(1, location.clone())?;
            Ok(match &value.value {
                ValueExpressionKind::Queue { .. } | ValueExpressionKind::IdentitySet { .. } => self
                    .worklists
                    .get(&(value as *const _ as usize))
                    .and_then(Option::as_ref)
                    .map(|d| crate::runtime_types::copy_declaration(d, budget, &location))
                    .transpose()?,
                ValueExpressionKind::PushBackAll { queue, .. } => {
                    self.value_declaration(queue, budget)?
                }
                ValueExpressionKind::Insert { set, .. } => self.value_declaration(set, budget)?,
                ValueExpressionKind::PopFront(queue) => match self
                    .value_declaration(queue, budget)?
                {
                    Some(DeclaredType::Queue(element)) => {
                        let queue = DeclaredType::Queue(Box::new(
                            crate::runtime_types::copy_declaration(&element, budget, &location)?,
                        ));
                        Some(DeclaredType::Optional(Box::new(DeclaredType::Tuple(vec![
                            *element, queue,
                        ]))))
                    }
                    _ => None,
                },
                ValueExpressionKind::Call { function, .. } => {
                    let (id, _) = self.function(function, budget)?;
                    Some(crate::runtime_types::copy_declaration(
                        &self.declarations[id].result.value,
                        budget,
                        &location,
                    )?)
                }
                ValueExpressionKind::PredicateSubject => self
                    .scope
                    .get(crate::value::PREDICATE_SUBJECT)
                    .or_else(|| self.generator_kinds.get(crate::value::PREDICATE_SUBJECT))
                    .cloned()
                    .map(DeclaredType::Native),
                ValueExpressionKind::Name(name) => match self.contracts.get(name) {
                    Some(d) => Some(crate::runtime_types::copy_declaration(
                        d, budget, &location,
                    )?),
                    None => self
                        .scope
                        .get(name)
                        .or_else(|| self.generator_kinds.get(name))
                        .cloned()
                        .map(DeclaredType::Native),
                },
                ValueExpressionKind::NativeValue(q) => self
                    .prepared
                    .get(&(q.as_ref() as *const _ as usize))
                    .map(|p| DeclaredType::Native(p.kind.clone())),
                ValueExpressionKind::Literal(v) => crate::worklists::scalar_contract(v),
                ValueExpressionKind::Array(values) => match values.first() {
                    Some(v) => self
                        .value_declaration(v, budget)?
                        .map(|d| DeclaredType::Sequence(Box::new(d))),
                    None => None,
                },
                ValueExpressionKind::Windows { items, .. } => {
                    let Some(kind) = self.local_kind(items, budget)? else {
                        return Ok(None);
                    };
                    let element = self.positional_element_kind(&kind, budget, &items.range)?;
                    Some(DeclaredType::Sequence(Box::new(DeclaredType::Sequence(
                        Box::new(DeclaredType::Native(element)),
                    ))))
                }
                ValueExpressionKind::RunsBy { rows, field } => {
                    let Some(kind) = self.local_kind(rows, budget)? else {
                        return Ok(None);
                    };
                    let element = self.positional_element_kind(&kind, budget, &rows.range)?;
                    let Some(record) = self.host.capture_record_kind(&element) else {
                        return Ok(None);
                    };
                    let mut key = None;
                    for (label, kind) in &record.fields {
                        budget.charge(1 + label.len().min(field.value.len()), location.clone())?;
                        if label == &field.value {
                            key = Some(kind.clone());
                            break;
                        }
                    }
                    let Some(key) = key else { return Ok(None) };
                    Some(DeclaredType::Sequence(Box::new(DeclaredType::Record(
                        vec![
                            ("key".into(), DeclaredType::Native(key)),
                            (
                                "items".into(),
                                DeclaredType::Sequence(Box::new(DeclaredType::Native(element))),
                            ),
                        ],
                    ))))
                }
                ValueExpressionKind::Map { array, argument }
                | ValueExpressionKind::FlatMap { array, argument } => self.mapping_declaration(
                    array,
                    argument,
                    matches!(value.value, ValueExpressionKind::FlatMap { .. }),
                    budget,
                )?,
                ValueExpressionKind::With { .. } => {
                    self.updated_record_declaration(value, budget)?
                }
                ValueExpressionKind::Record(fields) => {
                    let mut types = Vec::new();
                    for (name, field) in fields {
                        let Some(declaration) = self.value_declaration(field, budget)? else {
                            return Ok(None);
                        };
                        budget_name(&name.value, budget, &name.range)?;
                        crate::projection::admit_slots::<(String, DeclaredType<Kind<C, H>>)>(
                            1,
                            budget,
                            &name.range,
                        )?;
                        types.push((name.value.clone(), declaration));
                    }
                    Some(DeclaredType::Record(types))
                }
                ValueExpressionKind::Tuple(values) => {
                    let mut types = Vec::new();
                    for v in values {
                        if let Some(d) = self.value_declaration(v, budget)? {
                            crate::projection::admit_slots::<DeclaredType<Kind<C, H>>>(
                                1,
                                budget,
                                &value.range,
                            )?;
                            types.push(d);
                        } else {
                            return Ok(None);
                        }
                    }
                    Some(DeclaredType::Tuple(types))
                }
                _ => self.local_kind(value, budget)?.map(DeclaredType::Native),
            })
        })
    }

    fn updated_record_declaration(
        &mut self,
        value: &ValueExpression<Query<C, H>>,
        budget: &mut Budget,
    ) -> Result<Option<DeclaredType<Kind<C, H>>>, QueryError> {
        let ValueExpressionKind::With { record, fields } = &value.value else {
            unreachable!("record update expression")
        };
        let Some(base) = self.value_declaration(record, budget)? else {
            return Ok(None);
        };
        let mut types = match base {
            DeclaredType::Record(types) => types,
            DeclaredType::Native(kind) => {
                let Some(view) = self.host.capture_record_kind(&kind) else {
                    return Ok(None);
                };
                let mut types = Vec::new();
                for (label, kind) in &view.fields {
                    budget_name(label, budget, &record.range)?;
                    crate::projection::admit_slots::<(String, DeclaredType<Kind<C, H>>)>(
                        1,
                        budget,
                        &record.range,
                    )?;
                    types.push((label.clone(), DeclaredType::Native(kind.clone())));
                }
                types
            }
            _ => return Ok(None),
        };
        for (label, field) in fields {
            let index = execution::record_field_position(
                types.iter().map(|(label, _)| label.as_str()),
                &label.value,
                budget,
                &Location::Query(label.range.clone()),
            )?;
            let Some(declaration) = self.value_declaration(field, budget)? else {
                return Ok(None);
            };
            types[index].1 = declaration;
        }
        Ok(Some(DeclaredType::Record(types)))
    }

    fn positional_element_kind(
        &self,
        kind: &Kind<C, H>,
        budget: &mut Budget,
        range: &Range<usize>,
    ) -> Result<Kind<C, H>, QueryError> {
        if let Some(element) = self.host.positional_view_kind(kind, budget, range)? {
            return Ok(element);
        }
        let location = Location::Query(range.clone());
        let types = self
            .host
            .scalar_types(kind, budget, range)?
            .ok_or_else(|| {
                QueryError::query(range.clone(), "operation needs positional sequences")
            })?;
        let mut elements = Vec::new();
        for ty in types {
            for element in crate::value::positional_elements(ty, budget, &location)? {
                crate::projection::admit_slots::<ValueType>(1, budget, range)?;
                elements.push(element);
            }
        }
        Ok(self.host.constructed_kind(elements))
    }

    fn mapping_declaration(
        &mut self,
        array: &ValueExpression<Query<C, H>>,
        argument: &crate::PredicateArgument<Query<C, H>>,
        flatten: bool,
        budget: &mut Budget,
    ) -> Result<Option<DeclaredType<Kind<C, H>>>, QueryError> {
        let kind = self.local_kind(array, budget)?;
        let element = match kind {
            Some(kind) => Some(self.positional_element_kind(&kind, budget, &array.range)?),
            None => match self.value_declaration(array, budget)? {
                Some(DeclaredType::Sequence(element) | DeclaredType::Array(element)) => {
                    Some(self.host.declaration_kind(&element, budget, &array.range)?)
                }
                _ => None,
            },
        };
        self.enter_scope(budget, &argument.body.range)?;
        let result = (|| {
            let internal = crate::value::PREDICATE_SUBJECT;
            self.scope.remove(internal);
            self.generator_kinds.remove(internal);
            self.contracts.remove(internal);
            if let Some(element) = &element {
                budget_name(internal, budget, &array.range)?;
                self.scope.insert(internal.to_owned(), element.clone());
            }
            if let Some(pattern) = &argument.argument {
                self.pattern_kind(PatternRef::Pattern(pattern), element, None, budget)?;
            }
            self.value_declaration(&argument.body, budget)
        })();
        self.leave_scope();
        let Some(body) = result? else { return Ok(None) };
        let element =
            if flatten {
                match body {
                    DeclaredType::Sequence(element) | DeclaredType::Array(element) => *element,
                    DeclaredType::Native(kind) => DeclaredType::Native(
                        self.positional_element_kind(&kind, budget, &argument.body.range)?,
                    ),
                    _ => {
                        return Err(QueryError::query(
                            argument.body.range.clone(),
                            "flat_map body needs a positional sequence",
                        ));
                    }
                }
            } else {
                body
            };
        Ok(Some(DeclaredType::Sequence(Box::new(element))))
    }

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
                        if let Some(d) = declaration {
                            self.contracts.insert(
                                name.value.clone(),
                                crate::runtime_types::copy_declaration(
                                    d,
                                    budget,
                                    &Location::Query(range.clone()),
                                )?,
                            );
                        }
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
                        if let Some(d) = declaration {
                            self.contracts.insert(
                                name.value.clone(),
                                crate::runtime_types::copy_declaration(
                                    d,
                                    budget,
                                    &Location::Query(range.clone()),
                                )?,
                            );
                        }
                    }
                }
                crate::PatternKind::Present(child) => {
                    let declaration = match declaration {
                        Some(DeclaredType::Optional(child)) => Some(child.as_ref()),
                        other => other,
                    };
                    let kind = if let Some(declaration) = declaration {
                        Some(self.host.declaration_kind(declaration, budget, range)?)
                    } else if let Some(kind) = kind {
                        // Native callbacks must see the same presence facts as
                        // the lexical value walker. Opaque native kinds retain
                        // their identity without a scalar projection.
                        Some(match self.host.scalar_types(&kind, budget, range)? {
                            Some(types) => {
                                self.host
                                    .constructed_kind(crate::value::pattern::present_types(
                                        &types,
                                        budget,
                                        &Location::Query(range.clone()),
                                    )?)
                            }
                            None => kind,
                        })
                    } else {
                        None
                    };
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
                crate::PatternKind::List { elements, .. } => {
                    let declaration = declaration.map(|mut d| {
                        while let DeclaredType::Optional(child) = d {
                            d = child;
                        }
                        d
                    });
                    for child in elements {
                        let d = match declaration {
                            Some(DeclaredType::Array(d) | DeclaredType::Sequence(d)) => {
                                Some(d.as_ref())
                            }
                            _ => None,
                        };
                        let k = match d {
                            Some(d) => Some(self.host.declaration_kind(d, budget, range)?),
                            None => match kind.as_ref() {
                                Some(k) => self.host.positional_view_kind(k, budget, range)?,
                                None => None,
                            },
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

    // The outer Option distinguishes no break (continue) from a constructed
    // break result. Nested loops are ordinary values, never outer exits.
    fn loop_kind(
        &mut self,
        value: &ValueExpression<Query<C, H>>,
        budget: &mut Budget,
    ) -> Result<Option<Option<Kind<C, H>>>, QueryError> {
        budget.within_evaluation(1, Location::Query(value.range.clone()), |budget| {
            budget.charge(1, Location::Query(value.range.clone()))?;
            match &value.value {
                ValueExpressionKind::Break(value) => Ok(Some(self.local_kind(value, budget)?)),
                ValueExpressionKind::Continue(_) => Ok(None),
                ValueExpressionKind::Block { bindings, value } => {
                    self.enter_scope(budget, &value.range)?;
                    let result = (|| {
                        for (pattern, source) in bindings {
                            let kind = self.local_kind(source, budget)?;
                            self.pattern_kind(PatternRef::Pattern(pattern), kind, None, budget)?;
                        }
                        self.loop_kind(value, budget)
                    })();
                    self.leave_scope();
                    result
                }
                ValueExpressionKind::Conditional {
                    branches,
                    otherwise,
                } => {
                    let mut kind = self.loop_kind(otherwise, budget)?;
                    for (_, branch) in branches {
                        if let Some(next) = self.loop_kind(branch, budget)? {
                            if kind.as_ref().is_some_and(|previous| *previous != next) {
                                return Ok(Some(None));
                            }
                            kind = Some(next);
                        }
                    }
                    Ok(kind)
                }
                ValueExpressionKind::ValueMatch { subject, arms } => {
                    let subject = self.local_kind(subject, budget)?;
                    let mut kind = None;
                    for arm in arms {
                        self.enter_scope(budget, &arm.pattern.range)?;
                        let result = (|| {
                            self.pattern_kind(
                                PatternRef::Pattern(&arm.pattern),
                                subject.clone(),
                                None,
                                budget,
                            )?;
                            self.loop_kind(&arm.value, budget)
                        })();
                        self.leave_scope();
                        if let Some(next) = result? {
                            if kind.as_ref().is_some_and(|previous| *previous != next) {
                                return Ok(Some(None));
                            }
                            kind = Some(next);
                        }
                    }
                    Ok(kind)
                }
                _ => Ok(None), // Structural preparation reports a missing terminal transfer.
            }
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
                ValueExpressionKind::Queue { .. }
                | ValueExpressionKind::IdentitySet { .. }
                | ValueExpressionKind::PopFront(_)
                | ValueExpressionKind::PushBackAll { .. }
                | ValueExpressionKind::Insert { .. }
                | ValueExpressionKind::Array(_)
                | ValueExpressionKind::Record(_)
                | ValueExpressionKind::With { .. }
                | ValueExpressionKind::Map { .. }
                | ValueExpressionKind::FlatMap { .. }
                | ValueExpressionKind::Windows { .. }
                | ValueExpressionKind::RunsBy { .. }
                | ValueExpressionKind::Tuple(_) => match self.value_declaration(value, budget)? {
                    Some(d)
                        if crate::runtime_types::contains_native(
                            &d,
                            budget,
                            &Location::Query(value.range.clone()),
                        )? =>
                    {
                        Some(self.host.declaration_kind(&d, budget, &value.range)?)
                    }
                    _ => None,
                },
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
                ValueExpressionKind::PredicateSubject => self
                    .scope
                    .get(crate::value::PREDICATE_SUBJECT)
                    .or_else(|| self.generator_kinds.get(crate::value::PREDICATE_SUBJECT))
                    .cloned(),
                ValueExpressionKind::Name(name) => self
                    .scope
                    .get(name)
                    .or_else(|| self.generator_kinds.get(name))
                    .cloned(),
                ValueExpressionKind::SubjectScope { subject, argument } => {
                    let kind = self.local_kind(subject, budget)?;
                    self.enter_scope(budget, &value.range)?;
                    let result = (|| {
                        let internal = crate::value::PREDICATE_SUBJECT;
                        self.scope.remove(internal);
                        self.generator_kinds.remove(internal);
                        self.contracts.remove(internal);
                        if let Some(kind) = &kind {
                            budget_name(internal, budget, &subject.range)?;
                            self.scope.insert(internal.to_owned(), kind.clone());
                        }
                        if let Some(pattern) = &argument.argument {
                            self.pattern_kind(PatternRef::Pattern(pattern), kind, None, budget)?;
                        }
                        self.local_kind(&argument.body, budget)
                    })();
                    self.leave_scope();
                    result?
                }
                ValueExpressionKind::Field { value, field } => {
                    match self.local_kind(value, budget)? {
                        Some(kind) => self
                            .host
                            .capture_record_kind(&kind)
                            .map(|record| record.get(field, budget).cloned())
                            .transpose()?,
                        None => None,
                    }
                }
                ValueExpressionKind::Loop { states, body } => {
                    crate::projection::admit_slots::<Option<Kind<C, H>>>(
                        states.len(),
                        budget,
                        &value.range,
                    )?;
                    let mut kinds = Vec::with_capacity(states.len());
                    for (_, value) in states {
                        kinds.push(self.local_kind(value, budget)?);
                    }
                    self.enter_scope(budget, &value.range)?;
                    let result = (|| {
                        for ((name, _), kind) in states.iter().zip(kinds) {
                            if let Some(kind) = kind {
                                budget.charge(
                                    name.value.len() + 1,
                                    Location::Query(name.range.clone()),
                                )?;
                                self.scope.insert(name.value.clone(), kind);
                            }
                        }
                        self.loop_kind(body, budget)
                    })();
                    self.leave_scope();
                    result?.flatten()
                }
                ValueExpressionKind::Break(value) => self.local_kind(value, budget)?,
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
    fn prepare_computation(
        &mut self,
        expression: &ValueExpression<Query<C, H>>,
        budget: &mut Budget,
    ) -> Result<(), QueryError> {
        match &expression.value {
            ValueExpressionKind::Call {
                function,
                arguments,
            } => {
                let (id, _) = self.function(function, budget)?;
                for (v, p) in arguments
                    .iter()
                    .zip(self.declarations[id].parameters.iter())
                {
                    self.expect(v, p.declaration, budget)?;
                }
            }
            ValueExpressionKind::Fold { initial, step, .. }
            | ValueExpressionKind::Scan { initial, step, .. } => {
                let (id, _) = self.function(step, budget)?;
                let d = accumulator_state(&self.declarations[id], step, budget)?;
                self.expect(initial, d, budget)?;
            }
            ValueExpressionKind::Map { array, argument }
            | ValueExpressionKind::FlatMap { array, argument } => {
                self.mapping_declaration(
                    array,
                    argument,
                    matches!(expression.value, ValueExpressionKind::FlatMap { .. }),
                    budget,
                )?;
            }
            ValueExpressionKind::Loop { states, body } => {
                let mut types = Vec::new();
                for (_, v) in states {
                    crate::projection::admit_slots::<Option<DeclaredType<Kind<C, H>>>>(
                        1,
                        budget,
                        &expression.range,
                    )?;
                    types.push(self.value_declaration(v, budget)?);
                }
                self.expect_next(body, &types, budget)?;
            }
            _ => {}
        }
        Ok(())
    }
    fn worklist_id(
        &mut self,
        expression: &ValueExpression<Query<C, H>>,
        budget: &mut Budget,
    ) -> Result<usize, QueryError> {
        let d = self
            .worklists
            .get(&(expression as *const _ as usize))
            .expect("resolved worklist constructor");
        let d = d
            .as_ref()
            .map(|d| {
                crate::runtime_types::copy_declaration(
                    d,
                    budget,
                    &Location::Query(expression.range.clone()),
                )
            })
            .transpose()?;
        crate::projection::admit_slots::<Option<DeclaredType<Kind<C, H>>>>(
            1,
            budget,
            &expression.range,
        )?;
        let id = self.worklist_contracts.len();
        self.worklist_contracts.push(d);
        Ok(id)
    }
    fn worklist_types(
        &mut self,
        expression: &ValueExpression<Query<C, H>>,
        budget: &mut Budget,
    ) -> Result<Vec<ValueType>, QueryError> {
        let id = expression as *const _ as usize;
        let location = Location::Query(expression.range.clone());
        let declaration = match &expression.value {
            ValueExpressionKind::Queue { items, element } => {
                let explicit = element
                    .as_ref()
                    .map(|t| {
                        t.value.resolve(budget, &t.range, &mut |name, b| {
                            self.host.resolve_type_name(name, b)
                        })
                    })
                    .transpose()?;
                let contextual = match self.worklists.get(&id) {
                    Some(Some(DeclaredType::Queue(d))) => Some(
                        crate::runtime_types::copy_declaration(d, budget, &location)?,
                    ),
                    _ => None,
                };
                if let (Some(a), Some(b)) = (&explicit, &contextual)
                    && !crate::runtime_types::same_declaration(a, b, budget, &location)?
                {
                    return Err(QueryError::query(
                        expression.range.clone(),
                        "incompatible queue element declaration",
                    ));
                }
                let element = match explicit.or(contextual) {
                    Some(d) => Some(d),
                    None => match self.value_declaration(items, budget)? {
                        Some(DeclaredType::Sequence(d) | DeclaredType::Array(d)) => Some(*d),
                        Some(DeclaredType::Native(k))
                            if self.host.scalar_types(&k, budget, &items.range)?.is_none() =>
                        {
                            Some(DeclaredType::Native(self.host.subject_kind(
                                &k,
                                budget,
                                &items.range,
                            )?))
                        }
                        _ => None,
                    },
                };
                if element.is_none()
                    && matches!(&items.value, ValueExpressionKind::Array(v) if v.is_empty())
                {
                    return Err(QueryError::query(
                        expression.range.clone(),
                        "empty queue needs an explicit or contextual element contract",
                    ));
                }
                element.map(|d| DeclaredType::Queue(Box::new(d)))
            }
            ValueExpressionKind::IdentitySet { element } => Some(
                match element
                    .value
                    .resolve(budget, &element.range, &mut |name, b| {
                        self.host.resolve_type_name(name, b)
                    })? {
                    DeclaredType::Native(k) => DeclaredType::IdentitySet(k),
                    _ => {
                        return Err(QueryError::query(
                            element.range.clone(),
                            "identity_set requires a native element type",
                        ));
                    }
                },
            ),
            _ => return Ok(vec![ValueType::Unestablished]),
        };
        if let Some(d) = &declaration {
            crate::runtime_types::validate_declaration(d, budget, &location)?;
        }
        self.host
            .prepare_computational(declaration.as_ref(), budget, &expression.range)?;
        crate::projection::admit_slots::<Option<DeclaredType<Kind<C, H>>>>(
            1,
            budget,
            &expression.range,
        )?;
        self.worklists.insert(id, declaration);
        Ok(vec![ValueType::Unestablished])
    }
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
    fn predicate_subject_types(
        &mut self,
        range: &Range<usize>,
        budget: &mut Budget,
    ) -> Result<Vec<ValueType>, QueryError> {
        let kind = self
            .scope
            .get(crate::value::PREDICATE_SUBJECT)
            .ok_or_else(|| {
                QueryError::query(
                    range.clone(),
                    "unnamed argument is only available in a predicate",
                )
            })?;
        Ok(self
            .host
            .scalar_types(kind, budget, range)?
            .unwrap_or_else(|| vec![ValueType::Unestablished]))
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
        let mut contracts = BTreeMap::new();
        for (name, d) in &self.contracts {
            budget_name(name, budget, range)?;
            contracts.insert(
                name.clone(),
                crate::runtime_types::copy_declaration(d, budget, &Location::Query(range.clone()))?,
            );
        }
        self.scopes.push(LexicalKinds {
            scope,
            generators,
            contracts,
        });
        Ok(())
    }
    fn leave_scope(&mut self) {
        let saved = self.scopes.pop().expect("entered lexical scope");
        self.scope = saved.scope;
        self.generator_kinds = saved.generators;
        self.contracts = saved.contracts;
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
        if let Some(d) = self.value_declaration(value, budget)? {
            self.contracts.insert(name.value.clone(), d);
        }
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
        binder: &Pattern,
        source: &ValueExpression<Query<C, H>>,
        types: &[ValueType],
        element: bool,
        budget: &mut Budget,
    ) -> Result<(), QueryError> {
        let kind = match self.local_kind(source, budget)? {
            Some(kind)
                if element
                    && self
                        .host
                        .scalar_types(&kind, budget, &source.range)?
                        .is_none() =>
            {
                self.host.subject_kind(&kind, budget, &source.range)?
            }
            Some(kind) if !element => kind,
            _ => self.host.constructed_kind(copy_value_types(
                types,
                budget,
                Location::Query(binder.range.clone()),
            )?),
        };
        for name in crate::pattern_names(binder, budget)? {
            self.generator_kinds.remove(&name.value);
        }
        self.pattern_kind(PatternRef::Pattern(binder), Some(kind), None, budget)?;
        for name in crate::pattern_names(binder, budget)? {
            if let Some(kind) = self.scope.remove(&name.value) {
                self.generator_kinds.insert(name.value.clone(), kind);
            }
        }
        Ok(())
    }
    fn prepare_predicate(
        &mut self,
        argument: Option<&Pattern>,
        source: &ValueExpression<Query<C, H>>,
        types: &[ValueType],
        element: bool,
        budget: &mut Budget,
    ) -> Result<(), QueryError> {
        let kind = match self.local_kind(source, budget)? {
            Some(kind)
                if element
                    && self
                        .host
                        .scalar_types(&kind, budget, &source.range)?
                        .is_none() =>
            {
                return Err(QueryError::query(
                    source.range.clone(),
                    "constructed filter needs a positional array; use the host's native filter for native selections",
                ));
            }
            Some(kind) if !element => kind,
            _ => self.host.constructed_kind(copy_value_types(
                types,
                budget,
                Location::Query(source.range.clone()),
            )?),
        };
        let internal = crate::value::PREDICATE_SUBJECT;
        budget_name(internal, budget, &source.range)?;
        self.scope.insert(internal.to_owned(), kind.clone());
        self.generator_kinds
            .insert(internal.to_owned(), kind.clone());
        if let Some(pattern) = argument {
            self.pattern_kind(PatternRef::Pattern(pattern), Some(kind), None, budget)?;
        }
        Ok(())
    }
    fn prepare_mapping(
        &mut self,
        argument: Option<&Pattern>,
        source: &ValueExpression<Query<C, H>>,
        types: &[ValueType],
        budget: &mut Budget,
    ) -> Result<(), QueryError> {
        let kind = match self.local_kind(source, budget)? {
            Some(kind) => self.positional_element_kind(&kind, budget, &source.range)?,
            None => self.host.constructed_kind(copy_value_types(
                types,
                budget,
                Location::Query(source.range.clone()),
            )?),
        };
        let internal = crate::value::PREDICATE_SUBJECT;
        self.scope.remove(internal);
        self.generator_kinds.remove(internal);
        self.contracts.remove(internal);
        budget_name(internal, budget, &source.range)?;
        self.scope.insert(internal.to_owned(), kind.clone());
        self.generator_kinds
            .insert(internal.to_owned(), kind.clone());
        if let Some(pattern) = argument {
            self.pattern_kind(PatternRef::Pattern(pattern), Some(kind), None, budget)?;
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
    source: &'a ValueExpression<Q>,
    kind: K,
    prepared: BTreeMap<usize, Operand<K>>,
    value: crate::value::prepared::PreparedValue<'a, Q>,
    names: Vec<&'a str>,
    identity: std::sync::Arc<()>,
}

/// A prepared initializer and its atomic, ordered public bindings.
pub struct PreparedPatternBinding<'a, Q, K> {
    pattern: &'a Pattern,
    value: PreparedFunctionValue<'a, Q, K>,
    bindings: Vec<(&'a Located<String>, K)>,
}
impl<K, Q> PreparedPatternBinding<'_, Q, K> {
    /// Bound names and host kinds in written pattern order.
    pub fn bindings(&self) -> impl Iterator<Item = (&Located<String>, &K)> {
        self.bindings.iter().map(|(name, kind)| (*name, kind))
    }
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

    /// Prepare request-resolved pattern headers without cloning body trees.
    ///
    /// # Errors
    /// Every declaration and written branch receives the ordinary scope, purity
    /// and allowance checks before execution.
    pub fn prepare_patterns_borrowed(
        declarations: impl IntoIterator<
            Item = PatternFunctionDeclarationRef<'a, Query<C, H>, Kind<C, H>>,
        >,
        host: &'h H,
        budget: &mut Budget,
    ) -> Result<Self, FunctionError<EvaluationError<H::Error>>> {
        Self::prepare_headers(
            declarations.into_iter().map(DeclarationRef::from),
            host,
            budget,
        )
    }

    fn prepare_headers(
        declarations: impl IntoIterator<Item = DeclarationRef<'a, Query<C, H>, Kind<C, H>>>,
        host: &'h H,
        budget: &mut Budget,
    ) -> Result<Self, FunctionError<EvaluationError<H::Error>>> {
        let mut preflight = Preflight::new(declarations, host, budget)?;
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
            preflight.contracts.clear();
            preflight.worklists.clear();
            for p in declaration.parameters.iter() {
                let kind = host.declaration_kind(p.declaration, budget, p.range)?;
                preflight.pattern_kind(p.pattern, Some(kind), Some(p.declaration), budget)?;
            }
            preflight.expect(declaration.value, &declaration.result.value, budget)?;
            bodies.push(crate::value::prepare_function_body(
                declaration,
                &mut preflight,
                budget,
            )?);
            body_prepared.push(std::mem::take(&mut preflight.prepared));
        }
        preflight.scope.clear();
        preflight.contracts.clear();
        preflight.worklists.clear();
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
        let saved = std::mem::take(&mut self.preflight.prepared);
        let saved_worklists = std::mem::take(&mut self.preflight.worklists);
        let saved_tallies = std::mem::take(&mut self.preflight.general_key_expressions);
        let result = (|| {
            let kind = self.preflight.entry_kind(entry, scope, relative, budget)?;
            let mut preparation = Preparation::new();
            preparation.general_key_expressions =
                std::mem::take(&mut self.preflight.general_key_expressions);
            let value = preparation.resolve(entry, &mut self.preflight, budget)?;
            budget.charge(1, Location::Query(entry.range.clone()))?;
            Ok(PreparedFunctionValue {
                source: entry,
                kind,
                value,
                names: preparation.names,
                prepared: std::mem::take(&mut self.preflight.prepared),
                identity: std::sync::Arc::clone(&self.identity),
            })
        })();
        self.preflight.prepared = saved;
        self.preflight.worklists = saved_worklists;
        self.preflight.general_key_expressions = saved_tallies;
        result
    }

    /// Discover a value's host kind through the same preflight used by prepared
    /// execution. All written branches, lexical patterns and native arguments
    /// are checked; actual values still require runtime admission.
    ///
    /// # Errors
    /// Invalid scope, effects, host operations and exhausted allowances fail.
    pub fn value_kind(
        &mut self,
        entry: &ValueExpression<Query<C, H>>,
        scope: &BTreeMap<String, Kind<C, H>>,
        relative: Option<Kind<C, H>>,
        budget: &mut Budget,
    ) -> Result<Kind<C, H>, FunctionError<EvaluationError<H::Error>>> {
        let saved = std::mem::take(&mut self.preflight.prepared);
        let saved_worklists = std::mem::take(&mut self.preflight.worklists);
        let saved_tallies = std::mem::take(&mut self.preflight.general_key_expressions);
        let result = self.preflight.entry_kind(entry, scope, relative, budget);
        self.preflight.prepared = saved;
        self.preflight.worklists = saved_worklists;
        self.preflight.general_key_expressions = saved_tallies;
        Ok(result?)
    }

    /// Prepare one atomic pattern binding. The initializer cannot see its new
    /// names. Only selected bindings are materialized by [`Self::evaluate_binding`].
    ///
    /// # Errors
    /// Duplicate/shadowed names, invalid patterns, effects and limits fail.
    #[expect(
        clippy::type_complexity,
        reason = "The binding retains the existing host expression and kind types"
    )]
    pub fn prepare_binding(
        &mut self,
        pattern: &'a Pattern,
        initializer: &'a ValueExpression<Query<C, H>>,
        scope: &BTreeMap<String, Kind<C, H>>,
        relative: Option<Kind<C, H>>,
        budget: &mut Budget,
    ) -> Result<
        PreparedPatternBinding<'a, Query<C, H>, Kind<C, H>>,
        FunctionError<EvaluationError<H::Error>>,
    > {
        let value = self.prepare_value(initializer, scope, relative, budget)?;
        let types = self
            .preflight
            .host
            .scalar_types(value.kind(), budget, &initializer.range)?
            .unwrap_or_else(|| vec![ValueType::Unestablished]);
        let facts = crate::value::pattern::facts(pattern, &types, budget)?;
        let mut names = std::collections::BTreeSet::new();
        for (name, _) in &facts {
            budget_name(&name.value, budget, &name.range)?;
            crate::projection::admit_slots::<&str>(1, budget, &name.range)?;
            if scope.contains_key(&name.value) || !names.insert(name.value.as_str()) {
                return Err(
                    QueryError::query(name.range.clone(), "binding is already defined").into(),
                );
            }
        }
        self.preflight.pattern_kind(
            PatternRef::Pattern(pattern),
            Some(value.kind.clone()),
            None,
            budget,
        )?;
        let mut bindings = Vec::new();
        for (name, types) in facts {
            crate::projection::admit_slots::<(&Located<String>, Kind<C, H>)>(
                1,
                budget,
                &name.range,
            )?;
            let kind = self
                .preflight
                .scope
                .get(&name.value)
                .cloned()
                .unwrap_or_else(|| self.preflight.host.constructed_kind(types));
            bindings.push((name, kind));
        }
        Ok(PreparedPatternBinding {
            pattern,
            value,
            bindings,
        })
    }

    /// Evaluate the initializer once, then match and finish its selected fields.
    /// A mismatch returns no capture; ignored fields are never copied as output.
    ///
    /// # Errors
    /// A mismatch, host failure, invalid ownership, runtime contract or limit
    /// fails before the caller can install any names.
    pub fn evaluate_binding(
        &self,
        entry: &PreparedPatternBinding<'a, Query<C, H>, Kind<C, H>>,
        bindings: &Bindings<H::Value>,
        relative: Option<&H::Value>,
        context: &mut C,
        budget: &mut Budget,
        limits: FunctionLimits,
    ) -> Result<crate::NativeCapture<H::Value>, FunctionError<EvaluationError<H::Error>>> {
        let value = &entry.value;
        if !std::sync::Arc::ptr_eq(&self.identity, &value.identity) {
            return Err(QueryError::query(
                entry.pattern.range.clone(),
                "binding belongs to a different prepared function program",
            )
            .into());
        }
        let mut calls = CallTrace::default();
        let mut runtime = Runtime {
            host: self.preflight.host,
            context,
            bindings,
            contracts: &BTreeMap::new(),
            prepared: &value.prepared,
            body_prepared: &self.body_prepared,
            worklists: &self.preflight.worklist_contracts,
            source_function: None,
            relative,
            limit: limits.active_depth,
            calls: &mut calls,
        };
        let location = Location::Query(entry.pattern.range.clone());
        let result = (|| {
            let value = execution::execute(
                &value.value,
                &value.names,
                &self.bodies,
                &mut runtime,
                budget,
                limits.active_depth,
                0,
            )?;
            execution::Host::inspect_pattern(&mut runtime, &value, budget, &location)?;
            let selected = crate::value::pattern::bind::<Query<C, H>, _>(
                PatternRef::Pattern(entry.pattern),
                value,
                &mut runtime,
                budget,
            )?
            .ok_or_else(|| {
                QueryError::query(entry.pattern.range.clone(), "binding pattern did not match")
            })?;
            crate::projection::admit_slots::<(String, H::Value)>(
                selected.len(),
                budget,
                &entry.pattern.range,
            )?;
            let mut fields = Vec::with_capacity(selected.len());
            for (name, value) in selected {
                budget_name(name, budget, &entry.pattern.range)?;
                fields.push((
                    name.to_owned(),
                    runtime.finish_value(value, budget, &location)?,
                ));
            }
            Ok(crate::NativeCapture { fields })
        })();
        result.map_err(|error| self.function_error(error, calls))
    }

    fn function_error(
        &self,
        error: EvaluationError<H::Error>,
        calls: CallTrace,
    ) -> FunctionError<EvaluationError<H::Error>> {
        FunctionError {
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
        }
    }

    /// Execute a projection using its already-prepared fields in written order.
    /// Prepare each field once with the subject name in scope and its singleton
    /// kind as relative input. Pass those entries from this unit in field order.
    /// All fields must admit scalar projection, even when there are no subjects.
    /// Result facts retain the record schema of an empty report.
    ///
    /// # Errors
    /// Shadowing, duplicate labels, mismatched entries, scalar admission and
    /// allowances fail before enumeration. Field failures retain their locations
    /// and active function calls; all work uses the supplied cumulative budget.
    #[expect(
        clippy::type_complexity,
        reason = "The projection borrows existing prepared native field entries"
    )]
    #[expect(
        clippy::too_many_arguments,
        reason = "The bridge keeps the existing projection, prepared entries and execution borrows explicit"
    )]
    pub fn evaluate_projection(
        &self,
        projection: &crate::Projection<Query<C, H>>,
        fields: &[&PreparedFunctionValue<'a, Query<C, H>, Kind<C, H>>],
        input: H::Value,
        bindings: &Bindings<H::Value>,
        context: &mut C,
        budget: &mut Budget,
        limits: FunctionLimits,
    ) -> Result<crate::ConstructedResult, FunctionError<EvaluationError<H::Error>>> {
        let range = &projection.subject.range;
        budget.within_evaluation(1, Location::Query(range.clone()), |budget| {
            if bindings
                .iter()
                .any(|(name, _)| name == projection.subject.value)
            {
                return Err(QueryError::query(
                    range.clone(),
                    "projection subject shadows an existing binding",
                )
                .into());
            }
            if fields.len() != projection.fields.len() {
                return Err(QueryError::query(
                    range.clone(),
                    "projection requires every written prepared field",
                )
                .into());
            }
            let host = self.preflight.host;
            crate::projection::admit_slots::<Vec<ValueType>>(1, budget, range)?;
            let mut positions = vec![Vec::new()];
            for (index, ((_, expression), entry)) in
                projection.fields.iter().zip(fields).enumerate()
            {
                crate::projection::check_projection_label(projection, index, budget)?;
                if !std::sync::Arc::ptr_eq(&self.identity, &entry.identity) {
                    return Err(QueryError::query(
                        expression.range.clone(),
                        "value belongs to a different prepared function program",
                    )
                    .into());
                }
                if !std::ptr::eq(expression, entry.source) {
                    return Err(QueryError::query(
                        expression.range.clone(),
                        "prepared entry does not match the written projection field",
                    )
                    .into());
                }
                let types = host
                    .scalar_types(entry.kind(), budget, &expression.range)?
                    .ok_or_else(|| {
                        QueryError::query(
                            expression.range.clone(),
                            "native handles require an explicit scalar projection",
                        )
                    })?;
                positions = crate::value::extend_type_positions(
                    &positions,
                    &types,
                    budget,
                    &Location::Query(expression.range.clone()),
                )?;
            }
            let types = crate::projection::projection_result_types(projection, positions, budget)?;
            let subjects = host
                .subjects(input, context, budget, range)
                .map_err(|error| self.function_error(error, CallTrace::default()))?;
            let value = crate::projection::projection_rows(
                projection,
                subjects,
                bindings,
                context,
                budget,
                |index, local, context, budget| {
                    let expression = &projection.fields[index].1;
                    let relative = local.get(&projection.subject.value, range.clone())?;
                    let value = self.evaluate(
                        fields[index],
                        local,
                        Some(relative),
                        context,
                        budget,
                        limits,
                    )?;
                    host.project_value(value, context, budget, &expression.range)
                        .map_err(|error| self.function_error(error, CallTrace::default()))
                },
            )?;
            Ok(crate::ConstructedResult { value, types })
        })
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
            worklists: &self.preflight.worklist_contracts,
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
        result.map_err(|error| self.function_error(error, calls))
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
    worklists: &'r [Option<DeclaredType<Kind<C, H>>>],
    source_function: Option<usize>,
    relative: Option<&'r H::Value>,
    limit: usize,
    calls: &'r mut CallTrace,
}
impl<'a, C, H: FunctionHost<C>> Runtime<'_, 'a, C, H> {
    fn value_contract(
        &mut self,
        value: &H::Value,
        budget: &mut Budget,
        location: &Location,
    ) -> Result<DeclaredType<Kind<C, H>>, EvaluationError<H::Error>> {
        budget.within_evaluation(1, location.clone(), |budget| {
            budget.charge(1, location.clone())?;
            if let Some(c) = self.host.computational_value(value) {
                return Ok(c.declaration(budget, location)?);
            }
            if let Some(c) = self.host.constructed_value(value)
                && let Some(d) = crate::worklists::scalar_contract(c)
            {
                return Ok(d);
            }
            if let Some(values) = self.host.tuple_view(value) {
                let mut types = Vec::new();
                for v in values {
                    crate::projection::admit_slots::<DeclaredType<Kind<C, H>>>(
                        1,
                        budget,
                        match location {
                            Location::Query(r) | Location::Source(r) => r,
                        },
                    )?;
                    types.push(self.value_contract(v, budget, location)?);
                }
                return Ok(DeclaredType::Tuple(types));
            }
            let range = match location {
                Location::Query(r) | Location::Source(r) => r,
            };
            Ok(DeclaredType::Native(self.host.kind(value, budget, range)?))
        })
    }
    fn infer_queue_element(
        &mut self,
        value: &HostValue<'_, C, H>,
        budget: &mut Budget,
        location: &Location,
    ) -> Result<DeclaredType<Kind<C, H>>, EvaluationError<H::Error>> {
        let range = match location {
            Location::Query(r) | Location::Source(r) => r,
        };
        if let Some(data) = execution::Host::constructed(self, value, budget, location)? {
            let mut actual = constructed_value_type(data.value, budget, location.clone())?;
            if let Some(d) = data.declaration {
                budget.charge_results(1, location.clone())?;
                actual = ValueType::Declared {
                    actual: Box::new(actual),
                    declaration: crate::runtime_types::copy_declaration(d, budget, location)?,
                };
            }
            return match crate::worklists::fact_contract(&actual, budget, location)? {
                Some(DeclaredType::Array(d) | DeclaredType::Sequence(d)) => Ok(*d),
                _ => Err(QueryError::query(
                    range.clone(),
                    "empty queue needs an explicit or contextual element contract",
                )
                .into()),
            };
        }
        match &value.data {
            Data::Constructed(_) => unreachable!("constructed view handled above"),
            Data::Native(view) => {
                budget.charge(view.path.len(), location.clone())?;
                let kind = self
                    .host
                    .kind(native_view::<C, H>(view, self.host), budget, range)?;
                let element = self.host.subject_kind(&kind, budget, range)?;
                if let Some(types) = self.host.scalar_types(&element, budget, range)? {
                    if let [fact] = types.as_slice()
                        && let Some(d) = crate::worklists::fact_contract(fact, budget, location)?
                    {
                        return Ok(d);
                    }
                    return Err(QueryError::query(
                        range.clone(),
                        "queue input needs an established element contract",
                    )
                    .into());
                }
                Ok(DeclaredType::Native(element))
            }
        }
    }
    fn queue_items(
        &mut self,
        value: HostValue<'_, C, H>,
        element: &DeclaredType<Kind<C, H>>,
        budget: &mut Budget,
        location: &Location,
    ) -> Result<Vec<H::Value>, EvaluationError<H::Error>> {
        let value = execution::Host::normalize(self, value, budget, location)?;
        let range = match location {
            Location::Query(r) | Location::Source(r) => r,
        };
        match value.data {
            Data::Native(view) => {
                budget.charge(view.path.len(), location.clone())?;
                self.host.sequence_items(
                    native_view::<C, H>(&view, self.host),
                    element,
                    self.context,
                    budget,
                    range,
                )
            }
            Data::Constructed(data) => {
                let data = data.materialize(budget, location)?;
                let ConstructedValue::Array(values) = data.data() else {
                    return Err(QueryError::query(
                        range.clone(),
                        "queue input needs a positional sequence",
                    )
                    .into());
                };
                crate::projection::admit_slots::<H::Value>(values.len(), budget, range)?;
                let mut output = Vec::with_capacity(values.len());
                for v in values {
                    let v = copy_constructed_value(v, budget, location.clone())?;
                    let facts = vec![constructed_value_type(&v, budget, location.clone())?];
                    output.push(self.host.lift_constructed(
                        v,
                        facts,
                        self.context,
                        budget,
                        range,
                    )?);
                }
                Ok(output)
            }
        }
    }
    fn computational(
        &mut self,
        value: crate::ComputationalValue<H::Value, Kind<C, H>>,
        budget: &mut Budget,
        location: &Location,
    ) -> Result<H::Value, EvaluationError<H::Error>> {
        let range = match location {
            Location::Query(r) | Location::Source(r) => r,
        };
        crate::projection::admit_slots::<H::Value>(1, budget, range)?;
        self.host
            .lift_computational(value, self.context, budget, range)
    }
    fn compute_collection(
        &mut self,
        expression: &'a PreparedValue<'a, Query<C, H>>,
        values: &mut Vec<HostValue<'a, C, H>>,
        budget: &mut Budget,
    ) -> Result<Option<HostValue<'a, C, H>>, EvaluationError<H::Error>> {
        let location = Location::Query(expression.range.clone());
        let range = &expression.range;
        let output = match &expression.value {
            PreparedKind::Field { field, .. } if matches!(&values[0].data, Data::Native(view) if self.host.dictionary_view(native_view::<C,H>(view, self.host)).is_some()) =>
            {
                let dictionary =
                    self.finish_value(values.pop().expect("dictionary"), budget, &location)?;
                budget.charge_results(field.value.len() + 1, location.clone())?;
                let key = ConstructedValue::String(field.value.clone());
                let facts = vec![ValueType::String];
                let key = self
                    .host
                    .lift_constructed(key, facts, self.context, budget, range)?;
                let value = crate::lookup_native_dictionary(
                    &dictionary,
                    &key,
                    self.host,
                    self.context,
                    budget,
                    location.clone(),
                )?
                .ok_or_else(|| {
                    QueryError::query(field.range.clone(), "missing dictionary field")
                })?;
                return self
                    .retained(std::sync::Arc::new(value), budget, range)
                    .map(Some);
            }
            PreparedKind::Reduce {
                operator: crate::ValueReducer::Length,
                ..
            } if matches!(&values[0].data, Data::Native(view) if {
                let value = native_view::<C,H>(view, self.host);
                self.host.set_view(value).is_some() || self.host.dictionary_view(value).is_some() || self.host.positional_view(value).is_some()
            }) =>
            {
                let value =
                    self.finish_value(values.pop().expect("collection"), budget, &location)?;
                let len = self
                    .host
                    .set_view(&value)
                    .map(<[_]>::len)
                    .or_else(|| self.host.dictionary_view(&value).map(<[_]>::len))
                    .or_else(|| self.host.positional_view(&value).map(<[_]>::len))
                    .expect("native collection");
                let len = i64::try_from(len)
                    .map_err(|_| QueryError::query(range.clone(), "length overflow"))?;
                budget.charge_results(1, location.clone())?;
                return Ok(Some(Value::constructed(RuntimeValue::Owned(
                    ConstructedValue::Integer(len),
                ))));
            }
            PreparedKind::Binary {
                operator: crate::ValueBinaryOperator::Equal | crate::ValueBinaryOperator::NotEqual,
                ..
            } if values
                .iter()
                .any(|value| matches!(value.data, Data::Native(_))) =>
            {
                let right =
                    self.finish_value(values.pop().expect("right operand"), budget, &location)?;
                let left =
                    self.finish_value(values.pop().expect("left operand"), budget, &location)?;
                let equal = crate::native_values_equal(
                    &left,
                    &right,
                    self.host,
                    self.context,
                    budget,
                    location.clone(),
                )?;
                return self
                    .boolean_value(
                        equal
                            == matches!(
                                expression.value,
                                PreparedKind::Binary {
                                    operator: crate::ValueBinaryOperator::Equal,
                                    ..
                                }
                            ),
                        budget,
                        &location,
                    )
                    .map(Some);
            }
            PreparedKind::Binary {
                operator:
                    operator @ (crate::ValueBinaryOperator::Less
                    | crate::ValueBinaryOperator::LessEqual
                    | crate::ValueBinaryOperator::Greater
                    | crate::ValueBinaryOperator::GreaterEqual),
                ..
            } if values
                .iter()
                .any(|value| matches!(value.data, Data::Native(_))) =>
            {
                let right =
                    self.finish_value(values.pop().expect("right operand"), budget, &location)?;
                let left =
                    self.finish_value(values.pop().expect("left operand"), budget, &location)?;
                let order = crate::native_values_compare(
                    &left,
                    &right,
                    self.host,
                    self.context,
                    budget,
                    location.clone(),
                )?;
                use crate::ValueBinaryOperator as B;
                let found = match operator {
                    B::Less => order.is_lt(),
                    B::LessEqual => order.is_le(),
                    B::Greater => order.is_gt(),
                    B::GreaterEqual => order.is_ge(),
                    _ => unreachable!(),
                };
                return self.boolean_value(found, budget, &location).map(Some);
            }
            PreparedKind::Lookup { .. }
            | PreparedKind::HasKey { .. }
            | PreparedKind::ArrayAccess { .. }
                if matches!(&values[0].data, Data::Native(view) if self.host.dictionary_view(native_view::<C,H>(view, self.host)).is_some()) =>
            {
                let key =
                    self.finish_value(values.pop().expect("lookup key"), budget, &location)?;
                let dictionary =
                    self.finish_value(values.pop().expect("dictionary"), budget, &location)?;
                let found = crate::lookup_native_dictionary(
                    &dictionary,
                    &key,
                    self.host,
                    self.context,
                    budget,
                    location.clone(),
                )?;
                if matches!(expression.value, PreparedKind::HasKey { .. }) {
                    return self
                        .boolean_value(found.is_some(), budget, &location)
                        .map(Some);
                }
                return match found {
                    Some(value) => self.retained(std::sync::Arc::new(value), budget, range),
                    None if matches!(expression.value, PreparedKind::Lookup { .. }) => {
                        budget.charge_results(1, location.clone())?;
                        Ok(Value::constructed(RuntimeValue::Owned(
                            ConstructedValue::Absent,
                        )))
                    }
                    None => Err(QueryError::query(range.clone(), "missing dictionary key").into()),
                }
                .map(Some);
            }
            PreparedKind::Tally { .. } if matches!(&values[0].data, Data::Native(view) if self.host.positional_view(native_view::<C,H>(view, self.host)).is_some()) =>
            {
                let source =
                    self.finish_value(values.pop().expect("tally input"), budget, &location)?;
                let source = self.host.positional_view(&source).expect("native sequence");
                crate::projection::admit_slots::<H::Value>(source.len(), budget, range)?;
                let mut items = Vec::with_capacity(source.len());
                for value in source {
                    items.push(self.host.copy_value(value, self.context, budget, range)?);
                }
                crate::tally_native_values(
                    items,
                    self.host,
                    self.context,
                    budget,
                    location.clone(),
                )?
            }
            _ => return Ok(None),
        };
        self.retained(std::sync::Arc::new(output), budget, range)
            .map(Some)
    }
    fn compute(
        &mut self,
        expression: &'a PreparedValue<'a, Query<C, H>>,
        mut values: Vec<HostValue<'a, C, H>>,
        budget: &mut Budget,
    ) -> Result<HostValue<'a, C, H>, EvaluationError<H::Error>> {
        if let Some(result) = self.compute_collection(expression, &mut values, budget)? {
            return Ok(result);
        }
        use crate::ComputationalValue as W;
        let location = Location::Query(expression.range.clone());
        let range = &expression.range;
        let output = match &expression.value {
            PreparedKind::TupleAccess { position, .. }
                if matches!(values[0].data, Data::Native(_)) =>
            {
                let mut value = values.pop().expect("native tuple");
                let len = match execution::Host::constructed(self, &value, budget, &location)? {
                    Some(data) => match data.value.data() {
                        ConstructedValue::Tuple(v) => Some(v.len()),
                        _ => None,
                    },
                    None => {
                        let Data::Native(view) = &value.data else {
                            unreachable!()
                        };
                        self.host
                            .tuple_view(native_view::<C, H>(view, self.host))
                            .map(<[_]>::len)
                    }
                }
                .ok_or_else(|| QueryError::query(range.clone(), "tuple access needs a tuple"))?;
                let index = position
                    .value
                    .checked_sub(1)
                    .filter(|&i| i < len)
                    .ok_or_else(|| {
                        QueryError::query(position.range.clone(), "missing tuple position")
                    })?;
                return crate::value::pattern::select(
                    &mut value,
                    NativeStep::Tuple(index),
                    self,
                    budget,
                    &location,
                );
            }
            PreparedKind::Field { field, .. }
                if matches!(&values[0].data, Data::Native(view)
                    if self.host.capture_record(native_view::<C, H>(view, self.host)).is_some()) =>
            {
                let mut value = values.pop().expect("native capture");
                let Data::Native(view) = &value.data else {
                    unreachable!("native capture view")
                };
                let record = self
                    .host
                    .capture_record(native_view::<C, H>(view, self.host))
                    .expect("native capture view");
                let mut selected = None;
                for (index, (label, _)) in record.fields.iter().enumerate() {
                    budget.charge(1 + label.len().min(field.value.len()), location.clone())?;
                    if label == &field.value {
                        selected = Some(index);
                        break;
                    }
                }
                let index = selected.ok_or_else(|| {
                    QueryError::query(field.range.clone(), "unknown native capture field")
                })?;
                return crate::value::pattern::select(
                    &mut value,
                    NativeStep::Record(index),
                    self,
                    budget,
                    &location,
                );
            }
            PreparedKind::Field { field, .. }
                if matches!(values[0].data, Data::Native(_))
                    && execution::Host::constructed(self, &values[0], budget, &location)?
                        .is_some() =>
            {
                let mut value = values.pop().expect("constructed record view");
                let data = execution::Host::constructed(self, &value, budget, &location)?
                    .expect("constructed view");
                let ConstructedValue::Record(fields) = data.value.data() else {
                    return Err(
                        QueryError::query(range.clone(), "field access needs a record").into(),
                    );
                };
                let mut selected = None;
                for (index, (label, _)) in fields.iter().enumerate() {
                    budget.charge(1 + label.len().min(field.value.len()), location.clone())?;
                    if label == &field.value {
                        selected = Some(index);
                        break;
                    }
                }
                let index = selected.ok_or_else(|| {
                    QueryError::query(field.range.clone(), "missing record field")
                })?;
                return crate::value::pattern::select(
                    &mut value,
                    NativeStep::Record(index),
                    self,
                    budget,
                    &location,
                );
            }
            PreparedKind::Queue { source, .. } => {
                let value = values.pop().expect("queue input");
                let element = match self
                    .worklists
                    .get(*source)
                    .expect("prepared queue contract")
                {
                    Some(DeclaredType::Queue(element)) => {
                        crate::runtime_types::copy_declaration(element, budget, &location)?
                    }
                    None => self.infer_queue_element(&value, budget, &location)?,
                    _ => unreachable!("queue constructor contract"),
                };
                let items = self.queue_items(value, &element, budget, &location)?;
                let q = crate::NativeQueue::new(
                    items,
                    element,
                    self.host,
                    self.context,
                    budget,
                    location.clone(),
                )?;
                self.computational(W::Queue(q), budget, &location)?
            }
            PreparedKind::IdentitySet { source } => {
                let Some(DeclaredType::IdentitySet(kind)) = self
                    .worklists
                    .get(*source)
                    .expect("prepared identity-set contract")
                else {
                    unreachable!()
                };
                let s = crate::NativeIdentitySet::new(
                    kind.clone(),
                    self.host,
                    self.context,
                    budget,
                    location.clone(),
                )?;
                self.computational(W::IdentitySet(s), budget, &location)?
            }
            PreparedKind::PopFront(_) => {
                let value = self.finish_value(values.pop().expect("queue"), budget, &location)?;
                let Some(W::Queue(queue)) = self.host.computational_value(&value) else {
                    return Err(QueryError::query(range.clone(), "pop_front needs a queue").into());
                };
                let element =
                    crate::runtime_types::copy_declaration(queue.element(), budget, &location)?;
                let queue = queue.share(self.host, self.context, budget, location.clone())?;
                match queue.pop_front(self.host, self.context, budget, location.clone())? {
                    None => {
                        return Ok(Value::constructed(RuntimeValue::Owned(
                            ConstructedValue::Absent,
                        )));
                    }
                    Some((item, remaining)) => {
                        let item = match std::sync::Arc::try_unwrap(item) {
                            Ok(v) => v,
                            Err(v) => self.host.copy_value(&v, self.context, budget, range)?,
                        };
                        let remaining =
                            self.computational(W::Queue(remaining), budget, &location)?;
                        crate::projection::admit_slots::<H::Value>(2, budget, range)?;
                        let pair = self.host.construct_tuple(
                            vec![item, remaining],
                            self.context,
                            budget,
                            range,
                        )?;
                        let expected = DeclaredType::Optional(Box::new(DeclaredType::Tuple(vec![
                            crate::runtime_types::copy_declaration(&element, budget, &location)?,
                            DeclaredType::Queue(Box::new(element)),
                        ])));
                        crate::admit_computational(
                            &pair,
                            &expected,
                            self.host,
                            self.context,
                            budget,
                            location.clone(),
                        )?;
                        pair
                    }
                }
            }
            PreparedKind::PushBackAll { .. } => {
                let items = values.pop().expect("appended items");
                let value = self.finish_value(values.pop().expect("queue"), budget, &location)?;
                let Some(W::Queue(queue)) = self.host.computational_value(&value) else {
                    return Err(
                        QueryError::query(range.clone(), "push_back_all needs a queue").into(),
                    );
                };
                let queue = queue.share(self.host, self.context, budget, location.clone())?;
                let items = self.queue_items(items, queue.element(), budget, &location)?;
                let queue = queue.push_back_all(
                    items,
                    self.host,
                    self.context,
                    budget,
                    location.clone(),
                )?;
                self.computational(W::Queue(queue), budget, &location)?
            }
            PreparedKind::Insert { .. } => {
                let item =
                    self.finish_value(values.pop().expect("inserted item"), budget, &location)?;
                let value = self.finish_value(values.pop().expect("set"), budget, &location)?;
                let Some(W::IdentitySet(set)) = self.host.computational_value(&value) else {
                    return Err(QueryError::query(
                        range.clone(),
                        "insert needs a native identity set",
                    )
                    .into());
                };
                let set = set
                    .share(self.host, self.context, budget, location.clone())?
                    .insert(item, self.host, self.context, budget, location.clone())?;
                self.computational(W::IdentitySet(set), budget, &location)?
            }
            PreparedKind::Binary {
                operator: crate::ValueBinaryOperator::In,
                ..
            } if matches!(values[1].data, Data::Native(_))
                && execution::Host::constructed(self, &values[1], budget, &location)?.is_none() =>
            {
                let set = self.finish_value(values.pop().expect("set"), budget, &location)?;
                let item = self.finish_value(values.pop().expect("member"), budget, &location)?;
                if self.host.set_view(&set).is_some() {
                    let found = crate::native_set_contains(
                        &set,
                        &item,
                        self.host,
                        self.context,
                        budget,
                        location.clone(),
                    )?;
                    return self.boolean_value(found, budget, &location);
                }
                let Some(W::IdentitySet(set)) = self.host.computational_value(&set) else {
                    return Err(QueryError::query(
                        range.clone(),
                        "native membership needs an identity set",
                    )
                    .into());
                };
                return Ok(Value::constructed(RuntimeValue::Owned({
                    let found =
                        set.contains(&item, self.host, self.context, budget, location.clone())?;
                    budget.charge(1, location.clone())?;
                    budget.charge_results(1, location.clone())?;
                    ConstructedValue::Boolean(found)
                })));
            }
            PreparedKind::Array(_) => {
                return execution::Host::finish_array(self, values, budget, &location);
            }
            PreparedKind::Record(fields) => {
                let mut native = false;
                for value in &values {
                    native |=
                        execution::Host::constructed(self, value, budget, &location)?.is_none();
                }
                if !native {
                    return execution::apply(expression, values, self, budget);
                }
                crate::projection::admit_slots::<(String, H::Value)>(values.len(), budget, range)?;
                let mut output = Vec::with_capacity(values.len());
                for ((name, _), value) in fields.iter().zip(values) {
                    budget_name(&name.value, budget, &name.range)?;
                    output.push((
                        name.value.clone(),
                        self.finish_value(value, budget, &location)?,
                    ));
                }
                self.host
                    .construct_record(output, self.context, budget, range)?
            }
            PreparedKind::Tuple(_) if values.iter().any(|v| matches!(v.data, Data::Native(_))) => {
                let mut native = false;
                for value in &values {
                    native |=
                        execution::Host::constructed(self, value, budget, &location)?.is_none();
                }
                if !native {
                    return execution::apply(expression, values, self, budget);
                }
                crate::projection::admit_slots::<H::Value>(values.len(), budget, range)?;
                let mut output = Vec::with_capacity(values.len());
                for value in values {
                    output.push(self.finish_value(value, budget, &location)?);
                }
                self.host
                    .construct_tuple(output, self.context, budget, range)?
            }
            _ => return execution::apply(expression, values, self, budget),
        };
        self.retained(std::sync::Arc::new(output), budget, range)
    }
    fn boolean_value(
        &self,
        value: bool,
        budget: &mut Budget,
        location: &Location,
    ) -> Result<HostValue<'a, C, H>, EvaluationError<H::Error>> {
        budget.charge(1, location.clone())?;
        budget.charge_results(1, location.clone())?;
        Ok(Value::constructed(RuntimeValue::Owned(
            ConstructedValue::Boolean(value),
        )))
    }
    fn copy_host_value(
        &mut self,
        value: &HostValue<'a, C, H>,
        budget: &mut Budget,
        location: &Location,
    ) -> Result<H::Value, EvaluationError<H::Error>> {
        let range = match location {
            Location::Query(r) | Location::Source(r) => r,
        };
        if let Some(data) = execution::Host::constructed(self, value, budget, location)? {
            let data = copy_constructed_value(data.value, budget, location.clone())?;
            let facts = vec![constructed_value_type(&data, budget, location.clone())?];
            return self
                .host
                .lift_constructed(data, facts, self.context, budget, range);
        }
        let Data::Native(view) = &value.data else {
            unreachable!("constructed value handled")
        };
        budget.charge(view.path.len(), location.clone())?;
        self.host.copy_value(
            native_view::<C, H>(view, self.host),
            self.context,
            budget,
            range,
        )
    }
    fn owned_native(
        &mut self,
        value: NativeView<H::Value>,
        budget: &mut Budget,
        range: &Range<usize>,
    ) -> Result<std::sync::Arc<H::Value>, EvaluationError<H::Error>> {
        let at = Location::Query(range.clone());
        let wrapped = Value {
            data: Data::Native(value),
            contract: None,
        };
        if execution::Host::constructed(self, &wrapped, budget, &at)?.is_some() {
            let value = execution::Host::materialize(self, wrapped, budget, &at)?;
            let types = vec![constructed_value_type(&value, budget, at)?];
            crate::projection::admit_slots::<H::Value>(1, budget, range)?;
            return Ok(std::sync::Arc::new(self.host.lift_constructed(
                value,
                types,
                self.context,
                budget,
                range,
            )?));
        }
        let Data::Native(value) = wrapped.data else {
            unreachable!()
        };
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
    fn retained<'v>(
        &mut self,
        value: std::sync::Arc<H::Value>,
        budget: &mut Budget,
        range: &Range<usize>,
    ) -> Result<HostValue<'v, C, H>, EvaluationError<H::Error>> {
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
        value: Value<'a, H::Value, Kind<C, H>>,
        budget: &mut Budget,
        location: &Location,
    ) -> Result<H::Value, EvaluationError<H::Error>> {
        let range = match location {
            Location::Query(r) | Location::Source(r) => r,
        };
        if matches!(value.data, Data::Native(_))
            && execution::Host::constructed(self, &value, budget, location)?.is_some()
        {
            let value = execution::Host::materialize(self, value, budget, location)?;
            let types = vec![constructed_value_type(&value, budget, location.clone())?];
            return self
                .host
                .lift_constructed(value, types, self.context, budget, range);
        }
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
        if let Some(data) = execution::Host::constructed(self, value, budget, location)? {
            constructed_value_type(data.value, budget, location.clone())?;
            if let Some(d) = data.declaration {
                crate::admit_constructed(data.value, d, budget, location.clone())?;
            }
            return Ok(());
        }
        if value.contract.is_none()
            && let Data::Native(view) = &value.data
        {
            budget.charge(view.path.len(), location.clone())?;
            let value = native_view::<C, H>(view, self.host);
            let declaration = self.value_contract(value, budget, location)?;
            admit_value(
                value,
                &declaration,
                self.host,
                self.context,
                budget,
                location.clone(),
            )?;
        }
        Ok(())
    }
    fn native_constructed<'v>(&self, value: &'v H::Value) -> Option<&'v ConstructedValue> {
        self.host.constructed_value(value)
    }
    fn native_record<'v>(&self, value: &'v H::Value) -> Option<&'v crate::NativeCapture<H::Value>> {
        self.host.capture_record(value)
    }
    fn native_tuple<'v>(&self, value: &'v H::Value) -> Option<&'v [H::Value]> {
        self.host.tuple_view(value)
    }
    fn native_list<'v>(&self, value: &'v H::Value) -> Option<&'v [H::Value]> {
        self.host.positional_view(value)
    }
    fn collection_equal(
        &mut self,
        left: &HostValue<'a, C, H>,
        right: &HostValue<'a, C, H>,
        budget: &mut Budget,
        location: &Location,
    ) -> Result<bool, Self::Error> {
        if let (Some(left), Some(right)) = (
            execution::Host::constructed(self, left, budget, location)?,
            execution::Host::constructed(self, right, budget, location)?,
        ) {
            return Ok(crate::value::key_equal(
                left.value,
                right.value,
                budget,
                location,
            )?);
        }
        let left = self.copy_host_value(left, budget, location)?;
        let right = self.copy_host_value(right, budget, location)?;
        crate::native_values_equal(
            &left,
            &right,
            self.host,
            self.context,
            budget,
            location.clone(),
        )
    }
    fn collection_hash(
        &mut self,
        value: &HostValue<'a, C, H>,
        budget: &mut Budget,
        location: &Location,
    ) -> Result<u64, Self::Error> {
        if let Some(data) = execution::Host::constructed(self, value, budget, location)? {
            return Ok(crate::value::identity::hash_admitted(
                data.value, budget, location,
            )?);
        }
        let value = self.copy_host_value(value, budget, location)?;
        crate::native_value_hash(&value, self.host, self.context, budget, location.clone())
    }
    fn finish_array(
        &mut self,
        values: Vec<HostValue<'a, C, H>>,
        budget: &mut Budget,
        location: &Location,
    ) -> Result<HostValue<'a, C, H>, Self::Error> {
        let mut native = false;
        for value in &values {
            native |= execution::Host::constructed(self, value, budget, location)?.is_none();
        }
        if !native {
            return execution::constructed_array(values, self, budget, location);
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
            .construct_list(output, self.context, budget, range)?;
        self.retained(std::sync::Arc::new(value), budget, range)
    }
    fn finish_record(
        &mut self,
        fields: Vec<(String, HostValue<'a, C, H>)>,
        budget: &mut Budget,
        location: &Location,
    ) -> Result<HostValue<'a, C, H>, Self::Error> {
        let mut native = false;
        for (_, value) in &fields {
            native |= execution::Host::constructed(self, value, budget, location)?.is_none();
        }
        if !native {
            return execution::constructed_record(fields, self, budget, location);
        }
        let range = match location {
            Location::Query(r) | Location::Source(r) => r,
        };
        crate::projection::admit_slots::<(String, H::Value)>(fields.len(), budget, range)?;
        let mut output = Vec::with_capacity(fields.len());
        for (label, value) in fields {
            output.push((label, self.finish_value(value, budget, location)?));
        }
        let value = self
            .host
            .construct_record(output, self.context, budget, range)?;
        self.retained(std::sync::Arc::new(value), budget, range)
    }
    fn finish_dictionary(
        &mut self,
        values: Vec<(HostValue<'a, C, H>, HostValue<'a, C, H>)>,
        budget: &mut Budget,
        location: &Location,
    ) -> Result<HostValue<'a, C, H>, Self::Error> {
        let mut native = false;
        for (key, value) in &values {
            native |= execution::Host::constructed(self, key, budget, location)?.is_none()
                || execution::Host::constructed(self, value, budget, location)?.is_none();
        }
        if !native {
            return execution::constructed_dictionary(values, self, budget, location);
        }
        let range = match location {
            Location::Query(r) | Location::Source(r) => r,
        };
        crate::projection::admit_slots::<(H::Value, H::Value)>(values.len(), budget, range)?;
        let mut output = Vec::with_capacity(values.len());
        for (key, value) in values {
            output.push((
                self.finish_value(key, budget, location)?,
                self.finish_value(value, budget, location)?,
            ));
        }
        let value = crate::construct_native_dictionary(
            output,
            self.host,
            self.context,
            budget,
            location.clone(),
        )?;
        self.retained(std::sync::Arc::new(value), budget, range)
    }
    fn finish_set(
        &mut self,
        values: Vec<HostValue<'a, C, H>>,
        budget: &mut Budget,
        location: &Location,
    ) -> Result<HostValue<'a, C, H>, Self::Error> {
        let mut native = false;
        for value in &values {
            native |= execution::Host::constructed(self, value, budget, location)?.is_none();
        }
        if !native {
            return execution::constructed_set(values, self, budget, location);
        }
        let range = match location {
            Location::Query(r) | Location::Source(r) => r,
        };
        crate::projection::admit_slots::<H::Value>(values.len(), budget, range)?;
        let mut output = Vec::with_capacity(values.len());
        for value in values {
            output.push(self.finish_value(value, budget, location)?);
        }
        let value =
            crate::construct_native_set(output, self.host, self.context, budget, location.clone())?;
        self.retained(std::sync::Arc::new(value), budget, range)
    }
    fn check_native_context(
        &mut self,
        value: &H::Value,
        expected: &DeclaredType<Kind<C, H>>,
        budget: &mut Budget,
        location: &Location,
    ) -> Result<(), Self::Error> {
        crate::worklists::check_context(value, expected, self.host, self.context, budget, location)
    }
    fn apply(
        &mut self,
        expression: &'a PreparedValue<'a, Query<C, H>>,
        values: Vec<HostValue<'a, C, H>>,
        budget: &mut Budget,
    ) -> Result<HostValue<'a, C, H>, Self::Error> {
        self.compute(expression, values, budget)
    }
    fn retains_native_admission(&self) -> bool {
        self.host.retains_native_admission()
    }
    fn predicate_subject(
        &mut self,
        budget: &mut Budget,
        range: &Range<usize>,
    ) -> Result<HostValue<'a, C, H>, Self::Error> {
        self.outer(0, crate::value::PREDICATE_SUBJECT, budget, range)
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
        let worklists = self.worklists;
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
                    worklists,
                    source_function,
                    bodies,
                    arguments,
                    declared,
                    argument_kinds: &operand.argument_kinds,
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
        if crate::admit_computational(
            value,
            expected,
            self.host,
            self.context,
            budget,
            location.clone(),
        )? {
            return Ok(());
        }
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
                values, element, self, budget, location,
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
    fn native_contract(
        &mut self,
        value: &H::Value,
        budget: &mut Budget,
        location: &Location,
    ) -> Result<DeclaredType<Kind<C, H>>, Self::Error> {
        self.value_contract(value, budget, location)
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
        if let Some(values) = self.host.set_view(&value) {
            crate::projection::admit_slots::<H::Value>(values.len(), budget, range)?;
            let mut output = Vec::with_capacity(values.len());
            for value in values {
                output.push(self.host.copy_value(value, self.context, budget, range)?);
            }
            return Ok(output);
        }
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
    value.native(
        |v| host.capture_record(v),
        |v| host.tuple_view(v),
        |v| host.positional_view(v),
    )
}

struct Arguments<'r, 'a, C, H: FunctionHost<C>> {
    host: &'r H,
    context: &'r mut C,
    bindings: &'r Bindings<H::Value>,
    contracts: &'r BTreeMap<&'a str, &'a DeclaredType<Kind<C, H>>>,
    prepared: &'r BTreeMap<usize, Operand<Kind<C, H>>>,
    body_prepared: &'r [BTreeMap<usize, Operand<Kind<C, H>>>],
    worklists: &'r [Option<DeclaredType<Kind<C, H>>>],
    source_function: Option<usize>,
    bodies: &'a Bodies<'a, C, H>,
    arguments: &'a [PreparedArgument<'a, Query<C, H>>],
    declared: &'r [usize],
    argument_kinds: &'r BTreeMap<usize, Kind<C, H>>,
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
            worklists: self.worklists,
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
            Data::Native(view) => {
                let data = view
                    .constructed(
                        |v| self.host.constructed_value(v),
                        |v| self.host.capture_record(v),
                        |v| self.host.tuple_view(v),
                        |v| self.host.positional_view(v),
                        budget,
                        &location,
                    )?
                    .ok_or_else(|| {
                        QueryError::query(
                            expression.range.clone(),
                            "native argument requires an explicit scalar projection",
                        )
                    })?;
                RuntimeValue::Owned(data.materialize(budget, &location)?)
            }
        };
        Ok(ConstructedRead::from_runtime(value, location, declaration))
    }
    fn evaluate_predicate(
        &mut self,
        expression: &ValueExpression<Query<C, H>>,
        input: H::Value,
        captures: Option<&Bindings<H::Value>>,
        budget: &mut Budget,
    ) -> Result<bool, EvaluationError<H::Error>> {
        let value = self.run(expression, &input, captures, budget)?;
        let Data::Constructed(value) = value.data else {
            return Err(QueryError::query(
                expression.range.clone(),
                "predicate needs a fixed Boolean",
            )
            .into());
        };
        crate::value::boolean(
            value.materialize(budget, &Location::Query(expression.range.clone()))?,
            &Location::Query(expression.range.clone()),
        )
        .map_err(Into::into)
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
    fn evaluate_projection(
        &mut self,
        projection: &crate::Projection<Query<C, H>>,
        input: H::Value,
        budget: &mut Budget,
    ) -> Result<crate::ConstructedResult, EvaluationError<H::Error>> {
        let range = &projection.subject.range;
        budget.within_evaluation(1, Location::Query(range.clone()), |budget| {
            if self
                .bindings
                .iter()
                .any(|(name, _)| name == projection.subject.value)
            {
                return Err(QueryError::query(
                    range.clone(),
                    "projection subject shadows an existing binding",
                )
                .into());
            }
            crate::projection::admit_slots::<Vec<ValueType>>(1, budget, range)?;
            let mut positions = vec![Vec::new()];
            for (index, (_, expression)) in projection.fields.iter().enumerate() {
                crate::projection::check_projection_label(projection, index, budget)?;
                let id = expression as *const _ as usize;
                budget.charge(
                    self.declared.len() + self.arguments.len() + 1,
                    Location::Query(expression.range.clone()),
                )?;
                if !self.declared.contains(&id)
                    || !self
                        .arguments
                        .iter()
                        .any(|argument| std::ptr::eq(argument.source, expression))
                {
                    return Err(QueryError::query(
                        expression.range.clone(),
                        "projection requires every written prepared field",
                    )
                    .into());
                }
                let kind = self.argument_kinds.get(&id).ok_or_else(|| {
                    QueryError::query(
                        expression.range.clone(),
                        "projection field kind was not prepared",
                    )
                })?;
                let types = self
                    .host
                    .scalar_types(kind, budget, &expression.range)?
                    .ok_or_else(|| {
                        QueryError::query(
                            expression.range.clone(),
                            "native handles require an explicit scalar projection",
                        )
                    })?;
                positions = crate::value::extend_type_positions(
                    &positions,
                    &types,
                    budget,
                    &Location::Query(expression.range.clone()),
                )?;
            }
            let types = crate::projection::projection_result_types(projection, positions, budget)?;
            let bindings = self
                .bindings
                .with_shared_values(BTreeMap::new(), budget, range)?;
            let subjects = self.host.subjects(input, self.context, budget, range)?;
            let value = crate::projection::projection_rows(
                projection,
                subjects,
                &bindings,
                self,
                budget,
                |index, local, runner, budget| {
                    let expression = &projection.fields[index].1;
                    let relative = local.get(&projection.subject.value, range.clone())?;
                    let value = runner.run(expression, relative, Some(local), budget)?;
                    let read = runner.native_read(value, budget, &expression.range)?;
                    let value = match std::sync::Arc::try_unwrap(read.into_shared()) {
                        Ok(value) => value,
                        Err(value) => runner.host.copy_value(
                            &value,
                            runner.context,
                            budget,
                            &expression.range,
                        )?,
                    };
                    runner
                        .host
                        .project_value(value, runner.context, budget, &expression.range)
                },
            )?;
            Ok(crate::ConstructedResult { value, types })
        })
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
                if let Some(data) = owner.constructed(
                    |v| self.host.constructed_value(v),
                    |v| self.host.capture_record(v),
                    |v| self.host.tuple_view(v),
                    |v| self.host.positional_view(v),
                    budget,
                    &location,
                )? {
                    let value = data.materialize(budget, &location)?;
                    let types = vec![constructed_value_type(&value, budget, location.clone())?];
                    crate::projection::admit_slots::<H::Value>(1, budget, range)?;
                    std::sync::Arc::new(self.host.lift_constructed(
                        value,
                        types,
                        self.context,
                        budget,
                        range,
                    )?)
                } else if owner.path.is_empty() {
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

pub(crate) fn dispose_declaration<K>(declaration: DeclaredType<K>) {
    let mut pending = vec![declaration];
    while let Some(declaration) = pending.pop() {
        match declaration {
            DeclaredType::Optional(child)
            | DeclaredType::Array(child)
            | DeclaredType::Sequence(child)
            | DeclaredType::Keyed(child)
            | DeclaredType::Queue(child) => pending.push(*child),
            DeclaredType::Dictionary(key, value) => pending.extend([*key, *value]),
            DeclaredType::Tuple(children) => pending.extend(children),
            DeclaredType::Record(fields) => {
                pending.extend(fields.into_iter().map(|(_, value)| value))
            }
            DeclaredType::Value
            | DeclaredType::Boolean
            | DeclaredType::Integer
            | DeclaredType::Float
            | DeclaredType::String
            | DeclaredType::Member(_)
            | DeclaredType::Set(_)
            | DeclaredType::Native(_)
            | DeclaredType::IdentitySet(_) => {}
        }
    }
}
