// SPDX-License-Identifier: AGPL-3.0-or-later
//! FSpec FR-825 "Shared path-condition frames": the parent-linked condition
//! table the value-validity records cite by terminal frame id.
//!
//! The table is built once per package by one walk of each guarded owner's
//! body, in the reader's order: owner structural key ascending, then first
//! visit in the owner's body-reference order, depth first. Each distinct
//! `(parent, guard, occurrence, holds)` tuple is one row, so the table holds
//! one row per guard edge and a record stores only its terminal id. The
//! checker's per-record guard chain is compared with the chain a frame walk
//! yields; no record keeps an expanded guard vector.

use std::collections::{BTreeMap, HashMap, HashSet};

use filament_contract_model::{
    CheckedBindingNameKind, CheckedNodeId, CheckedOccurrenceRole, CheckedSemanticNodeV2,
    CheckedSourceMapEntry, CheckedSourceRef, CheckedTerm, LiteralKind, PathConditionFrame,
    PathConditionFrameId,
};
use filament_exact::NodeKey;
use fsl_foundation::source::provenance::OccurrenceKey;
use fsl_semantics::check::wire_id::StructuralIds;
use fsl_semantics::check::{Capability, CheckedGraph, RequirementRecord};

use super::{EmitRefusal, Omitted};
use crate::emit::node_id;

/// The shared table and each guarded record's terminal frame.
#[derive(Default)]
pub(super) struct Frames {
    /// The table in canonical order, holding only frames some record cites.
    pub(super) rows: Vec<PathConditionFrame>,
    /// The terminal frame of each written record with a non-empty path
    /// condition; an absent claim is unguarded.
    terminals: HashMap<OccurrenceKey, PathConditionFrameId>,
}

impl Frames {
    /// The terminal frame of the record at `claim`, `None` when unguarded.
    pub(super) fn terminal(&self, claim: &OccurrenceKey) -> Option<PathConditionFrameId> {
        self.terminals.get(claim).copied()
    }
}

/// One candidate row before unused rows are dropped.
struct Candidate {
    parent: Option<usize>,
    guard: usize,
    occurrence: u64,
    holds: bool,
}

#[derive(Default)]
struct Table {
    rows: Vec<Candidate>,
    lookup: HashMap<(Option<usize>, usize, u64, bool), usize>,
    /// Whether an owner's walk has touched the row yet.
    touched: Vec<bool>,
    /// The rows the current owner's walk touched first, in visit order.
    first_touched: Vec<usize>,
}

impl Table {
    fn intern(
        &mut self,
        parent: Option<usize>,
        guard: usize,
        occurrence: u64,
        holds: bool,
    ) -> usize {
        let row = *self
            .lookup
            .entry((parent, guard, occurrence, holds))
            .or_insert_with(|| {
                self.rows.push(Candidate {
                    parent,
                    guard,
                    occurrence,
                    holds,
                });
                self.touched.push(false);
                self.rows.len() - 1
            });
        if !std::mem::replace(&mut self.touched[row], true) {
            self.first_touched.push(row);
        }
        row
    }
}

enum Task<'a> {
    /// Leave the node at the position: its body is fully walked.
    Leave(usize),
    Node(&'a CheckedNodeId, Option<usize>),
    Term(&'a CheckedTerm, Option<usize>),
}

fn reference(term: &CheckedTerm) -> Option<&CheckedNodeId> {
    match term {
        CheckedTerm::Reference { target } => Some(target),
        CheckedTerm::DependencyReference { node, .. } => Some(node),
        _ => None,
    }
}

fn function_body_root(term: &CheckedTerm) -> Option<&CheckedNodeId> {
    let CheckedTerm::Aggregate { members, .. } = term else {
        return None;
    };
    members.iter().find_map(|member| {
        let CheckedTerm::Binding { name, value, .. } = member else {
            return None;
        };
        if !matches!(name.kind(), CheckedBindingNameKind::Body) {
            return None;
        }
        match value.as_ref() {
            CheckedTerm::Reference { target } => Some(target),
            _ => None,
        }
    })
}

/// The termination measure's root, whose walk takes expression occurrences but
/// derives no frame.
fn function_measure_root(term: &CheckedTerm) -> Option<&CheckedNodeId> {
    let CheckedTerm::Aggregate { members, .. } = term else {
        return None;
    };
    members.iter().find_map(|member| {
        let CheckedTerm::Binding { name, value, .. } = member else {
            return None;
        };
        if !matches!(
            name.kind(),
            CheckedBindingNameKind::Other("decreases" | "measure")
        ) {
            return None;
        }
        match value.as_ref() {
            CheckedTerm::Reference { target } => Some(target),
            _ => None,
        }
    })
}

/// The walk of one owner's body over the emitted graph.
struct Walk<'a> {
    nodes: &'a [CheckedSemanticNodeV2],
    index: &'a BTreeMap<&'a CheckedNodeId, usize>,
}

