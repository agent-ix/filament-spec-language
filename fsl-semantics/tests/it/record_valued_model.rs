// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-985 at the model level: record-valued model fields and observations.
//! The package checks of FR-056-AC-22 and the type environment of FR-056-AC-21
//! (TC-985 steps 1 and 2), and the snapshot and invocation admission of
//! FR-106-AC-14 and FR-106-AC-17 (steps 3, 4 and 6), over one domain package:
//! record value types `Street { name: Integer }`, `Address { street?: Street }`,
//! `PostalAddress: Address { zip: Integer }` and `Node { next?: Node }`, and the
//! object type `Account` with `address: Address`, `previous?: Address`,
//! `history: Sequence<Address>[0, 3]` and operations `relocate` (granting
//! `Account.address`) and `stay` (granting nothing).
//!
//! Every expectation is the specification's table: typed code, cause, path and
//! change, never rendered text.

use filament_exact::{FieldValue, Value as Datum, ValueType};
use filament_semantic_value::checking::{CheckMode, CheckingLimits};
use filament_semantic_value::declaration::FieldRef;
use filament_semantic_value::object_closure::ObjectClosure;
use fsl_forms::build_unit;
use fsl_foundation::diagnostic::Code;
use fsl_foundation::{IntakeLimits, SourceIdentity};
use fsl_semantics::check::{CheckedGraph, PackageDeclarations};
use fsl_semantics::model::accounting::ModelNormalizationLimits;
use fsl_semantics::model::domain_package::{DomainPackage, DomainPackageRef};
use fsl_semantics::model::intake::{admit, admit_unit, package_input, read_domain_package};
use fsl_semantics::model::normalize::{NormalizeOutcome, normalize_with_meter};
use fsl_semantics::model::observation::{
    AdmissionFailure, DocumentRef, FrameChange, FrameFacts, FrameVerdict, ObservationLimits,
    OperationFacts, PathStep, Provisions, admit_current_snapshot, admit_frame_invocation,
    population_universe_for,
};
use ix_trace_rs::trace;
use serde_json::{Value, json};
use std::collections::BTreeMap;

use crate::tc235_k2 as base;
#[path = "../support/variant_fields.rs"]
#[allow(
    clippy::duplicate_mod,
    reason = "a test binary may load the variant fixture directly"
)]
#[allow(dead_code, reason = "this fixture uses part of the shared builders")]
mod variants;

use base::{PACKAGE, id};

const INTEGER: &str = "ix://filament/native/Integer";

fn record(name: &str, members: Value) -> Value {
    base::declaration(name, "money", members)
}

/// The record value types of TC-985's preconditions.
fn record_types() -> Vec<Value> {
    let mut postal_address = record(
        "PostalAddress",
        json!({
            "supertypes": [id("Address")],
            "fields": [base::field("PostalAddress", "zip", INTEGER)],
        }),
    );
    postal_address["clauses"] =
        json!([base::clause("PostalAddress", "PositiveZip", "self.zip > 0")]);
    vec![
        record(
            "Street",
            json!({"fields": [base::field("Street", "name", INTEGER)]}),
        ),
        record(
            "Address",
            json!({"fields": [variants::optional("Address", "street", &id("Street"))]}),
        ),
        postal_address,
        record(
            "Node",
            json!({"fields": [variants::optional("Node", "next", &id("Node"))]}),
        ),
    ]
}

fn account(operations: Vec<Value>) -> Value {
    base::declaration(
        "Account",
        "thing",
        json!({
            "identityFields": [id("Account/number")],
            "fields": [
                base::field("Account", "number", INTEGER),
                base::field("Account", "address", &id("Address")),
                variants::optional("Account", "previous", &id("Address")),
                variants::many("Account", "history", &id("Address"), true, false),
            ],
            "operations": operations,
        }),
    )
}

fn operations() -> Vec<Value> {
    let mut relocate = variants::operation_with(
        "Account",
        "relocate",
        &[("next", id("Address"))],
        &id("Address"),
    );
    relocate["frame"]["modifies"] = json!([id("Account/address")]);
    let stay = variants::operation_with(
        "Account",
        "stay",
        &[("next", id("Address"))],
        &id("Address"),
    );
    vec![relocate, stay]
}

