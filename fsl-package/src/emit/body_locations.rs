// SPDX-License-Identifier: AGPL-3.0-or-later
//! Captured function-body locations in canonical relative form (FSpec FR-322).
//! Source locations come from the checked tree; relative encoding follows the
//! emitted graph's walk, including every reach through composite values.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use filament_contract_model::{
    CheckedBindingNameKind, CheckedBodyLocation, CheckedBodyLocationOrigin,
    CheckedBodySemanticLocation, CheckedKeyOrdinal, CheckedNodeId, CheckedSemanticNodeV2,
    CheckedTerm,
};
use filament_exact::NodeKey;
use filament_semantic_value::location::{
    ExpressionLocation as Location, ExpressionOwner as Origin,
};
use fsl_semantics::check::CheckedGraph;

use super::protocol_locations::{applications_at, ordered, term_rows, PendingRow, MAX_SAFE};
use super::EmitRefusal;
use crate::wire_form::{has_form, has_tag, WireForm, WireTag};

/// Interned checker paths make finding a common prefix proportional to the
/// encoded change, even when equivalent paths have independently built links.
#[derive(Default)]
struct Paths {
    addresses: HashMap<usize, usize>,
    // Keep memoized link addresses alive, including independently built paths.
    retained: Vec<Location>,
    children: HashMap<(usize, usize), usize>,
    // Entry zero is the empty path.
    links: Vec<(usize, usize, usize)>,
}

impl Paths {
    fn intern(&mut self, location: &Location) -> usize {
        if self.links.is_empty() {
            self.links.push((0, 0, 0));
        }
        if let Some(address) = location.chain_address() {
            if let Some(known) = self.addresses.get(&address) {
                return *known;
            }
            self.retained.push(location.clone());
        }
        let mut pending = Vec::new();
        let mut parent = 0;
        for (address, index) in location.ancestry() {
            if let Some(known) = self.addresses.get(&address) {
                parent = *known;
                break;
            }
            pending.push((address, index));
        }
        for (address, index) in pending.into_iter().rev() {
            let child = *self.children.entry((parent, index)).or_insert_with(|| {
                let child = self.links.len();
                self.links.push((parent, index, self.links[parent].2 + 1));
                child
            });
            self.addresses.insert(address, child);
            parent = child;
        }
        parent
    }

    fn delta(
        &self,
        mut previous: usize,
        mut current: usize,
    ) -> Option<(u64, Vec<CheckedKeyOrdinal>)> {
        let mut up = 0_u64;
        let mut down = Vec::new();
        while previous != current {
            let left = self.links.get(previous)?;
            let right = self.links.get(current)?;
            if left.2 >= right.2 {
                previous = left.0;
                up = up.checked_add(1)?;
            } else {
                let index = u64::try_from(right.1).ok()?;
                down.push(if index <= MAX_SAFE {
                    CheckedKeyOrdinal::Safe(index)
                } else {
                    CheckedKeyOrdinal::BeyondSafe(index.to_string().into())
                });
                current = right.0;
            }
        }
        down.reverse();
        Some((up, down))
    }
}

fn body(node: &CheckedSemanticNodeV2) -> Option<&CheckedTerm> {
    let CheckedTerm::Aggregate { members, .. } = &node.body else {
        return None;
    };
    members.iter().find_map(|member| match member {
        CheckedTerm::Binding { name, value, .. } if name.kind() == CheckedBindingNameKind::Body => {
            Some(value.as_ref())
        }
        _ => None,
    })
}

fn is_term(node: &CheckedSemanticNodeV2) -> bool {
    has_tag(node, WireTag::Expression)
        && !has_form(node, WireTag::Expression, WireForm::Reference)
        && !matches!(&node.body, CheckedTerm::Application { operation, .. }
            if operation.identity().is_some_and(|identity| matches!(identity,
                "filament.op.control.let" | "filament.op.control.if" | "filament.op.control.case")))
}

/// References encountered in stored member / ascending tuple order. Atomic
/// values and parameters end a reach; composite values expose their children.
fn references<'a>(
    term: &'a CheckedTerm,
    nodes: &BTreeMap<&CheckedNodeId, &'a CheckedSemanticNodeV2>,
    package: &str,
) -> Vec<&'a CheckedSemanticNodeV2> {
    let mut pending = vec![term];
    let mut found = Vec::new();
    while let Some(term) = pending.pop() {
        match term {
            CheckedTerm::Reference { target } => {
                if target.package.as_ref() != package {
                    continue;
                }
                if let Some(node) = nodes.get(target) {
                    if has_tag(node, WireTag::Expression)
                        || has_form(node, WireTag::Value, WireForm::RecordValue)
                        || has_form(node, WireTag::Value, WireForm::TupleValue)
                    {
                        found.push(*node);
                    }
                }
            }
            CheckedTerm::Application { arguments, .. } => pending.extend(arguments.iter().rev()),
            CheckedTerm::Aggregate { members, .. } => pending.extend(members.iter().rev()),
            CheckedTerm::Binding { value, .. } => pending.push(value),
            _ => {}
        }
    }
    found
}

