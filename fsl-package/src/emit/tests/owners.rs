// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-093-AC-21 and AC-22 (TC-416 steps 12 and 13): the v2 emission writes each
//! structural node's `owner` onto the node under FR-322's presence rule, and
//! every package is read back through IR's v2 reader.

use filament_checked::key::DeclarationKey;
use filament_exact::{EffectiveId, Integer};
use filament_semantic_value::checking::CheckMode;
use filament_semantic_value::declaration::ObjectTypeDeclaration;
use fsl_semantics::check::{
    checked_dispatch_operation, AdmittedModel, DispatchRoot, OperationClauses,
};
use fsl_semantics::model::accounting::{ModelNormalizationLimits, NormalizationMeter};
use fsl_semantics::model::dispatch::GeneralizationClosure;
use fsl_semantics::model::domain_package::{
    DomainPackage, DomainPackageRecord, DomainPackageRef, Multiplicity, NativeValueType,
    ObjectTypeRecord, OperationEffect, OperationMemberRecord, OperationResult, ValueTypeRef,
};
use fsl_semantics::model::normalize::{normalize, NormalizeOutcome};

use super::*;

/// The owner kind FR-322's presence rule requires of a wire node.
fn required_kind(node: &Value) -> Option<&'static str> {
    let structural = node["body"]["term"] != "application";
    if !structural {
        None
    } else if node.get("declaration").is_some() {
        Some("source")
    } else if matches!(
        (node["node_tag"].as_str(), node["semantic_form"].as_str()),
        (
            Some("model"),
            Some("object_type" | "systems_interface" | "record_value_type" | "event_type")
        ) | (Some("relation"), Some("relationship"))
            | (Some("function"), _)
    ) {
        Some("model")
    } else {
        None
    }
}

/// Every owner check of FR-093-AC-21 over one emitted package: each node's
/// `owner` follows FR-322's presence rule, and `read` admits the package.
/// Returns the wire.
fn assert_owners(
    _package: &CheckedPackage,
    emission: &Emission,
    read: impl Fn(&Emission),
) -> Value {
    let wire = wire(emission);
    assert!(
        wire["identity_preimage"]
            .get("identity_projection")
            .is_none(),
        "the wire preimage carries no identity_projection (FR-322)"
    );
    for node in nodes(&wire) {
        if node["semantic_form"] == "model_root" {
            // A root is derived from its package identity, not lowered from
            // a checked node (FR-322-AC-134).
            assert_eq!(node["owner"]["kind"], "model_root");
            assert!(node["owner"]["identity"].is_string());
            assert!(node["owner"].get("digest").is_none());
            continue;
        }
        assert_eq!(
            node.get("owner").and_then(|owner| owner["kind"].as_str()),
            required_kind(node),
            "{}: FR-322's presence rule",
            node["node_id"]
        );
    }
    read(emission);
    wire
}

fn verified(emission: &Emission) {
    assert!(matches!(read_back(emission), Read::Verified { .. }));
}

fn owner_json(authority: &str, identity: &str) -> Value {
    json!({"kind": "source", "authority": authority, "identity": identity})
}

pub(super) fn node_where<'w>(wire: &'w Value, tag: &str, form: &str) -> Vec<&'w Value> {
    nodes(wire)
        .iter()
        .filter(|node| node["node_tag"] == tag && node["semantic_form"] == form)
        .collect()
}

