// SPDX-License-Identifier: AGPL-3.0-or-later
//! Verification-plan execution between analysis and monitoring (FR-753).
//!
//! The coordinator preserves source ordering in its output, while running
//! prerequisites before dependents. It never selects a negotiated engine.

use std::collections::BTreeMap;
use std::num::NonZeroU64;

use filament_exact::Cancel;
use filament_semantic_value::catalog::{CatalogCode, CatalogCoded, InternalFault};
use fsl_foundation::diagnostic::{StageFailure, Staged};
use fsl_foundation::source::provenance::OccurrenceKey;
use fsl_foundation::RequestIndex;
use fsl_package::verification_plan::{
    read_verification_plan, VerificationPlan, VerificationPlanNode, VerificationPlanRefusal,
};
use fsl_package::CheckedPackage;
use fsl_replay::{TerminalRecord, TerminalValue};
use fsl_route::request::ExtentClassification;
use fsl_route::{CandidateOutcome, EvidenceKind, ItemEvidence, Registry};
use fsl_semantics::check::Capability;

use crate::analyze::document::Negotiate;
use crate::analyze::{
    analyze, AnalyzeLimits, AnalyzeRequest, ClaimItem, Engines, HybridItem, ItemForm,
    NegotiatedItem, ProbabilisticItem, ProbabilisticLimits, SourceProvision, SubjectBinding,
    TemporalItem,
};

/// Request inputs, with a separate admitted snapshot binding for each step.
///
/// `bindings` also supplies the domain packages, documents and finite key
/// universes used by the existing analyze and simulation entry points.
/// Bounds retain absence: the coordinator never supplies a generation default.
pub struct PlanInputs<'a> {
    /// Per-step subject bindings, indexed by authored step name.
    pub bindings: BTreeMap<String, SubjectBinding<'a>>,
    /// Source bytes under the package's declared labels.
    pub provision: SourceProvision,
    /// The request's generation seed, absent when omitted.
    pub seed: Option<u64>,
    /// The request's model-based trace bound.
    pub max_trace_length: Option<u64>,
    /// The request's unbounded-value generation size.
    pub max_value_bits: Option<u64>,
}

/// A whole-plan preflight refusal; no step has executed.
#[derive(Debug, thiserror::Error)]
pub enum PlanRefusal {
    /// The strict plan reader's refusal, including package identity mismatch.
    #[error(transparent)]
    Reader(#[from] VerificationPlanRefusal),
    /// None of the consuming models admits the supplied initial state.
    #[error("no consuming model admits initial: {0:?}")]
    Initial(fsl_semantics::model::observation::AdmissionFailure),
    /// An absent generation bound, naming the first consuming step.
    #[error("missing {member} consumed by step {step}")]
    MissingMember {
        /// The absent member.
        member: &'static str,
        /// First consumer in source order.
        step: String,
    },
}

impl CatalogCoded for PlanRefusal {
    fn catalog_code(&self) -> CatalogCode {
        match self {
            Self::MissingMember { .. } => CatalogCode::new("invalid_runtime_input", "missing-member"),
            Self::Reader(_) => CatalogCode::new("invalid_runtime_input", "invalid-value"),
            Self::Initial(failure) => failure.catalog_code(),
        }
    }
}

/// Closed FR-824 status, with the measured decision retained by its record.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PlanStatus {
    /// A proof.
    Proved,
    /// A counterexample.
    Refuted,
    /// Generated tests evaluated successfully.
    Tested,
    /// A statistical verdict.
    Measured,
    /// No conclusive result.
    Inconclusive,
    /// No supporting engine or capability.
    Unsupported,
    /// The item declined execution.
    Declined,
    /// A budget or cancellation stopped execution.
    Incomplete,
    /// An internal failure.
    Failed,
    /// An unsatisfied prerequisite prevented execution.
    Blocked,
}

/// One node's disposition. The step name is held by `PlanOutcome`.
#[derive(Debug)]
pub struct PlanDisposition {
    /// Closed FR-331 status or `blocked`.
    pub status: PlanStatus,
    /// Authored method.
    pub method: String,
    /// Produced evidence kind, never substituted for another kind.
    pub evidence: Option<String>,
    /// First unsatisfied prerequisite in source order.
    pub blocked_by: Option<String>,
    /// The settled analysis record, absent only on a blocked or simulation step.
    pub result: Option<TerminalRecord>,
    /// FR-181 simulation runs, kept separately from proof records.
    pub simulation: Option<serde_json::Value>,
}

