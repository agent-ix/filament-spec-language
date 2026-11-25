// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-561, FR-562: the kernel operation behind each shim.

use filament_exact::{
    evaluate_boolean, evaluate_decimal, evaluate_integer_arithmetic, evaluate_rational_arithmetic,
    form, order_numbers, retain_boolean, retain_composite, ArithmeticOperator, BooleanConnective,
    Charge, ChargePoint, Decimal, DecimalOperation, FieldValue, Integer, IntegerArithmetic, Meter,
    OrderedOperands, Rational, RationalArithmetic, Refusal, Undefined, Value, ValueType,
};
use filament_checked::node::Visit;
use filament_semantic_value::declaration::{operand_value, TypeEnvironment};
use filament_semantic_value::quantity::UnitScope;
use filament_semantic_value::loss::{LocatedLoss, ValueLoss};
use filament_semantic_value::stop::{outcome_into_stop, Stop};

use crate::translate::{divide, Op, OpKind};
use filament_checked::node::{Arithmetic, OrderedKind};
use fsl_eval::value::composite::{construct_record, construct_tuple};
use fsl_eval::value::collections::{Absorbed, Flow, Iteration, Shape as IterationShape, Spec};
use fsl_semantics::check::CheckedGraph;
use fsl_semantics::model::object_environment::ObjectEnvironment;

