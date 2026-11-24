// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-1044: the items a `compose` declaration selects through `analyze`.
//!
//! An item naming a `compose` declaration with `kind` `composition` selects
//! the declaration's composition, under every rule. Its premise items, the
//! component guarantees, are derived from the declaration, never supplied,
//! and settle in the same request after the caller's items. A component
//! guarantee is a clause of the component's own package, so its item is
//! resolved and run over that package ([`CompositionClaim::owner`]).
//!
//! One analysis serves both routes (FR-940): the source route reads the
//! declaration's [`CompositionFacts`] from its checked composition, the
//! `package` route from the admitted composition view, and everything after
//! that (the cycle answer, the derived slot list, the routed item, the
//! premise items and the restated `premises`) is read from those facts alone.
//! Each route resolves a premise clause over the component package it holds
//! ([`CompositionFront`]).
//!
//! No registered backend advertises `composition` by default, so a routed
//! composition settles `unsupported` at negotiation. A declaration whose
//! dependency graph holds a cycle settles `unsupported` before any premise is
//! read or any request is written (FSpec FR-489 "Cycles").

use std::collections::{BTreeMap, BTreeSet, HashMap};

use filament_contract_model::NativeJoinBudget;
use filament_exact::NodeKey;
use filament_semantic_value::catalog::CatalogCode;
use fsl_foundation::diagnostic::Code;
use fsl_foundation::source::provenance::OccurrenceKey;
use fsl_foundation::RequestIndex;
use fsl_package::{BarePackage, CheckedPackage, CompositionMemberValue, CompositionView};
use fsl_replay::planner::{
    composition_premises, DerivedSlot, ItemKey, PackageComposition, PackageCompositionRefusal,
    SlotKey, SlotRole,
};
use fsl_replay::{DeclineCode, ProofRefusalCause, TerminalRecord};
use fsl_route::request::ExtentClassification;
use fsl_route::Registry;
use fsl_semantics::check::wire_id::WireGraph;
use fsl_semantics::check::{Capability, CheckedComposition, ComponentPackages, DeclarationFamily};
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
    /// The component package the claim is a clause of, which the item is
    /// analyzed over; `None` for a claim of the request's own package.
    pub owner: Option<ComponentOwner>,
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

