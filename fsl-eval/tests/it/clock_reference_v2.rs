// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL-2178 retains the end-to-end acceptance oracles as explicit debt.
//! Executed controls assert the supported foundation and typed RAW refusals.

use std::collections::BTreeMap;

use filament_eval::{Evaluation, FamilyOutcome, FamilyResult};
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
    fixture_with(remove_x, QSlots::None)
}

/// The `q: Sequence<Clock>` place a fixture adds.
#[derive(Clone, Copy)]
enum QSlots {
    /// No `q`.
    None,
    /// No declared maximum (FR-106-AC-81).
    Unbounded,
    /// A declared maximum.
    Max(u32),
}

/// The fixture with the `q` place `q` names.
fn fixture_with(remove_x: bool, q_slots: QSlots) -> Checked {
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
    let mut queue = add_reference(ring_queue::queue_spec(), QUEUE, "Queue", "chosen");
    queue["types"]
        .as_array_mut()
        .expect("types")
        .iter_mut()
        .find(|ty| ty["identity"] == node(QUEUE, "Queue"))
        .expect("Queue")
        .get_mut("fields")
        .and_then(Json::as_array_mut)
        .expect("Queue fields")
        .push(ring_queue::field(
            QUEUE,
            "Queue",
            "clock_set",
            &node(QUEUE, "ClockRef"),
            (0, 3, false),
        ));
    let queue_fields = queue["types"]
        .as_array_mut()
        .expect("types")
        .iter_mut()
        .find(|ty| ty["identity"] == node(QUEUE, "Queue"))
        .expect("Queue")
        .get_mut("fields")
        .and_then(Json::as_array_mut)
        .expect("Queue fields");
    queue_fields.extend([
        ring_queue::single(QUEUE, "Queue", "set_count", "ix://filament/native/Integer"),
        ring_queue::single(QUEUE, "Queue", "set_exists", "ix://filament/native/Boolean"),
        ring_queue::single(QUEUE, "Queue", "set_all", "ix://filament/native/Boolean"),
        ring_queue::single(QUEUE, "Queue", "set_only", &node(QUEUE, "ClockRef")),
    ]);
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
    fields.push(ring_queue::field(
        CLOCKS,
        "Call",
        "clock_set",
        &node(CLOCKS, "ClockRef"),
        (0, 3, false),
    ));
    if !matches!(q_slots, QSlots::None) {
        let mut q = ring_queue::single(CLOCKS, "Call", "q", "ix://filament/native/Clock");
        q["multiplicity"] = json!({"lower": 0, "ordered": true, "unique": false});
        if let QSlots::Max(upper) = q_slots {
            q["multiplicity"]["upper"] = json!(upper);
        }
        fields.push(q);
    }
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
        "type ClockRefCount = Integer;\n{}\n\
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
             chosen = ref(self.{selected});\n\
             set_count = count<ClockRefCount>(item in self.clock_set: item = ref(self.{selected}));\n\
             set_exists = exists(item in self.clock_set: item = ref(self.{selected}));\n\
             set_all = forall(item in self.clock_set: item = ref(self.{selected}));\n\
             set_only = only(item in self.clock_set: item);"
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

/// A small source model with no scalar Reference<Clock> fields. The only
/// clock-reference input is an empty placeholder Set, which the AC test
/// replaces with typed candidates from ModelSystem's domain.
fn clock_set_fixture() -> Checked {
    let mut queue: Json = serde_json::from_slice(&ring_queue::queue_spec()).expect("queue JSON");
    let mut calls: Json = serde_json::from_slice(&ring_queue::clock_calls()).expect("calls JSON");
    for (document, domain) in [(&mut queue, QUEUE), (&mut calls, CLOCKS)] {
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
    }
    queue["types"]
        .as_array_mut()
        .expect("types")
        .iter_mut()
        .find(|ty| ty["identity"] == node(QUEUE, "Queue"))
        .expect("Queue")
        .get_mut("fields")
        .and_then(Json::as_array_mut)
        .expect("Queue fields")
        .extend([
            ring_queue::field(
                QUEUE,
                "Queue",
                "clock_set",
                &node(QUEUE, "ClockRef"),
                (0, 3, false),
            ),
            ring_queue::single(QUEUE, "Queue", "set_count", "ix://filament/native/Integer"),
            ring_queue::single(QUEUE, "Queue", "set_exists", "ix://filament/native/Boolean"),
            ring_queue::single(QUEUE, "Queue", "set_all", "ix://filament/native/Boolean"),
            ring_queue::single(QUEUE, "Queue", "set_only", &node(QUEUE, "ClockRef")),
        ]);
    calls["types"]
        .as_array_mut()
        .expect("types")
        .iter_mut()
        .find(|ty| ty["identity"] == node(CLOCKS, "Call"))
        .expect("Call")
        .get_mut("fields")
        .and_then(Json::as_array_mut)
        .expect("Call fields")
        .push(ring_queue::field(
            CLOCKS,
            "Call",
            "clock_set",
            &node(CLOCKS, "ClockRef"),
            (0, 3, false),
        ));
    let rows = "set_count = count<ClockRefCount>(item in self.clock_set: item = item);\n\
                set_exists = exists(item in self.clock_set: item = item);\n\
                set_all = forall(item in self.clock_set: item = item);\n\
                set_only = only(item in (self.clock_set): item);";
    let body = format!("type ClockRefCount = Integer;\n{}", clock_refinement(rows));
    ring_queue::check_models_with_profiles(
        &serde_json::to_vec(&queue).expect("queue"),
        ("C", CLOCKS, &serde_json::to_vec(&calls).expect("calls")),
        true,
        "",
        &body,
    )
    .unwrap_or_else(|refusals| {
        panic!(
            "Set-only fixture checks: {:?}",
            refusals
                .iter()
                .map(|refusal| (&refusal.refusal, refusal.at.as_str()))
                .collect::<Vec<_>>()
        )
    })
}

fn clock_set_snapshot() -> BTreeMap<DocumentRef, Vec<u8>> {
    let objects = [("c1", "1"), ("c2", "2")]
        .into_iter()
        .map(|(owner, n)| {
            json!({
                "key": owner,
                "type": node(CLOCKS, "Call"),
                "fields": {"n": {"integer": n}, "d": {"integer": "0"}, "clock_set": {"set": []}}
            })
        })
        .collect::<Vec<_>>();
    let document = json!({
        "format": "filament.state.snapshot/v1",
        "identity": {"authority": "test", "identity": "clock-v2"},
        "observation": "current",
        "model": {"identity": CLOCKS},
        "anchor": {"kind": "initialization", "name": "start"},
        "populations": [{"population": node(CLOCKS, "calls"), "complete": true, "objects": objects}]
    });
    BTreeMap::from([(identity(), serde_json::to_vec(&document).expect("snapshot"))])
}

fn replace_encoded_field(node: &mut Json, name: &str, replacement: Json) -> bool {
    match node {
        Json::Object(members) => {
            if members.get("name").and_then(Json::as_str) == Some(name) {
                if let Some(value) = members.get_mut("value") {
                    *value = replacement;
                    return true;
                }
            }
            members
                .values_mut()
                .any(|member| replace_encoded_field(member, name, replacement.clone()))
        }
        Json::Array(items) => items
            .iter_mut()
            .any(|item| replace_encoded_field(item, name, replacement.clone())),
        _ => false,
    }
}

struct RestoreBudget {
    meter: Meter,
    used: usize,
}

impl filament_exact::Reservation for RestoreBudget {
    fn reserve(&mut self, bytes: usize) -> Result<(), filament_exact::ReservationDenied> {
        self.used = self
            .used
            .checked_add(bytes)
            .filter(|used| *used <= 4_000_000)
            .ok_or(filament_exact::ReservationDenied)?;
        Ok(())
    }
}

impl filament_exact::IntegerDecimalBudget for RestoreBudget {
    fn work(&mut self, units: u64) -> Result<(), filament_exact::ReservationDenied> {
        self.meter
            .charge(
                filament_exact::Charge::new(filament_exact::ChargePoint::GraphEdge)
                    .work(Integer::from(units)),
            )
            .map_err(|_| filament_exact::ReservationDenied)
    }
}

impl fsl_eval::simulation::model::ModelKeyBudget for RestoreBudget {
    fn integer(&mut self, value: &Integer) -> Result<(), filament_exact::ReservationDenied> {
        self.meter
            .charge(
                filament_exact::Charge::new(filament_exact::ChargePoint::GraphResultRetain).size(
                    filament_exact::LimitKind::IntegerBits,
                    value.magnitude_bits(),
                ),
            )
            .map_err(|_| filament_exact::ReservationDenied)
    }
}

impl fsl_eval::simulation::model::ModelRestoreBudget for RestoreBudget {
    fn meter(&mut self) -> &mut Meter {
        &mut self.meter
    }
}

#[trace("TC-3084", "FR-1040-AC-12")]
#[test]
fn clock_reference_values_restore_from_state_key_with_equality() {
    use fsl_eval::simulation::model::{
        ExpansionLimits, ModelCheckInput, ModelSimulationRequest, ModelSystem, PopulationUniverse,
    };
    use fsl_semantics::model::accounting::ModelNormalizationLimits;

    let checked = clock_set_fixture();
    let package = CheckedPackage::link(checked.graph);
    let snapshots = clock_set_snapshot();
    let population = key(CLOCKS, "calls");
    let system = ModelSystem::new(ModelSimulationRequest {
        emit: EmitLimits::fixture(),
        input: ModelCheckInput::source(&package),
        packages: &checked.packages,
        normalization: ModelNormalizationLimits::default(),
        snapshots: &snapshots,
        initial: vec![identity()],
        universes: vec![PopulationUniverse {
            population: population.clone(),
            keys: vec!["c1".to_owned(), "c2".to_owned()],
        }],
        observation: ObservationLimits::default(),
        expansion: ExpansionLimits {
            max_candidates: 1_000,
        },
        meter: LIMITS,
    })
    .expect("the Source ModelSystem admits ordinary owners");
    let original = &system.initial_states()[0];
    let (initial_bytes, _) = system.state_key(original).expect("state key encodes");
    let mut encoded: Json = serde_json::from_slice(&initial_bytes).expect("canonical state key");
    let clock_set = json!({
        "type": "set",
        "elements": [
            {"type": "clock_reference", "place_id": expected_place("c1", "x")},
            {"type": "clock_reference", "place_id": expected_place("c2", "x")}
        ]
    });
    assert!(replace_encoded_field(&mut encoded, "clock_set", clock_set));
    let bytes = serde_json::to_vec(&encoded).expect("canonical seeded state key");
    let mut budget = RestoreBudget {
        meter: Meter::new(LIMITS),
        used: 0,
    };
    let restored = system
        .restore_state_key_with_budget(
            &bytes,
            fsl_foundation::IdentityLimits::default(),
            &mut budget,
            &filament_exact::Cancel::new(),
        )
        .expect("clock-reference state key restores");
    let (restored_bytes, _) = system.state_key(&restored).expect("restored key encodes");
    assert_eq!(restored_bytes, bytes);

    let field = system
        .fields_of(&key(CLOCKS, "Call"))
        .expect("Call fields")
        .iter()
        .find(|field| field.name == "clock_set")
        .expect("clock_set field")
        .key
        .clone();
    let restored_value = &restored.state().populations[&population]["c1"].fields[&field];
    let Value::Collection(restored_set) = restored_value else {
        panic!("restored value is a typed ClockReference Set");
    };
    let restored_ids = restored_set
        .elements()
        .iter()
        .map(|value| match value {
            Value::ClockReference(reference) => reference.text().to_owned(),
            _ => panic!("each restored Set member is a ClockReference"),
        })
        .collect::<Vec<_>>();
    assert_eq!(
        restored_ids,
        [expected_place("c1", "x"), expected_place("c2", "x")]
    );
    for value in restored_set.elements() {
        let Value::ClockReference(reference) = value else {
            unreachable!("checked above")
        };
        let matches_candidate = system
            .clock_reference_identities()
            .expect("candidate identities")
            .iter()
            .any(|candidate| {
                candidate.object().as_str() == reference.text()
                    && candidate.universe() == reference.universe()
            });
        assert!(
            matches_candidate,
            "restored typed identity equals its model candidate"
        );
    }
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
            "refs":{"sequence":[raw_reference(second),raw_reference(first)]},
            // Source admission needs a concrete placeholder; the AC-11 test
            // replaces this field with values from the typed model domain.
            "clock_set":{"set":[]}
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
#[ignore = "QSL-2776: Bare cannot read a Reference<Clock> data member (Contract IR object-field projection)"]
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
#[ignore = "QSL-2776: Bare cannot read a Reference<Clock> data member (Contract IR object-field projection); the Source half passes"]
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
#[ignore = "QSL-2776: Bare cannot read a Reference<Clock> data member (Contract IR object-field projection)"]
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
#[ignore = "QSL-2776: Bare cannot read a Reference<Clock> data member (Contract IR object-field projection)"]
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
#[ignore = "QSL-2776: Bare cannot read a Reference<Clock> data member (Contract IR object-field projection)"]
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

/// Source admits a declared Clock identity and refuses an undeclared or
/// malformed one at its actual value locus (FR-106 AC-72, AC-73). The Bare
/// route stops earlier, at type projection.
#[trace("FR-106", "TC-464")]
#[test]
#[ignore = "QSL-3002: raw-v2 clock snapshot admission and Source/Bare routing are slice C"]
fn raw_clock_data_admits_declared_and_refuses_undeclared_identities() {
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
        let catalog = if target == expected_place("c1", "x") {
            assert!(
                matches!(source, ObservationAdmission::Admitted(_)),
                "the declared place admits: {source:?}"
            );
            None
        } else if target.starts_with("place/v2/") {
            Some(CatalogCode::new(
                "dangling_reference",
                "absent-target-in-complete-population",
            ))
        } else {
            Some(CatalogCode::new("invalid_runtime_input", "invalid-value"))
        };
        if let Some(catalog) = catalog {
            let ObservationAdmission::Refused(record) = source else {
                panic!("typed Source refusal: {source:?}")
            };
            assert_eq!(
                record,
                AdmissionRecord {
                    catalog,
                    fields: BTreeMap::from([
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
        }
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
#[trace("FR-230-AC-71", "TC-3084", "FR-1040-AC-10")]
#[test]
fn source_generates_clock_references_from_ordinary_admitted_owners() {
    let body = format!("{}
        protocol Places using p over (call: C::Call) on each (opened: Boolean) when (opened) {{
            capture chosen: Reference<Clock> = ref(call.x);
            role Observer on C::Call;
            run sequence Main {{ check Ready using v {{ true }}; }}
            finish End as (outcome: Boolean) {{ true }};
        }}
        invariant Stored using m on C::Call at current {{ ref(self.x) = ref(self.x) }}
        function same_clock(left: Reference<Clock>, right: Reference<Clock>): Boolean pure {{ left = right }}
        function differ(left: Reference<Clock>, right: Reference<Clock>): Boolean pure {{ left != right }}
        ",
        clock_refinement("used = count(c in C::calls where true); busy = false;"));
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
    assert_producer_clause(
        &package,
        &package,
        package.graph().scope().types(),
        &checked.packages,
        &snapshots,
        true,
    );

    let model = source_model(&checked.packages, &package);
    let (state, _) = model
        .admit_initial(&snapshots, &identity(), ObservationLimits::default())
        .expect("the complete ordinary-owner population admits");
    let observation = model
        .observation(&state, identity())
        .expect("the admitted state observes");
    let same_clock = QualifiedName::unqualified("same_clock").expect("function name");
    let universe = crate::support::ids::universe_id(b"clock-reference-test");
    let clock = |owner: &str, field: &str| {
        Value::ClockReference(
            filament_exact::ClockPlaceReference::new(
                universe.clone(),
                &expected_place(owner, field),
            )
            .expect("canonical place reference"),
        )
    };
    let compare = |left: Value, right: Value| {
        completed(
            package
                .call(
                    &same_clock,
                    vec![left, right],
                    &observation.environment,
                    &mut Meter::new(LIMITS),
                )
                .expect("typed ClockReference comparison completes"),
        )
    };
    assert!(matches!(
        compare(clock("c1", "x"), clock("c1", "x")),
        Value::Boolean(true)
    ));
    assert!(matches!(
        compare(clock("c1", "x"), clock("c1", "y")),
        Value::Boolean(false)
    ));
    assert!(matches!(
        compare(clock("c1", "x"), clock("c2", "x")),
        Value::Boolean(false)
    ));
    // FR-829-AC-8: across universes `=` and `!=` never refuse; references
    // are equal exactly when their place id texts are.
    let foreign_universe = crate::support::ids::universe_id(b"foreign-clock-reference-test");
    let foreign = |owner: &str, field: &str| {
        Value::ClockReference(
            filament_exact::ClockPlaceReference::new(
                foreign_universe.clone(),
                &expected_place(owner, field),
            )
            .expect("place reference in another universe"),
        )
    };
    assert!(matches!(
        compare(clock("c1", "x"), foreign("c1", "x")),
        Value::Boolean(true)
    ));
    assert!(matches!(
        compare(clock("c1", "x"), foreign("c2", "x")),
        Value::Boolean(false)
    ));
    assert!(matches!(
        compare(clock("c1", "x"), foreign("c1", "y")),
        Value::Boolean(false)
    ));
    let differ = QualifiedName::unqualified("differ").expect("function name");
    let compare_ne = |left: Value, right: Value| {
        completed(
            package
                .call(
                    &differ,
                    vec![left, right],
                    &observation.environment,
                    &mut Meter::new(LIMITS),
                )
                .expect("typed ClockReference inequality completes"),
        )
    };
    assert!(matches!(
        compare_ne(clock("c1", "x"), foreign("c1", "x")),
        Value::Boolean(false)
    ));
    assert!(matches!(
        compare_ne(clock("c1", "x"), foreign("c2", "x")),
        Value::Boolean(true)
    ));
    let mut generated = Vec::new();
    for owner in ["c1", "c2"] {
        let Value::ClockReference(reference) =
            capture(&package, &checked.packages, &snapshots, 0, owner)
        else {
            panic!("the actual producer returns a clock-place reference")
        };
        assert_eq!(reference.text(), expected_place(owner, "x"));
        generated.push(reference);
    }
    assert_eq!(generated[0].universe(), generated[1].universe());
    assert_ne!(generated[0].text(), generated[1].text());
}

/// FR-1040-AC-11: all four set aggregators consume ClockReference elements in
/// the same canonical place-id order, regardless of the constructor order.
#[trace("TC-3084", "FR-1040-AC-11")]
#[test]
fn clock_reference_set_aggregators_use_the_canonical_member_order() {
    use fsl_eval::simulation::model::{
        ExpansionLimits, ModelCheckInput, ModelSimulationRequest, ModelSystem, PopulationUniverse,
    };
    use fsl_eval::value::evaluate_refinement_row;
    use fsl_semantics::model::accounting::ModelNormalizationLimits;
    use fsl_semantics::model::observation::declared_type_universe_for;

    let checked = clock_set_fixture();
    let package = CheckedPackage::link(checked.graph);
    let snapshots = clock_set_snapshot();
    let initial = identity();
    let population = key(CLOCKS, "calls");
    let system = ModelSystem::new(ModelSimulationRequest {
        emit: EmitLimits::fixture(),
        input: ModelCheckInput::source(&package),
        packages: &checked.packages,
        normalization: ModelNormalizationLimits::default(),
        snapshots: &snapshots,
        initial: vec![initial.clone()],
        universes: vec![PopulationUniverse {
            population: population.clone(),
            keys: vec!["c1".to_owned(), "c2".to_owned()],
        }],
        observation: ObservationLimits::default(),
        expansion: ExpansionLimits {
            max_candidates: 1_000,
        },
        meter: LIMITS,
    })
    .expect("the Source ModelSystem admits ordinary owners");
    let initial_state = &system.initial_states()[0];
    let visible = initial_state.state();
    let call = &visible.populations[&population]["c1"];
    let field = system
        .fields_of(&call.type_identity)
        .expect("Call fields")
        .iter()
        .find(|field| field.name == "clock_set")
        .expect("the unordered ClockReference Set field")
        .key
        .clone();
    let anchor = initial_state.anchor().expect("anchored state").clone();
    let mut budget = 1_000_000;
    let completed = system
        .complete_open_fields(
            visible,
            &anchor,
            &[(population.clone(), "c1".to_owned(), field.clone())],
            &mut budget,
        )
        .expect("the typed model domain supplies ClockReference Sets");
    let (call_type, call_universe) = declared_type_universe_for(
        package.graph().model_selections(),
        &checked.packages,
        ModelNormalizationLimits::default(),
        &key(CLOCKS, "Call"),
    )
    .expect("Call universe resolves")
    .expect("Call is selected");
    let receiver = ObjectReference::new(
        call_universe,
        call_type,
        filament_exact::ObjectId::new("c1").expect("receiver id"),
    );
    let [refinement] = package.graph().refinements() else {
        panic!("one refinement");
    };
    let [queue, _cell] = refinement.objects() else {
        panic!("Queue and Cell rows");
    };
    let model = source_model(&checked.packages, &package);
    let row_value = |state: &_, name: &str| {
        let field = key(QUEUE, &format!("Queue/{name}"));
        let expression = &queue
            .fields
            .iter()
            .find(|row| row.field == field)
            .unwrap_or_else(|| {
                panic!(
                    "aggregate row {name} not found among {:?}",
                    queue
                        .fields
                        .iter()
                        .map(|row| &row.field)
                        .collect::<Vec<_>>()
                )
            })
            .expression;
        let environment = model
            .observation(
                state,
                DocumentRef {
                    authority: "test".to_owned(),
                    identity: "clock-v2".to_owned(),
                },
            )
            .expect("generated typed Set state observes")
            .environment;
        evaluate_refinement_row(
            &package,
            expression,
            &environment,
            vec![Value::Reference(receiver.clone())],
            &mut Meter::new(LIMITS),
        )
        .expect("Set aggregate row evaluates")
        .outcome
    };
    assert!(!completed.is_empty(), "the Set domain has values");
    for state in &completed {
        let Some(Value::Collection(set)) = state.populations[&population]["c1"].fields.get(&field)
        else {
            panic!("the generated field is a typed Set");
        };
        let refs = set.elements();
        let place_ids = refs.iter().map(|value| {
            let Value::ClockReference(reference) = value else {
                panic!("a Set member is a clock reference");
            };
            reference.text().as_bytes()
        });
        assert!(place_ids.is_sorted());
        let count = row_value(state, "set_count");
        assert!(
            matches!(
                count,
                FamilyOutcome::Evaluated(Outcome::Completed(Value::Integer(ref value)))
                    if value == &Integer::from(refs.len() as i64)
            ),
            "count over {} Set members: {count:?}",
            refs.len()
        );
        assert!(matches!(
            row_value(state, "set_exists"),
            FamilyOutcome::Evaluated(Outcome::Completed(Value::Boolean(value)))
                if value == !refs.is_empty()
        ));
        assert!(matches!(
            row_value(state, "set_all"),
            FamilyOutcome::Evaluated(Outcome::Completed(Value::Boolean(value)))
                if value
        ));
        let only = row_value(state, "set_only");
        if refs.len() == 1 {
            let FamilyOutcome::Evaluated(Outcome::Completed(value)) = only else {
                panic!("only over one Set member completes: {only:?}");
            };
            assert_value(&value, refs.first().expect("one Set member"));
        } else {
            let FamilyOutcome::FamilyEvaluated(FamilyResult::Undefined(cause)) = only else {
                panic!("only over {} Set members: {only:?}", refs.len());
            };
            let record = cause.undefined_record();
            assert_eq!(record.reason.as_str(), "only-not-one");
            assert_eq!(record.fields["population"], "collection");
        }
    }
}

/// FR-106-AC-81: a valid `Reference<Clock>` identity whose selector lies
/// beyond the exploration-derived range is admitted as data. `q` declares no
/// maximum, so slot 7 is valid for its template although `max_clocks` 4 with
/// four live scalar places derives a smaller range; the seed is never refused
/// or truncated, and reading it back preserves the exact identity.
#[trace("TC-464", "TC-3084", "FR-106-AC-80", "FR-106-AC-81")]
#[test]
#[ignore = "QSL-2776: the Bare half cannot read a Reference<Clock> data member (Contract IR object-field projection); the Source half runs with --ignored"]
fn clock_reference_seed_ref_outside_timed_selector_range_remains_admitted() {
    use fsl_semantics::model::clock_reference::identity::{
        Decimal, PathEntry, PlaceIdentity, SlotIndex,
    };
    let checked = fixture_with(false, QSlots::Unbounded);
    let package = CheckedPackage::link(checked.graph);
    let table = package.graph().clock_declaration_table();
    let template = table
        .entries()
        .map(|(template, _)| template)
        .find(|template| matches!(template.path.last(), Some(PathEntry::Collection { .. })))
        .expect("the declared Call.q collection-of-Clock place");
    let population = PlaceIdentity::decode(
        &expected_place("c1", "x"),
        quire_canonical::Limits::new(1 << 16),
    )
    .expect("canonical producer identity")
    .population;
    let seed = PlaceIdentity {
        population,
        owner: "c1".to_owned(),
        template: template.clone(),
        indices: vec![SlotIndex {
            path_position: Decimal::new("1").expect("decimal"),
            index: Decimal::new("7").expect("decimal"),
        }],
    }
    .encode(quire_canonical::Limits::new(1 << 16))
    .expect("canonical seed identity");
    assert!(seed.starts_with("place/v2/"));
    let snapshots = snapshot(&seed, &expected_place("c2", "x"), true);
    let model = StateModel::new(
        package.graph().model_inputs(),
        package.graph().scope().types(),
        &checked.packages,
        ModelNormalizationLimits::default(),
    )
    .expect("model");
    let (state, _) = model
        .admit_initial(&snapshots, &identity(), ObservationLimits::default())
        .expect("Source retains the out-of-range reference as data");
    let stored = &state.populations[&key(CLOCKS, "calls")]["c1"].fields[&key(CLOCKS, "Call/r")];
    let Value::Reference(stored) = stored else {
        panic!("a stored Clock reference");
    };
    assert_eq!(stored.object().as_str(), seed);
    let observation = model
        .observation(&state, identity())
        .expect("the admitted state observes");
    let function = QualifiedName::unqualified("stored").expect("function");
    let read = completed(
        package
            .call(
                &function,
                vec![Value::Reference(owner(&observation.environment, "c1"))],
                &observation.environment,
                &mut Meter::new(LIMITS),
            )
            .expect("stored-data call"),
    );
    assert_value(&read, &Value::Reference(stored.clone()));
    // The Bare route admits the same snapshot and reads the same identity.
    let read = fsl_package::read_evaluation_package(
        &package,
        fsl_semantics::library::LibraryName::new(package.graph().package_name()).expect("name"),
        &checked.packages,
        V2ReadLimits::fixture(),
        EmitLimits::fixture(),
    )
    .expect("package read");
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
    let (bytes, _) = bare
        .admit_call_snapshot(
            &checked.packages,
            ModelNormalizationLimits::default(),
            &snapshots,
            &identity(),
            ObservationLimits::default(),
        )
        .expect("Bare retains the out-of-range reference as data");
    let bare_value = completed(
        bare.call(
            &function,
            vec![Value::Reference(owner(&bytes, "c1"))],
            &bytes,
            &mut Meter::new(LIMITS),
        )
        .expect("Bare stored-data call"),
    );
    assert_value(&bare_value, &Value::Reference(stored.clone()));
}

/// The canonical identity of the declared `Call.q` place of `owner` with the
/// given `(path position, index)` selectors.
fn q_identity(package: &CheckedPackage, owner: &str, selectors: &[(&str, &str)]) -> String {
    use fsl_semantics::model::clock_reference::identity::{
        Decimal, PathEntry, PlaceIdentity, SlotIndex,
    };
    let template = package
        .graph()
        .clock_declaration_table()
        .entries()
        .map(|(template, _)| template)
        .find(|template| matches!(template.path.last(), Some(PathEntry::Collection { .. })))
        .expect("the declared Call.q place");
    let population = PlaceIdentity::decode(
        &expected_place("c1", "x"),
        quire_canonical::Limits::new(1 << 16),
    )
    .expect("canonical producer identity")
    .population;
    PlaceIdentity {
        population,
        owner: owner.to_owned(),
        template: template.clone(),
        indices: selectors
            .iter()
            .map(|(position, index)| SlotIndex {
                path_position: Decimal::new(position).expect("decimal"),
                index: Decimal::new(index).expect("decimal"),
            })
            .collect(),
    }
    .encode(quire_canonical::Limits::new(1 << 16))
    .expect("canonical identity")
}

/// The `Call.q` identity of `owner` whose selector component is the raw JSON
/// `indices`, bypassing the codec's own validation (a hostile payload).
fn q_identity_raw(package: &CheckedPackage, owner: &str, indices: &str) -> String {
    let valid = q_identity(package, owner, &[("1", "0")]);
    let (head, _) = valid.rsplit_once('/').expect("five components");
    let mut text = format!("{head}/");
    for byte in indices.bytes() {
        text.push_str(&format!("{byte:02x}"));
    }
    text
}

/// `identity` retargeted to another owner key.
fn retargeted(identity: &str, owner: &str) -> String {
    let mut decoded = fsl_semantics::model::clock_reference::identity::PlaceIdentity::decode(
        identity,
        quire_canonical::Limits::new(1 << 16),
    )
    .expect("canonical identity");
    decoded.owner = owner.to_owned();
    decoded
        .encode(quire_canonical::Limits::new(1 << 16))
        .expect("canonical identity")
}

/// The snapshot with `field` of `c1` holding `reference` (nested as the field
/// needs), and the population's `complete` flag.
fn snapshot_with(
    base: &BTreeMap<DocumentRef, Vec<u8>>,
    field: &str,
    reference: &str,
    complete: bool,
) -> BTreeMap<DocumentRef, Vec<u8>> {
    let mut document: Json = serde_json::from_slice(&base[&identity()]).expect("snapshot");
    let hostile = raw_reference(reference);
    document["populations"][0]["objects"][0]["fields"][field] = match field {
        "r" => hostile,
        "maybe" => json!({"present": hostile}),
        "refs" => json!({"sequence": [hostile, raw_reference(&expected_place("c2", "x"))]}),
        _ => unreachable!("three transport fields"),
    };
    document["populations"][0]["complete"] = json!(complete);
    BTreeMap::from([(identity(), serde_json::to_vec(&document).expect("snapshot"))])
}

/// The exact Source refusal record of `field` of `c1` naming `target`.
fn source_record(catalog: CatalogCode, field: &str, target: &str) -> AdmissionRecord {
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
    AdmissionRecord {
        catalog,
        fields: BTreeMap::from([
            ("document", identity().identity),
            ("target", target.to_owned()),
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
    }
}

fn source_model(
    packages: &fsl_semantics::model::intake::PackageInput,
    package: &CheckedPackage,
) -> StateModel {
    StateModel::new(
        package.graph().model_inputs(),
        package.graph().scope().types(),
        packages,
        ModelNormalizationLimits::default(),
    )
    .expect("model")
}

/// The Source protocol observation of `snapshots`, which admits an incomplete
/// population that an initial state refuses.
fn observe<'p>(
    package: &'p CheckedPackage,
    packages: &fsl_semantics::model::intake::PackageInput,
    snapshots: &BTreeMap<DocumentRef, Vec<u8>>,
) -> ObservationAdmission<fsl_package::AdmittedProtocolObservation<'p>> {
    package.evaluation_view().admit_protocol_observation(
        &fsl_semantics::model::domain_package::DomainPackageRef {
            identity: CLOCKS.to_owned(),
        },
        packages,
        ModelNormalizationLimits::default(),
        snapshots,
        &identity(),
        ObservationLimits::default(),
    )
}

fn refused_with(
    model: &StateModel,
    snapshots: &BTreeMap<DocumentRef, Vec<u8>>,
    what: &str,
) -> AdmissionRecord {
    let failure = model
        .admit_initial(snapshots, &identity(), ObservationLimits::default())
        .expect_err(what);
    let AdmissionFailure::Refused(record) = failure else {
        panic!("{what}: a typed refusal")
    };
    *record
}

/// FR-106-AC-80: a selector the declared template does not admit refuses
/// `invalid_runtime_input`/`invalid-value` on Source, whatever the shape of
/// the fault; a valid selector, and any selector of an unbounded slot, admits.
#[trace("TC-464", "FR-106-AC-80")]
#[test]
#[ignore = "QSL-3002: raw-v2 clock snapshot admission and Source/Bare routing are slice C"]
fn invalid_clock_selectors_refuse_invalid_value_on_source() {
    let invalid = CatalogCode::new("invalid_runtime_input", "invalid-value");
    let checked = fixture_with(false, QSlots::Max(2));
    let package = CheckedPackage::link(checked.graph);
    let model = source_model(&checked.packages, &package);
    let base = snapshot(&expected_place("c1", "x"), &expected_place("c2", "x"), true);
    for (what, indices) in [
        ("a missing selector", r#"[]"#),
        (
            "an extra selector",
            r#"[["slot","1","0"],["slot","1","0"]]"#,
        ),
        ("a wrong path position", r#"[["slot","0","0"]]"#),
        ("an index at the declared maximum", r#"[["slot","1","2"]]"#),
        (
            "an index above the declared maximum",
            r#"[["slot","1","7"]]"#,
        ),
    ] {
        let target = q_identity_raw(&package, "c1", indices);
        for field in ["r", "maybe", "refs"] {
            let snapshots = snapshot_with(&base, field, &target, true);
            assert_eq!(
                refused_with(&model, &snapshots, what),
                source_record(invalid, field, &target),
                "{what} in {field}"
            );
        }
    }
    for index in ["0", "1"] {
        let target = q_identity(&package, "c1", &[("1", index)]);
        let snapshots = snapshot_with(&base, "r", &target, true);
        let (state, _) = model
            .admit_initial(&snapshots, &identity(), ObservationLimits::default())
            .expect("a valid selector admits");
        let stored = &state.populations[&key(CLOCKS, "calls")]["c1"].fields[&key(CLOCKS, "Call/r")];
        let Value::Reference(stored) = stored else {
            panic!("a stored Clock reference");
        };
        assert_eq!(stored.object().as_str(), target);
    }
}

/// FR-106-AC-80 and AC-81, Source half: an unbounded slot admits any index.
#[trace("TC-464", "FR-106-AC-81")]
#[test]
#[ignore = "QSL-3002: raw-v2 clock snapshot admission and Source/Bare routing are slice C"]
fn any_selector_of_an_unbounded_clock_slot_admits_on_source() {
    let checked = fixture_with(false, QSlots::Unbounded);
    let package = CheckedPackage::link(checked.graph);
    let model = source_model(&checked.packages, &package);
    let base = snapshot(&expected_place("c1", "x"), &expected_place("c2", "x"), true);
    let target = q_identity(&package, "c1", &[("1", "7")]);
    let (state, _) = model
        .admit_initial(
            &snapshot_with(&base, "r", &target, true),
            &identity(),
            ObservationLimits::default(),
        )
        .expect("slot 7 of an unbounded slot admits");
    let stored = &state.populations[&key(CLOCKS, "calls")]["c1"].fields[&key(CLOCKS, "Call/r")];
    let Value::Reference(stored) = stored else {
        panic!("a stored Clock reference");
    };
    assert_eq!(stored.object().as_str(), target);
}

/// FR-230-AC-45, Source half: a population or scope that disagrees with the
/// authenticated declaration is invalid input at the field's nested locus.
#[trace("TC-464", "FR-230-AC-45")]
#[test]
#[ignore = "QSL-3002: raw-v2 clock snapshot admission and Source/Bare routing are slice C"]
fn clock_scope_disagreement_is_invalid_input_at_the_nested_locus_on_source() {
    use fsl_semantics::model::clock_reference::identity::{DeclarationScope, PlaceIdentity};
    let checked = fixture(false);
    let package = CheckedPackage::link(checked.graph);
    let model = source_model(&checked.packages, &package);
    let base = snapshot(&expected_place("c1", "x"), &expected_place("c2", "x"), true);
    for component in ["population", "scope"] {
        let mut changed = PlaceIdentity::decode(
            &expected_place("c1", "x"),
            quire_canonical::Limits::new(4096),
        )
        .expect("canonical identity");
        if component == "population" {
            changed.population = DeclarationScope::Model {
                identity: QUEUE.to_owned(),
                declaration: node(QUEUE, "queues"),
            };
        } else {
            changed.template.scope = DeclarationScope::Model {
                identity: QUEUE.to_owned(),
                declaration: node(QUEUE, "Queue"),
            };
        }
        let target = changed
            .encode(quire_canonical::Limits::new(4096))
            .expect("canonical identity");
        for field in ["r", "maybe", "refs"] {
            assert_eq!(
                refused_with(
                    &model,
                    &snapshot_with(&base, field, &target, true),
                    "scope disagreement",
                ),
                source_record(
                    CatalogCode::new("invalid_runtime_input", "invalid-value"),
                    field,
                    &target,
                ),
                "{component} disagreement in {field}"
            );
        }
    }
}

/// An owner the population does not list: dangling in a complete population,
/// tolerated in an incomplete one with the scope and template checks still
/// applied (FR-106 check 7).
#[trace("TC-464", "FR-106-AC-72", "FR-106-AC-73")]
#[test]
#[ignore = "QSL-3002: raw-v2 clock snapshot admission and Source/Bare routing are slice C"]
fn an_unlisted_clock_owner_is_dangling_when_complete_and_tolerated_when_incomplete() {
    let dangling = CatalogCode::new("dangling_reference", "absent-target-in-complete-population");
    let invalid = CatalogCode::new("invalid_runtime_input", "invalid-value");
    let checked = fixture(false);
    let package = CheckedPackage::link(checked.graph);
    let model = source_model(&checked.packages, &package);
    let base = snapshot(&expected_place("c1", "x"), &expected_place("c2", "x"), true);
    let unlisted = retargeted(&expected_place("c1", "x"), "c9");
    for field in ["r", "maybe", "refs"] {
        assert_eq!(
            refused_with(
                &model,
                &snapshot_with(&base, field, &unlisted, true),
                "complete",
            ),
            source_record(dangling, field, &unlisted),
            "an unlisted owner is dangling in a complete population ({field})"
        );
        let admitted = observe(
            &package,
            &checked.packages,
            &snapshot_with(&base, field, &unlisted, false),
        );
        let ObservationAdmission::Admitted(admitted) = admitted else {
            panic!("an unlisted owner is tolerated in an incomplete population: {admitted:?}");
        };
        let rendered = format!("{:?}", admitted.environment());
        assert!(
            rendered.contains(&unlisted),
            "the tolerated identity is retained in {field}"
        );
    }
    let undeclared = retargeted(&expected_place("c1", "removed"), "c9");
    let ObservationAdmission::Refused(record) = observe(
        &package,
        &checked.packages,
        &snapshot_with(&base, "r", &undeclared, false),
    ) else {
        panic!("an undeclared place is refused");
    };
    assert_eq!(
        record,
        source_record(dangling, "r", &undeclared),
        "an unlisted owner does not excuse an undeclared place"
    );
    let mut changed = fsl_semantics::model::clock_reference::identity::PlaceIdentity::decode(
        &unlisted,
        quire_canonical::Limits::new(4096),
    )
    .expect("canonical identity");
    changed.template.scope =
        fsl_semantics::model::clock_reference::identity::DeclarationScope::Model {
            identity: QUEUE.to_owned(),
            declaration: node(QUEUE, "Queue"),
        };
    let mismatched = changed
        .encode(quire_canonical::Limits::new(4096))
        .expect("canonical identity");
    let ObservationAdmission::Refused(record) = observe(
        &package,
        &checked.packages,
        &snapshot_with(&base, "r", &mismatched, false),
    ) else {
        panic!("a scope disagreement is refused");
    };
    assert_eq!(
        record,
        source_record(invalid, "r", &mismatched),
        "an unlisted owner does not excuse a scope disagreement"
    );
}

/// FR-106-AC-73, Source half: after `x` is removed, every stored old `x`
/// identity refuses dangling at its nested locus, never resolving to `y`.
#[trace("TC-464", "FR-106-AC-73")]
#[test]
#[ignore = "QSL-3002: raw-v2 clock snapshot admission and Source/Bare routing are slice C"]
fn a_removed_clock_place_is_dangling_at_the_nested_locus_on_source() {
    let dangling = CatalogCode::new("dangling_reference", "absent-target-in-complete-population");
    let changed = fixture(true);
    let next = CheckedPackage::link(changed.graph);
    let model = source_model(&changed.packages, &next);
    let valid = snapshot(&expected_place("c1", "y"), &expected_place("c2", "y"), true);
    model
        .admit_initial(&valid, &identity(), ObservationLimits::default())
        .expect("the retained y admits");
    let old_x = expected_place("c1", "x");
    for field in ["r", "maybe", "refs"] {
        assert_eq!(
            refused_with(&model, &snapshot_with(&valid, field, &old_x, true), "old x"),
            source_record(dangling, field, &old_x),
            "old x in {field}"
        );
    }
}