/// `record P { x: Int[0, 9]; }` and `record Tree { kids: Sequence<Tree>[0, 3]; }`.
fn p_and_tree_types() -> TypeEnvironment {
    let tree = NodeKey::new("pkg", vec![PathSegment::Text("Tree".to_owned())], 0).unwrap();
    let kids = ValueType::collection(CollectionType::new(
        CollectionKind::Sequence,
        ValueType::Composite(tree.clone()),
        Some(CardinalityBound::new(0, 3).unwrap()),
    ));
    TypeEnvironment::new(
        [
            CompositeDeclaration::new(
                NodeKey::new("pkg", vec![PathSegment::Text("P".to_owned())], 0).unwrap(),
                "P",
                CompositeShape::Record(vec![FieldDeclaration::new(
                    "x",
                    int_0_9(),
                    Presence::Required,
                )]),
            ),
            CompositeDeclaration::new(
                tree,
                "Tree",
                CompositeShape::Record(vec![FieldDeclaration::new(
                    "kids",
                    kids,
                    Presence::Required,
                )]),
            ),
        ],
        [],
    )
    .expect("FR-143 admits P and Tree")
}

/// FR-093-AC-21 (TC-416 step 12), source-owned nodes: under owner (`a`, `u`)
/// `record P`, the recursive `Tree` and a function over `ordered enum Status`
/// carry the `SourceOwner`; the `Int[0, 9]`, `Sequence<Tree>`,
/// `collection_bounds`, parameter and expression nodes and the enum's
/// declaration and member nodes carry none.
#[trace("FR-093-AC-21", "TC-416")]
#[test]
fn source_declared_nodes_carry_their_source_owner() {
    let status = status_enum(
        json!({"kind": "source", "authority": "a", "identity": "u", "package_name": "pkg"}),
        "Status",
    );
    let status_type = || TypeForm::name("Status", SPAN);
    let keep = FunctionDeclaration::new(
        "keep",
        vec![("s".to_owned(), status_type())],
        status_type(),
        None,
        name("s"),
    );
    let mut declarations =
        PackageDeclarations::new(source(), "pkg", fsl_foundation::IdentityLimits::default());
    declarations.types = p_and_tree_types();
    declarations.enums = vec![status];
    declarations.functions = vec![keep];
    let package = CheckedPackage::link(
        declarations
            .check(
                CheckMode::Linked,
                CheckingLimits::default(),
                &fsl_semantics::check::test_support::ZeroRewardEvaluator,
            )
            .expect("the package checks"),
    );
    let emission = emit(&package);
    assert_eq!(emission.omitted, []);
    let wire = assert_owners(&package, &emission, verified);
    let a_u = owner_json("a", "u");
    for name in ["P", "Tree", "keep"] {
        assert_eq!(declared(&wire, name)["owner"], a_u, "{name}");
    }
    for (tag, form) in [
        ("bounded_domain", "integer_range"),
        ("bounded_domain", "collection_bounds"),
        ("composite_type", "sequence"),
        ("value", "parameter"),
    ] {
        let found = node_where(&wire, tag, form);
        assert!(!found.is_empty(), "{tag}/{form} is written");
        assert!(found.iter().all(|node| node.get("owner").is_none()));
    }
}

/// The AC-22 source, keyed identically while its owning source changes.
fn point_and_list_under(owner: &str) -> Emission {
    let point = CompositeDeclaration::new(
        NodeKey::new("pkg", vec![PathSegment::Text("Point".to_owned())], 0).unwrap(),
        "Point",
        CompositeShape::Record(vec![FieldDeclaration::new(
            "x",
            ValueType::Integer,
            Presence::Required,
        )]),
    );
    let list_key = NodeKey::new("pkg", vec![PathSegment::Text("List".to_owned())], 0).unwrap();
    let list = CompositeDeclaration::new(
        list_key.clone(),
        "List",
        CompositeShape::Record(vec![FieldDeclaration::new(
            "next",
            ValueType::Composite(list_key),
            Presence::Optional,
        )]),
    );
    let types = TypeEnvironment::new([point, list], []).expect("Point and recursive List");
    let mut declarations = PackageDeclarations::new(
        RawSourceRef::new("agent-ix", owner).expect("a source owner"),
        "pkg",
        fsl_foundation::IdentityLimits::default(),
    );
    declarations.types = types;
    let checked = declarations
        .check(
            CheckMode::Linked,
            CheckingLimits::default(),
            &fsl_semantics::check::test_support::ZeroRewardEvaluator,
        )
        .unwrap_or_else(|refusals| panic!("the source checks: {refusals:?}"));
    emit(&CheckedPackage::link(checked))
}

