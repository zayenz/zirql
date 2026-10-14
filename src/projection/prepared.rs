use super::*;

pub(crate) enum NativeRead {
    Outer(usize),
    Subject,
    Local(usize),
}

pub(crate) enum NativeOuter<'a, V> {
    Slots(&'a [&'a V]),
    Bindings {
        values: &'a Bindings<V>,
        names: &'a [String],
    },
}
impl<V> NativeOuter<'_, V> {
    fn get(&self, index: usize, range: &Range<usize>) -> Result<&V, QueryError> {
        match self {
            Self::Slots(values) => Ok(values[index]),
            Self::Bindings { values, names } => values.get(&names[index], range.clone()),
        }
    }
}

pub(crate) enum PreparedNative {
    Input,
    Binding(NativeRead),
    Group(Box<Self>),
    Field(Box<Self>),
    Sequence {
        input: Box<Self>,
        arguments: Vec<Vec<usize>>,
    },
    Set {
        left: Box<Self>,
        rights: Vec<Self>,
    },
}

impl PreparedNative {
    pub fn resolve<C, H: ProjectionHost<C>>(
        expression: &NativeExpression<C, H>,
        host: &H,
        outer: &[&str],
        subject: Option<&str>,
        locals: &[(&str, Vec<ValueType>)],
        budget: &mut Budget,
    ) -> Result<Self, QueryError> {
        budget.within_evaluation(
            expression.value.depth(),
            Location::Query(expression.range.clone()),
            |budget| {
                budget.charge(1, Location::Query(expression.range.clone()))?;
                Ok(match &expression.value {
                    ExpressionKind::Input => Self::Input,
                    ExpressionKind::Binding(name) => {
                        let work = outer
                            .iter()
                            .copied()
                            .chain(locals.iter().map(|(name, _)| *name))
                            .try_fold(0usize, |work, local| {
                                work.checked_add(1 + local.len().min(name.len()))
                                    .ok_or_else(|| {
                                        QueryError::query(
                                            expression.range.clone(),
                                            "native resolution metadata overflow",
                                        )
                                    })
                            })?;
                        budget.charge(work, Location::Query(expression.range.clone()))?;
                        Self::Binding(
                            if let Some(index) = locals.iter().position(|(local, _)| *local == name)
                            {
                                NativeRead::Local(index)
                            } else if subject == Some(name.as_str()) {
                                NativeRead::Subject
                            } else {
                                NativeRead::Outer(
                                    outer.iter().position(|outer| *outer == name).ok_or_else(
                                        || {
                                            QueryError::query(
                                                expression.range.clone(),
                                                format!("undefined binding: {name}"),
                                            )
                                        },
                                    )?,
                                )
                            },
                        )
                    }
                    ExpressionKind::Group(value) => Self::Group(Box::new(Self::resolve::<C, H>(
                        value, host, outer, subject, locals, budget,
                    )?)),
                    ExpressionKind::Field { value, .. } => {
                        Self::Field(Box::new(Self::resolve::<C, H>(
                            value, host, outer, subject, locals, budget,
                        )?))
                    }
                    ExpressionKind::Sequence { input, steps } => {
                        admit_slots::<Vec<usize>>(steps.len(), budget, &expression.range)?;
                        let mut arguments = Vec::with_capacity(steps.len());
                        for step in steps {
                            let ExpressionStep::Stage(stage) = &step.value else {
                                unreachable!("pure native stage preflight")
                            };
                            let declared = host.value_arguments(stage, budget, &step.range)?;
                            admit_slots::<usize>(declared.len(), budget, &step.range)?;
                            arguments.push(
                                declared
                                    .into_iter()
                                    .map(|(expression, _)| expression as *const _ as usize)
                                    .collect(),
                            );
                        }
                        Self::Sequence {
                            input: Box::new(Self::resolve::<C, H>(
                                input, host, outer, subject, locals, budget,
                            )?),
                            arguments,
                        }
                    }
                    ExpressionKind::Set { .. } => {
                        let mut rights = Vec::new();
                        let mut operand = expression;
                        while let ExpressionKind::Set { left, right, .. } = &operand.value {
                            admit_slots::<Self>(1, budget, &operand.range)?;
                            rights.push(Self::resolve::<C, H>(
                                right, host, outer, subject, locals, budget,
                            )?);
                            operand = left;
                        }
                        Self::Set {
                            left: Box::new(Self::resolve::<C, H>(
                                operand, host, outer, subject, locals, budget,
                            )?),
                            rights,
                        }
                    }
                    ExpressionKind::Record(_) => {
                        return Err(QueryError::query(
                            expression.range.clone(),
                            "native records require explicit scalar field projections",
                        ));
                    }
                })
            },
        )
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "Prepared native operands retain explicit context and lexical slots"
    )]
    pub fn run<C, H: ProjectionHost<C>>(
        &self,
        expression: &NativeExpression<C, H>,
        host: &H,
        input: H::Value,
        context: &mut C,
        bindings: &Bindings<H::Value>,
        outer: &[&H::Value],
        subject: &H::Value,
        locals: &[&H::Value],
        arguments: &mut dyn crate::ValueArguments<NativeExpression<C, H>, EvaluationError<H::Error>>,
        native: &BTreeMap<usize, Self>,
        scope: &BTreeMap<String, H::Kind>,
        budget: &mut Budget,
    ) -> Result<H::Value, EvaluationError<H::Error>> {
        self.execute(
            expression,
            host,
            Some(input),
            context,
            bindings,
            &NativeOuter::Slots(outer),
            Some(subject),
            locals,
            &mut |stage, declared, value, context, bindings, budget, range| {
                let mut arguments = StageArguments {
                    host,
                    context,
                    bindings,
                    scope,
                    prepared: native,
                    declared,
                    values: arguments,
                };
                host.evaluate_stage_with_arguments(stage, value, &mut arguments, budget, range)
            },
            budget,
        )
    }
    #[expect(
        clippy::too_many_arguments,
        reason = "One native traversal serves ordinary projections and function callbacks"
    )]
    pub(crate) fn execute<C, H: ProjectionHost<C>>(
        &self,
        expression: &NativeExpression<C, H>,
        host: &H,
        input: Option<H::Value>,
        context: &mut C,
        bindings: &Bindings<H::Value>,
        outer: &NativeOuter<'_, H::Value>,
        subject: Option<&H::Value>,
        locals: &[&H::Value],
        run_stage: &mut impl FnMut(
            &H::Stage,
            &[usize],
            H::Value,
            &mut C,
            &Bindings<H::Value>,
            &mut Budget,
            &Range<usize>,
        ) -> Result<H::Value, EvaluationError<H::Error>>,
        budget: &mut Budget,
    ) -> Result<H::Value, EvaluationError<H::Error>> {
        budget.within_evaluation(
            expression.value.depth(),
            Location::Query(expression.range.clone()),
            |budget| {
                budget.charge_program_work(1, Location::Query(expression.range.clone()))?;
                match (self, &expression.value) {
                    (Self::Input, ExpressionKind::Input) => input.ok_or_else(|| {
                        QueryError::query(
                            expression.range.clone(),
                            "native input requires an explicit argument input",
                        )
                        .into()
                    }),
                    (Self::Binding(read), ExpressionKind::Binding(_)) => {
                        let value = match *read {
                            NativeRead::Outer(index) => outer.get(index, &expression.range)?,
                            NativeRead::Subject => subject.expect("prepared subject"),
                            NativeRead::Local(index) => locals[index],
                        };
                        host.copy_value(value, context, budget, &expression.range)
                    }
                    (Self::Group(prepared), ExpressionKind::Group(value)) => prepared.execute(
                        value, host, input, context, bindings, outer, subject, locals, run_stage,
                        budget,
                    ),
                    (Self::Field(prepared), ExpressionKind::Field { value, field }) => {
                        let value = prepared.execute(
                            value, host, input, context, bindings, outer, subject, locals,
                            run_stage, budget,
                        )?;
                        let record = host.capture_record(&value).ok_or_else(|| {
                            QueryError::query(
                                field.range.clone(),
                                "field access requires a native capture",
                            )
                        })?;
                        host.copy_value(record.get(field, budget)?, context, budget, &field.range)
                    }
                    (
                        Self::Sequence {
                            input: prepared,
                            arguments: declared,
                        },
                        ExpressionKind::Sequence {
                            input: source,
                            steps,
                        },
                    ) => {
                        let explicit = matches!(source.value, ExpressionKind::Input)
                            && steps.first().is_some_and(|step| match &step.value {
                                ExpressionStep::Stage(stage) => host.is_explicit_source(stage),
                                ExpressionStep::Emit { .. } => false,
                            });
                        let mut value = if explicit {
                            budget.charge_results(1, Location::Query(expression.range.clone()))?;
                            host.lift_constructed(
                                crate::ConstructedValue::Absent,
                                vec![crate::ValueType::Absent],
                                context,
                                budget,
                                &expression.range,
                            )?
                        } else {
                            prepared.execute(
                                source, host, input, context, bindings, outer, subject, locals,
                                run_stage, budget,
                            )?
                        };
                        for (step, declared) in steps.iter().zip(declared) {
                            let ExpressionStep::Stage(stage) = &step.value else {
                                unreachable!("prepared native query is pure")
                            };
                            value = run_stage(
                                stage,
                                declared,
                                value,
                                context,
                                bindings,
                                budget,
                                &step.range,
                            )?;
                        }
                        Ok(value)
                    }
                    (Self::Set { left, rights }, ExpressionKind::Set { .. }) => {
                        let mut pending = Vec::new();
                        let mut operand = expression;
                        let mut input = input;
                        for prepared in rights {
                            let ExpressionKind::Set {
                                operator,
                                left,
                                right,
                            } = &operand.value
                            else {
                                unreachable!("prepared set shape")
                            };
                            if !pending.is_empty() {
                                budget.charge_program_work(
                                    1,
                                    Location::Query(operand.range.clone()),
                                )?;
                            }
                            admit_slots::<(
                                &H::SetOperator,
                                &NativeExpression<C, H>,
                                Option<H::Value>,
                                &Range<usize>,
                                &Self,
                            )>(1, budget, &operand.range)?;
                            let left_input = input
                                .as_ref()
                                .map(|input| host.copy_value(input, context, budget, &left.range))
                                .transpose()?;
                            pending.push((operator, right, input, &operand.range, prepared));
                            operand = left;
                            input = left_input;
                        }
                        let mut value = left.execute(
                            operand, host, input, context, bindings, outer, subject, locals,
                            run_stage, budget,
                        )?;
                        while let Some((operator, right, input, range, prepared)) = pending.pop() {
                            let right = prepared.execute(
                                right, host, input, context, bindings, outer, subject, locals,
                                run_stage, budget,
                            )?;
                            value = host.combine(operator, value, right, context, budget, range)?;
                        }
                        Ok(value)
                    }
                    _ => unreachable!("prepared native expression matches its borrowed source"),
                }
            },
        )
    }
}
