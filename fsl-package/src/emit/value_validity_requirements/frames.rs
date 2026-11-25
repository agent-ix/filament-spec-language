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
use fsl_semantics::check::wire_id::{StructuralIds, StructuralKeys, WireGraph};
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
    /// The node, syntax owner, scope and per-node syntax ordinal of each visit.
    visited: Vec<(usize, usize, bool, usize)>,
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

    /// Finish one owner's walk and make first-touch tracking owner-local.
    fn take_owner_touches(&mut self) -> Vec<usize> {
        let touched = std::mem::take(&mut self.first_touched);
        for &row in &touched {
            self.touched[row] = false;
        }
        touched
    }
}

#[derive(Clone, Copy)]
struct SyntaxContext {
    owner: usize,
    measure: bool,
    scope: usize,
}

enum Task<'a> {
    /// Leave the node at the position: its body is fully walked.
    Leave(usize),
    Node(&'a CheckedNodeId, Option<usize>, SyntaxContext),
    Term(&'a CheckedTerm, Option<usize>, SyntaxContext),
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

pub(super) enum KeyResolver<'a> {
    Reader(&'a mut StructuralKeys),
    Graph(&'a WireGraph),
}

impl KeyResolver<'_> {
    fn key(&mut self, id: &CheckedNodeId) -> Option<NodeKey> {
        match self {
            Self::Reader(reader) => reader.key(id).ok(),
            Self::Graph(graph) => graph.key(id).ok(),
        }
    }
}

/// The walk of one owner's body over the emitted graph.
struct Walk<'a, 'k> {
    nodes: &'a [CheckedSemanticNodeV2],
    index: &'a BTreeMap<&'a CheckedNodeId, usize>,
    key_index: Option<&'k HashMap<NodeKey, usize>>,
    resolver: Option<KeyResolver<'k>>,
    /// Resolve each immutable reference term once. The same term is visited
    /// from multiple package owners, and structural-ID comparison can walk a
    /// deep path; keying the cache by the term's stable borrowed ID address
    /// avoids repeating that comparison on every owner walk.
    resolved: HashMap<*const CheckedNodeId, usize>,
}

impl<'a, 'k> Walk<'a, 'k> {
    fn position(&mut self, id: &CheckedNodeId) -> Option<usize> {
        let pointer = std::ptr::from_ref(id);
        if let Some(&position) = self.resolved.get(&pointer) {
            return Some(position);
        }
        if let (Some(resolver), Some(key_index)) = (&mut self.resolver, self.key_index) {
            if let Some(position) = resolver
                .key(id)
                .and_then(|key| key_index.get(&key).copied())
            {
                self.resolved.insert(pointer, position);
                return Some(position);
            }
        }
        let position = *self.index.get(id)?;
        self.resolved.insert(pointer, position);
        Some(position)
    }

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
    fn literal(&mut self, term: &CheckedTerm) -> Option<bool> {
        let position = self.position(reference(term)?)?;
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
    fn guard(
        &mut self,
        term: &CheckedTerm,
        visits: &HashMap<usize, usize>,
    ) -> Option<(usize, u64)> {
        let position = self.position(reference(term)?)?;
        let ordinal = self.expression_ordinal(position, *visits.get(&position).unwrap_or(&0))?;
        Some((position, ordinal))
    }

    /// Walk the body rooted at `root`: the terminal frame of every
    /// application occurrence, keyed by node position and occurrence.
    fn owner(
        &mut self,
        root_owner: usize,
        root: &'a CheckedNodeId,
        derive: bool,
        syntax_measure: bool,
        visits: &mut HashMap<usize, usize>,
        table: &mut Table,
    ) -> HashMap<(usize, u64), Option<usize>> {
        let mut derived = HashMap::new();
        let mut pending = vec![Task::Node(
            root,
            None,
            SyntaxContext {
                owner: root_owner,
                measure: syntax_measure,
                scope: 0,
            },
        )];
        let mut next_scope = 1_usize;
        let mut syntax_visits = HashMap::<(usize, usize), usize>::new();
        // The nodes whose bodies the walk is inside. A reference back to one is
        // a recursive call: the walk does not enter it again, so a recursive
        // owner is walked once, not without end.
        let mut active = HashSet::<usize>::new();
        while let Some(task) = pending.pop() {
            match task {
                Task::Leave(position) => {
                    active.remove(&position);
                }
                Task::Node(id, terminal, mut syntax) => {
                    let Some(position) = self.position(id) else {
                        continue;
                    };
                    if !active.insert(position) {
                        continue;
                    }
                    // A call reaches the callee's declaration node. Follow its
                    // body so transitive syntax uses take their package
                    // ordinals, but reset the syntax owner and scope so those
                    // uses retain the callee's lexical source region. The
                    // active set stops recursive call cycles; the package-wide
                    // owner loop still derives each function's records under
                    // that function's own owner.
                    if self.nodes[position].node_tag.as_ref() == "function" {
                        syntax.owner = position;
                        syntax.measure = false;
                        syntax.scope = next_scope;
                        next_scope = next_scope.saturating_add(1);
                    }
                    let syntax_counter = syntax_visits.entry((syntax.scope, position)).or_default();
                    let syntax_ordinal = *syntax_counter;
                    *syntax_counter += 1;
                    let visit = visits.entry(position).or_default();
                    table
                        .visited
                        .push((position, syntax.owner, syntax.measure, syntax_ordinal));
                    let occurrence = self.expression_ordinal(position, *visit);
                    *visit += 1;
                    let node = &self.nodes[position];
                    if let (true, Some(expression_ordinal), CheckedTerm::Application { .. }) =
                        (derive, occurrence, &node.body)
                    {
                        derived.insert((position, expression_ordinal), terminal);
                    }
                    pending.push(Task::Leave(position));
                    pending.push(Task::Term(&node.body, terminal, syntax));
                }
                Task::Term(term, terminal, syntax) => match term {
                    CheckedTerm::Reference { target } => {
                        pending.push(Task::Node(target, terminal, syntax));
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
                                    pending.push(Task::Term(argument, terminal, syntax));
                                }
                                continue;
                            };
                            match self.literal(&arguments[0]) {
                                Some(true) => {
                                    let then = table.intern(terminal, guard, occurrence, true);
                                    pending.push(Task::Term(&arguments[1], Some(then), syntax));
                                }
                                Some(false) => {
                                    let other = table.intern(terminal, guard, occurrence, false);
                                    pending.push(Task::Term(&arguments[2], Some(other), syntax));
                                }
                                None => {
                                    let then = table.intern(terminal, guard, occurrence, true);
                                    let other = table.intern(terminal, guard, occurrence, false);
                                    pending.push(Task::Term(&arguments[2], Some(other), syntax));
                                    pending.push(Task::Term(&arguments[1], Some(then), syntax));
                                }
                            }
                            pending.push(Task::Term(&arguments[0], terminal, syntax));
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
                                pending.push(Task::Term(&arguments[1], right, syntax));
                            }
                            pending.push(Task::Term(&arguments[0], terminal, syntax));
                        } else {
                            for argument in arguments.iter().rev() {
                                pending.push(Task::Term(argument, terminal, syntax));
                            }
                        }
                    }
                    CheckedTerm::Aggregate { members, .. } => {
                        for member in members.iter().rev() {
                            pending.push(Task::Term(member, terminal, syntax));
                        }
                    }
                    CheckedTerm::Binding { name, value, .. } => {
                        let measure = match name.kind() {
                            CheckedBindingNameKind::Body => false,
                            CheckedBindingNameKind::Other("decreases" | "measure") => true,
                            _ => syntax.measure,
                        };
                        pending.push(Task::Term(
                            value,
                            terminal,
                            SyntaxContext { measure, ..syntax },
                        ));
                    }
                    _ => {}
                },
            }
        }
        derived
    }
}

