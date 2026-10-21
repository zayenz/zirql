//! Common programs over admitted, immutable constructed data.
use std::{collections::BTreeMap, ops::Range, sync::Arc};

use crate::{
    ArgumentReader, Bindings, Budget, ConstructedValue, DeclaredType, EvaluationError, Expression,
    ExpressionKind, ExpressionStep, FieldPipelineSource, FunctionError, FunctionHost,
    FunctionLimits, Located, Location, NativeOperands, ParameterDeclaration, ParameterInputs,
    PreparedFunctions, PreparedProgram, Program, ProgramExpression, ProgramHost, ProgramReaderHost,
    ProjectionHost, QueryError, QueryErrorCategory, ReadExpression, ReadStep,
    ResolvedPatternFunctionSignature, RuntimeTypeHost, Statement,
    UnresolvedPatternFunctionDeclaration, ValueReaderHost, ValueType, admit_parameters,
    copy_constructed_value, copy_value_types, declare_constructed, dispose_expression,
    dispose_native_value, dispose_pattern, dispose_unresolved_pattern_functions,
    is_common_operation_name, read_program, read_value_expression,
};

/// A value and its admitted declaration, shared by immutable name/input bindings.
/// Producers charge their allocations before calling [`Self::new`]. Admission
/// validates actual data and derives compact preparation facts from the schema.
#[derive(Clone, Debug)]
pub struct ConstructedBinding {
    value: Arc<ConstructedValue>,
    types: Arc<Vec<ValueType>>,
}
impl ConstructedBinding {
    /// Admit actual data, preserving declared empty schemas and positional meaning.
    ///
    /// # Errors
    /// Invalid data, incompatible declarations and limits retain `location`.
    pub fn new(
        value: ConstructedValue,
        declaration: DeclaredType<()>,
        budget: &mut Budget,
        location: Location,
    ) -> Result<Self, QueryError> {
        let value = declare_constructed(value, declaration, budget, location.clone())?;
        let types = crate::value::function_facts(
            crate::value::declaration(&value).expect("declared above"),
            budget,
            &location,
        )?;
        budget.charge(2, location.clone())?;
        budget.charge_results(2, location)?;
        Ok(Self {
            value: Arc::new(value),
            types: Arc::new(types),
        })
    }

    /// Borrow the admitted value, including its transparent declaration wrapper.
    pub fn value(&self) -> &ConstructedValue {
        &self.value
    }
}

