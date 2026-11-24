// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-383 "Protocol term locations": one `checked_term_data.protocol_locations`
//! row for every term of every protocol slot expression the emitter writes.
//!
//! Each slot is walked in pre-order over its checked expression tree, the
//! children of a node taken in its `NodeKind` operand order. A position is a
//! term when the checker lowered an application node there: it then recorded
//! an `expression` occurrence of that node at the position's `Location`, and
//! the row is keyed to that occurrence and carries that `Location`. A
//! literal, a `Local` read, a `let`, a conditional and a `case` are not terms
//! (FR-383 assumption A1) and get no row. Nothing here derives a location
//! from a source region, a node order or an operand position.

use std::collections::{BTreeMap, HashMap, HashSet};

use filament_checked::node::NodeKind;
use filament_contract_model::{
    CheckedKeyOrdinal, CheckedNodeId, CheckedOccurrence, CheckedProtocolLocation,
    CheckedProtocolLocationOrigin, CheckedProtocolSemanticLocation,
};
use filament_exact::{NodeKey, PathSegment};
use filament_semantic_value::location::{
    ExpressionLocation as Location, ExpressionOwner as Origin,
};
use fsl_semantics::check::{
    BodyTerm, CheckedActivation, CheckedBody, CheckedGraph, CheckedLifetime, CheckedNode,
    CheckedOrdering, CheckedProtocol, CheckedTerminalMember, LeafTerm, NodeTag,
};

use super::{checked_occurrence, EmitRefusal};

/// The largest integer a JSON number carries exactly.
pub(super) const MAX_SAFE: u64 = 9_007_199_254_740_991;

/// One row before its `term_ordinal` is assigned.
pub(super) struct PendingRow<L = CheckedProtocolLocation> {
    pub(super) application: CheckedNodeId,
    pub(super) occurrence: CheckedOccurrence,
    /// The origin `name`, which orders the origins of one group.
    pub(super) origin_name: String,
    /// The visit's place in the emitter's walk of every slot, which orders
    /// the rows of one origin by visit.
    pub(super) visit: usize,
    pub(super) location: L,
}

/// Every `protocol_locations` row of the emitted protocols, in the
/// (application, occurrence, `term_ordinal`) order FSpec FR-322-AC-291
/// requires.
pub(super) fn checked_protocol_locations(
    graph: &CheckedGraph,
    emitted_node_ids: &HashMap<NodeKey, CheckedNodeId>,
) -> Result<Vec<CheckedProtocolSemanticLocation>, EmitRefusal> {
    let mut slots = Vec::new();
    let mut roots = HashSet::new();
    for protocol in graph.protocols() {
        // A refused protocol writes no node, so none of its slots is
        // emitted (FR-503).
        let emitted = graph
            .protocol_node_keys(&protocol.name)
            .and_then(|keys| keys.clause.as_ref())
            .is_some_and(|clause| emitted_node_ids.contains_key(clause));
        if !emitted {
            continue;
        }
        for body in slot_bodies(protocol) {
            if roots.insert(body.root().location()) {
                slots.push(body);
            }
        }
    }
    let origins: HashSet<&Origin> = roots.iter().map(|root| &root.origin).collect();
    let applications = applications_at(graph, &origins);
    let mut pending = Vec::new();
    for body in slots {
        slot_rows(graph, body, &applications, emitted_node_ids, &mut pending)?;
    }
    Ok(ordered(
        pending,
        |application, occurrence, term_ordinal, location| CheckedProtocolSemanticLocation {
            application,
            occurrence,
            term_ordinal,
            location,
        },
    ))
}

/// The rows of one slot's walk, appended to `pending` in visit order.
pub(super) fn slot_rows(
    graph: &CheckedGraph,
    body: &CheckedBody,
    applications: &HashMap<&Location, Vec<(NodeKey, filament_exact::Origin)>>,
    emitted_node_ids: &HashMap<NodeKey, CheckedNodeId>,
    pending: &mut Vec<PendingRow>,
) -> Result<(), EmitRefusal> {
    term_rows(
        graph,
        body.root(),
        applications,
        emitted_node_ids,
        pending,
        &|location, _, _| wire_location(location),
        false,
    )
}