fn document_of(types: Vec<Value>) -> Value {
    let mut document = variants::package(types, true);
    document["populations"] = json!([{
        "identity": id("accounts"), "displayName": id("accounts"),
        "kind": {"module": base::BUSINESS, "name": "population"},
        "members": [id("Account")], "extent": "closed",
        "origin": base::origin("accounts"),
    }]);
    document
}

fn document() -> Value {
    let mut types = record_types();
    types.push(account(operations()));
    document_of(types)
}

// ---------------------------------------------------------------------------
// Package checks (FR-056-AC-21, FR-056-AC-22; TC-985 steps 1 and 2)
// ---------------------------------------------------------------------------

fn read(
    document: &Value,
) -> Result<DomainPackage, Vec<fsl_semantics::model::normalize::ModelRefusal>> {
    let bytes = serde_json::to_vec(document).expect("the fixture serializes");
    let offered = package_input([bytes.as_slice()]).unwrap();
    let selection = DomainPackageRef {
        identity: PACKAGE.to_owned(),
    };
    let (_selection, admitted) = admit(
        selection.identity.as_str(),
        &offered,
        IntakeLimits::default(),
    )
    .expect("S1 admits the package bytes");
    read_domain_package(selection, &admitted)
}

/// Every refusal reading and then normalizing `document` gives, in order.
fn refusals_of(document: &Value) -> Vec<fsl_semantics::model::normalize::ModelRefusal> {
    match read(document) {
        Err(refusals) => refusals,
        Ok(package) => {
            match normalize_with_meter(&package, ModelNormalizationLimits::default()).0 {
                NormalizeOutcome::Refused(refusals) => refusals.iter().cloned().collect(),
                other => panic!("the package refuses, not {other:?}"),
            }
        }
    }
}

fn type_mut<'a>(document: &'a mut Value, name: &str) -> &'a mut Value {
    document["types"]
        .as_array_mut()
        .expect("types")
        .iter_mut()
        .find(|ty| ty["identity"] == id(name))
        .unwrap_or_else(|| panic!("{name} is declared"))
}

fn assembled(document: &Value) -> PackageDeclarations {
    let bytes = serde_json::to_vec(document).expect("the fixture serializes");
    let packages = package_input([bytes.as_slice()]).unwrap();
    let source = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.model.complete/v1\";\n\
         model Config = {PACKAGE:?};\n\
         function keep using v(): Boolean pure {{ true }}\n\
         function previous using v(account: Reference<Config::Account>): Option<Config::Address> pure {{ deref(account).previous }}\n"
    );
    let parsed = fsl_cst::parse(
        SourceIdentity::new("test", "record-valued"),
        "record-valued.native",
        source.as_bytes(),
        fsl_cst::Limits::default(),
    )
    .expect("S1 reads the source unit");
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let unit = build_unit(&parsed).expect("S2 builds the source unit");
    let selected = admit_unit(
        &unit.selections().models,
        &packages,
        ModelNormalizationLimits::default(),
    )
    .expect("I1 admits and normalizes the package");
    PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        crate::TEST_PACKAGE_NAME.to_owned(),
        unit,
        selected,
        Vec::new(),
    )
    .unwrap_or_else(|refusal| panic!("assembly succeeds: {refusal:?}"))
}

fn object_field<'a>(
    declarations: &'a PackageDeclarations,
    object: &str,
    field: &str,
) -> &'a filament_semantic_value::declaration::FieldDeclaration {
    declarations
        .types
        .object_types()
        .find(|declared| declared.name() == format!("Config::{object}"))
        .unwrap_or_else(|| panic!("{object} is declared"))
        .attributes()
        .iter()
        .find(|attribute| attribute.name() == field)
        .unwrap_or_else(|| panic!("{object}.{field} is declared"))
}

