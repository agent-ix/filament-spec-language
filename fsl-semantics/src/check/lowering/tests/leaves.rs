// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-415 step 9 (FR-093-AC-11): FR-093's text-leaf
//! walk over declared records, unions and recursion groups.

use super::*;
use filament_semantic_value::checking::CheckMode;

fn text(max: u64, profile: TextProfile) -> ValueType {
    ValueType::Text(TextType::new(0, max, profile).unwrap())
}

fn handle(label: &str) -> NodeKey {
    fixture_key(label)
}

fn record(name: &str, fields: Vec<FieldDeclaration>) -> CompositeDeclaration {
    CompositeDeclaration::new(handle(name), name, CompositeShape::Record(fields))
}

fn required(name: &str, value_type: ValueType) -> FieldDeclaration {
    FieldDeclaration::new(name, value_type, Presence::Required)
}

fn optional(name: &str, value_type: ValueType) -> FieldDeclaration {
    FieldDeclaration::new(name, value_type, Presence::Optional)
}

/// `record Node { label: Text[0, 8; binary-utf8]; next: Node?; }`.
fn node_record() -> CompositeDeclaration {
    record(
        "Node",
        vec![
            required("label", text(8, TextProfile::BinaryUtf8)),
            optional("next", ValueType::Composite(handle("Node"))),
        ],
    )
}

fn named(name: &str) -> TypeForm {
    TypeForm::name(name, SPAN)
}

/// FR-092-AC-12: a declared node id comes from the declaration path and
/// ordinal, even when the checker receives a different caller handle.
#[trace("FR-092-AC-12", "TC-413")]
#[test]
fn declared_point_identity_uses_its_path_not_its_caller_handle() {
    let caller_handle = NodeKey::new(
        "fixture",
        vec![filament_exact::PathSegment::Text(
            "caller-handle".to_owned(),
        )],
        0,
    )
    .expect("a distinct caller handle");
    let point = CompositeDeclaration::new(
        caller_handle.clone(),
        "Point",
        CompositeShape::Record(vec![required("x", ValueType::Integer)]),
    );
    let checked = package(vec![point], Vec::new())
        .check(
            CheckMode::Linked,
            CheckingLimits::default(),
            &crate::check::test_support::ZeroRewardEvaluator,
        )
        .expect("Point checks");
    let declared = checked
        .semantic_graph()
        .nodes()
        .find(|node| {
            node.semantic_form() == "record"
                && preimage(node)["declaration"]["qualified_name"] == json!(["Point"])
        })
        .expect("the declared Point node");
    assert_eq!(declared.key().package(), "fixture");
    assert_eq!(
        declared.key().path(),
        [filament_exact::PathSegment::Text("Point".to_owned())]
    );
    assert_eq!(declared.key().ordinal(), 0);
    assert_ne!(declared.key(), caller_handle);
}

/// `function <name>(<left>: T, <right>: T): Boolean { <left> = <right> }`.
fn equality(name: &str, (left, right): (&str, &str), operand: TypeForm) -> FunctionDeclaration {
    function(
        name,
        &[(left, operand.clone()), (right, operand)],
        boolean(),
        None,
        binary(BinaryOperator::Equal, name_expr(left), name_expr(right)),
    )
}

fn record_a() -> CompositeDeclaration {
    record(
        "A",
        vec![
            required("name", text(8, TextProfile::BinaryUtf8)),
            optional("b", ValueType::Composite(handle("B"))),
        ],
    )
}

fn record_b() -> CompositeDeclaration {
    record(
        "B",
        vec![
            required("tag", text(4, TextProfile::Nfc)),
            optional("a", ValueType::Composite(handle("A"))),
        ],
    )
}

/// `eq`, `has`, `eqa` and `eqo`: equality over a recursive record, a
/// sequence membership test, equality over a mutually recursive record, and
/// equality over `Option<Node>`.
fn recursive_functions() -> Vec<FunctionDeclaration> {
    let nodes = TypeForm::collection(CollectionKind::Sequence, SPAN)
        .with_arguments(vec![named("Node")])
        .with_bounds(vec!["0".into(), "3".into()]);
    let option_node =
        TypeForm::builtin(BuiltinType::Option, SPAN).with_arguments(vec![named("Node")]);
    vec![
        equality("eq", ("a", "b"), named("Node")),
        function(
            "has",
            &[("s", nodes), ("b", named("Node"))],
            boolean(),
            None,
            Expression::contains(name_expr("s"), name_expr("b")),
        ),
        equality("eqa", ("x", "y"), named("A")),
        equality("eqo", ("a", "b"), option_node),
    ]
}

/// A package declaring `records` and holding `functions`.
fn package(
    records: Vec<CompositeDeclaration>,
    functions: Vec<FunctionDeclaration>,
) -> PackageDeclarations {
    PackageDeclarations {
        types: TypeEnvironment::new(records, []).expect("FR-143 admits the records"),
        functions,
        ..PackageDeclarations::new(
            fixture_source(),
            "fixture",
            fsl_foundation::IdentityLimits::default(),
        )
    }
}

