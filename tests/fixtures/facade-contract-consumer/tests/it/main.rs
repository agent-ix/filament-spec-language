// SPDX-License-Identifier: AGPL-3.0-or-later
//! Independently compiled facade consumers. These tests deliberately name
//! only shipped public APIs and enable no facade-specific feature.

use ix_trace_rs::trace;

const UNIT: &str = "language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.value.complete/v1\";\nfunction f using v(x: Int[0, 9]): Integer pure { x + 5 }\n";

fn evidence_for(
    bytes: &[u8],
    domain: Option<(&str, &[u8])>,
) -> filament_contract_model::CheckedPackageEvidence {
    let mut evidence = filament_contract_model::CheckedPackageEvidence::new();
    let package: serde_json::Value = serde_json::from_slice(bytes).expect("emitted package JSON");
    for feature in package["lock"]["required_features"]
        .as_array()
        .into_iter()
        .flatten()
    {
        evidence.support_feature(feature.as_str().expect("feature name").to_owned());
    }
    if let Some((identity, document)) = domain {
        evidence
            .insert_domain_package_document(identity, document.to_vec().into_boxed_slice())
            .expect("strict reader evidence key");
    }
    evidence
}

mod code_generation {
    use super::*;

    const LIBRARY: &str = "language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.value.complete/v1\";\nfunction increment using v(x: Integer): Integer pure { x + 1 }\n";

    fn model_with_touch() -> Vec<u8> {
        let mut model: serde_json::Value =
            serde_json::from_slice(include_bytes!("../../../spine-model.semantic-ir.json"))
                .expect("model fixture JSON");
        let operation = "ix://acme/orders/Widget/touch";
        let origin = serde_json::json!({"generated": {
            "generatorIdentity": operation, "generatorVersion": "1.0.0",
            "inputIdentities": [operation],
        }});
        model["types"]
            .as_array_mut()
            .expect("types")
            .iter_mut()
            .find(|ty| ty["identity"] == "ix://acme/orders/Widget")
            .expect("Widget")["operations"] = serde_json::json!([{
            "identity": operation, "name": "touch", "params": [], "pre": [], "post": [],
            "origin": origin, "frame": {"modifies": [], "creates": [], "deletes": []},
        }]);
        serde_json::to_vec(&model).expect("fixture serializes")
    }

    #[test]
    #[trace("TC-2582", "FR-913-AC-1")]
    fn fr_913_01_format_emit_graph_and_reader_verdict_agree() {
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
            let evidence = evidence_for(bytes, None);
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

    #[test]
    #[trace("TC-2583", "FR-911-AC-1")]
    fn cf_cg_02_01_imported_function_site_exposes_library_package_and_function() {
        let dependencies = fsl_replay::DependencyInput::new([fsl_replay::SuppliedLibrary {
            identity: "test/math".into(),
            source: fsl_replay::SourceIdentity::new("consumer", "math-library"),
            path: "math.native".into(),
            bytes: LIBRARY.as_bytes().to_vec(),
        }])
        .expect("dependency input");
        let unit = "language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.value.complete/v1\";\nimport \"test/math\" as Math;\nfunction use using v(x: Integer): Integer pure { Math::increment(x) }\n";
        let selected = fsl_replay::ImportedFunctionName {
            library: fsl_replay::LibraryName::new("test/math").expect("library name"),
            function: filament_exact::Identifier::new("increment").expect("function name"),
        };
        let site = fsl_replay::call_site(
            fsl_replay::SourceIdentity::new("consumer", "imported-call-site"),
            "consumer".into(),
            "unit.native",
            unit.as_bytes(),
            [],
            &dependencies,
            &selected,
        )
        .expect("public facade locates imported function");
        assert_eq!(site.site.library.as_str(), "test/math");
        assert!(!site.site.package.is_empty());
        assert_eq!(site.site.function.parameters[0].0.as_str(), "x");
    }

    #[test]
    #[trace("TC-2583", "FR-911-AC-1")]
    fn cf_cg_03_01_operation_site_exposes_its_obligation_identity() {
        let function_selection = fsl_replay::QualifiedName::new(vec![
            filament_exact::Identifier::new("f").expect("function name"),
        ])
        .expect("qualified function name");
        let function = fsl_replay::call_site(
            fsl_replay::SourceIdentity::new("consumer", "function-obligation"),
            "consumer".into(),
            "unit.native",
            UNIT.as_bytes(),
            [],
            &fsl_replay::DependencyInput::default(),
            &function_selection,
        )
        .expect("public facade locates function");
        assert!(!format!("{:?}", function.site.obligation_identity()).is_empty());

        let model = model_with_touch();
        let selected = fsl_replay::spine::OperationName {
            model: filament_exact::Identifier::new("M").expect("model alias"),
            object: filament_exact::Identifier::new("Widget").expect("object type"),
            operation: filament_exact::Identifier::new("touch").expect("operation"),
        };
        let unit = "language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.value.complete/v1\";\nmodel M = \"acme/orders\";\nfunction f using v(): Boolean pure { true }\n";
        let site = fsl_replay::call_site(
            fsl_replay::SourceIdentity::new("consumer", "operation-call-site"),
            "consumer".into(),
            "unit.native",
            unit.as_bytes(),
            [&model[..]],
            &fsl_replay::DependencyInput::default(),
            &selected,
        )
        .expect("public facade locates operation");
        assert_eq!(site.site.frame_operation.to_string(), "M::Widget::touch");
        assert!(!format!("{:?}", site.site.obligation_identity()).is_empty());
    }
}

mod ir {
    use super::*;