/// FR-056-AC-21 over TC-985 step 1: the package's record value types are
/// retained, the type environment gives `Account.address` the record value
/// type, an optional field the optional type and an ordered non-unique
/// `[0, 3]` field the bounded sequence, never a reference, and an operation
/// parameter and result naming `Address` have that same value type.
#[trace("TC-985", "FR-056-AC-21")]
#[test]
fn the_type_environment_gives_record_fields_the_record_value_type() {
    let declarations = assembled(&document());
    let package = read(&document()).expect("record-value package is admitted");
    let mut record_exports: Vec<_> = package
        .records
        .iter()
        .filter_map(|record| match record {
            fsl_semantics::model::domain_package::DomainPackageRecord::RecordValueType(record) => {
                Some(record.key.node.as_str().to_owned())
            }
            _ => None,
        })
        .collect();
    record_exports.sort();
    assert_eq!(
        record_exports,
        [id("Address"), id("Node"), id("PostalAddress"), id("Street"),],
        "one record export is retained for each record type"
    );
    for field in [
        "Street/name",
        "Address/street",
        "PostalAddress/zip",
        "Node/next",
    ] {
        assert!(
            package.records.iter().any(|record| matches!(
                record,
                fsl_semantics::model::domain_package::DomainPackageRecord::FieldMember(member)
                    if member.key.node.as_str() == id(field)
            )),
            "the record's effective field export is retained: {field}"
        );
    }
    assert!(
        package.records.iter().any(|record| matches!(
            record,
            fsl_semantics::model::domain_package::DomainPackageRecord::Clause(clause)
                if clause.key.node.as_str() == id("PostalAddress/PositiveZip")
                    && clause.owner.node.as_str() == id("PostalAddress")
                    && clause.text == "self.zip > 0"
        )),
        "the record invariant is retained as a clause export"
    );
    let composite = |name: &str| {
        declarations
            .types
            .composites()
            .find(|declared| declared.name() == format!("Config::{name}"))
            .unwrap_or_else(|| panic!("{name} is a record value type"))
    };
    let address = ValueType::Composite(composite("Address").key());
    let postal = composite("PostalAddress");
    assert_eq!(
        postal.supertypes(),
        [composite("Address").key()],
        "PostalAddress specializes Address"
    );

    let direct = object_field(&declarations, "Account", "address");
    assert_eq!(direct.value_type(), &address);
    assert_eq!(direct.presence(), filament_exact::Presence::Required);

    let optional = object_field(&declarations, "Account", "previous");
    assert_eq!(optional.value_type(), &address);
    assert_eq!(optional.presence(), filament_exact::Presence::Optional);

    let graph = assembled(&document())
        .check(
            CheckMode::Linked,
            CheckingLimits::default(),
            &crate::reward_evaluator::ZeroRewardEvaluator,
        )
        .expect("the optional model field read checks");
    let previous = graph
        .function_identity("previous")
        .expect("the optional model field reader");
    let read = graph
        .function_by_identity(previous)
        .expect("the checked field reader")
        .body;
    assert_eq!(
        read.value_type(),
        &ValueType::option(address.clone()),
        "reading Account.previous exposes Option<Address> while the raw declaration keeps presence separate"
    );

    let history = object_field(&declarations, "Account", "history");
    let ValueType::Collection(collection) = history.value_type() else {
        panic!("history is a collection: {:?}", history.value_type());
    };
    assert_eq!(collection.kind(), filament_exact::CollectionKind::Sequence);
    assert_eq!(collection.element(), &address);
    let bound = collection.bound().expect("history is bounded [0, 3]");
    assert_eq!((bound.minimum(), bound.maximum()), (0, 3));

    for field in [direct, optional] {
        assert!(
            !matches!(field.value_type(), ValueType::Reference(_)),
            "a record value is never a reference: {field:?}"
        );
    }

    // The operation's parameter and result are the same value type.
    let account = declarations
        .types
        .object_types()
        .find(|declared| declared.name() == "Config::Account")
        .expect("Account")
        .key();
    let relocate = declarations
        .operations
        .declared_by(account)
        .iter()
        .find(|operation| operation.name() == "relocate")
        .expect("relocate");
    assert_eq!(
        &relocate.parameters()[0].1,
        &address,
        "the parameter naming Address"
    );
    assert_eq!(relocate.result(), Some(&address), "the result");
}

fn assert_malformed(refusal: &fsl_semantics::model::normalize::ModelRefusal) {
    assert_eq!(refusal.code, Code::InvalidModelBinding);
    assert_eq!(refusal.cause.as_str(), "malformed-declaration");
}

