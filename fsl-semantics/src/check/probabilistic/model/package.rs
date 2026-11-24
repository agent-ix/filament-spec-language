// SPDX-License-Identifier: AGPL-3.0-or-later
//! FSL FR-740: the checked random parameters, workloads and delays read back from a
//! package's `probabilistic` nodes.
//!
//! A package carries a declaration's identity, its values and its weights;
//! the admitted model views (the domain packages a package selects) carry the
//! rest. [`StateModel::probabilistic_model`] joins the two: the view gives the
//! declaring type's [`EffectiveId`], the operation's declaration key, the
//! parameter's position and type, and with the type the enum ranks, the unit
//! and the text profile of each value, so the records equal the ones S3
//! checked.
//!
//! The package holds no `model` alias, so the alias of a read record is the
//! identity of the domain package that declares the operation, and a
//! record's source order is its order in the package.

use filament_exact::{
    EffectiveId, Integer, Meter, NodeKey, Quantity, Rational, ScalarLimits, TextPayload, Value,
    ValueType, admit_text,
};

use filament_checked::node::CheckedLiteral;

use super::{
    CheckedProbabilisticModel, Delay, DelayDistribution, ProbabilisticOperation, RandomParameter,
    ReadReward, SupportValue, Workload,
};
use crate::check::DelayFault;
use crate::model::operation::{OperationLookup, OperationTable};
use crate::model::state::StateModel;

/// A support value as a package spells it, before its parameter's type
/// reads it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PackageLiteral {
    /// A `boolean` literal.
    Boolean(bool),
    /// An `integer` literal.
    Integer(Integer),
    /// A `rational` literal.
    Rational(Rational),
    /// A `quantity` literal: the magnitude in the unit its parameter's type
    /// names.
    Quantity(Rational),
    /// A `text` literal.
    Text(String),
    /// An `enum` literal: the case of the parameter's enumeration.
    Enum(String),
}

/// One support value of a package's random parameter with its weight.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PackageSupport {
    /// The value.
    pub value: PackageLiteral,
    /// The weight, positive.
    pub weight: Rational,
}

/// A `random_parameter` node of a package.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PackageRandomParameter {
    /// The `model` object type node that declares the operation.
    pub declaring: NodeKey,
    /// The operation's name.
    pub operation: String,
    /// The parameter's name.
    pub parameter: String,
    /// The support in declared order.
    pub support: Vec<PackageSupport>,
}

/// One operation's weight in a package's workload.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PackageWeight {
    /// The `model` object type node that declares the operation.
    pub declaring: NodeKey,
    /// The operation's name.
    pub operation: String,
    /// The weight, not negative.
    pub weight: Rational,
}

/// A `workload` node of a package.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PackageWorkload {
    /// The workload's name.
    pub name: String,
    /// The `model` object type node the workload is on.
    pub model: NodeKey,
    /// The weights in ascending order of declaring node and operation.
    pub weights: Vec<PackageWeight>,
}

/// The distribution of a package's `delay_distribution` node, with its times
/// as magnitudes in the unit of the model's time source.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PackageDelayFamily {
    /// `uniform`.
    Uniform,
    /// `uniform_interval`.
    UniformInterval {
        /// The lower end.
        lower: Rational,
        /// The upper end.
        upper: Rational,
    },
    /// `exponential`.
    Exponential {
        /// The rate.
        rate: Rational,
    },
    /// `discrete`: each delay with its weight, in declared order.
    Discrete(Vec<(Rational, Rational)>),
}

/// A `delay_distribution` node of a package.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PackageDelay {
    /// The `model` object type node that declares the operation.
    pub declaring: NodeKey,
    /// The operation's name.
    pub operation: String,
    /// The distribution.
    pub family: PackageDelayFamily,
}

/// A `reward` node of a package.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PackageReward {
    /// The `model` object type node that declares the operation.
    pub declaring: NodeKey,
    /// The operation's name.
    pub operation: String,
    /// The reward's name.
    pub name: String,
    /// The integer or quantity type node of the reward.
    pub value_type: NodeKey,
    /// The clause function node the reward's expression lowered to.
    pub function: NodeKey,
}

/// The random parameters, workloads, delays and rewards of a package, as its
/// nodes spell them, in the package's node order.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PackageProbabilistic {
    /// Every `random_parameter` node.
    pub random_parameters: Vec<PackageRandomParameter>,
    /// Every `workload` node.
    pub workloads: Vec<PackageWorkload>,
    /// Every `delay_distribution` node.
    pub delays: Vec<PackageDelay>,
    /// Every `reward` node.
    pub rewards: Vec<PackageReward>,
}