/// FR-093-AC-22 (TC-416 step 13): owners change the declared nodes' owner
/// member while the structural ids and recursive group remain identical.
#[trace("FR-093-AC-22", "TC-416")]
#[test]
fn changing_source_owner_preserves_declared_structural_ids() {
    let a = point_and_list_under("example-a");
    let b = point_and_list_under("example-b");
    verified(&a);
    verified(&b);
    assert_ne!(a.package.bytes(), b.package.bytes());
    let a_wire = wire(&a);
    let b_wire = wire(&b);
    let a_point = declared(&a_wire, "Point");
    let b_point = declared(&b_wire, "Point");
    let expected_point = json!({"package": "pkg", "path": ["Point"], "ordinal": 0});
    assert_eq!(a_point["node_id"], expected_point);
    assert_eq!(b_point["node_id"], expected_point);
    assert_eq!(a_point["owner"], owner_json("agent-ix", "example-a"));
    assert_eq!(b_point["owner"], owner_json("agent-ix", "example-b"));

    let a_list = declared(&a_wire, "List");
    let b_list = declared(&b_wire, "List");
    assert_eq!(a_list["node_id"], b_list["node_id"]);
    assert_eq!(a_list["recursion_group"], b_list["recursion_group"]);
    let members = |wire: &Value, group: &Value| {
        nodes(wire)
            .iter()
            .filter(|node| node["recursion_group"] == *group)
            .map(|node| node["node_id"].clone())
            .collect::<Vec<_>>()
    };
    assert_eq!(
        members(&a_wire, &a_list["recursion_group"]),
        members(&b_wire, &b_list["recursion_group"])
    );
    let integer = |wire: &Value| {
        nodes(wire)
            .iter()
            .find(|node| node["node_tag"] == "scalar_type" && node["semantic_form"] == "integer")
            .expect("the Integer scalar type")
            .clone()
    };
    assert_eq!(integer(&a_wire)["node_id"], integer(&b_wire)["node_id"]);
}

fn one() -> Multiplicity {
    Multiplicity {
        lower: 1,
        upper: Some(1),
        ordered: false,
        unique: true,
    }
}

fn model_key(name: &str) -> DeclarationKey {
    DeclarationKey {
        package: "acme/orders".to_owned(),
        node: format!("ix://acme/orders/{name}"),
    }
}

fn object(name: &str, supertypes: &[&str]) -> DomainPackageRecord {
    DomainPackageRecord::ObjectType(ObjectTypeRecord {
        key: model_key(name),
        interface_features: None,
        abstract_type: false,
        supertypes: supertypes.iter().map(|name| model_key(name)).collect(),
    })
}

fn query(name: &str, owner: &str, redefines: Option<&str>) -> DomainPackageRecord {
    DomainPackageRecord::OperationMember(OperationMemberRecord {
        key: model_key(name),
        owner: model_key(owner),
        parameters: Vec::new(),
        result: Some(OperationResult {
            value_type: ValueTypeRef::Native(NativeValueType::Integer),
            multiplicity: one(),
        }),
        effect: OperationEffect::default(),
        requires: Vec::new(),
        ensures: Vec::new(),
        has_body: true,
        redefines: redefines.map(model_key),
    })
}