/// FR-056-AC-22 over TC-985 step 2: a record value type with an identity field
/// or an operation refuses with FR-208's named cause before assembly. An
/// object or record field whose `typeRef` names an event type, state machine,
/// process, persistence interface or namespace refuses
/// `invalid_model_binding`/`malformed-declaration` at intake. A finite
/// recursive record field is retained as a record type with its optional edge.
#[trace("TC-985", "FR-056-AC-22")]
#[test]
fn prohibited_record_forms_refuse_and_a_finite_recursive_record_is_retained() {
    // The identity field and the operation.
    let mut with_identity = document();
    type_mut(&mut with_identity, "Street")["identityFields"] = json!([id("Street/name")]);
    let refusals = refusals_of(&with_identity);
    assert_eq!(refusals.len(), 1, "{refusals:?}");
    assert_malformed(&refusals[0]);

    let mut with_operation = document();
    type_mut(&mut with_operation, "Street")["operations"] =
        json!([base::operation("Street", "add")]);
    let refusals = refusals_of(&with_operation);
    assert_eq!(refusals.len(), 1, "{refusals:?}");
    assert_eq!(refusals[0].code, Code::UnsupportedConstruct);

    // A field naming each prohibited business meaning, on an object and on a
    // record. The prohibited declarations come from TC-235's K2 package, whose
    // other types they replace.
    let k2 = base::k2();
    let prohibited = [
        ("OrderPlaced", "an event type"),
        ("OrderLifecycle", "a state machine"),
        ("Fulfilment", "a process"),
        ("OrderStore", "a persistence interface"),
        ("OrderManagement", "a namespace"),
    ];
    for (target, what) in prohibited {
        for (owner, field, _) in [
            ("Order", "total", "an object field"),
            ("Money", "currency", "a record field"),
        ] {
            let mut document = k2.clone();
            let node = type_mut(&mut document, owner)["fields"]
                .as_array_mut()
                .expect("fields")
                .iter_mut()
                .find(|each| each["name"] == field)
                .expect("the field");
            node["typeRef"] = json!(id(target));
            let refusals = refusals_of(&document);
            assert!(
                refusals.iter().any(|refusal| {
                    refusal.code == Code::InvalidModelBinding
                        && refusal.cause.as_str() == "malformed-declaration"
                }),
                "{owner}.{field} naming {what} refuses malformed-declaration: {refusals:?}"
            );
        }
    }

    // The control: the unmutated K2 package reads and normalizes.
    let control = read(&k2).expect("the K2 package reads");
    assert!(matches!(
        normalize_with_meter(&control, ModelNormalizationLimits::default()).0,
        NormalizeOutcome::Completed(_)
    ));

    // A finite recursive record: `Node { next?: Node }`.
    let declarations = assembled(&document());
    let node = declarations
        .types
        .composites()
        .find(|declared| declared.name() == "Config::Node")
        .expect("the recursive record is retained");
    let filament_semantic_value::declaration::CompositeShape::Record(fields) = node.shape() else {
        panic!("Node is a record");
    };
    let next = fields
        .iter()
        .find(|field| field.name() == "next")
        .expect("the recursive edge");
    assert_eq!(next.presence(), filament_exact::Presence::Optional);
    assert_eq!(next.value_type(), &ValueType::Composite(node.key()));
}

// ---------------------------------------------------------------------------
// Observation (FR-106-AC-14, FR-106-AC-17; TC-985 steps 3, 4 and 6)
// ---------------------------------------------------------------------------

struct World {
    declarations: PackageDeclarations,
    graph: CheckedGraph,
    packages: fsl_semantics::model::intake::PackageInput,
}

fn world() -> World {
    let document = document();
    let declarations = assembled(&document);
    let graph = assembled(&document)
        .check(
            CheckMode::Linked,
            CheckingLimits::default(),
            &crate::reward_evaluator::ZeroRewardEvaluator,
        )
        .unwrap_or_else(|refusals| panic!("the unit checks: {refusals:?}"));
    let bytes = serde_json::to_vec(&document).expect("the fixture serializes");
    World {
        declarations,
        graph,
        packages: package_input([bytes.as_slice()]).unwrap(),
    }
}

fn street(name: &str) -> Value {
    json!({"record": {"type": id("Street"), "fields": {"name": {"integer": name}}}})
}

