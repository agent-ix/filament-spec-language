// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-337's timestamped-event role readback and clock-selection boundaries.

use std::collections::BTreeMap;

use filament_exact::Cancel;
use fsl_package::{
    read_import_view, AdmittedPackages, CheckedNodeId, ClockConfiguration, ClockExactNumber,
    ClockSelectionCause, EmitLimits, EmitRefusal, V2ReadLimits,
};
use fsl_replay::spine::{
    check_with_execution_points_and_source_inputs, package_with_temporal_selections, parse, select,
    CompileRefusal, DependencyInput, PackageLimits, ParseRequest, RequestClocks, SourceInputs,
    SpineLimits,
};
use fsl_replay::StateGraphIdentity;
use fsl_semantics::model::intake::package_input;
use ix_trace_rs::trace;

const MODEL_SOURCE: &[u8] = br#"language "ix:native" edition "1-draft";
domain "acme/rpc";
dimension Time;
unit s : Time = rational(1, 1);
object Counter identity (id) { id: Int[0,3]; }
population counters: Counter;
"#;

const SOURCE: &[u8] = br#"language "ix:native" edition "1-draft";
profile v = "filament.model.complete/v1";
profile t = "filament.temporal.timestamped-event.finite-window/v1";
model M = "acme/rpc";
record Reading { serial: Integer; }
temporal Window using t over (r: Reading) clock "plant-ms" on origin {
  always[0,1] holds(true)
}
possible CounterClaim using v over (c: M::Counter) { true }
"#;

fn prepared_unit() -> (
    fsl_replay::spine::ParsedUnitSource,
    fsl_replay::spine::AdmittedModels,
    fsl_semantics::model::intake::PackageInput,
) {
    let cancel = Cancel::new();
    let source = fsl_foundation::SourceIdentity::new("agent-ix", "test:qsl-2094-ac30");
    let parsed = parse(
        &ParseRequest {
            source: &source,
            path: "unit.native",
            bytes: SOURCE,
        },
        fsl_cst::Limits::default(),
        &cancel,
    )
    .expect("S1 and S2 admit the fixture")
    .into_value();
    let model = model_bytes();
    let packages = package_input([model.as_slice()]).expect("the domain package admits");
    let models = select(
        &parsed,
        &packages,
        fsl_semantics::model::accounting::ModelNormalizationLimits::default(),
        &cancel,
    )
    .expect("the selected model admits")
    .into_value();
    (parsed, models, packages)
}

fn model_bytes() -> Vec<u8> {
    let input = package_input([MODEL_SOURCE]).expect("the base domain package compiles");
    let mut document: serde_json::Value =
        serde_json::from_slice(&input["acme/rpc"]).expect("the model package is JSON");
    let mut millisecond = document["units"][0].clone();
    millisecond["identity"] = serde_json::json!("ix://acme/rpc/unit/ms");
    millisecond["name"] = serde_json::json!("ms");
    millisecond["target"] = serde_json::json!("ix://acme/rpc/unit/s");
    millisecond["scale"] = serde_json::json!("1/1000");
    document["units"]
        .as_array_mut()
        .expect("model units are an array")
        .push(millisecond);
    document.to_string().into_bytes()
}

fn checked_with_clocks(
    clocks: &RequestClocks,
) -> (
    fsl_replay::spine::ParsedUnitSource,
    fsl_replay::spine::AdmittedModels,
    fsl_semantics::model::intake::PackageInput,
    fsl_replay::spine::CheckedUnit,
) {
    let cancel = Cancel::new();
    let (parsed, models, packages) = prepared_unit();
    let selections = clocks.check_selections(&parsed);
    let checked = check_with_execution_points_and_source_inputs(
        &parsed,
        "acme/orders".to_owned(),
        &BTreeMap::new(),
        SourceInputs {
            temporal_selections: &selections,
            clocks: Some(clocks),
            ..SourceInputs::default()
        },
        &models,
        &DependencyInput::default(),
        SpineLimits::default(),
        &cancel,
    )
    .expect("S3 and S4 check the timestamped clause")
    .into_value();
    (parsed, models, packages, checked)
}

fn timestamped_clock() -> ClockConfiguration {
    ClockConfiguration::TimestampedEvent {
        timestamp_unit: "ix://acme/rpc/unit/ms".into(),
    }
}