impl PlanDisposition {
    fn satisfies_requires(&self) -> bool {
        match self.status {
            PlanStatus::Proved | PlanStatus::Tested => true,
            PlanStatus::Measured => matches!(
                self.result.as_ref().map(TerminalRecord::value),
                Some(TerminalValue::Measured(verdict))
                    if matches!(verdict.decision, fsl_replay::statistical::Decision::Accepted)
            ),
            _ => false,
        }
    }
}

/// One disposition per node in source order; the coordinator renders nothing.
#[derive(Debug)]
pub struct PlanOutcome {
    /// Authored step names paired with their dispositions.
    pub dispositions: Vec<(String, PlanDisposition)>,
}

fn fault(site: &'static str) -> StageFailure<PlanRefusal> {
    StageFailure::Fault(InternalFault::new("verification-plan", site))
}

/// Run the selected plan, checking its identity before any request inputs.
///
/// # Errors
/// Returns a whole-plan `PlanRefusal` before any step runs. Internal engine
/// adapter failures remain stage failures. The model-based-test integration
/// requires the FR-752 entry point (QSL-2237); see `model_based_test` below.
pub fn execute_plan(
    package: &CheckedPackage,
    plan: &VerificationPlan,
    inputs: &PlanInputs<'_>,
    engines: &Engines,
    negotiate: &dyn Negotiate,
    limits: &AnalyzeLimits,
    cancel: &Cancel,
) -> Result<Staged<PlanOutcome>, StageFailure<PlanRefusal>> {
    // Re-admit direct callers as well as callers of the document reader.
    let bytes = serde_json::to_vec(plan).map_err(|_| fault("plan-serialization"))?;
    read_verification_plan(package, &bytes)
        .map_err(|cause| StageFailure::Refused(PlanRefusal::Reader(cause)))?;
    let order = execution_order(plan)?;
    let input = fsl_eval::simulation::model::ModelCheckInput::source(package);
    let mut admitted_initial = false;
    let mut initial_failure = None;
    let mut value_consumer = None;
    for node in &plan.nodes {
        if let Some(binding) = inputs.bindings.get(&node.name) {
            if consumes_initial(node) {
                match fsl_eval::simulation::model::ModelSystem::new(
                    crate::analyze::request::simulation(&input, binding, limits),
                ) {
                    Ok(system) => {
                        admitted_initial |= !binding.initial.is_empty();
                        if node.method.name == "model_based_test"
                            && value_consumer.is_none()
                            && matches!(
                                fsl_eval::simulation::model::check_model_bounds(
                                    &system, fsl_eval::simulation::BOUND_POSITIONS,
                                ),
                                Err(fsl_eval::simulation::NotSimulated::RequiresBound { .. })
                            )
                        {
                            value_consumer = Some(node.name.clone());
                        }
                    }
                    Err(cause) if !binding.initial.is_empty() => {
                        initial_failure.get_or_insert(cause);
                    }
                    Err(_) => {}
                }
            }
        }
    }
    if !admitted_initial {
        if let Some(cause) = initial_failure {
            return Err(StageFailure::Refused(PlanRefusal::Initial(cause)));
        }
    }
    if inputs.max_trace_length.is_none() {
        if let Some(node) = plan.nodes.iter().find(|node| {
            node.method.name == "model_based_test" && consumes_initial(node)
        }) {
            return Err(StageFailure::Refused(PlanRefusal::MissingMember {
                member: "max_trace_length", step: node.name.clone(),
            }));
        }
    }
    if inputs.max_value_bits.is_none() {
        if let Some(step) = value_consumer {
            return Err(StageFailure::Refused(PlanRefusal::MissingMember {
                member: "max_value_bits", step,
            }));
        }
    }
    let mut registry = Registry::new();
    for descriptor in engines.registrations() {
        registry.register(descriptor.clone()).map_err(|_| fault("engine-registration"))?;
    }
    let mut settled: BTreeMap<usize, PlanDisposition> = BTreeMap::new();
    for index in order {
        let node = &plan.nodes[index];
        let blocked = plan.nodes.iter().enumerate().find(|(at, prerequisite)| {
            plan.edges.iter().any(|edge| edge.from == node.name && edge.to == prerequisite.name)
                && settled.get(at).is_some_and(|step| !step.satisfies_requires())
        });
        let disposition = if let Some((_, prerequisite)) = blocked {
            PlanDisposition {
                status: PlanStatus::Blocked, method: node.method.name.clone(), evidence: None,
                blocked_by: Some(prerequisite.name.clone()), result: None, simulation: None,
            }
        } else {
            run_step(package, node, index, inputs, engines, &registry, negotiate, limits, cancel)?
        };
        settled.insert(index, disposition);
    }
    let dispositions = settled.into_iter().map(|(index, disposition)| {
        (plan.nodes[index].name.clone(), disposition)
    }).collect();
    Ok(Staged::new(PlanOutcome { dispositions }))
}

