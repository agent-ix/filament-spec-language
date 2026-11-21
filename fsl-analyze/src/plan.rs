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
    fn catalog_fields(&self) -> Option<BTreeMap<&'static str, String>> {
        match self {
            Self::MissingMember { member, step } => Some(BTreeMap::from([
                ("member", (*member).to_owned()),
                ("step", step.clone()),
            ])),
            Self::Reader(VerificationPlanRefusal::PackageIdentityMismatch { expected, actual }) => {
                Some(BTreeMap::from([
                    ("expected", expected.clone()),
                    ("actual", actual.clone()),
                ]))
            }
            Self::Reader(VerificationPlanRefusal::InvalidDocument { .. }) => None,
            Self::Initial(failure) => match failure {
                fsl_semantics::model::observation::AdmissionFailure::Refused(record)
                | fsl_semantics::model::observation::AdmissionFailure::Incomplete(record) => {
                    Some(record.fields.clone())
                }
                fsl_semantics::model::observation::AdmissionFailure::Fault(fault) => {
                    Some(BTreeMap::from([
                        ("stage", fault.stage().to_owned()),
                        ("invariant", fault.invariant().to_owned()),
                    ]))
                }
            },
        }
    }
    fn catalog_code(&self) -> CatalogCode {
        match self {
            Self::MissingMember { .. } => {
                CatalogCode::new("invalid_runtime_input", "missing-member")
            }
            Self::Reader(_) => CatalogCode::new("invalid_runtime_input", "invalid-value"),
            Self::Initial(failure) => match failure {
                fsl_semantics::model::observation::AdmissionFailure::Refused(record)
                | fsl_semantics::model::observation::AdmissionFailure::Incomplete(record) => {
                    record.catalog
                }
                fsl_semantics::model::observation::AdmissionFailure::Fault(fault) => {
                    fault.catalog_code()
                }
            },
        }
    }
}

/// A `measured` step's FSpec FR-409 decision, read from its FR-410 verdict.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PlanDecision {
    /// The measurement met its acceptance bound.
    Accepted,
    /// The measurement missed it.
    Rejected,
    /// The measurement decided nothing.
    Undecided,
}

/// Closed FR-824 status: the FR-331 result vocabulary plus `blocked`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PlanStatus {
    /// A proof.
    Proved,
    /// A counterexample.
    Refuted,
    /// Generated tests evaluated successfully.
    Tested,
    /// A statistical verdict, with the decision its record carries.
    Measured(PlanDecision),
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

impl PlanStatus {
    /// Whether a step requiring this one may run (FSpec FR-824 "Outcome
    /// document"): `proved`, `tested`, or `measured` with decision
    /// `accepted`.
    #[must_use]
    pub const fn satisfies_requires(self) -> bool {
        matches!(
            self,
            Self::Proved | Self::Tested | Self::Measured(PlanDecision::Accepted)
        )
    }
}

/// A step's native result, without converting simulation runs into a proof.
#[derive(Debug)]
pub enum PlanResult {
    /// The negotiated item's FR-331 terminal record.
    Analysis(Box<TerminalRecord>),
    /// The simulation item's FR-181 runs.
    Simulation(serde_json::Value),
    /// The FR-752 entry's FR-190 record, whose `result` and `cause` carry
    /// the step's FR-331 settlement.
    ModelBasedTest(Box<fsl_eval::simulation::model::ModelBasedTestRecord>),
    /// A step that ran no engine: the FR-752 entry refused its request, or
    /// the step names no bound for a method with no unbounded extent.
    NotRun {
        /// The FR-805 catalog code.
        code: &'static str,
        /// The catalog cause.
        cause: &'static str,
        /// The request member at fault.
        member: &'static str,
    },
}

/// One node's five-member disposition. The step name is held by `PlanOutcome`.
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
    /// The step's native result; absent on a blocked step.
    pub result: Option<PlanResult>,
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
/// adapter failures remain stage failures.
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
    let consumers = plan_consumers(package, plan, inputs, limits)?;
    if inputs.max_trace_length.is_none() {
        if let Some(step) = consumers.max_trace_length.first().cloned() {
            return Err(StageFailure::Refused(PlanRefusal::MissingMember {
                member: "max_trace_length",
                step,
            }));
        }
    }
    if inputs.max_value_bits.is_none() {
        if let Some(step) = consumers.max_value_bits.first().cloned() {
            return Err(StageFailure::Refused(PlanRefusal::MissingMember {
                member: "max_value_bits",
                step,
            }));
        }
    }
    let mut registry = Registry::new();
    for descriptor in engines.registrations() {
        registry
            .register(descriptor.clone())
            .map_err(|_| fault("engine-registration"))?;
    }
    let execution = Execution {
        package,
        inputs,
        consumers: &consumers,
        engines,
        registry: &registry,
        negotiate,
        limits,
        cancel,
    };
    let dispositions = settle_in_order(plan, &order, |index| {
        execution.run_step(&plan.nodes[index], index)
    })?;
    Ok(Staged::new(PlanOutcome { dispositions }))
}

