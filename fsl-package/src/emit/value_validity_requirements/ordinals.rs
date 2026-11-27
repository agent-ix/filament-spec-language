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
    CheckedSourceMapEntry,
};
use filament_exact::NodeKey;
use filament_semantic_value::location::ExpressionOwner;
use fsl_semantics::check::wire_id::StructuralIds;
use fsl_semantics::check::CheckedGraph;
use ix_trace_rs::implements;

use super::{frames, EmitRefusal, Omitted};
use crate::emit::location_paths::Paths;

/// What the checker located a use in: the declaration index of the function
/// whose body or measure holds it, or somewhere no walk reaches.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum Use {
    Body(usize),
    Measure(usize),
    /// Anywhere else: a state clause, or the body of an owner the checked
    /// graph keeps no function declaration for (a reward function).
    Other,
}

/// The new ordinal of each checker ordinal of one node, ascending by checker
/// ordinal; `None` is a use that is dropped with its omitted owner.
pub(in crate::emit) type Mapping = Vec<(u64, Option<u64>)>;

/// One expression occurrence the reader's owner walk will consume.
#[derive(Clone, Copy)]
enum Slot {
    /// A source-located checker occurrence, moved to `ordinal`.
    Source { old: u64, ordinal: u64 },
    /// A transitive visit with no checker use at this package root. Its region
    /// is copied from the matching syntax owner's source-located occurrence.
    Synthetic { from: u64, ordinal: u64 },
}

