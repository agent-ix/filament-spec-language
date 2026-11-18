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

    pub(super) fn model_with_touch() -> Vec<u8> {
        let mut model: serde_json::Value =
            serde_json::from_slice(include_bytes!("../../../spine-model.semantic-ir.json"))
                .expect("model fixture JSON");
        let operation = "ix://acme/orders/Widget/touch";
        let origin = serde_json::json!({"source": {
            "sourceIdentity": "ix://acme/orders/spec", "path": "domain.fspec",
            "startLine": 1, "startColumn": 1,
        }});
        let types = model["types"].as_array_mut().expect("types");
        let widget = types
            .iter_mut()
            .find(|ty| ty["identity"] == "ix://acme/orders/Widget")
            .expect("Widget");
        widget["operations"] = serde_json::json!([{
            "identity": operation, "name": "touch", "params": [],
            "returns": {"typeRef": "ix://acme/orders/Widget",
                "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true},
                "nullable": false},
            "pre": [], "post": [], "origin": origin,
            "frame": {"modifies": [], "creates": [], "deletes": []},
        }]);
        let mut attempt = widget.clone();
        attempt["identity"] = serde_json::json!("ix://acme/orders/Attempt");
        attempt["displayName"] = serde_json::json!("Attempt");
        attempt["kind"]["name"] = serde_json::json!("record");
        attempt["origin"] = serde_json::json!({"source": {
            "sourceIdentity": "ix://acme/orders/spec", "path": "domain.fspec",
            "startLine": 1, "startColumn": 1,
        }});
        attempt["operations"] = serde_json::json!([]);
        let attempt_origin = attempt["origin"].clone();
        for field in attempt["fields"].as_array_mut().expect("fields") {
            let name = field["name"].as_str().expect("field name");
            field["identity"] = serde_json::json!(format!("ix://acme/orders/Attempt/{name}"));
            field["origin"] = attempt_origin.clone();
        }
        types.push(attempt);
        model["constructs"]
            .as_array_mut()
            .expect("constructs")
            .push(serde_json::json!({
                "kind": {"module": "acme/orders", "name": "record"},
                "moduleVersion": "1.0.0",
                "construct": {"identity": "none", "shape": "record", "members": {},
                    "meaning": "filament.meaning.model.record-value-type/v1"},
            }));
        serde_json::to_vec(&model).expect("fixture serializes")
    }

    #[test]
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
            filament_exact::Identifier::new("f").expect("identifier"),
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
        let unit = "language \"ix:native\" edition \"1-draft\";\nprofile m = \"filament.model.complete/v1\";\nmodel M = \"acme/orders\";\npost Touch using m on M::Widget::touch { true }\n";
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

    #[test]
    #[trace("TC-2575", "FR-911-AC-2")]
    fn cf_cg_04_01_closed_form_check_exposes_the_checked_verdict() {
        let unit = "language \"ix:native\" edition \"1-draft\";\n\
profile v = \"filament.value.complete/v1\";\n\
profile t = \"filament.temporal.complete/v1\";\n\
dimension Time;\n\
unit ms : Time = rational(1, 1);\n\
taskset Ctl { task a periodic 4 ms deadline 4 ms wcet 1 ms priority 1 lo; }\n\
schedulable CtlFp on Ctl under fixed_priority;\n\
function f using v(x: Integer): Integer pure { x }\n";
        let source = fsl_replay::SourceIdentity::new("consumer", "closed-form-check");
        let function = fsl_replay::QualifiedName::new(vec![
            filament_exact::Identifier::new("f").expect("function name"),
        ])
        .expect("qualified function name");
        let site = fsl_replay::call_site(
            source,
            "consumer".into(),
            "unit.native",
            unit.as_bytes(),
            [],
            &fsl_replay::DependencyInput::default(),
            &function,
        )
        .expect("compile checked schedulability input and function");
        let evidence = evidence_for(&site.package, None);
        let checked = match filament_contract_model::read_checked_package(
            &site.package,
            filament_contract_model::CheckedPackageReadLimits::bounded(),
            &evidence,
        ) {
            filament_contract_model::CheckedPackageDispatchResult::AdmittedV2(package) => package,
            other => panic!("strict reader refused schedulability input: {other:?}"),
        };
        let roots = fsl_package::schedulability_roots(&checked).expect("schedulability roots");
        let [root] = roots.as_slice() else {
            panic!("one schedulability claim root: {roots:?}")
        };
        let claim = root.occurrence().clone();
        let obligation = root.obligation_identity();

        let replay_limits = fsl_replay::ReplayLimits::default();
        let request = fsl_replay::ReplayRequest::decode(
            fsl_replay::ReplayRequestWire {
                package: fsl_replay::ReplayPackage::read(&site.package, replay_limits.input_bytes)
                    .expect("read emitted package"),
                profile_selections: vec![],
                dependencies: vec![],
                replay_target: fsl_replay::RequestTargetWire::function(
                    function,
                    site.site.declaration.clone(),
                ),
                source: fsl_replay::ReplaySource::Input(vec![fsl_replay::CanonicalAssignment {
                    parameter: site.site.parameters[0].1.clone(),
                    value: fsl_replay::WitnessValue::Integer(1),
                }]),
                obligation_identity: site.site.obligation_identity().clone(),
                backend: "kani-backend-1".to_owned(),
                state_environment: fsl_replay::StateEnvironment::new(vec![]),
                accounting_limits: filament_exact::ScalarLimits {
                    integer_bits: u64::MAX,
                    decimal_digits: u64::MAX,
                    scale_expansion: u64::MAX,
                    text_input_bytes: u64::MAX,
                    text_scalars: u64::MAX,
                    normalized_scalars: u64::MAX,
                    unit_edges: u64::MAX,
                    value_occurrences: u64::MAX,
                    work_units: u64::MAX,
                    result_units: u64::MAX,
                },
                stage_limits: std::collections::BTreeMap::new(),
                supplied_limits: None,
                object_environment: None,
                declared_domains: vec![],
                byte_provision: vec![(
                    "consumer".to_owned(),
                    "closed-form-check".to_owned(),
                    unit.as_bytes().to_vec(),
                )],
                documents: vec![],
            },
            replay_limits,
        )
        .expect("decode checked schedulability replay request");

        let rational = |value| {
            filament_exact::Rational::new(
                filament_exact::Integer::from(value),
                filament_exact::Integer::from(1),
            )
            .expect("nonzero denominator")
        };
        let subject = fsl_replay::ScheduleSubject::new(
            fsl_replay::TaskSet::new(
                fsl_replay::QualifiedName::new(vec![
                    filament_exact::Identifier::new("Ctl").expect("task-set name"),
                ])
                .expect("qualified task-set name"),
                vec![fsl_replay::Task {
                    name: filament_exact::Identifier::new("a").expect("task name"),
                    arrival: fsl_replay::Arrival::Periodic {
                        period: rational(4),
                    },
                    deadline: rational(4),
                    wcet: fsl_replay::Wcet::Single(rational(1)),
                    priority: Some(1),
                    criticality: fsl_replay::Criticality::Lo,
                    jitter: rational(0),
                    blocking: rational(0),
                }],
            )
            .expect("task set is admitted"),
            fsl_replay::ScheduleClaim::Schedulable(fsl_replay::SchedulingPolicy::FixedPriority),
        )
        .expect("schedulability claim is admitted");
        let cancel = filament_exact::Cancel::new();
        let outcome = fsl_analyze::schedulability::analyze(
            &subject,
            &fsl_analyze::schedulability::ScheduleLimits::default(),
            &cancel,
        );
        let result = fsl_replay::ClosedFormResult {
            originating_obligation: obligation.clone(),
            outcome,
        };

        assert!(matches!(
            fsl_replay::check_closed_form(request, &obligation, &claim, &result, &cancel),
            Ok(fsl_replay::ClosedFormCheck::Agrees),
        ));
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
    #[trace("TC-2575", "FR-911-AC-2")]
    fn cf_ir_04_01_schedulability_root_exposes_obligation_identity() {
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
        let identity = root.obligation_identity();
        assert_eq!(identity.node(), root.occurrence().node());
        assert_eq!(identity.model(), None);
        assert_eq!(
            u64::from(identity.ordinal()),
            root.occurrence().origin().ordinal()
        );
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

mod protocol;

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
        let mut model: serde_json::Value =
            serde_json::from_slice(include_bytes!("../../../spine-model.semantic-ir.json"))
                .expect("model fixture JSON");
        model["types"]
            .as_array_mut()
            .expect("model types")
            .retain(|ty| ty["identity"] == "ix://acme/orders/Widget");
        let model = serde_json::to_vec(&model).expect("single-object model serializes");
        let packages =
            fsl_semantics::model::intake::package_input([model.as_slice()]).expect("model intake");
        let unit = "language \"ix:native\" edition \"1-draft\";\n\
profile v = \"filament.value.complete/v1\";\n\
profile t = \"filament.temporal.timed/v1\";\n\
profile tc = \"filament.temporal.complete/v1\";\n\
model M = \"acme/orders\";\n\
dimension Time;\n\
unit ms : Time = rational(1, 1);\n\
time M dense unit ms;\n\
temporal T using t over (r: M::Widget) clock \"model-time\" on origin { eventually[0 ms, 1 ms] holds(r.code > 0) }\n";
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
            unit: "ms".into(),
            width_bits: 8,
            uncertainty: zero,
            max_event_rate: Some(one),
            max_reading_gap: 1,
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

    #[test]
    #[trace("TC-2575", "FR-911-AC-2")]
    fn cf_rt_02_01_admitted_progress_view_exposes_plan_authority() {
        let source = fsl_replay::SourceIdentity::new("consumer", "plan-root");
        let package_name = fsl_replay::LibraryName::new("consumer").expect("library name");
        let mut model: serde_json::Value =
            serde_json::from_slice(include_bytes!("../../../spine-model.semantic-ir.json"))
                .expect("model fixture JSON");
        model["types"]
            .as_array_mut()
            .expect("model types")
            .retain(|ty| ty["identity"] == "ix://acme/orders/Widget");
        let model = serde_json::to_vec(&model).expect("single-object model serializes");
        let packages =
            fsl_semantics::model::intake::package_input([model.as_slice()]).expect("model intake");
        let unit = "language \"ix:native\" edition \"1-draft\";\n\
profile v = \"filament.value.complete/v1\";\n\
profile t = \"filament.temporal.timed/v1\";\n\
profile tc = \"filament.temporal.complete/v1\";\n\
model M = \"acme/orders\";\n\
dimension Time;\n\
unit ms : Time = rational(1, 1);\n\
time M dense unit ms;\n\
temporal T using t over (r: M::Widget) clock \"model-time\" on origin { eventually[0 ms, 1 ms] holds(r.code > 0) }\n";
        let zero = filament_exact::Rational::new(
            filament_exact::Integer::from(0_u64),
            filament_exact::Integer::from(1_u64),
        )
        .expect("nonzero denominator");
        let target = fsl_replay::MonitorTarget {
            counter: "counter".into(),
            unit: "ms".into(),
            width_bits: 8,
            uncertainty: zero,
            max_event_rate: None,
            max_reading_gap: 1,
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
                .expect("plan root emits");
        let selected_source = root.checked_unit().source().identity().clone();
        let claim = root.plan().subject().clone();
        let observation_subject = filament_observation::authority::SubjectSelection {
            scope_identity: filament_observation::Identity::new("scope:consumer"),
            population_identity: filament_observation::Identity::new("population:consumer"),
        };
        let observation_clock = filament_observation::authority::clock::Selection::new(
            filament_observation::Identity::new("clock:consumer"),
            filament_observation::Identity::new("revision:consumer"),
        );
        let observation_source = filament_observation::Identity::new("source:consumer");
        let mut budget = filament_contract_model::NativeJoinBudget::new(100, 100);
        let result = fsl_analyze::monitor_root::admit_progress_view(
            root,
            fsl_analyze::monitor_root::PlanAuthoritySelection {
                source: &selected_source,
                claim: &claim,
                observation_subject: &observation_subject,
                observation_clock: &observation_clock,
                observation_source: &observation_source,
            },
            None,
            fsl_analyze::monitor_root::PlanAuthorityValidationCaps {
                read: filament_contract_model::CheckedPackageReadLimits::bounded(),
                order: quire_canonical::Limits::new(1 << 20),
            },
            &mut budget,
            filament_contract_model::NativeJoinLocus {
                source: filament_contract_model::NativeJoinLocusSource::ReachedRow(0),
                form: "consumer_progress_view",
            },
        );
        assert!(matches!(
            result,
            Err(fsl_analyze::monitor_root::PlanAuthorityRefusal::MissingProgress)
        ));
    }
}

mod observation {
    use super::*;

    #[test]
    #[trace("TC-2575", "FR-911-AC-2")]
    fn cf_ob_01_01_checked_subject_kinds_expose_domain_declarations() {
        let requirements = fsl_package::checked_views::subject_kinds(super::protocol::package())
            .expect("checked protocol subject kinds");
        assert!(requirements.iter().any(|requirement| {
            requirement.kind == fsl_package::checked_views::SubjectKind::WorkflowInstance
        }));
        let workflows = requirements
            .iter()
            .filter(|requirement| {
                requirement.kind == fsl_package::checked_views::SubjectKind::WorkflowInstance
            })
            .collect::<Vec<_>>();
        assert_eq!(workflows.len(), 1);
        assert_eq!(workflows[0].scope, workflows[0].subject);
    }
}
