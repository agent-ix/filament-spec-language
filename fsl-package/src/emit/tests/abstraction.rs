// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-804 (FR-306-AC-1, FR-306-AC-2): the abstraction relation of a checked
//! package is written as one `correspondence`/`abstraction_relation` v2 node
//! (FSpec FR-451) that enters the package. The relation is bound with
//! `CheckedPackage::bind_abstraction` (the source path is tested in
//! `fsl-replay`'s model-gate tests), and the bytes are read back through IR's
//! reader. The expected body is written here from FR-451.

use filament_checked::key::DeclarationKey;
use filament_exact::Identifier;
use filament_semantic_value::checking::CheckMode;
use fsl_semantics::check::{
    AbstractionNodeRefusal, AuthoredBinding, FrameBinding, ObjectBinding, OperationKey,
    PopulationBinding, RustField, RustParameter, RustPath, RustReceiver,
};

use super::*;

#[path = "../../../../fsl-semantics/tests/support/abstraction.rs"]
#[allow(
    clippy::duplicate_mod,
    reason = "the export tests load the same fixture"
)]
pub(super) mod fixture;

/// The unit: the `attemptUpdate` postcondition names `ConfigVersion`'s
/// operation, so the package holds its object type, frame and anchor nodes.
const CLAUSES: &str = "post Updated using v on M::ConfigVersion::attemptUpdate { true }\n";

fn declaration(suffix: &str) -> DeclarationKey {
    DeclarationKey {
        package: fixture::PACKAGE.to_owned(),
        node: fixture::id(suffix),
    }
}

fn identifier(text: &str) -> Identifier {
    Identifier::new(text).expect("an identifier")
}

/// `ConfigVersion` bound to `ConfigVersion`, its `version` field to
/// `version_field` and `note` to `note`.
fn config_object(version_field: &str) -> AuthoredBinding {
    AuthoredBinding::Object {
        key: declaration("ConfigVersion"),
        value: ObjectBinding {
            rust_type: RustPath::new("ConfigVersion"),
            fields: BTreeMap::from([
                (identifier("version"), RustField::new(version_field)),
                (identifier("note"), RustField::new("note")),
            ]),
        },
    }
}

fn configs_population() -> AuthoredBinding {
    AuthoredBinding::Population {
        key: declaration("configs"),
        value: PopulationBinding {
            collection: RustPath::new("state::configs"),
        },
    }
}

fn attempt_update(function: &str) -> AuthoredBinding {
    AuthoredBinding::Frame {
        key: OperationKey {
            declaring: declaration("ConfigVersion"),
            operation: identifier("attemptUpdate"),
        },
        value: FrameBinding {
            function: RustPath::new(function),
            receiver: RustReceiver::SelfValue,
            parameters: BTreeMap::from([
                (identifier("n"), RustParameter::new("count")),
                (identifier("m"), RustParameter::new("limit")),
            ]),
        },
    }
}

/// FR-304-AC-1's relation, with `version` bound to `version_field`.
fn three_bindings(version_field: &str) -> Vec<AuthoredBinding> {
    vec![
        config_object(version_field),
        configs_population(),
        attempt_update("update"),
    ]
}

/// The fixture package: the document the unit is checked against
/// and the linked package.
fn linked() -> (Vec<u8>, CheckedPackage) {
    let document =
        crate::emit::tests::wire_bytes(&fixture::document()).expect("the fixture serializes");
    let packages = fsl_semantics::model::intake::package_input([document.as_slice()]).unwrap();
    let unit = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.value.complete/v1\";\n\
         model M = {package:?};\n\
         {CLAUSES}",
        package = fixture::PACKAGE,
    );
    let graph = assemble_with_models(unit.as_bytes(), &packages)
        .expect("the unit assembles")
        .check(
            CheckMode::Linked,
            CheckingLimits::default(),
            &fsl_semantics::check::test_support::ZeroRewardEvaluator,
        )
        .unwrap_or_else(|refusals| panic!("the package checks: {refusals:?}"));
    (document, CheckedPackage::link(graph))
}

struct Emitted {
    emission: Emission,
    wire: Value,
    evidence: CheckedPackageEvidence,
}

