// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-620 and FR-621 (FSpec FR-520): the items of a `native-analyze/1`
//! request document and the `analyze` result document.
//!
//! A document item names its claim by declared name and never carries the
//! members FSL computes (`request_index`, `extent`, `candidates`, the
//! resolved `temporal.fairness`). [`resolve_items`] resolves each item
//! against the checked package into the [`ClaimItem`] `analyze` takes, or
//! settles it on its own (`unknown-claim`, `absent-kind`). The caller
//! negotiates the resolved items ([`Negotiate`]); [`analyze_document`] then
//! runs them and writes the FR-621 document, each item echoing its admitted
//! FR-331 request item ([`ClaimItem::request_item_wire`]) so it can be
//! requested again exactly.

use std::collections::{BTreeMap, BTreeSet};
use std::num::NonZeroU64;

use filament_checked::key::DeclarationKey;
use filament_exact::Cancel;
use filament_semantic_value::catalog::{CatalogCode, Category};
use fsl_foundation::source::provenance::OccurrenceKey;
use fsl_foundation::{BackendId, RequestIndex};
use fsl_package::CheckedPackage;
use fsl_replay::{OutcomeAxes, TerminalRecord, TerminalValue};
use fsl_route::request::{ExtentClassification, ItemKind};
use fsl_route::routing::Disposition;
use fsl_route::{CandidateOutcome, Registry};
use fsl_semantics::check::Capability;
use fsl_semantics::model::observation::{DocumentProvision, DocumentRef};
use serde::Deserialize;
use serde_json::{json, Map, Value};

use super::{
    AnalyzeLimits, ClaimItem, Diagnostic, Engines, ItemForm, ItemOptions, NegotiatedItem,
    SourceProvision, SubjectBinding, TemporalItem,
};
use fsl_eval::simulation::model::PopulationUniverse;

/// The request document format FR-620 reads.
pub const REQUEST_FORMAT: &str = "native-analyze/1";

/// A `{authority, identity}` reference to a provisioned document.
#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd)]
#[serde(deny_unknown_fields)]
pub struct DocumentSelection {
    /// The document's declared authority.
    pub authority: String,
    /// The document's declared identity.
    pub identity: String,
}

impl DocumentSelection {
    fn reference(&self) -> DocumentRef {
        DocumentRef {
            authority: self.authority.clone(),
            identity: self.identity.clone(),
        }
    }

    fn of(reference: &DocumentRef) -> Self {
        Self {
            authority: reference.authority.clone(),
            identity: reference.identity.clone(),
        }
    }

    fn wire(&self) -> Value {
        json!({"authority": self.authority, "identity": self.identity})
    }
}

/// FSpec FR-520's model subject `{initial_states, universes, proof_bounds}`:
/// the snapshots the subject starts from and the snapshots whose complete
/// populations are its universes.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SubjectSelection {
    /// The initial-state snapshots, in request order.
    pub initial_states: Vec<DocumentSelection>,
    /// The snapshots whose complete populations are the universes.
    pub universes: Vec<DocumentSelection>,
    /// The substituted proof bounds; FSL reads none yet from a document.
    pub proof_bounds: Vec<Value>,
}

impl SubjectSelection {
    /// FR-620's default subject: every provisioned `initialization`
    /// snapshot, in ascending `{authority, identity}` order, as both the
    /// initial states and the universes, with no proof bound.
    pub fn default_over(documents: &DocumentProvision) -> Self {
        let initial: Vec<DocumentSelection> = documents
            .iter()
            .filter(|(_, bytes)| snapshot(bytes).is_some_and(|read| read.initialization()))
            .map(|(reference, _)| DocumentSelection::of(reference))
            .collect();
        Self {
            universes: initial.clone(),
            initial_states: initial,
            proof_bounds: Vec::new(),
        }
    }

    /// The binding `analyze` takes over this subject: the initial states,
    /// and one universe per complete population the universe snapshots
    /// declare, its keys the union of their objects' keys.
    pub fn binding<'a>(
        &self,
        packages: &'a fsl_semantics::model::intake::PackageInput,
        documents: &'a DocumentProvision,
    ) -> SubjectBinding<'a> {
        let mut universes: BTreeMap<DeclarationKey, BTreeSet<String>> = BTreeMap::new();
        for selection in &self.universes {
            let Some(read) = documents.get(&selection.reference()).and_then(|bytes| snapshot(bytes))
            else {
                continue;
            };
            for population in read.populations.iter().filter(|population| population.complete) {
                universes
                    .entry(DeclarationKey {
                        package: read.model.identity.clone(),
                        node: population.population.clone(),
                    })
                    .or_default()
                    .extend(population.objects.iter().map(|object| object.key.clone()));
            }
        }
        SubjectBinding {
            packages,
            snapshots: documents,
            initial: self
                .initial_states
                .iter()
                .map(DocumentSelection::reference)
                .collect(),
            universes: universes
                .into_iter()
                .map(|(population, keys)| PopulationUniverse {
                    population,
                    keys: keys.into_iter().collect(),
                })
                .collect(),
        }
    }

    fn wire(&self) -> Value {
        json!({
            "initial_states": self.initial_states.iter().map(DocumentSelection::wire).collect::<Vec<_>>(),
            "universes": self.universes.iter().map(DocumentSelection::wire).collect::<Vec<_>>(),
            "proof_bounds": self.proof_bounds,
        })
    }
}

