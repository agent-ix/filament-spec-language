// SPDX-License-Identifier: AGPL-3.0-or-later
//! Independently compiled facade consumers. These tests deliberately name
//! only shipped public APIs and enable no facade-specific feature.

use ix_trace_rs::trace;

const UNIT: &str = "language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.value.complete/v1\";\nfunction f using v(x: Int[0, 9]): Integer pure { x + 5 }\n";

mod code_generation {
    use super::*;

    #[test]
    #[trace("TC-2582", "FR-913-AC-1")]
    fn cf_fr_913_01_format_emit_graph_and_reader_verdict_agree() {
        let source = fsl_foundation::SourceIdentity::new("consumer", "format-readback");
        let parsed = fsl_cst::parse(
            source.clone(),
            "unit.native",
            UNIT.as_bytes(),
            fsl_cst::Limits::default(),
        )
        .expect("unit parses");
        let formatted = fsl_cst::format::format(&parsed).expect("unit formats");
        let emit = |text: &str| {
            fsl_replay::compile_package(
                fsl_replay::CompileUnit {
                    source: source.clone(),
                    package_name: "consumer".into(),
                    path: "unit.native",
                    bytes: text.as_bytes(),
                },
                [],
                &fsl_replay::DependencyInput::default(),
                fsl_replay::StageLimits::default(),
                fsl_replay::ReplayLimits::default(),
            )
            .expect("unit compiles")
            .bytes()
            .to_vec()
        };
        let before = emit(UNIT);
        let after = emit(&formatted);
        let graph = |bytes: &[u8]| {
            let mut value: serde_json::Value = serde_json::from_slice(bytes).unwrap();
            fn strip(value: &mut serde_json::Value) {
                match value {
                    serde_json::Value::Object(m) => {
                        m.remove("region");
                        m.remove("regions");
                        m.remove("span");
                        for child in m.values_mut() {
                            strip(child);
                        }
                    }
                    serde_json::Value::Array(a) => {
                        for child in a {
                            strip(child);
                        }
                    }
                    _ => {}
                }
            }
            strip(&mut value["semantic_graph"]);
            value["semantic_graph"].clone()
        };
        let verdict = |bytes: &[u8]| {
            let evidence = filament_contract_model::CheckedPackageEvidence::new();
            matches!(
                filament_contract_model::read_checked_package(
                    bytes,
                    filament_contract_model::CheckedPackageReadLimits::bounded(),
                    &evidence,
                ),
                filament_contract_model::CheckedPackageDispatchResult::AdmittedV2(_)
            )
        };
        assert_eq!(graph(&before), graph(&after));
        assert_eq!(verdict(&before), verdict(&after));
        assert!(verdict(&before));
    }

    #[test]
    #[trace("TC-2583", "FR-911-AC-1")]
    fn cf_cg_01_01_call_site_returns_the_selected_function_and_package() {
        let source = fsl_replay::SourceIdentity::new("consumer", "call-site");
        let selected = fsl_replay::QualifiedName::new(vec![
            filament_exact::Identifier::new("f").expect("identifier")
        ])
        .expect("qualified name");
        let site = fsl_replay::call_site(
            source,
            "consumer".into(),
            "unit.native",
            UNIT.as_bytes(),
            [],
            &fsl_replay::DependencyInput::default(),
            &selected,
        )
        .expect("public facade locates function");
        assert_eq!(site.site.parameters.len(), 1);
        assert_eq!(site.site.parameters[0].0.as_str(), "x");
        assert!(!format!("{:?}", site.site.obligation_identity()).is_empty());
        assert!(!site.package.is_empty());
    }
}

mod ir {
    use super::*;

    #[test]
    #[trace("TC-2584", "FR-911-AC-1")]
    fn cf_ir_01_01_strict_v2_reader_admits_emitted_package() {
        let source = fsl_replay::SourceIdentity::new("consumer", "strict-reader");
        let emitted = fsl_replay::compile_package(
            fsl_replay::CompileUnit {
                source,
                package_name: "consumer".into(),
                path: "unit.native",
                bytes: UNIT.as_bytes(),
            },
            [],
            &fsl_replay::DependencyInput::default(),
            fsl_replay::StageLimits::default(),
            fsl_replay::ReplayLimits::default(),
        )
        .expect("unit compiles");
        let evidence = filament_contract_model::CheckedPackageEvidence::new();
        let result = filament_contract_model::read_checked_package(
            emitted.bytes(),
            filament_contract_model::CheckedPackageReadLimits::bounded(),
            &evidence,
        );
        assert!(matches!(
            result,
            filament_contract_model::CheckedPackageDispatchResult::AdmittedV2(_)
        ));
    }
}

mod driver {
    use super::*;

    #[test]
    #[trace("TC-1340", "FR-535-AC-1")]
    fn cf_dr_02_01_driver_writer_and_bounded_reader_round_trip() {
        use fsl_replay::{ClauseDisposition, ClauseRunReport, NativeRunLimits, NativeRunResult};
        let report = ClauseRunReport {
            disposition: ClauseDisposition::MissingName {
                name: "Absent".into(),
            },
            usage: Default::default(),
            basis: fsl_replay::SettlementBasis::Unavailable,
            witness: None,
            frame_field_values: None,
            state_clause_counterexample: None,
            consequent_source_events: Default::default(),
            finite_window: None,
            counterexample: None,
            decisive: None,
            authenticated_temporal_selection: None,
            refused_at: None,
            ledger: None,
        };
        let bytes = fsl_replay::write_clause_run_result(&report).expect("writer accepts report");
        let read = fsl_replay::read_native_run_result_bounded(&bytes, NativeRunLimits::UNBOUNDED)
            .expect("reader accepts writer output");
        let NativeRunResult::Clause(clause) = read else {
            panic!("expected clause result")
        };
        assert_eq!(clause.code(), Some("missing_declaration"));
    }

    #[test]
    #[trace("TC-1340", "FR-535-AC-2")]
    fn cf_dr_01_01_driver_reader_enforces_the_input_bound() {
        let error = fsl_replay::read_native_run_result_bounded(
            br#"{"format":"native-run-result/2"}"#,
            fsl_replay::NativeRunLimits {
                max_input_bytes: 1,
                max_working_bytes: u64::MAX,
            },
        )
        .expect_err("input exceeds caller bound");
        assert!(matches!(error, fsl_replay::NativeRunReadError::Read(_)));
    }
}
