// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-1044: the items a `compose` declaration selects through `analyze`.
//!
//! An item naming a checked `compose` declaration with `kind` `composition`
//! selects the declaration's composition, under every rule. Its premise
//! items, the component guarantees, are derived from the declaration, never
//! supplied, and settle in the same request after the caller's items. A
//! component guarantee is a clause of the component's own checked package, so
//! its item is resolved over that package and run over it by the shared
//! analyze path ([`CompositionClaim::package`]).
//!
//! No registered backend advertises `composition` by default, so a routed
//! composition settles `unsupported` at negotiation. A declaration whose
//! dependency graph holds a cycle settles `unsupported` before any premise is
//! read or any request is written (FSpec FR-489 "Cycles").

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use filament_semantic_value::catalog::CatalogCode;
use fsl_foundation::diagnostic::Code;
use fsl_foundation::source::provenance::OccurrenceKey;
use fsl_foundation::RequestIndex;
use fsl_package::CheckedPackage;
use fsl_replay::planner::{composition_premises, CompositionPremise, ItemKey, SlotKey, SlotRole};
use fsl_replay::{DeclineCode, ProofRefusalCause, TerminalRecord};
use fsl_route::request::{ExtentClassification, ItemKind};
use fsl_route::Registry;
use fsl_semantics::check::{
    Capability, CheckedComposition, ClauseRef, ComponentPackages, CompositionSlotKey,
};
use fsl_semantics::library::LibraryName;
use serde_json::{json, Map, Value};

use super::{
    candidate_list, invalid_kind, resolve_one, unknown_option, ItemSelection, ResolvedClaim,
    ResolvedItem, SubjectSelection,
};
use crate::analyze::{
    AnalyzeLimits, ClaimItem, CompositionItem, Diagnostic, GraphUnavailable, ItemForm, ItemOptions,
    Payload, SubjectBinding,
};

/// A claim item the composition selection derives or settles (FR-1044),
/// beside the ordinary [`ResolvedItem::Claim`].
#[derive(Clone)]
pub struct CompositionClaim {
    /// The claim item and its candidates.
    pub claim: ResolvedClaim,
    /// The checked package of the component the claim is a clause of, which
    /// the item is analyzed over; `None` for a claim of the request's own
    /// package.
    pub package: Option<Arc<CheckedPackage>>,
    /// The record the item settles with before negotiation (FR-621 `S4`);
    /// `None` when negotiation settles it.
    pub settled: Option<Box<TerminalRecord>>,
    /// For a composition, each of its component-guarantee premises with the
    /// request index of its item, as assigned by [`premise_items`]: the items
    /// whose own settlements decide whether a provider's `proved` for the
    /// composition stands. Empty for every other claim.
    pub required: Vec<Guarantee>,
    /// For a composition, the package of each of its `components` entries by
    /// name and of the component of each guarantee clause: the graphs the ids
    /// of its component slots index (FSpec FR-322 "Scope"). Empty for every
    /// other claim.
    pub components: ComponentPackages,
}

/// A component-guarantee entry of a composition's `premises` and the item
/// that settles it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Guarantee {
    /// The entry's index in the composition's `premises`.
    pub premise: u64,
    /// The request index of the item whose typed slot key (assumptions
    /// included) is the entry's.
    pub item: RequestIndex,
}

/// What resolving one selection of a composition gives.
pub(super) struct Selected {
    /// The composition's item.
    pub item: ResolvedItem,
    /// The premise items the selection derives, in FSpec FR-491 slot order;
    /// empty when the selection settled before its slots were derived.
    pub premises: Vec<Premise>,
}

/// One premise item a selected composition derives.
pub(super) struct Premise {
    /// The slot's index in the composition's `premises`.
    slot: u64,
    /// The slot's key, which identifies the item within the request.
    key: ItemKey,
    kind: PremiseKind,
}

enum PremiseKind {
    /// A `component-guarantee` slot.
    Guarantee {
        clause: ClauseRef,
        library: LibraryName,
        /// Whether the guarantee is checked under peer guarantees, which no
        /// registered engine reads.
        assumed: bool,
    },
    /// A slot whose component or key the checked declaration does not
    /// resolve: an invariant break, settled as a typed `failed` record.
    Unresolved { composition: String },
}