/// A parsed common program with functions, parameters, ordered lets and one final
/// value. Native handles and domain operations use the native embedding interfaces.
/// This development API is newer than registry version 0.2.0.
///
/// ```
/// use zirql::{Budget, ConstructedBinding, ConstructedProgram, ConstructedValue as V,
///             DeclaredType as D, FunctionLimits, Location, ParameterInputs, ProgramLimits};
/// let mut budget = Budget::for_program(ProgramLimits {
///     work: 100_000, results: 10_000, nesting: 64, stream_items: 100,
/// });
/// let rows = ConstructedBinding::new(V::Array(vec![V::Integer(3), V::Integer(5)]),
///     D::Sequence(Box::new(D::Integer)), &mut budget, Location::Source(0..0))?;
/// let program = ConstructedProgram::read(
///     "param minimum: int; fn above(xs: sequence<int>, n: int) -> sequence<int> { [x for x in xs if x > n] } above(rows, minimum)",
///     &["rows"], &mut budget, 64)?;
/// let result = program.evaluate(&[("rows", &rows)], Some(&rows),
///     ParameterInputs::Cli(&["minimum=3".into()]), FunctionLimits { active_depth: 16 }, &mut budget)?;
/// assert!(matches!(result.data(), V::Array(values) if values.len() == 1));
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub struct ConstructedProgram {
    functions: Vec<UnresolvedPatternFunctionDeclaration<Expression<NoNative, NoNative, ()>>>,
    parameters: Vec<ParameterDeclaration>,
    program: Program<NoNative, NoNative, ()>,
}
impl ConstructedProgram {
    /// Read the shared function/parameter/statement grammar. `initial_names`
    /// reserves supplied binding names; values enter only during evaluation.
    ///
    /// # Errors
    /// Syntax, lexical scope, final-result policy and limits are located.
    pub fn read(
        source: &str,
        initial_names: &[&str],
        budget: &mut Budget,
        nesting: usize,
    ) -> Result<Self, QueryError> {
        let parsed = read_program(
            source,
            initial_names,
            &mut Reader { names: &[] },
            budget,
            nesting,
        )?;
        let mut result = Self {
            functions: parsed.functions,
            parameters: Vec::new(),
            program: Program::new(parsed.statements, parsed.range),
        };
        crate::projection::admit_slots::<ParameterDeclaration>(
            parsed.parameters.len(),
            budget,
            &result.program.range(),
        )?;
        result
            .parameters
            .extend(parsed.parameters.into_iter().map(|p| p.declaration));
        let statements = result.program.statements();
        if !matches!(
            statements.last().map(|s| &s.value),
            Some(Statement::Evaluate { .. })
        ) {
            return Err(QueryError::query(
                result.program.range(),
                "expected a final value expression",
            ));
        }
        if source[statements
            .last()
            .expect("final value checked above")
            .range
            .clone()]
        .trim_end()
        .ends_with(';')
        {
            return Err(QueryError::query(
                result.program.range(),
                "expected a final value without a trailing separator",
            ));
        }
        if let Some(statement) = statements[..statements.len() - 1]
            .iter()
            .find(|s| matches!(s.value, Statement::Evaluate { .. }))
        {
            return Err(QueryError::query(
                statement.range.clone(),
                "expected one final value expression",
            ));
        }
        Ok(result)
    }

    /// Prepare and execute with one continuing budget. A function sees only its
    /// explicit arguments, never these bindings, parameters or current input.
    /// Rendering is a separate operation after successful evaluation.
    ///
    /// # Errors
    /// Admission, preparation and execution failures retain locations and call frames.
    pub fn evaluate(
        &self,
        named: &[(&str, &ConstructedBinding)],
        current: Option<&ConstructedBinding>,
        parameters: ParameterInputs<'_>,
        limits: FunctionLimits,
        budget: &mut Budget,
    ) -> Result<ConstructedValue, FunctionError<QueryError>> {
        self.evaluate_prepared(named, current, parameters, limits, budget)
            .map_err(|error| FunctionError {
                error: match error.error {
                    EvaluationError::Core(error) | EvaluationError::Host(error) => error,
                },
                calls: error.calls,
                omitted_calls: error.omitted_calls,
            })
    }