/// Every captured row, in the wire's (application, occurrence, term ordinal)
/// order, with deltas computed in each function's actual body-walk order.
#[ix_trace_rs::string_edge]
pub(super) fn checked_body_locations(
    graph: &CheckedGraph,
    emitted_node_ids: &HashMap<NodeKey, CheckedNodeId>,
    nodes: &[CheckedSemanticNodeV2],
    source_keys: &super::SourceDeclarationKeys,
    meter: &mut super::OutputMeter,
) -> Result<Vec<CheckedBodySemanticLocation>, EmitRefusal> {
    let nodes: BTreeMap<_, _> = nodes.iter().map(|node| (&node.node_id, node)).collect();
    let with_body: BTreeSet<_> = nodes
        .values()
        .filter(|node| {
            node.node_tag.as_ref() == "function"
                && node.node_id.package.as_ref() == graph.package_name()
                && body(node).is_some()
        })
        .map(|node| &node.node_id)
        .collect();
    let mut functions = Vec::new();
    let mut index = 0;
    while let Some(state) = graph.function_state(index) {
        if let Some(id) = graph
            .function_identity_at(index)
            .and_then(|identity| emitted_node_ids.get(&identity))
            .filter(|id| with_body.contains(id))
        {
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
    let mut pending = Vec::new();

    meter.section(|writer| {
        for (id, name, index, root, origin) in &functions {
            let mut paths = Paths::default();
            let mut captured = Vec::new();
            term_rows(
                graph,
                *root,
                &applications,
                emitted_node_ids,
                &mut captured,
                &|at, _, _| (&at.origin == origin).then(|| (name.clone(), at.clone())),
                true,
            )?;
            let captured = ordered(captured, |application, occurrence, ordinal, location| {
                (application, occurrence, ordinal, location)
            });
            let mut by_application: BTreeMap<_, Vec<_>> = BTreeMap::new();
            for (application, occurrence, _, location) in &captured {
                by_application
                    .entry(application)
                    .or_default()
                    .push((occurrence, location));
            }
            let mut visits: BTreeMap<&CheckedNodeId, usize> = BTreeMap::new();
            let mut previous = 0;
            let mut walk = nodes
                .get(id)
                .and_then(|node| body(node))
                .and_then(|body| match body {
                    CheckedTerm::Reference { target }
                        if target.package.as_ref() == graph.package_name() =>
                    {
                        nodes.get(target).copied()
                    }
                    _ => None,
                })
                .into_iter()
                .collect::<Vec<_>>();
            while let Some(node) = walk.pop() {
                if is_term(node) {
                    let rank = visits.entry(&node.node_id).or_default();
                    let candidates = by_application
                        .get(&node.node_id)
                        .ok_or(EmitRefusal::SourceMapInvariant)?;
                    // Sharing through distinct value parents can reach a captured
                    // application again. Retain that application's genuine last
                    // capture; the new visit gets its own term coordinate.
                    let (occurrence, location) = candidates
                        .get(*rank)
                        .or_else(|| candidates.last())
                        .ok_or(EmitRefusal::SourceMapInvariant)?;
                    *rank += 1;
                    let current = paths.intern(location);
                    let (up, down) = paths
                        .delta(previous, current)
                        .ok_or(EmitRefusal::SourceMapInvariant)?;
                    previous = current;
                    let location = CheckedBodyLocation {
                        origin: CheckedBodyLocationOrigin::FunctionBody {
                            function: (*id).clone(),
                            name: super::emitted_function_name(name, source_keys).into(),
                            index: u64::try_from(*index)
                                .map_err(|_| EmitRefusal::SourceMapInvariant)?,
                        },
                        up,
                        down,
                    };
                    let row = CheckedBodySemanticLocation {
                        application: node.node_id.clone(),
                        occurrence: (*occurrence).clone(),
                        term_ordinal: 0,
                        location,
                    };
                    writer.push(&row)?;
                    pending.push(PendingRow {
                        application: row.application,
                        occurrence: row.occurrence,
                        origin_name: name.clone(),
                        visit: pending.len(),
                        location: row.location,
                    });
                }
                walk.extend(
                    references(&node.body, &nodes, graph.package_name())
                        .into_iter()
                        .rev(),
                );
            }
        }
        Ok(())
    })?;
    let rows = ordered(
        pending,
        |application, occurrence, term_ordinal, location| CheckedBodySemanticLocation {
            application,
            occurrence,
            term_ordinal,
            location,
        },
    );
    // Each row was charged with term ordinal zero; its ordinal's further
    // digits are bytes the package also carries.
    let digits = rows.iter().fold(0_u64, |total, row| {
        total.saturating_add(u64::from(row.term_ordinal.checked_ilog10().unwrap_or(0)))
    });
    meter.charge_bytes(digits)?;
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;
    use filament_contract_model::{read_checked_package, CheckedPackageDispatchResult};
    use ix_trace_rs::trace;

    #[ix_trace_rs::trace("FSpec-FR-322-AC-510")]
    #[test]
    fn compact_paths_use_the_longest_prefix_even_for_independent_chains() {
        let root = Location::root(Origin::Expression);
        let mut paths = Paths::default();
        let first = paths.intern(&Location::at(Origin::Expression, &[2, 4, 6]));
        let second = paths.intern(&Location::at(Origin::Expression, &[2, 4, 8, 1]));
        assert_eq!(
            paths.delta(first, second),
            Some((
                1,
                vec![CheckedKeyOrdinal::Safe(8), CheckedKeyOrdinal::Safe(1)]
            ))
        );
        assert_eq!(paths.delta(second, second), Some((0, vec![])));
        let ancestor = paths.intern(&root.child(2));
        assert_eq!(paths.delta(second, ancestor), Some((3, vec![])));
        let empty = paths.intern(&root);
        assert_eq!(paths.delta(ancestor, empty), Some((1, vec![])));
    }

    #[trace("FR-531-AC-39", "FSpec-FR-322-AC-510", "FSpec-FR-322-AC-513")]
    #[test]
    fn same_bound_ieee_ordering_keeps_shared_equality_body_visits() {
        let (_, checked) = super::super::tests::package_from_text(
            "function f using v(): Boolean pure {\n\
             let one: Float64[nearest-even] = float64(bits: 0x3ff0000000000000) in one <= one }",
        );
        let emission =
            super::super::emit_checked(&checked, crate::checked::EmitLimits::new(1 << 20)).unwrap();
        let package = match read_checked_package(
            emission.package().bytes(),
            crate::checked_v2::V2ReadLimits::fixture().for_ir(),
            emission.evidence(),
        ) {
            CheckedPackageDispatchResult::AdmittedV2(package) => package,
            other => panic!("shared IEEE ordering must admit: {other:?}"),
        };
        let rows = &package.checked_term_data().unwrap().body_locations;
        assert_eq!(rows.len(), 7, "each lowered term retains its visit");
        let equality = package
            .resolved_graph()
            .nodes
            .iter()
            .find(|node| {
                matches!(&node.body, CheckedTerm::Application { operation, .. }
                if operation.identity() == Some("filament.op.ieee.numeric_equal"))
            })
            .expect("the three equality checks share one application");
        let equality_rows: Vec<_> = rows
            .iter()
            .filter(|row| row.application == equality.node_id)
            .collect();
        assert_eq!(equality_rows.len(), 3);
        assert_eq!(
            equality_rows
                .iter()
                .map(|row| (&row.occurrence, row.term_ordinal))
                .collect::<BTreeSet<_>>()
                .len(),
            3,
            "the shared identity keeps each genuine occurrence"
        );
        let CheckedBodyLocationOrigin::FunctionBody { function, .. } = &rows[0].location.origin;
        let mut cursor = package.body_locations(function, 10_000).unwrap();
        let mut paths = Vec::new();
        while let Some(view) = cursor.next_location().unwrap() {
            paths.push(view.path.to_vec());
        }
        assert_eq!(paths, vec![vec![CheckedKeyOrdinal::Safe(1)]; 7]);
    }

    #[ix_trace_rs::trace("FR-531-AC-39", "FSpec-FR-322-AC-510", "FSpec-FR-322-AC-513")]
    #[test]
    fn compact_composite_bodies_keep_each_shared_application_reach() {
        let (_, checked) = super::super::tests::package_from_text(
            "record Pair { left: Integer; right: Integer; }\n\
             function pair using v(x: Integer): Pair pure { Pair { right: x - 1, left: x - 1 } }\n",
        );
        let emission =
            super::super::emit_checked(&checked, crate::checked::EmitLimits::new(1 << 20)).unwrap();
        let package = match read_checked_package(
            emission.package().bytes(),
            crate::checked_v2::V2ReadLimits::fixture().for_ir(),
            emission.evidence(),
        ) {
            CheckedPackageDispatchResult::AdmittedV2(package) => package,
            other => panic!("composite body must read with all reaches: {other:?}"),
        };
        let rows = &package.checked_term_data().unwrap().body_locations;
        assert_eq!(rows.len(), 2, "the record itself contributes no row");
        assert_eq!(
            rows[0].application, rows[1].application,
            "both source positions share the arithmetic application"
        );
        assert_ne!(
            (&rows[0].occurrence, rows[0].term_ordinal),
            (&rows[1].occurrence, rows[1].term_ordinal)
        );
        let CheckedBodyLocationOrigin::FunctionBody { function, .. } = &rows[0].location.origin;
        let mut cursor = package.body_locations(function, 10_000).unwrap();
        let mut paths = Vec::new();
        while let Some(view) = cursor.next_location().unwrap() {
            paths.push(view.path.to_vec());
        }
        assert_eq!(paths.len(), 2);
        assert_eq!(
            paths,
            vec![
                vec![CheckedKeyOrdinal::Safe(1)],
                vec![CheckedKeyOrdinal::Safe(0)]
            ],
            "stored record field order retains each source field's captured position"
        );
        let wire: serde_json::Value = serde_json::from_slice(emission.package().bytes()).unwrap();
        for row in wire["checked_term_data"]["body_locations"]
            .as_array()
            .unwrap()
        {
            assert!(row["location"].get("path").is_none());
            assert!(row["location"].get("up").is_some());
            assert!(row["location"].get("down").is_some());
        }
    }
    #[ix_trace_rs::trace("FR-531-AC-39", "FSpec-FR-322-AC-510", "FSpec-FR-322-AC-513")]
    #[test]
    fn compact_tuple_body_walks_two_distinct_record_parents() {
        let (_, checked) = super::super::tests::package_from_text(
            "record Cell { n: Integer; tag: Integer; }\n\
             tuple Pair(Cell, Cell);\n\
             function pair using v(x: Integer): Pair pure { Pair(Cell { n: x - 1, tag: 0 }, Cell { n: x - 1, tag: 1 }) }\n",
        );
        let emission =
            super::super::emit_checked(&checked, crate::checked::EmitLimits::new(1 << 20)).unwrap();
        let package = match read_checked_package(
            emission.package().bytes(),
            crate::checked_v2::V2ReadLimits::fixture().for_ir(),
            emission.evidence(),
        ) {
            CheckedPackageDispatchResult::AdmittedV2(package) => package,
            other => panic!("nested composite bodies must retain both reaches: {other:?}"),
        };
        let rows = &package.checked_term_data().unwrap().body_locations;
        assert_eq!(
            rows.len(),
            2,
            "neither the tuple nor its records contributes a row"
        );
        assert_eq!(rows[0].application, rows[1].application);
        assert_eq!(
            package
                .resolved_graph()
                .nodes
                .iter()
                .filter(|node| has_form(node, WireTag::Value, WireForm::RecordValue))
                .count(),
            2,
            "different tags keep two genuine record-value parents"
        );
        assert_eq!(
            package
                .resolved_graph()
                .nodes
                .iter()
                .filter(|node| has_form(node, WireTag::Value, WireForm::TupleValue))
                .count(),
            1
        );
        let CheckedBodyLocationOrigin::FunctionBody { function, .. } = &rows[0].location.origin;
        let mut cursor = package.body_locations(function, 10_000).unwrap();
        let mut paths = Vec::new();
        while let Some(view) = cursor.next_location().unwrap() {
            paths.push(view.path.to_vec());
        }
        assert_eq!(
            paths,
            vec![
                vec![CheckedKeyOrdinal::Safe(0), CheckedKeyOrdinal::Safe(0)],
                vec![CheckedKeyOrdinal::Safe(1), CheckedKeyOrdinal::Safe(0)]
            ]
        );
    }
    #[ix_trace_rs::trace("FR-531-AC-39", "FSpec-FR-322-AC-510")]
    #[test]
    fn compact_deep_body_stores_linear_delta_entries() {
        const TERMS: usize = 128;
        let (_, checked) = super::super::tests::package_from_text(&format!(
            "function f using v(x: Integer): Integer pure {{ {}x }}\n",
            "x + ".repeat(TERMS),
        ));
        let emission =
            super::super::emit_checked(&checked, crate::checked::EmitLimits::new(1 << 20)).unwrap();
        let package = match read_checked_package(
            emission.package().bytes(),
            crate::checked_v2::V2ReadLimits::fixture().for_ir(),
            emission.evidence(),
        ) {
            CheckedPackageDispatchResult::AdmittedV2(package) => package,
            other => panic!("the compact deep body must read: {other:?}"),
        };
        let rows = &package.checked_term_data().unwrap().body_locations;
        assert_eq!(rows.len(), TERMS);
        let entries: usize = rows.iter().map(|row| row.location.down.len()).sum();
        assert_eq!(
            entries,
            TERMS - 1,
            "each nested checker link is stored once"
        );
        assert!(rows.iter().all(|row| row.location.up == 0));
    }
}