/// The fixture package with `bindings` bound (none when `None`), emitted.
fn emit_with(bindings: Option<&[AuthoredBinding]>) -> Emitted {
    let (document, mut package) = linked();
    if let Some(bindings) = bindings {
        package
            .bind_abstraction(bindings)
            .expect("the relation checks");
    }
    // The relation's occurrence is at the package root, which names no region
    // of the unit until a source declaration places it, so the unit places
    // it; every other occurrence keeps its own region, which its
    // declaration's `declaration_region` contains (FSpec FR-322).
    let graph = package.graph();
    let source = graph.source().clone();
    let emission = emit_package(&package, |location| {
        graph
            .region(location)
            .or_else(|| Some(SourceRegion::new(source.clone(), 0, 1).expect("a nonempty region")))
    })
    .expect("the package emits");
    assert_eq!(emission.omitted, []);
    let wire = wire(&emission);
    let mut evidence = CheckedPackageEvidence::new();
    for feature in wire["lock"]["required_features"].as_array().unwrap() {
        evidence.support_feature(feature.as_str().unwrap());
    }
    evidence
        .insert_domain_package_document(package_identity(&document), document.as_slice())
        .expect("a fresh evidence key");
    Emitted {
        emission,
        wire,
        evidence,
    }
}

impl Emitted {
    fn read(&self) -> Read {
        read_with(&self.emission, &self.evidence)
    }

    /// The one node of `tag`/`form` that `matches`.
    fn node(&self, tag: &str, form: &str, matches: impl Fn(&Value) -> bool) -> &Value {
        let found: Vec<&Value> = super::owners::node_where(&self.wire, tag, form)
            .into_iter()
            .filter(|node| matches(node))
            .collect();
        let [node] = found[..] else {
            panic!("one {tag}/{form} node: {found:#?}");
        };
        node
    }

    /// The `model`/`object_type` node declared as `suffix`.
    fn object_type(&self, suffix: &str) -> &Value {
        self.node("model", "object_type", |node| {
            node["owner"]["node"] == fixture::id(suffix)
        })
    }

    /// The `relation`/`population` node of the population declared as
    /// `suffix`: the one whose model owner is its declaration's node.
    fn population(&self, suffix: &str) -> &Value {
        self.node("relation", "population", |node| {
            node["owner"]["node"] == fixture::id(suffix)
        })
    }

    /// The package's one relation node.
    fn relation(&self) -> &Value {
        self.node("correspondence", "abstraction_relation", |_| true)
    }
}

/// FR-306-AC-1 (TC-804 step 1): the unit with FR-304-AC-1's relation emits
/// one relation node whose body holds the three bindings the in-process
/// relation holds -- keyed by the object type's, the population's and the
/// operation's nodes, valued by the Rust spellings split into segments -- in
/// FR-451's canonical order, typed by itself, depending on exactly its
/// targets, with one `declaration` occurrence; IR's reader admits the bytes.
#[trace("TC-804", "FR-306-AC-1")]
#[test]
fn the_relation_is_written_as_one_node_holding_the_three_bindings() {
    let emitted = emit_with(Some(&three_bindings("version")));
    let config = emitted.object_type("ConfigVersion")["node_id"].clone();
    let configs = emitted.population("configs")["node_id"].clone();
    let relation = emitted.relation();

    assert_eq!(
        relation["body"],
        json!({
            "term": "abstraction_relation",
            "objects": [{
                "type": config,
                "rust_type": ["ConfigVersion"],
                "fields": [
                    {"name": "note", "rust_field": "note"},
                    {"name": "version", "rust_field": "version"},
                ],
            }],
            "populations": [{"population": configs, "collection": ["state", "configs"]}],
            "frames": [{
                "context": config,
                "operation": "attemptUpdate",
                "function": ["update"],
                "receiver": "self",
                "parameters": [
                    {"name": "m", "rust_parameter": "limit"},
                    {"name": "n", "rust_parameter": "count"},
                ],
            }],
        })
    );
    let mut targets = vec![config, configs];
    let graph = fsl_semantics::check::wire_id::WireGraph::of_package_value(&emitted.wire).unwrap();
    targets.sort_by_key(|value| graph.key_of_value(value).unwrap());
    assert_eq!(relation["dependencies"], json!(targets));
    assert_eq!(relation["semantic_type"], relation["node_id"]);
    assert_eq!(
        relation["occurrences"],
        json!([{"role": "declaration", "ordinal": 0}])
    );
    assert!(
        matches!(emitted.read(), Read::Verified { .. }),
        "{:?}",
        emitted.read()
    );
}

