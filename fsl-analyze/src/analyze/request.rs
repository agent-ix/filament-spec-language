// SPDX-License-Identifier: AGPL-3.0-or-later
//! The operation-owned request boundary shared by analyze and the provider.

use std::collections::BTreeMap;

use fsl_foundation::source::provenance::{OccurrenceKey, RawSourceRef};
use fsl_foundation::{BackendId, RequestIndex};
use fsl_route::request::{ExtentClassification, ItemKind};
use fsl_route::{Candidate, EvidenceKind};
use fsl_semantics::check::Capability;

use super::Diagnostic;

/// Source bytes keyed only by the package's declared source labels.
#[derive(Clone, Default, Eq, PartialEq)]
pub struct SourceProvision {
    entries: BTreeMap<(String, String), Vec<u8>>,
}

impl std::fmt::Debug for SourceProvision {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_map()
            .entries(
                self.entries
                    .iter()
                    .map(|(source, bytes)| (source, bytes.len())),
            )
            .finish()
    }
}

impl SourceProvision {
    /// Provide the bytes under their declared source label.
    pub fn insert(&mut self, source: &RawSourceRef, bytes: Vec<u8>) {
        self.entries.insert(
            (source.authority().to_owned(), source.identity().to_owned()),
            bytes,
        );
    }

    /// A provision containing one source unit.
    pub fn single(source: &RawSourceRef, bytes: &[u8]) -> Self {
        let mut provision = Self::default();
        provision.insert(source, bytes.to_vec());
        provision
    }

    /// Read bytes by the package's source label, never by a path.
    pub fn get(&self, source: &RawSourceRef) -> Option<&[u8]> {
        self.entries
            .get(&(source.authority().to_owned(), source.identity().to_owned()))
            .map(Vec::as_slice)
    }

    /// Every entry as `(authority, identity, bytes)`, in label order.
    pub fn entries(&self) -> impl Iterator<Item = (&str, &str, &[u8])> + '_ {
        self.entries.iter().map(|((authority, identity), bytes)| {
            (authority.as_str(), identity.as_str(), bytes.as_slice())
        })
    }

    pub(super) fn wire_entries(&self) -> Vec<(String, String, Vec<u8>)> {
        self.entries
            .iter()
            .map(|((authority, identity), bytes)| {
                (authority.clone(), identity.clone(), bytes.clone())
            })
            .collect()
    }
}

/// A source named by the package was absent from its provision.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[error("missing source {authority}:{identity}")]
pub struct ProvisionRefusal {
    /// The missing source's declared authority.
    pub authority: String,
    /// The missing source's declared identity.
    pub identity: String,
}

impl ProvisionRefusal {
    /// FR-098's missing-selection refusal.
    pub const fn catalog_code(&self) -> filament_semantic_value::catalog::CatalogCode {
        filament_semantic_value::catalog::CatalogCode::new("missing_import", "missing-selection")
    }
}

/// Common options of one requested claim.
#[derive(Clone, Debug, Default)]
pub struct ItemOptions {
    /// Complete aggregate success requires this item.
    pub required: bool,
    /// Optional finite assignment/case budget of the native check arms.
    pub max_cases: Option<std::num::NonZeroU64>,
    /// The provisioned document a `finite-replay` item replays.
    pub trace: Option<fsl_semantics::model::observation::DocumentRef>,
    /// The provisioned FR-737 execution assessed by `global-conformance`.
    pub recording: Option<fsl_semantics::model::observation::DocumentRef>,
    /// The named backend's options (FSpec FR-196, FR-331 `options`),
    /// each one the backend publishes (FR-967); empty when none are given.
    pub backend_options: std::collections::BTreeMap<String, String>,
}

/// One claim occurrence and its kind-specific, subject-free settings.
#[derive(Clone)]
pub struct ClaimItem {
    /// The original request index.
    pub request_index: RequestIndex,
    /// The checked claim occurrence; resolved against the operation's package.
    pub claim: OccurrenceKey,
    /// The requested capability.
    pub kind: Capability,
    /// The caller's extent classification, independent of proof bounds.
    pub extent: ExtentClassification,
    /// A caller-named backend, when present.
    pub backend: Option<BackendId>,
    /// The candidate set computed by the caller.
    pub candidates: Vec<Candidate>,
    /// Common request settings.
    pub options: ItemOptions,
    /// The capability's own request settings.
    pub form: Option<ItemForm>,
}