/// Why a package's probabilistic nodes do not rebuild S3's records.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum PackageProbabilisticRefusal {
    /// A node names an object type no selected domain package declares.
    #[error("no selected domain package declares the object type of {0:?}")]
    UnknownModel(NodeKey),
    /// A node names an operation its object type does not declare.
    #[error("the object type declares no operation `{0}`")]
    UnknownOperation(String),
    /// A random parameter names a parameter its operation does not have.
    #[error("operation `{operation}` has no parameter `{parameter}`")]
    UnknownParameter {
        /// The operation.
        operation: String,
        /// The parameter.
        parameter: String,
    },
    /// A support value is not a value of its parameter's type.
    #[error("a support value of `{operation}({parameter})` is not of the parameter's type")]
    ValueOutsideType {
        /// The operation.
        operation: String,
        /// The parameter.
        parameter: String,
    },
    /// A delay breaks FR-253: a `uniform_interval` outside `0 <= a <= b`, a
    /// negative or repeated `discrete` delay, or a rate or weight that is not
    /// positive.
    #[error("the delay of `{operation}` breaks FR-253: {fault:?}")]
    InvalidDelay {
        /// The operation.
        operation: String,
        /// The rule it breaks.
        fault: DelayFault,
    },
    /// Two rewards of one name on one operation.
    #[error("operation `{operation}` has two rewards named `{reward}`")]
    DuplicateReward {
        /// The operation.
        operation: String,
        /// The reward.
        reward: String,
    },
    /// A package's weights sum to zero.
    #[error("the weights of `{operation}({parameter})` sum to zero")]
    ZeroSum {
        /// The operation.
        operation: String,
        /// The parameter.
        parameter: String,
    },
}

impl StateModel {
    /// FSL FR-740: the random parameters, workloads and delays `read` spells, as S3
    /// records them, joined to the selected domain packages' views and to
    /// `operations`, the operation table of the same packages
    /// ([`Self::operation_table`]).
    ///
    /// # Errors
    ///
    /// A [`PackageProbabilisticRefusal`] for a node whose model type,
    /// operation, parameter or values the views do not carry.
    pub fn probabilistic_model(
        &self,
        operations: &OperationTable,
        read: &PackageProbabilistic,
    ) -> Result<CheckedProbabilisticModel, PackageProbabilisticRefusal> {
        let mut model = CheckedProbabilisticModel::default();
        let mut declared = 0;
        for parameter in &read.random_parameters {
            let operation =
                self.operation(operations, &parameter.declaring, &parameter.operation)?;
            model
                .random_parameters
                .push(self.random_parameter(declared, operation, parameter)?);
            declared += 1;
        }
        for workload in &read.workloads {
            model
                .workloads
                .push(self.workload(operations, declared, workload)?);
            declared += 1;
        }
        for delay in &read.delays {
            let (operation, _) = self.operation(operations, &delay.declaring, &delay.operation)?;
            if let Some(fault) = delay_fault(&delay.family) {
                return Err(PackageProbabilisticRefusal::InvalidDelay {
                    operation: delay.operation.clone(),
                    fault,
                });
            }
            model.delays.push(Delay {
                declared,
                operation,
                distribution: match &delay.family {
                    PackageDelayFamily::Uniform => DelayDistribution::Uniform,
                    PackageDelayFamily::UniformInterval { lower, upper } => {
                        DelayDistribution::UniformRange {
                            a: lower.clone(),
                            b: upper.clone(),
                        }
                    }
                    PackageDelayFamily::Exponential { rate } => {
                        DelayDistribution::Exponential { rate: rate.clone() }
                    }
                    PackageDelayFamily::Discrete(rows) => DelayDistribution::Discrete(rows.clone()),
                },
            });
            declared += 1;
        }
        for reward in &read.rewards {
            let (operation, declaration) =
                self.operation(operations, &reward.declaring, &reward.operation)?;
            if model.read_rewards.iter().any(|earlier| {
                earlier.operation.key == operation.key && earlier.name == reward.name
            }) {
                return Err(PackageProbabilisticRefusal::DuplicateReward {
                    operation: reward.operation.clone(),
                    reward: reward.name.clone(),
                });
            }
            model.read_rewards.push(ReadReward {
                declared,
                name: reward.name.clone(),
                operation,
                parameters: declaration.parameters().to_vec(),
                value_type: reward.value_type.clone(),
                function: reward.function.clone(),
            });
            declared += 1;
        }
        Ok(model)
    }