/// The members of a snapshot the default subject reads; the snapshot itself
/// is admitted by the engine's binding.
#[derive(Deserialize)]
struct SnapshotRead {
    model: SnapshotModel,
    anchor: SnapshotAnchor,
    #[serde(default)]
    populations: Vec<SnapshotPopulation>,
}

impl SnapshotRead {
    #[ix_trace_rs::string_edge]
    fn initialization(&self) -> bool {
        self.anchor.kind == "initialization"
    }
}

#[derive(Deserialize)]
struct SnapshotModel {
    identity: String,
}

#[derive(Deserialize)]
struct SnapshotAnchor {
    kind: String,
}

#[derive(Deserialize)]
struct SnapshotPopulation {
    population: String,
    complete: bool,
    #[serde(default)]
    objects: Vec<SnapshotObject>,
}

#[derive(Deserialize)]
struct SnapshotObject {
    key: String,
}

fn snapshot(bytes: &[u8]) -> Option<SnapshotRead> {
    serde_json::from_slice(bytes).ok()
}

/// FSpec FR-368's `item_kind` spellings.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum ItemKindSelection {
    /// An authored clause.
    Formula,
    /// The derived deadlock-freedom item.
    DeadlockFreedom,
    /// The derived time-lock-freedom item.
    TimeLockFreedom,
    /// The derived race-freedom item.
    RaceFreedom,
    /// A refinement item.
    Refinement,
}

impl ItemKindSelection {
    const fn kind(self) -> ItemKind {
        match self {
            Self::Formula => ItemKind::Formula,
            Self::DeadlockFreedom => ItemKind::DeadlockFreedom,
            Self::TimeLockFreedom => ItemKind::TimeLockFreedom,
            Self::RaceFreedom => ItemKind::RaceFreedom,
            Self::Refinement => ItemKind::Refinement,
        }
    }
}

/// FSpec FR-331's `temporal.limits`, each member a positive count.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct TemporalLimitsSelection {
    /// `max_states`.
    pub max_states: Option<NonZeroU64>,
    /// `max_transitions`.
    pub max_transitions: Option<NonZeroU64>,
    /// `max_automaton_states`.
    pub max_automaton_states: Option<NonZeroU64>,
    /// `max_walk_steps`.
    pub max_walk_steps: Option<NonZeroU64>,
    /// `max_witness_set`.
    pub max_witness_set: Option<NonZeroU64>,
    /// `max_relation_tuples`.
    pub max_relation_tuples: Option<NonZeroU64>,
    /// `max_clocks` (FSpec FR-415).
    pub max_clocks: Option<NonZeroU64>,
    /// `max_task_product_work` (FSpec FR-415).
    pub max_task_product_work: Option<NonZeroU64>,
    /// `max_store_buffer` (FSpec FR-439).
    pub max_store_buffer: Option<NonZeroU64>,
    /// `max_messages` (FSpec FR-439).
    pub max_messages: Option<NonZeroU64>,
    /// `max_live_instances` (FSpec FR-428).
    pub max_live_instances: Option<NonZeroU64>,
    /// `max_solver_steps` (FSpec FR-368).
    pub max_solver_steps: Option<NonZeroU64>,
    /// `max_solver_time` (FSpec FR-368).
    pub max_solver_time: Option<NonZeroU64>,
}

impl TemporalLimitsSelection {
    /// Whether a limit no explicit-state EN-1 run reads is set.
    fn sets_other_engine_limits(&self) -> bool {
        [
            self.max_clocks,
            self.max_task_product_work,
            self.max_store_buffer,
            self.max_messages,
            self.max_live_instances,
            self.max_solver_steps,
            self.max_solver_time,
        ]
        .iter()
        .any(Option::is_some)
    }
}

/// FSpec FR-331's `temporal` member of a document item, without the
/// `fairness` FSL resolves from the checked clause.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct TemporalSelection {
    /// The item kind; `formula` when absent.
    #[serde(default)]
    pub item_kind: Option<ItemKindSelection>,
    /// The item's model subject; the request's when absent.
    #[serde(default)]
    pub subject: Option<SubjectSelection>,
    /// A refinement item's abstract subject.
    #[serde(default)]
    pub abstract_subject: Option<Value>,
    /// The stutter-closure premise.
    #[serde(default)]
    pub stutter_closure: Option<bool>,
    /// A bounded search's horizon.
    #[serde(default)]
    pub max_depth: Option<u64>,
    /// The item's own budgets.
    #[serde(default)]
    pub limits: Option<TemporalLimitsSelection>,
    /// The SMT encoding budgets.
    #[serde(default)]
    pub smt_encoding: Option<Value>,
    /// The witness walks per initial state (FSpec FR-392).
    #[serde(default)]
    pub witness_samples: Option<u64>,
    /// The witness-sampling seed (FSpec FR-392).
    #[serde(default)]
    pub seed: Option<Value>,
}