fn consumes_initial(node: &VerificationPlanNode) -> bool {
    matches!(node.method.name.as_str(), "model_check" | "model_based_test" | "simulate" | "contract_check")
}

fn execution_order(plan: &VerificationPlan) -> Result<Vec<usize>, StageFailure<PlanRefusal>> {
    let mut order = Vec::new();
    while order.len() < plan.nodes.len() {
        let next = plan.nodes.iter().enumerate().find(|(index, node)| {
            !order.contains(index) && plan.edges.iter().filter(|edge| edge.from == node.name)
                .all(|edge| order.iter().any(|at| plan.nodes[*at].name == edge.to))
        });
        let Some((index, _)) = next else {
            return Err(StageFailure::Refused(PlanRefusal::Reader(
                VerificationPlanRefusal::InvalidDocument { reason: "requires cycle".into() },
            )));
        };
        order.push(index);
    }
    Ok(order)
}

fn claim(package: &CheckedPackage, node: &VerificationPlanNode) -> Option<OccurrenceKey> {
    package.graph().occurrences().find_map(|(key, origin, _)| {
        let structural = serde_json::to_value(fsl_semantics::check::wire_id::structural_id(&key)).ok()?;
        let requested = serde_json::to_value(&node.claim.node).ok()?;
        (structural == requested && origin.role().as_str() == "claim"
            && origin.ordinal() == node.claim.occurrence.ordinal)
            .then(|| OccurrenceKey::new(key, origin))
    })
}

fn status(record: &TerminalRecord) -> PlanStatus {
    match record.value() {
        TerminalValue::Proved { .. } => PlanStatus::Proved,
        TerminalValue::Refuted => PlanStatus::Refuted,
        TerminalValue::Tested => PlanStatus::Tested,
        TerminalValue::Measured(_) => PlanStatus::Measured,
        TerminalValue::Declined { .. } => PlanStatus::Declined,
        TerminalValue::Unsupported(_) => PlanStatus::Unsupported,
        TerminalValue::Incomplete(_) => PlanStatus::Incomplete,
        TerminalValue::Inconclusive(_) => PlanStatus::Inconclusive,
        TerminalValue::Failed | TerminalValue::ToolFailed(_) => PlanStatus::Failed,
    }
}

fn model_based_test(
    _package: &CheckedPackage,
    _node: &VerificationPlanNode,
    _inputs: &PlanInputs<'_>,
    _limits: &AnalyzeLimits,
    _cancel: &Cancel,
) -> Result<PlanDisposition, StageFailure<PlanRefusal>> {
    // QSL-2237 owns the FR-752 fsl-eval entry. Replace only this adapter
    // with that call, passing clause, domain, bound as max_cases, seed and
    // the consumed trace/value bounds. Its FR-190 record is the result;
    // no negotiated engine, synthesized test result or alternate route is valid.
    Err(fault("model-based-test-entry-unavailable-QSL-2237"))
}