    #[test]
    #[trace("TC-2584", "FR-911-AC-1")]
    fn cf_ir_01_01_strict_v2_reader_admits_emitted_package() {
        assert_eq!(
            fsl_package::CHECKED_PACKAGE_V2,
            "filament.checked-package/v2"
        );
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
        let evidence = evidence_for(emitted.bytes(), None);
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

    #[test]
    #[trace("TC-1202", "FR-911-AC-1")]
    fn cf_ir_03_01_schedulability_root_exposes_occurrence_identity() {
        let unit = "language \"ix:native\" edition \"1-draft\";\n\
profile t = \"filament.temporal.complete/v1\";\n\
dimension Time;\n\
unit ms : Time = rational(1, 1);\n\
taskset Ctl { task a periodic 4 ms deadline 4 ms wcet 1 ms priority 1 lo; }\n\
schedulable CtlFp on Ctl under fixed_priority;\n";
        let emitted = fsl_replay::compile_package(
            fsl_replay::CompileUnit {
                source: fsl_replay::SourceIdentity::new("consumer", "schedulability-root"),
                package_name: "consumer".into(),
                path: "unit.native",
                bytes: unit.as_bytes(),
            },
            [],
            &fsl_replay::DependencyInput::default(),
            fsl_replay::StageLimits::default(),
            fsl_replay::ReplayLimits::default(),
        )
        .expect("schedulability unit compiles");
        let evidence = evidence_for(emitted.bytes(), None);
        let package = match filament_contract_model::read_checked_package(
            emitted.bytes(),
            filament_contract_model::CheckedPackageReadLimits::bounded(),
            &evidence,
        ) {
            filament_contract_model::CheckedPackageDispatchResult::AdmittedV2(package) => *package,
            other => panic!("strict reader refused schedulability package: {other:?}"),
        };
        let roots = fsl_package::schedulability_roots(&package).expect("schedulability roots");
        let [root] = roots.as_slice() else {
            panic!("one schedulability root: {roots:?}")
        };
        assert_eq!(root.occurrence().origin().role().as_str(), "claim");
        assert_eq!(root.occurrence().origin().ordinal(), 0);
        assert!(!format!("{:?}", root.occurrence()).is_empty());
    }

    #[test]
    #[trace("TC-2587", "FR-911-AC-1")]
    fn cf_ir_02_01_import_view_exposes_checked_exports() {
        use filament_semantic_value::checking::{CheckMode, CheckingLimits};
        use fsl_semantics::check::PackageDeclarations;

        let source_text = "language \"ix:native\" edition \"1-draft\";\n\
profile v = \"filament.value.complete/v1\";\n\
function increment using v(x: Integer): Integer pure { x + 1 }\n";
        let source = fsl_foundation::SourceIdentity::new("consumer", "exported-library");
        let parsed = fsl_cst::parse(
            source,
            "library.native",
            source_text.as_bytes(),
            fsl_cst::Limits::default(),
        )
        .expect("library parses");
        let raw_source = parsed.source().reference().clone();
        let built = fsl_forms::build_unit(&parsed).expect("library forms build");
        let models = fsl_semantics::model::intake::PackageInput::new();
        let selected = fsl_semantics::model::intake::admit_unit(
            &built.selections().models,
            &models,
            fsl_semantics::model::accounting::ModelNormalizationLimits::default(),
        )
        .expect("empty model selection admits");
        let declarations = PackageDeclarations::assemble(
            raw_source,
            "test/math".into(),
            built,
            selected,
            Vec::new(),
        )
        .expect("library assembles");
        let package = fsl_package::CheckedPackage::link(
            declarations
                .check(CheckMode::Linked, CheckingLimits::default())
                .expect("library checks"),
        );
        let emission = fsl_package::emit_checked(&package, fsl_package::EmitLimits::new(1 << 20))
            .expect("library emits");
        let identity = fsl_replay::LibraryName::new("test/math").expect("library name");
        let view = fsl_package::read_import_view(
            &package,
            &emission,
            identity,
            &models,
            &mut fsl_package::AdmittedPackages::default(),
            fsl_package::V2ReadLimits::bounded(1 << 20),
            fsl_package::EmitLimits::new(1 << 20),
        )
        .expect("import view admits");
        assert!(view.exports().any(|(name, _)| name == "increment"));
    }
}

mod protocol {
    use super::*;

