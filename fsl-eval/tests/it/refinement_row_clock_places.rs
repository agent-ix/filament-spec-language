// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-1040-AC-3 (TC-3084): the population forms over `ref(c.x)` in a
//! refinement row evaluate as FSpec FR-829-AC-2 states, over the timed
//! `Call` of `tests/support/ring_queue.rs` with `calls = {c1, c2}`.

use std::collections::BTreeMap;

use filament_eval::FamilyOutcome;
use filament_exact::{Integer, Meter, ObjectId, ObjectReference, Outcome, ScalarLimits, Value};
use fsl_eval::value::evaluate_refinement_row;
use fsl_package::CheckedPackage;
use fsl_semantics::model::accounting::ModelNormalizationLimits;
use fsl_semantics::model::observation::{declared_type_universe_for, DocumentRef};
use fsl_semantics::model::state::{ModelState, StateModel, StateObject};
use ix_trace_rs::trace;

use crate::ring_queue::{check_clock_refinement, clock_refinement, key, CLOCKS};

const UNLIMITED: ScalarLimits = ScalarLimits {
    integer_bits: 1 << 20,
    decimal_digits: 1 << 20,
    scale_expansion: 1 << 20,
    text_input_bytes: 1 << 20,
    text_scalars: 1 << 20,
    normalized_scalars: 1 << 20,
    unit_edges: 1 << 20,
    value_occurrences: 1 << 20,
    work_units: 1 << 30,
    result_units: 1 << 30,
};

/// The `Queue` field rows, in field order: `used` then `busy`.
const USED: usize = 0;
const BUSY: usize = 1;

/// The state with `c1` (`n = 1`) and `c2` (`n = 2`). A clock has no data
/// slot, so a state that differs only in clock values is this same state.
fn state() -> ModelState {
    let call = |n: i64| StateObject {
        type_identity: key(CLOCKS, "Call"),
        fields: BTreeMap::from([
            (key(CLOCKS, "Call/n"), Value::Integer(Integer::from(n))),
            (key(CLOCKS, "Call/d"), Value::Integer(Integer::from(0_i64))),
        ]),
    };
    ModelState {
        populations: BTreeMap::from([(
            key(CLOCKS, "calls"),
            BTreeMap::from([("c1".to_owned(), call(1)), ("c2".to_owned(), call(2))]),
        )]),
        extents: Vec::new(),
    }
}

/// The row `index` of the `Queue` mapping evaluated with `self` the call
/// `receiver`, rendered.
fn evaluate(row: usize, receiver: &str) -> String {
    let checked = check_clock_refinement(&clock_refinement(
        "used = count(c in C::calls where ref(c.x) = ref(self.x));\n\
         busy = only(c in C::calls where c.n = 2: ref(c.x)) = ref(self.x);",
    ))
    .unwrap_or_else(|refusals| {
        panic!(
            "the rows check: {:?}",
            refusals
                .iter()
                .map(|found| &found.refusal)
                .collect::<Vec<_>>()
        )
    });
    let package = CheckedPackage::link(checked.graph);
    format!(
        "{:?}",
        evaluate_row(&package, &checked.packages, row, receiver, &state())
    )
}

/// Evaluate a row against supplied state, keeping the checked package fixed
/// while a reference field is updated.
fn evaluate_row(
    package: &CheckedPackage,
    packages: &fsl_semantics::model::intake::PackageInput,
    row: usize,
    receiver: &str,
    state: &ModelState,
) -> Value {
    let graph = package.graph();
    let (effective, universe) = declared_type_universe_for(
        graph.model_inputs(),
        packages,
        ModelNormalizationLimits::default(),
        &key(CLOCKS, "Call"),
    )
    .expect("the universe derives")
    .expect("the type is declared");
    let receiver =
        ObjectReference::new(universe, effective, ObjectId::new(receiver).expect("an id"));
    let observed = StateModel::new(
        graph.model_inputs(),
        graph.scope().types(),
        packages,
        ModelNormalizationLimits::default(),
    )
    .expect("model views")
    .observation(
        state,
        DocumentRef {
            authority: "test".to_owned(),
            identity: "state".to_owned(),
        },
    )
    .expect("the state observes")
    .environment;
    let [refinement] = graph.refinements() else {
        panic!("one refinement");
    };
    let [queue, _cell] = refinement.objects() else {
        panic!("the queue and the cell rows");
    };
    let mut meter = Meter::new(UNLIMITED);
    let evaluation = evaluate_refinement_row(
        package,
        &queue.fields[row].expression,
        &observed,
        vec![Value::Reference(receiver)],
        &mut meter,
    )
    .expect("the row evaluates");
    match evaluation.outcome {
        FamilyOutcome::Evaluated(Outcome::Completed(value)) => value,
        other => panic!("row {row} completes: {other:?}"),
    }
}

