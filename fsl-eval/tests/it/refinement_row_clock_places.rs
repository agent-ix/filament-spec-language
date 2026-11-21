// SPDX-License-Identifier: AGPL-3.0-or-later
//! Stored Clock references remain data; minting a place requires explicit
//! population provenance, which the legacy refinement-row entry lacks.

use std::collections::BTreeMap;

use filament_eval::{Evaluation, FamilyOutcome, FamilyResult};
use filament_exact::{Integer, Meter, ObjectId, ObjectReference, Outcome, ScalarLimits, Value};
use fsl_eval::value::{
    evaluate_refinement_row, CheckedPackageEvaluation, ProtocolReferenceOccurrence, ProtocolSlot,
    ProtocolSlotEvaluation, ProtocolSuppliedValue, ProtocolSupply, SuppliedValue,
};
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

/// The row `index` evaluated with an unseeded `self` receiver.
fn evaluate(row: usize, receiver: &str) -> Evaluation {
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
    evaluate_row_outcome(&package, &checked.packages, row, receiver, &state())
}

/// Evaluate a row against supplied state, keeping the checked package fixed
/// while a reference field is updated.
fn evaluate_row_outcome(
    package: &CheckedPackage,
    packages: &fsl_semantics::model::intake::PackageInput,
    row: usize,
    receiver: &str,
    state: &ModelState,
) -> Evaluation {
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
    evaluate_refinement_row(
        package,
        &queue.fields[row].expression,
        &observed,
        vec![Value::Reference(receiver)],
        &mut meter,
    )
    .expect("the row returns its typed outcome")
}

fn evaluate_row(
    package: &CheckedPackage,
    packages: &fsl_semantics::model::intake::PackageInput,
    row: usize,
    receiver: &str,
    state: &ModelState,
) -> Value {
    let evaluation = evaluate_row_outcome(package, packages, row, receiver, state);
    match evaluation.outcome {
        FamilyOutcome::Evaluated(Outcome::Completed(value)) => value,
        other => panic!("row {row} completes: {other:?}"),
    }
}

/// The legacy row entry's bare receiver has no authenticated population.
#[trace("TC-3084")]
#[test]
fn unseeded_refinement_receivers_cannot_mint_clock_references() {
    for receiver in ["c1", "c2"] {
        for row in [USED, BUSY] {
            let evaluation = evaluate(row, receiver);
            let FamilyOutcome::FamilyEvaluated(FamilyResult::Refused(cause)) = evaluation.outcome
            else {
                panic!(
                    "unseeded Clock producer must refuse: {:?}",
                    evaluation.outcome
                )
            };
            assert_eq!(
                cause.catalog_code(),
                filament_semantic_value::catalog::CatalogCode::new(
                    "unknown_required_feature",
                    "unsupported-feature"
                )
            );
            assert_eq!(
                cause.catalog_fields(),
                Some(BTreeMap::from([(
                    "reason",
                    "clock-owner-population-authority".to_owned()
                )]))
            );
        }
    }
}