/// The AC-14 address: a `PostalAddress` with a present `Street` and `zip` 42.
fn postal(name: &str) -> Value {
    json!({"record": {"type": id("PostalAddress"), "fields": {
        "street": {"present": street(name)},
        "zip": {"integer": "42"},
    }}})
}

/// A plain `Address` with no street.
fn plain() -> Value {
    json!({"record": {"type": id("Address"), "fields": {"street": {"absent": {}}}}})
}

fn snapshot_text(label: &str, observation: &str, address: &str) -> String {
    format!(
        r#"{{"format":"filament.state.snapshot/v1","identity":{{"authority":"test","identity":"{label}"}},"observation":"{observation}","model":{{"identity":"{PACKAGE}"}},"populations":[{{"population":"{population}","complete":true,"objects":[{{"key":"a1","type":"{account}","fields":{{"number":{{"integer":"1"}},"address":{address},"previous":{{"absent":{{}}}},"history":{{"sequence":[]}}}}}}]}}]}}"#,
        population = id("accounts"),
        account = id("Account"),
    )
}

fn reference(label: &str) -> DocumentRef {
    DocumentRef {
        authority: "test".to_owned(),
        identity: label.to_owned(),
    }
}

impl World {
    fn account(&self) -> filament_exact::EffectiveId {
        self.declarations
            .types
            .object_types()
            .find(|declared| declared.name() == "Config::Account")
            .expect("Account")
            .key()
    }

    fn admit_text(&self, text: &str) -> Result<ObjectClosure, AdmissionFailure> {
        let reference = reference("current");
        let provision = BTreeMap::from([(reference.clone(), text.as_bytes().to_vec())]);
        admit_current_snapshot(
            self.graph.model_selections(),
            self.graph.scope().types(),
            &self.packages,
            ModelNormalizationLimits::default(),
            &provision,
            &reference,
            ObservationLimits::default(),
        )
        .map(|(environment, _)| environment.objects().clone())
    }

    fn admit(&self, address: &Value) -> Result<ObjectClosure, AdmissionFailure> {
        self.admit_text(&snapshot_text("current", "current", &address.to_string()))
    }

    fn address_of(&self, closure: &ObjectClosure) -> FieldValue {
        let account = self.account();
        let universe = population_universe_for(
            self.graph.model_selections(),
            &self.packages,
            ModelNormalizationLimits::default(),
            account.clone(),
        )
        .expect("the Account universe");
        let reference = closure.find(universe, "a1").expect("the admitted object");
        closure
            .attribute(
                self.graph.scope().types(),
                reference,
                &FieldRef::new(account, "address"),
            )
            .cloned()
            .expect("the address slot")
    }

    fn operation(&self, name: &str) -> OperationFacts {
        let account = self.account();
        let declaration = self
            .declarations
            .operations
            .declared_by(account.clone())
            .iter()
            .find(|operation| operation.name() == name)
            .expect("the operation is declared")
            .clone();
        OperationFacts {
            declaring: account,
            declaration,
        }
    }

    /// Check 11 of `operation` invoked between the two addresses.
    fn frame(&self, operation: &str, pre: &Value, post: &Value) -> FrameVerdict {
        let (pre_ref, post_ref, call) = (reference("pre"), reference("post"), reference("call"));
        let snapshots = BTreeMap::from([
            (
                pre_ref.clone(),
                snapshot_text("pre", "pre", &pre.to_string()).into_bytes(),
            ),
            (
                post_ref.clone(),
                snapshot_text("post", "post", &post.to_string()).into_bytes(),
            ),
        ]);
        let identity = |document: &DocumentRef| json!({"authority": document.authority, "identity": document.identity});
        let invocation = serde_json::to_vec(&json!({
            "format": "filament.state.invocation/v1",
            "identity": identity(&call),
            "model": {"identity": PACKAGE},
            "context": id("Account"),
            "operation": operation,
            "self": {"population": id("accounts"), "key": "a1"},
            "pre": {"identity": identity(&pre_ref)},
            "post": {"identity": identity(&post_ref)},
            "parameters": {"next": post.clone()},
            "result": post.clone(),
            "created": [],
            "deleted": [],
        }))
        .expect("the invocation serializes");
        let invocations = BTreeMap::from([(call.clone(), invocation)]);
        let facts = FrameFacts {
            context: self.account(),
            frame: crate::tc235_k2::id("frame").parse_fixture_key(),
            operation: self.operation(operation),
        };
        let admitted = admit_frame_invocation(
            self.graph.model_selections(),
            self.graph.scope().types(),
            &facts,
            &self.packages,
            ModelNormalizationLimits::default(),
            &Provisions {
                snapshots: &snapshots,
                invocations: &invocations,
            },
            &call,
            ObservationLimits::default(),
        )
        .unwrap_or_else(|failure| panic!("the invocation admits: {failure:?}"));
        admitted
            .check_frame(facts.operation.declaration.effect())
            .expect("check 11 evaluates")
    }
}