/// FR-1040-AC-3 (FSpec FR-829-AC-2): over `calls = {c1, c2}`, `only(c in
/// M::calls where c.n = 2: ref(c.x))` is the place id of `c2.x`, so it equals
/// `ref(self.x)` for `c2` and not for `c1`; `count(c in M::calls where
/// ref(c.x) = ref(self.x))` is `1` for either receiver, as each call holds
/// one place `x`.
#[trace("TC-3084", "FR-1040-AC-3")]
#[test]
fn the_refinement_row_place_forms_evaluate_to_the_place_ids() {
    let (true_, false_, one) = (
        format!("{:?}", Value::Boolean(true)),
        format!("{:?}", Value::Boolean(false)),
        format!("{:?}", Value::Integer(Integer::from(1_i64))),
    );
    assert_eq!(evaluate(BUSY, "c2"), true_);
    assert_eq!(evaluate(BUSY, "c1"), false_);
    assert_eq!(evaluate(USED, "c1"), one);
    assert_eq!(evaluate(USED, "c2"), one);
}

/// The TC-3084 clock fixture with a stored reference and a row that returns
/// an actual evaluated place id for snapshot admission tests.
fn clock_reference_fixture(wrappers: bool) -> crate::ring_queue::Checked {
    clock_reference_fixture_variant(wrappers, false, false)
}

fn clock_reference_fixture_variant(
    wrappers: bool,
    remove_x: bool,
    map: bool,
) -> crate::ring_queue::Checked {
    use crate::ring_queue::{self, QUEUE};
    use serde_json::{json, Value as Json};

    let with_reference = |bytes: Vec<u8>, domain: &str, owner: &str, field: &str| {
        let mut document: Json = serde_json::from_slice(&bytes).expect("JSON");
        let reference = ring_queue::node(domain, "ClockRef");
        let mut definition = ring_queue::declaration(
            domain,
            "ClockRef",
            "unused",
            json!({"target": "ix://filament/native/Clock"}),
        );
        definition["kind"] = json!("reference");
        document["types"]
            .as_array_mut()
            .expect("types")
            .push(definition);
        let ty = document["types"]
            .as_array_mut()
            .expect("types")
            .iter_mut()
            .find(|ty| ty["identity"] == ring_queue::node(domain, owner))
            .expect("owner");
        ty["fields"]
            .as_array_mut()
            .expect("fields")
            .push(ring_queue::single(domain, owner, field, &reference));
        if wrappers && domain == CLOCKS {
            let mut maybe = ring_queue::single(domain, owner, "maybe", &reference);
            maybe["presence"] = json!("optional");
            let mut refs = ring_queue::single(domain, owner, "refs", &reference);
            refs["multiplicity"] =
                json!({"lower": 0, "upper": 3, "ordered": true, "unique": false});
            let mut peer =
                ring_queue::single(domain, owner, "peer", &ring_queue::node(domain, owner));
            peer["presence"] = json!("optional");
            ty["fields"]
                .as_array_mut()
                .expect("fields")
                .extend([maybe, refs, peer]);
        }
        serde_json::to_vec(&document).expect("JSON")
    };
    let queue = with_reference(ring_queue::queue_spec(), QUEUE, "Queue", "chosen");
    let mut calls: Json = serde_json::from_slice(&with_reference(
        ring_queue::clock_calls(),
        CLOCKS,
        "Call",
        "r",
    ))
    .expect("JSON");
    let fields = calls["types"]
        .as_array_mut()
        .expect("types")
        .iter_mut()
        .find(|ty| ty["identity"] == ring_queue::node(CLOCKS, "Call"))
        .expect("Call")["fields"]
        .as_array_mut()
        .expect("fields");
    let mut y = ring_queue::single(CLOCKS, "Call", "y", "ix://filament/native/Clock");
    y["multiplicity"]["unique"] = json!(false);
    fields.push(y);
    if remove_x {
        fields.retain(|field| field["name"] != "x");
    }
    if map {
        let map_identity = ring_queue::node(CLOCKS, "ClockMap");
        fields.push(ring_queue::single(CLOCKS, "Call", "mo", &map_identity));
        let mut declaration = ring_queue::declaration(
            CLOCKS,
            "ClockMap",
            "unused",
            json!({"keys":"ix://filament/native/Text", "values":"ix://filament/native/Clock", "bound":{"lower":0,"upper":2}}),
        );
        declaration["kind"] = json!("map");
        calls["types"]
            .as_array_mut()
            .expect("types")
            .push(declaration);
    }
    let calls = serde_json::to_vec(&calls).expect("JSON");
    let rows = "used = count(c in C::calls where ref(c.x) = self.r);\n\
         busy = only(c in C::calls where c.n = 2: ref(c.x)) = self.r;\n\
         chosen = ref(self.x);";
    let rows = if remove_x {
        rows.replace(".x", ".y")
    } else {
        rows.to_owned()
    };
    ring_queue::check_models(
        &queue,
        ("C", CLOCKS, &calls),
        true,
        &clock_refinement(&rows),
    )
    .unwrap_or_else(|refusals| {
        panic!(
            "reference-field rows check: {:?}",
            refusals
                .iter()
                .map(|found| &found.refusal)
                .collect::<Vec<_>>()
        )
    })
}