/// A row's wire path and located payload, or `None` when the node is not
/// written.
type WireRow<L> = Option<(String, L)>;

/// The rows of the walk from `root`, appended to `pending` in visit order,
/// each located by `wire`. `deref_by_record` keys the `deref` of every
/// attribute the checker recorded one for, not only of a derefed one.
pub(super) fn term_rows<L>(
    graph: &CheckedGraph,
    root: CheckedNode<'_>,
    applications: &HashMap<&Location, Vec<(NodeKey, filament_exact::Origin)>>,
    emitted_node_ids: &HashMap<NodeKey, CheckedNodeId>,
    pending: &mut Vec<PendingRow<L>>,
    wire: &dyn Fn(&Location, &CheckedNodeId, &CheckedOccurrence) -> WireRow<L>,
    deref_by_record: bool,
) -> Result<(), EmitRefusal> {
    let positions = walk(root);
    // The application each position names, so a position below it at
    // the same location (an implicit coercion's operand) names another.
    let mut claimed: Vec<Option<&NodeKey>> = Vec::with_capacity(positions.len());
    let mut visits: HashMap<(&Location, &NodeKey), usize> = HashMap::new();
    let mut siblings: Vec<Vec<&NodeKey>> = vec![Vec::new(); positions.len()];
    for &(node, parent) in &positions {
        let at = node.location();
        let under = parent.and_then(|index| {
            let claim = claimed.get(index).copied().flatten()?;
            // Distinct operands of a query can share a child location even
            // when their retained parent has another location. Its actual
            // ordered operands still identify each visit.
            Some((claim, siblings[index].as_slice()))
        });
        let mut taken = Vec::new();
        let mut above = parent;
        while let Some(index) = above {
            let Some(&(ancestor, next)) = positions.get(index) else {
                break;
            };
            if ancestor.location() != at {
                break;
            }
            taken.extend(claimed.get(index).copied().flatten());
            above = next;
        }
        // A literal and a `Local` read lower to no application of their own.
        let application = match applications.get(at) {
            Some(candidates)
                if !matches!(node.kind(), NodeKind::Literal(_) | NodeKind::Local(_)) =>
            {
                term_application(graph, candidates, &taken, under)?
            }
            _ => None,
        };
        claimed.push(application);
        if let (Some(parent), Some(application)) = (parent, application) {
            siblings[parent].push(application);
        }
        if !is_term_kind(node) {
            continue;
        }
        let Some(application) = application else {
            // FR-383 writes one row per term. A kind that always lowers
            // to an application yet has none recorded at its position
            // leaves no application to key the row to: refuse rather
            // than drop the row.
            if always_application(node) {
                return Err(EmitRefusal::UnrecordedProtocolTerm {
                    location: at.clone(),
                });
            }
            continue;
        };
        // An attribute read through `deref` lowers two terms at its one
        // position, the `project` and the `deref` it reads; each is keyed.
        // A function body is read from the wire, whose walk visits every
        // `deref` node it reaches, and a clause function's `self` read lowers
        // one for an attribute the checker does not flag as derefed; so for a
        // body the recorded `deref` applications decide, not the flag.
        let mut claimed_here = vec![application];
        let derefs = match node.kind() {
            NodeKind::Attribute { derefed, .. } => *derefed || deref_by_record,
            _ => false,
        };
        if derefs {
            claimed_here.extend(
                applications
                    .get(at)
                    .into_iter()
                    .flatten()
                    .map(|(key, _)| key)
                    .filter(|key| is_deref_application(key) && !taken.contains(key))
                    .collect::<std::collections::BTreeSet<_>>(),
            );
        }
        for application in claimed_here {
            // The `j`th visit of one position takes the `j`th occurrence
            // recorded there; a position recorded once keeps that one.
            let recorded: Vec<&filament_exact::Origin> = applications
                .get(at)
                .into_iter()
                .flatten()
                .filter(|(key, _)| key == application)
                .map(|(_, origin)| origin)
                .collect();
            let seen = visits.entry((at, application)).or_default();
            let origin = recorded
                .get(*seen)
                .or_else(|| recorded.last())
                .copied()
                .ok_or_else(|| refusal(application))?;
            *seen += 1;
            let row_application = emitted_node_ids
                .get(application)
                .cloned()
                .ok_or_else(|| refusal(application))?;
            let occurrence = checked_occurrence(origin).ok_or_else(|| refusal(application))?;
            let (origin_name, location) =
                wire(at, &row_application, &occurrence).ok_or_else(|| refusal(application))?;
            pending.push(PendingRow {
                application: row_application,
                occurrence,
                origin_name,
                visit: pending.len(),
                location,
            });
        }
    }
    Ok(())
}