/// What negotiation and the writer read of the component package a premise
/// item is analyzed over, as each route reads it from the package it holds.
#[derive(Clone, Debug)]
pub struct ComponentOwner {
    /// The component package's identity: the item's `package` member and the
    /// graph its node ids index (FSpec FR-322).
    pub identity: String,
    /// The model package owning the claim within the component, which the
    /// certificate identity names (FSpec FR-412).
    pub owning_model: Option<String>,
    /// The declaration family of the claim within the component (FR-507).
    pub family: Option<DeclarationFamily>,
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

/// The facts of one `compose` declaration the composition analysis reads
/// (FSpec FR-491): the same on both routes, built from the checked
/// declaration ([`Self::of_checked`]) or from the admitted package's
/// composition view ([`Self::of_admitted`]).
pub(super) struct CompositionFacts {
    /// The declared name `S`.
    name: String,
    /// The declaration's node: the item's anchor.
    node: NodeKey,
    /// Each dependency cycle's guarantees, in edge order, as
    /// `Component::Clause`.
    cycles: Vec<Vec<String>>,
    /// The derived slot list in FSpec FR-491 order; `Err` when the
    /// declaration derives none (a cycle, or a broken invariant).
    slots: Result<Vec<SlotFacts>, ()>,
    /// The item's `assumptions` member: per `component-guarantee` entry, the
    /// `premises` indices of the peer guarantees it is checked under.
    assumptions: Vec<Vec<usize>>,
    /// The package of each `components` entry and of each named clause.
    components: ComponentPackages,
}

/// One derived slot.
pub(super) struct SlotFacts {
    /// The slot's role and key, as the shared planner derives them.
    derived: DerivedSlot,
    /// The guarantee of a `component-guarantee` slot; `None` for every other
    /// role.
    guarantee: Option<GuaranteeClause>,
}

/// A guarantee clause a composition names: `component::name`.
pub(super) struct GuaranteeClause {
    /// The `components` entry's name.
    pub component: String,
    /// The clause's name in the component's package.
    pub name: String,
    /// The clause's node in the component's package.
    pub node: NodeKey,
    /// The identity of the component package that owns the clause: the
    /// package of its `components` entry; `None` when no entry names the
    /// component.
    pub owner: Option<String>,
}

impl CompositionFacts {
    /// The facts of the checked declaration `composition` of `package`.
    pub(super) fn of_checked(package: &CheckedPackage, composition: &CheckedComposition) -> Self {
        let components = composition.component_packages();
        let cycles = composition
            .cycles()
            .iter()
            .map(|cycle| cycle.guarantees.iter().map(ToString::to_string).collect())
            .collect();
        let slots = if composition.cycles().is_empty() {
            composition_premises(package, composition.node())
                .map(|premises| {
                    premises
                        .into_iter()
                        .map(|premise| SlotFacts {
                            guarantee: premise
                                .slot
                                .guarantee
                                .filter(|_| premise.derived.role == SlotRole::ComponentGuarantee)
                                .map(|clause| GuaranteeClause {
                                    owner: components
                                        .of_component(&clause.component)
                                        .map(str::to_owned),
                                    component: clause.component,
                                    name: clause.name,
                                    node: clause.node,
                                }),
                            derived: premise.derived,
                        })
                        .collect()
                })
                .map_err(|_| ())
        } else {
            Err(())
        };
        Self {
            name: composition.name().to_owned(),
            node: composition.node().clone(),
            cycles,
            slots,
            assumptions: composition.item().assumptions,
            components,
        }
    }

    /// The facts of the admitted composition `view` of `root`, read with the
    /// component packages `root` retains under the invocation's cumulative
    /// native `budget` (FR-940).
    ///
    /// # Errors
    ///
    /// The original projection refusal; no missing package fact is replaced.
    pub(super) fn of_admitted(
        root: &BarePackage<'_>,
        view: &CompositionView<'_>,
        budget: &mut NativeJoinBudget,
    ) -> Result<Self, PackageCompositionRefusal> {
        let projected = PackageComposition::from_view(root, view, budget)?;
        let clause_at = |at: usize| {
            projected
                .clauses()
                .get(at)
                .map(|clause| GuaranteeClause {
                    component: clause.component.to_owned(),
                    name: clause.name.name.as_ref().to_owned(),
                    node: clause.node.clone(),
                    // The owner `from_view` authenticated for the clause.
                    owner: Some(clause.owner.package().name().to_owned()),
                })
                .ok_or(PackageCompositionRefusal::Member)
        };
        let mut cycles = Vec::with_capacity(projected.cycles().len());
        for cycle in projected.cycles() {
            let mut guarantees = Vec::with_capacity(cycle.guarantees.len());
            for at in &cycle.guarantees {
                let clause = clause_at(*at)?;
                guarantees.push(format!("{}::{}", clause.component, clause.name));
            }
            cycles.push(guarantees);
        }
        let slots = if projected.cycles().is_empty() {
            let mut slots = Vec::with_capacity(projected.slots().len());
            for slot in projected.slots() {
                let guarantee = if slot.derived.role == SlotRole::ComponentGuarantee {
                    Some(clause_at(
                        slot.guarantee.ok_or(PackageCompositionRefusal::Member)?,
                    )?)
                } else {
                    None
                };
                slots.push(SlotFacts {
                    derived: slot.derived.clone(),
                    guarantee,
                });
            }
            Ok(slots)
        } else {
            Err(())
        };
        let mut by_name = BTreeMap::new();
        for member in view.members_with_budget(budget) {
            if let CompositionMemberValue::Component(component) = member?.value() {
                by_name.insert(
                    component.name.as_ref().to_owned(),
                    component.package.as_ref().to_owned(),
                );
            }
        }
        let by_clause = projected
            .clauses()
            .iter()
            .filter_map(|clause| {
                let package = by_name.get(clause.component)?;
                Some((clause.node.clone(), package.clone()))
            })
            .collect();
        Ok(Self {
            // The row's declared name, as the source route names it.
            name: view.name().name.to_string(),
            node: projected.node().clone(),
            cycles,
            slots,
            assumptions: projected.assumptions().to_vec(),
            components: ComponentPackages::new(by_name, by_clause),
        })
    }

