// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-1051 mutation registration for package-byte model-trace replay.

use super::*;
use filament_exact::Cancel;
use filament_semantic_value::model_transition::ModelTransition;
use fsl_analyze::analyze::{
    AnalyzeLimits, PackageDeadlockSubject, RunInput, TemporalItem, deadlock_model_item,
};
use fsl_certificate_mutation::{Fixture, Verdict, check_document};
use fsl_replay::{CounterexampleKind, ModelCheckOutcome, ModelTraceSettlement, ModelTraceValue};
use fsl_route::request::{DerivedLimits, ItemKind};
use ix_trace_rs::trace;
use serde_json::Value;

/// Trace: FR-1051-AC-11, FR-1051-AC-13.
#[trace("FR-1051-AC-11", "FR-1051-AC-13")]
#[test]
fn counterexample_replay_from_package_bytes_rejects_single_mutations() {
    let world = counter_with(CONTRACT);
    let written = crate::derived_request_support::written(
        &world.subject(&["c"], UNLIMITED),
        &DerivedLimits::default(),
    );
    let item = &written.items()[1];
    assert_eq!(item.item_kind(), Some(ItemKind::DeadlockFreedom));
    let occurrence = item.occurrence();
    let form = TemporalItem {
        item_kind: ItemKind::DeadlockFreedom,
        ..TemporalItem::default()
    };
    let binding = world.binding(&["c"]);
    let source_item = deadlock_model_item(
        &RunInput::source(&world.package),
        &binding,
        AnalyzeLimits::unbounded(),
        &world.provision,
        occurrence,
        &form,
    )
    .expect("the derived target is formed");
    let outcome = fsl_analyze::model_check::check_model(source_item.request, &Cancel::new())
        .expect("the source model check runs");
    let ModelCheckOutcome::Violated { counterexample, .. } = outcome else {
        panic!("the fixture has a deadlock counterexample: {outcome:?}");
    };
    let document = fsl_replay::temporal_wire(&counterexample, None, None)
        .expect("the model counterexample has a byte-route spelling");

    let handoff = world.handoff();
    let bare = handoff.bare();
    let input = PackageModelInput::of(&bare, binding.packages, ModelNormalizationLimits::default())
        .expect("the package route reads every form");
    let package_item = || {
        PackageDeadlockSubject::new(
            &input,
            &handoff.bytes,
            handoff.supply(),
            world.binding(&["c"]),
            AnalyzeLimits::unbounded(),
            occurrence,
            &form,
        )
        .model_item()
    };
    let mut parseable_document = document.clone();
    parseable_document["kind"] = "formula".into();
    set_trace_position(&mut parseable_document);
    let mut baseline_payload =
        fsl_replay::read_temporal_counterexample::<ModelTransition>(&parseable_document)
            .expect("the produced deadlock counterexample uses the byte-route schema");
    baseline_payload.kind = CounterexampleKind::Deadlock;
    let baseline = (package_item().replay)(&baseline_payload);
    assert!(
        matches!(
            baseline,
            Ok(ref replayed)
                if matches!(
                    &replayed.settlement,
                    ModelTraceSettlement::Reproduced {
                        value: ModelTraceValue::Truth(false),
                        ..
                    }
                )
        ),
        "baseline package replay: {baseline:?}"
    );

    let survivors = check_document(
        Fixture {
            route: "R-50 model-trace replay byte route",
            name: "counter-deadlock",
            document,
            allowed_to_survive: Vec::new(),
        },
        |bytes| {
            let Ok(value) = serde_json::from_slice::<Value>(bytes) else {
                return Verdict::Rejected;
            };
            let mut parseable = value;
            parseable["kind"] = "formula".into();
            set_trace_position(&mut parseable);
            let Ok(mut payload) =
                fsl_replay::read_temporal_counterexample::<ModelTransition>(&parseable)
            else {
                return Verdict::Rejected;
            };
            payload.kind = CounterexampleKind::Deadlock;
            match (package_item().replay)(&payload) {
                Ok(replayed)
                    if matches!(
                        replayed.settlement,
                        ModelTraceSettlement::Reproduced {
                            value: ModelTraceValue::Truth(false),
                            ..
                        }
                    ) =>
                {
                    Verdict::Accepted
                }
                _ => Verdict::Rejected,
            }
        },
    )
    .expect("all unlisted replay mutations fail to reproduce");
    assert!(survivors > 0, "the replay document has generated mutations");
}

fn set_trace_position(document: &mut Value) {
    let position = document["prefix"]
        .as_array()
        .expect("model trace prefix")
        .len();
    document["trace_position"] = Value::from(position);
}