/// FSpec FR-824 "Outcome document": settle each node in `order` (every
/// prerequisite before its dependents). A node whose `requires`
/// prerequisite is not satisfied is `blocked` by the first such prerequisite
/// in source order and is never handed to `run`, so it is not negotiated and
/// runs no engine; every other node is settled by `run`. The dispositions
/// come back in source order.
fn settle_in_order(
    plan: &VerificationPlan,
    order: &[usize],
    mut run: impl FnMut(usize) -> Result<PlanDisposition, StageFailure<PlanRefusal>>,
) -> Result<Vec<(String, PlanDisposition)>, StageFailure<PlanRefusal>> {
    let mut settled: BTreeMap<usize, PlanDisposition> = BTreeMap::new();
    for &index in order {
        let node = plan.nodes.get(index).ok_or_else(|| fault("plan-order"))?;
        let blocked = plan.nodes.iter().enumerate().find(|(at, prerequisite)| {
            plan.edges
                .iter()
                .any(|edge| edge.from == node.name && edge.to == prerequisite.name)
                && settled
                    .get(at)
                    .is_some_and(|step| !step.status.satisfies_requires())
        });
        let disposition = if let Some((_, prerequisite)) = blocked {
            PlanDisposition {
                status: PlanStatus::Blocked,
                method: node.method.name.clone(),
                evidence: None,
                blocked_by: Some(prerequisite.name.clone()),
                result: None,
            }
        } else {
            run(index)?
        };
        settled.insert(index, disposition);
    }
    settled
        .into_iter()
        .map(|(index, disposition)| {
            let node = plan.nodes.get(index).ok_or_else(|| fault("plan-order"))?;
            Ok((node.name.clone(), disposition))
        })
        .collect()
}

/// The steps, in source order, that consume each request input (FSpec
/// FR-824 "Request"); empty when no step consumes it.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PlanConsumers {
    /// Each `model_check`, `model_based_test`, `simulate` or
    /// `contract_check` step whose claim's subject is a state model.
    pub initial: Vec<String>,
    /// Each `model_based_test` or `probabilistic_statistical` step.
    pub seed: Vec<String>,
    /// Each `model_based_test` step whose claim's subject is a state model.
    pub max_trace_length: Vec<String>,
    /// Each `model_based_test` step with a generation root or an operation
    /// result of no finite bound (FSpec FR-190 "Value domains").
    pub max_value_bits: Vec<String>,
}

/// The steps of `plan` that consume each request input, admitting each
/// consuming step's `initial` snapshot for its model on the way.
///
/// The request reader (FR-1038) closes the member set with this before it
/// runs the plan, and [`execute_plan`] checks a direct caller's inputs with
/// it.
///
/// # Errors
///
/// [`PlanRefusal::Initial`] when a supplied `initial` is admitted by no
/// consuming step's model; a stage fault when a step names a claim the
/// package does not hold.
pub fn plan_consumers(
    package: &CheckedPackage,
    plan: &VerificationPlan,
    inputs: &PlanInputs<'_>,
    limits: &AnalyzeLimits,
) -> Result<PlanConsumers, StageFailure<PlanRefusal>> {
    let input = fsl_eval::simulation::model::ModelCheckInput::source(package);
    let mut consumers = PlanConsumers::default();
    let mut admitted_initial = false;
    let mut initial_failure = None;
    for node in &plan.nodes {
        let tester = node.method.name == "model_based_test";
        let state_subject = consumes_initial(package, node);
        if tester || node.method.name == "probabilistic_statistical" {
            consumers.seed.push(node.name.clone());
        }
        if state_subject {
            consumers.initial.push(node.name.clone());
        }
        if tester && state_subject {
            consumers.max_trace_length.push(node.name.clone());
        }
        if tester && !state_subject && value_clause_consumes_bits(package, node)? {
            consumers.max_value_bits.push(node.name.clone());
        }
        let Some(binding) = inputs.bindings.get(&node.name) else {
            continue;
        };
        if !state_subject {
            continue;
        }
        match fsl_eval::simulation::model::ModelSystem::over_model(
            crate::analyze::request::simulation(&input, binding, limits),
            &node.domain.package,
        ) {
            Ok(system) => {
                admitted_initial |= !binding.initial.is_empty();
                if tester && consumes_value_bits(&system)? {
                    consumers.max_value_bits.push(node.name.clone());
                }
            }
            Err(cause) if !binding.initial.is_empty() => {
                initial_failure.get_or_insert(cause);
            }
            Err(_) => {}
        }
    }
    if !admitted_initial {
        if let Some(cause) = initial_failure {
            return Err(StageFailure::Refused(PlanRefusal::Initial(cause)));
        }
    }
    Ok(consumers)
}