    fn evaluate_prepared(
        &self,
        named: &[(&str, &ConstructedBinding)],
        current: Option<&ConstructedBinding>,
        parameters: ParameterInputs<'_>,
        limits: FunctionLimits,
        budget: &mut Budget,
    ) -> Result<ConstructedValue, FunctionError<EvaluationError<QueryError>>> {
        let range = self.program.range();
        crate::projection::admit_slots::<&str>(named.len(), budget, &range)?;
        let names = named.iter().map(|(name, _)| *name).collect::<Vec<_>>();
        let parameters =
            admit_parameters(&self.parameters, parameters, &names, range.clone(), budget)?;
        let mut scope = BTreeMap::new();
        let mut bindings = Vec::new();
        crate::projection::admit_slots::<(Located<String>, Arc<ConstructedValue>)>(
            named.len(),
            budget,
            &range,
        )?;
        for (name, binding) in named {
            budget.charge(name.len() * 2 + 2, Location::Query(range.clone()))?;
            scope.insert((*name).to_owned(), Arc::clone(&binding.types));
            bindings.push((
                Located::new((*name).to_owned(), range.clone()),
                Arc::clone(&binding.value),
            ));
        }
        let bindings = Bindings::new(bindings)?;
        let relative = current.map(|binding| Arc::clone(&binding.types));
        let mut headers = Vec::new();
        crate::projection::admit_slots::<ResolvedPatternFunctionSignature<'_, Arc<Vec<ValueType>>>>(
            self.functions.len(),
            budget,
            &range,
        )?;
        for function in &self.functions {
            headers.push(function.resolve_signature(budget, |name, _| {
                Err(QueryError::query(
                    name.range,
                    "named types are unavailable in constructed programs",
                ))
            })?);
        }
        let host = ConstructedHost;
        let functions = PreparedFunctions::prepare_patterns_borrowed(
            self.functions
                .iter()
                .zip(&headers)
                .map(|(function, header)| function.with_signature(header)),
            &host,
            budget,
        )?;
        let prepared =
            PreparedProgram::prepare(functions, &self.program, &scope, relative, limits, budget)?;
        crate::projection::admit_slots::<(Located<String>, Arc<ConstructedValue>)>(
            parameters.len(),
            budget,
            &range,
        )?;
        let parameters = parameters
            .into_iter()
            .map(|(name, value)| (name, Arc::new(value)))
            .collect();
        let mut result = None;
        prepared.evaluate(
            bindings,
            parameters,
            current.map(|binding| Arc::clone(&binding.value)),
            &mut result,
            budget,
        )?;
        let result =
            result.ok_or_else(|| QueryError::query(range.clone(), "expected a final value"))?;
        Ok(into_owned(result, budget, &range)?)
    }
}
impl Drop for ConstructedProgram {
    fn drop(&mut self) {
        dispose_unresolved_pattern_functions(std::mem::take(&mut self.functions));
        let program = std::mem::replace(&mut self.program, Program::new(Vec::new(), 0..0));
        for statement in program.into_statements() {
            match statement.value {
                Statement::Parameter { .. } => {}
                Statement::Bind { expression, .. }
                | Statement::BindFields { expression, .. }
                | Statement::Evaluate { expression, .. } => dispose_expression(expression),
                Statement::BindPattern { pattern, value } => {
                    if let Ok(pattern) = Arc::try_unwrap(pattern) {
                        dispose_pattern(pattern);
                    }
                    if let Ok(value) = Arc::try_unwrap(value) {
                        dispose_native_value(value);
                    }
                }
            }
        }
    }
}

#[derive(Debug, PartialEq)]
enum NoNative {}
struct Reader<'a> {
    names: &'a [&'a str],
}
impl ValueReaderHost for Reader<'_> {
    type Stage = NoNative;
    type SetOperator = NoNative;
    type Emitter = ();
    fn is_binding(&self, name: &str) -> bool {
        self.names.contains(&name)
    }
    fn is_constructed_binding(&self, name: &str) -> bool {
        self.is_binding(name)
    }
    fn is_stage(&self, _: &str) -> bool {
        false
    }
    fn field_pipeline_source(&self) -> FieldPipelineSource {
        FieldPipelineSource::Constructed
    }
    fn parse_stage(
        &mut self,
        reader: &mut ArgumentReader<'_>,
        _: &mut Budget,
    ) -> Result<ReadStep<Self>, QueryError> {
        Err(reader.error("unknown binding or unavailable native stage"))
    }
    fn parse_predicate_leaf(
        &mut self,
        reader: &mut ArgumentReader<'_>,
        _: &mut Budget,
    ) -> Result<NoNative, QueryError> {
        Err(reader.error("expected a constructed predicate"))
    }
    fn set_operator(&self, _: &str) -> Option<NoNative> {
        None
    }
}
impl ProgramReaderHost for Reader<'_> {
    fn read_program_expression(
        &mut self,
        reader: &mut ArgumentReader<'_>,
        names: &[&str],
        purpose: ProgramExpression,
        budget: &mut Budget,
        nesting: usize,
    ) -> Result<ReadExpression<Self>, QueryError> {
        if purpose == ProgramExpression::Report {
            return Err(reader.error("use one final value expression"));
        }
        let value = read_value_expression(reader, &mut Reader { names }, budget, nesting)?;
        let range = value.range.clone();
        crate::projection::admit_slots::<Located<ExpressionStep<NoNative, (), NoNative>>>(
            1, budget, &range,
        )?;
        Ok(Located::new(
            ExpressionKind::Sequence {
                input: Box::new(Located::new(ExpressionKind::Input, range.clone())),
                steps: vec![Located::new(
                    ExpressionStep::Value(value.into()),
                    range.clone(),
                )],
            },
            range,
        ))
    }
    fn implicit_output(&mut self, _: ProgramExpression) -> Option<()> {
        Some(())
    }
}

