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
use fsl_foundation::source::provenance::{OccurrenceKey, RawSourceRef};
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
    /// The document reference this selection names.
    pub fn reference(&self) -> DocumentRef {
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

    /// FR-620: every `universes` entry names a provisioned snapshot, and
    /// every population those snapshots hold is complete, because a universe
    /// is a complete population (FSpec FR-331).
    ///
    /// # Errors
    ///
    /// [`UniverseRefusal::Unprovisioned`] naming the first entry no
    /// provisioned snapshot answers, and [`UniverseRefusal::Incomplete`]
    /// naming the first population a referenced snapshot marks incomplete.
    pub fn check_universes(&self, documents: &DocumentProvision) -> Result<(), UniverseRefusal> {
        for (position, selection) in self.universes.iter().enumerate() {
            let Some(read) = documents
                .get(&selection.reference())
                .and_then(|bytes| snapshot(bytes))
            else {
                return Err(UniverseRefusal::Unprovisioned {
                    position,
                    entry: selection.clone(),
                });
            };
            if let Some(population) = read
                .populations
                .iter()
                .find(|population| !population.complete)
            {
                return Err(UniverseRefusal::Incomplete {
                    population: population.population.clone(),
                });
            }
        }
        Ok(())
    }

    /// The binding `analyze` takes over this subject: the initial states,
    /// and one universe per complete population the universe snapshots
    /// declare, its keys the union of their objects' keys in reference order.
    /// An explicit subject's references were checked with
    /// [`Self::check_universes`]; the default subject leaves an incomplete
    /// population out (FR-620).
    pub fn binding<'a>(
        &self,
        packages: &'a fsl_semantics::model::intake::PackageInput,
        documents: &'a DocumentProvision,
    ) -> SubjectBinding<'a> {
        let mut universes: BTreeMap<DeclarationKey, (Vec<String>, BTreeSet<String>)> =
            BTreeMap::new();
        for read in self.universes.iter().filter_map(|selection| {
            documents
                .get(&selection.reference())
                .and_then(|bytes| snapshot(bytes))
        }) {
            for population in read
                .populations
                .iter()
                .filter(|population| population.complete)
            {
                let (keys, seen) = universes
                    .entry(DeclarationKey {
                        package: read.model.identity.clone(),
                        node: population.population.clone(),
                    })
                    .or_default();
                for object in &population.objects {
                    if seen.insert(object.key.clone()) {
                        keys.push(object.key.clone());
                    }
                }
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
                .map(|(population, (keys, _))| PopulationUniverse { population, keys })
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

/// Why a request's `subject.universes` cannot form the subject's universes
/// (FR-620).
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum UniverseRefusal {
    /// The entry at `position` names no provisioned snapshot document
    /// (`invalid-request`).
    #[error("subject.universes[{position}] {{{}, {}}} names no provisioned snapshot", entry.authority, entry.identity)]
    Unprovisioned {
        /// The entry's position in `subject.universes`.
        position: usize,
        /// The entry.
        entry: DocumentSelection,
    },
    /// A referenced snapshot marks `population` incomplete
    /// (`incomplete_population`).
    #[error("population {population} of a universe snapshot is incomplete")]
    Incomplete {
        /// The population's declaration identity.
        population: String,
    },
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

/// The largest count a request member carries: every JSON reader holds it
/// exactly (FSpec FR-520), and it fits the `usize` an explicit-state engine
/// counts in on every supported target.
const MAX_COUNT: u64 = 9_007_199_254_740_991;

/// Read one count member named `member`: a JSON integer from `minimum` to
/// 2^53 - 1 that fits `usize`. A larger value refuses the whole request
/// naming the member, never a silently dropped bound (FR-620).
fn count<'de, D: serde::Deserializer<'de>>(
    decoder: D,
    member: &str,
    minimum: u64,
) -> Result<u64, D::Error> {
    let value = u64::deserialize(decoder)?;
    if value < minimum || value > MAX_COUNT || usize::try_from(value).is_err() {
        return Err(serde::de::Error::custom(format!(
            "{member} {value} is not a count from {minimum} to {MAX_COUNT}"
        )));
    }
    Ok(value)
}

/// Generate the `deserialize_with` reader of each named count member.
macro_rules! count_members {
    ($($reader:ident => $member:literal),+ $(,)?) => {
        $(
            fn $reader<'de, D: serde::Deserializer<'de>>(
                decoder: D,
            ) -> Result<Option<NonZeroU64>, D::Error> {
                count(decoder, $member, 1).map(NonZeroU64::new)
            }
        )+
    };
}

count_members! {
    read_max_states => "temporal.limits.max_states",
    read_max_transitions => "temporal.limits.max_transitions",
    read_max_automaton_states => "temporal.limits.max_automaton_states",
    read_max_walk_steps => "temporal.limits.max_walk_steps",
    read_max_witness_set => "temporal.limits.max_witness_set",
    read_max_relation_tuples => "temporal.limits.max_relation_tuples",
    read_max_clocks => "temporal.limits.max_clocks",
    read_max_task_product_work => "temporal.limits.max_task_product_work",
    read_max_store_buffer => "temporal.limits.max_store_buffer",
    read_max_messages => "temporal.limits.max_messages",
    read_max_live_instances => "temporal.limits.max_live_instances",
    read_max_solver_steps => "temporal.limits.max_solver_steps",
    read_max_solver_time => "temporal.limits.max_solver_time",
}

/// `temporal.max_depth`: a count from 0 to 2^53 - 1 that fits `usize`.
fn read_max_depth<'de, D: serde::Deserializer<'de>>(decoder: D) -> Result<Option<u64>, D::Error> {
    count(decoder, "temporal.max_depth", 0).map(Some)
}

/// FSpec FR-331's `temporal.limits`, each member a positive count.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct TemporalLimitsSelection {
    /// `max_states`.
    #[serde(default, deserialize_with = "read_max_states")]
    pub max_states: Option<NonZeroU64>,
    /// `max_transitions`.
    #[serde(default, deserialize_with = "read_max_transitions")]
    pub max_transitions: Option<NonZeroU64>,
    /// `max_automaton_states`.
    #[serde(default, deserialize_with = "read_max_automaton_states")]
    pub max_automaton_states: Option<NonZeroU64>,
    /// `max_walk_steps`.
    #[serde(default, deserialize_with = "read_max_walk_steps")]
    pub max_walk_steps: Option<NonZeroU64>,
    /// `max_witness_set`.
    #[serde(default, deserialize_with = "read_max_witness_set")]
    pub max_witness_set: Option<NonZeroU64>,
    /// `max_relation_tuples`.
    #[serde(default, deserialize_with = "read_max_relation_tuples")]
    pub max_relation_tuples: Option<NonZeroU64>,
    /// `max_clocks` (FSpec FR-415).
    #[serde(default, deserialize_with = "read_max_clocks")]
    pub max_clocks: Option<NonZeroU64>,
    /// `max_task_product_work` (FSpec FR-415).
    #[serde(default, deserialize_with = "read_max_task_product_work")]
    pub max_task_product_work: Option<NonZeroU64>,
    /// `max_store_buffer` (FSpec FR-439).
    #[serde(default, deserialize_with = "read_max_store_buffer")]
    pub max_store_buffer: Option<NonZeroU64>,
    /// `max_messages` (FSpec FR-439).
    #[serde(default, deserialize_with = "read_max_messages")]
    pub max_messages: Option<NonZeroU64>,
    /// `max_live_instances` (FSpec FR-428).
    #[serde(default, deserialize_with = "read_max_live_instances")]
    pub max_live_instances: Option<NonZeroU64>,
    /// `max_solver_steps` (FSpec FR-368).
    #[serde(default, deserialize_with = "read_max_solver_steps")]
    pub max_solver_steps: Option<NonZeroU64>,
    /// `max_solver_time` (FSpec FR-368).
    #[serde(default, deserialize_with = "read_max_solver_time")]
    pub max_solver_time: Option<NonZeroU64>,
}

impl TemporalLimitsSelection {
    /// Whether a limit no explicit-state EN-1 run reads is set.
    fn sets_other_engine_limits(&self) -> bool {
        [
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
    #[serde(default, deserialize_with = "read_max_depth")]
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

impl TemporalSelection {
    /// The witness-sampling seed as an unsigned count: a JSON integer or its
    /// decimal string; `None` when absent or neither.
    fn seed_count(&self) -> Option<u64> {
        match self.seed.as_ref()? {
            Value::Number(number) => number.as_u64(),
            Value::String(text) => text.parse().ok(),
            _ => None,
        }
    }
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
        self.unrouted_beyond_sampling()
            || self.temporal.as_ref().is_some_and(|temporal| {
                temporal.witness_samples.is_some() || temporal.seed.is_some()
            })
    }

    /// [`Self::unrouted`] without the witness sampling members, which a
    /// state-graph claim reads (FR-628).
    fn unrouted_beyond_sampling(&self) -> bool {
        let temporal = self.temporal.as_ref();
        self.probabilistic.is_some()
            || self.hybrid.is_some()
            || self.correspondence.is_some()
            || self.premises.is_some()
            || self.assessed_root.is_some()
            || (self.trace.is_some()
                && self.kind.as_deref() != Some(Capability::FiniteReplay.to_wire()))
            || self.simulation.is_some()
            || self
                .options
                .as_ref()
                .is_some_and(|options| !options.is_empty())
            || temporal.is_some_and(|temporal| {
                temporal.abstract_subject.is_some()
                    || temporal.smt_encoding.is_some()
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

/// FR-628 "Refusals": `witness_samples` on a claim that is not `possible`,
/// and a `seed` that is no unsigned count, refuse the item `invalid-request`
/// naming the member.
fn sampling_refusal(
    index: RequestIndex,
    name: &str,
    claim: &fsl_semantics::check::CheckedStateGraphClaim,
    temporal: &TemporalSelection,
) -> Option<ResolvedItem> {
    let member = if temporal.witness_samples.is_some()
        && !matches!(
            claim.form(),
            fsl_semantics::check::StateGraphForm::Possible { .. }
        ) {
        "temporal.witness_samples"
    } else if temporal.seed.is_some() && temporal.seed_count().is_none() {
        "temporal.seed"
    } else {
        return None;
    };
    let mut detail = Map::new();
    detail.insert("member".to_owned(), json!(member));
    Some(ResolvedItem::Settled {
        claim_name: Some(name.to_owned()),
        record: Box::new(TerminalRecord::disposition_invalid_request(
            index,
            CatalogCode::new("invalid_runtime_input", "invalid-value"),
            detail,
        )),
    })
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
    names.extend(
        graph.state_graph_claims().iter().filter_map(|claim| {
            Some((claim.name().to_owned(), claim.occurrence()?.node().clone()))
        }),
    );
    names
}

/// Whether the claim at `occurrence` is a state-graph claim (FR-165).
pub(super) fn is_state_graph(package: &CheckedPackage, occurrence: &OccurrenceKey) -> bool {
    package
        .graph()
        .state_graph_claims()
        .iter()
        .any(|claim| claim.occurrence() == Some(occurrence))
}

/// The semantic graph's declaring nodes by qualified name.
fn declared_nodes(
    package: &CheckedPackage,
) -> impl Iterator<Item = (String, filament_exact::NodeKey)> + '_ {
    package.graph().semantic_graph().nodes().filter_map(|node| {
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
/// occurrence of its requirement record; for a `Boolean` function, the
/// Boolean value clause its body is (FSpec FR-290, FR-484 "Forms"), at its
/// `claim` or `declaration` occurrence; else its first checked occurrence.
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
        return Some(OccurrenceKey::new(
            clause.identity(),
            clause.claim().clone(),
        ));
    }
    let body = graph
        .semantic_graph()
        .node(node.clone())
        .and_then(|declaration| declaration.function_body());
    if let Some(body) = body {
        let clause = graph
            .occurrences()
            .filter(|(key, origin, _)| {
                *key == body && matches!(origin.role().as_str(), "claim" | "declaration")
            })
            .map(|(key, origin, _)| OccurrenceKey::new(key, origin))
            .find(|clause| crate::claim_family::boolean_expression(graph, clause));
        if clause.is_some() {
            return clause;
        }
    }
    graph
        .occurrences()
        .find(|(key, _, _)| key == node)
        .map(|(key, origin, _)| OccurrenceKey::new(key, origin))
}

/// The extent of the Boolean value clause `claim` that the function `name`
/// (declared at `node`) is: the domains of its parameters, the arguments
/// the clause ranges over (FSpec FR-290, FR-484). A claim that is not that
/// clause, or whose parameter types do not classify, is `bounded`, and its
/// item is refused by the checker that selects it.
fn value_clause_extent(
    package: &CheckedPackage,
    name: &str,
    node: &filament_exact::NodeKey,
    claim: &OccurrenceKey,
) -> ExtentClassification {
    let graph = package.graph();
    let declaration = graph.semantic_graph().node(node.clone());
    let is_clause = declaration
        .as_ref()
        .and_then(|declaration| declaration.function_body())
        .is_some_and(|body| body == *claim.node());
    let (Some(callable), Some(parameters), true) = (
        graph.callable(name),
        declaration.and_then(|declaration| declaration.function_parameters()),
        is_clause,
    ) else {
        return ExtentClassification::Bounded;
    };
    let roots: Vec<(filament_exact::NodeKey, &filament_exact::ValueType)> = parameters
        .into_iter()
        .zip(callable.parameters.iter().map(|(_, value_type)| value_type))
        .collect();
    fsl_semantics::family::requirements::classify_extent(&roots, graph.scope().types(), u64::MAX)
        .map_or(ExtentClassification::Bounded, |extent| {
            ExtentClassification::of(&extent)
        })
}

/// Resolve `selections` (FR-620 "Resolving items") over `package`, the
/// request's default `subject` and the caller's `registry`; `None`
/// selections are FR-620's default items: one per checked claim and kind,
/// then each model subject's derived items, the claims ordered by `units`,
/// the request's source units in request order (the program, then each
/// library as listed). Each result is either an item to negotiate and run
/// or an item already settled.
pub fn resolve_items(
    package: &CheckedPackage,
    subject: &SubjectSelection,
    selections: Option<&[ItemSelection]>,
    registry: &Registry,
    units: &[RawSourceRef],
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
        None => default_items(package, subject, registry, units),
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
        return resolve_derived(package, subject, index, selection, registry, name)
            .unwrap_or_else(|| unknown_claim(index, name));
    };
    let state_graph = package.graph().state_graph_claim(name).filter(|claim| {
        claim
            .occurrence()
            .is_some_and(|found| found.node() == &node)
    });
    let named = selection.backend.as_deref().map(BackendId::new);
    let records = requirement_items(package, registry, named.as_ref());
    let Some(claim) = claim_occurrence(package, &records, &node) else {
        return unknown_claim(index, name);
    };
    // A record keys the declaration, or the value clause a `Boolean`
    // function's name resolves to.
    let own: Vec<&fsl_route::request::RequirementItem> = records
        .iter()
        .filter(|record| record.node() == &node || record.occurrence() == &claim)
        .collect();
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
                [] if state_graph.is_some() => Capability::TemporalSatisfaction,
                _ => return invalid_kind(index, Some(name), "absent-kind"),
            }
        }
    };
    let record = own.iter().find(|record| record.kind() == kind);
    let extent = match record {
        Some(record) => record.extent(),
        None if kind == Capability::ValueValidity => {
            value_clause_extent(package, name, &node, &claim)
        }
        None => ExtentClassification::Bounded,
    };
    let candidates = record.map_or_else(
        || registry.candidates(kind, named.as_ref()),
        |record| record.candidates().clone(),
    );
    let candidates = if let Some(claim) = state_graph {
        registry.restrict_to_state_graph_form(
            registry.restrict_to_timing(candidates, timed(package)),
            fsl_route::StateGraphFormKind::of(claim.form()),
        )
    } else if kind == Capability::TemporalSatisfaction {
        registry.restrict_to_timing(candidates, timed(package))
    } else {
        candidates
    };
    let temporal = selection.temporal.clone().unwrap_or_default();
    let own_subject = temporal.subject.clone().unwrap_or_else(|| subject.clone());
    if let Some(claim) = state_graph {
        if timed(package) {
            // EN-6 lists no state-graph form and EN-1 reads no clocks, so a
            // timed subject has no state-graph route yet (QSL-2305).
            let mut detail = Map::new();
            detail.insert("kind".to_owned(), json!(kind.to_wire()));
            detail.insert(
                "reason".to_owned(),
                json!("a state-graph claim over a timed subject has no route (QSL-2305)"),
            );
            return ResolvedItem::Settled {
                claim_name: Some(name.to_owned()),
                record: Box::new(TerminalRecord::disposition_unsupported(
                    index,
                    fsl_route::NegotiationOutcome::UnsupportedRequestedCapability.code(),
                    detail,
                )),
            };
        }
        if let Some(refused) = sampling_refusal(index, name, claim, &temporal) {
            return refused;
        }
    }
    let form = (kind == Capability::TemporalSatisfaction).then(|| {
        let mut form = temporal_item(package, &claim, &temporal);
        form.pending_reductions.clone_from(&selection.reductions);
        ItemForm::Temporal(form)
    });
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
            trace: selection.trace.as_ref().map(DocumentSelection::reference),
        },
        form,
    };
    ResolvedItem::Claim(Box::new(ResolvedClaim {
        item,
        claim_name: name.to_owned(),
        candidates,
        unrouted: if state_graph.is_some() {
            selection.unrouted_beyond_sampling()
        } else {
            selection.unrouted()
        } || (selection.reductions.is_some() && kind != Capability::TemporalSatisfaction)
            || reads_no_clocks(package, &temporal)
            || own_subject != *subject,
        subject: own_subject,
    }))
}

/// FR-623: admit the item's pending `reductions` over what the unit declares,
/// so the item runs under FSpec FR-382's wire form and its echo states it. A
/// subject the binding refuses is left for the run to refuse.
fn admit_reductions(
    package: &CheckedPackage,
    binding: &SubjectBinding<'_>,
    limits: &AnalyzeLimits,
    item: &mut ClaimItem,
) -> Result<(), super::reduction_item::ReductionIssue> {
    let Some(ItemForm::Temporal(form)) = &mut item.form else {
        return Ok(());
    };
    let Some(selection) = form.pending_reductions.take() else {
        return Ok(());
    };
    let simulation = super::request::simulation(package, binding, limits);
    let Ok(system) = fsl_eval::simulation::model::ModelSystem::new(simulation) else {
        form.pending_reductions = Some(selection);
        return Ok(());
    };
    form.reductions = super::reduction_item::admit(&selection, &system, &binding.universes)?;
    Ok(())
}

/// FR-626: a derived item (`deadlock-freedom`, `time-lock-freedom` or
/// `race-freedom`) named by the model alias of its subject, which is the
/// declaration the derived item anchors at (FSpec FR-368 "Claim and
/// anchor"). `None` when the item is no derived item or `name` is no model
/// alias, so the caller settles `unknown-claim`; an alias whose subject
/// derives no such item (a time-lock item over an untimed subject, a
/// deadlock item under `terminal any`) settles `inapplicable-kind`.
fn resolve_derived(
    package: &CheckedPackage,
    subject: &SubjectSelection,
    index: RequestIndex,
    selection: &ItemSelection,
    registry: &Registry,
    name: &str,
) -> Option<ResolvedItem> {
    let temporal = selection.temporal.as_ref()?;
    let kind = temporal.item_kind?.kind();
    if matches!(kind, ItemKind::Formula | ItemKind::Refinement) {
        return None;
    }
    let graph = package.graph();
    let identity = graph.model_package_identity(name)?;
    let root = graph
        .model_roots()
        .find(|(selected, _)| selected.identity == identity)
        .map(|(_, root)| root)?;
    if temporal.max_depth.is_some() {
        // FSpec FR-368-AC-20: a derived item carries no `max_depth`.
        let mut detail = Map::new();
        detail.insert("member".to_owned(), json!("temporal.max_depth"));
        return Some(ResolvedItem::Settled {
            claim_name: Some(name.to_owned()),
            record: Box::new(TerminalRecord::disposition_invalid_request(
                index,
                CatalogCode::new("invalid_runtime_input", "invalid-value"),
                detail,
            )),
        });
    }
    let derived = derived_items(package, subject, registry, &[], 0)
        .into_iter()
        .find_map(|resolved| match resolved {
            ResolvedItem::Claim(claim)
                if claim.item.claim.node() == &root
                    && matches!(
                        &claim.item.form,
                        Some(ItemForm::Temporal(form)) if form.item_kind == kind
                    ) =>
            {
                Some(claim)
            }
            ResolvedItem::Claim(_) | ResolvedItem::Settled { .. } => None,
        });
    let Some(mut claim) = derived else {
        return Some(invalid_kind(index, Some(name), "inapplicable-kind"));
    };
    let own_subject = temporal.subject.clone().unwrap_or_else(|| subject.clone());
    claim.item.request_index = index;
    let named = selection.backend.as_deref().map(BackendId::new);
    claim.candidates = registry.restrict_to_timing(
        registry.candidates(claim.item.kind, named.as_ref()),
        timed(package),
    );
    claim.item.candidates = candidate_list(&claim.candidates);
    claim.item.backend = named;
    claim.item.options = ItemOptions {
        required: selection.required.unwrap_or(true),
        max_cases: None,
        trace: None,
    };
    let mut form = temporal_item(package, &claim.item.claim, temporal);
    form.pending_reductions.clone_from(&selection.reductions);
    claim.item.form = Some(ItemForm::Temporal(form));
    claim.claim_name = name.to_owned();
    claim.unrouted =
        selection.unrouted() || reads_no_clocks(package, temporal) || own_subject != *subject;
    claim.subject = own_subject;
    Some(ResolvedItem::Claim(claim))
}

/// Whether the item sets `max_clocks`, which only a timed subject reads
/// (FSpec FR-415): on an untimed subject no route reads it.
fn reads_no_clocks(package: &CheckedPackage, temporal: &TemporalSelection) -> bool {
    !timed(package)
        && temporal
            .limits
            .is_some_and(|limits| limits.max_clocks.is_some())
}

/// Whether the model subject the package's selections give is timed
/// (FR-243): a selected model declares a time source.
pub(super) fn timed(package: &CheckedPackage) -> bool {
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
        // The reader admits only counts that fit `usize` (`read_max_depth`).
        max_depth: selection
            .max_depth
            .map(|depth| usize::try_from(depth).unwrap_or(usize::MAX)),
        witness_samples: selection.witness_samples,
        seed: selection.seed_count(),
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
        // The reader admits only counts that fit `usize` (`count`).
        let set_usize = |value: Option<NonZeroU64>, slot: &mut usize| {
            if let Some(value) = value {
                *slot = usize::try_from(value.get()).unwrap_or(usize::MAX);
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
        item.max_clocks = limits.max_clocks;
        set(limits.max_witness_set, &mut item.limits.max_witness_set);
        set(
            limits.max_relation_tuples,
            &mut item.limits.max_relation_tuples,
        );
    }
    item
}

/// FR-620's default items: one per checked claim and kind, the claims in
/// source order across every declaration family, followed by the derived
/// items of the model subject.
fn default_items(
    package: &CheckedPackage,
    subject: &SubjectSelection,
    registry: &Registry,
    units: &[RawSourceRef],
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
    let position = |node: &filament_exact::NodeKey| -> (usize, u64) {
        let region = claim_occurrence(package, &records, node)
            .and_then(|occurrence| {
                graph.occurrence_region(occurrence.node().clone(), occurrence.origin())
            })
            .map(|region| (region.source().clone(), region.start()))
            .or_else(|| {
                // A temporal clause's own declaration span lies in the
                // program unit when its occurrence records no region.
                let clause = graph
                    .temporal_clauses()
                    .iter()
                    .find(|clause| clause.identity() == *node)?;
                let start = u64::try_from(clause.clause().span().start).ok()?;
                Some((units.first()?.clone(), start))
            });
        region.map_or((usize::MAX, u64::MAX), |(source, start)| {
            let unit = units
                .iter()
                .position(|candidate| *candidate == source)
                .unwrap_or(usize::MAX);
            (unit, start)
        })
    };
    // FR-620: source order across every declaration family, the units in
    // request order and each unit's declarations by the start byte of their
    // span; the sort is stable, so a node with no recorded region keeps its
    // place after those with one.
    nodes.sort_by_cached_key(|node| position(node));
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
    // FSpec FR-368: a model subject has its derived items whether or not the
    // unit authors a clause, so the subject is joined to the derivation
    // before any authored item is written.
    request.state_graph(
        &fsl_route::deadlock::ModelStateGraphItem {
            subject: &(),
            package: graph,
        },
        &selections,
    );
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
                        trace: None,
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
    /// When the claim's node key or a fairness constraint has no wire
    /// spelling.
    pub fn request_item_wire(
        &self,
        subject: &SubjectSelection,
    ) -> Result<Value, serde_json::Error> {
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
        if let Some(trace) = &self.options.trace {
            item.insert(
                "trace".to_owned(),
                json!({"authority": trace.authority, "identity": trace.identity}),
            );
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
            member.insert("item_kind".to_owned(), json!(temporal.item_kind.to_wire()));
            member.insert(
                "fairness".to_owned(),
                Value::Array(
                    temporal
                        .fairness
                        .iter()
                        .map(fairness_wire)
                        .collect::<Result<_, _>>()?,
                ),
            );
            if !temporal.reductions.is_empty() {
                item.insert(
                    "reductions".to_owned(),
                    super::reduction_item::wire(&temporal.reductions),
                );
            }
            if let Some(stutter) = temporal.stutter_closure {
                member.insert("stutter_closure".to_owned(), json!(stutter));
            }
            if let Some(depth) = temporal.max_depth {
                member.insert("max_depth".to_owned(), json!(depth));
            }
            if let Some(samples) = temporal.witness_samples {
                member.insert("witness_samples".to_owned(), json!(samples));
            }
            if let Some(seed) = temporal.seed {
                member.insert("seed".to_owned(), json!(seed));
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
///
/// # Errors
///
/// When the constraint has no canonical spelling, which the caller reports
/// as a fault rather than writing a placeholder entry.
fn fairness_wire(
    constraint: &filament_semantic_value::fairness::FairnessConstraint,
) -> Result<Value, serde_json::Error> {
    let bytes = quire_canonical::to_vec(constraint, quire_canonical::Limits::new(u64::MAX))
        .map_err(|error| <serde_json::Error as serde::ser::Error>::custom(error.to_string()))?;
    serde_json::from_slice(&bytes)
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
    let mut echoes: BTreeMap<RequestIndex, Echo> = BTreeMap::new();
    for resolved in items {
        match resolved {
            ResolvedItem::Settled { claim_name, record } => {
                settled.insert(record.request_index(), (claim_name, *record));
            }
            ResolvedItem::Claim(claim) => {
                let ResolvedClaim {
                    mut item,
                    claim_name,
                    candidates,
                    subject,
                    unrouted,
                } = *claim;
                let admitted = admit_reductions(package, &binding, &limits, &mut item);
                let echo = item
                    .request_item_wire(&subject)
                    .map_err(|_| AnalyzeDocumentError::Fault)?;
                let identity = owning_model(package, &item.claim)
                    .map(|model| fsl_replay::certificate_identity_wire(&item.claim, model))
                    .transpose()
                    .map_err(|_| AnalyzeDocumentError::Fault)?;
                echoes.insert(
                    item.request_index,
                    Echo {
                        claim_name,
                        request: echo,
                        identity,
                        required: item.options.required,
                        scoped: has_activation_scope(item.kind),
                    },
                );
                if let Err(refused) = admitted {
                    let record = match refused {
                        super::reduction_item::ReductionIssue::UnknownConstraint(name) => {
                            let mut detail = Map::new();
                            detail.insert("name".to_owned(), json!(name));
                            TerminalRecord::disposition_invalid_request(
                                item.request_index,
                                CatalogCode::new("invalid_runtime_input", "invalid-value"),
                                detail,
                            )
                        }
                        super::reduction_item::ReductionIssue::Malformed(_) => {
                            TerminalRecord::disposition_declined(
                                item.request_index,
                                fsl_replay::ProofRefusalCause::InvalidInput,
                                fsl_replay::DeclineCode::Fsl(
                                    fsl_foundation::Code::InvalidRuntimeInput,
                                ),
                                CatalogCode::new("invalid_runtime_input", "invalid-value"),
                            )
                        }
                    };
                    settled.insert(item.request_index, (None, record));
                    continue;
                }
                let (disposition, diagnostic) = negotiate.negotiate(&item, &candidates);
                // A member no route reads settles `unsupported`, except where
                // negotiation refused the item as invalid first: a kind its
                // claim's family does not admit (FR-507, FR-627-AC-4).
                let temporal_kind = match &item.form {
                    Some(ItemForm::Temporal(form)) => form.item_kind,
                    _ => ItemKind::Formula,
                };
                let inapplicable =
                    crate::claim_family::claim_family(package, temporal_kind, &item.claim)
                        .is_none_or(|family| !item.kind.applies_to_declaration(family));
                if unrouted
                    && !inapplicable
                    && !matches!(disposition, fsl_route::routing::Disposition::InvalidRequest)
                {
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
    let mut document = serde_json::to_value(&base).map_err(|_| AnalyzeDocumentError::Fault)?;
    let written: Vec<Value> = document["items"].as_array().cloned().unwrap_or_default();
    let mut items = Vec::with_capacity(records.len());
    let mut gaps = Vec::new();
    for (record, written) in records.iter().zip(written) {
        let index = record.request_index();
        let detail = outcome
            .details
            .iter()
            .find(|detail| detail.request_index == index);
        let echo = echoes.get(&index);
        let claim_name = match echo {
            Some(echo) => Some(echo.claim_name.clone()),
            None => settled.get(&index).and_then(|(name, _)| name.clone()),
        };
        let required = echo.is_none_or(|echo| echo.required);
        let disposed = record.disposition_detail().is_some();
        if required
            && !matches!(
                record.value(),
                TerminalValue::Proved { .. } | TerminalValue::Measured(_)
            )
        {
            gaps.push(json!({"request_index": index.get(), "reason": "not-proved",
                "result": written["result"].as_str().unwrap_or("invalid-request")}));
        }
        let target = outcome
            .counterexamples
            .iter()
            .find(|found| found.request_index == index)
            .and_then(|found| claim_name.as_deref().map(|name| replay_target(found, name)));
        items.push(item_document(
            written,
            ItemMembers {
                claim_name,
                request: echo.map(|echo| echo.request.clone()),
                identity: echo.and_then(|echo| echo.identity.clone()),
                target,
                detail: detail.copied(),
                scoped: echo.is_some_and(|echo| echo.scoped),
                axes: OutcomeAxes::of_record(record.value(), disposed),
            },
        ));
    }
    document["items"] = Value::Array(items);
    document["aggregate"] = json!({
        "status": if gaps.is_empty() { "complete" } else { "incomplete" },
        "gaps": gaps,
    });
    // FR-635 "Outputs": `{backend, tool}` of every registered backend,
    // sorted by backend.
    let mut registry: Vec<(&str, &super::BackendTool)> = engines
        .tools()
        .map(|(descriptor, tool)| (descriptor.id().as_str(), tool))
        .collect();
    registry.sort_unstable();
    document["registry"] = Value::Array(
        registry
            .into_iter()
            .map(|(backend, tool)| {
                json!({"backend": backend,
                    "tool": {"name": tool.name(), "version": tool.version()}})
            })
            .collect(),
    );
    let category = Category::most_severe(
        records
            .iter()
            .map(|record| record.value().category().exit_code()),
    )
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

/// What the run knows of an item it resolved, beside its terminal record.
struct Echo {
    /// The declared name the item resolved to.
    claim_name: String,
    /// The admitted FSpec FR-331 request item.
    request: Value,
    /// FSpec FR-412's certificate identity of the item's obligation;
    /// `None` when no one domain package owns the claim.
    identity: Option<Value>,
    /// Whether the aggregate requires the item.
    required: bool,
    /// Whether the item has an activation scope (FR-622).
    scoped: bool,
}

/// The FR-621 members [`item_document`] adds to an FR-286 item.
struct ItemMembers {
    claim_name: Option<String>,
    request: Option<Value>,
    identity: Option<Value>,
    target: Option<Value>,
    detail: Option<super::ItemDetail>,
    scoped: bool,
    axes: OutcomeAxes,
}

/// The identity of the domain package that owns the obligation `claim`
/// names, FR-621's certificate `model` (FSpec FR-201, FR-412): the package
/// of a temporal clause's `over` object type, else the package's one model
/// selection. `None` when the claim names no single owning package: a
/// package with no model selection, or several and a claim with no `over`
/// object type.
fn owning_model<'p>(package: &'p CheckedPackage, claim: &OccurrenceKey) -> Option<&'p str> {
    let graph = package.graph();
    let over = graph
        .temporal_clauses()
        .iter()
        .find(|clause| clause.identity() == *claim.node())
        .and_then(|clause| match &clause.clause().over().1 {
            filament_exact::ValueType::Reference(object_type) => {
                Some(object_type.original().package())
            }
            _ => None,
        });
    over.or_else(|| match graph.model_selections() {
        [selection] => Some(selection.identity.as_str()),
        _ => None,
    })
}

/// FSpec FR-323's replay target member object for the reproduced
/// counterexample `found` of the claim declared `name`: a temporal clause
/// (over a model or a protocol subject, and a probabilistic one) and an
/// operation contract's violated state clause name their clause by
/// `state_clause`, and a value-validity assignment names the function its
/// parameters belong to by `function`. The in-process engines produce no
/// protocol-refinement counterexample, so FR-071's `protocol_refinement`
/// target has no variant to come from here.
fn replay_target(found: &super::ItemCounterexample, name: &str) -> Value {
    let qualified = name.split("::").collect::<Vec<_>>().join(".");
    match &found.counterexample {
        super::Counterexample::Assignments(_) => json!({"function": qualified}),
        super::Counterexample::Temporal { .. }
        | super::Counterexample::Protocol { .. }
        | super::Counterexample::Probabilistic(_)
        | super::Counterexample::Contract(_)
        | super::Counterexample::StateGraph(_) => json!({"state_clause": qualified}),
    }
}

/// One FR-621 item from the FR-286 item `written`.
fn item_document(written: Value, members: ItemMembers) -> Value {
    let ItemMembers {
        claim_name,
        request,
        identity,
        target,
        detail,
        scoped,
        axes,
    } = members;
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
    if let (Some(checker), Some(identity), Some("proved")) = (
        detail.and_then(|detail| detail.checker),
        identity,
        record.get("result").and_then(Value::as_str),
    ) {
        item.insert(
            "certificate".to_owned(),
            json!({"identity": identity, "checker": checker.as_str(), "accepted": true}),
        );
    }
    if let Some(mut counterexample) = counterexample {
        spell_states(&mut counterexample);
        if let Some(target) = target {
            item.insert(
                "replay".to_owned(),
                json!({"target": target, "reproduced": true}),
            );
        }
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
    // FR-941 "Bare claim": a proof over a supplied trace names the trace and
    // the steps it replayed; it is about that trace only.
    if let Some(steps) = detail.and_then(|detail| detail.replayed_steps) {
        if let Some(trace) = item.get("request").and_then(|request| request.get("trace")) {
            record.insert("trace".to_owned(), trace.clone());
        }
        record.insert("steps".to_owned(), json!(steps));
    }
    item.insert("record".to_owned(), Value::Object(record));
    Value::Object(item)
}