/// What a selection is resolved against.
pub(super) struct Context<'a> {
    pub package: &'a CheckedPackage,
    pub subject: &'a SubjectSelection,
    pub registry: &'a Registry,
    pub binding: &'a SubjectBinding<'a>,
    pub limits: &'a AnalyzeLimits,
}

fn composition_named<'p>(
    package: &'p CheckedPackage,
    name: &str,
) -> Option<&'p CheckedComposition> {
    package
        .graph()
        .compositions()
        .iter()
        .find(|composition| composition.name() == name)
}

/// The composition of `package` declared at `node`.
pub(super) fn composition_at<'p>(
    package: &'p CheckedPackage,
    node: &filament_exact::NodeKey,
) -> Option<&'p CheckedComposition> {
    package
        .graph()
        .compositions()
        .iter()
        .find(|composition| composition.node() == node)
}

fn occurrence(node: &filament_exact::NodeKey) -> OccurrenceKey {
    OccurrenceKey::new(
        node.clone(),
        filament_exact::Origin::new(filament_exact::Role::new("claim"), 0),
    )
}

pub(super) fn failed(index: RequestIndex, name: &str) -> ResolvedItem {
    ResolvedItem::Settled {
        claim_name: Some(name.to_owned()),
        record: Box::new(TerminalRecord::disposition_failed(
            index,
            filament_semantic_value::catalog::InternalFault::new(
                "composition-selection",
                "slots-are-derived",
            )
            .catalog_code(),
        )),
    }
}

fn unsupported(index: RequestIndex, kind: Capability) -> TerminalRecord {
    let mut detail = Map::new();
    detail.insert("kind".to_owned(), json!(kind.to_wire()));
    TerminalRecord::disposition_unsupported(
        index,
        fsl_route::NegotiationOutcome::UnsupportedRequestedCapability.code(),
        detail,
    )
}

/// The record of a composition whose dependency graph holds a cycle (FSpec
/// FR-489 "Cycles"): `unsupported`, naming the kind `composition`, the form
/// `fixed-point` and every cycle's guarantees in edge order.
fn cyclic(index: RequestIndex, composition: &CheckedComposition) -> TerminalRecord {
    let cycles: Vec<Vec<String>> = composition
        .cycles()
        .iter()
        .map(|cycle| cycle.guarantees.iter().map(ToString::to_string).collect())
        .collect();
    let mut detail = Map::new();
    detail.insert("kind".to_owned(), json!(Capability::Composition.to_wire()));
    detail.insert("form".to_owned(), json!("fixed-point"));
    detail.insert("cycles".to_owned(), json!(cycles));
    TerminalRecord::disposition_unsupported(
        index,
        fsl_route::NegotiationOutcome::UnsupportedRequestedCapability.code(),
        detail,
    )
}