    /// The identity of each component package a premise item of the
    /// composition is analyzed over, in slot order; empty when the
    /// declaration derives no slot.
    pub(super) fn guarantee_packages(&self) -> Vec<&str> {
        self.slots
            .iter()
            .flatten()
            .filter_map(|slot| slot.guarantee.as_ref())
            .filter_map(|clause| clause.owner.as_deref())
            .collect()
    }

    /// The peer guarantee nodes the slot at `at` is checked under, sorted:
    /// the clause nodes of the `component-guarantee` entries its
    /// `assumptions` array names (FSpec FR-491 "Assumption binding"); empty
    /// for a slot of another role.
    fn assumed_at(&self, slots: &[SlotFacts], at: usize) -> Vec<NodeKey> {
        let Some(slot) = slots.get(at) else {
            return Vec::new();
        };
        if slot.derived.role != SlotRole::ComponentGuarantee {
            return Vec::new();
        }
        let entry = slots[..at]
            .iter()
            .filter(|slot| slot.derived.role == SlotRole::ComponentGuarantee)
            .count();
        let peers: BTreeSet<&NodeKey> = self
            .assumptions
            .get(entry)
            .into_iter()
            .flatten()
            .filter_map(|position| slots.get(*position)?.guarantee.as_ref())
            .map(|clause| &clause.node)
            .collect();
        peers.into_iter().cloned().collect()
    }
}

/// What each route supplies to the shared composition analysis: its
/// registry and subject, and what it reads of a component package.
pub(super) trait CompositionFront {
    /// Why a premise item could not be resolved over the route's package.
    type Error;

    /// The request's registry.
    fn registry(&self) -> &Registry;

    /// The request's default subject.
    fn subject(&self) -> &SubjectSelection;

    /// The graph of the component package `identity`, which the ids of the
    /// component's slots index (FSpec FR-322 "Scope"), or why it has none.
    fn component_graph(&self, identity: &str) -> Result<WireGraph, Unavailable>;

    /// The `temporal-satisfaction` item, at `index`, of the guarantee
    /// `clause` resolved over its component package `identity`.
    ///
    /// # Errors
    ///
    /// As the route refuses the item's package form.
    fn guarantee_item(
        &mut self,
        index: RequestIndex,
        clause: &GuaranteeClause,
        identity: &str,
    ) -> Result<GuaranteeResolution, Self::Error>;
}

/// How a route resolved a component guarantee's clause over its component
/// package.
pub(super) enum GuaranteeResolution {
    /// The item settled as that package's resolution settles it.
    Settled(ResolvedItem),
    /// The clause's claim item over the component package.
    Claim {
        /// The claim item and its candidates.
        claim: Box<ResolvedClaim>,
        /// The model package owning the claim within the component.
        owning_model: Option<String>,
    },
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
        clause: GuaranteeClause,
        /// The component package's identity.
        identity: String,
        /// Whether the guarantee is checked under peer guarantees, which no
        /// registered engine reads.
        assumed: bool,
    },
    /// A slot whose component or key the declaration does not resolve: an
    /// invariant break, settled as a typed `failed` record.
    Unresolved { composition: String },
}