    const UNIT: &str = "language \"ix:native\" edition \"1-draft\";\n\
profile p = \"filament.protocol.finite-global/v1\";\n\
profile v = \"filament.value.complete/v1\";\n\
profile t = \"filament.temporal.event-position.false-extension/v1\";\n\
model M = \"acme/orders\";\n\
protocol Flow using p over (input: M::Widget) on origin {\n\
  role Buyer on M::Widget;\n\
  role Seller on M::Widget;\n\
  channel Ch from Buyer to Seller carries Boolean ordering unordered delivery at-least-once capacity 4 overflow block;\n\
  compensate Full for Main::Applied as (fwd: M::Widget) by Buyer on M::Widget::touch using t clock \"wall\" {\n\
    activate first (trig: M::Widget) when { true } {}\n\
    within [0, 10]; attempts 2 of M::Widget;\n\
    retry (early: M::Widget, late: M::Widget) { false };\n\
    commit never; recover (rec: M::Widget) { true };\n\
  }\n\
  run sequence Main {\n\
    attempt Update by Buyer on M::Widget::touch contracts [] as (tried: M::Widget) { true };\n\
    effect Applied of Update as (applied: M::Widget) { true };\n\
    event Done by Buyer as (done: Boolean) { true };\n\
    await Wait after Done using t clock \"wall\" within [0, 1] match event Seen by Seller as (seen: Boolean) { true }; then sequence Then {} timeout sequence Late {}\n\
    choice Pick by Seller visible () { case yes when { true } sequence Yes {} case no when { false } sequence No {} }\n\
  }\n\
  finish End as (outcome: Boolean) { true };\n\
}\n";

    fn checked_protocol() -> filament_contract_model::CheckedPackageV2 {
        let source = fsl_replay::SourceIdentity::new("consumer", "protocol-contract");
        let mut model: serde_json::Value =
            serde_json::from_slice(include_bytes!("../../../spine-model.semantic-ir.json"))
                .expect("model fixture JSON");
        let operation = "ix://acme/orders/Widget/touch";
        let origin = serde_json::json!({"generated": {
            "generatorIdentity": operation, "generatorVersion": "1.0.0",
            "inputIdentities": [operation],
        }});
        model["types"]
            .as_array_mut()
            .expect("types")
            .iter_mut()
            .find(|ty| ty["identity"] == "ix://acme/orders/Widget")
            .expect("Widget")["operations"] = serde_json::json!([{
            "identity": operation, "name": "touch", "params": [], "pre": [], "post": [],
            "origin": origin, "frame": {"modifies": [], "creates": [], "deletes": []},
        }]);
        let model = serde_json::to_vec(&model).expect("fixture serializes");
        let await_start = UNIT.find("await Wait").expect("await");
        let await_end = UNIT[await_start..]
            .find("timeout sequence Late {}")
            .map(|end| await_start + end + "timeout sequence Late {}".len())
            .expect("await end");
        let compensate_start = UNIT.find("compensate Full").expect("compensation");
        let compensate_end = UNIT[compensate_start..]
            .find("\nrun sequence Main")
            .map(|end| compensate_start + end)
            .expect("compensation end");
        let source_ref =
            fsl_foundation::source::provenance::RawSourceRef::new("consumer", "protocol-contract")
                .expect("source reference");
        let clock = filament_contract_model::ClockConfiguration::EventPosition {
            sequence_authority: "protocol-events".into(),
        };
        let selections = [
            fsl_package::TemporalClockSelection::new(
                source_ref.clone(),
                fsl_foundation::Span {
                    start: await_start,
                    end: await_end,
                },
                clock.clone(),
            ),
            fsl_package::TemporalClockSelection::new(
                source_ref,
                fsl_foundation::Span {
                    start: compensate_start,
                    end: compensate_end,
                },
                clock,
            ),
        ];
        let emitted = fsl_replay::compile_package_with_temporal_selections(
            fsl_replay::CompileUnit {
                source,
                package_name: "protocol-consumer".into(),
                path: "protocol.native",
                bytes: UNIT.as_bytes(),
            },
            [&model[..]],
            &fsl_replay::DependencyInput::default(),
            &selections,
            fsl_replay::StageLimits::default(),
            fsl_replay::ReplayLimits::default(),
        )
        .expect("public facade compiles protocol and clock choices");
        let evidence = evidence_for(emitted.bytes(), Some(("acme/orders", &model)));
        match filament_contract_model::read_checked_package(
            emitted.bytes(),
            filament_contract_model::CheckedPackageReadLimits::bounded(),
            &evidence,
        ) {
            filament_contract_model::CheckedPackageDispatchResult::AdmittedV2(package) => *package,
            other => panic!("strict reader refused emitted package: {other:?}"),
        }
    }