#[derive(Default)]
struct NodeMapping {
    slots: Vec<Slot>,
}

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

    /// Relabel all term rows to the final owner-walk occurrence ordinals.
    #[implements("FSpec-FR-322-AC-490")]
    pub(in crate::emit) fn relabel_term_data(
        &self,
        ids: &HashMap<NodeKey, CheckedNodeId>,
        data: &mut filament_contract_model::CheckedTermDataV2,
    ) {
        let by_id = self.by_id(ids);
        relabel_term_data_inner(&by_id, data);
    }

    fn by_id(&self, ids: &HashMap<NodeKey, CheckedNodeId>) -> BTreeMap<CheckedNodeId, Mapping> {
        let mut by_id = BTreeMap::new();
        for (key, mapping) in &self.by_node {
            if let Some(id) = ids.get(key) {
                by_id.insert(id.clone(), mapping.clone());
            }
        }
        by_id
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
) -> Result<Renumbering, EmitRefusal> {
    // Static multi-use is not the only source of extra reads: a package-root
    // walk may enter a nested function body transitively. Walk every expression
    // node so those uses can be identified as well.
    let mut needed = multiply_used(graph);
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
        keep.push(
            node.occurrences
                .iter()
                .any(|occurrence| occurrence.role == CheckedOccurrenceRole::Expression),
        );
        node_keys.push(key);
    }
    // The visits each needed node receives, with both the package-root owner
    // and the function whose syntax the visit is inside.
    let walked = frames::visits(nodes, source_map, &keep);
    let mut visited = HashMap::<usize, Vec<(Use, bool, usize)>>::new();
    for visit in &walked {
        let function = function_index.get(&node_keys[visit.syntax_owner]).copied();
        let used = match function {
            Some(function) if visit.syntax_measure => Use::Measure(function),
            Some(function) => Use::Body(function),
            None => Use::Other,
        };
        visited.entry(visit.node).or_default().push((
            used,
            visit.owner == visit.syntax_owner && visit.measure == visit.syntax_measure,
            visit.syntax_ordinal,
        ));
    }
    for (&position, uses) in &visited {
        if uses.iter().any(|(_, root_owner, _)| !root_owner) {
            needed.insert(node_keys[position].clone());
        }
    }
    drop(walked);
    let by_key: HashSet<&NodeKey> = node_keys
        .iter()
        .zip(&keep)
        .filter(|(_, keep)| **keep)
        .map(|(key, _)| key)
        .collect();
    // The uses the checker located, each node's by owner: the interned path of
    // its position and its checker ordinal.
    let mut paths = Paths::default();
    let mut located = HashMap::<&NodeKey, HashMap<Use, Vec<(usize, u64)>>>::new();
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
            .push((paths.intern(location), origin.ordinal()));
    }
    // A path's rank is its place in the body's source order.
    let source_order = paths.lexicographic_ranks();
    let mut renumbering = Renumbering::default();
    let mut changed = BTreeMap::<usize, NodeMapping>::new();
    for (position, key) in node_keys.iter().enumerate() {
        let Some(groups) = located.get_mut(key) else {
            continue;
        };
        let mut mapping: Mapping = Vec::new();
        let mut kept: HashMap<Use, Vec<u64>> = HashMap::new();
        for (used, uses) in groups.iter_mut() {
            uses.sort_unstable_by_key(|&(path, old)| (source_order[path], old));
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
        let uses_in = visited.get(&position).map_or(&[][..], Vec::as_slice);
        let mut visit_positions = HashMap::<Use, Vec<usize>>::new();
        for (at, (used, root_owner, _)) in uses_in.iter().enumerate() {
            if *root_owner {
                visit_positions.entry(*used).or_default().push(at);
            }
        }
        // The next owner-root visits that still have located source uses.
        // Earlier transitive visits with no use in their root owner need a
        // synthetic slot if one of these future visits will consume a use.
        let mut future_uses = BTreeMap::<usize, Use>::new();
        for (used, old_uses) in &kept {
            if let Some(first) = visit_positions
                .get(used)
                .and_then(|positions| positions.first())
                .filter(|_| !old_uses.is_empty())
            {
                future_uses.insert(*first, *used);
            }
        }
        let mut next = HashMap::<Use, usize>::new();
        let mut slots = Vec::<Slot>::new();
        for (visit_at, (used, _root_owner, syntax_ordinal)) in uses_in.iter().enumerate() {
            if let Some(future_use) = future_uses.remove(&visit_at) {
                let at = next.entry(future_use).or_default();
                let old = kept[&future_use][*at];
                *at += 1;
                let ordinal = slots.len() as u64;
                slots.push(Slot::Source { old, ordinal });
                mapping.push((old, Some(ordinal)));
                if *at < kept[&future_use].len() {
                    if let Some(next_visit) = visit_positions[&future_use].get(*at) {
                        future_uses.insert(*next_visit, future_use);
                    }
                }
            } else if !future_uses.is_empty() {
                let from = if let Some(syntax_uses) = kept.get(used) {
                    syntax_uses
                        .get(*syntax_ordinal)
                        .or_else(|| (syntax_uses.len() == 1).then(|| &syntax_uses[0]))
                        .copied()
                } else if let Some((_, future_use)) = future_uses.first_key_value() {
                    let next_source = next.get(future_use).copied().unwrap_or_default();
                    kept.get(future_use)
                        .and_then(|uses| uses.get(next_source))
                        .copied()
                } else {
                    None
                }
                .ok_or(EmitRefusal::SourceMapInvariant)?;
                let ordinal = slots.len() as u64;
                slots.push(Slot::Synthetic { from, ordinal });
            }
        }
        // Source uses the reader's walk did not reach follow it in checker
        // order; this includes `Other` clause uses after every owner walk.
        let mut rest: Vec<u64> = kept
            .iter()
            .flat_map(|(used, uses)| {
                let consumed = next.get(used).copied().unwrap_or_default();
                uses.iter().skip(consumed).copied()
            })
            .collect();
        rest.sort_unstable();
        for old in rest {
            let ordinal = slots.len() as u64;
            slots.push(Slot::Source { old, ordinal });
            mapping.push((old, Some(ordinal)));
        }
        mapping.sort_unstable();
        let synthetic = slots
            .iter()
            .any(|slot| matches!(slot, Slot::Synthetic { .. }));
        if !synthetic && mapping.iter().all(|(old, new)| Some(*old) == *new) {
            continue;
        }
        renumbering.by_node.insert(key.clone(), mapping.clone());
        changed.insert(position, NodeMapping { slots });
    }
    apply(nodes, source_map, &changed)?;
    Ok(renumbering)
}

