// SPDX-License-Identifier: AGPL-3.0-or-later
//! Simulation uses the same engine, results and accounting from package bytes.

use fsl_foundation::source::SourceLabel;
use ix_trace_rs::trace;
use serde_json::{json, Value};

use crate::analyze_cli_support::{
    analyze, bounded, counter_request_with, label, package_selection_named, request_members,
    Workspace, HANDOFF,
};

#[trace("TC-1594", "FR-634-AC-1", "FR-850-AC-2", "FR-940-AC-6")]
#[test]
fn simulation_package_preserves_whole_source_result_and_exit() {
    for (case, settings, limits, expected) in [
        (
            "enumerate",
            json!({"mode": "enumerate"}),
            json!({}),
            "success",
        ),
        (
            "sample",
            json!({"mode": "sample", "seed": 5, "samples": 3}),
            json!({}),
            "success",
        ),
        (
            "states",
            json!({"mode": "enumerate", "max_states": 2}),
            json!({}),
            "incomplete",
        ),
        (
            "steps",
            json!({"mode": "enumerate", "max_steps": 2}),
            json!({}),
            "incomplete",
        ),
        (
            "enumerate-budget",
            json!({"mode": "enumerate"}),
            json!({"accounting": {"result_units": 1000}}),
            "incomplete",
        ),
        (
            "sample-budget",
            json!({"mode": "sample", "seed": 5, "samples": 1000}),
            json!({"accounting": {"result_units": 1000}}),
            "incomplete",
        ),
    ] {
        let ws = Workspace::new(&format!("package-simulation-parity-{case}"));
        let simulation = json!({"kind": "simulation", "simulation": settings});
        let (items, index) = if case.ends_with("budget") {
            (json!([simulation]), 0)
        } else {
            (json!([{"claim": "Bounded"}, simulation]), 1)
        };
        let mut members = request_members(Some(items));
        members["limits"] = limits;
        let path = counter_request_with(&ws, &bounded(), members);
        let source = analyze(&path);
        if index == 1 {
            assert_eq!(
                label(&source.item(0)),
                "proved",
                "{case}: {}",
                source.document()
            );
        }
        let item = source.item(index);
        assert_eq!(item["category"], expected, "{case}: {item}");
        if case == "enumerate" {
            assert_eq!(item["coverage"]["states"], "4", "{item}");
            assert_eq!(item["coverage"]["transitions"], "3", "{item}");
            assert_eq!(item["traces"].as_array().map(Vec::len), Some(1), "{item}");
        }
        assert!(item.get("record").is_none(), "{item}");
        let input = fsl_command::command::prepare_analyze(
            &path,
            fsl_command::command::RunLimits::default(),
        )
        .expect("source request admits")
        .into_input();
        let request: Value =
            serde_json::from_slice(&std::fs::read(&path).expect("request")).expect("request JSON");
        let (members, bytes) = package_selection_named(
            &ws,
            &request["request"],
            input
                .source
                .source()
                .identity()
                .label(SourceLabel::Identity),
        );
        assert_exploration_charge_parity(&ws, &input, &bytes);
        drop(input);
        std::fs::remove_file(ws.path("program.fsl")).expect("delete source program");
        std::fs::remove_dir_all(ws.path(&format!("{HANDOFF}/sources")))
            .expect("delete retained source copies");
        assert!(!ws.path("program.fsl").exists());
        assert!(!ws.path(&format!("{HANDOFF}/sources")).exists());
        let package = analyze(
            &ws.write(
                "package-analyze.json",
                json!({"format": "native-analyze/1", "request": members})
                    .to_string()
                    .as_bytes(),
            ),
        );
        assert_eq!(package.exit, source.exit, "{case}: {}", package.document());
        assert_eq!(
            package.stdout, source.stdout,
            "{case}: whole serialized result"
        );
    }
}