fn refusal(application: &NodeKey) -> EmitRefusal {
    EmitRefusal::MissingProtocolLocationContext {
        application: application.clone(),
    }
}

/// Number each group's rows from zero across its origins, the origins in
/// origin order and one origin's rows in visit order, then order every row
/// by (application, occurrence, `term_ordinal`).
pub(super) fn ordered<L, R>(
    pending: Vec<PendingRow<L>>,
    build: impl Fn(CheckedNodeId, CheckedOccurrence, u64, L) -> R,
) -> Vec<R> {
    let mut groups: BTreeMap<(CheckedNodeId, CheckedOccurrence), Vec<PendingRow<L>>> =
        BTreeMap::new();
    for row in pending {
        groups
            .entry((row.application.clone(), row.occurrence.clone()))
            .or_default()
            .push(row);
    }
    let mut rows = Vec::new();
    for (_, mut group) in groups {
        group.sort_by(|left, right| {
            left.origin_name
                .as_bytes()
                .cmp(right.origin_name.as_bytes())
                .then(left.visit.cmp(&right.visit))
        });
        rows.extend(group.into_iter().zip(0_u64..).map(|(row, term_ordinal)| {
            build(row.application, row.occurrence, term_ordinal, row.location)
        }));
    }
    rows
}

/// Every slot expression of `protocol` (FR-383's site-name registry). A
/// `repeat`'s loop proof is not a slot (FR-390) and is not listed.
pub(crate) fn slot_bodies(protocol: &CheckedProtocol) -> Vec<&CheckedBody> {
    let mut bodies = Vec::new();
    if let CheckedActivation::OnEach {
        guard: Some(guard), ..
    } = &protocol.obligations.activation
    {
        bodies.push(guard);
    }
    bodies.extend(
        protocol
            .obligations
            .captures
            .iter()
            .map(|capture| &capture.initializer),
    );
    for compensation in &protocol.obligations.compensations {
        bodies.push(&compensation.guard);
        bodies.push(&compensation.retry.relation);
        bodies.push(&compensation.recover.predicate);
    }
    bodies.extend(protocol.slots.bodies.iter().map(|body| &body.body));
    for choice in &protocol.slots.choices {
        bodies.extend(&choice.visible);
        bodies.extend(choice.cases.iter().map(|case| &case.guard));
    }
    for repeat in &protocol.slots.repeats {
        bodies.extend(&repeat.visible);
        bodies.push(&repeat.guard);
    }
    for related in &protocol.slots.related {
        bodies.extend(&related.ends);
    }
    if let CheckedTerminalMember::When { predicate, .. } = &protocol.slots.terminal {
        bodies.push(predicate);
    }
    for role in &protocol.slots.roles {
        if let CheckedLifetime::Until { predicate, .. } = &role.lifetime {
            bodies.push(predicate);
        }
    }
    for channel in &protocol.channels {
        if let CheckedOrdering::Fifo { key, .. } = &channel.ordering {
            bodies.push(key);
        }
    }
    bodies
}

/// The positions below `root` in pre-order, children in operand order, a node
/// reached through two parents once per parent, each with the index of its
/// parent position.
fn walk(root: CheckedNode<'_>) -> Vec<(CheckedNode<'_>, Option<usize>)> {
    let mut visited = Vec::new();
    let mut stack = vec![(root, None)];
    while let Some((node, parent)) = stack.pop() {
        let index = visited.len();
        visited.push((node, parent));
        stack.extend(
            node.children()
                .into_iter()
                .rev()
                .map(|child| (child, Some(index))),
        );
    }
    visited
}