/// The JSON document the lock selects for `acme/orders`, which IR's reader
/// reads to join every `ModelOwner`: object types `Order` (operations `size`
/// and `count`) and `Sub` (operation `size`).
fn acme_document() -> Vec<u8> {
    let mut document: Value = serde_json::from_slice(SPINE_MODEL_DOCUMENT).unwrap();
    let operation = |owner: &str, name: &str| {
        json!({
            "identity": format!("ix://acme/orders/{owner}/{name}"),
            "name": name,
            "params": [],
            "returns": {
                "typeRef": "ix://filament/native/Integer",
                "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": false}
            },
            "pre": [],
        })
    };
    let template = document["types"][0].clone();
    let object_type = |name: &str, supertypes: &[&str], operations: Vec<Value>| {
        let mut declared = template.clone();
        declared["identity"] = json!(format!("ix://acme/orders/{name}"));
        declared["displayName"] = json!(name);
        declared["supertypes"] = json!(supertypes
            .iter()
            .map(|name| format!("ix://acme/orders/{name}"))
            .collect::<Vec<_>>());
        declared["fields"] = json!([]);
        declared["operations"] = json!(operations);
        declared
    };
    document["types"] = json!([
        object_type(
            "Order",
            &[],
            vec![operation("Order", "size"), operation("Order", "count")]
        ),
        object_type("Sub", &["Order"], vec![operation("Sub", "size")]),
    ]);
    crate::emit::tests::wire_bytes(&document).unwrap()
}

/// Clause functions of `Order.size` (precondition `true`, body `7`),
/// `Order.count` (precondition `true`, body `1`) and `Sub.size` (precondition
/// `false`, body `8`), each over `self`.
fn clauses(order: EffectiveId, sub: EffectiveId) -> OperationClauses {
    let mut clauses = OperationClauses::default();
    for (operation, member, receiver, precondition, body) in [
        ("Order/size", "size", order.clone(), true, 7_i64),
        ("Order/count", "count", order, true, 1),
        ("Sub/size", "size", sub, false, 8),
    ] {
        let operation = model_key(operation);
        clauses.member.insert(operation.clone(), member.to_owned());
        clauses.parameters.insert(
            operation.clone(),
            vec![("self".to_owned(), ValueType::Reference(receiver))],
        );
        clauses.result.insert(operation.clone(), ValueType::Integer);
        clauses
            .own_precondition
            .insert(operation.clone(), Expression::boolean(precondition));
        clauses
            .own_body
            .insert(operation, Expression::integer(Integer::from(body)));
    }
    clauses
}

/// `checked_dispatch_operation` over `acme/orders`' effective view, rooted
/// at the operation `root`, joined by a declared function over a
/// `Reference<M::Order>` parameter, checked and linked.
fn acme_dispatch_package(root: &str) -> CheckedPackage {
    let package = DomainPackage::new(
        DomainPackageRef {
            identity: "acme/orders".to_owned(),
        },
        vec![
            object("Order", &[]),
            object("Sub", &["Order"]),
            query("Order/size", "Order", None),
            query("Order/count", "Order", None),
            query("Sub/size", "Sub", Some("Order/size")),
        ],
    );
    let NormalizeOutcome::Completed(view) =
        normalize(&package, ModelNormalizationLimits::UNLIMITED)
    else {
        panic!("acme/orders normalizes");
    };
    let model = AdmittedModel::new(&package, &view).expect("the view is the package's own");
    let id = |name: &str| view.type_identities()[&model_key(name)].clone();
    let (order, sub) = (id("Order"), id("Sub"));
    let types = TypeEnvironment::new(
        [],
        [
            ObjectTypeDeclaration::new(order.clone(), "M::Order", Vec::new()),
            ObjectTypeDeclaration::new(sub.clone(), "M::Sub", Vec::new())
                .with_supertypes(vec![order.clone()]),
        ],
    )
    .expect("the acme object types admit");
    let held = FunctionDeclaration::new(
        "held",
        vec![("r".to_owned(), TypeForm::name("M::Order", SPAN))],
        boolean(),
        None,
        Expression::boolean(true),
    );
    let mut meter = NormalizationMeter::new(ModelNormalizationLimits::UNLIMITED);
    let mut declarations = checked_dispatch_operation(
        &view,
        fsl_semantics::check::model_type_environment("fixture", &view).unwrap(),
        &DispatchRoot {
            key: model_key(root),
            closure: GeneralizationClosure::Closed,
        },
        &clauses(order, sub),
        source(),
        "pkg".to_owned(),
        &mut meter,
        fsl_foundation::IdentityLimits::default(),
    )
    .unwrap_or_else(|refusal| panic!("{root} links and checks: {refusal:?}"));
    declarations.types = types;
    if declarations.models.is_empty() {
        declarations.models.push(model);
    }
    declarations.functions.push(held);
    CheckedPackage::link(
        declarations
            .check(
                CheckMode::Linked,
                CheckingLimits::default(),
                &fsl_semantics::check::test_support::ZeroRewardEvaluator,
            )
            .unwrap_or_else(|refusals| panic!("{root} checks: {refusals:?}")),
    )
}