    /// The selected domain package's view, declaration key and effective
    /// identity of the object type `node` names.
    fn model_type(
        &self,
        node: &NodeKey,
    ) -> Result<
        (
            &crate::model::observation::ModelView,
            &filament_checked::key::DeclarationKey,
            &EffectiveId,
        ),
        PackageProbabilisticRefusal,
    > {
        self.views
            .iter()
            .find_map(|view| {
                view.view
                    .type_identities()
                    .iter()
                    .find_map(|(key, effective)| {
                        (crate::model::key::declaring_node_key(key, node.package()).as_ref()
                            == Some(node))
                        .then_some((view, key, effective))
                    })
            })
            .ok_or_else(|| PackageProbabilisticRefusal::UnknownModel(node.clone()))
    }

    fn operation<'o>(
        &self,
        operations: &'o OperationTable,
        node: &NodeKey,
        name: &str,
    ) -> Result<
        (
            ProbabilisticOperation,
            &'o crate::model::operation::OperationDeclaration,
        ),
        PackageProbabilisticRefusal,
    > {
        let (view, key, effective) = self.model_type(node)?;
        let unknown = || PackageProbabilisticRefusal::UnknownOperation(name.to_owned());
        let OperationLookup::Declared {
            declaring,
            operation,
        } = operations.resolve(&self.types, effective.clone(), name)
        else {
            return Err(unknown());
        };
        if &declaring != effective {
            return Err(unknown());
        }
        let resolved = crate::check::state_clause::ClauseOperation {
            declaring: declaring.clone(),
            declaration: operation.clone(),
        };
        let operation_key = crate::check::AdmittedModel::from_view(&view.view)
            .operation_key(&resolved)
            .cloned()
            .ok_or_else(unknown)?;
        Ok((
            ProbabilisticOperation {
                model: view.view.domain_package().model_selection.identity.clone(),
                key: operation_key,
                declaring,
                declaring_key: Some(key.clone()),
                name: name.to_owned(),
            },
            operation,
        ))
    }

    fn random_parameter(
        &self,
        declared: usize,
        (operation, declaration): (
            ProbabilisticOperation,
            &crate::model::operation::OperationDeclaration,
        ),
        read: &PackageRandomParameter,
    ) -> Result<RandomParameter, PackageProbabilisticRefusal> {
        let outside = || PackageProbabilisticRefusal::ValueOutsideType {
            operation: read.operation.clone(),
            parameter: read.parameter.clone(),
        };
        let (position, (_, value_type)) = declaration
            .parameters()
            .iter()
            .enumerate()
            .find(|(_, (name, _))| *name == read.parameter)
            .ok_or_else(|| PackageProbabilisticRefusal::UnknownParameter {
                operation: read.operation.clone(),
                parameter: read.parameter.clone(),
            })?;
        let mut total = Rational::from_integer(Integer::zero());
        for support in &read.support {
            total = total.add(&support.weight);
        }
        let support = read
            .support
            .iter()
            .map(|support| {
                let probability = support.weight.div(&total).ok_or_else(|| {
                    PackageProbabilisticRefusal::ZeroSum {
                        operation: read.operation.clone(),
                        parameter: read.parameter.clone(),
                    }
                })?;
                Ok(SupportValue {
                    value: CheckedLiteral(
                        value_of(&support.value, value_type).ok_or_else(outside)?,
                    ),
                    weight: support.weight.clone(),
                    probability,
                })
            })
            .collect::<Result<Vec<_>, PackageProbabilisticRefusal>>()?;
        Ok(RandomParameter {
            declared,
            operation,
            parameter: read.parameter.clone(),
            position,
            value_type: value_type.clone(),
            support,
        })
    }

    fn workload(
        &self,
        operations: &OperationTable,
        declared: usize,
        read: &PackageWorkload,
    ) -> Result<Workload, PackageProbabilisticRefusal> {
        let mut weights = Vec::with_capacity(read.weights.len());
        for weight in &read.weights {
            let (operation, _) =
                self.operation(operations, &weight.declaring, &weight.operation)?;
            weights.push((operation, weight.weight.clone()));
        }
        let (view, key, _) = self.model_type(&read.model)?;
        let (model, model_key) = (
            view.view.domain_package().model_selection.identity.clone(),
            Some(key.clone()),
        );
        Ok(Workload {
            declared,
            name: read.name.clone(),
            model,
            model_key,
            weights,
        })
    }
}

