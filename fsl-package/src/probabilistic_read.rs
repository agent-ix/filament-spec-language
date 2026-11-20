// SPDX-License-Identifier: AGPL-3.0-or-later
//! FSL FR-740: the random parameters, workloads, delays and rewards of an admitted
//! package, read from its `probabilistic` nodes.
//!
//! The IR's typed reads (such as [`CheckedPackageV2::random_parameters`] and
//! [`CheckedPackageV2::workloads`]) are keyed by a model node and an
//! operation, so a reader that does not know either walks the nodes itself;
//! this reads the admitted body of each random parameter, workload and
//! delay distribution node. The result names nodes and spells
//! values: [`fsl_semantics::model::state::StateModel::probabilistic_model`]
//! joins it to the selected domain packages and rebuilds S3's records.

use filament_contract_model::{
    CheckedLiteralValue, CheckedNodeKind, CheckedPackageV2, CheckedTerm, LiteralKind,
    ProbabilisticForm,
};
use filament_exact::{Integer, Rational};
use fsl_semantics::check::{
    PackageDelay, PackageDelayFamily, PackageLiteral, PackageProbabilistic, PackageRandomParameter,
    PackageReward, PackageSupport, PackageWeight, PackageWorkload,
};

use crate::checked_v2::wire_graph;
use crate::BarePackage;

/// The package's `probabilistic` nodes do not read.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
#[error("an admitted probabilistic node does not read: {0}")]
pub struct ProbabilisticReadFault(pub &'static str);

fn bound(term: &CheckedTerm, position: usize) -> Option<&CheckedTerm> {
    match term {
        CheckedTerm::Aggregate { members } => match members.get(position)? {
            CheckedTerm::Binding { value, .. } => Some(value),
            _ => None,
        },
        _ => None,
    }
}

fn members(term: &CheckedTerm) -> Option<&[CheckedTerm]> {
    match term {
        CheckedTerm::Aggregate { members } => Some(members),
        _ => None,
    }
}

fn text(term: &CheckedTerm) -> Option<&str> {
    match term.literal_read()?.value {
        CheckedLiteralValue::Text(text) => Some(text),
        _ => None,
    }
}

fn rational(term: &CheckedTerm) -> Option<Rational> {
    match term.literal_read()?.value {
        CheckedLiteralValue::Rational {
            numerator,
            denominator,
        } => Rational::new(
            numerator.to_string().parse().ok()?,
            denominator.to_string().parse().ok()?,
        )
        .ok(),
        CheckedLiteralValue::Integer(value) => Some(Rational::from_integer(
            value.to_string().parse::<Integer>().ok()?,
        )),
        _ => None,
    }
}

fn literal(term: &CheckedTerm) -> Option<PackageLiteral> {
    let read = term.literal_read()?;
    Some(match (read.kind, read.value) {
        (LiteralKind::Boolean, CheckedLiteralValue::Boolean(value)) => {
            PackageLiteral::Boolean(value)
        }
        (LiteralKind::Integer, CheckedLiteralValue::Integer(value)) => {
            PackageLiteral::Integer(value.to_string().parse().ok()?)
        }
        (LiteralKind::Quantity, _) => PackageLiteral::Quantity(rational(term)?),
        (LiteralKind::Rational, _) => PackageLiteral::Rational(rational(term)?),
        (LiteralKind::Text, CheckedLiteralValue::Text(value)) => {
            PackageLiteral::Text(value.to_owned())
        }
        (LiteralKind::Enum, CheckedLiteralValue::Enum(case)) => {
            PackageLiteral::Enum(case.to_owned())
        }
        _ => return None,
    })
}

impl BarePackage<'_> {
    /// FSL FR-740: every `random_parameter`, `workload` and `delay_distribution` node of the
    /// package, in node order, with the model nodes as structural keys.
    ///
    /// # Errors
    ///
    /// A [`ProbabilisticReadFault`] when an admitted node's body does not
    /// read, which admission rules out.
    pub fn probabilistic(&self) -> Result<PackageProbabilistic, ProbabilisticReadFault> {
        read(self.package())
    }
}

