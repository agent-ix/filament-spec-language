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

/// A step's native result, without converting simulation runs into a proof.
#[derive(Debug)]
pub enum PlanResult {
    /// The negotiated item's FR-331 terminal record.
    Analysis(Box<TerminalRecord>),
    /// The simulation item's FR-181 runs.
    Simulation(serde_json::Value),
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

impl PlanDisposition {
    fn satisfies_requires(&self) -> bool {
        match self.status {
            PlanStatus::Proved | PlanStatus::Tested => true,
            PlanStatus::Measured => matches!(
                self.result.as_ref(),
                Some(PlanResult::Analysis(record))
                    if matches!(record.value(), TerminalValue::Measured(verdict)
                    if matches!(verdict.decision, fsl_replay::statistical::Decision::Accepted)
                    )
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
        if node.method.name == "model_based_test"
            && !consumes_initial(package, node)
            && value_consumer.is_none()
            && value_clause_consumes_bits(package, node)?
        {
            value_consumer = Some(node.name.clone());
        }
        if let Some(binding) = inputs.bindings.get(&node.name) {
            if consumes_initial(package, node) {
                match fsl_eval::simulation::model::ModelSystem::over_model(
                    crate::analyze::request::simulation(&input, binding, limits),
                    &node.domain.package,
                ) {
                    Ok(system) => {
                        admitted_initial |= !binding.initial.is_empty();
                        if node.method.name == "model_based_test"
                            && value_consumer.is_none()
                            && consumes_value_bits(&system)?
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
        if let Some(node) = plan
            .nodes
            .iter()
            .find(|node| node.method.name == "model_based_test" && consumes_initial(package, node))
        {
            return Err(StageFailure::Refused(PlanRefusal::MissingMember {
                member: "max_trace_length",
                step: node.name.clone(),
            }));
        }
    }
    if inputs.max_value_bits.is_none() {
        if let Some(step) = value_consumer {
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
        engines,
        registry: &registry,
        negotiate,
        limits,
        cancel,
    };
    let mut settled: BTreeMap<usize, PlanDisposition> = BTreeMap::new();
    for index in order {
        let node = &plan.nodes[index];
        let blocked = plan.nodes.iter().enumerate().find(|(at, prerequisite)| {
            plan.edges
                .iter()
                .any(|edge| edge.from == node.name && edge.to == prerequisite.name)
                && settled
                    .get(at)
                    .is_some_and(|step| !step.satisfies_requires())
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
            execution.run_step(node, index)?
        };
        settled.insert(index, disposition);
    }
    let dispositions = settled
        .into_iter()
        .map(|(index, disposition)| (plan.nodes[index].name.clone(), disposition))
        .collect();
    Ok(Staged::new(PlanOutcome { dispositions }))
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

struct Execution<'a> {
    package: &'a CheckedPackage,
    inputs: &'a PlanInputs<'a>,
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
            return model_based_test(package, node, inputs, limits, cancel);
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
        let mut item = ClaimItem::new(
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
        )
        .map_err(|_| fault("inapplicable-plan-claim"))?;
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
        let outcome = analyze(package, &request, engines, limits, cancel)
            .map_err(|_| fault("analyze-adapter"))?
            .into_value();
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
            PlanStatus::Proved | PlanStatus::Refuted | PlanStatus::Measured
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
            PlanStatus::Measured => Some("statistical"),
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
    let (runs, category) =
        crate::analyze::simulation_item::simulate(package, binding, limits, &item, cancel)
            .map_err(|_| fault("simulation-adapter"))?;
    Ok(PlanDisposition {
        status: if category == filament_semantic_value::catalog::Category::Success {
            PlanStatus::Tested
        } else {
            PlanStatus::Incomplete
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