/// A requested key does not identify a checked claim of the requested kind.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum ClaimRefusal {
    /// No checked claim matches this complete occurrence key.
    #[error("unknown checked claim")]
    UnknownClaim,
}

impl ClaimItem {
    /// Resolve a claim against its owning checked package before configuring it.
    ///
    /// # Errors
    /// Returns [`ClaimRefusal::UnknownClaim`] for an absent node, a different
    /// occurrence, or a claim to which `kind` does not apply.
    pub fn new(
        package: &fsl_package::CheckedPackage,
        request_index: RequestIndex,
        claim: OccurrenceKey,
        kind: Capability,
        extent: ExtentClassification,
    ) -> Result<Self, ClaimRefusal> {
        let graph = package.graph();
        let family = crate::claim_family::claim_family(package, ItemKind::Formula, &claim);
        let occurs = graph
            .occurrences()
            .any(|(node, origin, _)| node == *claim.node() && origin == *claim.origin());
        let wrong_authored_occurrence = graph
            .temporal_clauses()
            .iter()
            .find(|clause| clause.identity() == *claim.node())
            .is_some_and(|clause| clause.claim() != claim.origin())
            || graph
                .probabilistic_model()
                .claims()
                .iter()
                .filter_map(|checked| checked.occurrence())
                .find(|checked| checked.node() == claim.node())
                .is_some_and(|checked| checked != &claim);
        if !occurs
            || wrong_authored_occurrence
            || family.is_none_or(|family| !kind.applies_to_declaration(family))
        {
            return Err(ClaimRefusal::UnknownClaim);
        }
        Ok(Self {
            request_index,
            claim,
            kind,
            extent,
            backend: None,
            candidates: Vec::new(),
            options: ItemOptions::default(),
            form: None,
        })
    }
}

/// The request form of a kind that has one.
#[derive(Clone)]
pub enum ItemForm {
    /// An authored temporal clause.
    Temporal(TemporalItem),
    /// An exact or statistical probabilistic claim.
    Probabilistic(ProbabilisticItem),
    /// A hybrid-dynamics request; only an engine implementing this form runs it.
    Hybrid(HybridItem),
}

/// Hybrid item settings kept distinct from temporal and probabilistic forms.
#[derive(Clone, Debug, Default)]
pub struct HybridItem {
    /// The subject's explicit proof-bound substitution.
    pub proof_bounds: Vec<fsl_foundation::bound::ProofBound>,
}

/// Temporal request settings, with no separately supplied model subject.
#[derive(Clone)]
pub struct TemporalItem {
    /// The proof-bound substitution of the subject.
    pub proof_bounds: Vec<fsl_foundation::bound::ProofBound>,
    /// Formula or derived temporal item.
    pub item_kind: ItemKind,
    /// The abstract subject selected by a refinement item.
    pub abstract_subject: Option<fsl_route::request::AbstractSubject>,
    /// The resolved fairness constraints of the checked clause.
    pub fairness: Vec<filament_semantic_value::fairness::FairnessConstraint>,
    /// A requested stutter-closure premise, when supplied.
    pub stutter_closure: Option<bool>,
    /// The search horizon; no horizon when absent.
    pub max_depth: Option<usize>,
    /// The model search and automaton budgets.
    pub limits: fsl_replay::ModelCheckLimits,
    /// The most successor candidates one expansion enumerates.
    pub max_candidates: u64,
    /// The qualified checker's SMT encoding budgets.
    pub smt_encoding: Option<fsl_replay::SmtEncodingLimits>,
    /// The reductions the caller selected.
    pub reductions: fsl_route::reductions::Reductions,
    /// The most live clocks one timed state holds (FSpec FR-415); the
    /// published default when absent.
    pub max_clocks: Option<std::num::NonZeroU64>,
    /// The request's `reductions` member as written, until it is admitted over
    /// the unit's declarations (FR-623).
    pub pending_reductions: Option<serde_json::Value>,
    /// The witness walks per initial state of a `possible` claim (FSpec
    /// FR-392); the published default when absent.
    pub witness_samples: Option<u64>,
    /// The witness-sampling seed; the published default when absent.
    pub seed: Option<u64>,
}