/// Whether a position can be a term: a literal, a `Local` read, a `let`, a
/// conditional and a `case` never are (FR-383 assumption A1), even where
/// lowering keys a node for them.
pub(super) fn is_term_kind(node: CheckedNode<'_>) -> bool {
    !matches!(
        node.kind(),
        NodeKind::Literal(_)
            | NodeKind::Local(_)
            | NodeKind::Let { .. }
            | NodeKind::If { .. }
            | NodeKind::Case { .. }
    )
}

/// Whether a position's kind always lowers to an application node, so a
/// position of it with no recorded application is a lowering gap rather than
/// a term the checker folded away.
fn always_application(node: CheckedNode<'_>) -> bool {
    matches!(
        node.kind(),
        NodeKind::Coerce(..)
            | NodeKind::Arithmetic(..)
            | NodeKind::Order(..)
            | NodeKind::Equality(..)
            | NodeKind::Connective(..)
            | NodeKind::Not(_)
            | NodeKind::Field { .. }
            | NodeKind::Attribute { .. }
            | NodeKind::Call { .. }
    )
}

/// Each `expression` occurrence of an application node located under one of
/// `origins`, by location, ascending by node and then ordinal.
pub(super) fn applications_at<'g>(
    graph: &'g CheckedGraph,
    origins: &HashSet<&Origin>,
) -> HashMap<&'g Location, Vec<(NodeKey, filament_exact::Origin)>> {
    let mut at: HashMap<&'g Location, Vec<(NodeKey, filament_exact::Origin)>> = HashMap::new();
    for (key, origin, location) in graph.occurrences() {
        if !crate::wire_form::is_expression_role(origin.role().as_str())
            || !origins.contains(&location.origin)
        {
            continue;
        }
        let application = graph
            .semantic_graph()
            .node(key.clone())
            .is_some_and(|node| {
                node.node_tag() == NodeTag::Expression
                    && matches!(node.body(), BodyTerm::Application(_))
            });
        if application {
            at.entry(location).or_default().push((key, origin));
        }
    }
    at
}

/// Every expression node reachable from the operands of `key`, through
/// nodes at other locations too: a lowering that joins its parts through a
/// binding node (an IEEE ordering binds its operands once) reaches them
/// only that way.
fn reachable_operands(graph: &CheckedGraph, key: &NodeKey) -> HashSet<NodeKey> {
    let mut reached: HashSet<NodeKey> = HashSet::new();
    let mut pending: Vec<NodeKey> = vec![key.clone()];
    while let Some(key) = pending.pop() {
        let Some(node) = graph.semantic_graph().node(key) else {
            continue;
        };
        // A call's callee is a function node whose body is its own walk.
        if node.node_tag() != NodeTag::Expression {
            continue;
        }
        for leaf in node.body().leaves() {
            let LeafTerm::Reference { target } = leaf else {
                continue;
            };
            let Some(target) = target.admitted() else {
                continue;
            };
            if reached.insert(target.clone()) {
                pending.push(target.clone());
            }
        }
    }
    reached
}

