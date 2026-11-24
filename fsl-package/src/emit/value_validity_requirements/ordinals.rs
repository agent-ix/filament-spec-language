// SPDX-License-Identifier: AGPL-3.0-or-later
//! FSpec FR-322 "Expression occurrence ordinals": the expression occurrences
//! of a node are numbered by the package-wide owner walk, not by the order the
//! checker met the uses in.
//!
//! The checker numbers a node's uses in its own lowering order and counts
//! every use, including the uses of an owner that is not written. The walk
//! consumes the next `expression` occurrence of a node at each visit, owners in
//! FR-322 order, each owner's body and then its termination measure. This
//! module renumbers each affected node's occurrences so the k-th visit takes
//! ordinal k. A visit in an owner takes the use the checker located in that
//! owner at the next place in the body's source order; uses no walk reaches
//! (state-clause bodies) follow the visits, and the uses of an omitted owner
//! are dropped from the nodes that are kept. The source-map regions follow
//! their uses, and so does every row that names one.

use std::collections::{BTreeMap, HashMap, HashSet};

use filament_contract_model::{
    CheckedNodeId, CheckedOccurrence, CheckedOccurrenceRole, CheckedSemanticNodeV2,
    CheckedSourceMapEntry, CheckedTerm,
};
use filament_exact::NodeKey;
use filament_semantic_value::location::ExpressionOwner;
use fsl_semantics::check::wire_id::StructuralIds;
use fsl_semantics::check::CheckedGraph;

use super::{frames, EmitRefusal, Omitted};

/// What the checker located a use in: the declaration index of the function
/// whose body or measure holds it, or somewhere no walk reaches.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum Use {
    Body(usize),
    Measure(usize),
    Other,
}

/// The new ordinal of each checker ordinal of one node, ascending by checker
/// ordinal; `None` is a use that is dropped with its omitted owner.
pub(in crate::emit) type Mapping = Vec<(u64, Option<u64>)>;

/// The new ordinal of each renumbered expression occurrence, by node.
#[derive(Default)]
pub(in crate::emit) struct Renumbering {
    by_node: HashMap<NodeKey, Mapping>,
}

impl Renumbering {
    /// The ordinal the package gives the checker's expression occurrence
    /// `old` of `node`; `None` when the use is dropped with its owner.
    pub(in crate::emit) fn ordinal(&self, node: &NodeKey, old: u64) -> Option<u64> {
        match self.by_node.get(node) {
            None => Some(old),
            Some(mapping) => match mapping.binary_search_by_key(&old, |(old, _)| *old) {
                Ok(at) => mapping[at].1,
                Err(_) => Some(old),
            },
        }
    }

    /// Whether any expression occurrence took another ordinal.
    pub(in crate::emit) fn renumbers(&self) -> bool {
        !self.by_node.is_empty()
    }

    /// Move every checked-term row naming a renumbered expression occurrence
    /// to the occurrence's new ordinal, and put each changed table back in
    /// its canonical order. `ids` names each node's id in the rows.
    pub(in crate::emit) fn relabel_term_data(
        &self,
        ids: &HashMap<NodeKey, CheckedNodeId>,
        data: &mut filament_contract_model::CheckedTermDataV2,
    ) {
        let mut by_id = BTreeMap::new();
        for (key, mapping) in &self.by_node {
            if let Some(id) = ids.get(key) {
                by_id.insert(id.clone(), mapping.clone());
            }
        }
        relabel_term_data(&by_id, data);
    }
}

/// The first and later uses of a node, to find the nodes the numbering can
/// change: a node used from one place keeps the checker's numbering, since its
/// uses are met in the order the walk visits them.
fn multiply_used(graph: &CheckedGraph) -> HashSet<NodeKey> {
    let mut first = HashMap::<NodeKey, Use>::new();
    let mut multiple = HashSet::new();
    for (key, origin, location) in graph.occurrences() {
        if !crate::wire_form::is_expression_role(origin.role().as_str()) {
            continue;
        }
        let used = use_of(&location.origin);
        match first.entry(key) {
            std::collections::hash_map::Entry::Vacant(entry) => {
                entry.insert(used);
            }
            std::collections::hash_map::Entry::Occupied(entry) => {
                if *entry.get() != used {
                    multiple.insert(entry.key().clone());
                }
            }
        }
    }
    multiple
}

