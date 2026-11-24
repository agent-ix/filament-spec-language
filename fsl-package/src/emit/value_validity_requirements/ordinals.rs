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
//! ordinal k: a visit in an owner takes the use the checker located in that
//! owner, uses no walk reaches (state-clause bodies) follow the visits, and the
//! uses of an omitted owner are dropped from the nodes that are kept. The
//! source-map regions follow their uses.

use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};

use filament_contract_model::{
    CheckedOccurrence, CheckedOccurrenceRole, CheckedSemanticNodeV2, CheckedSourceMapEntry,
};
use filament_exact::NodeKey;
use filament_semantic_value::location::ExpressionOwner;
use fsl_semantics::check::wire_id::StructuralIds;
use fsl_semantics::check::CheckedGraph;

use super::{frames, EmitRefusal, Omitted};

/// What the checker located a use in.
#[derive(Clone, PartialEq, Eq, Hash)]
enum Use {
    Body(String),
    Measure(String),
    Other,
}

/// The new ordinal of each renumbered expression occurrence, by node.
#[derive(Default)]
pub(in crate::emit) struct Renumbering {
    /// Ascending by checker ordinal; a dropped use has no entry.
    by_node: HashMap<NodeKey, Vec<(u64, u64)>>,
}

impl Renumbering {
    /// The ordinal the package gives the checker's expression occurrence
    /// `old` of `node`.
    pub(super) fn ordinal(&self, node: &NodeKey, old: u64) -> u64 {
        self.by_node
            .get(node)
            .and_then(|entries| {
                entries
                    .binary_search_by_key(&old, |(old, _)| *old)
                    .ok()
                    .map(|at| entries[at].1)
            })
            .unwrap_or(old)
    }
}