/// Resolve `selection` when it names a checked `compose` declaration of
/// `ctx.package`; `None` for any other name.
pub(super) fn resolve(
    ctx: &Context<'_>,
    index: RequestIndex,
    selection: &ItemSelection,
) -> Option<Selected> {
    let name = selection.claim.as_deref()?;
    let composition = composition_named(ctx.package, name)?;
    let settled = |item: ResolvedItem| {
        Some(Selected {
            item,
            premises: Vec::new(),
        })
    };
    let Some(label) = selection.kind.as_deref() else {
        return settled(invalid_kind(index, Some(name), "absent-kind"));
    };
    let kind = match Capability::from_wire(label) {
        Ok(kind) => kind,
        Err(_) => return settled(invalid_kind(index, Some(name), "unknown-kind")),
    };
    if kind != Capability::Composition {
        // FR-507: only a composition applies to its own claim.
        let diagnostic = Diagnostic {
            code: CatalogCode::new("invalid_capability", "inapplicable-kind"),
            payload: Payload::Inapplicable {
                request_index: index,
                capability: kind,
                family: crate::claim_family::claim_family(
                    ctx.package,
                    ItemKind::Formula,
                    &occurrence(composition.node()),
                ),
            },
        };
        return settled(ResolvedItem::Settled {
            claim_name: Some(name.to_owned()),
            record: Box::new(TerminalRecord::disposition_invalid_request(
                index,
                diagnostic.code,
                diagnostic.payload.detail(),
            )),
        });
    }
    if !composition.cycles().is_empty() {
        return settled(ResolvedItem::Settled {
            claim_name: Some(name.to_owned()),
            record: Box::new(cyclic(index, composition)),
        });
    }
    let slots = match composition_premises(ctx.package, composition.node()) {
        Ok(slots) => slots,
        Err(_) => return settled(failed(index, name)),
    };
    let Ok(listed) = list(&slots) else {
        return settled(failed(index, name));
    };
    if let Some(caller) = selection.premises.as_ref() {
        let restated = match restate(ctx, composition, &slots, index, caller) {
            Ok(restated) => restated,
            Err(refused) => return settled(*refused),
        };
        if restated != Value::Array(listed.clone()) {
            let mut detail = Map::new();
            detail.insert("member".to_owned(), json!("premises"));
            detail.insert(
                "pointer".to_owned(),
                json!(format!("/items/{}/premises", index.get())),
            );
            return settled(ResolvedItem::Settled {
                claim_name: Some(name.to_owned()),
                record: Box::new(TerminalRecord::disposition_invalid_request(
                    index,
                    CatalogCode::new("invalid_runtime_input", "invalid-value"),
                    detail,
                )),
            });
        }
    }
    let premises = premises_of(composition, &slots);
    let item = routed(ctx, index, selection, name, composition, listed);
    Some(Selected { item, premises })
}

/// The derived slot list a request's `premises` member spells, as
/// `{role, key}`.
fn list(slots: &[CompositionPremise]) -> Result<Vec<Value>, fsl_replay::planner::KeyWireError> {
    slots
        .iter()
        .map(|premise| {
            Ok(json!({
                "role": premise.derived.role.as_str(),
                "key": premise.derived.key.to_wire()?,
            }))
        })
        .collect()
}

/// The routed composition item (FSpec FR-331 `composition`).
fn routed(
    ctx: &Context<'_>,
    index: RequestIndex,
    selection: &ItemSelection,
    name: &str,
    composition: &CheckedComposition,
    listed: Vec<Value>,
) -> ResolvedItem {
    let named = selection
        .backend
        .as_deref()
        .map(fsl_foundation::BackendId::new);
    let backend_options = selection.options.clone().unwrap_or_default();
    if let Some(refused) = unknown_option(
        index,
        Some(name),
        ctx.registry,
        named.as_ref(),
        &backend_options,
    ) {
        return refused;
    }
    let unrouted = ItemSelection {
        premises: None,
        ..selection.clone()
    }
    .unrouted_routing_options(false, false);
    claim_item(
        ctx,
        index,
        (name, composition.node()),
        named,
        (backend_options, selection.required.unwrap_or(true)),
        CompositionItem {
            premises: listed,
            assumptions: composition.item().assumptions,
            components: composition.component_packages(),
        },
        unrouted,
    )
}

/// A composition item anchored at `node`.
fn claim_item(
    ctx: &Context<'_>,
    index: RequestIndex,
    (name, node): (&str, &filament_exact::NodeKey),
    named: Option<fsl_foundation::BackendId>,
    (backend_options, required): (BTreeMap<String, String>, bool),
    form: CompositionItem,
    unrouted: bool,
) -> ResolvedItem {
    let candidates = ctx
        .registry
        .candidates(Capability::Composition, named.as_ref());
    let components = form.components.clone();
    let item = ClaimItem {
        request_index: index,
        claim: occurrence(node),
        kind: Capability::Composition,
        extent: ExtentClassification::Unbounded {
            finite_bound_available: false,
        },
        backend: named,
        candidates: candidate_list(&candidates),
        options: ItemOptions {
            required,
            max_cases: None,
            max_task_product_work: None,
            max_check_work: None,
            trace: None,
            traces: None,
            recording: None,
            population: None,
            backend_options,
            protocol_analysis: None,
        },
        form: Some(ItemForm::Composition(form)),
    };
    ResolvedItem::Composition(Box::new(CompositionClaim {
        claim: ResolvedClaim {
            item,
            claim_name: name.to_owned(),
            candidates,
            subject: ctx.subject.clone(),
            unrouted,
        },
        package: None,
        settled: None,
        required: Vec::new(),
        components,
    }))
}