trait FixtureKey {
    fn parse_fixture_key(&self) -> filament_exact::NodeKey;
}

impl FixtureKey for String {
    fn parse_fixture_key(&self) -> filament_exact::NodeKey {
        filament_exact::NodeKey::new(
            "fixture",
            vec![filament_exact::PathSegment::Text(self.clone())],
            0,
        )
        .expect("a valid fixture key")
    }
}

fn record_of(value: &FieldValue) -> &filament_exact::CompositeValue {
    match value {
        FieldValue::Present(Datum::Composite(composite)) => composite,
        other => panic!("a present record, not {other:?}"),
    }
}

fn slot_record(value: &FieldValue, index: usize) -> &filament_exact::CompositeValue {
    match &record_of(value).slots()[index] {
        FieldValue::Present(Datum::Option(option)) => match option.payload() {
            Some(Datum::Composite(composite)) => composite.as_ref(),
            other => panic!("a present nested record, not {other:?}"),
        },
        FieldValue::Present(Datum::Composite(composite)) => composite,
        other => panic!("a record slot, not {other:?}"),
    }
}

fn node_text(name: &str) -> String {
    id(name)
}

/// FR-106-AC-14 over TC-985 step 3: a current snapshot whose required
/// `address` is a `PostalAddress` holding a present `Street` and `zip` 42
/// admits. Reordering the members of any JSON object yields the same typed
/// observation, and the two nested records keep their most-specific types.
#[trace("TC-985", "FR-106-AC-14")]
#[test]
fn a_nested_record_admits_with_its_most_specific_types_in_any_member_order() {
    let world = world();
    let admitted = world.admit(&postal("7")).expect("the record admits");
    let slot = world.address_of(&admitted);
    let outer = record_of(&slot);
    assert_eq!(
        outer.declaration(),
        &composite_key(&world, "PostalAddress"),
        "the outer record keeps its most-specific type, not Address"
    );
    // `PostalAddress` lists the inherited `street` before its own `zip`.
    assert_eq!(outer.slots().len(), 2);
    let inner = slot_record(&slot, 0);
    assert_eq!(inner.declaration(), &composite_key(&world, "Street"));
    assert!(matches!(
        &inner.slots()[0],
        FieldValue::Present(Datum::Integer(value)) if value.to_string() == "7"
    ));
    assert!(matches!(
        &outer.slots()[1],
        FieldValue::Present(Datum::Integer(value)) if value.to_string() == "42"
    ));

    // Reordering JSON object members yields the same typed observation.
    let reordered = format!(
        r#"{{"record":{{"fields":{{"zip":{{"integer":"42"}},"street":{{"present":{{"record":{{"fields":{{"name":{{"integer":"7"}}}},"type":"{street}"}}}}}}}},"type":"{postal}"}}}}"#,
        street = node_text("Street"),
        postal = node_text("PostalAddress"),
    );
    let reordered = world
        .admit_text(&snapshot_text("current", "current", &reordered))
        .expect("the reordered record admits");
    assert!(
        same_value(&world.address_of(&reordered), &slot),
        "member order does not change the typed observation"
    );

    // The nested records are reached by distinct field and option-value
    // paths: a defect in `Street.name` and one in `zip` name different paths.
    let refusal_path = |address: Value| match world.admit(&address) {
        Err(AdmissionFailure::Refused(record)) => (record.catalog, record.path),
        other => panic!("a refusal, not {other:?}"),
    };
    let mut bad_name = postal("7");
    bad_name["record"]["fields"]["street"]["present"]["record"]["fields"]["name"] =
        json!({"boolean": true});
    let mut bad_zip = postal("7");
    bad_zip["record"]["fields"]["zip"] = json!({"boolean": true});
    let (name_catalog, name_path) = refusal_path(bad_name);
    let (zip_catalog, zip_path) = refusal_path(bad_zip);
    assert_eq!(name_catalog.code(), "invalid_runtime_input");
    assert_eq!(name_catalog.cause(), "wrong-value-kind");
    assert_eq!(zip_catalog.cause(), "wrong-value-kind");
    let below = |path: &[PathStep]| {
        let at = path
            .iter()
            .position(|step| *step == PathStep::Field("address".into()))
            .expect("the path reaches the address field");
        path[at..].to_vec()
    };
    assert_eq!(
        below(&name_path),
        [
            PathStep::Field("address".into()),
            PathStep::Field("street".into()),
            PathStep::OptionValue,
            PathStep::Field("name".into()),
        ],
        "the nested Street is reached through its field and option value"
    );
    assert_eq!(
        below(&zip_path),
        [
            PathStep::Field("address".into()),
            PathStep::Field("zip".into())
        ]
    );

    // A required record omitted its members is refused, never partly admitted.
    let mut missing = postal("7");
    missing["record"]["fields"]
        .as_object_mut()
        .unwrap()
        .remove("zip");
    let AdmissionFailure::Refused(record) = world.admit(&missing).expect_err("a missing member")
    else {
        panic!("a refusal");
    };
    assert_eq!(record.catalog.cause(), "missing-member");
}