/// A stored clock reference is ordinary data: the population count reads the
/// stored place id, and changing that field changes which place is selected.
#[trace("TC-3084", "FR-1040-AC-3", "FR-1040-AC-8")]
#[test]
fn a_refinement_count_reads_the_stored_clock_reference() {
    let checked = clock_reference_fixture(false);
    let package = CheckedPackage::link(checked.graph);
    let c1 = generated_place(&package, &checked.packages, "c1");
    let c2 = generated_place(&package, &checked.packages, "c2");
    let mut state = state();
    assert_eq!(c1.universe(), c2.universe());
    assert_eq!(c1.object_type(), c2.object_type());
    assert_ne!(
        c1.object(),
        c2.object(),
        "the owner distinguishes clock place ids"
    );
    let set_reference = |state: &mut ModelState, reference: ObjectReference| {
        for call in state
            .populations
            .get_mut(&key(CLOCKS, "calls"))
            .expect("calls")
            .values_mut()
        {
            call.fields
                .insert(key(CLOCKS, "Call/r"), Value::Reference(reference.clone()));
        }
    };
    for (reference, receiver, expected_busy) in [(c1, "c2", false), (c2, "c1", true)] {
        set_reference(&mut state, reference);
        let Value::Integer(count) =
            evaluate_row(&package, &checked.packages, USED, receiver, &state)
        else {
            panic!("count row must return Integer")
        };
        assert_eq!(count, Integer::from(1_i64));
        let Value::Boolean(busy) =
            evaluate_row(&package, &checked.packages, BUSY, receiver, &state)
        else {
            panic!("busy row must return Boolean")
        };
        assert_eq!(busy, expected_busy);
    }
}

/// Run the actual `chosen = ref(self.x)` row with its owner binder alone.
fn generated_place(
    package: &CheckedPackage,
    packages: &fsl_semantics::model::intake::PackageInput,
    owner: &str,
) -> ObjectReference {
    let graph = package.graph();
    let (effective, universe) = declared_type_universe_for(
        graph.model_inputs(),
        packages,
        ModelNormalizationLimits::default(),
        &key(CLOCKS, "Call"),
    )
    .expect("universe")
    .expect("Call");
    let owner = ObjectReference::new(universe, effective, ObjectId::new(owner).expect("id"));
    let [refinement] = graph.refinements() else {
        panic!("one refinement")
    };
    let [queue, _] = refinement.objects() else {
        panic!("Queue and Cell")
    };
    let mut meter = Meter::new(UNLIMITED);
    let evaluation = evaluate_refinement_row(
        package,
        &queue.fields[2].expression,
        &fsl_semantics::model::object_environment::ObjectEnvironment::default(),
        vec![Value::Reference(owner)],
        &mut meter,
    )
    .expect("place row evaluates");
    match evaluation.outcome {
        FamilyOutcome::Evaluated(Outcome::Completed(Value::Reference(reference))) => reference,
        other => panic!("place row returns a reference: {other:?}"),
    }
}