/// The source route's front end: the checked package and what the request
/// reads beside it.
pub(super) struct Context<'a> {
    pub package: &'a CheckedPackage,
    pub subject: &'a SubjectSelection,
    pub registry: &'a Registry,
    pub binding: &'a SubjectBinding<'a>,
    pub limits: &'a AnalyzeLimits,
}

impl CompositionFront for Context<'_> {
    type Error = std::convert::Infallible;

    fn registry(&self) -> &Registry {
        self.registry
    }

    fn subject(&self) -> &SubjectSelection {
        self.subject
    }

    fn component_graph(&self, identity: &str) -> Result<WireGraph, Unavailable> {
        let Some(dependency) = LibraryName::new(identity)
            .ok()
            .and_then(|library| self.package.dependencies().get(&library).cloned())
        else {
            return Err(Unavailable::NotDependency);
        };
        let graph = match crate::provider::read_package(
            &dependency,
            self.binding.packages,
            self.limits.emit,
        ) {
            Ok(read) => crate::analyze::DocumentGraphs::of_bytes(read.emission().package().bytes())
                .map_err(Unavailable::Emission),
            Err(crate::provider::NotAdmitted::Refused(refusal)) => {
                Err(Unavailable::Refused(refusal))
            }
            Err(crate::provider::NotAdmitted::HostAllocationFailed(failure)) => {
                Err(Unavailable::HostAllocationFailed(failure))
            }
            Err(crate::provider::NotAdmitted::Other(refusal)) => Err(Unavailable::Emission(
                GraphUnavailable::of_not_admitted(refusal),
            )),
        };
        graph
    }

    fn guarantee_item(
        &mut self,
        index: RequestIndex,
        clause: &GuaranteeClause,
        identity: &str,
    ) -> Result<GuaranteeResolution, Self::Error> {
        let Some(component) = LibraryName::new(identity)
            .ok()
            .and_then(|library| self.package.dependencies().get(&library).cloned())
        else {
            return Ok(GuaranteeResolution::Settled(failed(index, &clause.name)));
        };
        let resolved = resolve_one(
            &component,
            self.subject,
            index,
            &guarantee_selection(clause),
            self.registry,
            self.binding,
        );
        let ResolvedItem::Claim(claim) = resolved else {
            return Ok(GuaranteeResolution::Settled(resolved));
        };
        let owning_model = super::owning_model(&component, &claim.item.claim).map(str::to_owned);
        Ok(GuaranteeResolution::Claim {
            claim,
            owning_model,
        })
    }
}