/// FSpec FR-451: a relation read from two `abstraction` declarations is one
/// node with a `declaration` occurrence for each, ordinals in source order.
#[trace("FR-306-AC-1")]
#[test]
fn a_relation_read_from_two_declarations_has_an_occurrence_for_each() {
    use filament_semantic_value::location::{
        ExpressionLocation as Location, ExpressionOwner as Origin,
    };

    let (_, mut package) = linked();
    let site = |name: &str| {
        Location::root(Origin::TypeDeclaration {
            name: name.to_owned(),
        })
    };
    package
        .bind_abstraction_at(
            &three_bindings("version"),
            vec![site("first"), site("second")],
        )
        .expect("the relation checks");
    let source = package.graph().source().clone();
    let emission = emit_package(&package, move |_| {
        Some(SourceRegion::new(source.clone(), 0, 1).expect("a nonempty region"))
    })
    .expect("the package emits");
    let wire = wire(&emission);
    let [relation] = super::owners::node_where(&wire, "correspondence", "abstraction_relation")[..]
    else {
        panic!("one relation node");
    };
    assert_eq!(
        relation["occurrences"],
        json!([
            {"role": "declaration", "ordinal": 0},
            {"role": "declaration", "ordinal": 1},
        ])
    );
}

/// TC-804 step 1: a package that declares no relation holds no relation node.
#[test]
fn a_package_with_no_relation_writes_no_relation_node() {
    let emitted = emit_with(None);
    assert!(
        super::owners::node_where(&emitted.wire, "correspondence", "abstraction_relation")
            .is_empty()
    );
    assert!(super::owners::node_where(&emitted.wire, "relation", "population").is_empty());
    assert!(matches!(emitted.read(), Read::Verified { .. }));
}

/// FR-306-AC-2 (TC-804 step 2): an unchanged recompile gives an equal
/// relation node id and body. Changing an `ObjectBinding` field's
/// `RustField` keeps the node id, writes the new spelling in that type's
/// `objects` entry, and the changed package reads back.
#[trace("TC-804", "FR-306-AC-2")]
#[test]
fn a_rebinding_changes_the_relation_body() {
    let first = emit_with(Some(&three_bindings("version")));
    let again = emit_with(Some(&three_bindings("version")));
    assert_eq!(first.relation()["node_id"], again.relation()["node_id"]);
    assert_eq!(first.relation()["body"], again.relation()["body"]);

    let rebound = emit_with(Some(&three_bindings("revision")));
    assert_eq!(first.relation()["node_id"], rebound.relation()["node_id"]);
    assert_ne!(first.relation()["body"], rebound.relation()["body"]);
    let bound_fields = |emitted: &Emitted| {
        let objects = emitted.relation()["body"]["objects"]
            .as_array()
            .expect("the objects");
        let [object] = &objects[..] else {
            panic!("one objects entry: {objects:?}");
        };
        assert_eq!(
            object["type"],
            emitted.object_type("ConfigVersion")["node_id"]
        );
        object["fields"].clone()
    };
    assert_eq!(
        bound_fields(&first),
        json!([
            {"name": "note", "rust_field": "note"},
            {"name": "version", "rust_field": "version"},
        ])
    );
    assert_eq!(
        bound_fields(&rebound),
        json!([
            {"name": "note", "rust_field": "note"},
            {"name": "version", "rust_field": "revision"},
        ])
    );
    assert!(
        matches!(rebound.read(), Read::Verified { .. }),
        "{:?}",
        rebound.read()
    );
}

/// A Rust path with an empty segment has no array of segments, so the
/// emission refuses with the path rather than writing a node IR's reader
/// refuses.
#[test]
fn a_rust_path_with_an_empty_segment_refuses_the_emission() {
    let (_, mut package) = linked();
    let mut bindings = three_bindings("version");
    bindings[2] = attempt_update("update::");
    package
        .bind_abstraction(&bindings)
        .expect("the relation checks");
    assert_eq!(
        emit_checked(&package).expect_err("the emission refuses"),
        EmitRefusal::Abstraction(AbstractionNodeRefusal::EmptyRustSegment {
            spelling: "update::".to_owned(),
        })
    );
}