/// Compare every admitted charge at the actual exploration/sampling seam.
/// Model admission and document emission own separate meters; their finite
/// result-budget behavior is exercised by the whole command cases above.
fn assert_exploration_charge_parity(
    ws: &Workspace,
    input: &fsl_command::command::AnalyzeInput,
    bytes: &[u8],
) {
    use filament_exact::Meter;
    use fsl_eval::simulation::model::{
        enumerate_model, sample_model_runs, ExpansionLimits, ModelCheckInput,
        ModelSimulationRequest, ModelSystem, PackageModelInput, SampledRuns,
    };
    use fsl_eval::simulation::{EnumerationBounds, BOUND_POSITIONS};
    use fsl_package::{BarePackage, BareSupply, ReadEqualityRows, ReadIdentityRows};

    let compiled =
        fsl_command::checked_handoff::compile(&ws.path(HANDOFF)).expect("actual handoff compiles");
    let limits = fsl_package::V2ReadLimits::bounded(1 << 24);
    let admitted = fsl_package::admit_checked_package(bytes, compiled.packages(), limits)
        .expect("actual producer bytes admit through the strict reader");
    let mut budget = limits.native_join_limits().budget();
    let identities = ReadIdentityRows::read(
        &std::fs::read(ws.path(&format!("{HANDOFF}/identities.json"))).expect("identities"),
        &admitted,
        &mut budget,
    )
    .expect("identity rows admit");
    let equality = ReadEqualityRows::read(
        &std::fs::read(ws.path(&format!("{HANDOFF}/equality.json"))).expect("equality"),
        &admitted,
    )
    .expect("equality rows admit");
    let bare = BarePackage::new(&admitted)
        .with_supply(
            BareSupply {
                identities: &identities,
                equality: Some(&equality),
            },
            &mut budget,
        )
        .expect("actual supply joins");
    let package_input =
        PackageModelInput::of(&bare, compiled.packages(), input.limits.normalization)
            .expect("package model input");
    let binding = input.subject.binding(compiled.packages(), &input.documents);
    let request = |model| ModelSimulationRequest {
        input: model,
        packages: binding.packages,
        normalization: input.limits.normalization,
        snapshots: binding.snapshots,
        initial: binding.initial.clone(),
        universes: binding.universes.clone(),
        observation: input.limits.observation,
        expansion: ExpansionLimits {
            max_candidates: u64::MAX,
        },
        meter: input.limits.accounting,
        emit: input.limits.emit,
    };
    let source = ModelSystem::new(request(ModelCheckInput::source(compiled.checked())))
        .expect("source system");
    let package = ModelSystem::new(request(package_input.input().clone())).expect("package system");
    for sample in [false, true] {
        let mut source_meter = Meter::new(input.limits.accounting);
        let mut package_meter = Meter::new(input.limits.accounting);
        if sample {
            let sampler = fsl_foundation::selection::DefinitionRef::new(
                "filament",
                "filament.simulation.sampler/v1",
            )
            .expect("sampler identity");
            let runs = SampledRuns {
                seed: 5,
                samples: 3,
                max_steps: 8,
            };
            let expected = sample_model_runs(
                &source,
                BOUND_POSITIONS,
                &sampler,
                runs,
                &mut source_meter,
                || false,
            )
            .expect("source sampling");
            let actual = sample_model_runs(
                &package,
                BOUND_POSITIONS,
                &sampler,
                runs,
                &mut package_meter,
                || false,
            )
            .expect("package sampling");
            assert_eq!(actual.traces, expected.traces);
            assert_eq!(actual.coverage, expected.coverage);
            assert_eq!(actual.stopped, expected.stopped);
        } else {
            let bounds = EnumerationBounds {
                max_states: 16,
                max_steps: 8,
                max_state_key_bytes: 1 << 20,
            };
            let expected =
                enumerate_model(&source, BOUND_POSITIONS, bounds, &mut source_meter, || {
                    false
                })
                .expect("source enumeration");
            let actual = enumerate_model(
                &package,
                BOUND_POSITIONS,
                bounds,
                &mut package_meter,
                || false,
            )
            .expect("package enumeration");
            assert_eq!(actual, expected);
        }
        assert!(!source_meter.charge_log_truncated());
        assert!(!package_meter.charge_log_truncated());
        assert!(!source_meter.admitted_charges().is_empty());
        assert_eq!(
            package_meter.admitted_charges(),
            source_meter.admitted_charges()
        );
    }
}