/// Renumber the expression occurrences of `nodes` and `source_map` by the
/// package-wide owner walk.
pub(super) fn renumber(
    ids: &StructuralIds,
    graph: &CheckedGraph,
    nodes: &mut [CheckedSemanticNodeV2],
    source_map: &mut Vec<CheckedSourceMapEntry>,
    omitted: &Omitted,
) -> Result<Renumbering, EmitRefusal> {
    let invariant = |_| EmitRefusal::SourceMapInvariant;
    let mut owner_names = HashMap::<NodeKey, String>::new();
    let mut omitted_names = HashSet::<String>::new();
    let mut at = 0;
    while let Some(state) = graph.function_state(at) {
        if let Some(identity) = graph.function_identity_at(at) {
            if omitted.contains_key(&identity) {
                omitted_names.insert(state.name.to_owned());
            }
            owner_names.insert(identity, state.name.to_owned());
        }
        at += 1;
    }
    let mut keys = ids.key_reader();
    let mut node_keys = Vec::with_capacity(nodes.len());
    for node in nodes.iter() {
        node_keys.push(keys.key(&node.node_id).map_err(invariant)?);
    }
    // The visits each node receives, by the use of the owner that made them.
    let mut visited = HashMap::<usize, Vec<Option<Use>>>::new();
    for visit in frames::visits(nodes, source_map) {
        let name = owner_names.get(&node_keys[visit.owner]).cloned();
        visited
            .entry(visit.node)
            .or_default()
            .push(name.map(|name| {
                if visit.measure {
                    Use::Measure(name)
                } else {
                    Use::Body(name)
                }
            }));
    }
    let mut located = HashMap::<&NodeKey, Vec<(u64, Use)>>::new();
    let wanted: HashSet<&NodeKey> = node_keys.iter().collect();
    let mut recorded = Vec::new();
    for (key, origin, location) in graph.occurrences() {
        if !crate::wire_form::is_expression_role(origin.role().as_str()) {
            continue;
        }
        let Some(&key) = wanted.get(&key) else {
            continue;
        };
        let used = match &location.origin {
            ExpressionOwner::Body { function, .. } => Use::Body(function.clone()),
            ExpressionOwner::Measure { function, .. } => Use::Measure(function.clone()),
            _ => Use::Other,
        };
        recorded.push((key, origin.ordinal(), used));
    }
    for (key, ordinal, used) in recorded {
        located.entry(key).or_default().push((ordinal, used));
    }
    let mut renumbering = Renumbering::default();
    let mut changed = HashMap::<usize, (usize, Vec<(u64, u64)>)>::new();
    for (position, key) in node_keys.iter().enumerate() {
        let Some(old) = located.get_mut(key) else {
            continue;
        };
        old.sort_by_key(|(ordinal, _)| *ordinal);
        let kept: Vec<&(u64, Use)> = old
            .iter()
            .filter(|(_, used)| match used {
                Use::Body(name) | Use::Measure(name) => !omitted_names.contains(name),
                Use::Other => true,
            })
            .collect();
        // A node only an omitted owner uses keeps the occurrences it has.
        if kept.is_empty() {
            continue;
        }
        let mut pool = HashMap::<&Use, VecDeque<u64>>::new();
        for (ordinal, used) in &kept {
            pool.entry(used).or_default().push_back(*ordinal);
        }
        let visits = visited.get(&position).map_or(&[][..], Vec::as_slice);
        let matched: Vec<Option<u64>> = visits
            .iter()
            .map(|used| {
                used.as_ref()
                    .and_then(|used| pool.get_mut(used))
                    .and_then(VecDeque::pop_front)
            })
            .collect();
        let taken: HashSet<u64> = matched.iter().flatten().copied().collect();
        let mut leftovers: VecDeque<u64> = kept
            .iter()
            .map(|(ordinal, _)| *ordinal)
            .filter(|ordinal| !taken.contains(ordinal))
            .collect();
        let mut sequence = Vec::with_capacity(kept.len());
        for slot in matched {
            match slot.or_else(|| leftovers.pop_front()) {
                Some(ordinal) => sequence.push(ordinal),
                None => {}
            }
        }
        sequence.extend(leftovers);
        let mut mapping: Vec<(u64, u64)> = sequence
            .iter()
            .enumerate()
            .map(|(new, old)| (*old, new as u64))
            .collect();
        mapping.sort_unstable();
        let identity = mapping.len() == old.len() && mapping.iter().all(|(old, new)| old == new);
        if identity {
            continue;
        }
        renumbering.by_node.insert(key.clone(), mapping.clone());
        changed.insert(position, (old.len(), mapping));
    }
    if changed.is_empty() {
        return Ok(renumbering);
    }
    let mut affected = BTreeMap::<_, usize>::new();
    for (&position, _) in &changed {
        affected.insert(nodes[position].node_id.clone(), position);
    }
    for (&position, (_, mapping)) in &changed {
        let node = &mut nodes[position];
        let count = mapping.len() as u64;
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
    let mut removed = HashSet::<usize>::new();
    for (position, entry_positions) in slots {
        let mapping = &changed[&position].1;
        let mut moved: Vec<CheckedSourceMapEntry> = Vec::new();
        for &entry_position in &entry_positions {
            let entry = &source_map[entry_position];
            match mapping.binary_search_by_key(&entry.ordinal, |(old, _)| *old) {
                Ok(at) => {
                    let mut entry = entry.clone();
                    entry.ordinal = mapping[at].1;
                    moved.push(entry);
                }
                Err(_) => {}
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

impl Renumbering {
    /// Move every checked-term row naming a renumbered expression occurrence
    /// to the occurrence's new ordinal, and put each changed table back in
    /// its canonical order. `ids` names each node's id in the rows.
    pub(in crate::emit) fn relabel_term_data(
        &self,
        ids: &HashMap<NodeKey, filament_contract_model::CheckedNodeId>,
        data: &mut filament_contract_model::CheckedTermDataV2,
    ) {
        if self.by_node.is_empty() {
            return;
        }
        let mut by_id = BTreeMap::new();
        for (key, entries) in &self.by_node {
            if let Some(id) = ids.get(key) {
                by_id.insert(id, entries);
            }
        }
        let relabel = |application: &filament_contract_model::CheckedNodeId,
                       occurrence: &mut CheckedOccurrence|
         -> bool {
            if occurrence.role != CheckedOccurrenceRole::Expression {
                return false;
            }
            let Some(entries) = by_id.get(application) else {
                return false;
            };
            let Ok(at) = entries.binary_search_by_key(&occurrence.ordinal, |(old, _)| *old) else {
                return false;
            };
            let new = entries[at].1;
            let changed = new != occurrence.ordinal;
            occurrence.ordinal = new;
            changed
        };
        let mut changed = false;
        for row in &mut data.equality_decisions {
            changed |= relabel(&row.application, &mut row.occurrence);
        }
        if changed {
            data.equality_decisions
                .sort_by(|left, right| (&left.application, &left.occurrence).cmp(&(&right.application, &right.occurrence)));
        }
        let mut changed = false;
        for row in &mut data.dispatch_tables {
            changed |= relabel(&row.application, &mut row.occurrence);
        }
        if changed {
            data.dispatch_tables
                .sort_by(|left, right| (&left.application, &left.occurrence).cmp(&(&right.application, &right.occurrence)));
        }
        let mut changed = false;
        for row in &mut data.protocol_locations {
            changed |= relabel(&row.application, &mut row.occurrence);
        }
        if changed {
            data.protocol_locations.sort_by(|left, right| {
                (&left.application, &left.occurrence, left.term_ordinal)
                    .cmp(&(&right.application, &right.occurrence, right.term_ordinal))
            });
        }
        let mut changed = false;
        for row in &mut data.body_locations {
            changed |= relabel(&row.application, &mut row.occurrence);
        }
        if changed {
            data.body_locations.sort_by(|left, right| {
                (&left.application, &left.occurrence, left.term_ordinal)
                    .cmp(&(&right.application, &right.occurrence, right.term_ordinal))
            });
        }
    }
}

impl Renumbering {
    /// Whether any expression occurrence took another ordinal.
    pub(in crate::emit) fn renumbers(&self) -> bool {
        !self.by_node.is_empty()
    }
}