/// The existing raw reference form names a place in a population's owner
/// context, including optional and sequence data fields.
fn clock_snapshot(reference: &ObjectReference) -> (DocumentRef, Vec<u8>) {
    use serde_json::json;
    let identity = DocumentRef {
        authority: "test".to_owned(),
        identity: "clock-snapshot".to_owned(),
    };
    let r = json!({"reference": {"population": crate::ring_queue::node(CLOCKS, "calls"), "key": reference.object().as_str()}});
    let bytes = serde_json::to_vec(&json!({
        "format": "filament.state.snapshot/v1", "identity": {"authority": identity.authority, "identity": identity.identity},
        "observation": "current", "model": {"identity": CLOCKS},
        "anchor": {"kind": "initialization", "name": "start"},
        "populations": [{"population": crate::ring_queue::node(CLOCKS, "calls"), "complete": true,
            "objects": [{"key": "c1", "type": crate::ring_queue::node(CLOCKS, "Call"), "fields": {
                "n": {"integer": "1"}, "d": {"integer": "0"}, "r": r,
                "maybe": {"present": r}, "refs": {"sequence": [r]}, "peer": {"absent": {}},
            }}]}],
    })).expect("JSON");
    (identity, bytes)
}

/// Only a declared place instance is admitted as clock-reference data;
/// its owner is not resolved as an object of type Clock.
#[trace("TC-464", "FR-106-AC-72", "TC-3084", "FR-1040-AC-8")]
#[test]
#[ignore = "QSL-2178 STEP2: route-neutral external Clock place identity required"]
fn clock_reference_snapshot_accepts_a_declared_place() {
    use serde_json::{json, Value as Json};
    let checked = clock_reference_fixture(true);
    let package = CheckedPackage::link(checked.graph);
    let reference = generated_place(&package, &checked.packages, "c1");
    let second = generated_place(&package, &checked.packages, "c2");
    assert_ne!(reference, second, "owners have distinct place identities");
    let (identity, bytes) = clock_snapshot(&reference);
    let graph = package.graph();
    let model = StateModel::new(
        graph.model_inputs(),
        graph.scope().types(),
        &checked.packages,
        ModelNormalizationLimits::default(),
    )
    .expect("model");
    for present in [true, false] {
        let mut document: Json = serde_json::from_slice(&bytes).expect("snapshot");
        let r1 = json!({"reference": {"population": crate::ring_queue::node(CLOCKS,"calls"), "key": reference.object().as_str()}});
        let r2 = json!({"reference": {"population": crate::ring_queue::node(CLOCKS,"calls"), "key": second.object().as_str()}});
        let mut c2 = document["populations"][0]["objects"][0].clone();
        c2["key"] = json!("c2");
        c2["fields"]["n"] = json!({"integer":"2"});
        c2["fields"]["r"] = r2.clone();
        c2["fields"]["maybe"] = json!({"absent":{}});
        c2["fields"]["refs"] = json!({"sequence":[r2,r1]});
        document["populations"][0]["objects"][0]["fields"]["maybe"] = if present {
            json!({"present":r1})
        } else {
            json!({"absent":{}})
        };
        document["populations"][0]["objects"][0]["fields"]["refs"] = json!({"sequence":[r2,r1]});
        document["populations"][0]["objects"]
            .as_array_mut()
            .expect("objects")
            .push(c2);
        let (state, _) = model
            .admit_initial(
                &BTreeMap::from([(
                    identity.clone(),
                    serde_json::to_vec(&document).expect("snapshot"),
                )]),
                &identity,
                fsl_semantics::model::observation::ObservationLimits::default(),
            )
            .expect("declared clock place admits");
        let call = &state.populations[&key(CLOCKS, "calls")]["c1"];
        let assert_reference = |value: &Value, expected: &ObjectReference| {
            let Value::Reference(stored) = value else {
                panic!("stored value must be Reference")
            };
            assert_eq!(stored, expected);
        };
        assert_reference(&call.fields[&key(CLOCKS, "Call/r")], &reference);
        assert_reference(
            &state.populations[&key(CLOCKS, "calls")]["c2"].fields[&key(CLOCKS, "Call/r")],
            &second,
        );
        // Optional fields retain presence by membership in StateObject.fields.
        if present {
            assert_reference(
                call.fields
                    .get(&key(CLOCKS, "Call/maybe"))
                    .expect("optional field is present"),
                &reference,
            );
        } else {
            assert!(!call.fields.contains_key(&key(CLOCKS, "Call/maybe")));
        }
        let Value::Collection(refs) = &call.fields[&key(CLOCKS, "Call/refs")] else {
            panic!("refs must be a sequence")
        };
        assert_eq!(
            refs.collection_type().kind(),
            filament_exact::CollectionKind::Sequence
        );
        let [first, last] = refs.elements() else {
            panic!("two references in sequence order")
        };
        assert_reference(first, &second);
        assert_reference(last, &reference);
        model
            .observation(&state, identity.clone())
            .expect("stored references remain data on readback");
        assert!(matches!(
            evaluate_row(&package, &checked.packages, BUSY, "c1", &state),
            Value::Boolean(false)
        ));
        assert!(matches!(
            evaluate_row(&package, &checked.packages, BUSY, "c2", &state),
            Value::Boolean(true)
        ));
    }
}