fn consumes_initial(package: &CheckedPackage, node: &VerificationPlanNode) -> bool {
    let state_subject = claim(package, node)
        .and_then(|claim| {
            crate::claim_family::claim_family(
                package,
                fsl_route::request::ItemKind::Formula,
                &claim,
            )
        })
        .is_some_and(|family| {
            matches!(
                family,
                fsl_semantics::check::DeclarationFamily::StateModel
                    | fsl_semantics::check::DeclarationFamily::TemporalTrace
                    | fsl_semantics::check::DeclarationFamily::RelationClaim
            )
        });
    state_subject
        && matches!(
            node.method.name.as_str(),
            "model_check" | "model_based_test" | "simulate" | "contract_check"
        )
}

fn execution_order(plan: &VerificationPlan) -> Result<Vec<usize>, StageFailure<PlanRefusal>> {
    let mut order = Vec::new();
    while order.len() < plan.nodes.len() {
        let next = plan.nodes.iter().enumerate().find(|(index, node)| {
            !order.contains(index)
                && plan
                    .edges
                    .iter()
                    .filter(|edge| edge.from == node.name)
                    .all(|edge| order.iter().any(|at| plan.nodes[*at].name == edge.to))
        });
        let Some((index, _)) = next else {
            return Err(StageFailure::Refused(PlanRefusal::Reader(
                VerificationPlanRefusal::InvalidDocument {
                    reason: "requires cycle".into(),
                },
            )));
        };
        order.push(index);
    }
    Ok(order)
}

fn claim(package: &CheckedPackage, node: &VerificationPlanNode) -> Option<OccurrenceKey> {
    package.graph().occurrences().find_map(|(key, origin, _)| {
        let structural =
            serde_json::to_value(fsl_semantics::check::wire_id::structural_id(&key)).ok()?;
        let requested = serde_json::to_value(&node.claim.node).ok()?;
        let requested_role = serde_json::to_value(&node.claim.occurrence.role).ok()?;
        (structural == requested
            && requested_role.as_str() == Some(origin.role().as_str())
            && origin.ordinal() == node.claim.occurrence.ordinal)
            .then(|| OccurrenceKey::new(key, origin))
    })
}

fn status(record: &TerminalRecord) -> PlanStatus {
    match record.value() {
        TerminalValue::Proved {
            basis: fsl_replay::ProofBasis::Checks { success_checks: 0 },
            ..
        } => PlanStatus::Inconclusive,
        TerminalValue::Proved { .. } => PlanStatus::Proved,
        TerminalValue::Refuted => PlanStatus::Refuted,
        TerminalValue::Tested => PlanStatus::Tested,
        TerminalValue::Measured(verdict) => PlanStatus::Measured(match verdict.decision {
            fsl_replay::statistical::Decision::Accepted => PlanDecision::Accepted,
            fsl_replay::statistical::Decision::Rejected => PlanDecision::Rejected,
            fsl_replay::statistical::Decision::Undecided(_) => PlanDecision::Undecided,
        }),
        TerminalValue::Declined { .. } => PlanStatus::Declined,
        TerminalValue::Unsupported(_) => PlanStatus::Unsupported,
        TerminalValue::Incomplete(_) => PlanStatus::Incomplete,
        TerminalValue::Inconclusive(_) => PlanStatus::Inconclusive,
        TerminalValue::Failed | TerminalValue::ToolFailed(_) => PlanStatus::Failed,
    }
}