/// FSpec FR-331's `check` member: the case budget of a value-validity or
/// operation-contract item.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CheckSelection {
    /// `limits.max_cases`.
    pub limits: CheckLimitsSelection,
}

/// `check.limits`.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CheckLimitsSelection {
    /// The case budget.
    pub max_cases: NonZeroU64,
}

/// One FSpec FR-520 analysis item, as a document writes it. Closed: an
/// unknown member, and each member FSL computes, refuses the request.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ItemSelection {
    /// The claim's declared name.
    #[serde(default)]
    pub claim: Option<String>,
    /// The FSpec FR-290 kind, or `simulation`.
    #[serde(default)]
    pub kind: Option<String>,
    /// The backend the caller names.
    #[serde(default)]
    pub backend: Option<String>,
    /// Whether the aggregate counts the item; `true` when absent.
    #[serde(default)]
    pub required: Option<bool>,
    /// FSpec FR-331 `temporal`.
    #[serde(default)]
    pub temporal: Option<TemporalSelection>,
    /// FSpec FR-331 `probabilistic`.
    #[serde(default)]
    pub probabilistic: Option<Value>,
    /// FSpec FR-331 `hybrid`.
    #[serde(default)]
    pub hybrid: Option<Value>,
    /// FSpec FR-331 `check`.
    #[serde(default)]
    pub check: Option<CheckSelection>,
    /// FSpec FR-331 `options`.
    #[serde(default)]
    pub options: Option<BTreeMap<String, String>>,
    /// FSpec FR-382 `reductions`, over the unit's declarations (FR-623).
    #[serde(default)]
    pub reductions: Option<Value>,
    /// FSpec FR-331 `correspondence`.
    #[serde(default)]
    pub correspondence: Option<Value>,
    /// FSpec FR-331 `premises`.
    #[serde(default)]
    pub premises: Option<Value>,
    /// FSpec FR-331 `assessed_root`.
    #[serde(default)]
    pub assessed_root: Option<Value>,
    /// A `finite-replay` item's supplied trace.
    #[serde(default)]
    pub trace: Option<DocumentSelection>,
    /// A `simulation` item's settings.
    #[serde(default)]
    pub simulation: Option<Value>,
}

impl ItemSelection {
    /// Whether the item carries a member whose in-process route `analyze`
    /// does not build yet, so the item settles `unsupported` on its own.
    fn unrouted(&self) -> bool {
        let temporal = self.temporal.as_ref();
        self.probabilistic.is_some()
            || self.hybrid.is_some()
            || self.reductions.is_some()
            || self.correspondence.is_some()
            || self.premises.is_some()
            || self.assessed_root.is_some()
            || self.trace.is_some()
            || self.simulation.is_some()
            || self.options.as_ref().is_some_and(|options| !options.is_empty())
            || temporal.is_some_and(|temporal| {
                temporal.abstract_subject.is_some()
                    || temporal.smt_encoding.is_some()
                    || temporal.witness_samples.is_some()
                    || temporal.seed.is_some()
                    || temporal
                        .limits
                        .is_some_and(|limits| limits.sets_other_engine_limits())
            })
    }
}

/// One item as [`resolve_items`] resolved it.
#[derive(Clone)]
pub enum ResolvedItem {
    /// An item `analyze` runs once it is negotiated.
    Claim(Box<ResolvedClaim>),
    /// An item settled without running: its terminal record.
    Settled {
        /// The declared name the item used, when it named one.
        claim_name: Option<String>,
        /// The settled record.
        record: Box<TerminalRecord>,
    },
}

/// A resolved claim item with its declared name and its candidate outcome.
#[derive(Clone)]
pub struct ResolvedClaim {
    /// The item `analyze` takes.
    pub item: ClaimItem,
    /// The declared name of the claim, or a derived item's anchor.
    pub claim_name: String,
    /// The candidates the registry gives the item.
    pub candidates: CandidateOutcome,
    /// The item's model subject.
    pub subject: SubjectSelection,
    /// An item whose own subject differs from the request's, or that
    /// carries a member `analyze` has no in-process route for, settles
    /// `unsupported` without being negotiated.
    pub unrouted: bool,
}

/// The caller's negotiation (FR-281 "Negotiated items"): FSL computes the
/// candidates, and the caller settles each item from them, as CG's
/// `negotiate_*` does in the driver.
pub trait Negotiate {
    /// The disposition and its diagnostic for `item` over `candidates`.
    fn negotiate(
        &self,
        item: &ClaimItem,
        candidates: &CandidateOutcome,
    ) -> (Disposition, Option<Diagnostic>);
}

fn unknown_claim(index: RequestIndex, name: &str) -> ResolvedItem {
    let mut detail = Map::new();
    detail.insert("name".to_owned(), json!(name));
    ResolvedItem::Settled {
        claim_name: Some(name.to_owned()),
        record: Box::new(TerminalRecord::disposition_invalid_request(
            index,
            CatalogCode::new("invalid_capability", "unknown-claim"),
            detail,
        )),
    }
}