fn run_step(
    package: &CheckedPackage,
    node: &VerificationPlanNode,
    index: usize,
    inputs: &PlanInputs<'_>,
    engines: &Engines,
    registry: &Registry,
    negotiate: &dyn Negotiate,
    limits: &AnalyzeLimits,
    cancel: &Cancel,
) -> Result<PlanDisposition, StageFailure<PlanRefusal>> {
    if node.method.name == "model_based_test" {
        return model_based_test(package, node, inputs, limits, cancel);
    }
    let binding = inputs.bindings.get(&node.name).ok_or_else(|| fault("missing-step-binding"))?;
    let bound = node.bound.as_deref().map(str::parse::<u64>).transpose()
        .map_err(|_| fault("invalid-method-bound"))?;
    if node.method.name == "simulate" {
        return simulate(package, node, index, binding, bound, limits, cancel);
    }
    let (kind, form, evidence) = match node.method.name.as_str() {
        "model_check" => (Capability::TemporalSatisfaction,
            Some(ItemForm::Temporal(TemporalItem {
                max_depth: bound.map(usize::try_from).transpose().map_err(|_| fault("method-bound-overflow"))?,
                ..TemporalItem::default()
            })), None),
        "contract_check" => (Capability::OperationContract, None, None),
        "value_check" => (Capability::ValueValidity, None, None),
        "probabilistic_exact" => (Capability::ProbabilisticSatisfaction,
            Some(ItemForm::Probabilistic(probabilistic(false, inputs.seed)?)), Some(EvidenceKind::Exact)),
        "probabilistic_statistical" => (Capability::ProbabilisticSatisfaction,
            Some(ItemForm::Probabilistic(probabilistic(true, inputs.seed)?)), Some(EvidenceKind::Statistical)),
        "hybrid_solver" => (Capability::TemporalSatisfaction,
            Some(ItemForm::Hybrid(HybridItem::default())), None),
        "schedulability" => (Capability::Schedulability, None, None),
        "replay" => (Capability::FiniteReplay, None, None),
        _ => return Err(fault("unknown-plan-method")),
    };
    let key = claim(package, node).ok_or_else(|| fault("unknown-plan-claim"))?;
    let mut item = ClaimItem::new(package, RequestIndex::new(index), key, kind,
        if bound.is_some() { ExtentClassification::Bounded } else {
            ExtentClassification::Unbounded { finite_bound_available: false }
        }).map_err(|_| fault("inapplicable-plan-claim"))?;
    item.form = form;
    item.options.max_cases = bound.and_then(NonZeroU64::new);
    item.options.required = true;
    let candidates = if let Some(evidence) = evidence {
        registry.candidates_for_evidence(kind, &ItemEvidence::Kind(evidence), None)
    } else {
        registry.candidates_for_dynamics(kind, None, node.method.name == "hybrid_solver")
    };
    if let CandidateOutcome::Candidates(set) = &candidates {
        item.candidates = set.candidates().to_vec();
    }
    let (disposition, diagnostic) = negotiate.negotiate(&item, &candidates);
    let request = AnalyzeRequest {
        binding: binding.clone(), provision: inputs.provision.clone(),
        items: vec![NegotiatedItem { item, disposition, diagnostic }],
    };
    let outcome = analyze(package, &request, engines, limits, cancel)
        .map_err(|_| fault("analyze-adapter"))?.into_value();
    let record = outcome.records.into_iter().next().ok_or_else(|| fault("missing-step-record"))?;
    let status = status(&record);
    let evidence = match status {
        PlanStatus::Proved => Some(if node.method.name == "replay" { "replay-disposition" } else { "proof" }),
        PlanStatus::Refuted => Some("refutation"),
        PlanStatus::Measured => Some("statistical"),
        _ => None,
    }.map(str::to_owned);
    Ok(PlanDisposition { status, method: node.method.name.clone(), evidence,
        blocked_by: None, result: Some(record), simulation: None })
}

fn probabilistic(statistical: bool, seed: Option<u64>) -> Result<ProbabilisticItem, StageFailure<PlanRefusal>> {
    Ok(ProbabilisticItem {
        evidence: if statistical { EvidenceKind::Statistical } else { EvidenceKind::Exact },
        limits: if statistical {
            ProbabilisticLimits::Statistical(fsl_replay::statistical::StatisticalLimits::default())
        } else { ProbabilisticLimits::Exact(crate::exact_probabilistic::ExactProbLimits::default()) },
        method: statistical.then_some(fsl_replay::statistical::StatisticalMethod::Okamoto),
        seed: statistical.then_some(seed.unwrap_or(0)),
        sampler: if statistical {
            Some(fsl_foundation::selection::DefinitionRef::new("agent-ix", "filament.simulation.sampler/v1")
                .map_err(|_| fault("sampler-identity"))?)
        } else { None },
        max_witnesses: None, symmetry: Vec::new(), timed_sampling: None,
    })
}

fn simulate(
    package: &CheckedPackage, node: &VerificationPlanNode, index: usize,
    binding: &SubjectBinding<'_>, bound: Option<u64>, limits: &AnalyzeLimits, cancel: &Cancel,
) -> Result<PlanDisposition, StageFailure<PlanRefusal>> {
    let item = crate::analyze::SimulationItem {
        index: RequestIndex::new(index), subject: None,
        settings: crate::analyze::SimulationSelection {
            mode: crate::analyze::SimulationMode::Enumerate,
            seed: None, samples: None, max_steps: bound.and_then(NonZeroU64::new),
            max_states: None, max_state_key_bytes: None,
        },
    };
    let (runs, category) = crate::analyze::simulation_item::simulate(package, binding, limits, &item, cancel)
        .map_err(|_| fault("simulation-adapter"))?;
    Ok(PlanDisposition {
        status: if category == filament_semantic_value::catalog::Category::Success {
            PlanStatus::Tested
        } else { PlanStatus::Incomplete },
        method: node.method.name.clone(), evidence: Some("simulation-runs".into()),
        blocked_by: None, result: None, simulation: Some(runs),
    })
}