/// The leaf paths of the one structural equality of `records`' `R`-typed
/// `eq`, each joined by `/`, with its mode or `recursion`.
fn equality_leaves(records: Vec<CompositeDeclaration>, compared: &str) -> Vec<String> {
    let checked = package(records, vec![equality("eq", ("a", "b"), named(compared))])
        .check(
            CheckMode::Linked,
            CheckingLimits::default(),
            &crate::check::test_support::ZeroRewardEvaluator,
        )
        .unwrap_or_else(|refusals| panic!("eq over {compared} checks: {refusals:?}"));
    let node = application(checked.semantic_graph(), "filament.op.structural.eq");
    node["body"]["operation"]["leaves"]
        .as_array()
        .expect("leaves")
        .iter()
        .map(|leaf| {
            let path: Vec<&str> = leaf["path"]
                .as_array()
                .expect("a path")
                .iter()
                .map(|segment| segment.as_str().expect("a segment"))
                .collect();
            let mode = leaf["mode"]["value"].as_str().unwrap_or("-");
            format!("{} ({mode})", path.join("/"))
        })
        .collect()
}

fn union_comparison_leaf_json(
    records: Vec<CompositeDeclaration>,
    compared: TypeForm,
    operator: BinaryOperator,
    operation: &str,
) -> Json {
    let comparison = function(
        "compare",
        &[("a", compared.clone()), ("b", compared)],
        boolean(),
        None,
        binary(operator, name_expr("a"), name_expr("b")),
    );
    let checked = package(records, vec![comparison])
        .check(
            CheckMode::Linked,
            CheckingLimits::default(),
            &crate::check::test_support::ZeroRewardEvaluator,
        )
        .unwrap_or_else(|refusals| panic!("union equality checks: {refusals:?}"));
    application(checked.semantic_graph(), operation)["body"]["operation"]["leaves"].clone()
}

fn assert_union_comparison_leaves(
    records: Vec<CompositeDeclaration>,
    compared: TypeForm,
    expected: Json,
) {
    assert_eq!(
        union_comparison_leaf_json(
            records.clone(),
            compared.clone(),
            BinaryOperator::Equal,
            "filament.op.structural.eq",
        ),
        expected,
        "structural equality leaves"
    );
    assert_eq!(
        union_comparison_leaf_json(
            records,
            compared,
            BinaryOperator::NotEqual,
            "filament.op.structural.ne",
        ),
        expected,
        "structural inequality leaves"
    );
}

fn union_collection_type(kind: CollectionKind) -> TypeForm {
    TypeForm::collection(kind, SPAN)
        .with_arguments(vec![named("Choice")])
        .with_bounds(vec!["0".into(), "4".into()])
}

fn collection_leaf_json(
    declarations: Vec<CompositeDeclaration>,
    function_declaration: FunctionDeclaration,
    operation: &str,
) -> Json {
    let checked = package(declarations, vec![function_declaration])
        .check(
            CheckMode::Linked,
            CheckingLimits::default(),
            &crate::check::test_support::ZeroRewardEvaluator,
        )
        .unwrap_or_else(|refusals| panic!("collection expression checks: {refusals:?}"));
    application(checked.semantic_graph(), operation)["body"]["operation"]["leaves"].clone()
}