/// FR-753 "Dispatch per method": a `model_based_test` step runs on
/// `fsl-eval`'s FR-752 entry, never negotiated. The step's `bound` is
/// `max_cases`; the request's `seed`, `max_trace_length` and
/// `max_value_bits` reach it only where the step consumes them (FSpec
/// FR-824), and a clause over a state model runs from the step's admitted
/// `initial` binding. The FR-190 record is the step's result.
fn model_based_test(
    package: &CheckedPackage,
    node: &VerificationPlanNode,
    inputs: &PlanInputs<'_>,
    consumers: &PlanConsumers,
    limits: &AnalyzeLimits,
    cancel: &Cancel,
) -> Result<PlanDisposition, StageFailure<PlanRefusal>> {
    use fsl_eval::simulation::model::{
        run_model_based_test, ModelBasedTestEvidence, ModelBasedTestRequest, ModelBasedTestSubject,
        NativeAdmission,
    };
    let not_run = |code, cause, member| PlanDisposition {
        status: PlanStatus::Unsupported,
        method: node.method.name.clone(),
        evidence: None,
        blocked_by: None,
        result: Some(PlanResult::NotRun {
            code,
            cause,
            member,
        }),
    };
    // FSpec FR-192: a step with no `bound` asks for the method's unbounded
    // extent, and FR-190 has no run without `max_cases`.
    let Some(bound) = node.bound.as_deref() else {
        // FSpec FR-290 pairs `unsupported_projection` with `unbounded-extent`;
        // `requires-bound` is a negotiation outcome a language method never has.
        return Ok(not_run(
            "unsupported_projection",
            "unbounded-extent",
            "bound",
        ));
    };
    let max_cases = bound
        .parse::<u64>()
        .map_err(|_| fault("invalid-method-bound"))?;
    let key = claim(package, node).ok_or_else(|| fault("unknown-plan-claim"))?;
    let graph = package.graph();
    // A Boolean value clause is named by its function's body; the FR-752
    // entry takes the function's identity.
    let clause = graph
        .semantic_graph()
        .nodes()
        .find(|declaration| declaration.function_body().as_ref() == Some(key.node()))
        .map_or_else(|| key.node().clone(), |declaration| declaration.key());
    let state_subject = consumes_initial(package, node);
    // FSpec FR-824: a step that consumes `initial` and finds none starts from
    // every state its `domain` admits. The FR-752 entry draws initial states
    // from admitted snapshots only and cannot enumerate a domain's states, so
    // such a step settles `unsupported` with a stated cause, never `failed`.
    if state_subject
        && inputs
            .bindings
            .get(&node.name)
            .is_some_and(|binding| binding.initial.is_empty())
    {
        return Ok(not_run(
            "unsupported_projection",
            "unsupported-requested-capability",
            "initial",
        ));
    }
    let consumes = |steps: &[String]| steps.iter().any(|step| *step == node.name);
    let subject = if state_subject {
        let binding = inputs
            .bindings
            .get(&node.name)
            .ok_or_else(|| fault("missing-step-binding"))?;
        Some(ModelBasedTestSubject {
            packages: binding.packages,
            snapshots: binding.snapshots,
            initial: binding.initial.clone(),
            universes: binding.universes.clone(),
        })
    } else {
        None
    };
    let profile =
        fsl_foundation::selection::DefinitionRef::new("agent-ix", "filament.simulation.sampler/v1")
            .map_err(|_| fault("sampler-identity"))?;
    let request = ModelBasedTestRequest {
        clause: Some(clause),
        profile,
        seed: inputs.seed.unwrap_or(0),
        max_cases,
        max_trace_length: state_subject.then_some(inputs.max_trace_length).flatten(),
        max_value_bits: if consumes(&consumers.max_value_bits) {
            inputs.max_value_bits
        } else {
            None
        },
        subject,
        expansion: fsl_eval::simulation::model::ExpansionLimits {
            max_candidates: u64::MAX,
        },
        accounting: limits.accounting,
        normalization: limits.normalization,
        observation: limits.observation,
        emit: limits.emit,
        exploration: fsl_eval::simulation::Limits::default(),
        position_limit: MODEL_BASED_POSITIONS,
    };
    let record = match run_model_based_test(package, &request, &NativeAdmission, cancel) {
        Ok(record) => record,
        Err(refusal) => {
            // The entry refused the request before any case ran: a refusal
            // as it is outside a plan, never an internal failure.
            let (code, cause) = refusal_catalog(refusal.cause);
            let mut settled = not_run(code, cause, refusal.member);
            // A stop by cancellation or the run's deadline is incomplete.
            settled.status = if refusal.cause == "cancelled" {
                PlanStatus::Incomplete
            } else {
                PlanStatus::Declined
            };
            return Ok(settled);
        }
    };
    let (status, evidence) = match record.result {
        ModelBasedTestEvidence::Tested => (PlanStatus::Tested, Some("tested")),
        ModelBasedTestEvidence::Refuted => (PlanStatus::Refuted, Some("refutation")),
        ModelBasedTestEvidence::Inconclusive => (PlanStatus::Inconclusive, None),
        ModelBasedTestEvidence::Incomplete => (PlanStatus::Incomplete, None),
        ModelBasedTestEvidence::Failed => (PlanStatus::Failed, None),
        ModelBasedTestEvidence::Unsupported => (PlanStatus::Unsupported, None),
    };
    Ok(PlanDisposition {
        status,
        method: node.method.name.clone(),
        evidence: evidence.map(str::to_owned),
        blocked_by: None,
        result: Some(PlanResult::ModelBasedTest(Box::new(record))),
    })
}

/// The longest generated trace the FR-752 entry admits for a plan step, in
/// positions; `max_trace_length` is the request's own bound under it.
const MODEL_BASED_POSITIONS: u64 = 10_000;

struct Execution<'a> {
    package: &'a CheckedPackage,
    inputs: &'a PlanInputs<'a>,
    consumers: &'a PlanConsumers,
    engines: &'a Engines,
    registry: &'a Registry,
    negotiate: &'a dyn Negotiate,
    limits: &'a AnalyzeLimits,
    cancel: &'a Cancel,
}