fn use_of(owner: &ExpressionOwner) -> Use {
    match owner {
        ExpressionOwner::Body { index, .. } => Use::Body(*index),
        ExpressionOwner::Measure { index, .. } => Use::Measure(*index),
        _ => Use::Other,
    }
}

/// Renumber the expression occurrences of `nodes` and `source_map` by the
/// package-wide owner walk.
pub(in crate::emit) fn renumber(
    ids: &StructuralIds,
    graph: &CheckedGraph,
    nodes: &mut [CheckedSemanticNodeV2],
    source_map: &mut Vec<CheckedSourceMapEntry>,
    omitted: &Omitted,
    meter: &mut super::super::OutputMeter,
) -> Result<Renumbering, EmitRefusal> {
    // Only a node used from more than one place can need another numbering, so
    // a package with none is not walked.
    let needed = multiply_used(graph);
    if needed.is_empty() {
        return Ok(Renumbering::default());
    }
    let invariant = |_| EmitRefusal::SourceMapInvariant;
    let mut function_index = HashMap::<NodeKey, usize>::new();
    let mut omitted_functions = HashSet::<usize>::new();
    let mut at = 0;
    while let Some(identity) = graph.function_identity_at(at) {
        if omitted.contains_key(&identity) {
            omitted_functions.insert(at);
        }
        function_index.insert(identity, at);
        at += 1;
    }
    let mut keys = ids.key_reader();
    let mut node_keys = Vec::with_capacity(nodes.len());
    let mut keep = Vec::with_capacity(nodes.len());
    for node in nodes.iter() {
        let key = keys.key(&node.node_id).map_err(invariant)?;
        keep.push(needed.contains(&key));
        node_keys.push(key);
    }
    // The visits each needed node receives, as the use of the owner that made
    // each; an owner that is no declared function has no use to take.
    let walked = frames::visits(nodes, source_map, &keep);
    meter.charge_bytes(
        (walked.len() as u64).saturating_mul(std::mem::size_of::<frames::Visit>() as u64),
    )?;
    let mut owner_use = HashMap::<usize, usize>::new();
    let mut visited = HashMap::<usize, Vec<Use>>::new();
    for visit in &walked {
        let function = match owner_use.get(&visit.owner) {
            Some(&function) => function,
            None => {
                let function = *function_index
                    .get(&node_keys[visit.owner])
                    .ok_or(EmitRefusal::SourceMapInvariant)?;
                owner_use.insert(visit.owner, function);
                function
            }
        };
        visited
            .entry(visit.node)
            .or_default()
            .push(if visit.measure {
                Use::Measure(function)
            } else {
                Use::Body(function)
            });
    }
    drop(walked);
    // The uses the checker located, each node's by owner in the order of the
    // body's source positions.
    let mut located = HashMap::<&NodeKey, HashMap<Use, Vec<(Vec<usize>, u64)>>>::new();
    let by_key: HashSet<&NodeKey> = node_keys
        .iter()
        .zip(&keep)
        .filter(|(_, keep)| **keep)
        .map(|(key, _)| key)
        .collect();
    for (key, origin, location) in graph.occurrences() {
        if !crate::wire_form::is_expression_role(origin.role().as_str()) {
            continue;
        }
        let Some(&key) = by_key.get(&key) else {
            continue;
        };
        located
            .entry(key)
            .or_default()
            .entry(use_of(&location.origin))
            .or_default()
            .push((location.path(), origin.ordinal()));
    }
    let mut renumbering = Renumbering::default();
    let mut changed = BTreeMap::<usize, Mapping>::new();
    for (position, key) in node_keys.iter().enumerate() {
        let Some(groups) = located.get_mut(key) else {
            continue;
        };
        let mut mapping: Mapping = Vec::new();
        let mut kept: HashMap<Use, Vec<u64>> = HashMap::new();
        for (used, uses) in groups.iter_mut() {
            uses.sort();
            let dropped = match used {
                Use::Body(function) | Use::Measure(function) => {
                    omitted_functions.contains(function)
                }
                Use::Other => false,
            };
            if dropped {
                mapping.extend(uses.iter().map(|(_, old)| (*old, None)));
            } else {
                kept.insert(*used, uses.iter().map(|(_, old)| *old).collect());
            }
        }
        // A node only an omitted owner uses keeps the occurrences it has.
        if kept.is_empty() {
            continue;
        }
        // Each visit takes the next use located in its owner; a visit with no
        // use to take means the walk and the checker disagree on the body.
        let mut next = HashMap::<Use, usize>::new();
        let mut order = Vec::new();
        let mut exhausted = false;
        // Only an application carries records, so only its numbering is checked
        // against the visits; a parameter or literal several owners share is read
        // by walks that reach it more often than the checker located a use.
        let applies = matches!(nodes[position].body, CheckedTerm::Application { .. });
        for used in visited.get(&position).map_or(&[][..], Vec::as_slice) {
            let at = next.entry(*used).or_default();
            let Some(old) = kept.get(used).and_then(|uses| uses.get(*at)) else {
                // A node reached again after its uses are taken: the visit past
                // the last occurrence reads none, which is harmless only while no
                // later visit has a use to read at a shifted place.
                exhausted = true;
                continue;
            };
            if exhausted && applies {
                return Err(EmitRefusal::SourceMapInvariant);
            }
            *at += 1;
            order.push(*old);
        }
        // The uses no walk reaches follow the visits, in checker order.
        let taken: HashSet<u64> = order.iter().copied().collect();
        let mut rest: Vec<u64> = kept
            .values()
            .flatten()
            .copied()
            .filter(|old| !taken.contains(old))
            .collect();
        rest.sort_unstable();
        order.extend(rest);
        mapping.extend(
            order
                .iter()
                .enumerate()
                .map(|(new, old)| (*old, Some(new as u64))),
        );
        mapping.sort_unstable();
        if mapping.iter().all(|(old, new)| Some(*old) == *new) {
            continue;
        }
        renumbering.by_node.insert(key.clone(), mapping.clone());
        changed.insert(position, mapping);
    }
    let removed = apply(nodes, source_map, &changed);
    if !removed.is_empty() {
        let mut at = 0;
        source_map.retain(|_| {
            let keep = !removed.contains(&at);
            at += 1;
            keep
        });
    }
    Ok(renumbering)
}