/// FR-093-AC-24: collection equality walks the collection's element with
/// `inner`; contains and result-inner start at the selected element itself.
#[trace("FR-093-AC-24", "TC-415")]
#[test]
fn collection_leaf_sources_follow_their_catalogued_paths() {
    let text_definition = json!({
        "authority": "agent-ix",
        "identity": "filament.value.text.unicode-17.0.0/v1"
    });
    let leaf = |path: &[&str], profile: &str| {
        json!({
            "path": path,
            "laws": [{"role":"text_profile", "definition": text_definition}],
            "mode": {"kind":"text_profile", "value":profile}
        })
    };
    let expected_inner = json!([
        leaf(&["inner", "member:Right", "position:0"], "binary-utf8"),
        leaf(&["inner", "member:Right", "position:1"], "nfc"),
        leaf(&["inner", "member:Left", "position:0"], "nfc"),
    ]);
    let expected_element = json!([
        leaf(&["member:Right", "position:0"], "binary-utf8"),
        leaf(&["member:Right", "position:1"], "nfc"),
        leaf(&["member:Left", "position:0"], "nfc"),
    ]);
    let choice = || {
        union_declaration(
            "Choice",
            vec![
                (
                    "Right",
                    vec![text(8, TextProfile::BinaryUtf8), text(8, TextProfile::Nfc)],
                ),
                ("Left", vec![text(8, TextProfile::Nfc)]),
                ("Empty", vec![]),
            ],
        )
    };
    for kind in [
        CollectionKind::Set,
        CollectionKind::Bag,
        CollectionKind::OrderedSet,
    ] {
        let collection = union_collection_type(kind);
        assert_eq!(
            union_comparison_leaf_json(
                vec![choice()],
                collection.clone(),
                BinaryOperator::Equal,
                "filament.op.structural.eq",
            ),
            expected_inner,
            "{kind:?} equality visits each Choice path through collection inner"
        );
        let contains = function(
            "contains_choice",
            &[("items", collection.clone()), ("item", named("Choice"))],
            boolean(),
            None,
            Expression::contains(name_expr("items"), name_expr("item")),
        );
        assert_eq!(
            collection_leaf_json(vec![choice()], contains, "filament.op.collection.contains"),
            expected_element,
            "{kind:?} contains selects the element path without inner"
        );
        let constructor = match kind {
            CollectionKind::Set => "filament.op.collection.set",
            CollectionKind::Bag => "filament.op.collection.bag",
            CollectionKind::OrderedSet => "filament.op.collection.ordered_set",
            CollectionKind::Sequence => unreachable!(),
        };
        let build_collection = function(
            "build_collection",
            &[("item", named("Choice"))],
            collection,
            None,
            Expression::collection(kind, vec![name_expr("item")]),
        );
        assert_eq!(
            collection_leaf_json(vec![choice()], build_collection, constructor),
            expected_element,
            "{kind:?} result_inner selects Choice with no prefix"
        );
    }
    let sequence = union_collection_type(CollectionKind::Sequence);
    let sequence_equality = union_comparison_leaf_json(
        vec![choice()],
        sequence.clone(),
        BinaryOperator::Equal,
        "filament.op.structural.eq",
    );
    assert_eq!(sequence_equality, expected_inner);
    let sequence_constructor = function(
        "build_sequence",
        &[("item", named("Choice"))],
        sequence,
        None,
        Expression::collection(CollectionKind::Sequence, vec![name_expr("item")]),
    );
    assert_eq!(
        collection_leaf_json(
            vec![choice()],
            sequence_constructor,
            "filament.op.collection.sequence"
        ),
        json!([]),
        "Sequence's result_inner has no leaves"
    );
}

/// TC-415 step 9 (FR-093-AC-11): an optional field's leaf passes through
/// `inner`, as its `Option` twin's does; a recursion reaching a text type
/// ends in `recursion:d`, `d` counting the segments where the walk entered
/// the composite; one that reaches none adds no leaf.
#[trace("FR-093-AC-11", "TC-415")]
#[test]
fn leaf_paths_follow_fr093_text_leaves() {
    let t64 = || text(64, TextProfile::Nfc);
    assert_eq!(
        equality_leaves(vec![record("R", vec![optional("t", t64())])], "R"),
        ["field:t/inner (nfc)"]
    );
    assert_eq!(
        equality_leaves(
            vec![record("S", vec![required("t", ValueType::option(t64()))])],
            "S"
        ),
        ["field:t/inner (nfc)"]
    );
    assert_eq!(
        equality_leaves(vec![record("W", vec![required("t", t64())])], "W"),
        ["field:t (nfc)"]
    );
    let tree2 = record(
        "Tree2",
        vec![
            required("label", text(8, TextProfile::BinaryUtf8)),
            required(
                "kids",
                sequence(ValueType::Composite(handle("Tree2")), Some((0, 3))),
            ),
        ],
    );
    assert_eq!(
        equality_leaves(vec![tree2], "Tree2"),
        [
            "field:label (binary-utf8)",
            "field:kids/inner/recursion:0 (-)"
        ]
    );
    let two = record(
        "Two",
        vec![
            required("x", ValueType::Composite(handle("Node"))),
            required("y", ValueType::Composite(handle("Node"))),
        ],
    );
    assert_eq!(
        equality_leaves(vec![node_record(), two], "Two"),
        [
            "field:x/field:label (binary-utf8)",
            "field:x/field:next/inner/recursion:1 (-)",
            "field:y/field:label (binary-utf8)",
            "field:y/field:next/inner/recursion:1 (-)",
        ]
    );
    // FR-092's `List`, which reaches no text type, carries no leaves, in
    // equality and in `contains`.
    let list = || {
        record(
            "List",
            vec![optional("next", ValueType::Composite(handle("List")))],
        )
    };
    assert!(equality_leaves(vec![list()], "List").is_empty());
    let lists = TypeForm::collection(CollectionKind::Sequence, SPAN)
        .with_arguments(vec![named("List")])
        .with_bounds(vec!["0".into(), "3".into()]);
    let has = function(
        "has",
        &[("s", lists), ("b", named("List"))],
        boolean(),
        None,
        Expression::contains(name_expr("s"), name_expr("b")),
    );
    let checked = package(vec![list()], vec![has])
        .check(
            CheckMode::Linked,
            CheckingLimits::default(),
            &crate::check::test_support::ZeroRewardEvaluator,
        )
        .expect("contains over List checks");
    let contains = application(checked.semantic_graph(), "filament.op.collection.contains");
    assert_eq!(contains["body"]["operation"]["leaves"], json!([]));
}