/// TC-1326 / FR-337-AC-30: an explicitly selected timestamp role and unit
/// survive package readback and identify the same state-graph claim; an
/// unused role and a fixed-sample configuration are refused at their loci.
#[trace("TC-1326", "FR-337-AC-30")]
#[test]
fn timestamped_role_readback_identity_and_selection_refusals_are_exact() {
    let clocks = RequestClocks::new(BTreeMap::from([(
        "plant-ms".to_owned(),
        timestamped_clock(),
    )]))
    .expect("the explicit timestamp role is well formed");
    let (parsed, _models, packages, checked) = checked_with_clocks(&clocks);
    let selections = clocks
        .package_selections(&checked, &[&parsed])
        .expect("the explicit role is used by the temporal clause");
    assert_eq!(selections.len(), 1);
    let emitted = package_with_temporal_selections(
        &checked,
        &selections,
        PackageLimits::default(),
        &Cancel::new(),
    )
    .expect("whole package emission admits the selected clock")
    .into_value();
    let bytes = emitted.package().bytes();
    let wire: serde_json::Value = serde_json::from_slice(bytes).expect("the package is JSON");
    let bindings = wire["temporal_bindings"]
        .as_array()
        .expect("the emitted package carries temporal bindings");
    assert_eq!(bindings.len(), 1);
    assert_eq!(bindings[0]["clock_name"], "plant-ms");
    assert_eq!(bindings[0]["clock"]["kind"], "timestamped_event");
    assert_eq!(
        bindings[0]["clock"]["timestamp_unit"],
        "ix://acme/rpc/unit/ms"
    );
    let library = fsl_semantics::library::LibraryName::new(emitted.package().name())
        .expect("the emitted package identity is a library name");
    let mut admitted = AdmittedPackages::default();
    let readback = read_import_view(
        checked.package(),
        emitted.emission(),
        library.clone(),
        &packages,
        &mut admitted,
        V2ReadLimits::bounded(0),
        EmitLimits::new(1 << 32),
    );
    let target = &wire["semantic_graph"]["nodes"][7]["semantic_type"];
    let target_nodes: Vec<_> = wire["semantic_graph"]["nodes"]
        .as_array()
        .expect("the emitted semantic graph has nodes")
        .iter()
        .filter(|node| &node["node_id"] == target)
        .collect();
    assert!(
        readback.is_ok(),
        "whole-package I2 readback: {readback:?}; relevant node: {}; type target: {target}; matching nodes: {target_nodes:?}",
        wire["semantic_graph"]["nodes"][7],
    );
    let read = admitted
        .package(&library)
        .expect("the read closure retains the admitted package");
    let clause_id = serde_json::from_value::<CheckedNodeId>(
        wire["temporal_clause_names"][0]["declaration"].clone(),
    )
    .expect("the temporal name row identifies its clause");
    let subject = fsl_package::checked_views::temporal_subject(read, &clause_id)
        .expect("the temporal formula reader resolves the clause");
    let binding = subject
        .clock()
        .expect("the formula view has its binding row");
    assert_eq!(binding.clock_name.as_ref(), "plant-ms");
    assert_eq!(binding.clock, timestamped_clock());

    let [claim] = checked.package().graph().state_graph_claims() else {
        panic!("one state-graph claim is checked");
    };
    let source_occurrence = claim
        .occurrence()
        .expect("the checked claim has an occurrence");
    let identity = StateGraphIdentity::resolve(
        checked.package(),
        &packages,
        claim,
        &selections,
        &Cancel::new(),
        EmitLimits::new(1 << 32),
    )
    .expect("the selected package resolves the state-graph identity");
    assert_eq!(identity.occurrence(), source_occurrence);
    assert!(
        identity.over().is_some(),
        "the identity retains its over node"
    );

    let with_unused = RequestClocks::new(BTreeMap::from([
        ("plant-ms".to_owned(), timestamped_clock()),
        (
            "unwritten-role".to_owned(),
            ClockConfiguration::TimestampedEvent {
                timestamp_unit: "ix://acme/rpc/unit/ms".into(),
            },
        ),
    ]))
    .expect("both timestamp configurations are well formed");
    let unused_refusal = with_unused
        .package_selections(&checked, &[&parsed])
        .expect_err("a role no declaration writes is refused");
    let CompileRefusal::Emit(EmitRefusal::ClockSelection(refusal), locus) = *unused_refusal else {
        panic!("typed unused-role clock refusal: {unused_refusal:?}");
    };
    assert_eq!(refusal.cause, ClockSelectionCause::UnusedClockSelection);
    assert_eq!(refusal.clock, "unwritten-role");
    assert_eq!(refusal.declaration, None);
    assert_eq!(refusal.supplied, None);
    assert_eq!(refusal.unit, None);
    assert_eq!(locus, None, "an unused request member has no source locus");

    let fixed = ClockConfiguration::FixedSample {
        epoch: ClockExactNumber::Integer {
            decimal: "0".into(),
        },
        period: ClockExactNumber::Integer {
            decimal: "1".into(),
        },
        unit: "ix://acme/rpc/unit/ms".into(),
    };
    let wrong_kind = RequestClocks::new(BTreeMap::from([("plant-ms".to_owned(), fixed)]))
        .expect("the fixed-sample member is itself well formed");
    let (wrong_parsed, wrong_models, _) = prepared_unit();
    let wrong_selection = wrong_kind.check_selections(&wrong_parsed);
    assert!(
        wrong_selection.is_empty(),
        "fixed-sample is not a timestamp selection"
    );
    let wrong_checked = check_with_execution_points_and_source_inputs(
        &wrong_parsed,
        "acme/orders".to_owned(),
        &BTreeMap::new(),
        SourceInputs {
            temporal_selections: &wrong_selection,
            clocks: Some(&wrong_kind),
            ..SourceInputs::default()
        },
        &wrong_models,
        &DependencyInput::default(),
        SpineLimits::default(),
        &Cancel::new(),
    )
    .expect("the source unit checks before clock-selection emission")
    .into_value();
    let wrong_refusal = wrong_kind
        .package_selections(&wrong_checked, &[&wrong_parsed])
        .expect_err("fixed-sample cannot serve timestamped-event");
    let CompileRefusal::Emit(EmitRefusal::ClockSelection(refusal), Some(locus)) = *wrong_refusal
    else {
        panic!("typed wrong-kind refusal with declaration locus: {wrong_refusal:?}");
    };
    assert_eq!(refusal.cause, ClockSelectionCause::ClockKindMismatch);
    assert_eq!(refusal.clock, "plant-ms");
    assert_eq!(refusal.supplied, Some("fixed_sample"));
    assert_eq!(refusal.unit, None);
    assert_eq!(
        refusal.declaration.as_ref().map(|row| row.name.as_str()),
        Some("Window")
    );
    assert!(
        locus.start() < locus.end(),
        "the refusal points at the clock role"
    );
    assert!(locus.source().identity().contains("qsl-2094-ac30"));
}