    fn package() -> &'static filament_contract_model::CheckedPackageV2 {
        static PACKAGE: std::sync::OnceLock<filament_contract_model::CheckedPackageV2> =
            std::sync::OnceLock::new();
        PACKAGE.get_or_init(checked_protocol)
    }

    fn protocol_id() -> filament_contract_model::CheckedNodeId {
        package()
            .resolved_graph()
            .nodes
            .iter()
            .find(|node| {
                node.node_tag.as_ref() == "protocol"
                    && node.semantic_form.as_ref() == "protocol_clause"
            })
            .expect("protocol clause")
            .node_id
            .clone()
    }

    fn verify_protocol_facts() {
        let package = package();
        let graph = package.resolved_graph();
        let id = protocol_id();
        let view = fsl_package::protocol_view::protocol_view(package, &id).expect("protocol view");
        assert_eq!(view.roles().len(), 2);
        assert_eq!(view.channels().len(), 1);
        assert!(view
            .queues()
            .iter()
            .all(|queue| graph.nodes.iter().any(|node| &node.node_id == *queue)));
        assert!(!view.controls().is_empty());
        assert_eq!(view.compensations().len(), 1);
        assert!(!format!("{:?}", view.run()).is_empty());
        assert!(!format!("{:?}", view.finish()).is_empty());

        let relation =
            fsl_package::protocol_view::causal_relation(package, &id).expect("causal relation");
        assert!(!relation.edges().is_empty());

        let channel = graph
            .nodes
            .iter()
            .find(|node| &node.node_id == view.channels()[0])
            .expect("channel node");
        let endpoints =
            fsl_package::protocol_view::channel_endpoints(channel).expect("channel endpoints");
        assert!(view.roles().contains(&endpoints.sender));
        assert!(view.roles().contains(&endpoints.receiver));
        assert!(!format!("{:?}", endpoints.message_type).is_empty());

        let await_node = graph
            .nodes
            .iter()
            .find(|node| {
                fsl_package::protocol_view::control_operation(node).ok()
                    == Some(fsl_package::protocol_view::ControlOperation::Await)
            })
            .expect("await control");
        let anchor = fsl_package::protocol_view::await_anchor(await_node).expect("await anchor");
        assert_eq!(
            anchor.kind,
            fsl_package::protocol_view::AwaitAnchorKind::Event
        );
        assert!(!format!("{:?}", anchor.target).is_empty());

        let choice_node = graph
            .nodes
            .iter()
            .find(|node| {
                fsl_package::protocol_view::control_operation(node).ok()
                    == Some(fsl_package::protocol_view::ControlOperation::Choice)
            })
            .expect("choice control");
        let choice =
            fsl_package::protocol_view::choice_visibility(choice_node).expect("choice visibility");
        assert!(!format!("{:?}", choice.choice).is_empty());
        assert!(!format!("{:?}", choice.owner).is_empty());
        assert!(choice.visible.is_empty());

        let compensation = view.compensations()[0];
        assert_eq!(
            fsl_package::protocol_view::attempts_maximum(package, compensation).unwrap(),
            2
        );
    }

    #[test]
    #[trace("TC-2585", "FR-911-AC-2", "FR-911-AC-3")]
    fn cf_pr_01_01_protocol_view_exposes_clause_members_and_controls() {
        verify_protocol_facts();
    }

    #[test]
    #[trace("TC-2585", "FR-911-AC-2", "FR-911-AC-3")]
    fn cf_pr_02_01_causal_relation_exposes_derived_edges() {
        verify_protocol_facts();
    }