impl Execution<'_> {
    fn run_step(
        &self,
        node: &VerificationPlanNode,
        index: usize,
    ) -> Result<PlanDisposition, StageFailure<PlanRefusal>> {
        let (package, inputs, engines, registry, negotiate, limits, cancel) = (
            self.package,
            self.inputs,
            self.engines,
            self.registry,
            self.negotiate,
            self.limits,
            self.cancel,
        );
        if node.method.name == "model_based_test" {
            return model_based_test(package, node, inputs, self.consumers, limits, cancel);
        }
        let binding = inputs
            .bindings
            .get(&node.name)
            .ok_or_else(|| fault("missing-step-binding"))?;
        let bound = node
            .bound
            .as_deref()
            .map(str::parse::<u64>)
            .transpose()
            .map_err(|_| fault("invalid-method-bound"))?;
        if node.method.name == "simulate" {
            return simulate(package, node, index, binding, bound, limits, cancel);
        }
        let unsettled = |status, code, cause, member| PlanDisposition {
            status,
            method: node.method.name.clone(),
            evidence: None,
            blocked_by: None,
            result: Some(PlanResult::NotRun {
                code,
                cause,
                member,
            }),
        };
        let (kind, mut form, evidence) = match node.method.name.as_str() {
            "model_check" => (
                Capability::TemporalSatisfaction,
                Some(ItemForm::Temporal(TemporalItem {
                    max_depth: bound
                        .map(usize::try_from)
                        .transpose()
                        .map_err(|_| fault("method-bound-overflow"))?,
                    ..TemporalItem::default()
                })),
                None,
            ),
            "contract_check" => (Capability::OperationContract, None, None),
            "value_check" => (Capability::ValueValidity, None, None),
            "probabilistic_exact" => (
                Capability::ProbabilisticSatisfaction,
                Some(ItemForm::Probabilistic(probabilistic(false, inputs.seed)?)),
                Some(EvidenceKind::Exact),
            ),
            "probabilistic_statistical" => (
                Capability::ProbabilisticSatisfaction,
                Some(ItemForm::Probabilistic(probabilistic(true, inputs.seed)?)),
                Some(EvidenceKind::Statistical),
            ),
            "hybrid_solver" => (
                Capability::TemporalSatisfaction,
                Some(ItemForm::Hybrid(HybridItem::default())),
                None,
            ),
            "schedulability" => (Capability::Schedulability, None, None),
            "replay" => (Capability::FiniteReplay, None, None),
            _ => return Err(fault("unknown-plan-method")),
        };
        let key = claim(package, node).ok_or_else(|| fault("unknown-plan-claim"))?;
        if let Some(ItemForm::Probabilistic(probability)) = &mut form {
            if probability.evidence == EvidenceKind::Statistical
                && package
                    .graph()
                    .probabilistic_model()
                    .claims()
                    .iter()
                    .any(|claim| {
                        claim.occurrence() == Some(&key)
                            && matches!(
                                claim.form,
                                fsl_semantics::check::ProbForm::LongRunFraction { .. }
                            )
                    })
            {
                // FR-189 admits only the regenerative estimator for a
                // long-run fraction; a fixed-sample test cannot decide it.
                probability.method =
                    Some(fsl_replay::statistical::StatisticalMethod::Regenerative {
                        min_cycles: 1,
                    });
            }
        }
        let fairness = package
            .graph()
            .temporal_clauses()
            .iter()
            .find(|clause| clause.identity() == *key.node() && clause.claim() == key.origin())
            .map(|clause| clause.clause().fairness().to_vec())
            .unwrap_or_default();
        if let Some(ItemForm::Temporal(temporal)) = &mut form {
            temporal.fairness = fairness.clone();
        }
        let item = ClaimItem::new(
            package,
            RequestIndex::new(index),
            key,
            kind,
            if bound.is_some() {
                ExtentClassification::Bounded
            } else {
                ExtentClassification::Unbounded {
                    finite_bound_available: false,
                }
            },
        );
        // A method whose kind does not apply to the claim settles that step
        // `unsupported`; its siblings still run (FSpec FR-824).
        let Ok(mut item) = item else {
            return Ok(unsettled(
                PlanStatus::Unsupported,
                "invalid_capability",
                "inapplicable-kind",
                "method",
            ));
        };
        item.form = form;
        item.options.max_cases = bound.and_then(NonZeroU64::new);
        item.options.required = true;
        let candidates = if let Some(evidence) = evidence {
            registry.candidates_for_evidence(kind, &ItemEvidence::Kind(evidence), None)
        } else {
            registry.candidates_for_temporal_dynamics(
                kind,
                None,
                &fairness
                    .iter()
                    .map(|constraint| constraint.kind())
                    .collect(),
                &Default::default(),
                node.method.name == "hybrid_solver",
            )
        };
        if let CandidateOutcome::Candidates(set) = &candidates {
            item.candidates = set.candidates().to_vec();
        }
        let (disposition, diagnostic) = negotiate.negotiate(&item, &candidates);
        let request = AnalyzeRequest {
            binding: binding.clone(),
            provision: inputs.provision.clone(),
            items: vec![NegotiatedItem {
                item,
                disposition,
                diagnostic,
            }],
        };
        let outcome = match analyze(package, &request, engines, limits, cancel) {
            Ok(staged) => staged.into_value(),
            // The item's binding or provision did not admit: that step is
            // declined, and its siblings still run.
            Err(StageFailure::Refused(refusal)) => {
                return Ok(unsettled(
                    PlanStatus::Declined,
                    "invalid_runtime_input",
                    analyze_refusal_cause(&refusal),
                    "binding",
                ));
            }
            Err(_) => return Err(fault("analyze-adapter")),
        };
        let mut record = outcome
            .records
            .into_iter()
            .next()
            .ok_or_else(|| fault("missing-step-record"))?;
        if record.diagnostic()
            == Some(fsl_route::NegotiationOutcome::UnsupportedRequestedCapability.code())
        {
            let mut detail = record.diagnostic_detail().cloned().unwrap_or_default();
            detail.insert(
                "kind".into(),
                serde_json::Value::String(kind.to_wire().into()),
            );
            detail.insert(
                "extent".into(),
                serde_json::Value::String(
                    if bound.is_some() {
                        "bounded"
                    } else {
                        "unbounded"
                    }
                    .into(),
                ),
            );
            record = record.with_diagnostic_detail(detail);
        }
        let status = status(&record);
        let evidence = match status {
            PlanStatus::Proved | PlanStatus::Refuted | PlanStatus::Measured(_)
                if node.method.name == "probabilistic_exact" =>
            {
                Some("exact")
            }
            PlanStatus::Proved | PlanStatus::Refuted if node.method.name == "hybrid_solver" => {
                Some("solver-result")
            }
            PlanStatus::Proved | PlanStatus::Refuted if node.method.name == "replay" => {
                Some("replay-disposition")
            }
            PlanStatus::Proved => Some(if node.method.name == "replay" {
                "replay-disposition"
            } else {
                "proof"
            }),
            PlanStatus::Refuted => Some("refutation"),
            PlanStatus::Measured(_) => Some("statistical"),
            _ => None,
        }
        .map(str::to_owned);
        Ok(PlanDisposition {
            status,
            method: node.method.name.clone(),
            evidence,
            blocked_by: None,
            result: Some(PlanResult::Analysis(Box::new(record))),
        })
    }
}