/// IR's v2 reader over `emission`, with the `acme/orders` document selected.
fn read_acme(emission: &Emission) -> Read {
    let document = acme_document();
    let mut evidence = read_evidence(emission);
    evidence
        .insert_domain_package_document(package_identity(&document[..]), &document[..])
        .expect("a fresh evidence key");
    read_with(emission, &evidence)
}

/// FR-093-AC-21 (TC-416 step 12), model-owned nodes: `checked_dispatch_operation`
/// over `acme/orders`' effective view builds the clause functions, and a
/// declared function over a `Reference<M::Order>` parameter joins them. The
/// model declaration nodes and every clause function carry their `ModelOwner`
/// (an operation member or an object type), the declared function its
/// `SourceOwner` (`a`, `u`), and the reference, parameter and expression nodes
/// none.
#[trace("FR-093-AC-21", "TC-416")]
#[test]
fn model_declaration_nodes_and_clause_functions_carry_their_model_owner() {
    let mut seen = BTreeSet::new();
    let mut model_nodes = BTreeSet::new();
    for root in ["Order/size", "Order/count", "Sub/size"] {
        let checked = acme_dispatch_package(root);
        let emission = emit(&checked);
        assert_eq!(emission.omitted, []);
        let read = |emission: &Emission| {
            assert!(
                matches!(read_acme(emission), Read::Verified { .. }),
                "{root}: IR's v2 reader admits the package"
            );
        };
        let wire = assert_owners(&checked, &emission, read);
        assert_eq!(declared(&wire, "held")["owner"], owner_json("a", "u"));
        let models = node_where(&wire, "model", "object_type");
        assert!(!models.is_empty(), "{root}: a model declaration node");
        for node in &models {
            assert_eq!(node["owner"]["kind"], "model");
            assert_eq!(node["owner"]["identity"], "acme/orders");
            assert!(node["owner"].get("version").is_none());
        }
        model_nodes.extend(
            models
                .iter()
                .map(|node| node["owner"]["node"].as_str().unwrap().to_owned()),
        );
        for node in node_where(&wire, "function", "pure_function")
            .into_iter()
            .filter(|node| node.get("declaration").is_none())
        {
            seen.insert(node["owner"]["node"].as_str().unwrap().to_owned());
        }
        for node in node_where(&wire, "composite_type", "reference") {
            assert!(node.get("owner").is_none());
        }
    }
    // FR-093-AC-21: exactly the document's two object types own a node.
    assert_eq!(
        model_nodes,
        BTreeSet::from([
            "ix://acme/orders/Order".to_owned(),
            "ix://acme/orders/Sub".to_owned(),
        ]),
        "the model owners' nodes"
    );
    assert_eq!(
        seen,
        BTreeSet::from([
            "ix://acme/orders/Order/count".to_owned(),
            "ix://acme/orders/Order/size".to_owned(),
            "ix://acme/orders/Sub/size".to_owned(),
        ]),
        "the clause functions are owned by their operation members"
    );
}

