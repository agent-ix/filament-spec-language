// SPDX-License-Identifier: AGPL-3.0-or-later
//! Scalar requirement facts survive emission and strict native admission.

use std::collections::{BTreeMap, BTreeSet};

use filament_contract_model::{
    CapabilityKind, CheckedPackageReadLimits, CheckedPackageV2, CheckedPackageV2ReadResult,
    CheckedTerm, ValueValidityRecord,
};
use fsl_semantics::check::wire_id::WireGraph;
use ix_trace_rs::trace;

use crate::emit::{emit_checked, Emission};
use crate::CheckedPackage;

const GUARDED: &str = "function Guarded using v(x: Int[0, 9], p: Boolean): Integer pure { if p then x + 1 else x * 2 }";

fn admitted_package(declarations: &str) -> (CheckedPackage, Emission, CheckedPackageV2) {
    let (_, package) = crate::emit::tests::package_from_text(declarations);
    let emission = emit_checked(&package, crate::EmitLimits::fixture()).expect("scalar unit emits");
    let admitted = match CheckedPackageV2::read(
        emission.package().bytes(),
        CheckedPackageReadLimits::bounded(),
        emission.evidence(),
    ) {
        CheckedPackageV2ReadResult::Admitted(admitted) => admitted,
        failure => panic!("the strict reader admits the scalar records: {failure:?}"),
    };
    (package, emission, *admitted)
}

fn emitted_records(
    declarations: &str,
) -> (
    CheckedPackage,
    Emission,
    CheckedPackageV2,
    Vec<ValueValidityRecord>,
) {
    let (package, emission, admitted) = admitted_package(declarations);
    let records = admitted
        .requirement_items()
        .iter()
        .filter(|row| row.kind == CapabilityKind::ValueValidity)
        .map(|row| {
            super::value_record(row)
                .expect("each scalar item has its record")
                .clone()
        })
        .collect::<Vec<_>>();
    assert!(
        !records.is_empty(),
        "the arithmetic produces scalar obligations"
    );
    (package, emission, admitted, records)
}

/// The guards of `record`, outermost first, as `(guard node, occurrence, holds)`.
fn chain(
    admitted: &CheckedPackageV2,
    record: &ValueValidityRecord,
) -> Vec<(serde_json::Value, bool)> {
    admitted
        .path_condition(record)
        .iter()
        .map(|guard| (serde_json::to_value(guard.guard).unwrap(), guard.holds))
        .collect()
}

/// The checker's own chain of the record at `claim`, in the wire's guard form.
fn source_chain(
    identities: &WireGraph,
    source: &fsl_semantics::check::RequirementRecord,
) -> Vec<(serde_json::Value, bool)> {
    source
        .path_condition()
        .iter()
        .map(|guard| {
            (
                identities.wire_value(guard.occurrence().node()).unwrap(),
                guard.holds(),
            )
        })
        .collect()
}