    #[test]
    #[trace("TC-2585", "FR-911-AC-2", "FR-911-AC-3")]
    fn cf_pr_03_01_channel_endpoints_expose_all_three_nodes() {
        verify_protocol_facts();
    }

    #[test]
    #[trace("TC-2585", "FR-911-AC-2", "FR-911-AC-3")]
    fn cf_pr_05_01_await_anchor_exposes_event_target() {
        verify_protocol_facts();
    }

    #[test]
    #[trace("TC-2585", "FR-911-AC-2", "FR-911-AC-3")]
    fn cf_pr_06_01_choice_visibility_exposes_owner_and_visible_set() {
        verify_protocol_facts();
    }

    #[test]
    #[trace("TC-2585", "FR-911-AC-2", "FR-911-AC-3")]
    fn cf_pr_07_01_attempts_maximum_exposes_authored_retry_bound() {
        verify_protocol_facts();
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
        let error = match fsl_replay::read_native_run_result_bounded(
            br#"{"format":"native-run-result/2"}"#,
            fsl_replay::NativeRunLimits {
                max_input_bytes: 1,
                max_working_bytes: u64::MAX,
            },
        ) {
            Ok(_) => panic!("input exceeded caller bound but was accepted"),
            Err(error) => error,
        };
        assert!(matches!(error, fsl_replay::NativeRunReadError::Read(_)));
    }

    #[test]
    #[trace("TC-1340", "FR-911-AC-4")]
    fn cf_dr_03_01_native_run_result_exposes_command_error_fields() {
        let value = serde_json::json!({
            "format": "native-run-result/2", "stage": "profile",
            "code": "unknown_edition", "message": "bad edition",
            "details": {"phase": "profile", "code": "unknown_edition", "path": "unit.native"},
            "basis": "unavailable",
        });
        let bytes = serde_json::to_vec(&value).expect("result serializes");
        let result = fsl_replay::read_native_run_result_bounded(
            &bytes,
            fsl_replay::NativeRunLimits::UNBOUNDED,
        )
        .expect("native result reads");
        let fsl_replay::NativeRunResult::CommandError(error) = result else {
            panic!("expected command-error result");
        };
        assert_eq!(error.stage(), "profile");
        assert_eq!(error.code(), "unknown_edition");
        assert_eq!(error.message(), "bad edition");
    }
}

mod runtime {
    use super::*;

    #[test]
    #[trace("TC-2586", "FR-911-AC-1")]
    fn cf_rt_01_01_emit_plan_root_exposes_plan_emission_and_checked_unit() {
        let source = fsl_replay::SourceIdentity::new("consumer", "plan-root");
        let package_name = fsl_replay::LibraryName::new("consumer").expect("library name");
        let packages = fsl_semantics::model::intake::PackageInput::new();
        let unit = "language \"ix:native\" edition \"1-draft\";\n\
profile t = \"filament.temporal.event-position.false-extension/v1\";\n\
record Reading { level: Int[0, 9]; }\n\
temporal T using t over (r: Reading) clock \"events\" on origin { eventually[0, 1] holds(r.level > 0) }\n";
        let one = filament_exact::Rational::new(
            filament_exact::Integer::from(1_u64),
            filament_exact::Integer::from(1_u64),
        )
        .expect("nonzero denominator");
        let zero = filament_exact::Rational::new(
            filament_exact::Integer::from(0_u64),
            filament_exact::Integer::from(1_u64),
        )
        .expect("nonzero denominator");
        let target = fsl_replay::MonitorTarget {
            counter: "counter".into(),
            unit: "event".into(),
            width_bits: 8,
            uncertainty: zero,
            max_event_rate: Some(one),
            max_reading_gap: 2,
            max_activations: 8,
        };
        let settings = fsl_replay::TimingObligationSettings::default();
        let request = fsl_analyze::monitor_root::PlanRootRequest {
            source: &source,
            path: "unit.native",
            bytes: unit.as_bytes(),
            packages: &packages,
            package_name: &package_name,
            limits: fsl_replay::spine::SpineLimits::default(),
            clause: "T",
            target: &target,
            settings: &settings,
        };
        let root =
            fsl_analyze::monitor_root::emit_plan_root(&request, &filament_exact::Cancel::new())
                .expect("public facade emits plan root");
        assert_eq!(root.obligations().len(), 2);
        assert!(!root.emission().package().bytes().is_empty());
        assert!(!format!("{:?}", root.plan()).is_empty());
        assert!(!format!("{:?}", root.checked_unit()).is_empty());
    }
}
