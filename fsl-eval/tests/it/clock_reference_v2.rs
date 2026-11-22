// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL-2178 retains the end-to-end acceptance oracles as explicit debt.
//! Executed controls assert the supported foundation and typed RAW refusals.

use std::collections::BTreeMap;

use filament_eval::{Evaluation, FamilyOutcome};
use filament_exact::{Integer, Meter, ObjectReference, Outcome, ScalarLimits, Value};
use filament_semantic_value::admission::{AdmissionRecord, ObservationAdmission, PathStep};
use filament_semantic_value::catalog::CatalogCode;
use fsl_eval::value::{
    CheckedPackageEvaluation, ProtocolReferenceOccurrence, ProtocolSlot, ProtocolSlotEvaluation,
    ProtocolSuppliedValue, ProtocolSupply, QualifiedName, SuppliedValue,
};
use fsl_package::{BarePackage, BareSupply, CheckedPackage, EmitLimits, V2ReadLimits};
use fsl_semantics::model::accounting::ModelNormalizationLimits;
use fsl_semantics::model::object_environment::ObjectEnvironment;
use fsl_semantics::model::observation::{
    admit_observations, AdmissionFailure, AnchorKind, ClauseFacts, ClauseSelection,
    ClauseSelectionInput, DocumentRef, ObservationLimits, Provisions, SelectedAnchor,
    SelectedObject,
};
use fsl_semantics::model::state::StateModel;
use ix_trace_rs::trace;
use serde_json::{json, Value as Json};

use crate::ring_queue::{self, clock_refinement, key, node, Checked, CLOCKS, QUEUE};