/// Why a kernel operation did not return a value.
pub(crate) enum Halt {
    /// The kernel's own stop (`Undefined`, `Refused` or `Incomplete`), passed
    /// up unchanged with the node's location.
    Stop(Stop),
    /// A backend invariant broke.
    Fault(&'static str),
}

impl From<Stop> for Halt {
    fn from(stop: Stop) -> Self {
        Self::Stop(stop)
    }
}

impl From<fsl_eval::value::collections::Halt> for Halt {
    fn from(halt: fsl_eval::value::collections::Halt) -> Self {
        iteration_halt(halt)
    }
}

fn integer(_values: &[Value], handle: i64) -> Result<Integer, Halt> {
    Ok(Integer::from(handle))
}

fn value(values: &[Value], handle: i64) -> Result<Value, Halt> {
    usize::try_from(handle)
        .ok()
        .and_then(|index| values.get(index))
        .cloned()
        .ok_or(Halt::Fault("jit-value-handle-is-valid"))
}

fn rational(values: &[Value], handle: i64) -> Result<Rational, Halt> {
    match usize::try_from(handle)
        .ok()
        .and_then(|index| values.get(index))
    {
        Some(Value::Rational(value)) => Ok(value.clone()),
        _ => Err(Halt::Fault("jit-rational-handle-is-valid")),
    }
}

fn decimal(values: &[Value], handle: i64) -> Result<Decimal, Halt> {
    match usize::try_from(handle)
        .ok()
        .and_then(|index| values.get(index))
    {
        Some(Value::Decimal(value)) => Ok(value.clone()),
        _ => Err(Halt::Fault("jit-decimal-handle-is-valid")),
    }
}

fn store_integer(values: &mut Vec<Value>, value: Integer) -> Result<i64, Halt> {
    let _ = values;
    crate::translate::integer_to_i64(&value)
        .ok_or(Halt::Fault("jit-integer-result-fits-register"))
}

fn store_value(values: &mut Vec<Value>, value: Value) -> Result<i64, Halt> {
    let value = match value {
        Value::Integer(integer) => return store_integer(values, integer),
        value => value,
    };
    let handle =
        i64::try_from(values.len()).map_err(|_| Halt::Fault("jit-value-handle-overflow"))?;
    values.push(value);
    Ok(handle)
}

fn iteration_index(iterations: &[Option<Iteration>], handle: i64) -> Result<usize, Halt> {
    usize::try_from(handle)
        .ok()
        .filter(|index| iterations.get(*index).is_some_and(Option::is_some))
        .ok_or(Halt::Fault("jit-iteration-handle-is-valid"))
}

fn iteration_halt(halt: fsl_eval::value::collections::Halt) -> Halt {
    match halt {
        fsl_eval::value::collections::Halt::Stop(stop) => Halt::Stop(stop),
        fsl_eval::value::collections::Halt::Fault(invariant) => Halt::Fault(invariant),
        fsl_eval::value::collections::Halt::SumSeedOutOfDomain => {
            Halt::Stop(Stop::Undefined(Undefined::SumOutOfDomain))
        }
        fsl_eval::value::collections::Halt::OnlyNotOne { .. } => {
            Halt::Fault("jit-only-cardinality-family-result-unimplemented")
        }
    }
}

fn decimal_conversion(
    operation: DecimalOperation<'_>,
    target: &filament_exact::DecimalType,
    location: &filament_semantic_value::location::ExpressionLocation,
    meter: &mut Meter,
    values: &mut Vec<Value>,
    losses: &mut Vec<LocatedLoss>,
) -> Result<i64, Halt> {
    let result = outcome_into_stop(evaluate_decimal(operation, target, meter))?;
    if let Some(loss) = result.loss() {
        losses.push(LocatedLoss {
            location: location.clone(),
            loss: ValueLoss::Decimal(loss.clone()),
        });
    }
    store_value(values, Value::Decimal(result.value().clone()))
}

/// Run `op` on the operands `a` and `b` under `meter`.
pub(crate) fn run(
    op: &Op,
    a: i64,
    b: i64,
    meter: &mut Meter,
    types: &TypeEnvironment,
    graph: &CheckedGraph,
    environment: Option<&ObjectEnvironment>,
    values: &mut Vec<Value>,
    builders: &mut Vec<Vec<Value>>,
    call_arguments: &mut Vec<Vec<Value>>,
    call_frames: &mut Vec<(usize, u64)>,
    frame_depth: &mut u32,
    live_occurrences: &mut u64,
    peak_live_occurrences: &mut u64,
    iterations: &mut Vec<Option<Iteration>>,
    losses: &mut Vec<LocatedLoss>,
) -> Result<i64, Halt> {
    match &op.kind {
        OpKind::Literal(value) => store_value(values, value.clone()),
        OpKind::FrameStart => {
            let index = call_arguments.len();
            call_arguments.push(Vec::new());
            Ok(i64::try_from(index).unwrap_or(i64::MAX))
        }
        OpKind::FrameArgument { boolean, integer: is_integer } => {
            let index = usize::try_from(a).map_err(|_| Halt::Fault("jit-call-frame-index"))?;
            let argument = if *boolean {
                Value::Boolean(b != 0)
            } else if *is_integer {
                Value::Integer(integer(values, b)?)
            } else {
                value(values, b)?
            };
            call_arguments
                .get_mut(index)
                .ok_or(Halt::Fault("jit-call-frame-index"))?
                .push(argument);
            Ok(0)
        }
        OpKind::FrameEnter => {
            let index = usize::try_from(a).map_err(|_| Halt::Fault("jit-call-frame-index"))?;
            let arguments = call_arguments
                .get(index)
                .ok_or(Halt::Fault("jit-call-frame-index"))?;
            let occurrences = arguments
                .iter()
                .fold(Integer::one(), |sum, argument| sum.add(&argument.occ()))
                .to_u64()
                .ok_or(Halt::Fault("jit-call-frame-occurrences-fit"))?;
            *live_occurrences = (*live_occurrences)
                .checked_add(occurrences)
                .ok_or(Halt::Fault("jit-live-occurrences-fit"))?;
            *peak_live_occurrences = (*peak_live_occurrences).max(*live_occurrences);
            *frame_depth = frame_depth
                .checked_add(1)
                .ok_or(Halt::Fault("jit-frame-depth-overflow"))?;
            call_frames.push((index, occurrences));
            Ok(0)
        }
        OpKind::FrameLeave => {
            let index = usize::try_from(a).map_err(|_| Halt::Fault("jit-call-frame-index"))?;
            let (opened_index, occurrences) = call_frames
                .pop()
                .ok_or(Halt::Fault("jit-call-frame-stack-present"))?;
            if opened_index != index {
                return Err(Halt::Fault("jit-call-frame-stack-matches"));
            }
            *frame_depth = frame_depth
                .checked_sub(1)
                .ok_or(Halt::Fault("jit-frame-depth-does-not-underflow"))?;
            *live_occurrences = (*live_occurrences)
                .checked_sub(occurrences)
                .ok_or(Halt::Fault("jit-live-occurrences-do-not-underflow"))?;
            call_arguments[index].clear();
            Ok(0)
        }
        OpKind::PopulationObjects { population, value_type } => {
            let environment =
                environment.ok_or(Halt::Fault("jit-population-environment-is-present"))?;
            let members = environment
                .population_members(&population.node)
                .ok_or(Halt::Fault("jit-population-members-are-observed"))?;
            let elements = members.iter().cloned().map(Value::Reference).collect();
            let ValueType::Collection(result_type) = value_type else {
                return Err(Halt::Fault("jit-population-objects-result-is-collection"));
            };
            store_value(values, outcome_into_stop(form(result_type, elements, meter))?)
        }
        OpKind::NamedAllInstances {
            population,
            members,
            value_type,
        } => {
            let environment =
                environment.ok_or(Halt::Fault("jit-population-environment-is-present"))?;
            if !environment.population_complete(&population.node) {
                return Err(Halt::Fault("jit-named-population-is-complete"));
            }
            let ValueType::Collection(result_type) = value_type else {
                return Err(Halt::Fault("jit-all-instances-result-is-collection"));
            };
            let ValueType::Reference(target) = result_type.element() else {
                return Err(Halt::Fault("jit-all-instances-element-is-reference"));
            };
            if !members
                .iter()
                .any(|member| types.conforms(target.clone(), member.clone()))
            {
                return Err(Halt::Fault("jit-all-instances-target-has-declared-coverage"));
            }
            let listed = environment
                .population_members(&population.node)
                .ok_or(Halt::Fault("jit-population-members-are-observed"))?;
            let mut selected = Vec::new();
            for reference in listed {
                meter
                    .charge(Charge::new(ChargePoint::PopulationVisit))
                    .map_err(Stop::Incomplete)?;
                if types.conforms(reference.object_type().clone(), target.clone()) {
                    selected.push(Value::Reference(reference.clone()));
                }
            }
            let count = u64::try_from(selected.len()).unwrap_or(u64::MAX);
            meter
                .charge(
                    Charge::new(ChargePoint::CollectionBound)
                        .size(filament_exact::LimitKind::ValueOccurrences, count),
                )
                .map_err(Stop::Incomplete)?;
            store_value(values, outcome_into_stop(form(result_type, selected, meter))?)
        }
        OpKind::BuildStart => {
            let index = i64::try_from(builders.len())
                .map_err(|_| Halt::Fault("jit-builder-handle-overflow"))?;
            builders.push(Vec::new());
            Ok(index)
        }
        OpKind::BuildValue {
            boolean,
            integer: is_integer,
        } => {
            let index = usize::try_from(a)
                .ok()
                .filter(|index| *index < builders.len())
                .ok_or(Halt::Fault("jit-builder-handle-is-valid"))?;
            let value = if *boolean {
                Value::Boolean(b != 0)
            } else if *is_integer {
                Value::Integer(integer(values, b)?)
            } else {
                value(values, b)?
            };
            builders[index].push(value);
            Ok(a)
        }
        OpKind::ChargeElement => {
            fsl_eval::value::collections::charge_element(meter).map_err(Stop::from)?;
            Ok(0)
        }
        OpKind::BuildRecord { declaration, slots } => {
            let index = usize::try_from(a)
                .ok()
                .filter(|index| *index < builders.len())
                .ok_or(Halt::Fault("jit-builder-handle-is-valid"))?;
            let arguments = std::mem::take(&mut builders[index]);
            let value =
                construct_record(types, declaration, slots, arguments).map_err(Halt::Fault)?;
            let value = outcome_into_stop(retain_composite(value, meter))?;
            store_value(values, value)
        }
        OpKind::BuildTuple { declaration } => {
            let index = usize::try_from(a)
                .ok()
                .filter(|index| *index < builders.len())
                .ok_or(Halt::Fault("jit-builder-handle-is-valid"))?;
            let arguments = std::mem::take(&mut builders[index]);
            let value = construct_tuple(types, declaration, arguments).map_err(Halt::Fault)?;
            let value = outcome_into_stop(retain_composite(value, meter))?;
            store_value(values, value)
        }
        OpKind::BuildCollection { collection_type } => {
            let index = usize::try_from(a)
                .ok()
                .filter(|index| *index < builders.len())
                .ok_or(Halt::Fault("jit-builder-handle-is-valid"))?;
            let arguments = std::mem::take(&mut builders[index]);
            let value = outcome_into_stop(form(collection_type, arguments, meter))?;
            store_value(values, value)
        }
        OpKind::Arithmetic(operator) => {
            let left = integer(values, a)?;
            let right = integer(values, b)?;
            let operation = match operator {
                Arithmetic::Add => IntegerArithmetic::Add(&left, &right),
                Arithmetic::Subtract => IntegerArithmetic::Subtract(&left, &right),
                Arithmetic::Multiply => IntegerArithmetic::Multiply(&left, &right),
            };
            let result = outcome_into_stop(evaluate_integer_arithmetic(operation, None, meter))?;
            store_integer(values, result)
        }
        OpKind::Rational { operator, domain } => {
            let left = rational(values, a)?;
            let right = rational(values, b)?;
            let operation = match operator {
                ArithmeticOperator::Add => RationalArithmetic::Add(&left, &right),
                ArithmeticOperator::Subtract => RationalArithmetic::Subtract(&left, &right),
                ArithmeticOperator::Multiply => RationalArithmetic::Multiply(&left, &right),
                ArithmeticOperator::Divide => RationalArithmetic::Divide(&left, &right),
            };
            let result =
                outcome_into_stop(evaluate_rational_arithmetic(operation, Some(domain), meter))?;
            store_value(values, Value::Rational(result))
        }
        OpKind::RationalNegate(domain) => {
            let operand = rational(values, a)?;
            let result = outcome_into_stop(filament_exact::evaluate_rational_arithmetic(
                RationalArithmetic::Negate(&operand),
                Some(domain),
                meter,
            ))?;
            store_value(values, Value::Rational(result))
        }
        OpKind::Decimal { operator, target } => {
            let left = decimal(values, a)?;
            let right = decimal(values, b)?;
            let operation = match operator {
                ArithmeticOperator::Add => DecimalOperation::Add(&left, &right),
                ArithmeticOperator::Subtract => DecimalOperation::Subtract(&left, &right),
                ArithmeticOperator::Multiply => DecimalOperation::Multiply(&left, &right),
                ArithmeticOperator::Divide => DecimalOperation::Divide(&left, &right),
            };
            let result = outcome_into_stop(evaluate_decimal(operation, target, meter))?;
            if let Some(loss) = result.loss() {
                losses.push(LocatedLoss {
                    location: op.location.clone(),
                    loss: ValueLoss::Decimal(loss.clone()),
                });
            }
            store_value(values, Value::Decimal(result.value().clone()))
        }
        OpKind::DecimalNegate(target) => {
            let operand = decimal(values, a)?;
            let result = outcome_into_stop(evaluate_decimal(
                DecimalOperation::Negate(&operand),
                target,
                meter,
            ))?;
            if let Some(loss) = result.loss() {
                losses.push(LocatedLoss {
                    location: op.location.clone(),
                    loss: ValueLoss::Decimal(loss.clone()),
                });
            }
            store_value(values, Value::Decimal(result.value().clone()))
        }
        OpKind::ConvertDecimal { target } => match value(values, a)? {
            Value::Decimal(source) => decimal_conversion(
                DecimalOperation::Round(&source),
                target,
                &op.location,
                meter,
                values,
                losses,
            ),
            Value::Rational(source) => {
                let numerator = Decimal::new(source.numerator().clone(), 0);
                let denominator = Decimal::new(source.denominator().clone(), 0);
                decimal_conversion(
                    DecimalOperation::Divide(&numerator, &denominator),
                    target,
                    &op.location,
                    meter,
                    values,
                    losses,
                )
            }
            _ => Err(Halt::Fault("jit-convert-decimal-source-is-covered")),
        },
        OpKind::ConvertText { target } => {
            let source = value(values, a)?;
            let (converted, loss) =
                fsl_eval::value::convert_text_value(types, target, &source, meter)
                    .map_err(Stop::from)?;
            if let Some(loss) = loss {
                losses.push(LocatedLoss {
                    location: op.location.clone(),
                    loss,
                });
            }
            if let Value::Boolean(value) = converted {
                Ok(i64::from(value))
            } else {
                store_value(values, converted)
            }
        }
        OpKind::ConvertScalar {
            operand,
            source_boolean,
            source_integer,
            integer_result,
        } => {
            let source = if *source_boolean {
                Value::Boolean(a != 0)
            } else if *source_integer {
                Value::Integer(integer(values, a)?)
            } else {
                value(values, a)?
            };
            let converted = operand_value(
                types,
                operand,
                &source,
                &UnitScope::new(types.units()),
                meter,
            )?;
            if *integer_result {
                let Value::Integer(converted) = converted else {
                    return Err(Halt::Fault("jit-convert-scalar-integer-result-is-integer"));
                };
                let register = converted
                    .to_string()
                    .parse::<i64>()
                    .map_err(|_| Halt::Fault("jit-convert-scalar-integer-fits-register"))?;
                Ok(register)
            } else {
                store_value(values, converted)
            }
        }
        OpKind::IterationStart {
            shape,
            value_type,
            only_source_filter,
            reduce,
            has_identity,
            identity_boolean,
            identity_integer,
        } => {
            let Value::Collection(source) = value(values, a)? else {
                return Err(Halt::Fault("jit-iteration-source-is-collection"));
            };
            let spec = Spec::new(*shape, value_type, *only_source_filter);
            let identity = if *has_identity {
                Some(if *identity_boolean {
                    Value::Boolean(b != 0)
                } else if *identity_integer {
                    Value::Integer(integer(values, b)?)
                } else {
                    value(values, b)?
                })
            } else {
                None
            };
            let state = Iteration::start(&spec, source, identity, *reduce, meter)
                .map_err(iteration_halt)?;
            let index = i64::try_from(iterations.len())
                .map_err(|_| Halt::Fault("jit-iteration-handle-overflow"))?;
            iterations.push(Some(state));
            Ok(index)
        }
        OpKind::IterationNext {
            shape,
            value_type,
            only_source_filter,
            element_boolean,
            element_integer,
        } => {
            let index = iteration_index(iterations, a)?;
            let spec = Spec::new(*shape, value_type, *only_source_filter);
            let state = iterations[index]
                .as_mut()
                .ok_or(Halt::Fault("jit-iteration-handle-is-live"))?;
            match state.next(&spec, meter).map_err(iteration_halt)? {
                Flow::Exhausted => Ok(-1),
                Flow::Visit { element, .. } => {
                    if *element_boolean {
                        match element {
                            Value::Boolean(value) => Ok(i64::from(value)),
                            _ => Err(Halt::Fault("jit-iteration-boolean-element")),
                        }
                    } else if *element_integer {
                        match element {
                            Value::Integer(value) => store_value(values, Value::Integer(value)),
                            _ => Err(Halt::Fault("jit-iteration-integer-element")),
                        }
                    } else {
                        store_value(values, element)
                    }
                }
            }
        }
        OpKind::IterationAccumulator { boolean, integer: is_integer } => {
            let index = iteration_index(iterations, a)?;
            let accumulator = iterations[index]
                .as_ref()
                .and_then(Iteration::accumulator)
                .cloned()
                .ok_or(Halt::Fault("jit-fold-accumulator-is-present"))?;
            if *boolean {
                match accumulator {
                    Value::Boolean(value) => Ok(i64::from(value)),
                    _ => Err(Halt::Fault("jit-fold-boolean-accumulator")),
                }
            } else if *is_integer {
                match accumulator {
                    Value::Integer(value) => store_value(values, Value::Integer(value)),
                    _ => Err(Halt::Fault("jit-fold-integer-accumulator")),
                }
            } else {
                store_value(values, accumulator)
            }
        }
        OpKind::IterationAbsorb {
            shape,
            value_type,
            only_source_filter,
            result_boolean,
            result_integer,
        } => {
            let index = iteration_index(iterations, a)?;
            let result = if *result_boolean {
                Value::Boolean(b != 0)
            } else if *result_integer {
                Value::Integer(integer(values, b)?)
            } else {
                value(values, b)?
            };
            let spec = Spec::new(*shape, value_type, *only_source_filter);
            let mut decimal_loss = None;
            let state = iterations[index]
                .as_mut()
                .ok_or(Halt::Fault("jit-iteration-handle-is-live"))?;
            let absorbed = state.absorb(&spec, result, |total, summand| {
                if matches!(shape, IterationShape::Query(Visit::Sum)) {
                    let Some(sum) = fsl_eval::value::collections::sum_exact(
                        value_type,
                        total,
                        summand,
                        meter,
                    ) else {
                        return Err(Halt::Fault("jit-query-sum-type-is-exact"));
                    };
                    match sum {
                        Ok((value, loss)) => {
                            decimal_loss = loss;
                            Ok(value)
                        }
                        Err(error) => Err(iteration_halt(error)),
                    }
                } else {
                    Ok(summand)
                }
            })?;
            if !matches!(absorbed, Absorbed::Continue) {
                return Err(Halt::Fault("jit-covered-iteration-does-not-short-circuit"));
            }
            if let Some(loss) = decimal_loss {
                losses.push(LocatedLoss {
                    location: op.location.clone(),
                    loss: ValueLoss::Decimal(loss),
                });
            }
            Ok(0)
        }
        OpKind::IterationFinish {
            shape,
            value_type,
            only_source_filter,
            result_boolean,
            result_integer,
        } => {
            let index = iteration_index(iterations, a)?;
            let state = iterations[index]
                .take()
                .ok_or(Halt::Fault("jit-iteration-handle-is-live"))?;
            let spec = Spec::new(*shape, value_type, *only_source_filter);
            let finished = state.finish(&spec, false, meter).map_err(iteration_halt)?;
            if *result_boolean {
                match finished.value {
                    Value::Boolean(value) => Ok(i64::from(value)),
                    _ => Err(Halt::Fault("jit-iteration-boolean-result")),
                }
            } else if *result_integer {
                match finished.value {
                    Value::Integer(value) => store_value(values, Value::Integer(value)),
                    _ => Err(Halt::Fault("jit-iteration-integer-result")),
                }
            } else {
                store_value(values, finished.value)
            }
        }
        OpKind::Field {
            field,
            optional,
            value_type,
        } => {
            let record = value(values, a)?;
            let read = fsl_eval::value::composite::read_field(
                types,
                record,
                field,
                |declaration, field| graph.scope().record_slot(declaration, field),
                *optional,
                value_type,
            )
            .map_err(|_| Halt::Fault("jit-record-field-read-valid"))?;
            match read.value {
                Value::Boolean(value) => Ok(i64::from(value)),
                value => store_value(values, value),
            }
        }
        OpKind::Contains {
            item_boolean,
            item_integer,
        } => {
            let item = if *item_boolean {
                Value::Boolean(b != 0)
            } else if *item_integer {
                Value::Integer(integer(values, b)?)
            } else {
                value(values, b)?
            };
            let Value::Collection(collection) = value(values, a)? else {
                return Err(Halt::Fault("jit-contains-collection-is-covered"));
            };
            let result = fsl_eval::value::collections::contains(&item, &collection, meter)
                .map_err(Stop::from)?;
            match result {
                Value::Boolean(found) => Ok(i64::from(found)),
                _ => Err(Halt::Fault("jit-contains-result-is-boolean")),
            }
        }
        OpKind::TupleProject {
            declaration,
            position,
        } => {
            let Value::Composite(tuple) = value(values, a)? else {
                return Err(Halt::Fault("jit-tuple-project-composite-is-covered"));
            };
            if tuple.declaration() != declaration {
                return Err(Halt::Fault("jit-tuple-project-declaration-is-covered"));
            }
            let Some(FieldValue::Present(value)) = tuple.slots().get(*position) else {
                return Err(Halt::Fault("jit-tuple-project-position-is-covered"));
            };
            match value.clone() {
                Value::Boolean(value) => Ok(i64::from(value)),
                value => store_value(values, value),
            }
        }
        OpKind::Index => {
            let position = integer(values, b)?;
            let Value::Collection(collection) = value(values, a)? else {
                return Err(Halt::Fault("jit-index-collection-is-covered"));
            };
            let Some(position) = position
                .to_u64()
                .and_then(|position| usize::try_from(position).ok())
            else {
                return Err(Halt::Stop(Stop::Undefined(Undefined::NoElement)));
            };
            let Some(value) = collection.elements().get(position).cloned() else {
                return Err(Halt::Stop(Stop::Undefined(Undefined::NoElement)));
            };
            fsl_eval::value::collections::charge_element(meter).map_err(Stop::from)?;
            let value =
                fsl_eval::value::collections::retain_scalar(value, meter).map_err(Stop::from)?;
            match value {
                Value::Boolean(value) => Ok(i64::from(value)),
                value => store_value(values, value),
            }
        }
        OpKind::DropFirst => {
            let Value::Collection(collection) = value(values, a)? else {
                return Err(Halt::Fault("jit-drop-first-collection-is-covered"));
            };
            let result = filament_eval::collection::drop_first(&collection, meter)
                .map_err(|_| Halt::Fault("jit-drop-first-valid-collection"))?;
            store_value(values, outcome_into_stop(result)?)
        }
        OpKind::RemoveOne {
            item_boolean,
            item_integer,
        } => {
            let item = if *item_boolean {
                Value::Boolean(b != 0)
            } else if *item_integer {
                Value::Integer(integer(values, b)?)
            } else {
                value(values, b)?
            };
            let Value::Collection(collection) = value(values, a)? else {
                return Err(Halt::Fault("jit-remove-one-collection-is-covered"));
            };
            let result = filament_eval::collection::remove_one(&collection, &item, types, meter)
                .map_err(|_| Halt::Fault("jit-remove-one-valid-collection"))?;
            store_value(values, outcome_into_stop(result)?)
        }
        OpKind::Flatten { result_type } => {
            let Value::Collection(outer) = value(values, a)? else {
                return Err(Halt::Fault("jit-flatten-collection-is-covered"));
            };
            let result = fsl_eval::value::collections::flatten(result_type, &outer, meter)
                .map_err(|halt| match halt {
                    fsl_eval::value::collections::Halt::Stop(stop) => Halt::Stop(stop),
                    _ => Halt::Fault("jit-flatten-valid-collection"),
                })?;
            store_value(values, result)
        }
        OpKind::ConvertCollection { target } => {
            let Value::Collection(source) = value(values, a)? else {
                return Err(Halt::Fault("jit-convert-collection-source-is-covered"));
            };
            let (result, _) = fsl_eval::value::collections::convert(target, &source, false, meter)?;
            store_value(values, result)
        }
        OpKind::Size => {
            let size = match value(values, a)? {
                Value::Text(text) => Integer::from(text.length()),
                Value::Collection(collection) => Integer::from(collection.elements().len()),
                _ => return Err(Halt::Fault("jit-size-operand-is-covered")),
            };
            meter
                .charge(Charge::new(ChargePoint::CollectionResultRetain).results(1))
                .map_err(Stop::Incomplete)?;
            store_value(values, Value::Integer(size))
        }
        OpKind::Negate => {
            let value = integer(values, a)?;
            let result = outcome_into_stop(evaluate_integer_arithmetic(
                IntegerArithmetic::Negate(&value),
                None,
                meter,
            ))?;
            store_integer(values, result)
        }
        OpKind::Divide {
            operator,
            selection,
        } => {
            let left = integer(values, a)?;
            let right = integer(values, b)?;
            let outcome = divide(*operator, selection.as_ref(), &left, &right, meter)
                .ok_or(Halt::Fault("jit-division-selection-missing"))?;
            let result = outcome_into_stop(outcome)?;
            store_integer(values, result)
        }
        OpKind::ExactDivide(domain) => {
            let left = Rational::from_integer(integer(values, a)?);
            let right = Rational::from_integer(integer(values, b)?);
            let result = outcome_into_stop(evaluate_rational_arithmetic(
                RationalArithmetic::Divide(&left, &right),
                Some(domain),
                meter,
            ))?;
            store_value(values, Value::Rational(result))
        }
        OpKind::Order { operator, kind } => {
            let outcome = match kind {
                OrderedKind::Integers => {
                    let left = integer(values, a)?;
                    let right = integer(values, b)?;
                    order_numbers(*operator, OrderedOperands::Integers(&left, &right), meter)
                }
                OrderedKind::Rationals => {
                    let left = rational(values, a)?;
                    let right = rational(values, b)?;
                    order_numbers(*operator, OrderedOperands::Rationals(&left, &right), meter)
                }
                OrderedKind::Decimals => {
                    let left = decimal(values, a)?;
                    let right = decimal(values, b)?;
                    order_numbers(*operator, OrderedOperands::Decimals(&left, &right), meter)
                }
                OrderedKind::Texts => {
                    let left = match value(values, a)? {
                        Value::Text(value) => value,
                        _ => return Err(Halt::Fault("jit-left-text-handle-is-valid")),
                    };
                    let right = match value(values, b)? {
                        Value::Text(value) => value,
                        _ => return Err(Halt::Fault("jit-right-text-handle-is-valid")),
                    };
                    let operator = match operator {
                        filament_exact::OrderingOperator::Less => {
                            filament_exact::ComparisonOperator::Less
                        }
                        filament_exact::OrderingOperator::LessOrEqual => {
                            filament_exact::ComparisonOperator::LessOrEqual
                        }
                        filament_exact::OrderingOperator::Greater => {
                            filament_exact::ComparisonOperator::Greater
                        }
                        filament_exact::OrderingOperator::GreaterOrEqual => {
                            filament_exact::ComparisonOperator::GreaterOrEqual
                        }
                    };
                    filament_exact::compare_text(operator, &left, &right, meter)
                        .map_err(|_| Halt::Fault("jit-text-order-types-are-compatible"))?
                }
                _ => return Err(Halt::Fault("jit-order-kind-is-supported")),
            };
            let holds = outcome_into_stop(outcome)?;
            Ok(i64::from(holds))
        }
        OpKind::Equality {
            checked,
            left_boolean,
            left_integer,
            right_boolean,
            right_integer,
        } => {
            let left = if *left_boolean {
                Value::Boolean(a != 0)
            } else if *left_integer {
                Value::Integer(integer(values, a)?)
            } else {
                value(values, a).map_err(|_| Halt::Fault("jit-left-equality-handle-is-valid"))?
            };
            let right = if *right_boolean {
                Value::Boolean(b != 0)
            } else if *right_integer {
                Value::Integer(integer(values, b)?)
            } else {
                value(values, b).map_err(|_| Halt::Fault("jit-right-equality-handle-is-valid"))?
            };
            let holds = outcome_into_stop(checked.evaluate(types, &left, &right, meter))?;
            Ok(i64::from(holds))
        }
        OpKind::Not => {
            let result =
                outcome_into_stop(evaluate_boolean(BooleanConnective::Not(a != 0), meter))?;
            Ok(i64::from(result))
        }
        OpKind::Retain => {
            let result = retain_boolean(a != 0, meter).map_err(Stop::Incomplete)?;
            Ok(i64::from(result))
        }
        OpKind::ChargeCall => {
            meter
                .charge(Charge::new(ChargePoint::FunctionCall))
                .map_err(Stop::Incomplete)?;
            Ok(0)
        }
        OpKind::Coerce(target) => {
            let value = integer(values, a)?;
            if target.contains(&value) {
                Ok(a)
            } else {
                Err(Halt::Stop(Stop::Refused(Refusal::IntegerOutOfDomain {
                    target: Box::new(target.clone()),
                })))
            }
        }
    }
}