/// Write the new numbering into the occurrences of the nodes at the positions
/// `changed` names and into their source-map entries: each region moves to its
/// use's new ordinal. The positions of the entries of a dropped use are
/// returned for the caller to remove.
fn apply(
    nodes: &mut [CheckedSemanticNodeV2],
    source_map: &mut [CheckedSourceMapEntry],
    changed: &BTreeMap<usize, Mapping>,
) -> HashSet<usize> {
    if changed.is_empty() {
        return HashSet::new();
    }
    let mut affected = BTreeMap::<CheckedNodeId, usize>::new();
    for (&position, mapping) in changed {
        affected.insert(nodes[position].node_id.clone(), position);
        let node = &mut nodes[position];
        let count = mapping.iter().filter(|(_, new)| new.is_some()).count() as u64;
        let first = node
            .occurrences
            .iter()
            .position(|occurrence| occurrence.role == CheckedOccurrenceRole::Expression);
        node.occurrences
            .retain(|occurrence| occurrence.role != CheckedOccurrenceRole::Expression);
        if let Some(first) = first {
            let first = first.min(node.occurrences.len());
            node.occurrences.splice(
                first..first,
                (0..count).map(|ordinal| CheckedOccurrence {
                    role: CheckedOccurrenceRole::Expression,
                    ordinal,
                }),
            );
        }
    }
    let mut slots = BTreeMap::<usize, Vec<usize>>::new();
    for (entry_position, entry) in source_map.iter().enumerate() {
        if entry.role != CheckedOccurrenceRole::Expression {
            continue;
        }
        if let Some(&position) = affected.get(&entry.node_id) {
            slots.entry(position).or_default().push(entry_position);
        }
    }
    let mut removed = HashSet::new();
    for (position, entry_positions) in slots {
        let mapping = &changed[&position];
        let mut moved: Vec<CheckedSourceMapEntry> = Vec::new();
        for &entry_position in &entry_positions {
            let entry = &source_map[entry_position];
            let Ok(at) = mapping.binary_search_by_key(&entry.ordinal, |(old, _)| *old) else {
                continue;
            };
            if let Some(new) = mapping[at].1 {
                let mut entry = entry.clone();
                entry.ordinal = new;
                moved.push(entry);
            }
        }
        moved.sort_by_key(|entry| entry.ordinal);
        let mut moved = moved.into_iter();
        for &entry_position in &entry_positions {
            match moved.next() {
                Some(entry) => source_map[entry_position] = entry,
                None => {
                    removed.insert(entry_position);
                }
            }
        }
    }
    removed
}