/// A union `name` declaring `members`, each with its payload types.
fn union_declaration(name: &str, members: Vec<(&str, Vec<ValueType>)>) -> CompositeDeclaration {
    CompositeDeclaration::new(
        handle(name),
        name,
        CompositeShape::Union(
            members
                .into_iter()
                .map(|(member, payload)| {
                    filament_semantic_value::declaration::UnionMemberDeclaration::resolved(
                        filament_exact::Identifier::new(member).expect("a member identifier"),
                        payload,
                    )
                })
                .collect(),
        ),
    )
}

/// FR-093-AC-23: all seven union vectors use declaration and payload order,
/// including text-law identity, optional paths and recursion depth.
#[trace("FR-093-AC-23", "TC-415")]
#[test]
fn union_leaf_vectors_match_the_complete_ordered_contract() {
    let text_definition = json!({
        "authority": "agent-ix",
        "identity": "filament.value.text.unicode-17.0.0/v1"
    });
    let leaf = |path: &[&str], profile: &str| {
        json!({
            "path": path,
            "laws": [{"role":"text_profile", "definition": text_definition}],
            "mode": {"kind":"text_profile", "value":profile}
        })
    };
    let recursion = |path: &[&str]| json!({"path":path,"laws":[],"mode":null});
    let nfc = || text(8, TextProfile::Nfc);
    let binary = || text(8, TextProfile::BinaryUtf8);
    let empty = || union_declaration("Empty", vec![("None", vec![]), ("Done", vec![])]);
    assert_union_comparison_leaves(vec![empty()], named("Empty"), json!([]));

    let choice = || {
        union_declaration(
            "Choice",
            vec![
                ("Right", vec![binary(), nfc()]),
                ("Left", vec![nfc()]),
                ("Empty", vec![]),
            ],
        )
    };
    let choice_expected = json!([
        leaf(&["member:Right", "position:0"], "binary-utf8"),
        leaf(&["member:Right", "position:1"], "nfc"),
        leaf(&["member:Left", "position:0"], "nfc"),
    ]);
    assert_union_comparison_leaves(vec![choice()], named("Choice"), choice_expected.clone());
    let renamed = union_declaration(
        "Choice",
        vec![
            ("Right", vec![binary(), nfc()]),
            ("Other", vec![nfc()]),
            ("Empty", vec![]),
        ],
    );
    assert_union_comparison_leaves(
        vec![renamed],
        named("Choice"),
        json!([
            leaf(&["member:Right", "position:0"], "binary-utf8"),
            leaf(&["member:Right", "position:1"], "nfc"),
            leaf(&["member:Other", "position:0"], "nfc"),
        ]),
    );
    let reordered = union_declaration(
        "Choice",
        vec![
            ("Left", vec![nfc()]),
            ("Right", vec![binary(), nfc()]),
            ("Empty", vec![]),
        ],
    );
    assert_union_comparison_leaves(
        vec![reordered],
        named("Choice"),
        json!([
            leaf(&["member:Left", "position:0"], "nfc"),
            leaf(&["member:Right", "position:0"], "binary-utf8"),
            leaf(&["member:Right", "position:1"], "nfc"),
        ]),
    );
    let swapped = union_declaration(
        "Choice",
        vec![
            ("Right", vec![nfc(), binary()]),
            ("Left", vec![nfc()]),
            ("Empty", vec![]),
        ],
    );
    assert_union_comparison_leaves(
        vec![swapped],
        named("Choice"),
        json!([
            leaf(&["member:Right", "position:0"], "nfc"),
            leaf(&["member:Right", "position:1"], "binary-utf8"),
            leaf(&["member:Left", "position:0"], "nfc"),
        ]),
    );

    let source_case_leaves = |arms: &str| {
        let source = format!(
            "language \"ix:native\" edition \"1-draft\"; profile v = \"filament.value.complete/v1\"; \
             union Choice {{ Right(Text[0, 8; binary-utf8], Text[0, 8; nfc]), Left(Text[0, 8; nfc]), Empty }} \
             function f using v(a: Choice, b: Choice): Boolean pure {{ case a {{ {arms} }} }}"
        );
        let parsed = fsl_cst::parse(
            SourceIdentity::new("union-vector", "source-arm-order"),
            "union.native",
            source.as_bytes(),
            fsl_cst::Limits::default(),
        )
        .expect("S1 reads the source");
        assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
        let unit = fsl_forms::build_unit(&parsed).expect("S2 builds the source forms");
        let checked = PackageDeclarations::assemble(
            parsed.source().reference().clone(),
            "fixture".to_owned(),
            unit,
            crate::model::intake::SelectedModels::default(),
            Vec::new(),
        )
        .expect("the source assembles")
        .check(
            CheckMode::Linked,
            CheckingLimits::default(),
            &crate::check::test_support::ZeroRewardEvaluator,
        )
        .expect("the source checks");
        application(checked.semantic_graph(), "filament.op.structural.eq")["body"]["operation"]
            ["leaves"]
            .clone()
    };
    assert_eq!(
        source_case_leaves("Right(x, y): a = b; Left(x): a = b; Empty: a = b;"),
        choice_expected
    );
    assert_eq!(
        source_case_leaves("Empty: a = b; Left(x): a = b; Right(x, y): a = b;"),
        choice_expected
    );

    let wrapped = union_declaration(
        "Wrapped",
        vec![
            (
                "Item",
                vec![ValueType::option(sequence(nfc(), Some((0, 2))))],
            ),
            ("Empty", vec![]),
        ],
    );
    assert_union_comparison_leaves(
        vec![wrapped],
        TypeForm::builtin(BuiltinType::Option, SPAN).with_arguments(vec![named("Wrapped")]),
        json!([leaf(
            &["inner", "member:Item", "position:0", "inner", "inner"],
            "nfc"
        )]),
    );
    let chain = || {
        union_declaration(
            "Chain",
            vec![
                ("End", vec![nfc()]),
                (
                    "Next",
                    vec![ValueType::option(ValueType::Composite(handle("Chain")))],
                ),
            ],
        )
    };
    assert_union_comparison_leaves(
        vec![chain()],
        named("Chain"),
        json!([
            leaf(&["member:End", "position:0"], "nfc"),
            recursion(&["member:Next", "position:0", "inner", "recursion:0"])
        ]),
    );
    assert_union_comparison_leaves(
        vec![chain()],
        TypeForm::builtin(BuiltinType::Option, SPAN).with_arguments(vec![named("Chain")]),
        json!([
            leaf(&["inner", "member:End", "position:0"], "nfc"),
            recursion(&["inner", "member:Next", "position:0", "inner", "recursion:1"])
        ]),
    );
    let silent = union_declaration(
        "Silent",
        vec![
            ("End", vec![]),
            (
                "Next",
                vec![ValueType::option(ValueType::Composite(handle("Silent")))],
            ),
        ],
    );
    assert_union_comparison_leaves(vec![silent], named("Silent"), json!([]));
    let link = union_declaration(
        "Link",
        vec![
            ("End", vec![binary()]),
            ("More", vec![ValueType::Composite(handle("Cell"))]),
        ],
    );
    let cell = record(
        "Cell",
        vec![
            required("tag", nfc()),
            optional("next", ValueType::Composite(handle("Link"))),
        ],
    );
    assert_union_comparison_leaves(
        vec![link, cell],
        named("Link"),
        json!([
            leaf(&["member:End", "position:0"], "binary-utf8"),
            leaf(&["member:More", "position:0", "field:tag"], "nfc"),
            recursion(&[
                "member:More",
                "position:0",
                "field:next",
                "inner",
                "recursion:0"
            ])
        ]),
    );
}