fn invalid_kind(index: RequestIndex, name: Option<&str>, cause: &'static str) -> ResolvedItem {
    ResolvedItem::Settled {
        claim_name: name.map(str::to_owned),
        record: Box::new(TerminalRecord::disposition_invalid_request(
            index,
            CatalogCode::new("invalid_capability", cause),
            Map::new(),
        )),
    }
}

/// The checked declarations by their declared qualified name.
fn declarations(package: &CheckedPackage) -> BTreeMap<String, filament_exact::NodeKey> {
    let graph = package.graph();
    let mut names: BTreeMap<String, filament_exact::NodeKey> = declared_nodes(package).collect();
    names.extend(
        graph
            .temporal_clauses()
            .iter()
            .map(|clause| (clause.clause().name().to_owned(), clause.identity())),
    );
    names
}

/// The semantic graph's declaring nodes by qualified name.
fn declared_nodes(
    package: &CheckedPackage,
) -> impl Iterator<Item = (String, filament_exact::NodeKey)> + '_ {
    package
        .graph()
        .semantic_graph()
        .nodes()
        .filter_map(|node| {
            node.declaration().map(|name| {
                let name = name
                    .iter()
                    .map(|segment| segment.as_str())
                    .collect::<Vec<_>>()
                    .join("::");
                (name, node.key())
            })
        })
}

/// The declared name of the declaration `node`, when it declares one.
fn declared_name(package: &CheckedPackage, node: &filament_exact::NodeKey) -> Option<String> {
    declarations(package)
        .into_iter()
        .find(|(_, key)| key == node)
        .map(|(name, _)| name)
}

/// The occurrence a claim item over the declaration `node` names: the
/// occurrence of its requirement record, else its first checked
/// occurrence.
fn claim_occurrence(
    package: &CheckedPackage,
    records: &[fsl_route::request::RequirementItem],
    node: &filament_exact::NodeKey,
) -> Option<OccurrenceKey> {
    if let Some(record) = records.iter().find(|record| record.node() == node) {
        return Some(record.occurrence().clone());
    }
    let graph = package.graph();
    if let Some(clause) = graph
        .temporal_clauses()
        .iter()
        .find(|clause| clause.identity() == *node)
    {
        return Some(OccurrenceKey::new(clause.identity(), clause.claim().clone()));
    }
    graph
        .occurrences()
        .find(|(key, _, _)| key == node)
        .map(|(key, origin, _)| OccurrenceKey::new(key, origin))
}

/// Resolve `selections` (FR-620 "Resolving items") over `package`, the
/// request's default `subject` and the caller's `registry`; `None`
/// selections are FR-620's default items: one per checked claim and kind,
/// then each model subject's derived items. Each result is either an item
/// to negotiate and run or an item already settled.
pub fn resolve_items(
    package: &CheckedPackage,
    subject: &SubjectSelection,
    selections: Option<&[ItemSelection]>,
    registry: &Registry,
) -> Vec<ResolvedItem> {
    match selections {
        Some(selections) => selections
            .iter()
            .enumerate()
            .map(|(position, selection)| {
                resolve_one(
                    package,
                    subject,
                    RequestIndex::new(position),
                    selection,
                    registry,
                )
            })
            .collect(),
        None => default_items(package, subject, registry),
    }
}

fn requirement_items(
    package: &CheckedPackage,
    registry: &Registry,
    named: Option<&BackendId>,
) -> Vec<fsl_route::request::RequirementItem> {
    fsl_route::request::items_from_requirements(package.graph().requirements(), registry, named)
}

fn resolve_one(
    package: &CheckedPackage,
    subject: &SubjectSelection,
    index: RequestIndex,
    selection: &ItemSelection,
    registry: &Registry,
) -> ResolvedItem {
    let name = selection.claim.as_deref();
    let Some(name) = name else {
        // A claimless item is a simulation item (FR-634), which settles no
        // claim and is not negotiated; it has no in-process route yet.
        return ResolvedItem::Settled {
            claim_name: None,
            record: Box::new(TerminalRecord::disposition_unsupported(
                index,
                fsl_route::NegotiationOutcome::UnsupportedRequestedCapability.code(),
                Map::new(),
            )),
        };
    };
    let Some(node) = declarations(package).get(name).cloned() else {
        return unknown_claim(index, name);
    };
    let named = selection.backend.as_deref().map(BackendId::new);
    let records = requirement_items(package, registry, named.as_ref());
    let own: Vec<&fsl_route::request::RequirementItem> =
        records.iter().filter(|record| record.node() == &node).collect();
    let kind = match selection.kind.as_deref() {
        Some(label) => match Capability::from_wire(label) {
            Ok(kind) => kind,
            Err(_) => return invalid_kind(index, Some(name), "unknown-kind"),
        },
        None => {
            let mut kinds: Vec<Capability> = Vec::new();
            for record in &own {
                if !kinds.contains(&record.kind()) {
                    kinds.push(record.kind());
                }
            }
            match kinds.as_slice() {
                [kind] => *kind,
                _ => return invalid_kind(index, Some(name), "absent-kind"),
            }
        }
    };
    let Some(claim) = claim_occurrence(package, &records, &node) else {
        return unknown_claim(index, name);
    };
    let record = own.iter().find(|record| record.kind() == kind);
    let extent = record.map_or(ExtentClassification::Bounded, |record| record.extent());
    let candidates = record.map_or_else(
        || registry.candidates(kind, named.as_ref()),
        |record| record.candidates().clone(),
    );
    let candidates = if kind == Capability::TemporalSatisfaction {
        registry.restrict_to_timing(candidates, timed(package))
    } else {
        candidates
    };
    let temporal = selection.temporal.clone().unwrap_or_default();
    let own_subject = temporal.subject.clone().unwrap_or_else(|| subject.clone());
    let form = (kind == Capability::TemporalSatisfaction)
        .then(|| ItemForm::Temporal(temporal_item(package, &claim, &temporal)));
    let item = ClaimItem {
        request_index: index,
        claim,
        kind,
        extent,
        backend: named,
        candidates: candidate_list(&candidates),
        options: ItemOptions {
            required: selection.required.unwrap_or(true),
            max_cases: selection.check.map(|check| check.limits.max_cases),
        },
        form,
    };
    ResolvedItem::Claim(Box::new(ResolvedClaim {
        item,
        claim_name: name.to_owned(),
        candidates,
        unrouted: selection.unrouted() || own_subject != *subject,
        subject: own_subject,
    }))
}