/// The TC-3084 clock fixture with stored reference data and a protocol capture
/// whose supplied owner retains the authenticated population.
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
        let mut mo = ring_queue::single(CLOCKS, "Call", "mo", &map_identity);
        mo["presence"] = json!("optional");
        fields.push(mo);
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
    let selected = if remove_x { "y" } else { "x" };
    let body = format!("{}\n\
        protocol ClockPlaces using p over (call: C::Call) on each (opened: Boolean) when (opened) {{\n\
          capture chosen: Reference<Clock> = ref(call.{selected});\n\
          role Observer on C::Call;\n\
          run sequence Main {{ check Ready using v {{ true }}; }}\n\
          finish End as (outcome: Boolean) {{ true }};\n\
        }}", clock_refinement(&rows));
    ring_queue::check_models_with_profiles(
        &queue,
        ("C", CLOCKS, &calls),
        true,
        "profile p = \"filament.protocol.finite-global/v1\";\n",
        &body,
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
#[trace("TC-3084", "FR-1040-AC-3")]
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
    let state_model = StateModel::new(
        package.graph().model_inputs(),
        package.graph().scope().types(),
        &checked.packages,
        ModelNormalizationLimits::default(),
    )
    .expect("source model");
    for (reference, receiver, expected_busy) in [(c1, "c2", false), (c2, "c1", true)] {
        set_reference(&mut state, reference);
        state_model.protocol_observation(&state,DocumentRef{authority:"test".to_owned(),identity:"run-generated-clock-state".to_owned()})
            .expect("the actual run-generated Clock state observes without requiring Clock object holders");
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

/// The model fixture's independently specified canonical v2 text. Arrays and
/// strings have identical compact serde and RFC 8785 JSON encodings.
fn place_text(owner: &str, path: serde_json::Value, indices: serde_json::Value) -> String {
    use serde_json::json;
    let mut text = "place/v2".to_owned();
    for component in [
        json!(["model", CLOCKS, crate::ring_queue::node(CLOCKS, "calls")]),
        json!(owner),
        json!(["model", CLOCKS, crate::ring_queue::node(CLOCKS, "Call")]),
        path,
        indices,
    ] {
        text.push('/');
        for byte in serde_json::to_vec(&component).expect("canonical component") {
            use std::fmt::Write;
            write!(&mut text, "{byte:02x}").expect("string write");
        }
    }
    text
}

fn declared_place_text(owner: &str, field: &str) -> String {
    use serde_json::json;
    place_text(
        owner,
        json!([[
            "field",
            ["model", CLOCKS, crate::ring_queue::node(CLOCKS, "Call")],
            crate::ring_queue::node(CLOCKS, &format!("Call/{field}")),
            ["clock"]
        ]]),
        json!([]),
    )
}

/// Evaluate the existing protocol slot with provenance validated against the
/// actual admitted population. No K2 type/universe lookup chooses a population.
fn generated_place(
    package: &CheckedPackage,
    packages: &fsl_semantics::model::intake::PackageInput,
    owner: &str,
) -> ObjectReference {
    use filament_semantic_value::admission::ObservationAdmission;
    use fsl_semantics::model::observation::ObservationLimits;
    use serde_json::{json, Value as Json};
    let document: Json = serde_json::from_slice(&packages[CLOCKS]).expect("actual model document");
    let fields = document["types"]
        .as_array()
        .expect("types")
        .iter()
        .find(|ty| ty["identity"] == crate::ring_queue::node(CLOCKS, "Call"))
        .expect("Call")["fields"]
        .as_array()
        .expect("fields");
    let selected = if fields.iter().any(|field| field["name"] == "x") {
        "x"
    } else {
        "y"
    };
    let identity = DocumentRef {
        authority: "test".to_owned(),
        identity: "clock-producer".to_owned(),
    };
    let objects: Vec<_> = [("c1", "1"), ("c2", "2")].into_iter().map(|(owner, n)| {
        let mut data = json!({"n":{"integer":n},"d":{"integer":"0"},"r":{"reference":{"population":crate::ring_queue::node(CLOCKS,"calls"),"key":declared_place_text(owner,selected)}}});
        for field in fields {
            match field["name"].as_str().expect("field name") {
                "maybe" | "peer" => { data[field["name"].as_str().expect("name")] = json!({"absent":{}}); }
                "refs" => data["refs"] = json!({"sequence":[]}),
                "mo" => data["mo"] = json!({"absent":{}}),
                _ => {}
            }
        }
        json!({"key":owner,"type":crate::ring_queue::node(CLOCKS,"Call"),"fields":data})
    }).collect();
    let snapshot = json!({"format":"filament.state.snapshot/v1","identity":{"authority":identity.authority,"identity":identity.identity},"observation":"current","model":{"identity":CLOCKS},"populations":[{"population":crate::ring_queue::node(CLOCKS,"calls"),"complete":true,"objects":objects}]});
    let snapshots = BTreeMap::from([(
        identity.clone(),
        serde_json::to_vec(&snapshot).expect("snapshot"),
    )]);
    let view = package.evaluation_view();
    let admission = view.admit_protocol_observation(
        &fsl_semantics::model::domain_package::DomainPackageRef {
            identity: CLOCKS.to_owned(),
        },
        packages,
        ModelNormalizationLimits::default(),
        &snapshots,
        &identity,
        ObservationLimits::default(),
    );
    let ObservationAdmission::Admitted(admitted) = &admission else {
        panic!("actual producer snapshot admits: {admission:?}")
    };
    let environment = admitted.environment();
    let reference = environment
        .population_members(&crate::ring_queue::node(CLOCKS, "calls"))
        .expect("calls")
        .iter()
        .find(|reference| reference.object().as_str() == owner)
        .expect("actual admitted owner")
        .clone();
    let keys = package
        .graph()
        .protocol_node_keys("ClockPlaces")
        .expect("protocol keys");
    let binder = keys
        .binders
        .iter()
        .find(|binder| binder.name == "call")
        .expect("call binder");
    let supply = ProtocolSupply {
        binders: vec![(
            binder.node.clone(),
            ProtocolSuppliedValue {
                value: SuppliedValue::Value(Value::Reference(reference)),
                references: vec![ProtocolReferenceOccurrence {
                    path: Vec::new(),
                    population: environment
                        .population_node(&crate::ring_queue::node(CLOCKS, "calls"))
                        .expect("authenticated P")
                        .clone(),
                }],
            },
        )],
        observation: Some(&admission),
    };
    let slot = ProtocolSlot {
        node: keys.clause.clone().expect("protocol root"),
        member: "captures".to_owned(),
        index: Some(0),
    };
    let ProtocolSlotEvaluation::Evaluated(evaluation, _) = package
        .evaluate_protocol_slot(&slot, &supply, &mut Meter::new(UNLIMITED))
        .expect("seeded capture evaluates")
    else {
        panic!("capture available")
    };
    match evaluation.outcome {
        FamilyOutcome::Evaluated(Outcome::Completed(Value::Reference(reference))) => {
            assert_eq!(
                reference.object().as_str(),
                declared_place_text(owner, selected)
            );
            reference
        }
        other => panic!("seeded capture returns a reference: {other:?}"),
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
#[trace("TC-464", "FR-106-AC-72", "TC-3084")]
#[test]
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
            json!({"present":r2})
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
                &second,
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
fn clock_reference_snapshot_refuses_an_undeclared_place() {
    use serde_json::{json, Value as Json};
    let old = clock_reference_fixture(true);
    let old_package = CheckedPackage::link(old.graph);
    let valid = generated_place(&old_package, &old.packages, "c1");
    let checked = clock_reference_fixture_variant(true, true, false);
    let package = CheckedPackage::link(checked.graph);
    let graph = package.graph();
    assert_eq!(
        graph
            .timed_models()
            .iter()
            .flat_map(|model| model.clocks().iter())
            .map(|clock| clock.name())
            .collect::<Vec<_>>(),
        ["y"],
        "the real model variant removes x and retains y"
    );
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
        assert_eq!(*record, expected, "exact undeclared-place refusal");
    }
}

/// Model-owned raw references authenticate through the actual declarations;
/// samples without a model catalog cannot resolve the same transported value.
#[trace("FR-531-AC-1", "FR-106-AC-72", "FR-230-AC-45")]
#[test]
fn model_owned_clock_observations_admit_and_unbound_targets_refuse() {
    use filament_semantic_value::admission::ObservationAdmission;
    use fsl_semantics::model::observation::{AdmissionFailure, ObservationLimits};
    use serde_json::{json, Value as Json};
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
    let mut budget = fsl_package::V2ReadLimits::fixture().native_join_budget();
    let rows = read
        .emission()
        .identity_rows(read.admitted(), &mut budget)
        .expect("identity rows");
    let equality = read
        .emission()
        .equality_witness(read.admitted(), &mut budget)
        .expect("equality rows");
    let bare = fsl_package::BarePackage::new(read.admitted())
        .with_supply(
            fsl_package::BareSupply {
                identities: &rows,
                equality: Some(&equality),
            },
            &mut budget,
        )
        .expect("actual Bare supply");
    let reference = generated_place(&package, &checked.packages, "c1");
    let (identity, bytes) = clock_snapshot(&reference);
    let snapshots = BTreeMap::from([(identity.clone(), bytes)]);
    let model = fsl_semantics::model::domain_package::DomainPackageRef {
        identity: CLOCKS.to_owned(),
    };
    let assert_unbound = |record: &filament_semantic_value::admission::AdmissionRecord| {
        assert_eq!(
            record.catalog,
            filament_semantic_value::catalog::CatalogCode::new(
                "dangling_reference",
                "absent-target-in-complete-population"
            )
        );
        assert_eq!(
            record.fields.get("population").map(String::as_str),
            Some(crate::ring_queue::node(CLOCKS, "calls").as_str())
        );
    };
    let source_model = StateModel::new(
        package.graph().model_inputs(),
        package.graph().scope().types(),
        &checked.packages,
        ModelNormalizationLimits::default(),
    )
    .expect("source model");
    let (admitted_state, _) = source_model
        .admit_initial(&snapshots, &identity, ObservationLimits::default())
        .expect("authenticated Model-owned reference admits");
    assert!(
        matches!(&admitted_state.populations[&key(CLOCKS,"calls")]["c1"].fields[&key(CLOCKS,"Call/r")], Value::Reference(stored) if stored == &reference)
    );
    for complete in [true, false] {
        for target in ["not-a-place-id", "place/v2/not-canonical"] {
            let mut snapshot: Json =
                serde_json::from_slice(snapshots.get(&identity).expect("snapshot"))
                    .expect("snapshot JSON");
            snapshot["populations"][0]["complete"] = json!(complete);
            snapshot["populations"][0]["objects"][0]["fields"]["r"]["reference"]["key"] =
                json!(target);
            let Err(AdmissionFailure::Refused(record)) = source_model.admit_initial(
                &BTreeMap::from([(
                    identity.clone(),
                    serde_json::to_vec(&snapshot).expect("snapshot"),
                )]),
                &identity,
                ObservationLimits::default(),
            ) else {
                panic!("malformed v2 Clock value must refuse before completeness")
            };
            use filament_semantic_value::admission::{AdmissionRecord, PathStep};
            assert_eq!(
                *record,
                AdmissionRecord {
                    catalog: filament_semantic_value::catalog::CatalogCode::new(
                        "invalid_runtime_input",
                        "invalid-value"
                    ),
                    fields: BTreeMap::from([
                        ("document", identity.identity.clone()),
                        ("population", crate::ring_queue::node(CLOCKS, "calls")),
                        ("object", "c1".to_owned()),
                        ("field", "r".to_owned()),
                        ("path", "/populations/0/objects/0/fields/r".to_owned()),
                    ]),
                    path: vec![
                        PathStep::Field("populations".to_owned()),
                        PathStep::Index(0),
                        PathStep::Field("objects".to_owned()),
                        PathStep::Index(0),
                        PathStep::Field("fields".to_owned()),
                        PathStep::Field("r".to_owned())
                    ],
                    kernel: None,
                }
            );
        }
    }
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
            package
                .graph()
                .scope()
                .types()
                .composites()
                .cloned()
                .chain([CompositeDeclaration::new(
                    record_key.clone(),
                    "Record",
                    CompositeShape::Record(vec![FieldDeclaration::new(
                        "r",
                        value_type,
                        filament_exact::Presence::Required,
                    )]),
                )]),
            package.graph().scope().types().object_types().cloned(),
        )
        .expect("typed record");
        let record = fsl_semantics::model::observation::admit_record_sample(
            &types,
            &record_key,
            &serde_json::to_vec(&json!({"r":value})).expect("sample"),
        )
        .expect_err("record Clock reference is external");
        assert_unbound(&record);
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
            let mut peer = field("OrderLifecycle", "peer", &id("Order"));
            peer["presence"] = json!("optional");
            machine["fields"].as_array_mut().expect("fields").push(peer);
            machine["operations"][0]["params"] = json!([{"identity":id("OrderLifecycle/place/target"),"name":"target","typeRef":id("ClockRef"),"presence":"required","nullable":false,"defaultKind":"none","multiplicity":{"lower":1,"upper":1,"ordered":false,"unique":true},"origin":origin("OrderLifecycle/place/target")}]);
            document["populations"] = json!([{"identity":id("orders"),"displayName":"orders","kind":{"module":crate::tc235_k2::BUSINESS,"name":"population"},"members":[id("Order")],"extent":"closed","origin":origin("orders")}]);
        },
    );
    let (context_package,_,context_packages) = crate::tc235_business_meaning_runtime::model_package_unpinned(&document,"invariant ContextClock using v on M::OrderLifecycle at current { true }\npre Target using v on M::OrderLifecycle::place { true }").expect("typed Clock context and parameter");
    for mode in ["context", "parameter", "ordinary"] {
        let parameter = mode == "parameter";
        use fsl_semantics::model::observation::{
            ClauseSelectionInput, SelectedObject, SnapshotValue,
        };
        let selected =
            crate::tc235_business_meaning_runtime::lifecycle_document_ref("clock-external");
        let mut snapshot = json!({"format":"filament.state.snapshot/v1","identity":crate::tc235_business_meaning_runtime::lifecycle_identity(&selected),"observation":"pre","model":{"identity":crate::tc235_k2::PACKAGE},"populations":[],"contexts":[{"machine":crate::tc235_k2::id("OrderLifecycle"),"fields":{"placed":{"boolean":false},"cancelled":{"boolean":false},"r":if parameter {json!({"absent":{}})} else {json!({"present":raw})}}}]});
        snapshot["contexts"][0]["fields"]["peer"] = json!({"absent":{}});
        if mode == "ordinary" {
            snapshot["contexts"][0]["fields"]["r"] = json!({"absent":{}});
            snapshot["contexts"][0]["fields"]["peer"] = json!({"present":{"reference":{"population":crate::tc235_k2::id("orders"),"key":"ghost"}}});
            snapshot["populations"] =
                json!([{"population":crate::tc235_k2::id("orders"),"complete":true,"objects":[]}]);
        }
        let snapshots = BTreeMap::from([(
            selected.clone(),
            serde_json::to_vec(&snapshot).expect("snapshot"),
        )]);
        let input = ClauseSelectionInput::PreCall {
            snapshot: selected,
            self_object: None,
            parameters: BTreeMap::from([(
                filament_exact::Identifier::new("target").expect("identifier"),
                SnapshotValue::reference(SelectedObject {
                    population: crate::ring_queue::node(CLOCKS, "calls"),
                    key: reference.object().as_str().to_owned(),
                }),
            )]),
        };
        let Err(AdmissionFailure::Refused(record)) =
            crate::tc235_business_meaning_runtime::lifecycle_selection(
                &context_package,
                &context_packages,
                "Target",
                &snapshots,
                &BTreeMap::new(),
                input,
            )
        else {
            panic!("external Clock context/parameter must refuse")
        };
        if mode == "ordinary" {
            assert_eq!(
                record.catalog,
                filament_semantic_value::catalog::CatalogCode::new(
                    "dangling_reference",
                    "absent-target-in-complete-population"
                )
            );
            assert_eq!(record.fields.get("field").map(String::as_str), Some("peer"));
            assert_eq!(
                record.fields.get("object").map(String::as_str),
                Some("ghost")
            );
        } else {
            assert_unbound(&record);
        }
    }
    let (environment, _) = bare
        .admit_call_snapshot(
            &checked.packages,
            ModelNormalizationLimits::default(),
            &snapshots,
            &identity,
            ObservationLimits::default(),
        )
        .expect("Bare authenticates the actual Model-owned Clock reference");
    assert_eq!(
        environment
            .population_members(&crate::ring_queue::node(CLOCKS, "calls"))
            .expect("calls")
            .len(),
        1
    );
    let ObservationAdmission::Admitted(admitted) = bare.admit_protocol_observation(
        &model,
        &checked.packages,
        ModelNormalizationLimits::default(),
        &snapshots,
        &identity,
        ObservationLimits::default(),
    ) else {
        panic!("Bare protocol snapshot must authenticate Model-owned Clock data")
    };
    assert_eq!(
        admitted
            .environment()
            .population_members(&crate::ring_queue::node(CLOCKS, "calls"))
            .expect("calls")
            .len(),
        1
    );
    let ObservationAdmission::Admitted(admitted) = bare.admit_protocol_state(
        &model,
        &checked.packages,
        ModelNormalizationLimits::default(),
        &admitted_state,
        identity,
    ) else {
        panic!("Bare observation of actually admitted state preserves Clock data")
    };
    assert_eq!(
        admitted
            .environment()
            .population_members(&crate::ring_queue::node(CLOCKS, "calls"))
            .expect("calls")
            .len(),
        1
    );

    let mut ordinary_calls: Json =
        serde_json::from_slice(&crate::ring_queue::clock_calls()).expect("ordinary model");
    ordinary_calls["types"]
        .as_array_mut()
        .expect("types")
        .iter_mut()
        .find(|ty| ty["identity"] == crate::ring_queue::node(CLOCKS, "Call"))
        .expect("Call")["fields"]
        .as_array_mut()
        .expect("fields")
        .retain(|field| field["name"] != "x");
    let ordinary_calls = serde_json::to_vec(&ordinary_calls).expect("ordinary model");
    let ordinary = crate::ring_queue::check_models(
        &crate::ring_queue::queue_spec(),
        ("C", CLOCKS, &ordinary_calls),
        true,
        &clock_refinement("used = count(c in C::calls where c.n = self.n); busy = self.n = 2;"),
    )
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
    match ordinary_bare.admit_protocol_state(
        &model,
        &ordinary.packages,
        ModelNormalizationLimits::default(),
        &state(),
        DocumentRef {
            authority: "test".to_owned(),
            identity: "ordinary".to_owned(),
        },
    ) {
        ObservationAdmission::Refused(record) => assert_eq!(
            record,
            filament_semantic_value::admission::AdmissionRecord {
                catalog: filament_semantic_value::catalog::CatalogCode::new(
                    "unknown_required_feature",
                    "unsupported-feature"
                ),
                fields: BTreeMap::from([
                    (
                        "package",
                        ordinary_package.graph().package_name().to_owned()
                    ),
                    ("reason", "missing-type-representation".to_owned())
                ]),
                path: Vec::new(),
                kernel: None,
            },
            "ordinary missing representation keeps its existing refusal"
        ),
        ObservationAdmission::Admitted(_) | ObservationAdmission::Incomplete(_) => {
            panic!("ordinary unreached type projection must preserve its refusal")
        }
        ObservationAdmission::Fault(fault) => panic!("ordinary Bare fault: {fault:?}"),
    }
}