impl Default for TemporalItem {
    fn default() -> Self {
        Self {
            proof_bounds: Vec::new(),
            item_kind: ItemKind::Formula,
            abstract_subject: None,
            fairness: Vec::new(),
            stutter_closure: None,
            max_depth: None,
            limits: fsl_replay::ModelCheckLimits::default(),
            max_candidates: super::clause_item::DEFAULT_MAX_CANDIDATES,
            smt_encoding: None,
            reductions: fsl_route::reductions::Reductions::default(),
            max_clocks: None,
            pending_reductions: None,
            witness_samples: None,
            seed: None,
        }
    }
}

/// The settings of one probabilistic claim, never its model subject.
#[derive(Clone)]
pub struct ProbabilisticItem {
    /// Exact or statistical evidence requested.
    pub evidence: EvidenceKind,
    /// The matching engine budgets.
    pub limits: ProbabilisticLimits,
    /// Statistical method; absent for exact evidence.
    pub method: Option<fsl_replay::statistical::StatisticalMethod>,
    /// Statistical seed; absent for exact evidence.
    pub seed: Option<u64>,
    /// Statistical sampler; absent for exact evidence.
    pub sampler: Option<fsl_foundation::selection::DefinitionRef>,
    /// Maximum sampled witnesses retained.
    pub max_witnesses: Option<u32>,
    /// Declared statistical symmetries.
    pub symmetry: Vec<fsl_route::reductions::SymmetryDeclaration>,
    /// Timed statistical sampling settings.
    pub timed_sampling: Option<fsl_replay::statistical::TimedSampling>,
}

/// Kind-specific probabilistic budgets; the evidence and budget must agree.
#[derive(Clone, Copy)]
pub enum ProbabilisticLimits {
    /// Statistical sampling limits.
    Statistical(fsl_replay::statistical::StatisticalLimits),
    /// Exact product/solver/checker limits.
    Exact(crate::exact_probabilistic::ExactProbLimits),
}

/// A claim with the disposition and diagnostic supplied by negotiation.
#[derive(Clone)]
pub struct NegotiatedItem {
    /// The typed requested item.
    pub item: ClaimItem,
    /// The caller's negotiated disposition.
    pub disposition: fsl_route::routing::Disposition,
    /// The disposition's catalog diagnostic.
    pub diagnostic: Option<Diagnostic>,
}

/// Analyze negotiated items using only the admitted package, subject and supplied bytes.
///
/// # Errors
/// Returns the shared binding or missing-provision refusal before any item runs.
pub fn analyze(
    package: &fsl_package::CheckedPackage,
    request: &super::AnalyzeRequest<'_>,
    engines: &super::Engines,
    limits: &super::AnalyzeLimits,
    cancel: &filament_exact::Cancel,
) -> Result<
    fsl_foundation::diagnostic::Staged<super::AnalyzeOutcome>,
    fsl_foundation::diagnostic::StageFailure<super::AnalyzeRefusal>,
> {
    analyze_bound(
        package,
        &request.binding,
        &request.provision,
        &request.items,
        engines,
        limits,
        cancel,
    )
}

pub(crate) fn analyze_bound(
    package: &fsl_package::CheckedPackage,
    binding: &super::SubjectBinding<'_>,
    provision: &SourceProvision,
    requested: &[NegotiatedItem],
    engines: &super::Engines,
    limits: &super::AnalyzeLimits,
    cancel: &filament_exact::Cancel,
) -> Result<
    fsl_foundation::diagnostic::Staged<super::AnalyzeOutcome>,
    fsl_foundation::diagnostic::StageFailure<super::AnalyzeRefusal>,