impl<'a> Walk<'a> {
    /// The `nth` expression occurrence ordinal of the node at `position`.
    fn expression_ordinal(&self, position: usize, nth: usize) -> Option<u64> {
        self.nodes[position]
            .occurrences
            .iter()
            .filter(|occurrence| occurrence.role == CheckedOccurrenceRole::Expression)
            .nth(nth)
            .map(|occurrence| occurrence.ordinal)
    }

    /// The Boolean literal a guard reference names, if it names one.
    fn literal(&self, term: &CheckedTerm) -> Option<bool> {
        let position = *self.index.get(reference(term)?)?;
        let node = &self.nodes[position];
        if node.node_tag.as_ref() != "value" {
            return None;
        }
        match &node.body {
            CheckedTerm::Literal {
                value_kind: LiteralKind::Boolean,
                value,
                ..
            } => value.as_bool(),
            _ => None,
        }
    }

    /// The guard of `term` as its node position and the expression occurrence
    /// the walk reads next at it.
    fn guard(&self, term: &CheckedTerm, visits: &HashMap<usize, usize>) -> Option<(usize, u64)> {
        let position = *self.index.get(reference(term)?)?;
        let ordinal = self.expression_ordinal(position, *visits.get(&position).unwrap_or(&0))?;
        Some((position, ordinal))
    }

    /// Walk the body rooted at `root`: the terminal frame of every
    /// application occurrence, keyed by node position and occurrence.
    fn owner(
        &self,
        root: &'a CheckedNodeId,
        derive: bool,
        visits: &mut HashMap<usize, usize>,
        table: &mut Table,
    ) -> HashMap<(usize, u64), Option<usize>> {
        let mut derived = HashMap::new();
        let mut pending = vec![Task::Node(root, None)];
        // The nodes whose bodies the walk is inside. A reference back to one is
        // a recursive call: the walk does not enter it again, so a recursive
        // owner is walked once, not without end.
        let mut active = HashSet::<usize>::new();
        while let Some(task) = pending.pop() {
            match task {
                Task::Leave(position) => {
                    active.remove(&position);
                }
                Task::Node(id, terminal) => {
                    let Some(&position) = self.index.get(id) else {
                        continue;
                    };
                    if !active.insert(position) {
                        continue;
                    }
                    let visit = visits.entry(position).or_default();
                    let occurrence = self.expression_ordinal(position, *visit);
                    *visit += 1;
                    let node = &self.nodes[position];
                    if let (true, Some(ordinal), CheckedTerm::Application { .. }) =
                        (derive, occurrence, &node.body)
                    {
                        derived.insert((position, ordinal), terminal);
                    }
                    pending.push(Task::Leave(position));
                    pending.push(Task::Term(&node.body, terminal));
                }
                Task::Term(term, terminal) => match term {
                    CheckedTerm::Reference { target } => {
                        pending.push(Task::Node(target, terminal));
                    }
                    CheckedTerm::Application {
                        operation,
                        arguments,
                        ..
                    } => {
                        let identity = operation.identity();
                        if derive
                            && identity == Some("filament.op.control.if")
                            && arguments.len() == 3
                        {
                            let Some((guard, occurrence)) = self.guard(&arguments[0], visits)
                            else {
                                for argument in arguments.iter().rev() {
                                    pending.push(Task::Term(argument, terminal));
                                }
                                continue;
                            };
                            match self.literal(&arguments[0]) {
                                Some(true) => {
                                    let then = table.intern(terminal, guard, occurrence, true);
                                    pending.push(Task::Term(&arguments[1], Some(then)));
                                }
                                Some(false) => {
                                    let other = table.intern(terminal, guard, occurrence, false);
                                    pending.push(Task::Term(&arguments[2], Some(other)));
                                }
                                None => {
                                    let then = table.intern(terminal, guard, occurrence, true);
                                    let other = table.intern(terminal, guard, occurrence, false);
                                    pending.push(Task::Term(&arguments[2], Some(other)));
                                    pending.push(Task::Term(&arguments[1], Some(then)));
                                }
                            }
                            pending.push(Task::Term(&arguments[0], terminal));
                        } else if derive
                            && matches!(
                                identity,
                                Some(
                                    "filament.op.boolean.and"
                                        | "filament.op.boolean.or"
                                        | "filament.op.boolean.implies"
                                )
                            )
                            && arguments.len() == 2
                        {
                            let holds = identity != Some("filament.op.boolean.or");
                            let literal = self.literal(&arguments[0]);
                            let short_circuited = match identity {
                                Some("filament.op.boolean.and" | "filament.op.boolean.implies") => {
                                    literal == Some(false)
                                }
                                Some("filament.op.boolean.or") => literal == Some(true),
                                _ => false,
                            };
                            let right = if short_circuited {
                                terminal
                            } else if let Some((guard, occurrence)) =
                                self.guard(&arguments[0], visits)
                            {
                                Some(table.intern(terminal, guard, occurrence, holds))
                            } else {
                                terminal
                            };
                            if !short_circuited {
                                pending.push(Task::Term(&arguments[1], right));
                            }
                            pending.push(Task::Term(&arguments[0], terminal));
                        } else {
                            for argument in arguments.iter().rev() {
                                pending.push(Task::Term(argument, terminal));
                            }
                        }
                    }
                    CheckedTerm::Aggregate { members, .. } => {
                        for member in members.iter().rev() {
                            pending.push(Task::Term(member, terminal));
                        }
                    }
                    CheckedTerm::Binding { value, .. } => {
                        pending.push(Task::Term(value, terminal));
                    }
                    _ => {}
                },
            }
        }
        derived
    }
}