fn probabilistic(
    statistical: bool,
    seed: Option<u64>,
) -> Result<ProbabilisticItem, StageFailure<PlanRefusal>> {
    Ok(ProbabilisticItem {
        evidence: if statistical {
            EvidenceKind::Statistical
        } else {
            EvidenceKind::Exact
        },
        limits: if statistical {
            ProbabilisticLimits::Statistical(fsl_replay::statistical::StatisticalLimits::default())
        } else {
            ProbabilisticLimits::Exact(crate::exact_probabilistic::ExactProbLimits::default())
        },
        method: statistical.then_some(fsl_replay::statistical::StatisticalMethod::Okamoto),
        seed: statistical.then_some(seed.unwrap_or(0)),
        sampler: if statistical {
            Some(
                fsl_foundation::selection::DefinitionRef::new(
                    "agent-ix",
                    "filament.simulation.sampler/v1",
                )
                .map_err(|_| fault("sampler-identity"))?,
            )
        } else {
            None
        },
        max_witnesses: None,
        workers: None,
        symmetry: Vec::new(),
        timed_sampling: None,
    })
}

fn simulate(
    package: &CheckedPackage,
    node: &VerificationPlanNode,
    index: usize,
    binding: &SubjectBinding<'_>,
    bound: Option<u64>,
    limits: &AnalyzeLimits,
    cancel: &Cancel,
) -> Result<PlanDisposition, StageFailure<PlanRefusal>> {
    let item = crate::analyze::SimulationItem {
        index: RequestIndex::new(index),
        subject: None,
        settings: crate::analyze::SimulationSelection {
            mode: crate::analyze::SimulationMode::Enumerate,
            seed: None,
            samples: None,
            max_steps: bound.and_then(NonZeroU64::new),
            max_states: None,
            max_state_key_bytes: None,
        },
    };
    use crate::analyze::simulation_item::SimulationFault;
    use filament_semantic_value::catalog::Category;
    let (runs, category) =
        match crate::analyze::simulation_item::simulate(package, binding, limits, &item, cancel) {
            Ok(done) => done,
            // The subject did not admit: that step is declined, and its
            // siblings still run (FSpec FR-824).
            Err(SimulationFault::Binding(_)) => {
                return Ok(PlanDisposition {
                    status: PlanStatus::Declined,
                    method: node.method.name.clone(),
                    evidence: None,
                    blocked_by: None,
                    result: Some(PlanResult::NotRun {
                        code: "invalid_runtime_input",
                        cause: "invalid-value",
                        member: "initial",
                    }),
                });
            }
            Err(SimulationFault::Fault) => return Err(fault("simulation-adapter")),
        };
    // FR-634 simulation enumerates or samples states and never evaluates the
    // step's claim, so an exhausted run is no evidence for it: it settles
    // `inconclusive`, which satisfies no `requires` edge.
    Ok(PlanDisposition {
        status: match category {
            Category::Success => PlanStatus::Inconclusive,
            Category::Refusal => PlanStatus::Declined,
            Category::InternalFailure => PlanStatus::Failed,
            _ => PlanStatus::Incomplete,
        },
        method: node.method.name.clone(),
        evidence: Some("simulation-runs".into()),
        blocked_by: None,
        result: Some(PlanResult::Simulation(runs)),
    })
}