fn composite_key(world: &World, name: &str) -> filament_exact::NodeKey {
    world
        .declarations
        .types
        .composites()
        .find(|declared| declared.name() == format!("Config::{name}"))
        .unwrap_or_else(|| panic!("{name} is a record value type"))
        .key()
}

fn same_value(left: &FieldValue, right: &FieldValue) -> bool {
    match (left, right) {
        (FieldValue::Present(left), FieldValue::Present(right)) => {
            filament_exact::compare_keys(left, right) == Some(std::cmp::Ordering::Equal)
        }
        _ => false,
    }
}

fn assert_unauthorized(verdict: FrameVerdict, what: &str) {
    let FrameVerdict::Violation(witness) = verdict else {
        panic!("{what}: a change without the grant violates the frame, not {verdict:?}");
    };
    assert_eq!(witness.code, Code::FrameViolation, "{what}");
    assert_eq!(witness.cause, "unauthorized-change", "{what}");
    assert!(
        matches!(&witness.change, FrameChange::FieldWrite { object, field, .. }
            if object == "a1" && field == "address"),
        "{what}: the change names Account.address: {:?}",
        witness.change
    );
}

/// FR-106-AC-17 over TC-985 step 6: an invocation changing `Account.address`
/// from one admitted record value to another admits its frame when the
/// selected `modifies` grant covers the declared object field. Without the
/// grant it refuses `frame_violation`/`unauthorized-change` naming
/// `Account.address`, for a change to a record member and for a change to its
/// most-specific type alike.
#[trace("TC-985", "FR-106-AC-17")]
#[test]
fn a_changed_record_value_needs_the_declared_object_field_grant() {
    let world = world();
    let before = postal("7");
    let member_changed = postal("8");
    // The same shared fields with the record's type replaced by `Address`.
    let retyped = plain();
    let retyped_from = json!({"record": {"type": id("Address"),
        "fields": {"street": {"present": street("7")}}}});
    let retyped_to = json!({"record": {"type": id("PostalAddress"), "fields": {
        "street": {"present": street("7")}, "zip": {"integer": "42"}}}});

    // With the grant (`relocate`), both changes hold.
    assert_eq!(
        world.frame("relocate", &before, &member_changed),
        FrameVerdict::Holds
    );
    assert_eq!(
        world.frame("relocate", &retyped_from, &retyped_to),
        FrameVerdict::Holds,
        "the type replacement is granted by the same field"
    );
    assert_eq!(
        world.frame("relocate", &before, &retyped),
        FrameVerdict::Holds
    );

    // Without it (`stay`), each names the declared object field.
    assert_unauthorized(
        world.frame("stay", &before, &member_changed),
        "a changed record member",
    );
    assert_unauthorized(
        world.frame("stay", &retyped_from, &retyped_to),
        "a replaced most-specific type with no shared field changed",
    );
    assert_unauthorized(
        world.frame("stay", &before, &retyped),
        "a different record type and member set",
    );
    // An unchanged address needs no grant.
    assert_eq!(world.frame("stay", &before, &before), FrameVerdict::Holds);
}