/// FSpec FR-440's "Structural leaves": every declared member is an anchor,
/// its text leaves at `member:<Name>` then `position:<i>`, `i` the index in
/// the member's payload; a cycle through a union is a recursion leaf when a
/// text type is reachable and adds nothing when none is.
#[trace("FR-093-AC-11", "TC-415", "FSpec-FR-440-AC-8")]
#[test]
fn union_leaf_paths_follow_member_then_position() {
    let t8 = || text(8, TextProfile::Nfc);
    let label = union_declaration(
        "Label",
        vec![
            ("Named", vec![t8()]),
            ("Tagged", vec![ValueType::Integer, t8()]),
            ("Empty", vec![]),
        ],
    );
    assert_eq!(
        equality_leaves(vec![label], "Label"),
        [
            "member:Named/position:0 (nfc)",
            "member:Tagged/position:1 (nfc)"
        ]
    );
    let chain = union_declaration(
        "Chain",
        vec![
            (
                "Link",
                vec![
                    t8(),
                    ValueType::option(ValueType::Composite(handle("Chain"))),
                ],
            ),
            ("End", vec![]),
        ],
    );
    assert_eq!(
        equality_leaves(vec![chain], "Chain"),
        [
            "member:Link/position:0 (nfc)",
            "member:Link/position:1/inner/recursion:0 (-)"
        ]
    );
    let list = union_declaration(
        "List",
        vec![
            (
                "Cons",
                vec![
                    ValueType::Integer,
                    ValueType::option(ValueType::Composite(handle("List"))),
                ],
            ),
            ("Nil", vec![]),
        ],
    );
    assert!(equality_leaves(vec![list], "List").is_empty());
}