/// Write the new numbering into the occurrences of the nodes at the positions
/// `changed` names and rebuild their source-map entries. Synthetic slots clone
/// the regions of the syntax-owner occurrence they repeat.
fn apply(
    nodes: &mut [CheckedSemanticNodeV2],
    source_map: &mut Vec<CheckedSourceMapEntry>,
    changed: &BTreeMap<usize, NodeMapping>,
) -> Result<(), EmitRefusal> {
    if changed.is_empty() {
        return Ok(());
    }
    let mut affected = BTreeMap::<CheckedNodeId, usize>::new();
    for (&position, mapping) in changed {
        affected.insert(nodes[position].node_id.clone(), position);
        let node = &mut nodes[position];
        let count = mapping.slots.len() as u64;
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
    let mut source_entries = HashMap::<usize, HashMap<u64, CheckedSourceMapEntry>>::new();
    for entry in source_map.iter() {
        if entry.role != CheckedOccurrenceRole::Expression {
            continue;
        }
        if let Some(&position) = affected.get(&entry.node_id) {
            if source_entries
                .entry(position)
                .or_default()
                .insert(entry.ordinal, entry.clone())
                .is_some()
            {
                return Err(EmitRefusal::SourceMapInvariant);
            }
        }
    }
    let mut replacements = BTreeMap::<usize, Vec<CheckedSourceMapEntry>>::new();
    for (&position, mapping) in changed {
        let entries = source_entries.get(&position);
        let mut moved = Vec::with_capacity(mapping.slots.len());
        for slot in &mapping.slots {
            let (source_ordinal, new_ordinal) = match *slot {
                Slot::Source { old, ordinal } => (old, ordinal),
                Slot::Synthetic { from, ordinal } => (from, ordinal),
            };
            let mut entry = entries
                .and_then(|entries| entries.get(&source_ordinal))
                .cloned()
                .ok_or(EmitRefusal::SourceMapInvariant)?;
            entry.ordinal = new_ordinal;
            moved.push(entry);
        }
        replacements.insert(position, moved);
    }
    let mut inserted = HashSet::<usize>::new();
    let synthetic_count = changed
        .values()
        .flat_map(|mapping| &mapping.slots)
        .filter(|slot| matches!(slot, Slot::Synthetic { .. }))
        .count();
    let mut rebuilt = Vec::with_capacity(source_map.len().saturating_add(synthetic_count));
    for entry in source_map.iter() {
        if entry.role == CheckedOccurrenceRole::Expression {
            if let Some(&position) = affected.get(&entry.node_id) {
                if inserted.insert(position) {
                    rebuilt.extend(replacements[&position].iter().cloned());
                }
                continue;
            }
        }
        rebuilt.push(entry.clone());
    }
    if replacements
        .iter()
        .any(|(position, entries)| !entries.is_empty() && !inserted.contains(position))
    {
        return Err(EmitRefusal::SourceMapInvariant);
    }
    *source_map = rebuilt;
    Ok(())
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
    source_map: &mut Vec<CheckedSourceMapEntry>,
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
                .any(|occurrence| occurrence.role == CheckedOccurrenceRole::Expression)
        })
        .collect();
    let owners = |nodes: &[CheckedSemanticNodeV2],
                  source_map: &[CheckedSourceMapEntry]|
     -> HashMap<usize, Vec<WalkVisit>> {
        let mut owners = HashMap::<usize, Vec<WalkVisit>>::new();
        for visit in frames::visits(nodes, source_map, &keep) {
            owners.entry(visit.node).or_default().push(WalkVisit {
                owner: visit.owner,
                measure: visit.measure,
                lexical: (
                    visit.syntax_owner,
                    visit.syntax_measure,
                    visit.syntax_ordinal,
                ),
            });
        }
        owners
    };
    let before = owners(base_nodes, base_source_map);
    let after = owners(finals, source_map);
    let mut changed = BTreeMap::<usize, NodeMapping>::new();
    for (position, base) in &before {
        let Some(now) = after.get(position) else {
            return Err(carry("a rekey changed which nodes the owner walk reaches"));
        };
        if base.len() != now.len() {
            return Err(carry(
                "a rekey changed how often the owner walk visits a node",
            ));
        }
        // The j-th visit of an owner takes, in the new order, the place the
        // owner's j-th visit has there.
        let mut seen = HashMap::<(usize, bool), usize>::new();
        let mut places = HashMap::<(usize, bool, usize), usize>::new();
        for (new, visit) in now.iter().enumerate() {
            let nth = seen.entry((visit.owner, visit.measure)).or_default();
            places.insert((visit.owner, visit.measure, *nth), new);
            *nth += 1;
        }
        let mut seen = HashMap::<(usize, bool), usize>::new();
        let mut placed = Vec::with_capacity(base.len());
        for visit in base {
            let nth = seen.entry((visit.owner, visit.measure)).or_default();
            placed.push(
                *places
                    .get(&(visit.owner, visit.measure, *nth))
                    .ok_or(carry("a rekey changed an owner's visits of a node"))?,
            );
            *nth += 1;
        }
        let listed = base_nodes[*position]
            .occurrences
            .iter()
            .filter(|occurrence| occurrence.role == CheckedOccurrenceRole::Expression)
            .count();
        let lexical: Vec<_> = base.iter().map(|visit| visit.lexical).collect();
        let slots = rewalk_slots(listed, &lexical, &placed).ok_or(carry(
            "a rekey moved a visit whose lexical occurrence has no source region",
        ))?;
        let identity = slots.len() == listed
            && slots.iter().enumerate().all(|(at, slot)| {
                matches!(*slot, Slot::Source { old, ordinal } if old == at as u64 && ordinal == old)
            });
        if !identity {
            changed.insert(*position, NodeMapping { slots });
        }
    }
    let mut rebuilt_source_map = source_map.clone();
    apply(nodes, &mut rebuilt_source_map, &changed)?;
    *source_map = rebuilt_source_map;
    Ok(changed
        .into_iter()
        .map(|(position, mapping)| {
            let mut moved: Mapping = mapping
                .slots
                .iter()
                .filter_map(|slot| match *slot {
                    Slot::Source { old, ordinal } => Some((old, Some(ordinal))),
                    Slot::Synthetic { .. } => None,
                })
                .collect();
            moved.sort_unstable();
            (nodes[position].node_id.clone(), moved)
        })
        .collect())
}