/// FR-496-AC-1 (TC-1286): a dispatch bridge admits `acme/orders` with no
/// `model` alias. Rooted at `Order/count`, whose family never names `Sub`,
/// the package still carries one row per operation of the document, each
/// `declaring` its own emitted object-type node (`Sub/size` under `Sub`),
/// and IR's v2 reader admits it.
#[trace("FR-496-AC-1", "TC-1286")]
#[test]
fn an_aliasless_models_rows_name_emitted_declaring_nodes() {
    let emission = emit(&acme_dispatch_package("Order/count"));
    assert_eq!(emission.omitted, []);
    let wire = wire(&emission);
    let declaring_owner = |row: &Value| {
        let node = nodes(&wire)
            .iter()
            .find(|node| node["node_id"] == row["declaring"])
            .unwrap_or_else(|| panic!("{}: its declaring node is emitted", row["operation"]));
        assert_eq!(
            (node["node_tag"].as_str(), node["semantic_form"].as_str()),
            (Some("model"), Some("object_type"))
        );
        node["owner"]["node"].as_str().unwrap().to_owned()
    };
    let carried = wire["operation_effects"]
        .as_array()
        .expect("the package carries operation_effects");
    assert_eq!(carried.len(), 3, "one row per operation");
    let rows: BTreeSet<(String, String)> = carried
        .iter()
        .map(|row| {
            (
                declaring_owner(row),
                row["operation"].as_str().unwrap().to_owned(),
            )
        })
        .collect();
    assert_eq!(
        rows,
        BTreeSet::from([
            ("ix://acme/orders/Order".to_owned(), "count".to_owned()),
            ("ix://acme/orders/Order".to_owned(), "size".to_owned()),
            ("ix://acme/orders/Sub".to_owned(), "size".to_owned()),
        ])
    );
    assert!(matches!(read_acme(&emission), Read::Verified { .. }));
}

/// FR-092-AC-8: a source enum and its members are written with structural
/// ids, the declaration path and the source owner.
#[trace("FR-092-AC-8", "TC-413")]
#[test]
fn a_source_enum_and_member_carry_structural_ids_path_and_source_owner() {
    let status = status_enum(
        json!({"kind": "source", "authority": "a", "identity": "u", "package_name": "pkg"}),
        "Status",
    );
    let package = declared_types(vec![status], Vec::new());
    let emission = emit(&package);
    assert_eq!(emission.omitted, []);
    verified(&emission);
    let wire = wire(&emission);
    let declaration = declared(&wire, "Status");
    assert_eq!(
        declaration["declaration"]["qualified_name"],
        json!(["Status"])
    );
    assert_eq!(declaration["owner"], owner_json("a", "u"));
    let members: Vec<&Value> = nodes(&wire)
        .iter()
        .filter(|node| node["semantic_form"] == "enum_value")
        .collect();
    assert_eq!(members.len(), 2);
    let graph = fsl_semantics::check::wire_id::WireGraph::of_package_value(&wire).unwrap();
    for node in std::iter::once(declaration).chain(members) {
        graph
            .key_of_value(&node["node_id"])
            .expect("a structural node id");
        assert!(node.get("nominal_identity_preimage").is_none());
    }
}