const LIMITS: ScalarLimits = ScalarLimits {
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

fn fixture(remove_x: bool) -> Checked {
    let add_reference = |bytes: Vec<u8>, domain: &str, owner: &str, name: &str| {
        let mut document: Json = serde_json::from_slice(&bytes).expect("model JSON");
        let mut reference = ring_queue::declaration(
            domain,
            "ClockRef",
            "unused",
            json!({"target":"ix://filament/native/Clock"}),
        );
        reference["kind"] = json!("reference");
        document["types"]
            .as_array_mut()
            .expect("types")
            .push(reference);
        let owner = document["types"]
            .as_array_mut()
            .expect("types")
            .iter_mut()
            .find(|ty| ty["identity"] == node(domain, owner))
            .expect("owner");
        owner["fields"]
            .as_array_mut()
            .expect("fields")
            .push(ring_queue::single(
                domain,
                if domain == CLOCKS { "Call" } else { "Queue" },
                name,
                &node(domain, "ClockRef"),
            ));
        document
    };
    let queue = add_reference(ring_queue::queue_spec(), QUEUE, "Queue", "chosen");
    let mut calls = add_reference(ring_queue::clock_calls(), CLOCKS, "Call", "r");
    let call = calls["types"]
        .as_array_mut()
        .expect("types")
        .iter_mut()
        .find(|ty| ty["identity"] == node(CLOCKS, "Call"))
        .expect("Call");
    let fields = call["fields"].as_array_mut().expect("fields");
    let mut y = ring_queue::single(CLOCKS, "Call", "y", "ix://filament/native/Clock");
    y["multiplicity"]["unique"] = json!(false);
    fields.push(y);
    let mut maybe = ring_queue::single(CLOCKS, "Call", "maybe", &node(CLOCKS, "ClockRef"));
    maybe["presence"] = json!("optional");
    fields.push(maybe);
    fields.push(ring_queue::field(
        CLOCKS,
        "Call",
        "refs",
        &node(CLOCKS, "ClockRef"),
        (0, 3, true),
    ));
    if remove_x {
        fields.retain(|field| field["name"] != "x");
    }
    let selected = if remove_x { "y" } else { "x" };
    // Definedness::range_path documents that presence uses stable_path,
    // which takes a "model read only in a state clause (FR-104)".
    // Its local-root rule is `NodeKind::Local(slot) => break *slot`.
    // Bind the optional read once so present/value share that stable local
    // rather than trying to prove presence of a function's model Attribute.
    let body = format!(
        "{}\n\
         protocol Places using p over (call: C::Call) on each (opened: Boolean) when (opened) {{\n\
           capture chosen: Reference<Clock> = ref(call.{selected});\n\
           capture used: Integer = count(c in C::calls where ref(c.{selected}) = call.r);\n\
           capture busy: Boolean = only(c in C::calls where c.n = 2: ref(c.{selected})) = call.r;\n\
           capture retained_y: Reference<Clock> = ref(call.y);\n\
           capture echoed: Reference<Clock> = chosen;\n\
           role Observer on C::Call;\n\
           run sequence Main {{ check Ready using v {{ true }}; }}\n\
           finish End as (outcome: Boolean) {{ true }};\n\
         }}\n\
         invariant Stored using m on C::Call at current {{ self.r = ref(self.{selected}) }}\n\
         function stored using m(c: Reference<C::Call>): Reference<Clock> pure {{ deref(c).r }}\n\
         function maybe_stored using m(c: Reference<C::Call>): Reference<Clock> pure {{ let maybe = deref(c).maybe in if present(maybe) then value(maybe) else deref(c).r }}\n\
         function has_maybe using m(c: Reference<C::Call>): Boolean pure {{ present(deref(c).maybe) }}\n\
         function stored_refs using m(c: Reference<C::Call>): Sequence<Reference<Clock>>[0, 3] pure {{ deref(c).refs }}",
        clock_refinement(&format!(
            "used = count(c in C::calls where ref(c.{selected}) = self.r);\n\
             busy = only(c in C::calls where c.n = 2: ref(c.{selected})) = self.r;\n\
             chosen = ref(self.{selected});"
        ))
    );
    ring_queue::check_models_with_profiles(
        &serde_json::to_vec(&queue).expect("queue"),
        ("C", CLOCKS, &serde_json::to_vec(&calls).expect("calls")),
        true,
        "profile p = \"filament.protocol.finite-global/v1\";\n",
        &body,
    )
    .unwrap_or_else(|refusals| {
        panic!(
            "clock fixture checks: {:?}",
            refusals
                .iter()
                .map(|r| (&r.refusal, r.at.as_str()))
                .collect::<Vec<_>>()
        )
    })
}

/// An independent FR-230 grammar oracle. All five components contain only
/// arrays and strings, so serde's compact JSON is already RFC 8785 canonical.
fn expected_place(owner: &str, field: &str) -> String {
    let scope = json!(["model", CLOCKS, node(CLOCKS, "Call")]);
    let components = [
        json!(["model", CLOCKS, node(CLOCKS, "calls")]),
        json!(owner),
        scope.clone(),
        json!([[
            "field",
            scope,
            node(CLOCKS, &format!("Call/{field}")),
            ["clock"]
        ]]),
        json!([]),
    ];
    let mut text = "place/v2".to_owned();
    for component in components {
        text.push('/');
        for byte in serde_json::to_vec(&component).expect("canonical component") {
            use std::fmt::Write;
            write!(&mut text, "{byte:02x}").expect("write to string");
        }
    }
    text
}

fn identity() -> DocumentRef {
    DocumentRef {
        authority: "test".to_owned(),
        identity: "clock-v2".to_owned(),
    }
}

fn raw_reference(reference: &str) -> Json {
    json!({"reference":{"population":node(CLOCKS,"calls"),"key":reference}})
}

fn snapshot(first: &str, second: &str, present: bool) -> BTreeMap<DocumentRef, Vec<u8>> {
    let objects: Vec<_> = [("c1", "1", first), ("c2", "2", second)].into_iter().map(|(owner, n, reference)| json!({
        "key":owner,"type":node(CLOCKS,"Call"),"fields":{
            "n":{"integer":n},"d":{"integer":"0"},"r":raw_reference(reference),
            "maybe":if present {json!({"present":raw_reference(second)})} else {json!({"absent":{}})},
            "refs":{"sequence":[raw_reference(second),raw_reference(first)]}
        }
    })).collect();
    let document = json!({
        "format":"filament.state.snapshot/v1",
        "identity":{"authority":"test","identity":"clock-v2"},
        "observation":"current","model":{"identity":CLOCKS},
        "anchor":{"kind":"initialization","name":"start"},
        "populations":[{"population":node(CLOCKS,"calls"),"complete":true,"objects":objects}]
    });
    BTreeMap::from([(identity(), serde_json::to_vec(&document).expect("snapshot"))])
}

fn completed(evaluation: Evaluation) -> Value {
    match evaluation.outcome {
        FamilyOutcome::Evaluated(Outcome::Completed(value)) => value,
        other => panic!("evaluation completes: {other:?}"),
    }
}

fn owner(environment: &ObjectEnvironment, name: &str) -> ObjectReference {
    environment
        .population_members(&node(CLOCKS, "calls"))
        .expect("calls")
        .iter()
        .find(|owner| owner.object().as_str() == name)
        .expect("admitted owner")
        .clone()
}

fn capture(
    package: &CheckedPackage,
    packages: &fsl_semantics::model::intake::PackageInput,
    snapshots: &BTreeMap<DocumentRef, Vec<u8>>,
    index: usize,
    name: &str,
) -> Value {
    let view = package.evaluation_view();
    let observation = view.admit_protocol_observation(
        &fsl_semantics::model::domain_package::DomainPackageRef {
            identity: CLOCKS.to_owned(),
        },
        packages,
        ModelNormalizationLimits::default(),
        snapshots,
        &identity(),
        ObservationLimits::default(),
    );
    let ObservationAdmission::Admitted(admitted) = &observation else {
        panic!("actual protocol observation admits: {observation:?}")
    };
    let environment = admitted.environment();
    let keys = package
        .graph()
        .protocol_node_keys("Places")
        .expect("protocol keys");
    let binder = keys
        .binders
        .iter()
        .find(|binder| binder.name == "call")
        .expect("owner binder");
    let supply = ProtocolSupply {
        binders: vec![(
            binder.node.clone(),
            ProtocolSuppliedValue {
                value: SuppliedValue::Value(Value::Reference(owner(environment, name))),
                references: vec![ProtocolReferenceOccurrence {
                    path: Vec::new(),
                    population: environment
                        .population_node(&node(CLOCKS, "calls"))
                        .expect("authenticated population")
                        .clone(),
                }],
            },
        )],
        observation: Some(&observation),
    };
    let slot = ProtocolSlot {
        node: keys.clause.clone().expect("protocol root"),
        member: "captures".to_owned(),
        index: Some(index),
    };
    let ProtocolSlotEvaluation::Evaluated(evaluation, _) = package
        .evaluate_protocol_slot(&slot, &supply, &mut Meter::new(LIMITS))
        .expect("actual seeded protocol capture")
    else {
        panic!("protocol capture must be available")
    };
    completed(evaluation)
}

fn assert_value(actual: &Value, expected: &Value) {
    assert_eq!(
        filament_exact::compare_keys(actual, expected),
        Some(std::cmp::Ordering::Equal),
        "actual {actual:?}; expected {expected:?}"
    );
}

fn assert_producer_clause(
    package: &CheckedPackage,
    evaluator: &impl CheckedPackageEvaluation,
    types: &filament_semantic_value::declaration::TypeEnvironment,
    packages: &fsl_semantics::model::intake::PackageInput,
    snapshots: &BTreeMap<DocumentRef, Vec<u8>>,
    first_matches: bool,
) {
    let clause = package
        .graph()
        .state_clause("Stored")
        .expect("checked invariant");
    let facts = ClauseFacts {
        identity: clause.identity(),
        kind: clause.kind(),
        context: clause.context(),
        operation: None,
        operations: package.graph().scope().operations().clone(),
        named_populations: clause.named_reads(),
    };
    for name in ["c1", "c2"] {
        let observations = admit_observations(
            package.graph().model_inputs(),
            types,
            &facts,
            packages,
            ModelNormalizationLimits::default(),
            &Provisions {
                snapshots,
                invocations: &BTreeMap::new(),
            },
            &ClauseSelection {
                name: "Stored".to_owned(),
                input: ClauseSelectionInput::Current {
                    snapshot: identity(),
                    anchor: SelectedAnchor {
                        kind: AnchorKind::Initialization,
                        name: "start".to_owned(),
                    },
                    self_object: Some(SelectedObject {
                        population: node(CLOCKS, "calls"),
                        key: name.to_owned(),
                    }),
                },
            },
            ObservationLimits::default(),
        )
        .expect("raw clause admission retains explicit owner population");
        assert!(
            observations.self_population.is_some(),
            "producer has authenticated P"
        );
        let evaluation = evaluator
            .evaluate_clause(
                &QualifiedName::unqualified("Stored").expect("name"),
                &observations,
                &mut Meter::new(LIMITS),
            )
            .expect("actual route evaluates its Clock producer");
        assert_value(
            &completed(evaluation.evaluation),
            &Value::Boolean(name != "c1" || first_matches),
        );
    }
}

fn assert_supplied_clock_data(
    evaluator: &impl CheckedPackageEvaluation,
    observation: &ObservationAdmission<fsl_package::AdmittedProtocolObservation<'_>>,
    slot: &ProtocolSlot,
    binder: &filament_exact::NodeKey,
    reference: &ObjectReference,
    complete: bool,
) {
    use filament_eval::{CallAdmissionFailure, CallAdmissionLocus, CallFailure};
    let ObservationAdmission::Admitted(admitted) = observation else {
        panic!("the current observation admits before binder completeness: {observation:?}")
    };
    let population = admitted
        .environment()
        .population_node(&node(CLOCKS, "calls"))
        .expect("actual supplied owner population")
        .clone();
    let supply = |reference: ObjectReference| ProtocolSupply {
        binders: vec![(
            binder.clone(),
            ProtocolSuppliedValue {
                value: SuppliedValue::Value(Value::Reference(reference)),
                references: vec![ProtocolReferenceOccurrence {
                    path: Vec::new(),
                    population: population.clone(),
                }],
            },
        )],
        observation: Some(observation),
    };
    let forged = ObjectReference::new(
        reference.universe().clone(),
        reference.object_type().clone(),
        filament_exact::ObjectId::new(expected_place("c1", "undeclared")).expect("negative key"),
    );
    if complete {
        let ProtocolSlotEvaluation::Evaluated(evaluation, _) = evaluator
            .evaluate_protocol_slot(slot, &supply(reference.clone()), &mut Meter::new(LIMITS))
            .expect("Clock DATA supply")
        else {
            panic!("supplied Clock DATA is available");
        };
        assert_value(&completed(evaluation), &Value::Reference(reference.clone()));
    }
    let failure = evaluator
        .evaluate_protocol_slot(slot, &supply(forged), &mut Meter::new(LIMITS))
        .expect_err("an undeclared Clock target is never accepted");
    let CallFailure::Admission(failure) = failure else {
        panic!("typed admission refusal");
    };
    let record = match (*failure, complete) {
        (CallAdmissionFailure::Refused(record), true) => {
            assert_eq!(
                record.record.catalog,
                CatalogCode::new("dangling_reference", "absent-target-in-complete-population")
            );
            record
        }
        (CallAdmissionFailure::Incomplete(record), false) => {
            assert_eq!(
                record.record.catalog,
                CatalogCode::new("incomplete_population", "incomplete-scope")
            );
            record
        }
        (other, _) => {
            panic!("owner population completeness precedes Clock target presence: {other:?}")
        }
    };
    assert!(record.record.path.is_empty());
    if complete {
        assert_eq!(record.locus, CallAdmissionLocus::Binder(binder.clone()));
    } else {
        assert_eq!(record.locus, CallAdmissionLocus::Population(population));
    }
}

/// Clock DATA retains explicit P without being tested as a Call object;
/// complete-population admission still precedes declared-place membership.
#[trace("FR-390-AC-23", "TC-464")]
#[test]
#[ignore = "QSL-2897: end-to-end raw-v2 Clock reference support remains unsupported (Source missing-route-neutral-place-identity; Bare missing-type-representation)"]
fn supplied_clock_data_validates_owner_population_on_source_and_bare() {
    let checked = fixture(false);
    let package = CheckedPackage::link(checked.graph);
    let snapshots = snapshot(&expected_place("c1", "x"), &expected_place("c2", "x"), true);
    let Value::Reference(reference) = capture(&package, &checked.packages, &snapshots, 0, "c1")
    else {
        panic!("actual seeded producer");
    };
    let read = fsl_package::read_evaluation_package(
        &package,
        fsl_semantics::library::LibraryName::new(package.graph().package_name()).expect("name"),
        &checked.packages,
        V2ReadLimits::fixture(),
        EmitLimits::fixture(),
    )
    .expect("actual package read");
    let rows = read
        .emission()
        .identity_rows(
            read.admitted(),
            &mut V2ReadLimits::fixture().native_join_budget(),
        )
        .expect("rows");
    let equality = read
        .emission()
        .equality_witness(
            read.admitted(),
            &mut V2ReadLimits::fixture().native_join_budget(),
        )
        .expect("equality");
    let bare = BarePackage::new(read.admitted())
        .with_supply(
            BareSupply {
                identities: &rows,
                equality: Some(&equality),
            },
            &mut V2ReadLimits::fixture().native_join_budget(),
        )
        .expect("Bare");
    let keys = package.graph().protocol_node_keys("Places").expect("keys");
    let binder = &keys
        .binders
        .iter()
        .find(|binder| binder.name == "chosen")
        .expect("Clock DATA binder")
        .node;
    let slot = ProtocolSlot {
        node: keys.clause.clone().expect("protocol"),
        member: "captures".to_owned(),
        index: Some(4),
    };
    let model = fsl_semantics::model::domain_package::DomainPackageRef {
        identity: CLOCKS.to_owned(),
    };
    for complete in [true, false] {
        let mut document: Json =
            serde_json::from_slice(snapshots.get(&identity()).expect("snapshot")).expect("JSON");
        document["populations"][0]["complete"] = json!(complete);
        let snapshots =
            BTreeMap::from([(identity(), serde_json::to_vec(&document).expect("snapshot"))]);
        let view = package.evaluation_view();
        let source_observation = view.admit_protocol_observation(
            &model,
            &checked.packages,
            ModelNormalizationLimits::default(),
            &snapshots,
            &identity(),
            ObservationLimits::default(),
        );
        assert_supplied_clock_data(
            &package,
            &source_observation,
            &slot,
            binder,
            &reference,
            complete,
        );
        let bare_observation = bare.admit_protocol_observation(
            &model,
            &checked.packages,
            ModelNormalizationLimits::default(),
            &snapshots,
            &identity(),
            ObservationLimits::default(),
        );
        assert_supplied_clock_data(
            &bare,
            &bare_observation,
            &slot,
            binder,
            &reference,
            complete,
        );
    }
}

#[trace("TC-464", "TC-3084", "FR-106-AC-72", "FR-106-AC-76")]
#[test]
#[ignore = "QSL-2897: end-to-end raw-v2 Clock reference support remains unsupported (Source missing-route-neutral-place-identity; Bare missing-type-representation)"]
fn actual_clock_references_roundtrip_as_data_through_source_and_bare() {
    let checked = fixture(false);
    let package = CheckedPackage::link(checked.graph);
    let model = StateModel::new(
        package.graph().model_inputs(),
        package.graph().scope().types(),
        &checked.packages,
        ModelNormalizationLimits::default(),
    )
    .expect("model");
    let seed = snapshot(&expected_place("c1", "x"), &expected_place("c2", "x"), true);
    let (state, _) = model
        .admit_initial(&seed, &identity(), ObservationLimits::default())
        .expect("declared v2 seed snapshot");
    model
        .observation(&state, identity())
        .expect("admitted state observes");
    let places: Vec<_> = ["c1", "c2"]
        .into_iter()
        .map(|name| {
            let Value::Reference(reference) = capture(&package, &checked.packages, &seed, 0, name)
            else {
                panic!("Clock producer returns Reference")
            };
            assert_eq!(reference.object().as_str(), expected_place(name, "x"));
            assert_ne!(
                reference.object().as_str(),
                name,
                "place is not owner object key"
            );
            reference
        })
        .collect();
    assert_ne!(places[0], places[1], "actual owners distinguish places");
    let read = fsl_package::read_evaluation_package(
        &package,
        fsl_semantics::library::LibraryName::new(package.graph().package_name()).expect("name"),
        &checked.packages,
        V2ReadLimits::fixture(),
        EmitLimits::fixture(),
    )
    .expect("Source emits and strict reader admits");
    let rows = read
        .emission()
        .identity_rows(
            read.admitted(),
            &mut V2ReadLimits::fixture().native_join_budget(),
        )
        .expect("identity rows");
    let equality = read
        .emission()
        .equality_witness(
            read.admitted(),
            &mut V2ReadLimits::fixture().native_join_budget(),
        )
        .expect("equality rows");
    let bare = BarePackage::new(read.admitted())
        .with_supply(
            BareSupply {
                identities: &rows,
                equality: Some(&equality),
            },
            &mut V2ReadLimits::fixture().native_join_budget(),
        )
        .expect("Bare supply");
    let world = bare.world().expect("Bare world");
    for present in [true, false] {
        let mut snapshots = snapshot(
            places[0].object().as_str(),
            places[1].object().as_str(),
            present,
        );
        if !present {
            let mut document: Json =
                serde_json::from_slice(&snapshots[&identity()]).expect("snapshot");
            document["populations"][0]["objects"][0]["fields"]["r"] =
                raw_reference(places[1].object().as_str());
            snapshots.insert(identity(), serde_json::to_vec(&document).expect("snapshot"));
        }
        let (state, _) = model
            .admit_initial(&snapshots, &identity(), ObservationLimits::default())
            .expect("actual references read back");
        let source = model
            .observation(&state, identity())
            .expect("readback observation");
        let (bytes, _) = bare
            .admit_call_snapshot(
                &checked.packages,
                ModelNormalizationLimits::default(),
                &snapshots,
                &identity(),
                ObservationLimits::default(),
            )
            .expect("actual Bare raw admission");
        assert_producer_clause(
            &package,
            &package,
            package.graph().scope().types(),
            &checked.packages,
            &snapshots,
            present,
        );
        assert_producer_clause(
            &package,
            &bare,
            world.types(),
            &checked.packages,
            &snapshots,
            present,
        );
        for (index, name) in ["c1", "c2"].into_iter().enumerate() {
            assert_value(
                &capture(&package, &checked.packages, &snapshots, 1, name),
                &Value::Integer(Integer::from(1_i64)),
            );
            assert_value(
                &capture(&package, &checked.packages, &snapshots, 2, name),
                &Value::Boolean(index == 1 || !present),
            );
            for function in ["stored", "has_maybe", "stored_refs"]
                .into_iter()
                .chain(present.then_some("maybe_stored"))
            {
                let function_name = function;
                let function = QualifiedName::unqualified(function_name).expect("function");
                let source_value = completed(
                    package
                        .call(
                            &function,
                            vec![Value::Reference(owner(&source.environment, name))],
                            &source.environment,
                            &mut Meter::new(LIMITS),
                        )
                        .expect("source stored-data call"),
                );
                let bare_value = completed(
                    bare.call(
                        &function,
                        vec![Value::Reference(owner(&bytes, name))],
                        &bytes,
                        &mut Meter::new(LIMITS),
                    )
                    .expect("actual Bare stored-data call"),
                );
                assert_value(&bare_value, &source_value);
                match function_name {
                    "stored" => assert_value(
                        &bare_value,
                        &Value::Reference(
                            places[if !present && index == 0 { 1 } else { index }].clone(),
                        ),
                    ),
                    "maybe_stored" => {
                        assert_value(&bare_value, &Value::Reference(places[1].clone()))
                    }
                    "has_maybe" => assert_value(&bare_value, &Value::Boolean(present)),
                    "stored_refs" => {
                        let Value::Collection(refs) = bare_value else {
                            panic!("stored sequence")
                        };
                        assert_eq!(refs.elements().len(), 2);
                        assert_value(&refs.elements()[0], &Value::Reference(places[1].clone()));
                        assert_value(&refs.elements()[1], &Value::Reference(places[0].clone()));
                    }
                    _ => unreachable!("fixture function"),
                }
            }
        }
    }
}

#[trace("TC-464", "FR-106-AC-73", "FR-106-AC-76")]
#[test]
#[ignore = "QSL-2897: end-to-end raw-v2 Clock reference support remains unsupported (Source missing-route-neutral-place-identity; Bare missing-type-representation)"]
fn removing_x_never_rebinds_its_actual_reference_to_y_at_the_reused_ordinal() {
    let checked = fixture(false);
    let package = CheckedPackage::link(checked.graph);
    let model = StateModel::new(
        package.graph().model_inputs(),
        package.graph().scope().types(),
        &checked.packages,
        ModelNormalizationLimits::default(),
    )
    .expect("original model");
    let seed = snapshot(&expected_place("c1", "x"), &expected_place("c2", "x"), true);
    let (state, _) = model
        .admit_initial(&seed, &identity(), ObservationLimits::default())
        .expect("original declared snapshot");
    model
        .observation(&state, identity())
        .expect("original state");
    let Value::Reference(old_x) = capture(&package, &checked.packages, &seed, 0, "c1") else {
        panic!("actual old x reference")
    };
    let Value::Reference(old_y) = capture(&package, &checked.packages, &seed, 3, "c1") else {
        panic!("actual retained y reference before removing x")
    };
    let old_ordinal = package
        .graph()
        .timed_model("C")
        .expect("timed")
        .clock_fields()
        .find(|(_, field)| field.field() == &key(CLOCKS, "Call/x"))
        .expect("x")
        .0;
    let changed = fixture(true);
    let next = CheckedPackage::link(changed.graph);
    let new_ordinal = next
        .graph()
        .timed_model("C")
        .expect("timed")
        .clock_fields()
        .find(|(_, field)| field.field() == &key(CLOCKS, "Call/y"))
        .expect("y retained")
        .0;
    assert_eq!(
        old_ordinal, new_ordinal,
        "y genuinely reuses x's internal clock ordinal"
    );
    let model = StateModel::new(
        next.graph().model_inputs(),
        next.graph().scope().types(),
        &changed.packages,
        ModelNormalizationLimits::default(),
    )
    .expect("changed model");
    let read = fsl_package::read_evaluation_package(
        &next,
        fsl_semantics::library::LibraryName::new(next.graph().package_name()).expect("name"),
        &changed.packages,
        V2ReadLimits::fixture(),
        EmitLimits::fixture(),
    )
    .expect("changed emitted package reads");
    let rows = read
        .emission()
        .identity_rows(
            read.admitted(),
            &mut V2ReadLimits::fixture().native_join_budget(),
        )
        .expect("rows");
    let equality = read
        .emission()
        .equality_witness(
            read.admitted(),
            &mut V2ReadLimits::fixture().native_join_budget(),
        )
        .expect("equality");
    let bare = BarePackage::new(read.admitted())
        .with_supply(
            BareSupply {
                identities: &rows,
                equality: Some(&equality),
            },
            &mut V2ReadLimits::fixture().native_join_budget(),
        )
        .expect("Bare supply");
    let valid = snapshot(&expected_place("c1", "y"), &expected_place("c2", "y"), true);
    let (state, _) = model
        .admit_initial(&valid, &identity(), ObservationLimits::default())
        .expect("retained y admits");
    model
        .observation(&state, identity())
        .expect("changed state");
    let Value::Reference(y) = capture(&next, &changed.packages, &valid, 0, "c1") else {
        panic!("actual y")
    };
    assert_ne!(
        old_x, y,
        "full actual references cannot alias at a reused ordinal"
    );
    assert_eq!(
        old_y, y,
        "the surviving original declaration retains its full identity"
    );
    bare.admit_call_snapshot(
        &changed.packages,
        ModelNormalizationLimits::default(),
        &valid,
        &identity(),
        ObservationLimits::default(),
    )
    .expect("Bare admits retained y positive control");
    for field in ["r", "maybe", "refs"] {
        let mut document: Json = serde_json::from_slice(&valid[&identity()]).expect("snapshot");
        let old = raw_reference(old_x.object().as_str());
        document["populations"][0]["objects"][0]["fields"][field] = match field {
            "r" => old,
            "maybe" => json!({"present":old}),
            "refs" => json!({"sequence":[old, raw_reference(y.object().as_str())]}),
            _ => unreachable!("fixture field"),
        };
        let snapshots =
            BTreeMap::from([(identity(), serde_json::to_vec(&document).expect("snapshot"))]);
        let source = model
            .admit_initial(&snapshots, &identity(), ObservationLimits::default())
            .expect_err("old x must be undeclared");
        let bytes = bare
            .admit_call_snapshot(
                &changed.packages,
                ModelNormalizationLimits::default(),
                &snapshots,
                &identity(),
                ObservationLimits::default(),
            )
            .expect_err("Bare must reject old x, never resolve y");
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
            catalog: CatalogCode::new("dangling_reference", "absent-target-in-complete-population"),
            fields: BTreeMap::from([
                ("document", identity().identity),
                ("population", node(CLOCKS, "calls")),
                ("object", "c1".to_owned()),
                ("field", field.to_owned()),
                ("target", old_x.object().as_str().to_owned()),
                (
                    "path",
                    format!("/populations/0/objects/0/fields/{field}{suffix}"),
                ),
            ]),
            path,
            kernel: None,
        };
        for refusal in [source, bytes] {
            let AdmissionFailure::Refused(record) = refusal else {
                panic!("typed stale-reference refusal")
            };
            assert_eq!(
                *record, expected,
                "exact stale-reference owner, target and nested path"
            );
        }
    }
}