> {
    use fsl_foundation::diagnostic::{StageFailure, Staged};
    let simulation = simulation(package, binding, limits);
    fsl_eval::simulation::model::ModelSystem::new(simulation)
        .map_err(|failure| StageFailure::Refused(super::AnalyzeRefusal::Binding(failure)))?;
    check_provision(package, provision)
        .map_err(|failure| StageFailure::Refused(super::AnalyzeRefusal::Provision(failure)))?;
    let items = requested
        .iter()
        .map(|item| work(package, binding, provision, item, engines, limits))
        .collect::<Vec<_>>();
    let requirements = items
        .iter()
        .map(super::AnalyzeItem::requirement)
        .collect::<Vec<_>>();
    let settled = super::analyze_items(engines, items, cancel);
    let aggregate = super::aggregate(&requirements, &settled);
    let details = settled.iter().map(super::ItemDetail::of).collect();
    let protocol_conformance = settled
        .iter()
        .filter_map(|record| {
            record
                .protocol_conformance
                .clone()
                .map(|output| (record.request_index, output))
        })
        .collect();
    let (records, counterexamples) = super::split_records(settled, package);
    Ok(Staged::new(super::AnalyzeOutcome {
        records,
        counterexamples,
        aggregate,
        details,
        protocol_conformance,
    }))
}

fn check_provision(
    package: &fsl_package::CheckedPackage,
    provision: &SourceProvision,
) -> Result<(), ProvisionRefusal> {
    let mut pending = vec![package];
    let mut visited = std::collections::BTreeSet::new();
    while let Some(package) = pending.pop() {
        if !visited.insert(package.graph().package_name()) {
            continue;
        }
        for source in package.graph().sources() {
            if provision.get(source).is_none() {
                return Err(ProvisionRefusal {
                    authority: source.authority().to_owned(),
                    identity: source.identity().to_owned(),
                });
            }
        }
        pending.extend(
            package
                .dependencies()
                .values()
                .map(std::convert::AsRef::as_ref),
        );
    }
    Ok(())
}

pub(super) fn dependency_wires(
    package: &fsl_package::CheckedPackage,
) -> Vec<fsl_replay::DependencyEntryWire> {
    let mut pending = package.dependencies().iter().collect::<Vec<_>>();
    let mut dependencies = BTreeMap::new();
    while let Some((identity, package)) = pending.pop() {
        if dependencies.contains_key(identity) {
            continue;
        }
        dependencies.insert(
            identity.clone(),
            fsl_replay::DependencyEntryWire {
                identity: identity.to_string(),
                sources: package
                    .graph()
                    .sources()
                    .into_iter()
                    .map(|source| (source.authority().to_owned(), source.identity().to_owned()))
                    .collect(),
                refinement_internals: None,
            },
        );
        pending.extend(package.dependencies().iter());
    }
    dependencies.into_values().collect()
}

pub(super) fn simulation<'a>(
    package: &'a fsl_package::CheckedPackage,
    binding: &super::SubjectBinding<'a>,
    limits: &super::AnalyzeLimits,
) -> fsl_eval::simulation::model::ModelSimulationRequest<'a> {
    fsl_eval::simulation::model::ModelSimulationRequest {
        package,
        packages: binding.packages,
        normalization: limits.normalization,
        snapshots: binding.snapshots,
        initial: binding.initial.clone(),
        universes: binding.universes.clone(),
        observation: limits.observation,
        expansion: fsl_eval::simulation::model::ExpansionLimits {
            max_candidates: u64::MAX,
        },
        meter: limits.accounting,
        emit: limits.emit,
    }
}

