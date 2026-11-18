// SPDX-License-Identifier: AGPL-3.0-or-later
//! FSL FR-740: the random parameters, workloads and delays of an admitted
//! package, read from its `probabilistic` nodes.
//!
//! The IR's typed read ([`CheckedPackageV2::random_parameters`],
//! [`CheckedPackageV2::workloads`]) is keyed by a model node and an
//! operation, so a reader that does not know either walks the nodes itself;
//! this reads each node's admitted body. The result names nodes and spells
//! values: [`fsl_semantics::model::state::StateModel::probabilistic_model`]
//! joins it to the selected domain packages and rebuilds S3's records.

use filament_contract_model::{
    CheckedLiteralValue, CheckedNodeKind, CheckedPackageV2, CheckedTerm, LiteralKind,
    ProbabilisticForm,
};
use filament_exact::{Integer, Rational};
use fsl_semantics::check::{
    PackageDelay, PackageDelayFamily, PackageLiteral, PackageProbabilistic, PackageRandomParameter,
    PackageSupport, PackageWeight, PackageWorkload,
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