/// The selection a premise item of the guarantee `clause` is resolved from.
pub(super) fn guarantee_selection(clause: &GuaranteeClause) -> ItemSelection {
    ItemSelection {
        claim: Some(clause.name.clone()),
        kind: Some(Capability::TemporalSatisfaction.to_wire().to_owned()),
        ..ItemSelection::default()
    }
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
fn cyclic(index: RequestIndex, facts: &CompositionFacts) -> TerminalRecord {
    let mut detail = Map::new();
    detail.insert("kind".to_owned(), json!(Capability::Composition.to_wire()));
    detail.insert("form".to_owned(), json!("fixed-point"));
    detail.insert("cycles".to_owned(), json!(facts.cycles));
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
    Some(select(
        ctx,
        &CompositionFacts::of_checked(ctx.package, composition),
        index,
        selection,
    ))
}

/// Resolve `selection` of the composition `facts` describe, on either route.
pub(super) fn select<F: CompositionFront>(
    front: &F,
    facts: &CompositionFacts,
    index: RequestIndex,
    selection: &ItemSelection,
) -> Selected {
    let name = facts.name.as_str();
    let settled = |item: ResolvedItem| Selected {
        item,
        premises: Vec::new(),
    };
    let Some(label) = selection.kind.as_deref() else {
        return settled(invalid_kind(index, Some(name), "absent-kind"));
    };
    let kind = match Capability::from_wire(label) {
        Ok(kind) => kind,
        Err(_) => return settled(invalid_kind(index, Some(name), "unknown-kind")),
    };
    if kind != Capability::Composition {
        // FR-507: only a composition applies to its own claim, whose
        // declaration family is `protocol` (FSpec FR-290's node table).
        let diagnostic = Diagnostic {
            code: CatalogCode::new("invalid_capability", "inapplicable-kind"),
            payload: Payload::Inapplicable {
                request_index: index,
                capability: kind,
                family: Some(DeclarationFamily::Protocol),
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
    if !facts.cycles.is_empty() {
        return settled(ResolvedItem::Settled {
            claim_name: Some(name.to_owned()),
            record: Box::new(cyclic(index, facts)),
        });
    }
    let Ok(slots) = facts.slots.as_ref() else {
        return settled(failed(index, name));
    };
    let Ok(listed) = list(slots) else {
        return settled(failed(index, name));
    };
    if let Some(caller) = selection.premises.as_ref() {
        let restated = match restate(front, facts, slots, index, caller) {
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
    let premises = premises_of(facts, slots);
    let item = routed(front, index, selection, facts, listed);
    Selected { item, premises }
}

/// The derived slot list a request's `premises` member spells, as
/// `{role, key}`.
fn list(slots: &[SlotFacts]) -> Result<Vec<Value>, fsl_replay::planner::KeyWireError> {
    slots
        .iter()
        .map(|slot| {
            Ok(json!({
                "role": slot.derived.role.as_str(),
                "key": slot.derived.key.to_wire()?,
            }))
        })
        .collect()
}

/// The routed composition item (FSpec FR-331 `composition`).
fn routed<F: CompositionFront>(
    front: &F,
    index: RequestIndex,
    selection: &ItemSelection,
    facts: &CompositionFacts,
    listed: Vec<Value>,
) -> ResolvedItem {
    let name = facts.name.as_str();
    let named = selection
        .backend
        .as_deref()
        .map(fsl_foundation::BackendId::new);
    let backend_options = selection.options.clone().unwrap_or_default();
    if let Some(refused) = unknown_option(
        index,
        Some(name),
        front.registry(),
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
        front,
        index,
        (name, &facts.node),
        named,
        (backend_options, selection.required.unwrap_or(true)),
        CompositionItem {
            premises: listed,
            assumptions: facts.assumptions.clone(),
            components: facts.components.clone(),
        },
        unrouted,
    )
}

/// A composition item anchored at `node`.
fn claim_item<F: CompositionFront>(
    front: &F,
    index: RequestIndex,
    (name, node): (&str, &filament_exact::NodeKey),
    named: Option<fsl_foundation::BackendId>,
    (backend_options, required): (BTreeMap<String, String>, bool),
    form: CompositionItem,
    unrouted: bool,
) -> ResolvedItem {
    let candidates = front
        .registry()
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
            assessment: None,
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
            subject: front.subject().clone(),
            unrouted,
        },
        owner: None,
        settled: None,
        required: Vec::new(),
        components,
    }))
}

/// The premise items of the composition `facts` describe, in FSpec FR-491
/// order: each `component-guarantee` slot.
fn premises_of(facts: &CompositionFacts, slots: &[SlotFacts]) -> Vec<Premise> {
    slots
        .iter()
        .enumerate()
        .filter_map(|(at, slot)| premise_of(facts, slots, at, slot))
        .collect()
}

/// The premise item the slot at `at` derives, `None` for a slot no item
/// backs.
fn premise_of(
    facts: &CompositionFacts,
    slots: &[SlotFacts],
    at: usize,
    slot: &SlotFacts,
) -> Option<Premise> {
    let SlotKey::Item(key) = &slot.derived.key else {
        return None;
    };
    if slot.derived.role != SlotRole::ComponentGuarantee {
        return None;
    }
    let kind = slot
        .guarantee
        .as_ref()
        .and_then(|clause| {
            let identity = clause.owner.as_deref()?;
            LibraryName::new(identity).ok()?;
            Some(PremiseKind::Guarantee {
                clause: GuaranteeClause {
                    component: clause.component.clone(),
                    name: clause.name.clone(),
                    node: clause.node.clone(),
                    owner: clause.owner.clone(),
                },
                identity: identity.to_owned(),
                assumed: !facts.assumed_at(slots, at).is_empty(),
            })
        })
        .unwrap_or_else(|| PremiseKind::Unresolved {
            composition: facts.name.clone(),
        });
    Some(Premise {
        slot: u64::try_from(at).ok()?,
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
///
/// # Errors
///
/// As `front` refuses a premise item's package form.
pub(super) fn premise_items<F: CompositionFront>(
    front: &mut F,
    groups: Vec<(RequestIndex, Vec<Premise>)>,
    resolved: &mut [ResolvedItem],
    first: usize,
) -> Result<Vec<ResolvedItem>, F::Error> {
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
                    items.push(premise_item(front, index, premise)?);
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
    Ok(items)
}

fn premise_item<F: CompositionFront>(
    front: &mut F,
    index: RequestIndex,
    premise: Premise,
) -> Result<ResolvedItem, F::Error> {
    match premise.kind {
        PremiseKind::Unresolved { composition } => Ok(failed(index, &composition)),
        PremiseKind::Guarantee {
            clause,
            identity,
            assumed,
        } => {
            let (mut claim, owning_model) = match front.guarantee_item(index, &clause, &identity)? {
                GuaranteeResolution::Settled(settled) => return Ok(settled),
                GuaranteeResolution::Claim {
                    claim,
                    owning_model,
                } => (claim, owning_model),
            };
            let settled = assumed.then(|| {
                // FSpec FR-491: the slot is checked under peer guarantees, an
                // assumption set no registered engine reads. Running the
                // clause alone would settle another obligation under this
                // slot's key.
                claim.item.candidates.clear();
                Box::new(unsupported(index, Capability::TemporalSatisfaction))
            });
            Ok(ResolvedItem::Composition(Box::new(CompositionClaim {
                claim: *claim,
                owner: Some(ComponentOwner {
                    identity,
                    owning_model,
                    // FSpec FR-491: a component guarantee is a temporal
                    // clause (the checker admits no other guarantee), so its
                    // family is `temporal-trace` on either route.
                    family: Some(DeclarationFamily::TemporalTrace),
                }),
                settled,
                required: Vec::new(),
                components: ComponentPackages::default(),
            })))
        }
    }
}

/// Why a component package has no graph to resolve a key against.
pub(super) enum Unavailable {
    /// The strict reader refused the package the run emitted for it.
    Refused(fsl_package::PackageAdmissionRefusal),
    /// The host refused memory the reader had already admitted.
    HostAllocationFailed(filament_contract_model::CheckedPackageHostAllocationFailed),
    /// The package emitted no complete admitted bytes, or they hold no
    /// readable graph.
    Emission(GraphUnavailable),
    /// The component's package is no dependency of the request's package, so
    /// there is none to emit.
    NotDependency,
}

/// A code FSpec FR-272-AC-22 gives no cause: the cause is the empty string,
/// which writes no `cause` member.
fn without_cause(code: &'static str) -> CatalogCode {
    CatalogCode::new(code, "")
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
            // FSpec FR-272-AC-22: only the reader-vocabulary codes carry no
            // cause, and `Code` names none of them, so a reader refusal keeps
            // its catalog cause.
            CatalogCode::new(refusal.code.as_str(), refusal.catalog_cause()),
        ),
        Unavailable::HostAllocationFailed(failure) => TerminalRecord::disposition_failed(
            index,
            CatalogCode::new(Code::RuntimeInvariant.as_str(), "host-allocation-failed"),
        )
        .with_diagnostic_detail(serde_json::Map::from_iter([
                (
                    "stage".to_owned(),
                    serde_json::Value::String(format!("{:?}", failure.stage)),
                ),
                (
                    "requested".to_owned(),
                    serde_json::Value::String(failure.requested.to_string()),
                ),
                (
                    "consumed".to_owned(),
                    serde_json::Value::String(failure.consumed.to_string()),
                ),
            ])),
        Unavailable::Emission(GraphUnavailable::EmitRefused(refusal)) => {
            TerminalRecord::disposition_declined(
                index,
                if refusal.code() == Code::StageLimitExceeded {
                    ProofRefusalCause::IncompleteInput
                } else {
                    ProofRefusalCause::InvalidInput
                },
                DeclineCode::Fsl(refusal.code()),
                refusal.cause().map_or_else(
                    || without_cause(refusal.code().as_str()),
                    |cause| CatalogCode::new(refusal.code().as_str(), cause),
                ),
            )
        }
        Unavailable::Emission(GraphUnavailable::NotEmitted) => {
            TerminalRecord::disposition_declined(
                index,
                ProofRefusalCause::InvalidInput,
                DeclineCode::Fsl(Code::InvalidPackage),
                CatalogCode::new("invalid_package", "invalid-value"),
            )
        }
        Unavailable::NotDependency => TerminalRecord::disposition_declined(
            index,
            ProofRefusalCause::InvalidInput,
            DeclineCode::Fsl(Code::MissingImport),
            CatalogCode::new("missing_import", "missing-selection"),
        ),
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
fn restate<F: CompositionFront>(
    front: &F,
    facts: &CompositionFacts,
    slots: &[SlotFacts],
    index: RequestIndex,
    caller: &Value,
) -> Result<Value, Box<ResolvedItem>> {
    use fsl_semantics::check::wire_id::{
        check_component_key_package, component_key_structural, key_holds_index_segment,
        ComponentKeyRefusal,
    };
    let Value::Array(entries) = caller else {
        return Ok(caller.clone());
    };
    let packages = &facts.components;
    let refuse = |code: CatalogCode, pointer: String| {
        let mut detail = Map::new();
        detail.insert("member".to_owned(), json!("premises"));
        detail.insert("pointer".to_owned(), json!(pointer));
        Box::new(ResolvedItem::Settled {
            claim_name: Some(facts.name.clone()),
            record: Box::new(TerminalRecord::disposition_invalid_request(
                index, code, detail,
            )),
        })
    };
    let mut restated = entries.clone();
    let mut graphs: BTreeMap<String, Result<WireGraph, Unavailable>> = BTreeMap::new();
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
        let package = slots
            .get(at)
            .and_then(|slot| slot.guarantee.as_ref())
            .and_then(|clause| packages.of_component(&clause.component));
        check_component_key_package(package, key).map_err(|_| invalid())?;
        let Some(package) = package else {
            return Err(invalid());
        };
        if !key_holds_index_segment(key) {
            continue;
        }
        let assumed = facts.assumed_at(slots, at);
        let mut needed = vec![package];
        needed.extend(assumed.iter().filter_map(|node| packages.of_clause(node)));
        for identity in needed {
            if !graphs.contains_key(identity) {
                graphs.insert(identity.to_owned(), front.component_graph(identity));
            }
        }
        let graph_of = |package: &str| match graphs.get(package) {
            Some(Ok(graph)) => Ok(Some(graph)),
            Some(Err(failure)) => Err(unavailable(index, &facts.name, failure)),
            None => Ok(None),
        };
        let Some(graph) = graph_of(package)? else {
            return Err(invalid());
        };
        let mut peers = Vec::with_capacity(assumed.len());
        for node in &assumed {
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
                    without_cause("malformed_wire"),
                    format!("{key_pointer}{pointer}"),
                ));
            }
            Err(_) => return Err(invalid()),
        }
    }
    Ok(Value::Array(restated))
}

#[cfg(test)]
mod tests {
    use super::*;
    use fsl_package::{EmitRefusal, PackageAdmissionRefusal};
    use ix_trace_rs::trace;

    /// The code and cause the item settled with.
    fn settled(failure: &Unavailable) -> (&'static str, &'static str) {
        let item = unavailable(RequestIndex::new(0), "Sys", failure);
        let ResolvedItem::Settled { record, .. } = *item else {
            panic!("an unavailable graph settles the item");
        };
        let code = record.diagnostic().expect("a typed diagnostic");
        (code.code(), code.cause())
    }

    fn reader_refusal(code: Code, cause: Option<&str>) -> PackageAdmissionRefusal {
        PackageAdmissionRefusal {
            limit: None,
            code,
            cause: cause.map(str::to_owned),
            pointer: None,
            identity: None,
            host_allocation_failure: None,
        }
    }

    /// FR-1044-AC-15, no-graph clause: a component package whose emission
    /// reaches the emit limit settles with the emitter's own
    /// `stage_limit_exceeded` and the limit kind's cause; no other cause is
    /// written (FSpec FR-272's `insufficient-next-charge` has a payload the
    /// record does not carry).
    #[trace("TC-3155", "FR-1044-AC-15")]
    #[test]
    fn a_component_package_over_the_emit_limit_settles_with_the_emitters_limit_cause() {
        let limit = fsl_foundation::diagnostic::LimitExceeded::new(
            fsl_foundation::Setting::EmitOutputBytes,
            1,
            2,
        );
        let kind_cause = limit.kind().catalog_cause();
        let refusal = EmitRefusal::OutputLimit(limit);
        let failure = Unavailable::Emission(GraphUnavailable::of_not_admitted(Some(Box::new(
            refusal.clone(),
        ))));
        let (code, cause) = settled(&failure);
        assert_eq!(code, "stage_limit_exceeded");
        assert_eq!(cause, kind_cause);
        assert_eq!(refusal.cause(), Some(kind_cause));
        assert_ne!(cause, "insufficient-next-charge");
    }

    /// FR-1044-AC-15: an emission that ended with no refusal of its own is no
    /// limit stop and writes no cause.
    #[trace("TC-3155", "FR-1044-AC-15")]
    #[test]
    fn an_emission_without_a_refusal_is_no_limit_stop() {
        let (code, cause) = settled(&Unavailable::Emission(GraphUnavailable::of_not_admitted(
            None,
        )));
        assert_eq!(code, "invalid_package");
        assert_eq!(cause, "invalid-value");
    }

    /// FR-1044-AC-15: an `invalid_package` reader refusal without a cause keeps
    /// `invalid-value`; a reader cause is written as read.
    #[trace("TC-3155", "FR-1044-AC-15")]
    #[test]
    fn a_reader_refusal_without_a_cause_keeps_its_catalog_cause() {
        let none = reader_refusal(Code::InvalidPackage, None);
        assert_eq!(
            settled(&Unavailable::Refused(none)),
            ("invalid_package", "invalid-value")
        );
        let named = reader_refusal(Code::InvalidPackage, Some("ambiguous-name"));
        assert_eq!(
            settled(&Unavailable::Refused(named)),
            ("invalid_package", "ambiguous-name")
        );
    }

    /// FR-1044-AC-15: a component package that is no dependency of the
    /// request's package is the catalog's `missing_import`/`missing-selection`.
    #[trace("TC-3155", "FR-1044-AC-15")]
    #[test]
    fn a_component_package_that_is_no_dependency_settles_missing_import() {
        assert_eq!(
            settled(&Unavailable::NotDependency),
            ("missing_import", "missing-selection")
        );
    }
}
