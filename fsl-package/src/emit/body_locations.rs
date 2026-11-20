// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-531 "Function body locations in the bytes": one
//! `checked_term_data.body_locations` row for every term of every function
//! body the emitter writes.
//!
//! A body is walked exactly as FR-383 walks a protocol slot, from the body's
//! root, and each term is keyed and located as a slot's term is, under the
//! checker's `Body` owner of the function. A function has a body here when
//! its emitted node carries a `body` binding, which is the walk FSpec FR-322
//! defines on the wire.

use std::cell::RefCell;
use std::collections::{BTreeSet, HashMap};

use filament_contract_model::{
    CheckedBindingNameKind, CheckedBodyLocation, CheckedBodyLocationOrigin,
    CheckedBodySemanticLocation, CheckedNodeId, CheckedOccurrence, CheckedSemanticNodeV2,
    CheckedTerm,
};
use filament_exact::NodeKey;
use filament_semantic_value::location::{
    ExpressionLocation as Location, ExpressionOwner as Origin,
};
use fsl_semantics::check::CheckedGraph;

use super::protocol_locations::{
    applications_at, ordered, term_rows, wire_path, PendingRow, MAX_SAFE,
};
use super::EmitRefusal;

/// The serialized bytes the path indices of a location add to a row, memoized
/// by the address of the location's last link: a location's bytes are its
/// parent's and its own index's.
type PathBytes = HashMap<usize, u128>;

/// The bytes one path index takes in the canonical array: its digits, the two
/// quotes of a position above the safe range, and the separator that follows.
fn index_bytes(index: usize) -> u128 {
    let digits = u128::from(index.checked_ilog10().map_or(1, |log| log + 1));
    let quotes = if u64::try_from(index).map_or(true, |index| index > MAX_SAFE) {
        2
    } else {
        0
    };
    digits + quotes + 1
}

/// The bytes the indices of `at`'s path add to a row whose path is empty,
/// without building the path.
fn path_bytes(at: &Location, memo: &mut PathBytes) -> u128 {
    let mut walked = Vec::new();
    let mut total = 0;
    for (address, index) in at.ancestry() {
        if let Some(known) = memo.get(&address) {
            total = *known;
            break;
        }
        walked.push((address, index_bytes(index)));
    }
    for (address, own) in walked.into_iter().rev() {
        total += own;
        memo.insert(address, total);
    }
    // An empty path is `[]`; a path of n indices adds its n indices and n - 1
    // separators, and the row itself is followed by a separator.
    if at.depth() == 0 {
        1
    } else {
        total
    }
}

/// The bytes a `term_ordinal` adds beyond the one digit its row is charged.
fn ordinal_bytes(term_ordinal: u64) -> u128 {
    u128::from(term_ordinal.checked_ilog10().unwrap_or(0))
}

/// Whether an emitted function node carries a `body` binding.
fn has_body_binding(node: &CheckedSemanticNodeV2) -> bool {
    let CheckedTerm::Aggregate { members } = &node.body else {
        return false;
    };
    members.iter().any(|member| {
        matches!(
            member,
            CheckedTerm::Binding { name, .. } if name.kind() == CheckedBindingNameKind::Body
        )
    })
}

/// Whether `node` is a `function` node of the package named `package` that
/// carries a `body` binding: the only nodes the reader owes rows for (FSpec
/// FR-322).
///
/// `#[string_edge]`: `function` is the admitted wire's fixed node tag.
#[ix_trace_rs::string_edge]
fn is_local_function_with_body(node: &CheckedSemanticNodeV2, package: &str) -> bool {
    node.node_tag.as_ref() == "function"
        && node.node_id.package.as_ref() == package
        && has_body_binding(node)
}

/// Every `body_locations` row of the emitted function bodies, in the
/// (application, occurrence, `term_ordinal`) order FSpec FR-322 requires.
pub(super) fn checked_body_locations(
    graph: &CheckedGraph,
    emitted_node_ids: &HashMap<NodeKey, CheckedNodeId>,
    nodes: &[CheckedSemanticNodeV2],
    source_keys: &super::SourceDeclarationKeys,
    output_bytes: u64,
) -> Result<Vec<CheckedBodySemanticLocation>, EmitRefusal> {
    let with_body: BTreeSet<&CheckedNodeId> = nodes
        .iter()
        .filter(|node| is_local_function_with_body(node, graph.package_name()))
        .map(|node| &node.node_id)
        .collect();
    let mut functions = Vec::new();
    let mut index = 0;
    while let Some(state) = graph.function_state(index) {
        let located = graph
            .function_identity_at(index)
            .and_then(|identity| emitted_node_ids.get(&identity))
            .filter(|id| with_body.contains(id));
        if let Some(id) = located {
            functions.push((
                id,
                state.name.to_owned(),
                index,
                state.body,
                Origin::Body {
                    function: state.name.to_owned(),
                    index,
                },
            ));
        }
        index += 1;
    }
    let origins = functions.iter().map(|function| &function.4).collect();
    let applications = applications_at(graph, &origins);
    let mut pending: Vec<PendingRow<CheckedBodyLocation>> = Vec::new();
    // Each row carries its path from the body's root, so the rows of a deep
    // body grow with the square of its depth. A row is charged the bytes it
    // is written in, before its path is built: the row with an empty path,
    // serialized, and the path's indices as they serialize.
    let charged = std::cell::Cell::new(0_u128);
    let exceeded = std::cell::Cell::new(false);
    let memo = RefCell::new(PathBytes::new());
    for (id, name, index, root, origin) in &functions {
        let wire = |at: &Location,
                    application: &CheckedNodeId,
                    occurrence: &CheckedOccurrence|
         -> Option<(String, CheckedBodyLocation)> {
            if &at.origin != origin {
                return None;
            }
            let origin = CheckedBodyLocationOrigin::FunctionBody {
                function: (*id).clone(),
                name: super::emitted_function_name(name, source_keys).into(),
                index: u64::try_from(*index).ok()?,
            };
            let empty = CheckedBodySemanticLocation {
                application: application.clone(),
                occurrence: occurrence.clone(),
                term_ordinal: 0,
                location: CheckedBodyLocation {
                    origin,
                    path: Vec::new(),
                },
            };
            let row = u128::try_from(serde_json::to_vec(&empty).ok()?.len()).ok()?;
            let cost = row.saturating_add(path_bytes(at, &mut memo.borrow_mut()));
            charged.set(charged.get().saturating_add(cost));
            if charged.get() > u128::from(output_bytes) {
                exceeded.set(true);
                return None;
            }
            let location = CheckedBodyLocation {
                path: wire_path(at)?,
                ..empty.location
            };
            Some((name.clone(), location))
        };
        if let Err(refusal) = term_rows(
            graph,
            *root,
            &applications,
            emitted_node_ids,
            &mut pending,
            &wire,
        ) {
            return Err(if exceeded.get() {
                EmitRefusal::OutputLimit(super::output_limit(output_bytes, charged.get()))
            } else {
                refusal
            });
        }
    }
    let rows = ordered(
        pending,
        |application, occurrence, term_ordinal, location| CheckedBodySemanticLocation {
            application,
            occurrence,
            term_ordinal,
            location,
        },
    );
    // A row was charged one digit for its `term_ordinal`, which is assigned
    // once the rows are grouped.
    let written = rows.iter().fold(charged.get(), |total, row| {
        total.saturating_add(ordinal_bytes(row.term_ordinal))
    });
    if written > u128::from(output_bytes) {
        return Err(EmitRefusal::OutputLimit(super::output_limit(
            output_bytes,
            written,
        )));
    }
    Ok(rows)
}