/// Authenticated scope disagreement is malformed input, independently of the
/// dangling outcome for an old field removed from the correct declaration.
#[trace("TC-464", "FR-230-AC-45")]
#[test]
#[ignore = "QSL-2897: end-to-end raw-v2 Clock reference support remains unsupported (Source missing-route-neutral-place-identity; Bare missing-type-representation)"]
fn clock_reference_scope_disagreement_is_invalid_input_on_source_and_bare() {
    use filament_eval::{CallAdmissionFailure, CallAdmissionLocus, CallFailure};
    use fsl_semantics::model::clock_reference::identity::{DeclarationScope, PlaceIdentity};

    let checked = fixture(false);
    let package = CheckedPackage::link(checked.graph);
    let model = StateModel::new(
        package.graph().model_inputs(),
        package.graph().scope().types(),
        &checked.packages,
        ModelNormalizationLimits::default(),
    )
    .expect("actual model declarations");
    let valid = snapshot(&expected_place("c1", "x"), &expected_place("c2", "x"), true);
    let Value::Reference(reference) = capture(&package, &checked.packages, &valid, 0, "c1") else {
        panic!("actual seeded Clock producer");
    };
    let read = fsl_package::read_evaluation_package(
        &package,
        fsl_semantics::library::LibraryName::new(package.graph().package_name()).expect("name"),
        &checked.packages,
        V2ReadLimits::fixture(),
        EmitLimits::fixture(),
    )
    .expect("actual checked package emits and reads");
    let rows = read
        .emission()
        .identity_rows(
            read.admitted(),
            &mut V2ReadLimits::fixture().native_join_budget(),
        )
        .expect("identity rows");
    let equality = read
        .emission()
        .equality_witness(
            read.admitted(),
            &mut V2ReadLimits::fixture().native_join_budget(),
        )
        .expect("equality rows");
    let bare = BarePackage::new(read.admitted())
        .with_supply(
            BareSupply {
                identities: &rows,
                equality: Some(&equality),
            },
            &mut V2ReadLimits::fixture().native_join_budget(),
        )
        .expect("actual Bare supply");
    model
        .admit_initial(&valid, &identity(), ObservationLimits::default())
        .expect("unchanged reference admits on Source");
    bare.admit_call_snapshot(
        &checked.packages,
        ModelNormalizationLimits::default(),
        &valid,
        &identity(),
        ObservationLimits::default(),
    )
    .expect("unchanged reference admits on Bare");

    for changed_component in ["population", "scope"] {
        let mut changed = PlaceIdentity::decode(
            reference.object().as_str(),
            quire_canonical::Limits::new(4096),
        )
        .expect("actual producer's canonical identity");
        match changed_component {
            "population" => {
                changed.population = DeclarationScope::Model {
                    identity: QUEUE.to_owned(),
                    declaration: node(QUEUE, "queues"),
                }
            }
            "scope" => {
                changed.template.scope = DeclarationScope::Model {
                    identity: QUEUE.to_owned(),
                    declaration: node(QUEUE, "Queue"),
                }
            }
            _ => unreachable!("two scope components"),
        }
        let changed = changed
            .encode(quire_canonical::Limits::new(4096))
            .expect("canonical hostile payload");
        for field in ["r", "maybe", "refs"] {
            let mut document: Json = serde_json::from_slice(&valid[&identity()]).expect("snapshot");
            let hostile = raw_reference(&changed);
            document["populations"][0]["objects"][0]["fields"][field] = match field {
                "r" => hostile,
                "maybe" => json!({"present":hostile}),
                "refs" => json!({"sequence":[hostile,raw_reference(reference.object().as_str())]}),
                _ => unreachable!("three transport fields"),
            };
            let snapshots =
                BTreeMap::from([(identity(), serde_json::to_vec(&document).expect("snapshot"))]);
            let source = model
                .admit_initial(&snapshots, &identity(), ObservationLimits::default())
                .expect_err("scope disagreement cannot admit Source state");
            let bytes = bare
                .admit_call_snapshot(
                    &checked.packages,
                    ModelNormalizationLimits::default(),
                    &snapshots,
                    &identity(),
                    ObservationLimits::default(),
                )
                .expect_err("scope disagreement cannot admit Bare observation");
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
                catalog: CatalogCode::new("invalid_runtime_input", "invalid-value"),
                fields: BTreeMap::from([
                    ("document", identity().identity),
                    ("target", changed.clone()),
                    ("population", node(CLOCKS, "calls")),
                    ("object", "c1".to_owned()),
                    ("field", field.to_owned()),
                    (
                        "path",
                        format!("/populations/0/objects/0/fields/{field}{suffix}"),
                    ),
                ]),
                path,
                kernel: None,
            };
            for failure in [source, bytes] {
                let AdmissionFailure::Refused(record) = failure else {
                    panic!("typed invalid input refusal")
                };
                assert_eq!(
                    *record, expected,
                    "{changed_component} disagreement retains its original {field} locus"
                );
            }
        }
    }

    let (_, foreign_universe) = fsl_semantics::model::observation::declared_type_universe_for(
        package.graph().model_inputs(),
        &checked.packages,
        ModelNormalizationLimits::default(),
        &key(QUEUE, "Queue"),
    )
    .expect("actual selected Queue declaration")
    .expect("Queue has a declared universe");
    assert_ne!(
        &foreign_universe,
        reference.universe(),
        "the negative uses a distinct real catalog universe"
    );
    let hostile = ObjectReference::new(
        foreign_universe,
        reference.object_type().clone(),
        reference.object().clone(),
    );
    let keys = package
        .graph()
        .protocol_node_keys("Places")
        .expect("protocol keys");
    let binder = &keys
        .binders
        .iter()
        .find(|binder| binder.name == "chosen")
        .expect("Clock DATA binder")
        .node;
    let slot = ProtocolSlot {
        node: keys.clause.clone().expect("protocol root"),
        member: "captures".to_owned(),
        index: Some(4),
    };
    let selection = fsl_semantics::model::domain_package::DomainPackageRef {
        identity: CLOCKS.to_owned(),
    };
    let source_view = package.evaluation_view();
    let source = source_view.admit_protocol_observation(
        &selection,
        &checked.packages,
        ModelNormalizationLimits::default(),
        &valid,
        &identity(),
        ObservationLimits::default(),
    );
    let bytes = bare.admit_protocol_observation(
        &selection,
        &checked.packages,
        ModelNormalizationLimits::default(),
        &valid,
        &identity(),
        ObservationLimits::default(),
    );
    for (evaluator, observation) in [
        (&package as &dyn CheckedPackageEvaluation, &source),
        (&bare as &dyn CheckedPackageEvaluation, &bytes),
    ] {
        let ObservationAdmission::Admitted(admitted) = observation else {
            panic!("actual complete owner observation admits")
        };
        let population = admitted
            .environment()
            .population_node(&node(CLOCKS, "calls"))
            .expect("actual owner population")
            .clone();
        let supply = ProtocolSupply {
            binders: vec![(
                binder.clone(),
                ProtocolSuppliedValue {
                    value: SuppliedValue::Value(Value::Reference(hostile.clone())),
                    references: vec![ProtocolReferenceOccurrence {
                        path: Vec::new(),
                        population,
                    }],
                },
            )],
            observation: Some(observation),
        };
        let failure = evaluator
            .evaluate_protocol_slot(&slot, &supply, &mut Meter::new(LIMITS))
            .expect_err("outer Clock universe disagreement is invalid input");
        let CallFailure::Admission(failure) = failure else {
            panic!("typed call admission refusal")
        };
        let CallAdmissionFailure::Refused(record) = *failure else {
            panic!("complete owner population is not incomplete")
        };
        assert_eq!(
            record.record,
            AdmissionRecord {
                catalog: CatalogCode::new("invalid_runtime_input", "invalid-value"),
                fields: BTreeMap::new(),
                path: Vec::new(),
                kernel: None
            }
        );
        assert_eq!(record.locus, CallAdmissionLocus::Binder(binder.clone()));
    }
}