/// The application a position names: of the applications lowered at its
/// location and not `taken` by an enclosing position at the same location,
/// the parent's next direct operand, or the one no other candidate takes as
/// an operand when no parent operand names the visit. An attribute read lowers a
/// `deref` and the `project` over it at one position; an implicit coercion
/// and its operand are two positions at one location.
fn term_application<'c>(
    graph: &CheckedGraph,
    candidates: &'c [(NodeKey, filament_exact::Origin)],
    taken: &[&NodeKey],
    under: Option<(&NodeKey, &[&NodeKey])>,
) -> Result<Option<&'c NodeKey>, EmitRefusal> {
    let mut keys: Vec<&NodeKey> = Vec::new();
    for (key, _) in candidates {
        if !keys.contains(&key) && !taken.contains(&key) {
            keys.push(key);
        }
    }
    // One application at the position is the position's own; only several
    // need the parent operand context or ordering by reach.
    if let [only] = keys[..] {
        return Ok(Some(only));
    }
    // A shared descendant can be read by several sibling parts of one
    // lowering. The parent's direct operand order identifies this visit
    // before transitive reach through another sibling can hide it. Count
    // visits rather than distinct keys: both operands may name one node.
    if let Some((parent, earlier)) = under {
        if let Some(parent) = graph.semantic_graph().node(parent.clone()) {
            let seen = earlier.iter().filter(|key| keys.contains(key)).count();
            let named = parent
                .body()
                .leaves()
                .into_iter()
                .filter_map(|leaf| match leaf {
                    LeafTerm::Reference { target } => target.admitted(),
                    LeafTerm::Literal { .. } | LeafTerm::DependencyReference { .. } => None,
                })
                .filter(|operand| keys.contains(operand))
                .nth(seen);
            if let Some(key) = named.and_then(|operand| keys.iter().find(|key| **key == operand)) {
                return Ok(Some(*key));
            }
        }
    }
    // A recursive call reaches its own application again: only another
    // application at the position makes one an operand.
    let reach: Vec<HashSet<NodeKey>> = keys
        .iter()
        .map(|key| reachable_operands(graph, key))
        .collect();
    let outermost: Vec<&&NodeKey> = keys
        .iter()
        .enumerate()
        .filter(|(index, key)| {
            !reach
                .iter()
                .enumerate()
                .any(|(other, operands)| other != *index && operands.contains(**key))
        })
        .map(|(_, key)| key)
        .collect();
    let mut outer = outermost.into_iter();
    match (outer.next(), outer.next()) {
        (Some(key), None) => Ok(Some(key)),
        (None, None) if keys.is_empty() => Ok(None),
        // Two outermost applications, or a cycle, at one position: the
        // lowering left no single term there.
        (Some(key), Some(_)) => Err(refusal(key)),
        (None, _) => Err(keys
            .first()
            .map_or(EmitRefusal::SourceMapInvariant, |key| refusal(key))),
    }
}

/// The child-index path of `location` as the wire carries it.
pub(super) fn wire_path(location: &Location) -> Option<Vec<CheckedKeyOrdinal>> {
    location
        .path()
        .into_iter()
        .map(|index| {
            let index = u64::try_from(index).ok()?;
            Some(if index <= MAX_SAFE {
                CheckedKeyOrdinal::Safe(index)
            } else {
                CheckedKeyOrdinal::BeyondSafe(index.to_string().into())
            })
        })
        .collect()
}

/// The checker's `Location` of a term as the wire carries it, with the
/// slot's site name: every slot expression is located under a
/// type-declaration origin named by its site (FR-383), never under a
/// `protocol_attempt` origin.
fn wire_location(location: &Location) -> Option<(String, CheckedProtocolLocation)> {
    let name = match &location.origin {
        Origin::TypeDeclaration { name } => name.clone(),
        Origin::Body { .. }
        | Origin::Measure { .. }
        | Origin::Expression
        | Origin::StateClause { .. }
        | Origin::ProtocolAttempt { .. }
        | Origin::ModelOperationClause { .. } => return None,
    };
    let path = wire_path(location)?;
    let origin = CheckedProtocolLocationOrigin::TypeDeclaration {
        name: name.as_str().into(),
    };
    Some((name, CheckedProtocolLocation { origin, path }))
}

