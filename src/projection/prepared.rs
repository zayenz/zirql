use super::*;

pub(super) enum NativeRead {
    Outer(usize),
    Subject,
    Local(usize),
}

pub(super) enum PreparedNative {
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
        reason = "Resolved native operands retain the current context, slots, scope and active allowance"
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
        budget.within_evaluation(
            expression.value.depth(),
            Location::Query(expression.range.clone()),
            |budget| {
                budget.charge_program_work(1, Location::Query(expression.range.clone()))?;
                match (self, &expression.value) {
                    (Self::Input, ExpressionKind::Input) => Ok(input),
                    (Self::Binding(read), ExpressionKind::Binding(_)) => {
                        let value = match *read {
                            NativeRead::Outer(index) => outer[index],
                            NativeRead::Subject => subject,
                            NativeRead::Local(index) => locals[index],
                        };
                        host.copy_value(value, context, budget, &expression.range)
                    }
                    (Self::Group(prepared), ExpressionKind::Group(value)) => prepared.run(
                        value, host, input, context, bindings, outer, subject, locals, arguments,
                        native, scope, budget,
                    ),
                    (Self::Field(prepared), ExpressionKind::Field { value, field }) => {
                        let value = prepared.run(
                            value, host, input, context, bindings, outer, subject, locals,
                            arguments, native, scope, budget,
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
                        let mut value = prepared.run(
                            source, host, input, context, bindings, outer, subject, locals,
                            arguments, native, scope, budget,
                        )?;
                        for (step, declared) in steps.iter().zip(declared) {
                            let ExpressionStep::Stage(stage) = &step.value else {
                                unreachable!("prepared native query is pure")
                            };
                            let mut stage_arguments = StageArguments {
                                host,
                                context: &mut *context,
                                bindings,
                                scope,
                                prepared: native,
                                declared,
                                values: arguments,
                            };
                            value = host.evaluate_stage_with_arguments(
                                stage,
                                value,
                                &mut stage_arguments,
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
                                H::Value,
                                &Range<usize>,
                                &Self,
                            )>(1, budget, &operand.range)?;
                            let left_input =
                                host.copy_value(&input, context, budget, &left.range)?;
                            pending.push((operator, right, input, &operand.range, prepared));
                            operand = left;
                            input = left_input;
                        }
                        let mut value = left.run(
                            operand, host, input, context, bindings, outer, subject, locals,
                            arguments, native, scope, budget,
                        )?;
                        while let Some((operator, right, input, range, prepared)) = pending.pop() {
                            let right = prepared.run(
                                right, host, input, context, bindings, outer, subject, locals,
                                arguments, native, scope, budget,
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