/// A bare Boolean parameter keeps its own identity and occurrence as the
/// guard, now held once in the shared frame table: `if p then x + 1 else x * 2`
/// holds exactly the frames `(None, p, true)` and `(None, p, false)`, in that
/// order, and the `+` and `*` records cite them as their terminal frames.
#[trace("FR-940-AC-1", "FR-940-AC-13", "FR-062-AC-17")]
#[test]
fn bare_parameter_guard_is_one_shared_frame_per_outcome() {
    let (package, emission, admitted, records) = emitted_records(GUARDED);
    let wire: serde_json::Value = serde_json::from_slice(emission.package().bytes()).unwrap();
    let identities = WireGraph::of_package_value(&wire).expect("the emitted graph has identities");
    let frames = admitted.path_condition_frames();
    assert_eq!(frames.len(), 2, "one frame per outcome of the one guard");
    assert_eq!(frames[0].parent, None);
    assert_eq!(frames[1].parent, None);
    assert!(
        frames[0].holds && !frames[1].holds,
        "then frame first, else frame second"
    );
    assert_eq!(
        frames[0].guard, frames[1].guard,
        "both read the one parameter"
    );
    assert_eq!(frames[0].occurrence, frames[1].occurrence);
    // contract-ir reader defect B (QSL-2776 chain): the reader derives the
    // guard node's first `expression` occurrence (0) although the checker's
    // ordinal for the read is 1; this value changes once B makes numbering
    // package-wide.
    assert_eq!(
        frames[0].occurrence, 0,
        "the first `expression` occurrence of the guard node, as the reader derives it"
    );
    let nodes = crate::emit::tests::typed_nodes(&emission);
    let guard = nodes
        .iter()
        .find(|node| node.node_id == frames[0].guard)
        .expect("the guard is a document-held node");
    assert_eq!(guard.node_tag.as_ref(), "value");
    assert_eq!(
        guard.semantic_form.as_ref(),
        "parameter",
        "the frame names the original parameter, without an expression shim"
    );
    let mut terminals = Vec::new();
    let mut guarded = 0;
    for (claim, source) in package.graph().requirements().iter() {
        let application = identities
            .wire_value(claim.node())
            .expect("the claim is emitted");
        let record = records
            .iter()
            .find(|record| {
                serde_json::to_value(&record.application).unwrap() == application
                    && record.occurrence == claim.origin().ordinal()
            })
            .expect("the source claim has an admitted record");
        assert_eq!(
            chain(&admitted, record),
            source_chain(&identities, source),
            "the shared view yields the checker's own chain"
        );
        if source.path_condition().is_empty() {
            assert_eq!(record.path_condition_frame, None);
            continue;
        }
        assert_eq!(source.path_condition().len(), 1);
        terminals.push(
            record
                .path_condition_frame
                .expect("a guarded record cites a frame")
                .0,
        );
        guarded += 1;
    }
    terminals.sort_unstable();
    assert_eq!(
        terminals,
        [0, 1],
        "each branch's record cites its own frame"
    );
    assert_eq!(guarded, 2, "both branches retain their source guard");
}

/// Both arms of an if retain the positive or negative condition of that arm.
#[trace("FR-940-AC-1", "FR-940-AC-13")]
#[test]
fn both_guard_holds_outcomes_survive_native_admission() {
    let (package, _, admitted, records) = emitted_records(GUARDED);
    let expected = package
        .graph()
        .requirements()
        .values()
        .flat_map(|source| source.path_condition().iter().map(|guard| guard.holds()))
        .collect::<BTreeSet<_>>();
    let actual = records
        .iter()
        .flat_map(|record| chain(&admitted, record).into_iter().map(|guard| guard.1))
        .collect::<BTreeSet<_>>();
    assert_eq!(expected, BTreeSet::from([false, true]));
    assert_eq!(actual, expected);
}

/// An unconditional arithmetic record cites no frame and the package holds
/// no table.
#[trace("FR-940-AC-1", "FR-940-AC-13")]
#[test]
fn unguarded_record_keeps_an_empty_path_condition() {
    let (package, _, admitted, records) =
        emitted_records("function Plain using v(x: Int[0, 9]): Integer pure { x + 1 }");
    assert!(package
        .graph()
        .requirements()
        .values()
        .all(|source| source.path_condition().is_empty()));
    assert!(records
        .iter()
        .all(|record| record.path_condition_frame.is_none()
            && admitted.path_condition(record).is_empty()));
    assert!(admitted.path_condition_frames().is_empty());
}

/// The emitted `path_condition_frames` rows as `(parent, holds)`, and each
/// record's cited terminal frame (`None` when it cites none), read from the
/// wire without the reader.
type WireFrames = (Vec<(Option<u64>, bool)>, Vec<Option<u64>>);