/// The shared table of `graph`'s written value-validity records and each
/// guarded record's terminal frame, derived from the emitted owner bodies in
/// `nodes` and compared with the checker's own guard chain.
pub(super) fn derive(
    ids: &StructuralIds,
    graph: &CheckedGraph,
    nodes: &[CheckedSemanticNodeV2],
    source_map: &[CheckedSourceMapEntry],
    omitted: &Omitted,
) -> Result<Frames, EmitRefusal> {
    // Written owners holding a guarded record, in structural-key order.
    let mut owners = BTreeMap::<NodeKey, Vec<(&OccurrenceKey, &RequirementRecord)>>::new();
    let mut identities = HashMap::<&str, NodeKey>::new();
    for (claim, source) in graph.requirements().iter() {
        if source.requirements().kind() != Capability::ValueValidity
            || source.path_condition().is_empty()
        {
            continue;
        }
        let name = source.owner().ok_or(EmitRefusal::SourceMapInvariant)?;
        let key = match identities.get(name) {
            Some(key) => key.clone(),
            None => {
                let key = graph
                    .function_identity(name)
                    .ok_or(EmitRefusal::SourceMapInvariant)?;
                identities.insert(name, key.clone());
                key
            }
        };
        if !omitted.contains_key(&key) {
            owners.entry(key).or_default().push((claim, source));
        }
    }
    if owners.is_empty() {
        return Ok(Frames::default());
    }
    let index: BTreeMap<&CheckedNodeId, usize> = nodes
        .iter()
        .enumerate()
        .map(|(position, node)| (&node.node_id, position))
        .collect();
    let walk = Walk {
        nodes,
        index: &index,
    };
    // The reader walks every function owner, in FR-322's package-wide order
    // (declaration source position, then node id), with each node's
    // expression-occurrence counter shared by all owners (FSpec FR-825): an
    // owner without a guarded record still takes the occurrences it reads.
    let mut declared = BTreeMap::<&CheckedNodeId, (&CheckedSourceRef, u64, u64)>::new();
    for entry in source_map {
        if entry.role != CheckedOccurrenceRole::Declaration {
            continue;
        }
        let Some(region) = entry.regions.first() else {
            continue;
        };
        let start = (&region.source, region.start, region.end);
        let known = declared.entry(&entry.node_id).or_insert(start);
        if start < *known {
            *known = start;
        }
    }
    let mut walked: Vec<usize> = nodes
        .iter()
        .enumerate()
        .filter(|(_, node)| {
            node.node_tag.as_ref() == "function" && function_body_root(&node.body).is_some()
        })
        .map(|(position, _)| position)
        .collect();
    walked.sort_by(|&left, &right| {
        declared
            .get(&nodes[left].node_id)
            .cmp(&declared.get(&nodes[right].node_id))
            .then_with(|| nodes[left].node_id.cmp(&nodes[right].node_id))
    });
    let records_of: BTreeMap<CheckedNodeId, &Vec<(&OccurrenceKey, &RequirementRecord)>> = owners
        .iter()
        .map(|(owner, records)| (node_id(ids, owner), records))
        .collect();
    for owner_id in records_of.keys() {
        // A guarded owner is a written function with a body.
        let position = *index.get(owner_id).ok_or(EmitRefusal::SourceMapInvariant)?;
        function_body_root(&nodes[position].body).ok_or(EmitRefusal::SourceMapInvariant)?;
    }
    let mut table = Table::default();
    let mut visits = HashMap::<usize, usize>::new();
    let mut terminals = Vec::<(&OccurrenceKey, &RequirementRecord, usize)>::new();
    // Each owner's first-touched frame rows, in its own visit order.
    let mut touched_by = Vec::<(&CheckedNodeId, Vec<usize>)>::new();
    for position in walked {
        let owner_id = &nodes[position].node_id;
        let root =
            function_body_root(&nodes[position].body).ok_or(EmitRefusal::SourceMapInvariant)?;
        let derived = walk.owner(root, true, &mut visits, &mut table);
        // The termination measure takes occurrences but derives no frame.
        if let Some(measure) = function_measure_root(&nodes[position].body) {
            walk.owner(measure, false, &mut visits, &mut table);
        }
        touched_by.push((owner_id, std::mem::take(&mut table.first_touched)));
        let Some(records) = records_of.get(owner_id) else {
            continue;
        };
        for &(claim, source) in records.iter() {
            let application = *index
                .get(&node_id(ids, claim.node()))
                .ok_or(EmitRefusal::SourceMapInvariant)?;
            let terminal = derived
                .get(&(application, claim.origin().ordinal()))
                .copied()
                .flatten()
                .ok_or(EmitRefusal::SourceMapInvariant)?;
            terminals.push((claim, source, terminal));
        }
    }
    // The checker's chain and the frame chain name the same guard nodes with
    // the same outcomes, in order. The frame's occurrence is the one the
    // reader derives from the body walk (the guard node's next `expression`
    // occurrence in visit order, FSpec FR-825), so it is not compared with the
    // checker's own occurrence ordinal, which may name a different read of a
    // parameter whose declaration is also an `expression` occurrence.
    for &(_, source, terminal) in &terminals {
        let mut at = Some(terminal);
        for guard in source.path_condition().iter().rev() {
            let frame = &table.rows[at.ok_or(EmitRefusal::SourceMapInvariant)?];
            let same = nodes[frame.guard].node_id == node_id(ids, guard.occurrence().node())
                && frame.holds == guard.holds();
            if !same {
                return Err(EmitRefusal::SourceMapInvariant);
            }
            at = frame.parent;
        }
        if at.is_some() {
            return Err(EmitRefusal::SourceMapInvariant);
        }
    }
    // Keep the frames some record's chain holds, in canonical order.
    let mut used = vec![false; table.rows.len()];
    for &(_, _, terminal) in &terminals {
        let mut at = Some(terminal);
        while let Some(frame) = at {
            if std::mem::replace(&mut used[frame], true) {
                break;
            }
            at = table.rows[frame].parent;
        }
    }
    // The table is in the reader's canonical order: owner node id ascending,
    // then the first visit within that owner of each frame no earlier owner
    // touched (FSpec FR-825).
    touched_by.sort_by(|left, right| left.0.cmp(right.0));
    let mut renumbered = vec![0_usize; table.rows.len()];
    let mut placed = vec![false; table.rows.len()];
    let mut rows = Vec::new();
    for old in touched_by.into_iter().flat_map(|(_, touched)| touched) {
        let frame = &table.rows[old];
        if !used[old] {
            continue;
        }
        if frame.parent.is_some_and(|parent| !placed[parent]) {
            return Err(EmitRefusal::SourceMapInvariant);
        }
        placed[old] = true;
        renumbered[old] = rows.len();
        rows.push(PathConditionFrame {
            parent: frame
                .parent
                .map(|parent| PathConditionFrameId(renumbered[parent])),
            guard: nodes[frame.guard].node_id.clone(),
            occurrence: frame.occurrence,
            holds: frame.holds,
        });
    }
    Ok(Frames {
        rows,
        terminals: terminals
            .into_iter()
            .map(|(claim, _, terminal)| (claim.clone(), PathConditionFrameId(renumbered[terminal])))
            .collect(),
    })
}
