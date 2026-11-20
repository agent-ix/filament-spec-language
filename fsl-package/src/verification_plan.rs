// SPDX-License-Identifier: AGPL-3.0-or-later
//! S4 emission and strict reading for FSL verification plans (FR-753).

use alloc::{collections::BTreeSet, string::String, vec, vec::Vec};
use filament_contract_model::CheckedOccurrence;
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
    /// Plan declaration identity and source location.
    pub plan: VerificationPlanIdentity,
    /// The selected profile alias.
    pub profile: String,
    /// The plan's steps in source order.
    pub nodes: Vec<VerificationPlanNode>,
    /// Prerequisite edges, from dependent step to prerequisite step.
    pub edges: Vec<VerificationPlanEdge>,
}

/// Plan name, package-local qualified name, and declaration location.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VerificationPlanIdentity {
    /// Authored name.
    pub name: String,
    /// Declaring scopes followed by the plan name.
    pub qualified_name: Vec<String>,
    /// Source document and half-open byte interval.
    pub source_location: VerificationPlanSourceLocation,
}

/// Source position of the plan declaration.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VerificationPlanSourceLocation {
    /// Source document identity.
    pub source: VerificationPlanSource,
    /// Inclusive start byte.
    pub start: u64,
    /// Exclusive end byte.
    pub end: u64,
}

/// Source document identity.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VerificationPlanSource {
    /// Source authority.
    pub authority: String,
    /// Source identity.
    pub identity: String,
}

/// One checked plan step.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VerificationPlanNode {
    /// The step's authored name.
    pub name: String,
    /// Bound checked clause identity and claim occurrence.
    pub claim: VerificationPlanClaim,
    /// Method semantics negotiated from FR-192's closed method vocabulary.
    pub method: VerificationPlanMethod,
    /// The resolved domain declaration key.
    pub domain: VerificationPlanDeclarationKey,
    /// The optional finite method bound.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bound: Option<String>,
}

/// A clause node identity and its claim occurrence in the checked package.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VerificationPlanClaim {
    /// Structural checked node identity.
    pub node: VerificationPlanNodeIdentity,
    /// Claim occurrence role and stable ordinal.
    pub occurrence: CheckedOccurrence,
}

/// Recursive structural node identity emitted by S4 before package graph
/// indexing. Its shape is deliberately distinct from the indexed checked
/// package node ID consumed by the package reader.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VerificationPlanNodeIdentity {
    /// Owning checked package identity.
    pub package: String,
    /// Structural path segments.
    pub path: Vec<VerificationPlanPathSegment>,
    /// Position among nodes sharing the package and path.
    pub ordinal: u64,
}

/// One structural path segment of a plan-bound checked node.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum VerificationPlanPathSegment {
    /// Authored or derived name segment.
    Name(String),
    /// Operand node structural identity.
    Node(Box<VerificationPlanNodeIdentity>),
}

/// Method identity and FR-290 semantics.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VerificationPlanMethod {
    /// Closed method name.
    pub name: String,
    /// FR-290 method kind.
    pub kind: String,
    /// FR-290 extent classification.
    pub extent: String,
    /// Evidence kinds the method can produce for the selected claim.
    pub evidence_kind: Vec<String>,
}

/// Resolved checked declaration key for a plan domain.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VerificationPlanDeclarationKey {
    /// Domain package identity.
    pub package: String,
    /// Checked declaration node identity.
    pub node: String,
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
    /// The supplied document is malformed or violates the plan graph shape.
    #[error("invalid verification plan document: {reason}")]
    InvalidDocument {
        /// Parsing or validation failure.
        reason: String,
    },
    /// The document is bound to a different checked package.
    #[error("verification plan package identity mismatch: expected {expected:?}, got {actual:?}")]
    PackageIdentityMismatch {
        /// Identity of the package being executed.
        expected: String,
        /// Identity carried by the plan document.
        actual: String,
    },
}