/// Both routes enter the same atomic declaration table before admission.
/// Colliding entry keys reject both distinct real origins; repeated views of
/// each same original declaration retain one entry and admit real data.
#[trace("FR-106-AC-77", "TC-464")]
#[test]
#[ignore = "QSL-2897: end-to-end raw-v2 Clock reference support remains unsupported (Source missing-route-neutral-place-identity; Bare missing-type-representation)"]
fn declared_table_collision_refuses_both_origins_and_aliases_admit_on_both_routes() {
    use fsl_semantics::model::clock_reference::normalization::ClockDeclarationTable;
    use fsl_semantics::model::observation::ModelSelections;
    let checked = fixture(false);
    let package = CheckedPackage::link(checked.graph);
    let snapshots = snapshot(&expected_place("c1", "x"), &expected_place("c2", "x"), true);
    let Value::Reference(actual) = capture(&package, &checked.packages, &snapshots, 0, "c1") else {
        panic!("actual seeded producer");
    };
    let read = fsl_package::read_evaluation_package(
        &package,
        fsl_semantics::library::LibraryName::new(package.graph().package_name()).expect("name"),
        &checked.packages,
        V2ReadLimits::fixture(),
        EmitLimits::fixture(),
    )
    .expect("actual read");
    let rows = read
        .emission()
        .identity_rows(
            read.admitted(),
            &mut V2ReadLimits::fixture().native_join_budget(),
        )
        .expect("rows");
    let equality = read
        .emission()
        .equality_witness(
            read.admitted(),
            &mut V2ReadLimits::fixture().native_join_budget(),
        )
        .expect("equality");
    let bare = BarePackage::new(read.admitted())
        .with_supply(
            BareSupply {
                identities: &rows,
                equality: Some(&equality),
            },
            &mut V2ReadLimits::fixture().native_join_budget(),
        )
        .expect("Bare");
    let bare_table = bare
        .clock_declaration_table()
        .expect("actual admitted declarations");
    let bare_world = bare.world().expect("actual admitted world");
    for (table, types) in [
        (
            package.graph().clock_declaration_table(),
            package.graph().scope().types(),
        ),
        (&bare_table, bare_world.types()),
    ] {
        let entries = table.entries().collect::<Vec<_>>();
        let (template, first) = entries[0];
        let (_, second) = entries
            .iter()
            .copied()
            .find(|(_, origin)| *origin != first)
            .expect("two distinct actual original declarations");
        // Exercise the declared-table entry boundary under its collision
        // precondition, using genuine origins from this route. This is not
        // a claim that the fixture's valid source normalizes inconsistently.
        let collision = ClockDeclarationTable::from_entries([
            (template.clone(), first.clone()),
            (template.clone(), second.clone()),
        ])
        .expect_err("a conflicting table is not returned to admission");
        assert_eq!(&collision.first, first);
        assert_eq!(&collision.second, second);
        let cause =
            fsl_semantics::check::CheckCause::DuplicateClockDeclaration(Box::new(collision));
        assert_eq!(
            cause.code(),
            fsl_foundation::diagnostic::Code::ConflictingDeclaration
        );
        assert_eq!(cause.cause(), Some("duplicate-declaration"));
        let aliases = ClockDeclarationTable::from_entries(
            table
                .entries()
                .chain(table.entries())
                .map(|(template, origin)| (template.clone(), origin.clone())),
        )
        .expect("two aliases of each original retain one table view")
        .with_source_declarations(table.source_declarations().clone())
        .with_run_places(table.run_places().map(|(node, place)| {
            (
                node.clone(),
                place.owner.clone(),
                place.template.as_ref().clone(),
            )
        }))
        .expect("actual run declarations retain their checked identity");
        assert_eq!(
            aliases.entries().collect::<Vec<_>>(),
            table.entries().collect::<Vec<_>>()
        );
        let selections = [fsl_semantics::model::domain_package::DomainPackageRef {
            identity: CLOCKS.to_owned(),
        }];
        let model = StateModel::new(
            ModelSelections::new(&selections, &[]).with_clock_declaration_table(&aliases),
            types,
            &checked.packages,
            ModelNormalizationLimits::default(),
        )
        .expect("alias table enters the actual Model");
        let (state, _) = model
            .admit_initial(&snapshots, &identity(), ObservationLimits::default())
            .expect("otherwise valid snapshot admits with the single alias view");
        let stored = &state.populations[&key(CLOCKS, "calls")]["c1"].fields[&key(CLOCKS, "Call/r")];
        assert_value(stored, &Value::Reference(actual.clone()));
        model
            .observation(&state, identity())
            .expect("no partial or counterfeit observation");
    }
}