type ResultSlot = Option<Arc<ConstructedValue>>;
struct ConstructedHost;
fn into_owned(
    value: Arc<ConstructedValue>,
    budget: &mut Budget,
    range: &Range<usize>,
) -> Result<ConstructedValue, QueryError> {
    match Arc::try_unwrap(value) {
        Ok(value) => Ok(value),
        Err(value) => copy_constructed_value(&value, budget, Location::Query(range.clone())),
    }
}
impl ProgramHost<ResultSlot> for ConstructedHost {
    type Value = Arc<ConstructedValue>;
    type Stage = NoNative;
    type SetOperator = NoNative;
    type Emitter = ();
    type Error = QueryError;
    fn copy_value(
        &self,
        value: &Self::Value,
        _: &mut ResultSlot,
        budget: &mut Budget,
        range: &Range<usize>,
    ) -> Result<Self::Value, EvaluationError<QueryError>> {
        budget.charge(1, Location::Query(range.clone()))?;
        Ok(Arc::clone(value))
    }
    fn evaluate_stage(
        &self,
        stage: &NoNative,
        _: Self::Value,
        _: &mut ResultSlot,
        _: &Bindings<Self::Value>,
        _: &mut Budget,
        _: &Range<usize>,
    ) -> Result<Self::Value, EvaluationError<QueryError>> {
        match *stage {}
    }
    fn combine(
        &self,
        operator: &NoNative,
        _: Self::Value,
        _: Self::Value,
        _: &mut ResultSlot,
        _: &mut Budget,
        _: &Range<usize>,
    ) -> Result<Self::Value, EvaluationError<QueryError>> {
        match *operator {}
    }
    fn construct_record(
        &self,
        fields: Vec<(String, Self::Value)>,
        _: &mut ResultSlot,
        budget: &mut Budget,
        range: &Range<usize>,
    ) -> Result<Self::Value, EvaluationError<QueryError>> {
        let mut output = Vec::new();
        crate::projection::admit_slots::<(String, ConstructedValue)>(fields.len(), budget, range)?;
        for (name, value) in fields {
            output.push((name, into_owned(value, budget, range)?));
        }
        Ok(Arc::new(ConstructedValue::Record(output)))
    }
    fn emit(
        &self,
        _: &(),
        value: &Self::Value,
        result: &mut ResultSlot,
        budget: &mut Budget,
        range: &Range<usize>,
    ) -> Result<(), EvaluationError<QueryError>> {
        budget.charge(1, Location::Query(range.clone()))?;
        *result = Some(Arc::clone(value));
        Ok(())
    }
}
impl ProjectionHost<ResultSlot> for ConstructedHost {
    type Kind = Arc<Vec<ValueType>>;
    fn kind(
        &self,
        value: &Self::Value,
        budget: &mut Budget,
        range: &Range<usize>,
    ) -> Result<Self::Kind, QueryError> {
        Ok(Arc::new(crate::value::admitted_constructed_facts(
            value,
            budget,
            &Location::Query(range.clone()),
        )?))
    }
    fn scalar_types(
        &self,
        kind: &Self::Kind,
        budget: &mut Budget,
        range: &Range<usize>,
    ) -> Result<Option<Vec<ValueType>>, QueryError> {
        Ok(Some(copy_value_types(
            kind,
            budget,
            Location::Query(range.clone()),
        )?))
    }
    fn constructed_kind(&self, types: Vec<ValueType>) -> Self::Kind {
        Arc::new(types)
    }
    fn is_constructed_kind(&self, _: &Self::Kind) -> bool {
        true
    }
    fn stage_kind(
        &self,
        stage: &NoNative,
        _: &Self::Kind,
        _: &BTreeMap<String, Self::Kind>,
        _: &mut Budget,
        _: &Range<usize>,
    ) -> Result<Self::Kind, QueryError> {
        match *stage {}
    }
    fn is_pure(&self, stage: &NoNative) -> bool {
        match *stage {}
    }
    fn nested_queries(
        &self,
        stage: &NoNative,
        _: &mut Budget,
        _: &Range<usize>,
    ) -> Result<NativeOperands<ResultSlot, Self>, QueryError> {
        match *stage {}
    }
    fn subjects(
        &self,
        _: Self::Value,
        _: &mut ResultSlot,
        _: &mut Budget,
        range: &Range<usize>,
    ) -> Result<Vec<Self::Value>, EvaluationError<QueryError>> {
        Err(QueryError::query(range.clone(), "native subjects are unavailable").into())
    }
    fn project_value(
        &self,
        value: Self::Value,
        _: &mut ResultSlot,
        budget: &mut Budget,
        range: &Range<usize>,
    ) -> Result<ConstructedValue, EvaluationError<QueryError>> {
        Ok(into_owned(value, budget, range)?)
    }
    fn share_constructed(
        &self,
        value: &Self::Value,
        _: &mut ResultSlot,
        budget: &mut Budget,
        range: &Range<usize>,
    ) -> Result<Option<Arc<ConstructedValue>>, EvaluationError<QueryError>> {
        budget.charge(1, Location::Query(range.clone()))?;
        Ok(Some(Arc::clone(value)))
    }
    fn lift_shared_constructed(
        &self,
        value: Arc<ConstructedValue>,
        _: Vec<ValueType>,
        _: &mut ResultSlot,
        _: &mut Budget,
        _: &Range<usize>,
    ) -> Result<Self::Value, EvaluationError<QueryError>> {
        Ok(value)
    }
    fn lift_constructed(
        &self,
        value: ConstructedValue,
        _: Vec<ValueType>,
        _: &mut ResultSlot,
        budget: &mut Budget,
        range: &Range<usize>,
    ) -> Result<Self::Value, EvaluationError<QueryError>> {
        budget.charge_results(1, Location::Query(range.clone()))?;
        Ok(Arc::new(value))
    }
}
impl RuntimeTypeHost<ResultSlot> for ConstructedHost {
    type Kind = Arc<Vec<ValueType>>;
    fn constructed_value<'v>(&self, value: &'v Self::Value) -> Option<&'v ConstructedValue> {
        Some(value)
    }
    fn admit_native(
        &self,
        _: &Self::Value,
        _: &DeclaredType<Self::Kind>,
        _: &mut ResultSlot,
        _: &mut Budget,
        range: Location,
    ) -> Result<(), EvaluationError<QueryError>> {
        Err(QueryError {
            location: range,
            message: "native values are unavailable".into(),
            category: QueryErrorCategory::Other,
        }
        .into())
    }
}
impl FunctionHost<ResultSlot> for ConstructedHost {
    fn declaration_kind(
        &self,
        declaration: &DeclaredType<Arc<Vec<ValueType>>>,
        budget: &mut Budget,
        range: &Range<usize>,
    ) -> Result<Arc<Vec<ValueType>>, QueryError> {
        Ok(Arc::new(crate::value::function_facts(
            declaration,
            budget,
            &Location::Query(range.clone()),
        )?))
    }
    fn is_operation_name(&self, name: &str) -> bool {
        is_common_operation_name(name)
    }
}