fn read(package: &CheckedPackageV2) -> Result<PackageProbabilistic, ProbabilisticReadFault> {
    let fault = ProbabilisticReadFault;
    let graph = wire_graph(package).map_err(|_| fault("node-ids-rebuild-structural-keys"))?;
    let key = |term: &CheckedTerm| match term {
        CheckedTerm::Reference { target } => graph.key(target).ok(),
        _ => None,
    };
    let mut read = PackageProbabilistic::default();
    for (node, kind) in package
        .resolved_graph()
        .nodes
        .iter()
        .zip(package.node_kinds())
    {
        let body = &node.body;
        match kind {
            CheckedNodeKind::Probabilistic(ProbabilisticForm::RandomParameter) => {
                let support = members(bound(body, 3).ok_or(fault("random-parameter-support"))?)
                    .ok_or(fault("random-parameter-support"))?
                    .iter()
                    .map(|row| {
                        Some(PackageSupport {
                            value: literal(bound(row, 0)?)?,
                            weight: rational(bound(row, 1)?)?,
                        })
                    })
                    .collect::<Option<Vec<_>>>()
                    .ok_or(fault("random-parameter-support-row"))?;
                read.random_parameters.push(PackageRandomParameter {
                    declaring: bound(body, 0)
                        .and_then(key)
                        .ok_or(fault("random-parameter-declaring"))?,
                    operation: bound(body, 1)
                        .and_then(text)
                        .ok_or(fault("random-parameter-operation"))?
                        .to_owned(),
                    parameter: bound(body, 2)
                        .and_then(text)
                        .ok_or(fault("random-parameter-parameter"))?
                        .to_owned(),
                    support,
                });
            }
            CheckedNodeKind::Probabilistic(ProbabilisticForm::Workload) => {
                let weights = members(bound(body, 2).ok_or(fault("workload-weights"))?)
                    .ok_or(fault("workload-weights"))?
                    .iter()
                    .map(|row| {
                        Some(PackageWeight {
                            declaring: key(bound(row, 0)?)?,
                            operation: text(bound(row, 1)?)?.to_owned(),
                            weight: rational(bound(row, 2)?)?,
                        })
                    })
                    .collect::<Option<Vec<_>>>()
                    .ok_or(fault("workload-weight-row"))?;
                read.workloads.push(PackageWorkload {
                    name: bound(body, 1)
                        .and_then(text)
                        .ok_or(fault("workload-name"))?
                        .to_owned(),
                    model: bound(body, 0)
                        .and_then(key)
                        .ok_or(fault("workload-model"))?,
                    weights,
                });
            }
            CheckedNodeKind::Probabilistic(ProbabilisticForm::DelayDistribution) => {
                read.delays.push(PackageDelay {
                    declaring: bound(body, 0)
                        .and_then(key)
                        .ok_or(fault("delay-declaring"))?,
                    operation: bound(body, 1)
                        .and_then(text)
                        .ok_or(fault("delay-operation"))?
                        .to_owned(),
                    family: delay_family(body).ok_or(fault("delay-family"))?,
                });
            }
            CheckedNodeKind::Probabilistic(ProbabilisticForm::Reward) => {
                read.rewards.push(PackageReward {
                    declaring: bound(body, 0)
                        .and_then(key)
                        .ok_or(fault("reward-declaring"))?,
                    operation: bound(body, 1)
                        .and_then(text)
                        .ok_or(fault("reward-operation"))?
                        .to_owned(),
                    name: bound(body, 2)
                        .and_then(text)
                        .ok_or(fault("reward-name"))?
                        .to_owned(),
                    value_type: bound(body, 3)
                        .and_then(key)
                        .ok_or(fault("reward-value-type"))?,
                    function: bound(body, 4)
                        .and_then(key)
                        .ok_or(fault("reward-expression"))?,
                });
            }
            _ => {}
        }
    }
    Ok(read)
}

/// The family of a `delay_distribution` body and its parameters.
fn delay_family(body: &CheckedTerm) -> Option<PackageDelayFamily> {
    let parameters = bound(body, 3)?;
    Some(match text(bound(body, 2)?)? {
        "uniform" => PackageDelayFamily::Uniform,
        "uniform_interval" => PackageDelayFamily::UniformInterval {
            lower: rational(bound(parameters, 0)?)?,
            upper: rational(bound(parameters, 1)?)?,
        },
        "exponential" => PackageDelayFamily::Exponential {
            rate: rational(bound(parameters, 0)?)?,
        },
        "discrete" => PackageDelayFamily::Discrete(
            members(parameters)?
                .iter()
                .map(|row| Some((rational(bound(row, 0)?)?, rational(bound(row, 1)?)?)))
                .collect::<Option<Vec<_>>>()?,
        ),
        _ => return None,
    })
}

/// A package's source-object rows do not describe a source model a
/// probabilistic node can join to.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum SourceModelFault {
    /// A parameter's type node projects no Exact type.
    #[error("the type of parameter `{parameter}` of `{operation}` does not project")]
    UnprojectedParameter {
        /// The operation's source identity.
        operation: String,
        /// The parameter's name.
        parameter: String,
    },
    /// A row's direct supertype is the node of no source-object row.
    #[error("the supertype of `{object}` is no source object type of the package")]
    UnknownSupertype {
        /// The row's type name.
        object: String,
    },
    /// The package has no joined identity rows to read the source objects from.
    #[error("the package has no joined identity rows")]
    NoIdentityRows,
    /// A row's operation identity is not `<type>::<operation>`.
    #[error("the operation identity `{0}` is not `<type>::<operation>`")]
    MalformedOperation(String),
}