/// Whether the model subject the package's selections give is timed
/// (FR-243): a selected model declares a time source.
fn timed(package: &CheckedPackage) -> bool {
    let graph = package.graph();
    let selections: BTreeSet<&str> = graph
        .model_selections()
        .iter()
        .map(|selection| selection.identity.as_str())
        .collect();
    fsl_route::deadlock::SubjectReading::read(graph, |identity| selections.contains(identity))
        .timed()
}

fn candidate_list(candidates: &CandidateOutcome) -> Vec<fsl_route::Candidate> {
    match candidates {
        CandidateOutcome::Candidates(set) => set.candidates().to_vec(),
        CandidateOutcome::UnknownBackend(_) => Vec::new(),
    }
}

/// The temporal form of an item over `claim`: the clause's resolved
/// fairness, and each limit the item sets over the published default.
fn temporal_item(
    package: &CheckedPackage,
    claim: &OccurrenceKey,
    selection: &TemporalSelection,
) -> TemporalItem {
    let mut item = TemporalItem {
        item_kind: selection
            .item_kind
            .map_or(ItemKind::Formula, ItemKindSelection::kind),
        stutter_closure: selection.stutter_closure,
        max_depth: selection
            .max_depth
            .and_then(|depth| usize::try_from(depth).ok()),
        ..TemporalItem::default()
    };
    if item.item_kind == ItemKind::Formula {
        if let Some(clause) = package
            .graph()
            .temporal_clauses()
            .iter()
            .find(|clause| clause.identity() == *claim.node())
        {
            item.fairness = clause.clause().fairness().to_vec();
        }
    }
    if let Some(limits) = &selection.limits {
        let set = |value: Option<NonZeroU64>, slot: &mut u64| {
            if let Some(value) = value {
                *slot = value.get();
            }
        };
        let set_usize = |value: Option<NonZeroU64>, slot: &mut usize| {
            if let Some(value) = value.and_then(|value| usize::try_from(value.get()).ok()) {
                *slot = value;
            }
        };
        set_usize(limits.max_states, &mut item.limits.limits.max_states);
        set_usize(
            limits.max_transitions,
            &mut item.limits.limits.max_transitions,
        );
        set(
            limits.max_automaton_states,
            &mut item.limits.max_automaton_states,
        );
        set(limits.max_walk_steps, &mut item.limits.max_walk_steps);
        set(limits.max_witness_set, &mut item.limits.max_witness_set);
        set(
            limits.max_relation_tuples,
            &mut item.limits.max_relation_tuples,
        );
    }
    item
}

/// FR-620's default items: one per checked claim and kind, temporal clauses
/// first in their checked order and then every other requirement record in
/// its key order, followed by the derived items of the model subject.
fn default_items(
    package: &CheckedPackage,
    subject: &SubjectSelection,
    registry: &Registry,
) -> Vec<ResolvedItem> {
    let graph = package.graph();
    let records = requirement_items(package, registry, None);
    let mut nodes: Vec<filament_exact::NodeKey> = graph
        .temporal_clauses()
        .iter()
        .map(|clause| clause.identity())
        .collect();
    for record in &records {
        if !nodes.contains(record.node()) {
            nodes.push(record.node().clone());
        }
    }
    let mut resolved = Vec::new();
    for node in &nodes {
        let Some(name) = declared_name(package, node) else {
            continue;
        };
        let kinds: Vec<Capability> = records
            .iter()
            .filter(|record| record.node() == node)
            .map(|record| record.kind())
            .collect();
        for kind in Capability::ALL.iter().filter(|kind| kinds.contains(kind)) {
            let selection = ItemSelection {
                claim: Some(name.clone()),
                kind: Some(kind.to_wire().to_owned()),
                ..ItemSelection::default()
            };
            resolved.push(resolve_one(
                package,
                subject,
                RequestIndex::new(resolved.len()),
                &selection,
                registry,
            ));
        }
    }
    resolved.extend(derived_items(
        package,
        subject,
        registry,
        &resolved,
        resolved.len(),
    ));
    resolved
}