/// The FR-253 rule `family` breaks, if any: the checks S3 makes of a written
/// distribution, repeated on the read-back so a package from another producer
/// rebuilds no record S3 could not have checked.
fn delay_fault(family: &PackageDelayFamily) -> Option<DelayFault> {
    let positive = |value: &Rational| !value.is_zero() && !value.numerator().is_negative();
    match family {
        PackageDelayFamily::Uniform => None,
        PackageDelayFamily::UniformInterval { lower, upper } => (lower.numerator().is_negative()
            || upper.sub(lower).numerator().is_negative())
        .then_some(DelayFault::UniformRange),
        PackageDelayFamily::Exponential { rate } => {
            (!positive(rate)).then_some(DelayFault::NonPositiveRate)
        }
        PackageDelayFamily::Discrete(rows) => {
            for (at, (delay, weight)) in rows.iter().enumerate() {
                if delay.numerator().is_negative() {
                    return Some(DelayFault::NegativeDelay);
                }
                if rows[..at].iter().any(|(earlier, _)| earlier == delay) {
                    return Some(DelayFault::RepeatedDelay);
                }
                if !positive(weight) {
                    return Some(DelayFault::NonPositiveWeight);
                }
            }
            None
        }
    }
}

/// The scalar-limit bits of one package literal (FR-185's exact-value
/// measure).
const LITERAL_INTEGER_BITS: u64 = 65_536;

/// The value of `ty` that `literal` spells, admitted by `ty` itself: an
/// `Int`, `Rational` or bounded-quantity literal outside its type's bounds
/// is no value of it, so the reader refuses what S3 would have refused.
fn value_of(literal: &PackageLiteral, ty: &ValueType) -> Option<Value> {
    let value = unchecked_value_of(literal, ty)?;
    ty.admits(&value).then_some(value)
}

fn unchecked_value_of(literal: &PackageLiteral, ty: &ValueType) -> Option<Value> {
    match (literal, ty) {
        (PackageLiteral::Boolean(value), ValueType::Boolean) => Some(Value::Boolean(*value)),
        (PackageLiteral::Integer(value), ValueType::Integer | ValueType::Int(_)) => {
            Some(Value::Integer(value.clone()))
        }
        (PackageLiteral::Rational(value), ValueType::Rational(_)) => {
            Some(Value::Rational(value.clone()))
        }
        (PackageLiteral::Quantity(magnitude), ValueType::Quantity(unit)) => Some(Value::Quantity(
            Quantity::new(magnitude.clone(), unit.clone()),
        )),
        (PackageLiteral::Quantity(magnitude), ValueType::BoundedQuantity(bounded)) => Some(
            Value::Quantity(Quantity::new(magnitude.clone(), bounded.unit().clone())),
        ),
        (PackageLiteral::Text(text), ValueType::Text(text_type)) => {
            let payload = TextPayload::from_utf8(text.as_bytes()).ok()?;
            // Limits sized to this literal: its own bytes and scalars in,
            // and the worst normalization expansion (18x, NFKC) out.
            let length = u64::try_from(text.len()).ok()?;
            let mut meter = Meter::new(ScalarLimits {
                integer_bits: LITERAL_INTEGER_BITS,
                decimal_digits: LITERAL_INTEGER_BITS,
                scale_expansion: LITERAL_INTEGER_BITS,
                text_input_bytes: length,
                text_scalars: length,
                normalized_scalars: length.saturating_mul(18),
                unit_edges: LITERAL_INTEGER_BITS,
                value_occurrences: LITERAL_INTEGER_BITS,
                work_units: length
                    .saturating_mul(64)
                    .saturating_add(LITERAL_INTEGER_BITS),
                result_units: length
                    .saturating_mul(64)
                    .saturating_add(LITERAL_INTEGER_BITS),
            });
            Some(Value::Text(
                admit_text(&payload, text_type, &mut meter).completed()?,
            ))
        }
        (PackageLiteral::Enum(case), ValueType::Enum(shape)) => {
            let variant = shape.variants().find(|variant| variant.case() == case)?;
            Some(Value::Enum(filament_exact::EnumMember::new(
                variant.clone(),
                shape.rank(variant)?,
            )))
        }
        _ => None,
    }
}