/// Whether `key` is the key of a `deref` application node.
#[ix_trace_rs::string_edge]
fn is_deref_application(key: &NodeKey) -> bool {
    matches!(
        key.path().as_slice(),
        [PathSegment::Text(form), PathSegment::Text(kind), ..] if form == "expression" && kind == "deref"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use filament_semantic_value::checking::{CheckMode, CheckingLimits};
    use fsl_semantics::check::{test_support::ZeroRewardEvaluator, PackageDeclarations};
    use ix_trace_rs::trace;

    /// A query's source and predicate share a child location but belong to
    /// distinct ordered operands of its actual retained root. Both terms
    /// must keep their own application and occurrence in the admitted rows.
    #[trace("FR-383", "TC-1063")]
    #[test]
    fn query_siblings_share_a_location_but_keep_their_actual_parent_operands() {
        let document = include_bytes!("../../../examples/config-version/model.semantic-ir.json");
        let packages = fsl_semantics::model::intake::package_input([document.as_slice()])
            .expect("the actual ConfigVersion model admits");
        let source = r#"language "ix:native" edition "1-draft";
profile v = "filament.model.complete/v1";
profile p = "filament.protocol.finite-global/v1";
model C = "example/config-version";
protocol Places using p over (call: Reference<C::ConfigVersion>) on each (opened: Boolean) when (opened) {
  capture chosen: Boolean = true;
  capture used: Integer = count(c in C::config_history where c = call);
  role Observer on C::ConfigVersion;
  run sequence Main { check Ready using v { true }; }
  finish End as (outcome: Boolean) { true };
}
"#;
        let parsed = fsl_cst::parse(
            fsl_foundation::SourceIdentity::new("agent-ix", "test:query-locations"),
            "query.native",
            source.as_bytes(),
            fsl_cst::Limits::default(),
        )
        .expect("the source parses");
        assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
        let unit = fsl_forms::build_unit(&parsed).expect("the source forms build");
        let models = fsl_semantics::model::intake::admit_unit(
            &unit.selections().models,
            &packages,
            fsl_semantics::model::accounting::ModelNormalizationLimits::default(),
        )
        .expect("the selected Model admits");
        let graph = PackageDeclarations::assemble(
            parsed.source().reference().clone(),
            "fixture".to_owned(),
            unit,
            models,
            Vec::new(),
        )
        .expect("the source assembles")
        .check(
            CheckMode::Linked,
            CheckingLimits::default(),
            &ZeroRewardEvaluator,
        )
        .expect("the protocol checks");
        let capture = graph.protocols()[0]
            .obligations
            .captures
            .iter()
            .find(|capture| capture.name == "used")
            .expect("the actual count capture");
        let root = capture.initializer.root().location();
        let origins = HashSet::from([&root.origin]);
        let applications = applications_at(&graph, &origins);
        let root_keys = applications.get(root).expect("the lowered count root");
        // The query's source and predicate can share the root's location.
        // Select the retained count by its two actual direct operands instead
        // of assuming a source location uniquely identifies one application.
        let roots_with_two_operands = root_keys
            .iter()
            .filter_map(|(key, _)| {
                let operands = graph
                    .semantic_graph()
                    .node(key.clone())?
                    .body()
                    .leaves()
                    .into_iter()
                    .filter_map(|leaf| match leaf {
                        LeafTerm::Reference { target } => target.admitted().cloned(),
                        _ => None,
                    })
                    .collect::<Vec<_>>();
                (operands.len() == 2).then_some((key.clone(), operands))
            })
            .collect::<Vec<_>>();
        let [(_, operands)] = roots_with_two_operands.as_slice() else {
            panic!("one actual retained count root with its two operands");
        };
        assert_eq!(
            operands.len(),
            2,
            "the population source and equality predicate"
        );
        let (shared, _) = applications
            .iter()
            .find(|(_, candidates)| {
                operands
                    .iter()
                    .all(|operand| candidates.iter().any(|(key, _)| key == operand))
            })
            .expect("both direct operands really share one recorded location");
        assert_ne!(
            *shared, root,
            "the retained parent has a different location"
        );
        let name = match &shared.origin {
            Origin::TypeDeclaration { name } => name.clone(),
            _ => panic!("an actual protocol slot origin"),
        };
        let path = shared.path();
        let package = crate::CheckedPackage::link(graph);
        let read = crate::read_evaluation_package(
            &package,
            fsl_semantics::library::LibraryName::new("fixture").expect("library"),
            &packages,
            crate::V2ReadLimits::fixture(),
            super::super::EmitLimits::fixture(),
        )
        .expect("the actual emitted protocol and all its locations admit");
        let wire: serde_json::Value = serde_json::from_slice(read.emission().package.bytes())
            .expect("the actual emitted wire");
        let structural = crate::checked_v2::wire_graph(read.admitted())
            .expect("the admitted structural correspondence");
        let rows = wire["checked_term_data"]["protocol_locations"]
            .as_array()
            .expect("the actual protocol rows");
        for operand in operands {
            let matching = rows
                .iter()
                .filter(|row| {
                    if row["location"]["origin"]["name"] != name
                        || row["location"]["path"] != serde_json::json!(path)
                    {
                        return false;
                    }
                    let id: CheckedNodeId = serde_json::from_value(row["application"].clone())
                        .expect("the actual application id");
                    structural.key(&id).expect("the actual producer key") == *operand
                })
                .count();
            assert_eq!(
                matching, 1,
                "each distinct source/predicate operand keeps its own row"
            );
        }
    }
}