/// The numbering a writer that reorders the owners (a rekey of declaration-less
/// functions) gives the nodes of `nodes`, which hold the base package's nodes at
/// their base positions: each node's uses keep their owners, so the j-th visit
/// of an owner takes the ordinal the owner's j-th visit takes in the new walk.
/// The nodes and the source map are renumbered; the mapping by final node id is
/// returned for the rows that name an occurrence.
pub(in crate::emit) fn rewalk(
    base_nodes: &[CheckedSemanticNodeV2],
    base_source_map: &[CheckedSourceMapEntry],
    nodes: &mut [CheckedSemanticNodeV2],
    source_map: &mut [CheckedSourceMapEntry],
) -> Result<BTreeMap<CheckedNodeId, Mapping>, EmitRefusal> {
    let carry = |reason| EmitRefusal::RequirementRowCarry(reason);
    let finals = nodes
        .get(..base_nodes.len())
        .ok_or(carry("the writer's graph holds fewer nodes than the base"))?;
    if frames::owner_order(base_nodes, base_source_map) == frames::owner_order(finals, source_map) {
        return Ok(BTreeMap::new());
    }
    let keep: Vec<bool> = base_nodes
        .iter()
        .map(|node| {
            node.occurrences
                .iter()
                .filter(|occurrence| occurrence.role == CheckedOccurrenceRole::Expression)
                .count()
                > 1
        })
        .collect();
    let owners = |nodes: &[CheckedSemanticNodeV2], source_map: &[CheckedSourceMapEntry]| {
        let mut owners = HashMap::<usize, Vec<(usize, bool)>>::new();
        for visit in frames::visits(nodes, source_map, &keep) {
            owners
                .entry(visit.node)
                .or_default()
                .push((visit.owner, visit.measure));
        }
        owners
    };
    let before = owners(base_nodes, base_source_map);
    let after = owners(finals, source_map);
    let mut changed = BTreeMap::<usize, Mapping>::new();
    for (position, base) in &before {
        let Some(now) = after.get(position) else {
            return Err(carry("a rekey changed which nodes the owner walk reaches"));
        };
        if base.len() != now.len() {
            return Err(carry(
                "a rekey changed how often the owner walk visits a node",
            ));
        }
        // The j-th visit of an owner takes, in the new order, the ordinal the
        // owner's j-th visit is given there.
        let mut seen = HashMap::<(usize, bool), usize>::new();
        let mut places = HashMap::<(usize, bool, usize), usize>::new();
        for (new, use_of_visit) in now.iter().enumerate() {
            let nth = seen.entry(*use_of_visit).or_default();
            places.insert((use_of_visit.0, use_of_visit.1, *nth), new);
            *nth += 1;
        }
        let mut seen = HashMap::<(usize, bool), usize>::new();
        let mut mapping: Mapping = Vec::with_capacity(base.len());
        for (old, use_of_visit) in base.iter().enumerate() {
            let nth = seen.entry(*use_of_visit).or_default();
            let new = *places
                .get(&(use_of_visit.0, use_of_visit.1, *nth))
                .ok_or(carry("a rekey changed an owner's visits of a node"))?;
            *nth += 1;
            mapping.push((old as u64, Some(new as u64)));
        }
        let listed = base_nodes[*position]
            .occurrences
            .iter()
            .filter(|occurrence| occurrence.role == CheckedOccurrenceRole::Expression)
            .count();
        // Uses the walk does not reach keep their ordinals after the visits.
        mapping.extend((mapping.len()..listed).map(|rest| (rest as u64, Some(rest as u64))));
        if mapping.iter().any(|(old, new)| Some(*old) != *new) {
            changed.insert(*position, mapping);
        }
    }
    let removed = apply(nodes, source_map, &changed);
    debug_assert!(removed.is_empty(), "a rekey drops no use");
    Ok(changed
        .into_iter()
        .map(|(position, mapping)| (nodes[position].node_id.clone(), mapping))
        .collect())
}