fn wire_frames(emission: &Emission) -> WireFrames {
    let wire: serde_json::Value = serde_json::from_slice(emission.package().bytes()).unwrap();
    let frames = wire["path_condition_frames"]
        .as_array()
        .map(|rows| {
            rows.iter()
                .map(|row| (row["parent"].as_u64(), row["holds"].as_bool().unwrap()))
                .collect()
        })
        .unwrap_or_default();
    let mut terminals = wire["semantic_graph"]["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|node| node["semantic_form"] == "value_validity")
        .map(|node| {
            let cited = node["body"]["members"][5]["value"]["members"]
                .as_array()
                .unwrap();
            assert!(
                cited.len() <= 1,
                "a record cites at most one terminal frame"
            );
            cited.first().map(|id| {
                let value = &id["value"];
                value
                    .as_u64()
                    .unwrap_or_else(|| value.as_str().unwrap().parse().unwrap())
            })
        })
        .collect::<Vec<_>>();
    terminals.sort();
    (frames, terminals)
}

/// FSpec FR-825-AC-6's source with one branch holding no scalar operation:
/// that branch's frame is held by no record, so the table holds only the frame
/// of the branch with the `+`. Read from the wire, not the reader: contract-ir
/// reader defect A (QSL-2776 chain) refuses this package until it accepts a
/// missing branch frame, after which this test reads it through the reader.
#[trace("FR-825-AC-6", "FR-062-AC-17")]
#[test]
fn a_branch_without_a_scalar_operation_holds_no_frame() {
    let (_, package) = crate::emit::tests::package_from_text(
        "function f using v(x: Int[0, 9], p: Boolean): Integer pure { if p then x + 1 else x }",
    );
    let emission = emit_checked(&package, crate::EmitLimits::fixture()).expect("emits");
    assert_eq!(
        wire_frames(&emission),
        (vec![(None, true)], vec![Some(0)]),
        "one then-frame, cited by the one record"
    );
}

/// FSpec FR-825-AC-2: nested guards share every prefix once. `>=` is
/// unguarded, `<` holds the `n >= 0` frame, `+` holds the whole `if` condition.
/// Read from the wire, not the reader: contract-ir reader defect A (QSL-2776
/// chain) refuses the `else true` branch's frame until it is fixed.
#[trace("FR-825-AC-2", "FR-062-AC-17")]
#[test]
fn nested_guards_share_every_prefix_in_one_table() {
    let (_, package) = crate::emit::tests::package_from_text(
        "function g using v(p: Int[0, 10]): Boolean pure { true }\n\
         function f using v(n: Integer): Boolean pure { if n >= 0 and n < 10 then g(n + 1) else true }",
    );
    let emission = emit_checked(&package, crate::EmitLimits::fixture()).expect("emits");
    let (frames, terminals) = wire_frames(&emission);
    assert_eq!(
        frames,
        [(None, true), (None, true)],
        "the `if` condition and the `n >= 0` read of `and`, both outermost"
    );
    assert_eq!(
        terminals,
        [None, Some(0), Some(1)],
        "`>=`, then `+` and `<` each one frame"
    );
}

/// FSpec FR-825 package order over hand-built rows: claim key first, then
/// role spelling, ordinal and capability spelling on ties. The `NodeKey`
/// order used for the claim equals the order of the structural ids the reader
/// compares (`CheckedNodeId`'s order), including nested operand keys and
/// ordinals.
#[trace("FR-940-AC-1")]
#[test]
fn package_order_sorts_by_claim_key_then_role_ordinal_and_kind() {
    use filament_contract_model::{
        CheckedNodeId, CheckedOccurrence, CheckedOccurrenceRole, RequirementExtent,
        RequirementItemRow,
    };
    use filament_exact::{NodeKey, PathSegment};

    let text = |value: &str| PathSegment::Text(value.to_owned());
    let leaf = |name: &str, ordinal| NodeKey::new("p", vec![text(name)], ordinal).unwrap();
    let nested =
        |inner: NodeKey| NodeKey::new("p", vec![text("b"), PathSegment::Node(inner)], 0).unwrap();
    let keys = [
        nested(leaf("z", 0)),
        leaf("b", 1),
        nested(leaf("a", 0)),
        leaf("b", 0),
        NodeKey::new("o", vec![text("z")], 0).unwrap(),
        NodeKey::new("p", vec![text("b"), text("c")], 0).unwrap(),
    ];
    let ids = fsl_semantics::check::wire_id::StructuralIds::default();
    let mut by_key = keys.to_vec();
    by_key.sort();
    let mut by_id = keys.to_vec();
    by_id.sort_by_cached_key(|key| crate::emit::node_id(&ids, key));
    assert_eq!(
        by_key, by_id,
        "NodeKey order is the reader's structural-id order"
    );

    let claim = CheckedNodeId {
        package: "p".into(),
        path: Vec::new(),
        ordinal: 0,
    };
    let row = |role, ordinal, kind| RequirementItemRow {
        claim: claim.clone(),
        occurrence: CheckedOccurrence { role, ordinal },
        kind,
        extent: RequirementExtent::Bounded,
        record: None,
    };
    let tie = leaf("b", 0);
    let first = leaf("a", 0);
    let rows = vec![
        (
            tie.clone(),
            row(
                CheckedOccurrenceRole::Expression,
                1,
                CapabilityKind::ValueValidity,
            ),
        ),
        (
            tie.clone(),
            row(
                CheckedOccurrenceRole::Expression,
                0,
                CapabilityKind::ValueValidity,
            ),
        ),
        (
            tie.clone(),
            row(
                CheckedOccurrenceRole::Claim,
                3,
                CapabilityKind::TemporalSatisfaction,
            ),
        ),
        (
            tie.clone(),
            row(
                CheckedOccurrenceRole::Expression,
                0,
                CapabilityKind::OperationContract,
            ),
        ),
        (
            first,
            row(
                CheckedOccurrenceRole::Generated,
                9,
                CapabilityKind::ValueValidity,
            ),
        ),
    ];
    let ordered = super::package_order(rows)
        .into_iter()
        .map(|row| (row.occurrence.role, row.occurrence.ordinal, row.kind))
        .collect::<Vec<_>>();
    assert_eq!(
        ordered,
        [
            (
                CheckedOccurrenceRole::Generated,
                9,
                CapabilityKind::ValueValidity
            ),
            (
                CheckedOccurrenceRole::Claim,
                3,
                CapabilityKind::TemporalSatisfaction
            ),
            (
                CheckedOccurrenceRole::Expression,
                0,
                CapabilityKind::OperationContract
            ),
            (
                CheckedOccurrenceRole::Expression,
                0,
                CapabilityKind::ValueValidity
            ),
            (
                CheckedOccurrenceRole::Expression,
                1,
                CapabilityKind::ValueValidity
            ),
        ]
    );
}

/// FSpec FR-251 (monitor-plan writer, FR-251-AC-55): when an owner-key rename
/// puts one owner's frames before another's, the writer moves the frames,
/// keeps each parent before its child, and re-points every record's terminal id
/// in its row, its body, its identity path and its source-map entry.
#[trace("FR-251-AC-55", "FR-062-AC-17")]
#[test]
fn reordering_frames_remaps_every_record_citation() {
    use filament_contract_model::{
        CheckedNodeId, CheckedNodePathSegment as Segment, CheckedOccurrence, CheckedOccurrenceRole,
        CheckedSemanticNodeV2, CheckedSourceMapEntry, CheckedTerm, LiteralKind, PathConditionFrame,
        PathConditionFrameId, RequirementExtent, RequirementItemRow,
        RequirementRecord as WireRecord,
    };

    let id = |name: &str, frame: Option<usize>| CheckedNodeId {
        package: "p".into(),
        path: [Segment::Name("claim".into()), Segment::Name(name.into())]
            .into_iter()
            .chain(frame.map(|_| Segment::Name("path_condition".into())))
            .chain(frame.map(|frame| Segment::Name(frame.to_string().into())))
            .chain([Segment::Name("roots".into())])
            .collect(),
        ordinal: 0,
    };
    let guard = |name: &str| CheckedNodeId {
        package: "p".into(),
        path: vec![Segment::Name(name.into())],
        ordinal: 0,
    };
    let frame = |parent: Option<usize>, guard_name: &str, holds| PathConditionFrame {
        parent: parent.map(PathConditionFrameId),
        guard: guard(guard_name),
        occurrence: 0,
        holds,
    };
    let mut frames = vec![
        frame(None, "a", true),
        frame(Some(0), "b", true),
        frame(None, "c", false),
    ];
    let record_node = |name: &str, cited: usize| CheckedSemanticNodeV2 {
        node_id: id(name, Some(cited)),
        schema_version: "v2".into(),
        node_tag: "claim".into(),
        semantic_form: "value_validity".into(),
        semantic_type: guard("boolean"),
        dependencies: Vec::new(),
        occurrences: vec![CheckedOccurrence {
            role: CheckedOccurrenceRole::Generated,
            ordinal: 0,
        }],
        recursion_group: None,
        declaration: None,
        owner: None,
        owner_model: None,
        check_mode: None,
        body: CheckedTerm::Aggregate {
            members: (0..6)
                .map(|member| CheckedTerm::Binding {
                    name: format!("m{member}").as_str().into(),
                    binder: None,
                    value: Box::new(CheckedTerm::Aggregate {
                        members: if member == 5 {
                            vec![CheckedTerm::Literal {
                                type_id: guard("integer"),
                                value_kind: LiteralKind::Integer,
                                value: crate::emit::checked_literal_json(&(cited as u64)).unwrap(),
                            }]
                        } else {
                            Vec::new()
                        },
                        temporal_profile: None,
                    }),
                })
                .collect(),
            temporal_profile: None,
        },
    };
    let mut nodes = vec![record_node("one", 1), record_node("two", 2)];
    let mut source_map = nodes
        .iter()
        .map(|node| CheckedSourceMapEntry {
            node_id: node.node_id.clone(),
            role: CheckedOccurrenceRole::Generated,
            ordinal: 0,
            regions: Vec::new(),
        })
        .collect::<Vec<_>>();
    let row = |node: &CheckedSemanticNodeV2, cited: usize| RequirementItemRow {
        claim: guard("claim"),
        occurrence: CheckedOccurrence {
            role: CheckedOccurrenceRole::Expression,
            ordinal: 0,
        },
        kind: CapabilityKind::ValueValidity,
        extent: RequirementExtent::Bounded,
        record: Some(WireRecord::ValueValidity(ValueValidityRecord {
            node: node.node_id.clone(),
            owner: guard("owner"),
            application: guard("claim"),
            occurrence: 0,
            result_bound: guard("integer"),
            conversion: None,
            path_condition_frame: Some(PathConditionFrameId(cited)),
            extent_roots: Vec::new(),
            objects: Vec::new(),
        })),
    };
    let mut rows = vec![row(&nodes[0], 1), row(&nodes[1], 2)];

    // The owner of `c` now sorts first: `c`, then `a`, then `a`'s child `b`.
    super::reorder_frames(
        &[1, 2, 0],
        &mut frames,
        &mut rows,
        &mut nodes,
        &mut source_map,
    )
    .expect("a parent stays before its child");

    assert_eq!(
        frames
            .iter()
            .map(|frame| (frame.parent, frame.guard.clone(), frame.holds))
            .collect::<Vec<_>>(),
        [
            (None, guard("c"), false),
            (None, guard("a"), true),
            (Some(PathConditionFrameId(1)), guard("b"), true),
        ],
        "the moved frames keep their guards and outcomes, and `b` follows `a`'s new id"
    );
    let cited = rows
        .iter()
        .map(|row| super::value_record(row).unwrap().path_condition_frame)
        .collect::<Vec<_>>();
    assert_eq!(
        cited,
        [Some(PathConditionFrameId(2)), Some(PathConditionFrameId(0))]
    );
    assert_eq!(
        nodes[0].node_id,
        id("one", Some(2)),
        "the identity path cites the new id"
    );
    assert_eq!(nodes[1].node_id, id("two", Some(0)));
    assert_eq!(source_map[0].node_id, nodes[0].node_id);
    assert_eq!(source_map[1].node_id, nodes[1].node_id);
    assert_eq!(
        super::value_record(&rows[0]).unwrap().node,
        nodes[0].node_id
    );
    for (node, expected) in nodes.iter().zip([2_u64, 0]) {
        let CheckedTerm::Aggregate { members, .. } = &node.body else {
            panic!("a record body is an aggregate");
        };
        let CheckedTerm::Binding { value, .. } = &members[5] else {
            panic!("the path condition is a binding");
        };
        let CheckedTerm::Aggregate { members, .. } = &**value else {
            panic!("the path condition is an aggregate");
        };
        let [CheckedTerm::Literal { value, .. }] = members.as_slice() else {
            panic!("one terminal frame id");
        };
        assert_eq!(value.as_u64(), Some(expected), "the body cites the new id");
    }

    // A frame moved before its parent is refused.
    let mut broken = vec![frame(None, "a", true), frame(Some(0), "b", true)];
    assert!(
        super::reorder_frames(&[1, 0], &mut broken, &mut rows, &mut nodes, &mut source_map)
            .is_err()
    );
}

/// FSpec FR-322 "Expression occurrence ordinals", FR-825-AC-11: two function
/// owners holding the structurally equal `x + 1` share one application node.
/// The owner walk gives the second owner's visit the second ordinal, so the
/// two records name distinct occurrences of that node and the strict reader,
/// which derives each record's path from the same walk, admits both.
#[trace("FR-825-AC-11", "FR-322", "FR-062-AC-17")]
#[test]
fn records_of_a_shared_application_follow_the_owner_walk() {
    let (_, _, admitted, records) = emitted_records(
        "function alpha using v(x: Int[0, 9]): Integer pure { x + 1 }\n\
         function beta using v(x: Int[0, 9]): Integer pure { x + 1 }\n",
    );
    assert_eq!(records.len(), 2, "one record per owner");
    assert_eq!(
        records[0].application, records[1].application,
        "the equal bodies share one application node"
    );
    let mut occurrences: Vec<u64> = records.iter().map(|record| record.occurrence).collect();
    occurrences.sort_unstable();
    assert_eq!(
        occurrences,
        [0, 1],
        "each owner's visit takes the next ordinal"
    );
    let application = admitted
        .resolved_graph()
        .nodes
        .iter()
        .find(|node| node.node_id == records[0].application)
        .expect("the application node is written");
    let expressions: Vec<u64> = application
        .occurrences
        .iter()
        .filter(|occurrence| {
            occurrence.role == filament_contract_model::CheckedOccurrenceRole::Expression
        })
        .map(|occurrence| occurrence.ordinal)
        .collect();
    assert_eq!(expressions, [0, 1]);
}

/// FSpec FR-322 "Expression occurrence ordinals", FSpec FR-825-AC-11: mutual
/// recursive functions reuse the same bounded decrement and guard shape.
/// Their guarded range records must still emit and pass strict admission.
#[trace("FSpec-FR-825-AC-11", "FSpec-FR-322", "FR-062-AC-17")]
#[test]
fn bounded_mutual_recursion_emits_guarded_range_records() {
    let declarations = "function a using v(n: Int[0, 9]): Integer pure decreases(n) { if n > 0 then b(n - 1) else 0 }\n\
         function b using v(n: Int[0, 9]): Integer pure decreases(n) { if n > 0 then a(n - 1) else 0 }\n";
    let (package, emission, admitted, records) = emitted_records(declarations);
    let wire: serde_json::Value = serde_json::from_slice(emission.package().bytes()).unwrap();
    let identities = WireGraph::of_package_value(&wire).expect("the emitted graph has identities");
    let subtraction_node = crate::emit::tests::typed_nodes(&emission)
        .iter()
        .find(|node| {
            matches!(
                &node.body,
                CheckedTerm::Application { operation, .. }
                    if operation.identity() == Some("filament.op.integer.sub")
            )
        })
        .expect("the shared decrement application is emitted")
        .node_id
        .clone();
    let subtraction_node = serde_json::to_value(subtraction_node).unwrap();
    let decrement_records: Vec<_> = records
        .iter()
        .filter(|record| serde_json::to_value(&record.application).unwrap() == subtraction_node)
        .collect();
    assert_eq!(decrement_records.len(), 2, "one decrement record per owner");
    assert_eq!(
        decrement_records[0].application,
        decrement_records[1].application
    );
    let owner_a = identities
        .wire_value(
            &package
                .graph()
                .function_identity("a")
                .expect("function a has an identity"),
        )
        .expect("function a is emitted");
    let owner_b = identities
        .wire_value(
            &package
                .graph()
                .function_identity("b")
                .expect("function b has an identity"),
        )
        .expect("function b is emitted");
    let record_a = decrement_records
        .iter()
        .find(|record| serde_json::to_value(&record.owner).unwrap() == owner_a)
        .expect("function a has a bounded decrement record");
    let record_b = decrement_records
        .iter()
        .find(|record| serde_json::to_value(&record.owner).unwrap() == owner_b)
        .expect("function b has a bounded decrement record");
    assert_eq!(record_a.application, record_b.application);
    assert_eq!(record_a.occurrence, 1, "a names its root-body visit");
    assert_eq!(record_b.occurrence, 3, "b names its root-body visit");

    for (name, record) in [("a", record_a), ("b", record_b)] {
        let source = package
            .graph()
            .requirements()
            .iter()
            .find(|(claim, source)| {
                source.owner() == Some(name)
                    && identities.wire_value(claim.node()).ok().as_ref()
                        == Some(&serde_json::to_value(&record.application).unwrap())
            })
            .map(|(_, source)| source)
            .expect("the owner has its checked requirement");
        assert_eq!(source.path_condition().len(), 1);
        let actual = chain(&admitted, record);
        assert_eq!(actual.len(), 1, "{name} has exactly one frame guard");
        assert!(actual[0].1, "{name}'s decrement is under the true branch");
        assert_eq!(actual, source_chain(&identities, source));
    }

    // The reader visits the shared application under b while walking a, then
    // under a while walking b. Those synthetic slots retain the source region
    // of the syntax they repeat and keep the root-owner claims at ordinals 1/3.
    let application = serde_json::to_value(&record_a.application).unwrap();
    let mut regions = BTreeMap::<u64, serde_json::Value>::new();
    for entry in wire["source_map"].as_array().expect("source map") {
        if entry["node_id"] == application && entry["role"] == "expression" {
            let ordinal = entry["ordinal"].as_u64().expect("expression ordinal");
            assert!(regions.insert(ordinal, entry["regions"].clone()).is_none());
        }
    }
    assert_eq!(regions.keys().copied().collect::<Vec<_>>(), [0, 1, 2, 3]);
    assert_eq!(
        regions[&0], regions[&3],
        "the transitive b slot clones b's region"
    );
    assert_eq!(
        regions[&1], regions[&2],
        "the transitive a slot clones a's region"
    );
    assert_ne!(
        regions[&0], regions[&1],
        "a and b retain distinct source regions"
    );
    let source_prefix = "language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.model.complete/v1\";\n";
    let source_a_start =
        source_prefix.len() as u64 + declarations.find("n - 1").expect("a decrement") as u64;
    let source_b_start =
        source_prefix.len() as u64 + declarations.rfind("n - 1").expect("b decrement") as u64;
    let region_start =
        |regions: &serde_json::Value| regions[0]["start"].as_u64().expect("source region start");
    assert_eq!(region_start(&regions[&1]), source_a_start);
    assert_eq!(region_start(&regions[&3]), source_b_start);
}

/// FSpec FR-322 and FSpec FR-825-AC-11: repeated structurally equal uses keep the
/// source region belonging to each lexical occurrence through transitive walks.
#[trace("FSpec-FR-825-AC-11", "FSpec-FR-322", "FR-062-AC-17")]
#[test]
fn mutual_recursion_preserves_repeated_syntax_owner_regions() {
    let declarations = "function a using v(n: Int[0, 9]): Integer pure decreases(n) { if n > 0 then b(n - 1) + b(n - 1) else 0 }\n\
         function b using v(n: Int[0, 9]): Integer pure decreases(n) { if n > 0 then a(n - 1) + a(n - 1) else 0 }\n";
    let (package, emission, admitted, records) = emitted_records(declarations);
    let wire: serde_json::Value = serde_json::from_slice(emission.package().bytes()).unwrap();
    let identities = WireGraph::of_package_value(&wire).expect("the emitted graph has identities");
    let subtraction_node = crate::emit::tests::typed_nodes(&emission)
        .iter()
        .find(|node| {
            matches!(
                &node.body,
                CheckedTerm::Application { operation, .. }
                    if operation.identity() == Some("filament.op.integer.sub")
            )
        })
        .expect("the shared decrement application is emitted")
        .node_id
        .clone();
    let subtraction_node = serde_json::to_value(subtraction_node).unwrap();
    let decrements: Vec<_> = records
        .iter()
        .filter(|record| serde_json::to_value(&record.application).unwrap() == subtraction_node)
        .collect();
    assert_eq!(
        decrements.len(),
        4,
        "both source uses survive for both owners"
    );

    let owner_a = identities
        .wire_value(
            &package
                .graph()
                .function_identity("a")
                .expect("function a has an identity"),
        )
        .expect("function a is emitted");
    let owner_b = identities
        .wire_value(
            &package
                .graph()
                .function_identity("b")
                .expect("function b has an identity"),
        )
        .expect("function b is emitted");
    let mut a_ordinals: Vec<_> = decrements
        .iter()
        .filter(|record| serde_json::to_value(&record.owner).unwrap() == owner_a)
        .map(|record| record.occurrence)
        .collect();
    let mut b_ordinals: Vec<_> = decrements
        .iter()
        .filter(|record| serde_json::to_value(&record.owner).unwrap() == owner_b)
        .map(|record| record.occurrence)
        .collect();
    a_ordinals.sort_unstable();
    b_ordinals.sort_unstable();
    assert_eq!(
        a_ordinals,
        [2, 5],
        "a's claims name its two root-body visits"
    );
    assert_eq!(
        b_ordinals,
        [8, 11],
        "b's claims name its two root-body visits"
    );

    for (name, owner) in [("a", owner_a), ("b", owner_b)] {
        let decrement_application = serde_json::to_value(&decrements[0].application).unwrap();
        let source = package
            .graph()
            .requirements()
            .iter()
            .find(|(claim, source)| {
                source.owner() == Some(name)
                    && identities.wire_value(claim.node()).ok().as_ref()
                        == Some(&decrement_application)
            })
            .map(|(_, source)| source)
            .expect("the owner has a checked decrement requirement");
        assert_eq!(source.path_condition().len(), 1);
        for record in decrements
            .iter()
            .filter(|record| serde_json::to_value(&record.owner).unwrap() == owner)
        {
            let actual = chain(&admitted, record);
            assert_eq!(actual.len(), 1, "{name} has one guard per decrement");
            assert!(actual[0].1, "{name}'s decrement is under the true branch");
            assert_eq!(actual, source_chain(&identities, source));
        }
    }

    let application = serde_json::to_value(&decrements[0].application).unwrap();
    let mut regions = BTreeMap::<u64, serde_json::Value>::new();
    for entry in wire["source_map"].as_array().expect("source map") {
        if entry["node_id"] == application && entry["role"] == "expression" {
            let ordinal = entry["ordinal"].as_u64().expect("expression ordinal");
            assert!(regions.insert(ordinal, entry["regions"].clone()).is_none());
        }
    }
    assert_eq!(
        regions.keys().copied().collect::<Vec<_>>(),
        (0..12).collect::<Vec<_>>()
    );
    let source_prefix = "language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.model.complete/v1\";\n";
    let source_starts: Vec<u64> = declarations
        .match_indices("n - 1")
        .map(|(start, _)| source_prefix.len() as u64 + start as u64)
        .collect();
    assert_eq!(source_starts.len(), 4);
    let region_start =
        |regions: &serde_json::Value| regions[0]["start"].as_u64().expect("source region start");
    let actual_starts: Vec<u64> = (0..12)
        .map(|ordinal| region_start(&regions[&ordinal]))
        .collect();
    assert_eq!(
        actual_starts,
        [
            source_starts[2],
            source_starts[3],
            source_starts[0],
            source_starts[2],
            source_starts[3],
            source_starts[1],
            source_starts[0],
            source_starts[1],
            source_starts[2],
            source_starts[0],
            source_starts[1],
            source_starts[3],
        ],
        "synthetic slots copy the matching lexical occurrence's region"
    );
}