/// FSpec FR-368's derived items of the subject the authored temporal items
/// of `authored` are over, numbered from `first`.
fn derived_items(
    package: &CheckedPackage,
    subject: &SubjectSelection,
    registry: &Registry,
    authored: &[ResolvedItem],
    first: usize,
) -> Vec<ResolvedItem> {
    let graph = package.graph();
    let selections: BTreeSet<&str> = graph
        .model_selections()
        .iter()
        .map(|selection| selection.identity.as_str())
        .collect();
    let mut request = fsl_route::model_request::ModelRequest::<()>::new();
    let occurrences: Vec<OccurrenceKey> = authored
        .iter()
        .filter_map(|resolved| match resolved {
            ResolvedItem::Claim(claim)
                if claim.item.kind == Capability::TemporalSatisfaction
                    && graph
                        .temporal_clauses()
                        .iter()
                        .any(|clause| clause.identity() == *claim.item.claim.node()) =>
            {
                Some(claim.item.claim.clone())
            }
            ResolvedItem::Claim(_) | ResolvedItem::Settled { .. } => None,
        })
        .collect();
    let requirements = fsl_semantics::family::Requirements::new(
        Capability::TemporalSatisfaction,
        fsl_semantics::family::ClaimExtent::Bounded,
    );
    for occurrence in &occurrences {
        request.authored(
            &fsl_route::deadlock::ModelTemporalItem {
                occurrence,
                subject: &(),
                package: graph,
            },
            &requirements,
            &selections,
        );
    }
    let written = request.derived(
        registry,
        None,
        &fsl_route::request::DerivedLimits::default(),
    );
    written
        .items()
        .iter()
        .filter(|item| {
            item.item_kind()
                .is_some_and(|kind| kind != ItemKind::Formula)
        })
        .enumerate()
        .map(|(offset, item)| {
            let index = RequestIndex::new(first + offset);
            let candidates = registry.restrict_to_timing(
                item.candidates()
                    .cloned()
                    .unwrap_or_else(|| registry.candidates(item.kind(), None)),
                timed(package),
            );
            let claim_name = declared_name(package, item.node()).unwrap_or_default();
            ResolvedItem::Claim(Box::new(ResolvedClaim {
                item: ClaimItem {
                    request_index: index,
                    claim: item.occurrence().clone(),
                    kind: item.kind(),
                    extent: item.extent(),
                    backend: None,
                    candidates: candidate_list(&candidates),
                    options: ItemOptions {
                        required: true,
                        max_cases: None,
                    },
                    form: Some(ItemForm::Temporal(TemporalItem {
                        item_kind: item.item_kind().unwrap_or(ItemKind::Formula),
                        ..TemporalItem::default()
                    })),
                },
                claim_name,
                candidates,
                subject: subject.clone(),
                unrouted: false,
            }))
        })
        .collect()
}

impl ClaimItem {
    /// FR-620's canonical FSpec FR-331 request item of this item over
    /// `subject`: `request_index`, the `claim` occurrence key, the `kind`,
    /// the `extent`, the `candidates`, a named `backend`, and for a temporal
    /// item its `subject`, `item_kind`, resolved `fairness`, `max_depth`
    /// and every `limits` member made explicit.
    ///
    /// # Errors
    ///
    /// When the claim's node key has no wire spelling.
    pub fn request_item_wire(&self, subject: &SubjectSelection) -> Result<Value, serde_json::Error> {
        let mut item = Map::new();
        item.insert("request_index".to_owned(), json!(self.request_index.get()));
        item.insert("kind".to_owned(), json!(self.kind.to_wire()));
        item.insert(
            "claim".to_owned(),
            fsl_replay::claim_locus_wire(&self.claim)?,
        );
        item.insert(
            "extent".to_owned(),
            match self.extent {
                ExtentClassification::Bounded => {
                    json!({"classification": "bounded", "bounds": []})
                }
                ExtentClassification::Unbounded {
                    finite_bound_available,
                } => json!({"classification": "unbounded",
                    "finite_bound_available": finite_bound_available}),
            },
        );
        item.insert(
            "candidates".to_owned(),
            json!({"type": "set", "backends": self
                .candidates
                .iter()
                .map(|candidate| candidate.id().as_str())
                .collect::<Vec<_>>()}),
        );
        if let Some(backend) = &self.backend {
            item.insert("backend".to_owned(), json!(backend.as_str()));
        }
        if let Some(max_cases) = self.options.max_cases {
            item.insert(
                "check".to_owned(),
                json!({"limits": {"max_cases": max_cases.get()}}),
            );
        }
        if let Some(ItemForm::Temporal(temporal)) = &self.form {
            let mut member = Map::new();
            member.insert("subject".to_owned(), subject.wire());
            member.insert(
                "item_kind".to_owned(),
                json!(temporal.item_kind.to_wire()),
            );
            member.insert(
                "fairness".to_owned(),
                Value::Array(temporal.fairness.iter().map(fairness_wire).collect()),
            );
            if let Some(stutter) = temporal.stutter_closure {
                member.insert("stutter_closure".to_owned(), json!(stutter));
            }
            if let Some(depth) = temporal.max_depth {
                member.insert("max_depth".to_owned(), json!(depth));
            }
            let limits = &temporal.limits;
            member.insert(
                "limits".to_owned(),
                json!({
                    "max_states": limits.limits.max_states,
                    "max_transitions": limits.limits.max_transitions,
                    "max_automaton_states": limits.max_automaton_states,
                    "max_walk_steps": limits.max_walk_steps,
                    "max_witness_set": limits.max_witness_set,
                    "max_relation_tuples": limits.max_relation_tuples,
                }),
            );
            item.insert("temporal".to_owned(), Value::Object(member));
        }
        Ok(Value::Object(item))
    }
}