/// The function owners with a body in FR-322's package-wide walk order:
/// declaration source position, then node id.
pub(super) fn owner_order(
    nodes: &[CheckedSemanticNodeV2],
    source_map: &[CheckedSourceMapEntry],
) -> Vec<usize> {
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
    walked
}

/// One visit of the package-wide owner walk (FSpec FR-322 "Expression
/// occurrence ordinals"): the root function whose walk made this visit, the
/// function whose syntax is visited, their measure/body contexts, and the node.
pub(super) struct Visit {
    /// The root function whose package-wide walk made this visit.
    pub(super) owner: usize,
    /// The function body that lexically owns the visited syntax.
    pub(super) syntax_owner: usize,
    /// Whether the visited syntax belongs to that owner's measure.
    pub(super) syntax_measure: bool,
    /// The ordinal of this node among visits in the current syntax scope.
    pub(super) syntax_ordinal: usize,
    pub(super) measure: bool,
    pub(super) node: usize,
}

pub(super) fn key_index(node_keys: &[NodeKey]) -> HashMap<NodeKey, usize> {
    node_keys
        .iter()
        .cloned()
        .enumerate()
        .map(|(position, key)| (key, position))
        .collect()
}

/// Every visit of the package-wide owner walk over `nodes`, in walk order: the
/// function owners in FR-322's order, each owner's body and then its
/// termination measure, with the guard-literal pruning of FSpec FR-825.
pub(super) fn visits(
    nodes: &[CheckedSemanticNodeV2],
    source_map: &[CheckedSourceMapEntry],
    keep: &[bool],
    resolver: Option<KeyResolver<'_>>,
    keys: Option<HashMap<NodeKey, usize>>,
) -> Vec<Visit> {
    let index: BTreeMap<&CheckedNodeId, usize> = nodes
        .iter()
        .enumerate()
        .map(|(position, node)| (&node.node_id, position))
        .collect();
    let mut walk = Walk {
        nodes,
        index: &index,
        key_index: keys.as_ref(),
        resolver,
        resolved: HashMap::new(),
    };
    let mut table = Table::default();
    let mut counters = HashMap::<usize, usize>::new();
    let mut found = Vec::new();
    for owner in owner_order(nodes, source_map) {
        let Some(root) = function_body_root(&nodes[owner].body) else {
            continue;
        };
        walk.owner(owner, root, true, false, &mut counters, &mut table);
        found.extend(
            table
                .visited
                .drain(..)
                .filter(|&(node, _, _, _)| keep[node])
                .map(
                    |(node, syntax_owner, syntax_measure, syntax_ordinal)| Visit {
                        owner,
                        syntax_owner,
                        syntax_measure,
                        syntax_ordinal,
                        measure: false,
                        node,
                    },
                ),
        );
        if let Some(measure) = function_measure_root(&nodes[owner].body) {
            walk.owner(owner, measure, false, true, &mut counters, &mut table);
            found.extend(
                table
                    .visited
                    .drain(..)
                    .filter(|&(node, _, _, _)| keep[node])
                    .map(
                        |(node, syntax_owner, syntax_measure, syntax_ordinal)| Visit {
                            owner,
                            syntax_owner,
                            syntax_measure,
                            syntax_ordinal,
                            measure: true,
                            node,
                        },
                    ),
            );
        }
    }
    found
}