/// The premise items of `composition` in FSpec FR-491 order: each
/// `component-guarantee` slot.
fn premises_of(composition: &CheckedComposition, slots: &[CompositionPremise]) -> Vec<Premise> {
    slots
        .iter()
        .enumerate()
        .filter_map(|(slot, premise)| premise_of(composition, slot, premise))
        .collect()
}

/// The premise item `premise` derives, `None` for a slot no item backs.
fn premise_of(
    composition: &CheckedComposition,
    slot: usize,
    premise: &CompositionPremise,
) -> Option<Premise> {
    let SlotKey::Item(key) = &premise.derived.key else {
        return None;
    };
    let unresolved = || PremiseKind::Unresolved {
        composition: composition.name().to_owned(),
    };
    let Some(CompositionSlotKey::ComponentGuarantee(guarantee)) = &premise.slot.key else {
        return None;
    };
    let kind = premise
        .slot
        .guarantee
        .as_ref()
        .and_then(|clause| {
            let component = composition
                .components()
                .iter()
                .find(|component| component.name == clause.component)?;
            Some(PremiseKind::Guarantee {
                clause: clause.clone(),
                library: LibraryName::new(component.subject.package.as_str()).ok()?,
                assumed: !guarantee.assumptions.is_empty(),
            })
        })
        .unwrap_or_else(unresolved);
    Some(Premise {
        slot: u64::try_from(slot).ok()?,
        key: key.clone(),
        kind,
    })
}

/// The premise items of `groups`, one group per selected composition at the
/// request index it is paired with, each written once however many
/// selections derive it and numbered after `resolved`. A composition's item
/// in `resolved` records the request index of each of its component
/// guarantees, which is the item its own slot key (assumptions included)
/// identifies, so a guarantee checked under other peers is never read as
/// this one.
pub(super) fn premise_items(
    ctx: &Context<'_>,
    groups: Vec<(RequestIndex, Vec<Premise>)>,
    resolved: &mut [ResolvedItem],
    first: usize,
) -> Vec<ResolvedItem> {
    let mut written: HashMap<ItemKey, RequestIndex> = HashMap::new();
    let mut items = Vec::new();
    for (composition, premises) in groups {
        let mut required = Vec::new();
        for premise in premises {
            let slot = premise.slot;
            let index = match written.get(&premise.key) {
                Some(index) => *index,
                None => {
                    let index = RequestIndex::new(first + items.len());
                    written.insert(premise.key.clone(), index);
                    items.push(premise_item(ctx, index, premise));
                    index
                }
            };
            required.push(Guarantee {
                premise: slot,
                item: index,
            });
        }
        if let Some(ResolvedItem::Composition(claim)) = resolved.get_mut(composition.get()) {
            claim.required = required;
        }
    }
    items
}

fn premise_item(ctx: &Context<'_>, index: RequestIndex, premise: Premise) -> ResolvedItem {
    match premise.kind {
        PremiseKind::Unresolved { composition } => failed(index, &composition),
        PremiseKind::Guarantee {
            clause,
            library,
            assumed,
        } => {
            let Some(component) = ctx.package.dependencies().get(&library).cloned() else {
                return failed(index, &clause.name);
            };
            let selection = ItemSelection {
                claim: Some(clause.name.clone()),
                kind: Some(Capability::TemporalSatisfaction.to_wire().to_owned()),
                ..ItemSelection::default()
            };
            let resolved = resolve_one(
                &component,
                ctx.subject,
                index,
                &selection,
                ctx.registry,
                ctx.binding,
            );
            let ResolvedItem::Claim(mut claim) = resolved else {
                return resolved;
            };
            let settled = assumed.then(|| {
                // FSpec FR-491: the slot is checked under peer guarantees, an
                // assumption set no registered engine reads. Running the
                // clause alone would settle another obligation under this
                // slot's key.
                claim.item.candidates.clear();
                Box::new(unsupported(index, Capability::TemporalSatisfaction))
            });
            ResolvedItem::Composition(Box::new(CompositionClaim {
                claim: *claim,
                package: Some(component),
                settled,
                required: Vec::new(),
                components: ComponentPackages::default(),
            }))
        }
    }
}