/// FR-093 "Text leaves" rule 4: a composite from which no text type is
/// reachable is not entered, so a cluster of twelve text-free records that
/// each hold an optional field of every other adds no leaf and no walk, and
/// the compared record's one text field is its only leaf.
#[trace("FR-093-AC-11", "TC-415")]
#[test]
fn a_text_free_recursive_cluster_is_not_walked() {
    let n = 12;
    let cluster: Vec<CompositeDeclaration> = (0..n)
        .map(|at| {
            record(
                &format!("R{at}"),
                (0..n)
                    .filter(|other| *other != at)
                    .map(|other| {
                        optional(
                            &format!("r{other}"),
                            ValueType::Composite(handle(&format!("R{other}"))),
                        )
                    })
                    .collect(),
            )
        })
        .collect();
    let mut records = cluster;
    records.push(record(
        "Top",
        vec![
            required("s", text(8, TextProfile::Nfc)),
            required("r", ValueType::Composite(handle("R0"))),
        ],
    ));
    // Under a small work budget too: each composite entered charges it.
    assert_eq!(equality_leaves(records, "Top"), ["field:s (nfc)"]);
}

/// `eq` over `compared`, one of `records`.
fn eq_over(
    records: Vec<CompositeDeclaration>,
    compared: &str,
    limits: CheckingLimits,
) -> Result<CheckedGraph, Vec<CheckRefusal>> {
    package(records, vec![equality("eq", ("a", "b"), named(compared))]).check(
        CheckMode::Linked,
        limits,
        &crate::check::test_support::ZeroRewardEvaluator,
    )
}

fn eq_over_node(limits: CheckingLimits) -> Result<CheckedGraph, Vec<CheckRefusal>> {
    eq_over(vec![node_record()], "Node", limits)
}

fn node_limit(nodes: u64) -> CheckingLimits {
    CheckingLimits::new(nodes)
}

fn work_limit(work: u64) -> CheckingLimits {
    CheckingLimits::default().with_work_budget(work)
}

/// The smallest node limit `checks` passes under.
fn smallest_limit(checks: impl Fn(u64) -> bool) -> u64 {
    let (mut refused, mut admitted) = (0_u64, 4096_u64);
    assert!(checks(admitted));
    while admitted - refused > 1 {
        let middle = refused + (admitted - refused) / 2;
        if checks(middle) {
            admitted = middle;
        } else {
            refused = middle;
        }
    }
    admitted
}

/// TC-415 step 9 (FR-093-AC-11): `eq` over `Node` charges its two leaves
/// to the node limit: under a limit that admits the package's nodes but not
/// also its two leaves, it refuses on the node limit and yields no node.
#[trace("FR-093-AC-11", "FR-093-AC-25", "TC-415")]
#[test]
fn a_leaf_walk_charges_the_node_limit() {
    let admitted = smallest_limit(|limit| eq_over_node(node_limit(limit)).is_ok());
    // The same `eq` over a record with no text leaf types the same nodes:
    // `Node`'s two leaves are the last two units.
    let plain = record(
        "Plain",
        vec![
            required("label", int(0, 9)),
            optional("next", ValueType::Composite(handle("Plain"))),
        ],
    );
    let typed =
        smallest_limit(|limit| eq_over(vec![plain.clone()], "Plain", node_limit(limit)).is_ok());
    assert_eq!(admitted, typed + 2);

    let exhausted = |refusals: Vec<CheckRefusal>, limit: u64| {
        assert!(
            matches!(
                *refusals[0].cause,
                CheckCause::ResourceExhausted(ref exceeded)
                    if exceeded.kind == CheckingLimitKind::Nodes && exceeded.limit == limit
            ),
            "{refusals:?}"
        );
        assert_eq!(refusals[0].cause.code().as_str(), "stage_limit_exceeded");
        assert_eq!(refusals[0].cause.cause(), Some("node-count-exceeded"));
    };
    for limit in [admitted - 1, typed] {
        exhausted(
            eq_over_node(node_limit(limit)).expect_err("the leaves pass the limit"),
            limit,
        );
    }
}

#[trace("FR-093-AC-25", "TC-415")]
#[test]
fn an_union_leaf_walk_charges_its_composite_and_full_path_bytes() {
    let union = || {
        union_declaration(
            "U",
            vec![("M", vec![text(8, TextProfile::Nfc)]), ("Empty", vec![])],
        )
    };
    package(vec![union()], vec![equality("eq", ("a", "b"), named("U"))])
        .check(
            CheckMode::Linked,
            work_limit(52),
            &crate::check::test_support::ZeroRewardEvaluator,
        )
        .expect("52 units admit the composite and full leaf path");
    let refusals = package(vec![union()], vec![equality("eq", ("a", "b"), named("U"))])
        .check(
            CheckMode::Linked,
            work_limit(51),
            &crate::check::test_support::ZeroRewardEvaluator,
        )
        .expect_err("the union composite and text leaf path each charge work");
    assert_eq!(refusals[0].cause.code().as_str(), "stage_limit_exceeded");
    assert_eq!(refusals[0].cause.cause(), Some("work-budget-exceeded"));
    assert!(matches!(
        refusals[0].cause.as_ref(),
        CheckCause::ResourceExhausted(ref limit)
            if limit.kind == CheckingLimitKind::WorkBudget
                && limit.limit == 51
                && limit.actual == 52
    ));
}