/// FR-098-AC-34: the function ids and modes `function_check_modes` reads from
/// the typed nodes spell exactly what the emitted bytes record, for a package
/// with a declared function and clause functions (ids that hold operand
/// slots), so a replay compares one id form on both sides.
#[trace("FR-098-AC-34", "TC-1784")]
#[test]
fn function_check_modes_spell_the_ids_the_emitted_bytes_record() {
    let emission = emit(&acme_dispatch_package("Order/count"));
    let document = quire_canonical::read(emission.package().bytes(), u64::MAX).unwrap();
    let root = document.root();
    let nodes = match member(&root, "semantic_graph")
        .and_then(|graph| member(&graph, "nodes"))
        .map(|nodes| nodes.node())
    {
        Some(quire_canonical::Node::Array(nodes)) => nodes,
        other => panic!("semantic_graph.nodes is an array: {other:?}"),
    };
    let recorded: Vec<(Vec<u8>, Option<String>)> = nodes
        .filter(|node| {
            matches!(
                member(node, "node_tag").map(|tag| tag.node()),
                Some(quire_canonical::Node::String("function"))
            )
        })
        .map(|node| {
            let id = quire_canonical::to_vec(
                &member(&node, "node_id").expect("a node id"),
                quire_canonical::Limits::new(u64::MAX),
            )
            .unwrap();
            let mode = member(&node, "check_mode").and_then(|mode| match mode.node() {
                quire_canonical::Node::String(mode) => Some(mode.to_owned()),
                _ => None,
            });
            (id, mode)
        })
        .collect();
    let typed: Vec<(Vec<u8>, Option<String>)> = emission
        .function_check_modes()
        .iter()
        .map(|(id, mode)| {
            (
                quire_canonical::to_vec(id, quire_canonical::Limits::new(u64::MAX)).unwrap(),
                mode.map(str::to_owned),
            )
        })
        .collect();
    assert!(
        recorded.len() >= 3,
        "a declared function and the clause functions: {}",
        recorded.len()
    );
    assert_eq!(typed, recorded);
}

/// FR-098-AC-34: a function node whose id holds a document index (the form of
/// every typed node) is keyed by its operand-slot id, the form the emitted
/// bytes record, never by the index, and by the exact slot the emitter's own
/// pass assigns.
///
/// This is the only guard of that keying. The function ids of the fixture
/// above hold no operand segment (the typed and the emitted forms are equal
/// there), and no other fixture reachable through `emit_package` is known to
/// produce one, so the test above cannot fail on a wrong form by itself.
#[trace("FR-098-AC-34", "TC-1784")]
#[test]
fn function_check_modes_key_an_indexed_function_id_by_its_operand_slot() {
    use filament_contract_model::{CheckedNodePathSegment as Segment, CheckedOperandEntry};
    let package = acme_dispatch_package("Order/count");
    let source = package.graph().source().clone();
    let mut nodes = crate::emit::emitted_document_nodes(
        &package,
        move |_| Some(SourceRegion::new(source.clone(), 0, TEXT.len() as u64).unwrap()),
        crate::EmitLimits::fixture(),
    )
    .expect("the package emits");
    let node = nodes
        .iter_mut()
        .find(|node| &*node.node_tag == "function")
        .expect("a function node");
    node.node_id.path.push(Segment::Index(0));
    // The expected id comes from the emitter's own pass over a clone.
    let mut expected = [node.clone()];
    let rows = write_operand_slots(&mut expected, crate::EmitLimits::fixture()).unwrap();
    let slot = rows[0]
        .iter()
        .position(|entry| *entry == CheckedOperandEntry::Node(0))
        .expect("the pushed index is an operand entry of the node");
    let [expected] = expected;
    assert_eq!(
        expected.node_id.path.last(),
        Some(&Segment::Operand(u64::try_from(slot).unwrap()))
    );
    // The retained modes are read from the graph the bytes are encoded
    // from: every node after the same operand-slot pass.
    let operand_rows = write_operand_slots(&mut nodes, crate::EmitLimits::fixture()).unwrap();
    let retained =
        crate::emit::RetainedNodes::of(&filament_contract_model::CheckedSemanticGraphV2 {
            graph_version: GRAPH_V2.into(),
            nodes,
            operand_rows,
        });
    let modes = &retained.function_modes;
    assert_eq!(modes[0].0, expected.node_id);
    assert!(modes
        .iter()
        .all(|(id, _)| !id.path.iter().any(|s| matches!(s, Segment::Index(_)))));
}

fn member<'d>(
    node: &quire_canonical::NodeRef<'d>,
    name: &str,
) -> Option<quire_canonical::NodeRef<'d>> {
    match node.node() {
        quire_canonical::Node::Object(members) => members
            .into_iter()
            .find(|(key, _)| *key == name)
            .map(|(_, value)| value),
        _ => None,
    }
}