/// Why a component package has no graph to resolve a key against.
enum Unavailable {
    /// The strict reader refused the package the run emitted for it.
    Refused(fsl_package::PackageAdmissionRefusal),
    /// The package emitted no complete admitted bytes, or they hold no
    /// readable graph.
    Emission(GraphUnavailable),
}

/// The settled item that reports the typed cause of an unavailable component
/// graph: the emit limit is the limit cause, every other failure its own
/// (FSL FR-621-AC-17).
fn unavailable(index: RequestIndex, name: &str, failure: &Unavailable) -> Box<ResolvedItem> {
    let record = match failure {
        Unavailable::Refused(refusal) => TerminalRecord::disposition_declined(
            index,
            if refusal.code == Code::ResourceExhausted {
                ProofRefusalCause::IncompleteInput
            } else {
                ProofRefusalCause::InvalidInput
            },
            DeclineCode::Fsl(refusal.code),
            CatalogCode::new(refusal.code.as_str(), refusal.catalog_cause()),
        ),
        Unavailable::Emission(GraphUnavailable::NotEmitted) => {
            TerminalRecord::disposition_declined(
                index,
                ProofRefusalCause::IncompleteInput,
                DeclineCode::Fsl(Code::ResourceExhausted),
                CatalogCode::new("resource_exhausted", "insufficient-next-charge"),
            )
        }
        Unavailable::Emission(GraphUnavailable::Unreadable(_)) => {
            TerminalRecord::disposition_declined(
                index,
                ProofRefusalCause::InvalidInput,
                DeclineCode::Fsl(Code::InvalidPackage),
                CatalogCode::new("invalid_package", "invalid-value"),
            )
        }
    };
    Box::new(ResolvedItem::Settled {
        claim_name: Some(name.to_owned()),
        record: Box::new(record),
    })
}