// FR-190's class S is distinct from requires-bound: a population without
// a maximum consumes an admitted universe, and declared finite scalar
// domains never consume max_value_bits. This classifies inputs only; it
// does not generate cases or implement the FR-752 tester.
fn consumes_value_bits(
    system: &fsl_eval::simulation::model::ModelSystem<'_>,
) -> Result<bool, StageFailure<PlanRefusal>> {
    types_consume_value_bits(
        system
            .generation_domains()
            .into_iter()
            .map(|(_, _, ty)| ty)
            .collect(),
        system.input().types,
    )
}

fn value_clause_consumes_bits(
    package: &CheckedPackage,
    node: &VerificationPlanNode,
) -> Result<bool, StageFailure<PlanRefusal>> {
    let Some(claim) = claim(package, node) else {
        return Err(fault("unknown-plan-claim"));
    };
    let graph = package.graph();
    for declared in graph
        .semantic_graph()
        .nodes()
        .filter_map(|node| node.declaration())
        .filter_map(|name| name.last())
    {
        let name = declared.as_str();
        if fsl_replay::value_check::ValueTarget::select(package, name, &claim).is_ok() {
            let callable = graph
                .callable(name)
                .ok_or_else(|| fault("value-clause-callable"))?;
            return types_consume_value_bits(
                callable
                    .parameters
                    .iter()
                    .map(|(_, ty)| ty.clone())
                    .collect(),
                graph.scope().types(),
            );
        }
    }
    Ok(false)
}