/// One visit of the owner walk to a node, as a rekey re-reads it.
struct WalkVisit {
    owner: usize,
    measure: bool,
    /// The lexical occurrence the visit is inside: the function whose syntax
    /// it is, whether that is its measure, and its place among the node's
    /// visits in that syntax scope.
    lexical: (usize, bool, usize),
}

/// The slots of a node whose owner walk was reordered by a rekey. `listed` is
/// the number of expression occurrences the base lists; the first of them
/// belong, in order, to the base's visits (a visit's slot is a source use or a
/// synthetic copy of one) and any beyond the visits are source uses the walk
/// never reached. `lexical` names each base visit's lexical occurrence and
/// `placed[i]` the place base visit `i` takes in the new walk. A visit keeps
/// its occurrence and moves to its new place; a visit before the last moved
/// one that had none gets a synthetic slot copying the region of an
/// occurrence of the same lexical place. `None` when no such occurrence
/// exists.
fn rewalk_slots(
    listed: usize,
    lexical: &[(usize, bool, usize)],
    placed: &[usize],
) -> Option<Vec<Slot>> {
    let numbered = listed.min(placed.len());
    let end = placed[..numbered]
        .iter()
        .map(|place| place + 1)
        .max()
        .unwrap_or(0);
    let mut at_place = vec![None; end];
    for (visit, &place) in placed.iter().enumerate() {
        if place < end {
            at_place[place] = Some(visit);
        }
    }
    let mut region_of = HashMap::<(usize, bool, usize), u64>::new();
    for visit in 0..numbered {
        region_of.entry(lexical[visit]).or_insert(visit as u64);
    }
    let mut slots = Vec::with_capacity(end + listed - numbered);
    for place in 0..end {
        let ordinal = place as u64;
        let visit = at_place[place]?;
        slots.push(if visit < numbered {
            Slot::Source {
                old: visit as u64,
                ordinal,
            }
        } else {
            // A place no numbered visit holds borrows the next numbered
            // visit's occurrence, as `renumber` borrows the next future use.
            let from = region_of.get(&lexical[visit]).copied().or_else(|| {
                (place + 1..end)
                    .find_map(|later| at_place[later].filter(|&v| v < numbered))
                    .map(|v| v as u64)
            })?;
            Slot::Synthetic { from, ordinal }
        });
    }
    for old in numbered..listed {
        slots.push(Slot::Source {
            old: old as u64,
            ordinal: (end + old - numbered) as u64,
        });
    }
    Some(slots)
}