/// A map layout never authorizes a numerically forged clock-place identity.
#[trace("TC-3084", "FR-230-AC-62")]
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
    let map_clock = graph
        .timed_models()
        .iter()
        .flat_map(|model| model.clocks().iter())
        .find(|clock| clock.slots().any(|shape| shape.map))
        .expect("actual map declaration");
    assert_eq!(map_clock.field(), &key(CLOCKS, "Call/mo"));
    let valid = generated_place(&package, &checked.packages, "c1");
    let declared = DeclaredClockPlaces::with_declaration_table(
        graph.timed_models(),
        graph.clock_declaration_table(),
    );
    assert!(
        declared.contains(&valid, &owner, graph.scope().types()),
        "actual table validates the evaluated declared x place"
    );
    use serde_json::json;
    let scope = json!(["model", CLOCKS, crate::ring_queue::node(CLOCKS, "Call")]);
    let text = place_text(
        "c1",
        json!([
            [
                "field",
                scope,
                crate::ring_queue::node(CLOCKS, "Call/mo"),
                ["option", ["collection", "sequence", "0", "2", ["clock"]]]
            ],
            ["option"],
            ["collection", "sequence", "0", "2", ["clock"]]
        ]),
        json!([["slot", "2", "0"]]),
    );
    let identity =
        clock_reference::identity::PlaceIdentity::decode(&text, quire_canonical::Limits::new(4096))
            .expect("well-formed v2 numeric forgery, not a legacy codec ID");
    let forged =
        clock_reference::encode_identity(&owner, &identity, quire_canonical::Limits::new(4096))
            .expect("hostile v2 value in the actual owner's universe");
    assert!(
        !declared.contains(&forged, &owner, graph.scope().types()),
        "a map template never authenticates a numeric place ID"
    );
    let (identity, bytes) = clock_snapshot(&valid);
    let mut snapshot: serde_json::Value = serde_json::from_slice(&bytes).expect("snapshot");
    snapshot["populations"][0]["objects"][0]["fields"]["mo"] = json!({"absent":{}});
    snapshot["populations"][0]["objects"][0]["fields"]["r"] = json!({"reference":{"population":crate::ring_queue::node(CLOCKS,"calls"),"key":forged.object().as_str()}});
    let Err(AdmissionFailure::Refused(record)) = model.admit_initial(
        &BTreeMap::from([(
            identity.clone(),
            serde_json::to_vec(&snapshot).expect("snapshot"),
        )]),
        &identity,
        ObservationLimits::default(),
    ) else {
        panic!("raw map-place value must refuse")
    };
    use filament_semantic_value::admission::{AdmissionRecord, PathStep};
    assert_eq!(
        *record,
        AdmissionRecord {
            catalog: filament_semantic_value::catalog::CatalogCode::new(
                "dangling_reference",
                "absent-target-in-complete-population"
            ),
            fields: BTreeMap::from([
                ("document", identity.identity.clone()),
                ("population", crate::ring_queue::node(CLOCKS, "calls")),
                ("object", "c1".to_owned()),
                ("field", "r".to_owned()),
                ("target", forged.object().as_str().to_owned()),
                ("path", "/populations/0/objects/0/fields/r".to_owned()),
            ]),
            path: vec![
                PathStep::Field("populations".to_owned()),
                PathStep::Index(0),
                PathStep::Field("objects".to_owned()),
                PathStep::Index(0),
                PathStep::Field("fields".to_owned()),
                PathStep::Field("r".to_owned())
            ],
            kernel: None,
        }
    );
}