// ---------------------------------------------------------------------------
// FR-056-AC-34 (TC-992 step 1): native String and UUID are distinct leaves
// ---------------------------------------------------------------------------

/// The declared value type of `owner.field`, wherever the type environment
/// keeps the owner: an object type's attributes or a record's fields.
fn member_type(declarations: &PackageDeclarations, owner: &str, field: &str) -> ValueType {
    let qualified = format!("Config::{owner}");
    if let Some(object) = declarations
        .types
        .object_types()
        .find(|declared| declared.name() == qualified)
    {
        return object
            .attributes()
            .iter()
            .find(|attribute| attribute.name() == field)
            .unwrap_or_else(|| panic!("{owner}.{field} is declared"))
            .value_type()
            .clone();
    }
    let composite = declarations
        .types
        .composites()
        .find(|declared| declared.name() == qualified)
        .unwrap_or_else(|| panic!("{owner} is declared"));
    let filament_semantic_value::declaration::CompositeShape::Record(fields) = composite.shape()
    else {
        panic!("{owner} is a record: {:?}", composite.shape());
    };
    fields
        .iter()
        .find(|declared| declared.name() == field)
        .unwrap_or_else(|| panic!("{owner}.{field} is declared"))
        .value_type()
        .clone()
}

/// FR-056-AC-34 (TC-992 step 1): over TC-235's K2 package, assembled without
/// rewriting its source, `Money.currency` has the checked type `String`, and
/// `Order.order_id`, `Fulfilment.correlation_id` (the identity slots) and the
/// ordinary event field `OrderPlaced.order_id` have the checked type `UUID`.
/// The two are distinct first-class leaves: neither is `Text` or a
/// `Reference`, and the two types differ from each other. Changing the
/// declared native name of one member changes its checked type, so the
/// mapping is read from the declaration and not fixed.
#[trace("TC-992", "FR-056-AC-34")]
#[test]
fn k2_string_and_uuid_members_map_to_distinct_value_leaves() {
    let declarations = assembled(&base::k2());
    let currency = member_type(&declarations, "Money", "currency");
    let order = member_type(&declarations, "Order", "order_id");
    let correlation = member_type(&declarations, "Fulfilment", "correlation_id");
    let placed = member_type(&declarations, "OrderPlaced", "order_id");
    assert_eq!(currency, ValueType::String);
    for (name, found) in [
        ("Order.order_id", &order),
        ("Fulfilment.correlation_id", &correlation),
        ("OrderPlaced.order_id", &placed),
    ] {
        assert_eq!(found, &ValueType::Uuid, "{name}");
    }
    assert_ne!(currency, order, "String and UUID are different leaves");
    for leaf in [&currency, &order, &correlation, &placed] {
        assert!(
            !matches!(leaf, ValueType::Reference(_) | ValueType::Text(_)),
            "a native leaf is never Text or Reference: {leaf:?}"
        );
    }

    // The mapping follows the declaration: the same package with
    // `Money.currency` declared UUID types it UUID, and `Order.order_id`
    // declared String types it String.
    let mut swapped = base::k2();
    for (owner, field, native) in [
        ("Money", "currency", "ix://filament/native/UUID"),
        ("Order", "order_id", "ix://filament/native/String"),
    ] {
        let declared = type_mut(&mut swapped, owner)["fields"]
            .as_array_mut()
            .expect("fields")
            .iter_mut()
            .find(|each| each["name"] == field)
            .expect("the field");
        declared["typeRef"] = json!(native);
    }
    let declarations = assembled(&swapped);
    assert_eq!(
        member_type(&declarations, "Money", "currency"),
        ValueType::Uuid
    );
    assert_eq!(
        member_type(&declarations, "Order", "order_id"),
        ValueType::String
    );
}
