// SPDX-License-Identifier: AGPL-3.0-or-later
//! FSL FR-740: the checked random parameters and workloads read back from a
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
    admit_text, EffectiveId, Integer, Meter, NodeKey, Quantity, Rational, ScalarLimits,
    TextPayload, Value, ValueType,
};

use filament_checked::node::CheckedLiteral;

use super::{
    CheckedProbabilisticModel, ProbabilisticOperation, RandomParameter, SupportValue, Workload,
};
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

/// The random parameters and workloads of a package, as its nodes spell
/// them, in the package's node order.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PackageProbabilistic {
    /// Every `random_parameter` node.
    pub random_parameters: Vec<PackageRandomParameter>,
    /// Every `workload` node.
    pub workloads: Vec<PackageWorkload>,
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
    /// FSL FR-740: the random parameters and workloads `read` spells, as S3
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
            .filter(|view| !view.is_source())
            .find_map(|view| {
                view.view
                    .type_identities()
                    .iter()
                    .find_map(|(key, effective)| {
                        (crate::model::key::object_type_node_key(key).ok().as_ref() == Some(node))
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
                declaring_key: key.clone(),
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

/// The value of `ty` that `literal` spells.
fn value_of(literal: &PackageLiteral, ty: &ValueType) -> Option<Value> {
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
            let mut meter = Meter::new(ScalarLimits {
                integer_bits: u64::MAX,
                decimal_digits: u64::MAX,
                scale_expansion: u64::MAX,
                text_input_bytes: u64::MAX,
                text_scalars: u64::MAX,
                normalized_scalars: u64::MAX,
                unit_edges: u64::MAX,
                value_occurrences: u64::MAX,
                work_units: u64::MAX,
                result_units: u64::MAX,
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