/// A well-formed place id of another owner instance, and an ordinary
/// dangling object reference, both retain the dangling-reference boundary.
#[trace("TC-464", "FR-106-AC-73")]
#[test]
#[ignore = "QSL-2178 STEP2: route-neutral external Clock place identity required"]
fn clock_reference_snapshot_refuses_an_undeclared_place() {
    use serde_json::{json, Value as Json};
    let old = clock_reference_fixture(true);
    let old_package = CheckedPackage::link(old.graph);
    let valid = generated_place(&old_package, &old.packages, "c1");
    let checked = clock_reference_fixture_variant(true, true, false);
    let package = CheckedPackage::link(checked.graph);
    let graph = package.graph();
    let model = StateModel::new(
        graph.model_inputs(),
        graph.scope().types(),
        &checked.packages,
        ModelNormalizationLimits::default(),
    )
    .expect("model");
    let y = generated_place(&package, &checked.packages, "c1");
    let (identity, bytes) = clock_snapshot(&y);
    for field in ["r", "maybe", "refs"] {
        let mut document: Json = serde_json::from_slice(&bytes).expect("JSON");
        let reference = json!({"reference": {"population": crate::ring_queue::node(CLOCKS, "calls"),
            "key": valid.object().as_str()}});
        document["populations"][0]["objects"][0]["fields"][field] = match field {
            "maybe" => json!({"present": reference}),
            "refs" => json!({"sequence": [reference]}),
            _ => reference,
        };
        let failure = model
            .admit_initial(
                &BTreeMap::from([(
                    identity.clone(),
                    serde_json::to_vec(&document).expect("JSON"),
                )]),
                &identity,
                fsl_semantics::model::observation::ObservationLimits::default(),
            )
            .expect_err("undeclared place/object refuses");
        let fsl_semantics::model::observation::AdmissionFailure::Refused(record) = failure else {
            panic!("{field}: {failure:?}")
        };
        use filament_semantic_value::admission::{AdmissionRecord, PathStep};
        let mut path = vec![
            PathStep::Field("populations".to_owned()),
            PathStep::Index(0),
            PathStep::Field("objects".to_owned()),
            PathStep::Index(0),
            PathStep::Field("fields".to_owned()),
            PathStep::Field(field.to_owned()),
        ];
        let suffix = match field {
            "maybe" => {
                path.push(PathStep::OptionValue);
                "/present"
            }
            "refs" => {
                path.push(PathStep::Index(0));
                "/sequence/0"
            }
            _ => "",
        };
        let expected = AdmissionRecord {
            catalog: filament_semantic_value::catalog::CatalogCode::new(
                "dangling_reference",
                "absent-target-in-complete-population",
            ),
            fields: BTreeMap::from([
                ("document", identity.identity.clone()),
                ("population", crate::ring_queue::node(CLOCKS, "calls")),
                ("object", "c1".to_owned()),
                ("field", field.to_owned()),
                ("target", valid.object().as_str().to_owned()),
                (
                    "path",
                    format!("/populations/0/objects/0/fields/{field}{suffix}"),
                ),
            ]),
            path,
            kernel: None,
        };
        assert_eq!(
            *record, expected,
            "exact pending STEP2 undeclared-place refusal"
        );
    }
}