/// FSpec FR-364's `{kind, operation, granularity}` fairness constraint.
fn fairness_wire(constraint: &filament_semantic_value::fairness::FairnessConstraint) -> Value {
    quire_canonical::to_vec(constraint, quire_canonical::Limits::new(u64::MAX))
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or(Value::Null)
}

/// What one request settles for the FR-621 document.
pub struct AnalyzeRun<'a> {
    /// The checked package.
    pub package: &'a CheckedPackage,
    /// The request's shared subject binding.
    pub binding: SubjectBinding<'a>,
    /// Source bytes under every label the package names.
    pub provision: SourceProvision,
    /// The resolved items, in request order.
    pub items: Vec<ResolvedItem>,
    /// The engines `analyze` may run.
    pub engines: &'a Engines,
    /// The shared limits.
    pub limits: AnalyzeLimits,
}

/// Why no FR-621 document could be written.
#[derive(Debug, thiserror::Error)]
pub enum AnalyzeDocumentError {
    /// The shared subject or source provision was refused before any item
    /// ran (FR-624).
    #[error(transparent)]
    Refused(#[from] super::AnalyzeRefusal),
    /// An internal fault stopped the run.
    #[error("the analyze run faulted")]
    Fault,
}

/// Negotiate `run`'s items with `negotiate`, analyze them under `cancel`
/// and write FR-621's `filament-outcome/1` document with its FR-285 exit
/// code.
///
/// # Errors
///
/// [`AnalyzeDocumentError::Refused`] for a whole-request refusal (the
/// caller writes FR-624's refusal document), and
/// [`AnalyzeDocumentError::Fault`] for an internal fault.
pub fn analyze_document(
    run: AnalyzeRun<'_>,
    negotiate: &dyn Negotiate,
    cancel: &Cancel,
) -> Result<(Value, u8), AnalyzeDocumentError> {
    let AnalyzeRun {
        package,
        binding,
        provision,
        items,
        engines,
        limits,
    } = run;
    let mut negotiated = Vec::new();
    let mut settled: BTreeMap<RequestIndex, (Option<String>, TerminalRecord)> = BTreeMap::new();
    let mut echoes: BTreeMap<RequestIndex, (String, Value, bool, bool)> = BTreeMap::new();
    for resolved in items {
        match resolved {
            ResolvedItem::Settled { claim_name, record } => {
                settled.insert(record.request_index(), (claim_name, *record));
            }
            ResolvedItem::Claim(claim) => {
                let ResolvedClaim {
                    item,
                    claim_name,
                    candidates,
                    subject,
                    unrouted,
                } = *claim;
                let echo = item
                    .request_item_wire(&subject)
                    .map_err(|_| AnalyzeDocumentError::Fault)?;
                let scoped = has_activation_scope(item.kind);
                echoes.insert(
                    item.request_index,
                    (claim_name, echo, item.options.required, scoped),
                );
                if unrouted {
                    let mut detail = Map::new();
                    detail.insert("kind".to_owned(), json!(item.kind.to_wire()));
                    settled.insert(
                        item.request_index,
                        (
                            None,
                            TerminalRecord::disposition_unsupported(
                                item.request_index,
                                fsl_route::NegotiationOutcome::UnsupportedRequestedCapability
                                    .code(),
                                detail,
                            ),
                        ),
                    );
                    continue;
                }
                let (disposition, diagnostic) = negotiate.negotiate(&item, &candidates);
                negotiated.push(NegotiatedItem {
                    item,
                    disposition,
                    diagnostic,
                });
            }
        }
    }
    let outcome = super::request::analyze_bound(
        package,
        &binding,
        &provision,
        &negotiated,
        engines,
        &limits,
        cancel,
    )
    .map_err(|failure| match failure {
        fsl_foundation::diagnostic::StageFailure::Refused(refusal) => {
            AnalyzeDocumentError::Refused(refusal)
        }
        _ => AnalyzeDocumentError::Fault,
    })?
    .into_value();
    let mut records: Vec<TerminalRecord> = outcome.records;
    records.extend(settled.values().map(|(_, record)| record.clone()));
    records.sort_by_key(TerminalRecord::request_index);
    let base = super::analyze_document(&records, &outcome.counterexamples)
        .map_err(|_| AnalyzeDocumentError::Fault)?;
    let mut document =
        serde_json::to_value(&base).map_err(|_| AnalyzeDocumentError::Fault)?;
    let written: Vec<Value> = document["items"].as_array().cloned().unwrap_or_default();
    let mut items = Vec::with_capacity(records.len());
    let mut gaps = Vec::new();
    for (record, written) in records.iter().zip(written) {
        let index = record.request_index();
        let detail = outcome
            .details
            .iter()
            .find(|detail| detail.request_index == index);
        let (claim_name, request, required, scoped) = match echoes.get(&index) {
            Some((name, request, required, scoped)) => {
                (Some(name.clone()), Some(request.clone()), *required, *scoped)
            }
            None => (
                settled.get(&index).and_then(|(name, _)| name.clone()),
                None,
                true,
                false,
            ),
        };
        let disposed = record.disposition_detail().is_some();
        if required
            && !matches!(
                record.value(),
                TerminalValue::Proved { .. } | TerminalValue::Measured(_)
            )
        {
            gaps.push(json!({"request_index": index.get(),
                "reason": written["result"].as_str().unwrap_or("invalid-request")}));
        }
        items.push(item_document(
            written,
            claim_name,
            request,
            detail.copied(),
            scoped,
            OutcomeAxes::of_record(record.value(), disposed),
        ));
    }
    document["items"] = Value::Array(items);
    document["aggregate"] = json!({
        "status": if gaps.is_empty() { "complete" } else { "incomplete" },
        "gaps": gaps,
    });
    document["registry"] = Value::Array(
        engines
            .registrations()
            .map(|descriptor| json!(descriptor.id().as_str()))
            .collect(),
    );
    let category = Category::most_severe(records.iter().map(|record| record.value().category().exit_code()))
        .unwrap_or(0);
    Ok((document, category))
}

/// Whether an item of `kind` has an activation scope, so it carries
/// FR-622's `axes`.
const fn has_activation_scope(kind: Capability) -> bool {
    !matches!(
        kind,
        Capability::ValueValidity
            | Capability::OperationContract
            | Capability::Schedulability
            | Capability::ProbabilisticSatisfaction
    )
}

/// FR-621: beside each state key of a counterexample, the state it names,
/// whose values are FSpec FR-181's typed canonical forms. A state key is the
/// unpadded base64url spelling of the state's canonical bytes, so the state
/// is read back from the key and nothing is recomputed.
#[ix_trace_rs::string_edge]
fn spell_states(value: &mut Value) {
    use base64::Engine;
    match value {
        Value::Object(members) => {
            let state = members
                .get("key")
                .and_then(Value::as_str)
                .and_then(|key| {
                    base64::engine::general_purpose::URL_SAFE_NO_PAD
                        .decode(key)
                        .ok()
                })
                .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok())
                .filter(|state| state["type"] == "simulation-state");
            if let Some(state) = state {
                members.insert("state".to_owned(), state);
            }
            for member in members.values_mut() {
                spell_states(member);
            }
        }
        Value::Array(entries) => entries.iter_mut().for_each(spell_states),
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {}
    }
}