/// The caller's `premises` with the node ids of each `component-guarantee`
/// key that holds index segments resolved to their structural form against
/// the graph of the slot's component package (FSpec FR-322 "Scope"), so that
/// the list a result wrote can be restated as written. A peer clause id of the
/// key's `assumptions` resolves against the graph of the package of the
/// component of the peer guarantee the declaration's assumption set names at
/// that position.
///
/// # Errors
///
/// The settled item that refuses the key: `invalid_runtime_input`
/// `invalid-value` at the key for a key of another package or of a guarantee
/// naming no component (every `component-guarantee` key, with or without an
/// index segment), `malformed_wire`, with no cause, at the `node` member of an
/// index segment the component graph does not hold; or the typed cause of a
/// component package that has no graph.
fn restate(
    ctx: &Context<'_>,
    composition: &CheckedComposition,
    slots: &[CompositionPremise],
    index: RequestIndex,
    caller: &Value,
) -> Result<Value, Box<ResolvedItem>> {
    use fsl_semantics::check::wire_id::{
        check_component_key_package, component_key_structural, key_holds_index_segment,
        ComponentKeyRefusal, WireGraph,
    };
    let Value::Array(entries) = caller else {
        return Ok(caller.clone());
    };
    let packages = composition.component_packages();
    let refuse = |code: CatalogCode, pointer: String| {
        let mut detail = Map::new();
        detail.insert("member".to_owned(), json!("premises"));
        detail.insert("pointer".to_owned(), json!(pointer));
        Box::new(ResolvedItem::Settled {
            claim_name: Some(composition.name().to_owned()),
            record: Box::new(TerminalRecord::disposition_invalid_request(
                index, code, detail,
            )),
        })
    };
    let mut restated = entries.clone();
    let mut graphs: Option<BTreeMap<String, Result<WireGraph, Unavailable>>> = None;
    for (at, entry) in restated.iter_mut().enumerate() {
        let key_pointer = format!("/items/{}/premises/{at}/key", index.get());
        let invalid = || {
            refuse(
                CatalogCode::new("invalid_runtime_input", "invalid-value"),
                key_pointer.clone(),
            )
        };
        if entry.get("role").and_then(Value::as_str) != Some(SlotRole::ComponentGuarantee.as_str())
        {
            continue;
        }
        let Some(key) = entry.get_mut("key") else {
            continue;
        };
        let slot = slots.get(at).map(|premise| &premise.slot);
        let package = slot
            .and_then(|slot| slot.guarantee.as_ref())
            .and_then(|clause| packages.of_component(&clause.component));
        check_component_key_package(package, key).map_err(|_| invalid())?;
        let Some(package) = package else {
            return Err(invalid());
        };
        if !key_holds_index_segment(key) {
            continue;
        }
        let graphs = graphs.get_or_insert_with(|| component_graphs(ctx, composition));
        let graph_of = |package: &str| match graphs.get(package) {
            Some(Ok(graph)) => Ok(Some(graph)),
            Some(Err(failure)) => Err(unavailable(index, composition.name(), failure)),
            None => Ok(None),
        };
        let Some(graph) = graph_of(package)? else {
            return Err(invalid());
        };
        let assumed: &[filament_exact::NodeKey] = match slot.and_then(|slot| slot.key.as_ref()) {
            Some(CompositionSlotKey::ComponentGuarantee(guarantee)) => &guarantee.assumptions,
            _ => &[],
        };
        let mut peers = Vec::with_capacity(assumed.len());
        for node in assumed {
            let peer = match packages.of_clause(node) {
                Some(package) => graph_of(package)?,
                None => None,
            };
            peers.extend(peer);
        }
        let carried = key
            .pointer("/temporal/subject/assumptions")
            .and_then(Value::as_array)
            .map_or(0, Vec::len);
        if peers.len() != assumed.len() || carried != assumed.len() {
            // Another assumption set than the declaration derives: the
            // comparison refuses the list at `/premises`.
            continue;
        }
        match component_key_structural(graph, Some(package), key, &peers) {
            Ok(structural) => *key = structural,
            Err(ComponentKeyRefusal::IndexOutOfRange { pointer, .. }) => {
                // FSpec FR-272: `malformed_wire` carries no cause.
                return Err(refuse(
                    CatalogCode::new("malformed_wire", ""),
                    format!("{key_pointer}{pointer}"),
                ));
            }
            Err(_) => return Err(invalid()),
        }
    }
    Ok(Value::Array(restated))
}

/// The graph of each component package of `composition`, by package identity:
/// the graph the ids of the component's slots index (FSpec FR-322 "Scope"),
/// or why it has none.
fn component_graphs(
    ctx: &Context<'_>,
    composition: &CheckedComposition,
) -> BTreeMap<String, Result<fsl_semantics::check::wire_id::WireGraph, Unavailable>> {
    composition
        .components()
        .iter()
        .map(|component| {
            let package = component.subject.package.as_str();
            let graph = match LibraryName::new(package)
                .ok()
                .and_then(|library| ctx.package.dependencies().get(&library).cloned())
            {
                None => Err(Unavailable::Emission(GraphUnavailable::NotEmitted)),
                Some(dependency) => {
                    match crate::provider::read_package(
                        &dependency,
                        ctx.binding.packages,
                        ctx.limits.emit,
                    ) {
                        Ok(read) => crate::analyze::DocumentGraphs::of_bytes(
                            read.emission().package().bytes(),
                        )
                        .map_err(Unavailable::Emission),
                        Err(crate::provider::NotAdmitted::Refused(refusal)) => {
                            Err(Unavailable::Refused(refusal))
                        }
                        Err(crate::provider::NotAdmitted::Other) => {
                            Err(Unavailable::Emission(GraphUnavailable::NotEmitted))
                        }
                    }
                }
            };
            (package.to_owned(), graph)
        })
        .collect()
}