/// RAW Clock DATA is an explicit implementation boundary. Source refusal
/// keeps its actual value locus; Bare may stop earlier at type projection.
#[trace("FR-106", "TC-464")]
#[test]
fn raw_clock_data_has_explicit_source_and_bare_refusals() {
    let checked = fixture(false);
    let package = CheckedPackage::link(checked.graph);
    let read = fsl_package::read_evaluation_package(
        &package,
        fsl_semantics::library::LibraryName::new(package.graph().package_name()).expect("name"),
        &checked.packages,
        V2ReadLimits::fixture(),
        EmitLimits::fixture(),
    )
    .expect("the foundation emits and its strict reader admits");
    let rows = read
        .emission()
        .identity_rows(
            read.admitted(),
            &mut V2ReadLimits::fixture().native_join_budget(),
        )
        .expect("actual identity rows");
    let equality = read
        .emission()
        .equality_witness(
            read.admitted(),
            &mut V2ReadLimits::fixture().native_join_budget(),
        )
        .expect("actual equality rows");
    let bare = BarePackage::new(read.admitted())
        .with_supply(
            BareSupply {
                identities: &rows,
                equality: Some(&equality),
            },
            &mut V2ReadLimits::fixture().native_join_budget(),
        )
        .expect("the actual Bare view");
    for target in [
        expected_place("c1", "x"),
        expected_place("c1", "removed"),
        "place/v1/not-supported".to_owned(),
    ] {
        let snapshots = snapshot(&target, &expected_place("c2", "x"), true);
        let selected = fsl_semantics::model::domain_package::DomainPackageRef {
            identity: CLOCKS.to_owned(),
        };
        let source = package.evaluation_view().admit_protocol_observation(
            &selected,
            &checked.packages,
            ModelNormalizationLimits::default(),
            &snapshots,
            &identity(),
            ObservationLimits::default(),
        );
        let ObservationAdmission::Refused(record) = source else {
            panic!("explicit Source boundary: {source:?}")
        };
        assert_eq!(
            record,
            AdmissionRecord {
                catalog: CatalogCode::new("unknown_required_feature", "unsupported-feature"),
                fields: BTreeMap::from([
                    ("reason", "missing-route-neutral-place-identity".to_owned()),
                    ("document", identity().identity),
                    ("population", node(CLOCKS, "calls")),
                    ("object", "c1".to_owned()),
                    ("field", "r".to_owned()),
                    ("target", target),
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
        let admission = bare.admit_protocol_observation(
            &selected,
            &checked.packages,
            ModelNormalizationLimits::default(),
            &snapshots,
            &identity(),
            ObservationLimits::default(),
        );
        let ObservationAdmission::Refused(record) = admission else {
            panic!("explicit Bare boundary: {admission:?}")
        };
        assert_eq!(
            record,
            AdmissionRecord {
                catalog: CatalogCode::new("unknown_required_feature", "unsupported-feature"),
                fields: BTreeMap::from([
                    ("package", "fixture".to_owned()),
                    ("reason", "missing-type-representation".to_owned())
                ]),
                path: Vec::new(),
                kernel: None,
            }
        );
    }
}

/// An ordinary snapshot supplies real owner population authority; generated
/// Clock references do not require accepting transported Clock DATA.
#[trace("FR-230", "TC-3084")]
#[test]
fn source_generates_clock_references_from_ordinary_admitted_owners() {
    let body = format!("{}\nprotocol Places using p over (call: C::Call) on each (opened: Boolean) when (opened) {{
        capture chosen: Reference<Clock> = ref(call.x);
        role Observer on C::Call;
        run sequence Main {{ check Ready using v {{ true }}; }}
        finish End as (outcome: Boolean) {{ true }};
    }}", clock_refinement("used = count(c in C::calls where true); busy = false;"));
    let checked = ring_queue::check_models_with_profiles(
        &ring_queue::queue_spec(),
        ("C", CLOCKS, &ring_queue::clock_calls()),
        true,
        "profile p = \"filament.protocol.finite-global/v1\";\n",
        &body,
    )
    .unwrap_or_else(|refusals| {
        panic!(
            "the ordinary-owner fixture checks: {:?}",
            refusals
                .iter()
                .map(|refusal| (&refusal.refusal, refusal.at.as_str()))
                .collect::<Vec<_>>()
        )
    });
    let package = CheckedPackage::link(checked.graph);
    let objects: Vec<_> = [("c1", "1"), ("c2", "2")].into_iter().map(|(key, n)| json!({
        "key":key, "type":node(CLOCKS,"Call"), "fields":{"n":{"integer":n},"d":{"integer":"0"}}
    })).collect();
    let document = json!({
        "format":"filament.state.snapshot/v1", "identity":{"authority":"test","identity":"clock-v2"},
        "observation":"current", "model":{"identity":CLOCKS}, "anchor":{"kind":"initialization","name":"start"},
        "populations":[{"population":node(CLOCKS,"calls"),"complete":true,"objects":objects}]
    });
    let snapshots =
        BTreeMap::from([(identity(), serde_json::to_vec(&document).expect("snapshot"))]);
    let mut generated = Vec::new();
    for owner in ["c1", "c2"] {
        let Value::Reference(reference) =
            capture(&package, &checked.packages, &snapshots, 0, owner)
        else {
            panic!("the actual producer returns Clock DATA")
        };
        assert_eq!(reference.object().as_str(), expected_place(owner, "x"));
        generated.push(reference);
    }
    assert_eq!(generated[0].universe(), generated[1].universe());
    assert_eq!(generated[0].object_type(), generated[1].object_type());
    assert_ne!(generated[0].object(), generated[1].object());
}