/// `n` records `R0` .. `R{n-1}`, each with a `label` text field and an
/// optional field of every other record: every record reaches a text type,
/// so FR-093's leaf list over `R0` holds one leaf per simple path through
/// the cluster, on the order of `(n - 1)!` leaves.
fn text_cluster(n: usize) -> Vec<CompositeDeclaration> {
    (0..n)
        .map(|at| {
            let mut fields = vec![required("label", text(8, TextProfile::Nfc))];
            fields.extend((0..n).filter(|other| *other != at).map(|other| {
                optional(
                    &format!("r{other}"),
                    ValueType::Composite(handle(&format!("R{other}"))),
                )
            }));
            record(&format!("R{at}"), fields)
        })
        .collect()
}

/// TC-423 steps 1 and 2 (NFR-011-M-1): at the default checking limits,
/// structural equality over a nine-record Text-reachable cluster refuses on
/// the node ceiling, naming the node limit kind and its default bound, and
/// yields no node; the six-record cluster's leaves fit.
#[trace("NFR-011-M-1", "TC-423")]
#[test]
fn a_text_reachable_cluster_refuses_on_the_default_node_ceiling() {
    let refusals = eq_over(text_cluster(9), "R0", CheckingLimits::default())
        .expect_err("the nine-record cluster's leaves pass the default node ceiling");
    assert_eq!(
        *refusals[0].cause,
        CheckCause::ResourceExhausted(Box::new(StageLimitCause {
            stage: CheckingStage::Typing,
            kind: CheckingLimitKind::Nodes,
            limit: 100_000,
            actual: 100_001,
            region: None,
        }))
    );
    assert_eq!(refusals[0].cause.code().as_str(), "stage_limit_exceeded");
    assert_eq!(refusals[0].cause.cause(), Some("node-count-exceeded"));
    eq_over(text_cluster(6), "R0", CheckingLimits::default())
        .expect("the six-record cluster checks at the default limits");
}

/// TC-423 step 3 (NFR-011-M-1 to NFR-011-M-4): a checked package and a
/// checked expression each record the ceilings they were checked under --
/// the defaults when the caller sets none, and a caller's ceilings as
/// given, above or below the defaults.
#[trace("NFR-011-M-1", "NFR-011-M-2", "NFR-011-M-3", "NFR-011-M-4", "TC-423")]
#[test]
fn a_checked_result_records_its_effective_limits() {
    let defaults = CheckingLimits::default();
    assert_eq!(
        (
            defaults.nodes(),
            defaults.input_bytes(),
            defaults.work_budget()
        ),
        (100_000, 16_777_216, 16_777_216)
    );
    let checked = eq_over_node(defaults).expect("eq over Node checks");
    assert_eq!(checked.effective_limits(), defaults);

    let raised = CheckingLimits::new(u64::MAX)
        .with_input_bytes(u64::MAX)
        .with_work_budget(u64::MAX);
    let lowered = node_limit(64)
        .with_work_budget(5_000)
        .with_input_bytes(4_096);
    for limits in [raised, lowered] {
        let checked = eq_over_node(limits).expect("eq over Node checks");
        assert_eq!(checked.effective_limits(), limits);
        let expression = checked
            .check_expression(
                vec![("x".to_owned(), ValueType::Boolean)],
                &name_expr("x"),
                None,
                CheckMode::Linked,
                limits,
            )
            .expect("a Boolean name checks");
        assert_eq!(expression.effective_limits(), limits);
    }
}

/// TC-423 step 3 (NFR-011-M-3, NFR-011-M-4): `CheckingLimits::new` sets the
/// node ceiling and keeps the default input-byte and work ceilings.
#[trace("NFR-011-M-3", "NFR-011-M-4", "TC-423")]
#[test]
fn new_keeps_the_default_byte_and_work_ceilings() {
    let limits = CheckingLimits::new(7);
    assert_eq!(
        (limits.nodes(), limits.input_bytes(), limits.work_budget()),
        (7, 16_777_216, 16_777_216)
    );
}

/// A `chain`-record chain of optional fields named `name` into a binary
/// tree of `levels` levels of optional fields, whose last level holds a
/// text field: `2^(levels - 1)` text leaves, each under a path of about
/// `2 * (chain + levels)` segments.
fn deep_wide(chain: usize, levels: usize, name: &str) -> Vec<CompositeDeclaration> {
    let link = |to: String| ValueType::Composite(handle(&to));
    let mut records: Vec<CompositeDeclaration> = (0..chain)
        .map(|at| {
            let next = if at + 1 < chain {
                format!("C{}", at + 1)
            } else {
                "T0".to_owned()
            };
            record(&format!("C{at}"), vec![optional(name, link(next))])
        })
        .collect();
    records.extend((0..levels).map(|level| {
        let fields = if level + 1 < levels {
            let next = format!("T{}", level + 1);
            vec![
                optional(&format!("{name}l"), link(next.clone())),
                optional(&format!("{name}r"), link(next)),
            ]
        } else {
            vec![required(name, text(8, TextProfile::Nfc))]
        };
        record(&format!("T{level}"), fields)
    }));
    records
}