/// Emit each checked plan as its own `filament.verification-plan/v1`
/// document, retaining all package and declaration bindings from S3.
pub fn emit_verification_plans(package: &CheckedPackage) -> Vec<VerificationPlan> {
    package
        .graph()
        .verification_plans()
        .iter()
        .enumerate()
        .map(|(plan_index, plan)| {
            let source = package
                .graph()
                .verification_plan_source(&plan.name.name)
                .unwrap_or_else(|| package.graph().source());
            VerificationPlan {
                format: VERIFICATION_PLAN_FORMAT.into(),
                package: package.graph().package_name().into(),
                plan: VerificationPlanIdentity {
                    name: plan.name.name.clone(),
                    qualified_name: vec![plan.name.name.clone()],
                    source_location: VerificationPlanSourceLocation {
                        source: VerificationPlanSource {
                            authority: source.authority().into(),
                            identity: source.identity().into(),
                        },
                        start: plan.span.start as u64,
                        end: plan.span.end as u64,
                    },
                },
                profile: plan.profile.alias.clone(),
                nodes: plan
                    .steps
                    .iter()
                    .enumerate()
                    .map(|(step_index, step)| {
                        let (node, origin) = claim_site(package, &step.claim.name)
                            .expect("S3 accepted this plan claim");
                        let key = package
                            .graph()
                            .verification_plan_domain_key(plan_index, step_index)
                            .expect("S3 accepted and resolved this model domain");
                        VerificationPlanNode {
                            name: step.name.name.clone(),
                            claim: VerificationPlanClaim {
                                node: serde_json::from_value(
                                    serde_json::to_value(
                                        fsl_semantics::check::wire_id::structural_id(&node),
                                    )
                                    .expect("structural node identity serializes"),
                                )
                                .expect("structural node identity has the FR-192 shape"),
                                occurrence: CheckedOccurrence {
                                    role: serde_json::from_value(serde_json::Value::String(
                                        origin.role().as_str().into(),
                                    ))
                                    .expect("an occurrence role is a checked occurrence role"),
                                    ordinal: origin.ordinal(),
                                },
                            },
                            method: method_semantics(&step.method.name, step.bound.is_some()),
                            domain: VerificationPlanDeclarationKey {
                                package: key.package.clone(),
                                node: key.node.clone(),
                            },
                            bound: step.bound.clone(),
                        }
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
            }
        })
        .collect()
}

/// The checked node and claim occurrence of the claim S3 resolved by `name`
/// (FSpec FR-192 "Source form"): a temporal clause, a state clause, a
/// state-graph, hybrid, hyper, refinement or probabilistic claim.
fn claim_site(
    package: &CheckedPackage,
    name: &str,
) -> Option<(filament_exact::NodeKey, filament_exact::Origin)> {
    let graph = package.graph();
    let split = |key: &fsl_foundation::source::provenance::OccurrenceKey| {
        (key.node().clone(), key.origin().clone())
    };
    if let Some(clause) = graph
        .temporal_clauses()
        .iter()
        .find(|clause| clause.clause().name() == name)
    {
        return Some((clause.identity(), clause.claim().clone()));
    }
    if let Some(clause) = graph.state_clause(name) {
        return Some((clause.identity(), clause.claim().clone()));
    }
    if let Some(site) = graph
        .hybrid_claim_sites()
        .iter()
        .find(|site| site.name == name)
    {
        return Some((site.identity.clone(), site.claim.clone()));
    }
    graph
        .state_graph_claims()
        .iter()
        .find(|claim| claim.name() == name)
        .and_then(|claim| claim.occurrence())
        .or_else(|| {
            graph
                .hyper_clauses()
                .iter()
                .find(|clause| clause.name() == name)
                .and_then(|clause| clause.assembled_occurrence())
        })
        .or_else(|| {
            graph
                .refinements()
                .iter()
                .find(|refinement| refinement.name() == name)
                .and_then(|refinement| refinement.requirement_occurrence())
        })
        .or_else(|| {
            graph
                .probabilistic_model()
                .claims()
                .iter()
                .find(|claim| claim.name == name)
                .and_then(|claim| claim.occurrence())
        })
        .map(split)
}

fn method_semantics(name: &str, bounded: bool) -> VerificationPlanMethod {
    let (kind, evidence_kind) = match name {
        "model_check" => ("temporal-satisfaction", vec!["proof", "refutation"]),
        "contract_check" => ("operation-contract", vec!["proof", "refutation"]),
        "value_check" => ("value-validity", vec!["proof", "refutation"]),
        "probabilistic_exact" => ("probabilistic-satisfaction", vec!["exact"]),
        "probabilistic_statistical" => ("probabilistic-satisfaction", vec!["statistical"]),
        "hybrid_solver" => ("temporal-satisfaction", vec!["solver-result"]),
        "schedulability" => ("schedulability", vec!["proof", "refutation"]),
        "model_based_test" => ("temporal-satisfaction", vec!["tested", "refutation"]),
        "simulate" => ("temporal-satisfaction", vec!["simulation-runs"]),
        "replay" => ("finite-replay", vec!["replay-disposition"]),
        _ => unreachable!("S3 admits only the closed verification method vocabulary"),
    };
    VerificationPlanMethod {
        name: name.into(),
        kind: kind.into(),
        extent: if bounded { "bounded" } else { "unbounded" }.into(),
        evidence_kind: evidence_kind.into_iter().map(Into::into).collect(),
    }
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