impl BarePackage<'_> {
    /// FSL FR-740 "Source objects": the synthetic source package the
    /// package's source-object identity rows describe, which the reader joins
    /// its `probabilistic` nodes to as S3 joins them to the package it built
    /// ([`fsl_semantics::check::CheckedGraph::source_packages`]). `None` when
    /// the package declares no source object type. The rows carry no effect
    /// frame, field or result, so the package holds the types, their
    /// supertypes and their operations' parameters, which is what a random
    /// parameter, a workload and a delay read.
    ///
    /// # Errors
    ///
    /// A [`SourceModelFault`] when the rows do not describe such a package.
    pub fn source_model_package(
        &self,
    ) -> Result<Option<fsl_semantics::model::domain_package::DomainPackage>, SourceModelFault> {
        use fsl_semantics::check::{
            package_of_source_rows, SourceRowOperation, SourceRowParameter,
        };
        let (supply, _) = self
            .witness()
            .map_err(|_| SourceModelFault::NoIdentityRows)?;
        let rows = supply.identities.source_objects();
        let Some(first) = rows.first() else {
            return Ok(None);
        };
        let mut objects = self.source_row_shells()?;
        for (object, row) in objects.iter_mut().zip(rows) {
            let mut operations = Vec::with_capacity(row.operations.len());
            for operation in &row.operations {
                let name = operation
                    .identity
                    .split_once("::")
                    .map(|(_, name)| name)
                    .ok_or_else(|| {
                        SourceModelFault::MalformedOperation(operation.identity.to_string())
                    })?;
                let mut parameters = Vec::with_capacity(operation.parameters.len());
                for parameter in &operation.parameters {
                    let value_type = match &parameter.type_node {
                        None if operation.clock_parameters.contains(&parameter.name) => None,
                        None => Some(
                            parameter
                                .member_type
                                .as_deref()
                                .and_then(rendered_scalar)
                                .ok_or_else(|| SourceModelFault::UnprojectedParameter {
                                    operation: operation.identity.to_string(),
                                    parameter: parameter.name.to_string(),
                                })?,
                        ),
                        Some(node) => Some(
                            self.exact_type_projections()
                                .get(node)
                                .and_then(|projection| projection.as_ref().ok())
                                .map(|projection| projection.value_type().clone())
                                .ok_or_else(|| SourceModelFault::UnprojectedParameter {
                                    operation: operation.identity.to_string(),
                                    parameter: parameter.name.to_string(),
                                })?,
                        ),
                    };
                    parameters.push(SourceRowParameter {
                        name: parameter.name.to_string(),
                        optional: parameter.optional,
                        value_type,
                    });
                }
                operations.push(SourceRowOperation {
                    name: name.to_owned(),
                    parameters,
                });
            }
            object.operations = operations;
        }
        let identity = format!("source/{}", first.structural_node.package);
        Ok(Some(package_of_source_rows(&identity, &objects)))
    }
}

/// The value type a source operation's parameter row renders, for the scalar
/// types a parameter row spells without a type node: `Boolean`, `Integer`
/// and `Int[lower, upper]`. `None` for any other rendering, which the row
/// does not let a reader rebuild.
fn rendered_scalar(rendering: &str) -> Option<filament_exact::ValueType> {
    use filament_exact::{IntegerInterval, ValueType};
    match rendering.strip_prefix("resolved:")? {
        "Boolean" => Some(ValueType::Boolean),
        "Integer" => Some(ValueType::Integer),
        bounded => {
            let (lower, upper) = bounded
                .strip_prefix("Int[")?
                .strip_suffix(']')?
                .split_once(", ")?;
            let bound = |text: &str| text.parse::<i64>().ok().map(Integer::from);
            IntegerInterval::new(bound(lower)?, bound(upper)?)
                .ok()
                .map(ValueType::Int)
        }
    }
}

impl BarePackage<'_> {
    /// The source object types the supply's identity rows describe, by name
    /// and direct supertypes, in row order; empty when the package declares
    /// none or holds no identity rows. No operation is read.
    pub(crate) fn source_row_shells(
        &self,
    ) -> Result<Vec<fsl_semantics::check::SourceRowObject>, SourceModelFault> {
        let Ok((supply, _)) = self.witness() else {
            return Ok(Vec::new());
        };
        let rows = supply.identities.source_objects();
        let names: std::collections::BTreeMap<_, _> = rows
            .iter()
            .map(|row| (&row.structural_node, row.source_identity.as_ref()))
            .collect();
        rows.iter()
            .map(|row| {
                let supertypes = row
                    .supertypes
                    .iter()
                    .map(|node| {
                        names
                            .get(node)
                            .map(|name| (*name).to_owned())
                            .ok_or_else(|| SourceModelFault::UnknownSupertype {
                                object: row.source_identity.to_string(),
                            })
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(fsl_semantics::check::SourceRowObject {
                    name: row.source_identity.to_string(),
                    abstract_type: row.is_abstract,
                    supertypes,
                    operations: Vec::new(),
                })
            })
            .collect()
    }
}
