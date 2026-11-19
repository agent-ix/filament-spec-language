// SPDX-License-Identifier: AGPL-3.0-or-later
//! S4 emission and strict reading for FSL verification plans (FR-753).

use alloc::{collections::BTreeSet, string::String, vec::Vec};
use serde::{Deserialize, Serialize};

use crate::CheckedPackage;

/// The wire format identifier for an emitted verification plan.
pub const VERIFICATION_PLAN_FORMAT: &str = "filament.verification-plan/v1";

/// One source-ordered verification plan bound to its checked package.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VerificationPlan {
    /// The wire format identifier.
    pub format: String,
    /// The checked package identity that owns this plan.
    pub package: String,
    /// The authored plan name.
    pub name: String,
    /// The selected profile alias.
    pub profile: String,
    /// The plan's steps in source order.
    pub nodes: Vec<VerificationPlanNode>,
    /// Prerequisite edges, from dependent step to prerequisite step.
    pub edges: Vec<VerificationPlanEdge>,
}

/// One checked plan step.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VerificationPlanNode {
    /// The step's authored name.
    pub name: String,
    /// The checked claim name.
    pub claim: String,
    /// The selected evidence method.
    pub method: String,
    /// The verification domain name.
    pub domain: String,
    /// The optional finite method bound.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bound: Option<String>,
}

/// A `requires` edge in a verification plan graph.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VerificationPlanEdge {
    /// The dependent step.
    pub from: String,
    /// The prerequisite step.
    pub to: String,
    /// Always `requires`.
    #[serde(rename = "type")]
    pub edge_type: String,
}

/// An S4 plan emission or strict-reader refusal.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum VerificationPlanRefusal {
    /// The checked package has no plan with the requested name.
    #[error("verification plan {name:?} is not declared in package {package:?}")]
    PlanNotFound { package: String, name: String },
    /// The supplied document is malformed or violates the plan graph shape.
    #[error("invalid verification plan document: {reason}")]
    InvalidDocument { reason: String },
    /// The document is bound to a different checked package.
    #[error("verification plan package identity mismatch: expected {expected:?}, got {actual:?}")]
    PackageIdentityMismatch { expected: String, actual: String },
}

/// Emit each checked plan as its own `filament.verification-plan/v1`
/// document, retaining the checked package identity.
pub fn emit_verification_plans(package: &CheckedPackage) -> Vec<VerificationPlan> {
    package
        .graph()
        .verification_plans()
        .iter()
        .map(|plan| VerificationPlan {
            format: VERIFICATION_PLAN_FORMAT.into(),
            package: package.graph().package_name().into(),
            name: plan.name.name.clone(),
            profile: plan.profile.alias.clone(),
            nodes: plan
                .steps
                .iter()
                .map(|step| VerificationPlanNode {
                    name: step.name.name.clone(),
                    claim: step.claim.name.clone(),
                    method: step.method.name.clone(),
                    domain: step.domain.name.clone(),
                    bound: step.bound.clone(),
                })
                .collect(),
            edges: plan
                .steps
                .iter()
                .flat_map(|step| {
                    step.depends.iter().map(|dependency| VerificationPlanEdge {
                        from: step.name.name.clone(),
                        to: dependency.name.clone(),
                        edge_type: "requires".into(),
                    })
                })
                .collect(),
        })
        .collect()
}

/// Read and validate a plan document before any coordinator can execute it.
/// Unknown fields, an unsupported format, malformed graph edges and a package
/// identity mismatch are refused.
pub fn read_verification_plan(
    package: &CheckedPackage,
    document: &[u8],
) -> Result<VerificationPlan, VerificationPlanRefusal> {
    let plan: VerificationPlan = serde_json::from_slice(document).map_err(|error| {
        VerificationPlanRefusal::InvalidDocument {
            reason: error.to_string(),
        }
    })?;
    if plan.format != VERIFICATION_PLAN_FORMAT {
        return Err(VerificationPlanRefusal::InvalidDocument {
            reason: alloc::format!("unsupported format {:?}", plan.format),
        });
    }
    let expected = package.graph().package_name();
    if plan.package != expected {
        return Err(VerificationPlanRefusal::PackageIdentityMismatch {
            expected: expected.into(),
            actual: plan.package,
        });
    }
    validate_graph(&plan)?;
    Ok(plan)
}

fn validate_graph(plan: &VerificationPlan) -> Result<(), VerificationPlanRefusal> {
    let mut names = BTreeSet::new();
    for node in &plan.nodes {
        if node.name.is_empty() || !names.insert(node.name.as_str()) {
            return Err(VerificationPlanRefusal::InvalidDocument {
                reason: alloc::format!("empty or duplicate node name {:?}", node.name),
            });
        }
    }
    let mut edges = BTreeSet::new();
    for edge in &plan.edges {
        if edge.edge_type != "requires"
            || !names.contains(edge.from.as_str())
            || !names.contains(edge.to.as_str())
            || edge.from == edge.to
            || !edges.insert((edge.from.as_str(), edge.to.as_str()))
        {
            return Err(VerificationPlanRefusal::InvalidDocument {
                reason: alloc::format!("invalid requires edge {:?} -> {:?}", edge.from, edge.to),
            });
        }
    }
    Ok(())
}