/// TC-423 step 5 (NFR-011-M-4): a leaf list whose count fits the node
/// ceiling but whose paths are long -- 65,536 text leaves under 256-byte
/// field names, each path 35 segments -- refuses at the default ceilings
/// on the work budget, which each leaf's materialized key bytes are
/// charged to, and yields no node. (The chain is one record long: a
/// 30-record chain overflows a debug test thread's 2 MiB stack in
/// composite lowering, before any leaf walk; `fsl-bench`'s `deep-wide`
/// probe runs the reviewer's 46-record chain in release.)
#[trace("FR-093-AC-25", "NFR-011-M-4", "TC-423")]
#[test]
fn long_leaf_paths_refuse_on_the_default_work_budget() {
    let name = format!("n{}", "x".repeat(255));
    let refusals = eq_over(deep_wide(1, 17, &name), "C0", CheckingLimits::default())
        .expect_err("the leaves' key bytes pass the default work budget");
    assert!(
        matches!(
            *refusals[0].cause,
            CheckCause::ResourceExhausted(ref exceeded)
                if exceeded.stage == CheckingStage::Typing
                    && exceeded.kind == CheckingLimitKind::WorkBudget
                    && exceeded.limit == 16_777_216
        ),
        "{:?}",
        refusals[0]
    );
    assert_eq!(refusals[0].cause.code().as_str(), "stage_limit_exceeded");
    assert_eq!(refusals[0].cause.cause(), Some("work-budget-exceeded"));
    // The same shape with one-letter names and a shorter tree fits.
    eq_over(deep_wide(4, 5, "n"), "C0", CheckingLimits::default())
        .expect("sixteen short leaves check");
}

/// TC-415 step 8 (FR-093-AC-11): `eq`, `has`, `eqa` and `eqo` check over
/// recursive text-leaf records, and their nodes key the same with `A`
/// declared before `B` and after it.
#[trace("FR-093-AC-11", "TC-415")]
#[test]
fn recursive_text_leaf_functions_check_and_key_in_either_order() {
    let mut keys = Vec::new();
    for records in [
        vec![node_record(), record_a(), record_b()],
        vec![node_record(), record_b(), record_a()],
    ] {
        let checked = package(records, recursive_functions())
            .check(
                CheckMode::Linked,
                CheckingLimits::default(),
                &crate::check::test_support::ZeroRewardEvaluator,
            )
            .unwrap_or_else(|refusals| panic!("the recursive functions check: {refusals:?}"));
        keys.push(
            checked
                .semantic_graph()
                .nodes()
                .map(SemanticNode::key)
                .collect::<Vec<_>>(),
        );
    }
    assert_eq!(keys[0], keys[1], "declaring B before A gives the same keys");
}

/// FR-092 "Recursion groups" (FR-092-AC-11): `Option<Node>` is one node
/// wherever it is named, whichever function names it first, and the nodes key
/// the same under every order of the functions.
#[trace("FR-092-AC-11", "TC-413")]
#[test]
fn an_in_group_type_is_its_member_in_every_declaration_order() {
    let keys_of = |functions: Vec<FunctionDeclaration>| {
        let checked = package(vec![node_record(), record_a(), record_b()], functions)
            .check(
                CheckMode::Linked,
                CheckingLimits::default(),
                &crate::check::test_support::ZeroRewardEvaluator,
            )
            .unwrap_or_else(|refusals| panic!("the functions check: {refusals:?}"));
        let graph = checked.semantic_graph();
        let node_record = graph
            .nodes()
            .find(|node| {
                node.semantic_form() == "record"
                    && node.recursion().is_some()
                    && preimage(node)["declaration"]["qualified_name"] == json!(["Node"])
            })
            .expect("the graph holds record Node");
        let node_key = node_record.key().to_string();
        let options = graph
            .nodes()
            .filter(|node| {
                node.semantic_form() == "option"
                    && node.recursion().is_some()
                    && preimage(node)["body"]["members"][0]["target"]["digest"] == json!(node_key)
            })
            .count();
        assert_eq!(options, 1, "exactly one Option<Node> option node");
        graph.nodes().map(SemanticNode::key).collect::<Vec<_>>()
    };
    let eqo = || recursive_functions().remove(3);
    keys_of(vec![eqo()]);
    keys_of(vec![eqo(), recursive_functions().remove(0)]);
    let forward = keys_of(recursive_functions());
    let mut reversed = recursive_functions();
    reversed.reverse();
    assert_eq!(keys_of(reversed), forward);
    let mut rotated = recursive_functions();
    rotated.rotate_left(2);
    assert_eq!(keys_of(rotated), forward);
}
