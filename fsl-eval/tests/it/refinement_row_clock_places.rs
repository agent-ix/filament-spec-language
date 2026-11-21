// SPDX-License-Identifier: AGPL-3.0-or-later
//! RAW Clock DATA and unseeded refinement receivers retain explicit
//! unsupported boundaries; declaration and codec controls remain independent.

use std::collections::BTreeMap;

use filament_eval::{Evaluation, FamilyOutcome, FamilyResult};
use filament_exact::{Integer, Meter, ObjectId, ObjectReference, ScalarLimits, Value};
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

/// A reference-dependent refinement cannot be initialized from RAW Clock
/// DATA while the route-neutral admission boundary remains unsupported.
#[trace("FR-106", "TC-464", "TC-3084")]
#[test]
fn raw_clock_data_refuses_before_refinement_evaluation() {
    use fsl_semantics::model::observation::{AdmissionFailure, ObservationLimits};
    use serde_json::json;
    let checked = clock_reference_fixture(true);
    let model = StateModel::new(
        checked.graph.model_inputs(),
        checked.graph.scope().types(),
        &checked.packages,
        ModelNormalizationLimits::default(),
    )
    .expect("actual model");
    let identity = DocumentRef {
        authority: "test".to_owned(),
        identity: "clock-snapshot".to_owned(),
    };
    for target in [
        declared_place_text("c1", "x"),
        declared_place_text("c1", "removed"),
    ] {
        let reference = json!({"reference":{"population":crate::ring_queue::node(CLOCKS,"calls"),"key":target}});
        let document = json!({
            "format":"filament.state.snapshot/v1", "identity":{"authority":identity.authority,"identity":identity.identity},
            "observation":"current", "model":{"identity":CLOCKS}, "anchor":{"kind":"initialization","name":"start"},
            "populations":[{"population":crate::ring_queue::node(CLOCKS,"calls"),"complete":true,"objects":[{
                "key":"c1","type":crate::ring_queue::node(CLOCKS,"Call"),"fields":{
                    "n":{"integer":"1"},"d":{"integer":"0"},"r":reference,
                    "maybe":{"present":reference},"refs":{"sequence":[reference]},"peer":{"absent":{}}
                }
            }]}]
        });
        let snapshots = BTreeMap::from([(
            identity.clone(),
            serde_json::to_vec(&document).expect("snapshot"),
        )]);
        let Err(AdmissionFailure::Refused(record)) =
            model.admit_initial(&snapshots, &identity, ObservationLimits::default())
        else {
            panic!("RAW Clock DATA cannot initialize a refinement")
        };
        assert_eq!(record.catalog.code(), "unknown_required_feature");
        assert_eq!(record.catalog.cause(), "unsupported-feature");
        assert_eq!(
            record.fields.get("reason").map(String::as_str),
            Some("missing-route-neutral-place-identity")
        );
        assert_eq!(record.fields.get("target"), Some(&target));
        assert_eq!(record.fields.get("document"), Some(&identity.identity));
        assert_eq!(
            record.fields.get("path").map(String::as_str),
            Some("/populations/0/objects/0/fields/r")
        );
        assert_eq!(
            record.path,
            vec![
                filament_semantic_value::admission::PathStep::Field("populations".to_owned()),
                filament_semantic_value::admission::PathStep::Index(0),
                filament_semantic_value::admission::PathStep::Field("objects".to_owned()),
                filament_semantic_value::admission::PathStep::Index(0),
                filament_semantic_value::admission::PathStep::Field("fields".to_owned()),
                filament_semantic_value::admission::PathStep::Field("r".to_owned()),
            ]
        );
    }
}

/// A real map declaration never authorizes a numerically forged template;
/// this static-table control makes no RAW admission claim.
#[trace("FR-230", "TC-3084")]
#[test]
fn numeric_selectors_do_not_authorize_a_map_template() {
    use fsl_semantics::model::clock_reference::{self, DeclaredClockPlaces};
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
    let declared = DeclaredClockPlaces::with_declaration_table(
        graph.timed_models(),
        graph.clock_declaration_table(),
    );
    assert!(
        graph.clock_declaration_table().entries().next().is_some(),
        "the actual non-map clock declarations remain present"
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
}
