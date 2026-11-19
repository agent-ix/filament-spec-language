// SPDX-License-Identifier: AGPL-3.0-or-later
//! S3's finite reward-domain validation (FR-185).

use filament_checked::node::NodeKind;
use filament_exact::{Charge, ChargePoint, Integer, Meter, Value, ValueType};

use super::{
    CheckCause, CheckRefusal, CheckedGraph, CheckingLimitKind, CheckingStage, Reward,
    RewardEvaluator, StageLimitCause,
};

enum Domain {
    Values { values: Vec<Value>, index: usize },
    Integers { current: Integer, upper: Integer },
    Empty,
}

impl Domain {
    fn current(&self) -> Option<Value> {
        match self {
            Self::Values { values, index } => values.get(*index).cloned(),
            Self::Integers { current, .. } => Some(Value::Integer(current.clone())),
            Self::Empty => None,
        }
    }

    fn advance(&mut self) -> bool {
        match self {
            Self::Values { values, index } if index + 1 < values.len() => {
                *index += 1;
                true
            }
            Self::Integers { current, upper } => {
                let next = current.add(&Integer::one());
                if next <= upper.clone() {
                    *current = next;
                    true
                } else {
                    false
                }
            }
            Self::Values { .. } | Self::Empty => false,
        }
    }

    fn reset(&mut self) {
        match self {
            Self::Values { index, .. } => *index = 0,
            Self::Integers { .. } | Self::Empty => {}
        }
    }
}

/// Check every finite argument vector of every reward read at S3. A reward
/// that reads an unbounded domain is deferred to model admission (FR-187).
pub(super) fn check_reward_signs(
    graph: &CheckedGraph,
    evaluator: &dyn RewardEvaluator,
    meter: &mut Meter,
    work_limit: u64,
) -> Result<(), Vec<CheckRefusal>> {
    for reward in graph.probabilistic_model().rewards() {
        check_reward(graph, reward, evaluator, meter, work_limit)?;
    }
    Ok(())
}

fn check_reward(
    graph: &CheckedGraph,
    reward: &Reward,
    evaluator: &dyn RewardEvaluator,
    meter: &mut Meter,
    work_limit: u64,
) -> Result<(), Vec<CheckRefusal>> {
    let parameter_count = reward.parameters().len();
    let mut read = vec![false; parameter_count];
    for node in reward.expression().descendants() {
        if let NodeKind::Local(slot) = node.kind() {
            if *slot < parameter_count {
                read[*slot] = true;
            }
        }
    }

    let mut arguments = reward
        .parameters()
        .iter()
        .map(|(_, ty)| representative(ty))
        .collect::<Vec<_>>();
    let mut positions = Vec::new();
    let mut domains = Vec::new();
    for (position, (name, ty)) in reward.parameters().iter().enumerate() {
        if !read[position] {
            continue;
        }
        let domain = graph
            .probabilistic_model()
            .random_parameter(reward.operation().key(), name)
            .map(|random| {
                Domain::Values(
                    random
                        .support()
                        .iter()
                        .map(|support| support.value.0.clone())
                        .collect(),
                    0,
                )
            })
            .or_else(|| finite_domain(ty));
        let Some(domain) = domain else {
            return Ok(());
        };
        if matches!(domain, Domain::Empty) {
            return Ok(());
        }
        if let Some(value) = domain.current() {
            arguments[position] = value;
        }
        positions.push(position);
        domains.push(domain);
    }

    loop {
        let denied = meter
            .charge(Charge::new(ChargePoint::DeclarationCheck).work(Integer::one()))
            .err();
        if let Some(denied) = denied {
            let refusal = CheckRefusal {
                location: reward.evaluation_location(),
                cause: CheckCause::ResourceExhausted(Box::new(StageLimitCause {
                    stage: CheckingStage::Typing,
                    kind: CheckingLimitKind::WorkBudget,
                    limit: work_limit,
                    actual: u128::from(denied.consumed)
                        + denied.next_charge.to_u64().unwrap_or(u64::MAX) as u128,
                    region: None,
                })),
            };
            return Err(vec![refusal]);
        }

        match evaluator.evaluate(graph, reward, arguments.clone(), meter) {
            Ok(Value::Integer(value)) if value.is_negative() => {
                return Err(vec![negative_refusal(reward, &arguments)]);
            }
            Ok(Value::Quantity(value)) if value.magnitude().numerator().is_negative() => {
                return Err(vec![negative_refusal(reward, &arguments)]);
            }
            Ok(_) => {}
            Err(failure) => {
                return Err(vec![CheckRefusal {
                    location: reward.evaluation_location(),
                    cause: CheckCause::RewardEvaluation(Box::new(failure)),
                }]);
            }
        }

        if positions.is_empty() {
            break;
        }
        let mut advanced = false;
        for index in (0..domains.len()).rev() {
            if domains[index].advance() {
                arguments[positions[index]] = domains[index]
                    .current()
                    .expect("a finite non-empty domain advances to a value");
                advanced = true;
                break;
            }
            domains[index].reset();
            arguments[positions[index]] = domains[index]
                .current()
                .expect("a finite non-empty domain resets to a value");
        }
        if !advanced {
            break;
        }
    }
    Ok(())
}

fn finite_domain(ty: &ValueType) -> Option<Domain> {
    match ty {
        ValueType::Boolean => Some(Domain::Values {
            values: vec![Value::Boolean(false), Value::Boolean(true)],
            index: 0,
        }),
        ValueType::Int(interval) => Some(Domain::Integers {
            current: interval.lower().clone(),
            upper: interval.upper().clone(),
        }),
        ValueType::Enum(shape) => {
            let values = shape
                .variants()
                .enumerate()
                .map(|(rank, variant)| {
                    Value::Enum(filament_exact::EnumMember::new(
                        variant.clone(),
                        u32::try_from(rank).expect("enum rank fits u32"),
                    ))
                })
                .collect::<Vec<_>>();
            if values.is_empty() {
                Some(Domain::Empty)
            } else {
                Some(Domain::Values { values, index: 0 })
            }
        }
        _ => None,
    }
}

fn representative(ty: &ValueType) -> Value {
    match ty {
        ValueType::Boolean => Value::Boolean(false),
        ValueType::Integer | ValueType::Int(_) => Value::Integer(Integer::from(0_i64)),
        ValueType::Enum(shape) => shape
            .variants()
            .next()
            .cloned()
            .map(|variant| Value::Enum(filament_exact::EnumMember::new(variant, 0)))
            .unwrap_or(Value::Boolean(false)),
        _ => Value::Boolean(false),
    }
}

fn negative_refusal(reward: &Reward, arguments: &[Value]) -> CheckRefusal {
    let values = reward
        .parameters()
        .iter()
        .zip(arguments)
        .map(|((name, _), value)| (name.clone(), format!("{value:?}")))
        .collect();
    CheckRefusal {
        location: reward.evaluation_location(),
        cause: CheckCause::Probabilistic(Box::new(super::ProbabilisticCause::NegativeReward {
            operation: reward.operation().name().to_owned(),
            reward: reward.name().to_owned(),
            arguments: values,
        })),
    }
}