/// Bare observations cannot authenticate the source codec's template key.
#[trace("FR-531-AC-1", "FR-1040-AC-8")]
#[test]
fn clock_reference_external_observations_are_explicitly_unsupported() {
    use filament_semantic_value::admission::ObservationAdmission;
    use fsl_semantics::model::observation::{AdmissionFailure, ObservationLimits};
    use serde_json::json;
    let checked = clock_reference_fixture(true);
    let package = CheckedPackage::link(checked.graph);
    let read = fsl_package::read_evaluation_package(
        &package,
        fsl_semantics::library::LibraryName::new(package.graph().package_name()).expect("name"),
        &checked.packages,
        fsl_package::V2ReadLimits::fixture(),
        fsl_package::EmitLimits::fixture(),
    )
    .expect("actual emitted bytes admit");
    let bare = fsl_package::BarePackage::new(read.admitted());
    let reference = generated_place(&package, &checked.packages, "c1");
    let (identity, bytes) = clock_snapshot(&reference);
    let snapshots = BTreeMap::from([(identity.clone(), bytes)]);
    let model = fsl_semantics::model::domain_package::DomainPackageRef {
        identity: CLOCKS.to_owned(),
    };
    let assert_unsupported = |record: &filament_semantic_value::admission::AdmissionRecord| {
        assert_eq!(
            record.catalog,
            filament_semantic_value::catalog::CatalogCode::new(
                "unknown_required_feature",
                "unsupported-feature"
            )
        );
        assert_eq!(
            record.fields.get("reason").map(String::as_str),
            Some("missing-route-neutral-place-identity")
        );
    };
    let source_model = StateModel::new(
        package.graph().model_inputs(),
        package.graph().scope().types(),
        &checked.packages,
        ModelNormalizationLimits::default(),
    )
    .expect("source model");
    let Err(AdmissionFailure::Refused(record)) =
        source_model.admit_initial(&snapshots, &identity, ObservationLimits::default())
    else {
        panic!("source raw snapshot must refuse external Clock identity")
    };
    assert_unsupported(&record);
    let raw = json!({"reference":{"population":crate::ring_queue::node(CLOCKS,"calls"),"key":reference.object().as_str()}});
    let target = filament_exact::ValueType::Reference(reference.object_type().clone());
    for (value_type, value) in [
        (target.clone(), raw.clone()),
        (
            filament_exact::ValueType::Option(std::sync::Arc::new(target.clone())),
            json!({"present":raw}),
        ),
        (
            filament_exact::ValueType::collection(filament_exact::CollectionType::new(
                filament_exact::CollectionKind::Sequence,
                target.clone(),
                Some(filament_exact::CardinalityBound::new(0, 3).expect("bound")),
            )),
            json!({"sequence":[raw]}),
        ),
    ] {
        use filament_semantic_value::declaration::{
            CompositeDeclaration, CompositeShape, FieldDeclaration, TypeEnvironment,
        };
        let record_key = filament_exact::NodeKey::new(
            "external-clock-fixture",
            vec![filament_exact::PathSegment::Text("Record".to_owned())],
            0,
        )
        .expect("record key");
        let types = TypeEnvironment::new(
            [CompositeDeclaration::new(
                record_key.clone(),
                "Record",
                CompositeShape::Record(vec![FieldDeclaration::new(
                    "r",
                    value_type,
                    filament_exact::Presence::Required,
                )]),
            )],
            [],
        )
        .expect("typed record");
        let record = fsl_semantics::model::observation::admit_record_sample(
            &types,
            &record_key,
            &serde_json::to_vec(&json!({"r":value})).expect("sample"),
        )
        .expect_err("record Clock reference is external");
        assert_unsupported(&record);
    }

    let document = crate::tc235_business_meaning_runtime::lifecycle_document_with(
        "true",
        |document| {
            use crate::tc235_k2::{declaration, field, id, origin, type_named_mut};
            let mut clock_ref = declaration(
                "ClockRef",
                "unused",
                json!({"target":"ix://filament/native/Clock"}),
            );
            clock_ref["kind"] = json!("reference");
            document["types"]
                .as_array_mut()
                .expect("types")
                .push(clock_ref);
            let machine = type_named_mut(document, "OrderLifecycle");
            let mut r = field("OrderLifecycle", "r", &id("ClockRef"));
            r["presence"] = json!("optional");
            machine["fields"].as_array_mut().expect("fields").push(r);
            machine["operations"][0]["params"] = json!([{"identity":id("OrderLifecycle/place/target"),"name":"target","typeRef":id("ClockRef"),"presence":"required","nullable":false,"defaultKind":"none","multiplicity":{"lower":1,"upper":1,"ordered":false,"unique":true},"origin":origin("OrderLifecycle/place/target")}]);
        },
    );
    let (context_package,_,context_packages) = crate::tc235_business_meaning_runtime::model_package_unpinned(&document,"invariant ContextClock using v on M::OrderLifecycle at current { true }\npre Target using v on M::OrderLifecycle::place { true }").expect("typed Clock context and parameter");
    for parameter in [false, true] {
        use fsl_semantics::model::observation::{
            AnchorKind, ClauseSelectionInput, SelectedAnchor, SelectedObject, SnapshotValue,
        };
        let selected =
            crate::tc235_business_meaning_runtime::lifecycle_document_ref("clock-external");
        let mut snapshot = json!({"format":"filament.state.snapshot/v1","identity":crate::tc235_business_meaning_runtime::lifecycle_identity(&selected),"observation":if parameter {"pre"} else {"current"},"model":{"identity":crate::tc235_k2::PACKAGE},"populations":[],"contexts":[{"machine":crate::tc235_k2::id("OrderLifecycle"),"fields":{"placed":{"boolean":false},"cancelled":{"boolean":false},"r":if parameter {json!({"absent":{}})} else {json!({"present":raw})}}}]});
        if !parameter {
            snapshot["anchor"] = json!({"kind":"handler","name":"validate"});
        }
        let snapshots = BTreeMap::from([(
            selected.clone(),
            serde_json::to_vec(&snapshot).expect("snapshot"),
        )]);
        let input = if parameter {
            ClauseSelectionInput::PreCall {
                snapshot: selected,
                self_object: None,
                parameters: BTreeMap::from([(
                    filament_exact::Identifier::new("target").expect("identifier"),
                    SnapshotValue::reference(SelectedObject {
                        population: crate::ring_queue::node(CLOCKS, "calls"),
                        key: reference.object().as_str().to_owned(),
                    }),
                )]),
            }
        } else {
            ClauseSelectionInput::Current {
                snapshot: selected,
                anchor: SelectedAnchor {
                    kind: AnchorKind::Handler,
                    name: "validate".to_owned(),
                },
                self_object: None,
            }
        };
        let Err(AdmissionFailure::Refused(record)) =
            crate::tc235_business_meaning_runtime::lifecycle_selection(
                &context_package,
                &context_packages,
                if parameter { "Target" } else { "ContextClock" },
                &snapshots,
                &BTreeMap::new(),
                input,
            )
        else {
            panic!("external Clock context/parameter must refuse")
        };
        assert_unsupported(&record);
    }
    let Err(AdmissionFailure::Refused(record)) = bare.admit_call_snapshot(
        &checked.packages,
        ModelNormalizationLimits::default(),
        &snapshots,
        &identity,
        ObservationLimits::default(),
    ) else {
        panic!("Bare call snapshot must name missing authority")
    };
    assert_unsupported(&record);
    let ObservationAdmission::Refused(record) = bare.admit_protocol_observation(
        &model,
        &checked.packages,
        ModelNormalizationLimits::default(),
        &snapshots,
        &identity,
        ObservationLimits::default(),
    ) else {
        panic!("Bare protocol snapshot must name missing authority")
    };
    assert_unsupported(&record);
    let ObservationAdmission::Refused(record) = bare.admit_protocol_state(
        &model,
        &checked.packages,
        ModelNormalizationLimits::default(),
        &state(),
        identity,
    ) else {
        panic!("Bare simulated state must name missing authority")
    };
    assert_unsupported(&record);

    let ordinary = check_clock_refinement(&clock_refinement("used = 0; busy = false;"))
        .unwrap_or_else(|refusals| {
            panic!(
                "ordinary fixture: {:?}",
                refusals
                    .iter()
                    .map(|refusal| &refusal.refusal)
                    .collect::<Vec<_>>()
            )
        });
    let ordinary_package = CheckedPackage::link(ordinary.graph);
    let ordinary_read = fsl_package::read_evaluation_package(
        &ordinary_package,
        fsl_semantics::library::LibraryName::new(ordinary_package.graph().package_name())
            .expect("name"),
        &ordinary.packages,
        fsl_package::V2ReadLimits::fixture(),
        fsl_package::EmitLimits::fixture(),
    )
    .expect("ordinary bytes");
    let emission = fsl_package::emit_checked(&ordinary_package, fsl_package::EmitLimits::fixture())
        .expect("emits");
    let mut budget = fsl_package::V2ReadLimits::fixture().native_join_budget();
    let rows = emission
        .identity_rows(ordinary_read.admitted(), &mut budget)
        .expect("actual identity rows");
    let equality = emission
        .equality_witness(ordinary_read.admitted(), &mut budget)
        .expect("actual equality witness");
    let ordinary_bare = fsl_package::BarePackage::new(ordinary_read.admitted())
        .with_supply(
            fsl_package::BareSupply {
                identities: &rows,
                equality: Some(&equality),
            },
            &mut budget,
        )
        .expect("typed rows");
    let model = fsl_semantics::model::domain_package::DomainPackageRef {
        identity: CLOCKS.to_owned(),
    };
    assert!(
        matches!(
            ordinary_bare.admit_protocol_state(
                &model,
                &ordinary.packages,
                ModelNormalizationLimits::default(),
                &state(),
                DocumentRef {
                    authority: "test".to_owned(),
                    identity: "ordinary".to_owned()
                }
            ),
            ObservationAdmission::Admitted(_)
        ),
        "ordinary Bare state admission remains supported"
    );
}