/// One FR-621 item from the FR-286 item `written`.
fn item_document(
    written: Value,
    claim_name: Option<String>,
    request: Option<Value>,
    detail: Option<super::ItemDetail>,
    scoped: bool,
    axes: OutcomeAxes,
) -> Value {
    let Value::Object(mut record) = written else {
        return written;
    };
    let index = record.remove("request_index").unwrap_or(Value::Null);
    let category = record.remove("category").unwrap_or(Value::Null);
    let counterexample = record.remove("counterexample");
    let mut item = Map::new();
    item.insert("request_index".to_owned(), index);
    item.insert(
        "claim_name".to_owned(),
        claim_name.map_or(Value::Null, Value::String),
    );
    item.insert("request".to_owned(), request.unwrap_or(Value::Null));
    item.insert("category".to_owned(), category);
    if let (Some(checker), Some("proved")) = (
        detail.and_then(|detail| detail.checker),
        record.get("result").and_then(Value::as_str),
    ) {
        item.insert(
            "certificate".to_owned(),
            json!({"checker": checker.as_str(), "accepted": true}),
        );
    }
    if let Some(mut counterexample) = counterexample {
        spell_states(&mut counterexample);
        item.insert(
            "replay".to_owned(),
            json!({"target": {"state_clause": item["claim_name"].clone()}, "reproduced": true}),
        );
        item.insert("counterexample".to_owned(), counterexample);
    }
    if let Some(stats) = detail.and_then(|detail| detail.exploration) {
        item.insert(
            "exploration".to_owned(),
            json!({
                "states": stats.product_states.to_string(),
                "transitions": stats
                    .product_edges
                    .saturating_sub(stats.stutter_edges)
                    .to_string(),
                "automaton_states": stats.automaton_states.to_string(),
            }),
        );
    }
    if scoped {
        item.insert(
            "axes".to_owned(),
            serde_json::to_value(axes).unwrap_or(Value::Null),
        );
    }
    item.insert("record".to_owned(), Value::Object(record));
    Value::Object(item)
}