/// Move the checked-term rows that name a renumbered expression occurrence of
/// the nodes `by_id` holds, and drop the rows of a dropped use.
pub(in crate::emit) fn relabel_term_data(
    by_id: &BTreeMap<CheckedNodeId, Mapping>,
    data: &mut filament_contract_model::CheckedTermDataV2,
) {
    relabel_term_data_inner(by_id, data);
}

fn relabel_term_data_inner(
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

#[cfg(test)]
mod rewalk_tests {
    use super::{rewalk_slots, Slot};

    fn pairs(slots: &[Slot]) -> Vec<(bool, u64, u64)> {
        slots
            .iter()
            .map(|slot| match *slot {
                Slot::Source { old, ordinal } => (true, old, ordinal),
                Slot::Synthetic { from, ordinal } => (false, from, ordinal),
            })
            .collect()
    }

    /// QSL-3559: two visits listed one slot each swap places: each occurrence
    /// follows its visit and no synthetic slot appears.
    #[test]
    fn a_rekey_that_swaps_two_numbered_visits_moves_their_occurrences() {
        let lexical = [(0, false, 0), (1, false, 0)];
        let slots = rewalk_slots(2, &lexical, &[1, 0]).unwrap();
        assert_eq!(pairs(&slots), [(true, 1, 0), (true, 0, 1)]);
    }

    /// QSL-3559: the base numbers two of three visits (a third visit comes
    /// after the last source use, so it has no slot). A rekey moves that
    /// third visit first and the numbered ones behind it: the moved visit now
    /// precedes numbered visits, so it takes a synthetic slot that copies the
    /// region of the occurrence of its lexical place, and the numbered
    /// visits shift by one.
    #[test]
    fn a_rekey_moving_an_unnumbered_visit_first_gives_it_a_synthetic_slot() {
        let lexical = [(0, false, 0), (1, false, 0), (0, false, 0)];
        let slots = rewalk_slots(2, &lexical, &[1, 2, 0]).unwrap();
        assert_eq!(pairs(&slots), [(false, 0, 0), (true, 0, 1), (true, 1, 2)]);
    }

    /// A visit moved before the numbered ones whose lexical place no numbered
    /// visit holds borrows the next numbered visit's occurrence.
    #[test]
    fn an_unnumbered_visit_of_an_unseen_place_borrows_the_next_numbered_one() {
        let lexical = [(0, false, 0), (1, false, 0), (2, false, 0)];
        let slots = rewalk_slots(2, &lexical, &[1, 2, 0]).unwrap();
        assert_eq!(pairs(&slots), [(false, 0, 0), (true, 0, 1), (true, 1, 2)]);
    }
}