/// A map layout never authorizes a numerically forged clock-place identity.
#[trace("TC-3084", "FR-1040-AC-8", "FR-230-AC-62")]
#[test]
fn clock_reference_admission_refuses_map_place_forgery() {
    use fsl_semantics::model::clock_reference::{self, DeclaredClockPlaces};
    use fsl_semantics::model::observation::{AdmissionFailure, ObservationLimits};
    let checked = clock_reference_fixture_variant(true, false, true);
    let package = CheckedPackage::link(checked.graph);
    let graph = package.graph();
    let model = StateModel::new(
        graph.model_inputs(),
        graph.scope().types(),
        &checked.packages,
        ModelNormalizationLimits::default(),
    )
    .expect("model");
    let owner = model
        .object_reference(&key(CLOCKS, "Call"), "c1")
        .expect("actual owner");
    let template = graph
        .timed_models()
        .iter()
        .flat_map(|model| model.clocks().iter().zip(model.place_nodes()))
        .find(|(clock, _)| clock.slots().any(|shape| shape.map))
        .expect("actual map template")
        .1;
    let forged = clock_reference::encode(&owner, template, &[Value::Integer(Integer::from(0_i64))])
        .expect("syntactically encoded forged numeric slot");
    assert!(
        !DeclaredClockPlaces::new(graph.timed_models()).contains(
            &forged,
            &owner,
            graph.scope().types()
        ),
        "a map template never authenticates a numeric place ID"
    );
    let (identity, bytes) = clock_snapshot(&forged);
    let Err(AdmissionFailure::Refused(record)) = model.admit_initial(
        &BTreeMap::from([(identity.clone(), bytes)]),
        &identity,
        ObservationLimits::default(),
    ) else {
        panic!("raw map-place value must refuse")
    };
    assert_eq!(
        record.catalog,
        filament_semantic_value::catalog::CatalogCode::new(
            "unknown_required_feature",
            "unsupported-feature"
        )
    );
    assert_eq!(
        record.fields.get("reason").map(String::as_str),
        Some("missing-route-neutral-place-identity")
    );
}