/// The frames some record's chain holds, in the reader's canonical order, and
/// each candidate row's id in that table.
fn canonical(
    nodes: &[CheckedSemanticNodeV2],
    table: &Table,
    mut touched_by: Vec<(usize, Vec<usize>)>,
    terminals: &[usize],
) -> Result<(Vec<PathConditionFrame>, Vec<usize>), EmitRefusal> {
    // Keep the frames some record's chain holds, in canonical order.
    let mut used = vec![false; table.rows.len()];
    for &terminal in terminals {
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
    touched_by.sort_by(|left, right| nodes[left.0].node_id.cmp(&nodes[right.0].node_id));
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
    Ok((rows, renumbered))
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
    renumbering: &super::ordinals::Renumbering,
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
    let mut keys = ids.key_reader();
    let node_keys = nodes
        .iter()
        .map(|node| {
            keys.key(&node.node_id)
                .map_err(|_| EmitRefusal::SourceMapInvariant)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let key_index = key_index(&node_keys);
    let mut walk = Walk {
        nodes,
        index: &index,
        key_index: Some(&key_index),
        resolver: Some(KeyResolver::Reader(&mut keys)),
        resolved: HashMap::new(),
    };
    // The reader walks every function owner, in FR-322's package-wide order
    // (declaration source position, then node id), with each node's
    // expression-occurrence counter shared by all owners (FSpec FR-825): an
    // owner without a guarded record still takes the occurrences it reads.
    let walked = owner_order(nodes, source_map);
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
    let mut touched_by = Vec::<(usize, Vec<usize>)>::new();
    for position in walked {
        let owner_id = &nodes[position].node_id;
        let root =
            function_body_root(&nodes[position].body).ok_or(EmitRefusal::SourceMapInvariant)?;
        let derived = walk.owner(position, root, true, false, &mut visits, &mut table);
        // The termination measure takes occurrences but derives no frame.
        if let Some(measure) = function_measure_root(&nodes[position].body) {
            walk.owner(position, measure, false, true, &mut visits, &mut table);
        }
        touched_by.push((position, table.take_owner_touches()));
        table.visited.clear();
        let Some(records) = records_of.get(owner_id) else {
            continue;
        };
        for &(claim, source) in records.iter() {
            let application_id = node_id(ids, claim.node());
            let application = *index
                .get(&application_id)
                .ok_or(EmitRefusal::SourceMapInvariant)?;
            let ordinal = renumbering
                .ordinal(claim.node(), claim.origin().ordinal())
                .ok_or(EmitRefusal::SourceMapInvariant)?;
            let terminal = derived
                .get(&(application, ordinal))
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
            let frame_at = at.ok_or(EmitRefusal::SourceMapInvariant)?;
            let frame = &table.rows[frame_at];
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
    let terminal_ids: Vec<usize> = terminals.iter().map(|&(_, _, terminal)| terminal).collect();
    let (rows, renumbered) = canonical(nodes, &table, touched_by, &terminal_ids)?;
    Ok(Frames {
        rows,
        terminals: terminals
            .into_iter()
            .map(|(claim, _, terminal)| (claim.clone(), PathConditionFrameId(renumbered[terminal])))
            .collect(),
    })
}

/// A guarded value-validity record a writer carries over: the function whose
/// body holds its application, the application, and the occurrence it names.
pub(super) struct CarriedRecord<'a> {
    pub(super) owner: &'a CheckedNodeId,
    pub(super) application: &'a CheckedNodeId,
    pub(super) occurrence: u64,
}

/// The canonical frame table of the final `nodes`, re-derived by the reader's
/// walk (FSpec FR-825), and each record's terminal frame in it. A writer that
/// rekeys functions changes the owner order and with it the table's order, so
/// the carried table is derived again, never permuted.
pub(super) fn rederive(
    nodes: &[CheckedSemanticNodeV2],
    source_map: &[CheckedSourceMapEntry],
    records: &[CarriedRecord<'_>],
) -> Result<(Vec<PathConditionFrame>, Vec<PathConditionFrameId>), EmitRefusal> {
    let index: BTreeMap<&CheckedNodeId, usize> = nodes
        .iter()
        .enumerate()
        .map(|(position, node)| (&node.node_id, position))
        .collect();
    let graph = WireGraph::new(nodes.iter().map(|node| &node.node_id))
        .map_err(|_| EmitRefusal::SourceMapInvariant)?;
    let key_index = key_index(graph.keys());
    let mut walk = Walk {
        nodes,
        index: &index,
        key_index: Some(&key_index),
        resolver: Some(KeyResolver::Graph(&graph)),
        resolved: HashMap::new(),
    };
    let mut wanted = HashMap::<usize, Vec<(usize, usize, u64)>>::new();
    for (at, record) in records.iter().enumerate() {
        let owner = *index
            .get(record.owner)
            .ok_or(EmitRefusal::SourceMapInvariant)?;
        let application = *index
            .get(record.application)
            .ok_or(EmitRefusal::SourceMapInvariant)?;
        wanted
            .entry(owner)
            .or_default()
            .push((at, application, record.occurrence));
    }
    let mut table = Table::default();
    let mut visits = HashMap::<usize, usize>::new();
    let mut terminals = vec![0_usize; records.len()];
    let mut touched_by = Vec::<(usize, Vec<usize>)>::new();
    for position in owner_order(nodes, source_map) {
        let root =
            function_body_root(&nodes[position].body).ok_or(EmitRefusal::SourceMapInvariant)?;
        let derived = walk.owner(position, root, true, false, &mut visits, &mut table);
        if let Some(measure) = function_measure_root(&nodes[position].body) {
            walk.owner(position, measure, false, true, &mut visits, &mut table);
        }
        touched_by.push((position, table.take_owner_touches()));
        table.visited.clear();
        for &(at, application, occurrence) in wanted.get(&position).into_iter().flatten() {
            terminals[at] = derived
                .get(&(application, occurrence))
                .copied()
                .flatten()
                .ok_or(EmitRefusal::SourceMapInvariant)?;
        }
    }
    let (rows, renumbered) = canonical(nodes, &table, touched_by, &terminals)?;
    Ok((
        rows,
        terminals
            .into_iter()
            .map(|terminal| PathConditionFrameId(renumbered[terminal]))
            .collect(),
    ))
}