/// The `occurrences` of each relation node of `emission`.
fn relation_occurrences(emission: &Emission) -> Vec<Value> {
    let wire = wire(emission);
    super::owners::node_where(&wire, "correspondence", "abstraction_relation")
        .into_iter()
        .map(|relation| relation["occurrences"].clone())
        .collect()
}

/// The unit's one clause: a site the unit's own regions resolve.
fn clause_site() -> filament_semantic_value::location::ExpressionLocation {
    use filament_semantic_value::location::{
        ExpressionLocation as Location, ExpressionOwner as Origin,
    };
    Location::root(Origin::StateClause {
        clause: "Updated".to_owned(),
        index: 0,
    })
}

/// NFR-015-AC-9 (TC-1111 step 7): `bind_abstraction` after an emission clears
/// the remembered emission, and `emit_package`, with the caller placing the
/// unsited relation, includes the bound relation.
#[trace("TC-1111", "NFR-015-AC-9")]
#[test]
fn a_binding_after_an_emission_emits_the_relation() {
    let (_, mut package) = linked();
    let first = emit_checked(&package).expect("the package emits");
    assert_eq!(relation_occurrences(&first), Vec::<Value>::new());
    assert_eq!(package.emitted_package_name(), Some("pkg"));

    package
        .bind_abstraction(&three_bindings("version"))
        .expect("the relation checks");
    assert_eq!(package.emitted_package_name(), None);
    // An unsited relation names no region of the unit, so `emit_checked`,
    // which places each occurrence from its own location, refuses the
    // generated occurrence of the population node the relation targets.
    let refusal = emit_checked(&package).expect_err("no region to place");
    let EmitRefusal::UnlocatedOccurrence {
        node,
        role: CheckedOccurrenceRole::Generated,
        ordinal: 0,
    } = &refusal
    else {
        panic!("an unlocated generated occurrence: {refusal:?}");
    };
    assert_eq!(
        format!("{node:?}"),
        format!(
            "NodeKey({}:[\"relation\",\"population\",\"{}\",]:0)",
            fixture::PACKAGE,
            fixture::id("configs")
        )
    );
    // `emit_package` takes the caller's region for it.
    let graph = package.graph();
    let source = graph.source().clone();
    let second = emit_package(&package, |location| {
        graph
            .region(location)
            .or_else(|| Some(SourceRegion::new(source.clone(), 0, 1).expect("a nonempty region")))
    })
    .expect("the bound package emits");
    assert_eq!(
        relation_occurrences(&second),
        [json!([{"role": "declaration", "ordinal": 0}])]
    );
}

/// NFR-015-AC-10 (TC-1111 step 8): `bind_abstraction_at` after an emission
/// clears the remembered emission, so the next emission holds the relation
/// node and its `declaration` occurrence; a refused binding keeps the
/// remembered emission.
#[trace("TC-1111", "NFR-015-AC-10")]
#[test]
fn a_binding_at_a_site_after_an_emission_emits_the_relation() {
    let (_, mut package) = linked();
    let first = emit_checked(&package).expect("the package emits");
    assert_eq!(relation_occurrences(&first), Vec::<Value>::new());
    assert_eq!(package.emitted_package_name(), Some("pkg"));

    package
        .bind_abstraction_at(&three_bindings("version"), vec![clause_site()])
        .expect("the relation checks");
    assert_eq!(package.emitted_package_name(), None);
    let second = emit_checked(&package).expect("the bound package emits");
    assert_eq!(
        relation_occurrences(&second),
        [json!([{"role": "declaration", "ordinal": 0}])]
    );

    let (_, mut refused) = linked();
    emit_checked(&refused).expect("the package emits");
    let absent = AuthoredBinding::Population {
        key: declaration("absent"),
        value: PopulationBinding {
            collection: RustPath::new("state::absent"),
        },
    };
    let refusals = refused
        .bind_abstraction_at(&[absent], vec![clause_site()])
        .expect_err("the package holds no such declaration");
    assert!(
        matches!(
            &refusals[..],
            [fsl_semantics::check::AbstractionRefusal::MissingDeclaration { .. }]
        ),
        "{refusals:?}"
    );
    assert!(refused.abstraction().is_none());
    assert_eq!(refused.emitted_package_name(), Some("pkg"));
    let kept = refused.remembered_emission().expect("the emission is kept");
    assert_eq!(relation_occurrences(&kept), Vec::<Value>::new());
}