fn types_consume_value_bits(
    roots: Vec<filament_exact::ValueType>,
    types: &filament_semantic_value::declaration::TypeEnvironment,
) -> Result<bool, StageFailure<PlanRefusal>> {
    use filament_exact::ValueType;
    use filament_semantic_value::declaration::CompositeShape;
    let mut pending: Vec<_> = roots.into_iter().map(|ty| (ty, Vec::new())).collect();
    while let Some((ty, mut path)) = pending.pop() {
        match &ty {
            ValueType::Integer
            | ValueType::Quantity(_)
            | ValueType::String
            | ValueType::Bytes
            | ValueType::JsonObject => return Ok(true),
            ValueType::Option(payload) => pending.push(((**payload).clone(), path)),
            ValueType::Collection(collection) => {
                if collection.bound().is_none() {
                    return Ok(true);
                }
                pending.push((collection.element().clone(), path));
            }
            ValueType::Map(map) => {
                if map.bound().is_none() {
                    return Ok(true);
                }
                pending.push((map.key().clone(), path.clone()));
                pending.push((map.values().clone(), path));
            }
            ValueType::Composite(key) => {
                if path.contains(key) {
                    return Ok(true);
                }
                path.push(key.clone());
                let declaration = types
                    .composite(key.clone())
                    .ok_or_else(|| fault("generation-composite-missing"))?;
                let members: Vec<_> = match declaration.shape() {
                    CompositeShape::Record(fields) => fields
                        .iter()
                        .map(|field| field.value_type().clone())
                        .collect(),
                    CompositeShape::Tuple(positions) => positions.clone(),
                    CompositeShape::Union(members) => members
                        .iter()
                        .flat_map(|member| member.positions().iter().cloned())
                        .collect(),
                };
                pending.extend(members.into_iter().map(|ty| (ty, path.clone())));
            }
            ValueType::Boolean
            | ValueType::Int(_)
            | ValueType::Rational(_)
            | ValueType::Decimal(_)
            | ValueType::Float(_)
            | ValueType::Text(_)
            | ValueType::CarriedQuantity(_)
            | ValueType::BoundedQuantity(_)
            | ValueType::Uuid
            | ValueType::Timestamp
            | ValueType::Enum(_)
            | ValueType::Reference(_)
            | ValueType::Population(_) => {}
        }
    }
    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;
    use serde_json::json;

    /// A plan of `steps` (name, prerequisites) over one placeholder claim;
    /// the gate reads only names and `requires` edges.
    fn plan(steps: &[(&str, &[&str])]) -> VerificationPlan {
        let nodes: Vec<_> = steps
            .iter()
            .map(|(name, _)| {
                json!({
                    "name": name,
                    "claim": {
                        "node": {"package": "p", "path": ["c"], "ordinal": 0},
                        "occurrence": {"role": "claim", "ordinal": 0},
                    },
                    "method": {"name": "model_check", "kind": "temporal-satisfaction",
                        "extent": "unbounded", "evidence_kind": ["proof", "refutation"]},
                    "domain": {"package": "p", "node": "ix://p/M"},
                })
            })
            .collect();
        let edges: Vec<_> = steps
            .iter()
            .flat_map(|(name, requires)| {
                requires
                    .iter()
                    .map(move |to| json!({"from": name, "to": to, "type": "requires"}))
            })
            .collect();
        serde_json::from_value(json!({
            "format": "filament.verification-plan/v1", "package": "p",
            "plan": {"name": "P", "qualified_name": ["P"], "source_location": {
                "source": {"authority": "a", "identity": "u"}, "start": 0, "end": 1}},
            "profile": "v", "nodes": nodes, "edges": edges,
        }))
        .expect("plan document")
    }

    fn settled(status: PlanStatus) -> PlanDisposition {
        PlanDisposition {
            status,
            method: "model_check".to_owned(),
            evidence: None,
            blocked_by: None,
            result: None,
        }
    }

    /// Settle `plan` through the coordinator's gate, every handed node with
    /// `status(name)`; returns the dispositions and the handed names.
    fn gate(
        plan: &VerificationPlan,
        status: impl Fn(&str) -> PlanStatus,
    ) -> (Vec<(String, PlanDisposition)>, Vec<String>) {
        let order = execution_order(plan).expect("acyclic");
        let mut handed = Vec::new();
        let dispositions = settle_in_order(plan, &order, |index| {
            let name = plan.nodes[index].name.clone();
            let disposition = settled(status(&name));
            handed.push(name);
            Ok(disposition)
        })
        .expect("settles");
        (dispositions, handed)
    }

    #[test]
    #[trace("FSpec-TC-2045", "FSpec-FR-824-AC-4")]
    #[trace("TC-2103", "FR-753-AC-4")]
    fn a_step_requiring_a_later_step_waits_for_it_and_is_blocked_when_it_fails() {
        let plan = plan(&[("early", &["late"]), ("late", &[])]);
        let (dispositions, handed) = gate(&plan, |_| PlanStatus::Refuted);
        assert_eq!(
            handed,
            ["late"],
            "early is not handed on before late settles"
        );
        assert_eq!(dispositions[0].0, "early");
        assert_eq!(dispositions[0].1.status, PlanStatus::Blocked);
        assert_eq!(dispositions[0].1.blocked_by.as_deref(), Some("late"));
        assert_eq!(dispositions[1].1.status, PlanStatus::Refuted);
        let (dispositions, handed) = gate(&plan, |_| PlanStatus::Proved);
        assert_eq!(handed, ["late", "early"]);
        assert_eq!(dispositions[0].1.status, PlanStatus::Proved);
    }

    #[test]
    #[trace("FSpec-TC-2045", "FSpec-FR-824-AC-4")]
    #[trace("TC-2103", "FR-753-AC-4")]
    fn a_blocked_prerequisite_blocks_its_dependents_by_the_first_unsatisfied_one() {
        let plan = plan(&[("a", &[]), ("b", &["a"]), ("c", &["b", "a"])]);
        let (dispositions, handed) = gate(&plan, |_| PlanStatus::Inconclusive);
        assert_eq!(handed, ["a"]);
        assert_eq!(dispositions[1].1.blocked_by.as_deref(), Some("a"));
        assert_eq!(dispositions[2].1.blocked_by.as_deref(), Some("a"));
    }

    #[test]
    #[trace("FSpec-TC-2045", "FSpec-FR-824-AC-5")]
    fn a_measured_step_gates_its_dependent_by_its_decision() {
        let plan = plan(&[("s", &[]), ("t", &["s"])]);
        for (decision, dependent_runs) in [
            (PlanDecision::Accepted, true),
            (PlanDecision::Rejected, false),
            (PlanDecision::Undecided, false),
        ] {
            let (dispositions, handed) = gate(&plan, |name| match name {
                "s" => PlanStatus::Measured(decision),
                _ => PlanStatus::Proved,
            });
            assert_eq!(
                handed.contains(&"t".to_owned()),
                dependent_runs,
                "{decision:?}"
            );
            assert_eq!(
                dispositions[1].1.status,
                if dependent_runs {
                    PlanStatus::Proved
                } else {
                    PlanStatus::Blocked
                }
            );
        }
    }
}

/// The catalog `(code, cause)` of an FR-752 entry refusal's `cause`: either
/// `code/cause`, a bare `invalid_*` code, or a bare cause under
/// `invalid_runtime_input`.
fn refusal_catalog(cause: &'static str) -> (&'static str, &'static str) {
    match cause.split_once('/') {
        Some(pair) => pair,
        None if cause.starts_with("invalid_") => (cause, "invalid-value"),
        None => ("invalid_runtime_input", cause),
    }
}

/// The catalog cause of an analyze refusal of a step's item.
const fn analyze_refusal_cause(refusal: &crate::analyze::AnalyzeRefusal) -> &'static str {
    match refusal {
        crate::analyze::AnalyzeRefusal::Provision(_) => "missing-member",
        crate::analyze::AnalyzeRefusal::Binding(_) | crate::analyze::AnalyzeRefusal::Package(_) => {
            "invalid-value"
        }
    }
}
