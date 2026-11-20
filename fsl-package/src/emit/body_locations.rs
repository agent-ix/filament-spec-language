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

use std::collections::{BTreeSet, HashMap};

use filament_contract_model::{
    CheckedBindingNameKind, CheckedBodyLocation, CheckedBodyLocationOrigin,
    CheckedBodySemanticLocation, CheckedNodeId, CheckedSemanticNodeV2, CheckedTerm,
};
use filament_exact::NodeKey;
use filament_semantic_value::location::{
    ExpressionLocation as Location, ExpressionOwner as Origin,
};
use fsl_semantics::check::CheckedGraph;

use super::protocol_locations::{applications_at, ordered, term_rows, wire_path, PendingRow};
use super::EmitRefusal;

/// The least bytes one row costs beyond its path.
const ROW_BYTES: u128 = 64;

/// The bytes charged for each index of a row's path: its spelling and the
/// room it takes while the rows are held.
const PATH_ENTRY_BYTES: u128 = 8;

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

/// Every `body_locations` row of the emitted function bodies, in the
/// (application, occurrence, `term_ordinal`) order FSpec FR-322 requires.
pub(super) fn checked_body_locations(
    graph: &CheckedGraph,
    emitted_node_ids: &HashMap<NodeKey, CheckedNodeId>,
    nodes: &[CheckedSemanticNodeV2],
    output_bytes: u64,
) -> Result<Vec<CheckedBodySemanticLocation>, EmitRefusal> {
    let with_body: BTreeSet<&CheckedNodeId> = nodes
        .iter()
        .filter(|node| {
            node.node_tag.as_ref() == "function"
                && node.node_id.package.as_ref() == graph.package_name()
                && has_body_binding(node)
        })
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
    // body grow with the square of its depth; the byte ceiling is charged
    // before a path is built.
    let charged = std::cell::Cell::new(0_u128);
    let exceeded = std::cell::Cell::new(false);
    for (id, name, index, root, origin) in &functions {
        let wire = |at: &Location| -> Option<(String, CheckedBodyLocation)> {
            if &at.origin != origin {
                return None;
            }
            let cost = PATH_ENTRY_BYTES * u128::try_from(at.depth()).ok()? + ROW_BYTES;
            charged.set(charged.get().saturating_add(cost));
            if charged.get() > u128::from(output_bytes) {
                exceeded.set(true);
                return None;
            }
            let location = CheckedBodyLocation {
                origin: CheckedBodyLocationOrigin::FunctionBody {
                    function: (*id).clone(),
                    name: name.as_str().into(),
                    index: u64::try_from(*index).ok()?,
                },
                path: wire_path(at)?,
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
    Ok(ordered(
        pending,
        |application, occurrence, term_ordinal, location| CheckedBodySemanticLocation {
            application,
            occurrence,
            term_ordinal,
            location,
        },
    ))
}