fn work<'a>(
    package: &'a fsl_package::CheckedPackage,
    binding: &super::SubjectBinding<'a>,
    provision: &'a SourceProvision,
    negotiated: &'a NegotiatedItem,
    engines: &super::Engines,
    limits: &super::AnalyzeLimits,
) -> super::AnalyzeItem<'a> {
    use super::AnalyzeItem;
    let item = &negotiated.item;
    let temporal_kind = match &item.form {
        Some(ItemForm::Temporal(form)) => form.item_kind,
        Some(ItemForm::Probabilistic(_) | ItemForm::Hybrid(_)) | None => ItemKind::Formula,
    };
    let family = crate::claim_family::claim_family(package, temporal_kind, &item.claim);
    let wrong_occurrence = package
        .graph()
        .temporal_clauses()
        .iter()
        .find(|clause| clause.identity() == *item.claim.node())
        .is_some_and(|clause| clause.claim() != item.claim.origin())
        || package
            .graph()
            .probabilistic_model()
            .claims()
            .iter()
            .filter_map(|claim| claim.occurrence())
            .find(|claim| claim.node() == item.claim.node())
            .is_some_and(|claim| claim != &item.claim);
    let family = if wrong_occurrence { None } else { family };
    let absent = || AnalyzeItem::unadapted(item.request_index, item.kind, family);
    let mut built = match &item.form {
        Some(ItemForm::Temporal(_form)) if item.kind == Capability::GlobalConformance => {
            let protocol = package.graph().protocols().iter().find(|protocol| {
                package
                    .graph()
                    .protocol_node_keys(&protocol.name)
                    .and_then(|keys| keys.clause.as_ref())
                    == Some(item.claim.node())
            });
            match protocol {
                Some(protocol) => {
                    let missing_role = if item.options.recording.is_some() {
                        None
                    } else {
                        let selected_model_packages: std::collections::BTreeSet<&str> = binding
                            .universes
                            .iter()
                            .map(|universe| universe.population.package.as_str())
                            .collect();
                        protocol.roles.iter().find(|role| {
                            role.context.as_ref().is_some_and(|context| {
                                !selected_model_packages.contains(context.original().package())
                            })
                        })
                    };
                    if let Some(role) = missing_role {
                        crate::protocol_analysis::conformance::missing_role_item(
                            item.request_index,
                            &role.name,
                        )
                    } else {
                        let recording = item
                            .options
                            .recording
                            .as_ref()
                            .and_then(|reference| binding.snapshots.get(reference));
                        let recording_identity = item
                            .options
                            .recording
                            .as_ref()
                            .cloned();
                        match recording {
                            Some(recording) => {
                                crate::protocol_analysis::conformance::global_recording_item(
                                    item.request_index,
                                    package,
                                    None,
                                    recording_identity,
                                    Some(recording),
                                )
                            }
                            None => crate::protocol_analysis::conformance::global_behavior_item(
                                item.request_index,
                                package,
                                binding.clone(),
                                *limits,
                                protocol,
                                _form,
                            ),
                        }
                    }
                }
                None => absent(),
            }
        }
        Some(ItemForm::Temporal(form))
            if item.kind == Capability::TemporalSatisfaction
                && super::document::is_state_graph(package, &item.claim) =>
        {
            super::state_graph_item::state_graph_item(
                package,
                binding,
                *limits,
                provision,
                &item.claim,
                item.request_index,
                form,
            )
            .unwrap_or_else(absent)
        }
        Some(ItemForm::Temporal(form))
            if item.kind == Capability::TemporalSatisfaction
                && matches!(form.item_kind, ItemKind::Formula | ItemKind::Refinement)
                && !super::document::timed(package)
                && super::refinement_item::refinement_at(package, &item.claim).is_some() =>
        {
            super::refinement_item::refinement_item(
                package,
                binding,
                *limits,
                &item.claim,
                item.request_index,
                form,
            )
            .unwrap_or_else(absent)
        }
        Some(ItemForm::Temporal(form))
            if item.kind == Capability::TemporalSatisfaction
                && matches!(
                    form.item_kind,
                    ItemKind::DeadlockFreedom | ItemKind::TimeLockFreedom
                ) =>
        {
            if super::document::timed(package) {
                super::timed_item::zone_derived_item(
                    package,
                    binding,
                    *limits,
                    provision,
                    &item.claim,
                    item.request_index,
                    form,
                )
                .unwrap_or_else(absent)
            } else if form.item_kind == ItemKind::DeadlockFreedom {
                super::derived_item::deadlock_item(
                    package,
                    binding,
                    *limits,
                    provision,
                    &item.claim,
                    item.request_index,
                    form,
                )
                .unwrap_or_else(absent)
            } else {
                absent()
            }
        }
        Some(ItemForm::Temporal(form))
            if item.kind == Capability::TemporalSatisfaction
                && form.item_kind == ItemKind::Formula
                && super::document::timed(package) =>
        {
            super::timed_item::zone_clause_item(
                package,
                binding,
                *limits,
                provision,
                &item.claim,
                item.request_index,
                form,
            )
            .unwrap_or_else(absent)
        }
        Some(ItemForm::Temporal(form)) if item.kind == Capability::TemporalSatisfaction => {
            match super::ClauseSubject::from_provision(
                package,
                binding.clone(),
                *limits,
                provision,
                &item.claim,
                super::ClauseLimits {
                    limits: form.limits.limits,
                    max_depth: form.max_depth,
                    max_candidates: form.max_candidates,
                },
                form.proof_bounds.clone(),
            ) {
                Ok(subject) => subject.configured(form.clone()).item(item.request_index),
                Err(_) => absent(),
            }
        }
        Some(ItemForm::Probabilistic(form))
            if item.kind == Capability::ProbabilisticSatisfaction =>
        {
            let claim = package
                .graph()
                .probabilistic_model()
                .claims()
                .iter()
                .find(|claim| claim.occurrence() == Some(&item.claim));
            match (claim, form.evidence, form.limits) {
                (Some(claim), EvidenceKind::Exact, ProbabilisticLimits::Exact(exact)) => {
                    super::exact_probabilistic_item(
                        item.request_index,
                        crate::exact_probabilistic::ProductRequest {
                            subject: fsl_eval::model_subject::ModelSubject {
                                simulation: simulation(package, binding, limits),
                                proof_bounds: Vec::new(),
                            },
                            claim,
                            limits: exact,
                        },
                    )
                }
                (
                    Some(claim),
                    EvidenceKind::Statistical,
                    ProbabilisticLimits::Statistical(statistical),
                ) => match (form.method, form.seed, &form.sampler) {
                    (Some(method), Some(seed), Some(sampler)) => super::statistical_item(
                        item.request_index,
                        crate::statistical::StatisticalRequest {
                            subject: fsl_eval::model_subject::ModelSubject {
                                simulation: simulation(package, binding, limits),
                                proof_bounds: Vec::new(),
                            },
                            claim,
                            method,
                            seed,
                            sampler: sampler.clone(),
                            limits: statistical,
                            max_witnesses: form
                                .max_witnesses
                                .unwrap_or(crate::statistical::DEFAULT_MAX_WITNESSES),
                            symmetry: form.symmetry.clone(),
                            timed_sampling: form.timed_sampling.unwrap_or_default(),
                        },
                    ),
                    _ => absent(),
                },
                _ => absent(),
            }
        }
        None if item.kind == Capability::ValueValidity => {
            let selection = package
                .graph()
                .semantic_graph()
                .nodes()
                .filter_map(|node| node.declaration())
                .filter_map(|name| name.last())
                .find_map(|name| {
                    fsl_replay::value_check::ValueTarget::select(
                        package,
                        name.as_str(),
                        &item.claim,
                    )
                    .ok()
                })
                .ok_or(fsl_replay::value_check::Selection::NoForm)
                .map(|target| target.with_limits(fsl_replay::AccountingLimits(limits.accounting)));
            if let Some(backend) = negotiated.disposition.target().filter(|backend| {
                engines
                    .get(backend.id())
                    .is_some_and(|engine| engine.provider().is_some())
            }) {
                let routed = crate::provider::routed_item(
                    package,
                    engines.registrations(),
                    item.request_index,
                    &item.claim,
                    item.backend.as_ref(),
                )
                .map(|routed| routed.with_options(item.options.backend_options.clone()));
                crate::provider::provider_item(
                    item.request_index,
                    backend.id(),
                    crate::provider::ProviderItemInput {
                        package,
                        packages: binding.packages,
                        claim: &item.claim,
                        routed,
                        emit: limits.emit,
                        selection,
                    },
                )
            } else {
                let selection = selection.map(|target| crate::value_check::ValueRequest {
                    target,
                    max_cases: item.options.max_cases,
                    reported: None,
                });
                super::value_item(item.request_index, package, &item.claim, selection)
            }
        }
        None if item.kind == Capability::FiniteReplay => {
            let protocol = package.graph().protocols().iter().find(|protocol| {
                package
                    .graph()
                    .protocol_node_keys(&protocol.name)
                    .and_then(|keys| keys.clause.as_ref())
                    == Some(item.claim.node())
            });
            match protocol {
                Some(protocol) => {
                    let trace = item
                        .options
                        .trace
                        .as_ref()
                        .and_then(|reference| binding.snapshots.get(reference))
                        .map_or(
                            Err(crate::protocol_analysis::TraceDefect::unreadable()),
                            |bytes| crate::protocol_analysis::SuppliedTrace::read(bytes),
                        );
                    crate::protocol_analysis::finite_replay_item(
                        item.request_index,
                        package,
                        binding.clone(),
                        *limits,
                        protocol,
                        trace,
                    )
                }
                None => absent(),
            }
        }
        None if item.kind == Capability::OperationContract => {
            let selection =
                fsl_replay::contract_check::select_contract(package, &item.claim).map(|claim| {
                    crate::contract_check::ContractRequest {
                        package,
                        claim,
                        subject: simulation(package, binding, limits),
                        abstract_subject: None,
                        max_cases: item.options.max_cases,
                    }
                });
            super::contract_item(item.request_index, package, &item.claim, selection)
        }
        _ => absent(),
    };
    let expected = match &item.form {
        _ if item.kind == Capability::GlobalConformance => {
            Some(crate::protocol_analysis::manifest::BACKEND_ID)
        }
        Some(ItemForm::Temporal(_)) if super::document::is_state_graph(package, &item.claim) => {
            Some(crate::model_check::manifest::BACKEND_ID)
        }
        Some(ItemForm::Temporal(_)) if super::document::timed(package) => {
            Some(crate::zone_check::manifest::BACKEND_ID)
        }
        Some(ItemForm::Temporal(_)) => Some(crate::model_check::manifest::BACKEND_ID),
        Some(ItemForm::Probabilistic(form)) => match form.evidence {
            EvidenceKind::Exact => Some(crate::exact_probabilistic::manifest::BACKEND_ID),
            EvidenceKind::Statistical => Some(crate::statistical::manifest::BACKEND_ID),
        },
        Some(ItemForm::Hybrid(_)) => None,
        None => None,
    };
    if let (Some(expected), fsl_route::routing::Disposition::Supported(candidate)) =
        (expected, &negotiated.disposition)
    {
        if candidate.id().as_str() != expected {
            built.work = Some(super::ItemWork::Engine(Box::new(move |_, _| {
                super::EngineReport::Refused(super::Refusal::Unsupported(
                    fsl_route::NegotiationOutcome::UnsupportedRequestedCapability.code(),
                ))
            })));
        }
    }
    built.kind = item.kind;
    built.family = family;
    built.claim = Some(item.claim.clone());
    // EN-6 decides over the subject's universes alone (FR-239): it advertises
    // the bounded mode, and an item it runs is read in it.
    let zone = matches!(&item.form, Some(ItemForm::Temporal(_))) && super::document::timed(package);
    built.mode = match item.extent {
        _ if zone => fsl_route::Mode::Bounded,
        ExtentClassification::Bounded => fsl_route::Mode::Bounded,
        ExtentClassification::Unbounded { .. } => fsl_route::Mode::Unbounded,
    };
    built.required(item.options.required).disposed(
        negotiated.disposition.clone(),
        negotiated.diagnostic.clone(),
    )
}

#[cfg(test)]
mod tests {
    use ix_trace_rs::trace;

    use super::SourceProvision;
    use fsl_foundation::source::provenance::RawSourceRef;

    #[trace("TC-2804", "FR-992-AC-1")]
    #[test]
    fn source_provision_entries_are_in_label_order() {
        let mut provision = SourceProvision::default();
        let z = RawSourceRef::new("b", "z").expect("valid source label");
        let a = RawSourceRef::new("a", "a").expect("valid source label");
        provision.insert(&z, b"z".to_vec());
        provision.insert(&a, b"a".to_vec());

        let entries: Vec<_> = provision.entries().collect();
        assert_eq!(entries, vec![("a", "a", &b"a"[..]), ("b", "z", &b"z"[..])]);
    }
}