/// Move the checked-term rows that name a renumbered expression occurrence of
/// the nodes `by_id` holds, and drop the rows of a dropped use.
pub(in crate::emit) fn relabel_term_data(
    by_id: &BTreeMap<CheckedNodeId, Mapping>,
    data: &mut filament_contract_model::CheckedTermDataV2,
) {
    if by_id.is_empty() {
        return;
    }
    // Some(true): the row moved, Some(false): unchanged, None: dropped.
    let relabel = |application: &CheckedNodeId, occurrence: &mut CheckedOccurrence| {
        if occurrence.role != CheckedOccurrenceRole::Expression {
            return Some(false);
        }
        let mapping = by_id.get(application)?;
        let Ok(at) = mapping.binary_search_by_key(&occurrence.ordinal, |(old, _)| *old) else {
            return Some(false);
        };
        let new = mapping[at].1?;
        let moved = new != occurrence.ordinal;
        occurrence.ordinal = new;
        Some(moved)
    };
    // A node the map does not hold is never relabelled, and `by_id.get(..)?`
    // above would drop it: distinguish "no entry" from "dropped".
    let relabel = |application: &CheckedNodeId, occurrence: &mut CheckedOccurrence| {
        if by_id.contains_key(application) {
            relabel(application, occurrence)
        } else {
            Some(false)
        }
    };
    let mut changed = false;
    data.equality_decisions.retain_mut(|row| {
        let moved = relabel(&row.application, &mut row.occurrence);
        changed |= moved != Some(false);
        moved.is_some()
    });
    if changed {
        data.equality_decisions.sort_by(|left, right| {
            (&left.application, &left.occurrence).cmp(&(&right.application, &right.occurrence))
        });
    }
    let mut changed = false;
    data.dispatch_tables.retain_mut(|row| {
        let moved = relabel(&row.application, &mut row.occurrence);
        changed |= moved != Some(false);
        moved.is_some()
    });
    if changed {
        data.dispatch_tables.sort_by(|left, right| {
            (&left.application, &left.occurrence).cmp(&(&right.application, &right.occurrence))
        });
    }
    let mut changed = false;
    data.protocol_locations.retain_mut(|row| {
        let moved = relabel(&row.application, &mut row.occurrence);
        changed |= moved != Some(false);
        moved.is_some()
    });
    if changed {
        data.protocol_locations.sort_by(|left, right| {
            (&left.application, &left.occurrence, left.term_ordinal).cmp(&(
                &right.application,
                &right.occurrence,
                right.term_ordinal,
            ))
        });
    }
    let mut changed = false;
    data.body_locations.retain_mut(|row| {
        let moved = relabel(&row.application, &mut row.occurrence);
        changed |= moved != Some(false);
        moved.is_some()
    });
    if changed {
        data.body_locations.sort_by(|left, right| {
            (&left.application, &left.occurrence, left.term_ordinal).cmp(&(
                &right.application,
                &right.occurrence,
                right.term_ordinal,
            ))
        });
    }
}
