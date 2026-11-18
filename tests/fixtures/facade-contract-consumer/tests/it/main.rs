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
    #[ignore = "QSL-1109"]
    #[trace("TC-2575", "FR-911-AC-2")]
    fn cf_cg_04_01_closed_form_check_exposes_the_checked_verdict() {
        panic!("pending QSL-1109: CF-CG-04 check_closed_form consumer contract");
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
    #[ignore = "QSL-1109"]
    #[trace("TC-2575", "FR-911-AC-2")]
    fn cf_ir_04_01_schedulability_root_exposes_obligation_identity() {
        panic!("pending QSL-1109: CF-IR-04 obligation_identity consumer contract");
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
    use filament_contract_model::{
        CheckedBindingNameKind, CheckedNodeId, CheckedPackageV2, CheckedSemanticNodeV2, CheckedTerm,
    };

    fn protocol_emission(model: &[u8]) -> fsl_package::Emission {
        use filament_semantic_value::checking::{CheckMode, CheckingLimits};
        use fsl_semantics::check::PackageDeclarations;

        let source = fsl_foundation::SourceIdentity::new("consumer", "protocol-contract");
        let parsed = fsl_cst::parse(
            source,
            "protocol.native",
            UNIT.as_bytes(),
            fsl_cst::Limits::default(),
        )
        .expect("protocol parses");
        let raw_source = parsed.source().reference().clone();
        let built = fsl_forms::build_unit(&parsed).expect("protocol forms build");
        let models = fsl_semantics::model::intake::package_input([model]).expect("model intake");
        let selected = fsl_semantics::model::intake::admit_unit(
            &built.selections().models,
            &models,
            fsl_semantics::model::accounting::ModelNormalizationLimits::default(),
        )
        .expect("model selections admit");
        let graph = PackageDeclarations::assemble(
            raw_source,
            "protocol-consumer".into(),
            built,
            selected,
            Vec::new(),
        )
        .expect("protocol assembles")
        .check(CheckMode::Linked, CheckingLimits::default())
        .expect("protocol checks");
        let source = graph.source().clone();
        let clock = filament_contract_model::ClockConfiguration::Timed {
            unit: "ticks".into(),
        };
        let selections = graph
            .protocols()
            .iter()
            .flat_map(|protocol| {
                protocol
                    .obligations
                    .compensations
                    .iter()
                    .filter(|template| {
                        matches!(
                            template.temporal_profile,
                            Some(
                                fsl_semantics::check::TemporalProfile::EventPositionFalseExtension
                                    | fsl_semantics::check::TemporalProfile::FixedSampleFalseExtension
                                    | fsl_semantics::check::TemporalProfile::TimestampedEventFiniteWindow
                            )
                        )
                    })
                    .map(|template| template.span)
                    .chain(
                        protocol
                            .awaits
                            .iter()
                            .filter(|awaited| {
                                matches!(
                                    awaited.profile,
                                    Some(
                                        fsl_semantics::check::TemporalProfile::EventPositionFalseExtension
                                            | fsl_semantics::check::TemporalProfile::FixedSampleFalseExtension
                                            | fsl_semantics::check::TemporalProfile::TimestampedEventFiniteWindow
                                    )
                                )
                            })
                            .map(|awaited| awaited.span),
                    )
            })
            .map(|span| {
                fsl_package::TemporalClockSelection::new(source.clone(), span, clock.clone())
            })
            .collect::<Vec<_>>();
        let package = fsl_package::CheckedPackage::link(graph);
        fsl_package::emit_checked_with_temporal_selections(
            &package,
            &selections,
            fsl_package::EmitLimits::new(1 << 20),
        )
        .expect("protocol package emits")
    }

    const UNIT: &str = "language \"ix:native\" edition \"1-draft\";\n\
profile p = \"filament.protocol.finite-global/v1\";\n\
profile v = \"filament.value.complete/v1\";\n\
profile t = \"filament.temporal.timed/v1\";\n\
profile u = \"filament.temporal.timed/v1\";\n\
model M = \"acme/orders\";\n\
protocol Flow using p over (input: M::Widget) on origin {\n\
  role Buyer on M::Widget;\n\
  role Seller on M::Widget;\n\
  channel Ch from Buyer to Seller carries Boolean ordering unordered delivery at-least-once capacity 4 overflow block;\n\
  compensate Full for Main::Applied as (fwd: M::Widget) by Buyer on M::Widget::touch using t clock \"ticks\" {\n\
    activate first (trig: M::Widget) when { true } {}\n\
    within [0, 10]; attempts 2 of M::Attempt;\n\
    retry (early: M::Attempt, late: M::Attempt) { false };\n\
    commit never; recover (rec: M::Widget) { true };\n\
  }\n\
  run sequence Main {\n\
    attempt Update by Buyer on M::Widget::touch contracts [] as (tried: M::Widget) { true };\n\
    effect Applied of Update as (applied: M::Widget) { true };\n\
    send Offer via Ch as (sent: Boolean) { true };\n\
    receive Seen via Ch of Offer as (received: Boolean) { true };\n\
    await Wait after Main::Applied using u clock \"ticks\" within [0, 1] match event Arrived by Seller as (arrived: Boolean) { true }; then sequence Then {} timeout sequence Late {}\n\
  choice Pick by Seller visible (received) { case yes when { true } sequence Yes {} case no when { false } sequence No {} }\n\
  }\n\
  finish End as (outcome: Boolean) { true };\n\
}\n";

    fn checked_protocol() -> filament_contract_model::CheckedPackageV2 {
        let model = super::code_generation::model_with_touch();
        let emitted = protocol_emission(&model);
        let evidence = evidence_for(emitted.package().bytes(), Some(("acme/orders", &model)));
        match filament_contract_model::read_checked_package(
            emitted.package().bytes(),
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

    fn local_name(node: &CheckedSemanticNodeV2) -> Option<&str> {
        node.declaration
            .as_ref()?
            .qualified_name
            .last()
            .map(AsRef::as_ref)
    }

    fn node_for_id<'a>(
        package: &'a CheckedPackageV2,
        id: &CheckedNodeId,
    ) -> &'a CheckedSemanticNodeV2 {
        package
            .resolved_graph()
            .nodes
            .iter()
            .find(|node| &node.node_id == id)
            .expect("checked graph contains the listed node")
    }

    fn id_named<'a>(
        package: &'a CheckedPackageV2,
        ids: &[&'a CheckedNodeId],
        name: &str,
    ) -> &'a CheckedNodeId {
        ids.iter()
            .copied()
            .find(|id| local_name(node_for_id(package, id)) == Some(name))
            .expect("protocol view contains the declared node")
    }

    fn control_named<'a>(
        package: &'a CheckedPackageV2,
        operation: fsl_package::protocol_view::ControlOperation,
        name: &str,
    ) -> &'a CheckedSemanticNodeV2 {
        package
            .resolved_graph()
            .nodes
            .iter()
            .find(|node| {
                local_name(node) == Some(name)
                    && fsl_package::protocol_view::control_operation(node).ok() == Some(operation)
            })
            .expect("checked graph contains the declared control")
    }

    fn binding_value<'a>(term: &'a CheckedTerm, name: &str) -> Option<&'a CheckedTerm> {
        match term {
            CheckedTerm::Binding {
                name: binding,
                value,
            } => {
                if matches!(binding.kind(), CheckedBindingNameKind::Other(found) if found == name) {
                    Some(value)
                } else {
                    binding_value(value, name)
                }
            }
            CheckedTerm::Application { arguments, .. } => arguments
                .iter()
                .find_map(|argument| binding_value(argument, name)),
            CheckedTerm::Aggregate { members } => members
                .iter()
                .find_map(|member| binding_value(member, name)),
            _ => None,
        }
    }

    fn collect_references<'a>(term: &'a CheckedTerm, references: &mut Vec<&'a CheckedNodeId>) {
        match term {
            CheckedTerm::Reference { target } => references.push(target),
            CheckedTerm::Binding { value, .. } => collect_references(value, references),
            CheckedTerm::Application { arguments, .. } => {
                for argument in arguments {
                    collect_references(argument, references);
                }
            }
            CheckedTerm::Aggregate { members } => {
                for member in members {
                    collect_references(member, references);
                }
            }
            _ => {}
        }
    }

    fn binding_references<'a>(
        node: &'a CheckedSemanticNodeV2,
        name: &str,
    ) -> Vec<&'a CheckedNodeId> {
        let value = binding_value(&node.body, name).expect("checked node has the declared member");
        let mut references = Vec::new();
        collect_references(value, &mut references);
        references
    }

    fn binding_reference<'a>(node: &'a CheckedSemanticNodeV2, name: &str) -> &'a CheckedNodeId {
        let references = binding_references(node, name);
        assert_eq!(
            references.len(),
            1,
            "member {name} is one checked reference"
        );
        references[0]
    }

    fn verify_protocol_facts() {
        let package = package();
        let id = protocol_id();
        let view = fsl_package::protocol_view::protocol_view(package, &id).expect("protocol view");
        assert_eq!(view.clause(), &id);
        assert_eq!(view.roles().len(), 2);
        let buyer = id_named(package, view.roles(), "Buyer");
        let seller = id_named(package, view.roles(), "Seller");
        assert_eq!(view.channels().len(), 1);
        let channel_id = id_named(package, view.channels(), "Ch");
        assert_eq!(view.queues().len(), 1);
        assert_eq!(view.compensations().len(), 1);
        let compensation_id = id_named(package, view.compensations(), "Full");
        let run = control_named(
            package,
            fsl_package::protocol_view::ControlOperation::Sequence,
            "Main",
        );
        let finish = control_named(
            package,
            fsl_package::protocol_view::ControlOperation::Finish,
            "End",
        );
        assert_eq!(view.run(), &run.node_id);
        assert_eq!(view.finish(), &finish.node_id);

        let relation =
            fsl_package::protocol_view::causal_relation(package, &id).expect("causal relation");
        let applied = control_named(
            package,
            fsl_package::protocol_view::ControlOperation::Effect,
            "Applied",
        );
        let await_node = control_named(
            package,
            fsl_package::protocol_view::ControlOperation::Await,
            "Wait",
        );
        let anchor_edge = fsl_package::protocol_view::CausalEdge {
            from: fsl_package::protocol_view::PortRef {
                node: applied.node_id.clone(),
                port: fsl_package::protocol_view::Port::Exit,
            },
            to: fsl_package::protocol_view::PortRef {
                node: await_node.node_id.clone(),
                port: fsl_package::protocol_view::Port::Enter,
            },
            kind: fsl_package::protocol_view::EdgeKind::AwaitAnchor,
        };
        assert!(relation.edges().contains(&anchor_edge));

        let channel = node_for_id(package, channel_id);
        let endpoints =
            fsl_package::protocol_view::channel_endpoints(channel).expect("channel endpoints");
        assert_eq!(endpoints.sender, buyer);
        assert_eq!(endpoints.receiver, seller);
        assert_eq!(endpoints.sender, binding_reference(channel, "from"));
        assert_eq!(endpoints.receiver, binding_reference(channel, "to"));
        assert_eq!(
            endpoints.message_type,
            binding_reference(channel, "carries")
        );

        let anchor = fsl_package::protocol_view::await_anchor(await_node).expect("await anchor");
        assert_eq!(
            anchor.kind,
            fsl_package::protocol_view::AwaitAnchorKind::Event
        );
        assert_eq!(anchor.target, &applied.node_id);

        let choice_node = control_named(
            package,
            fsl_package::protocol_view::ControlOperation::Choice,
            "Pick",
        );
        assert!(view.controls().contains(&&applied.node_id));
        assert!(view.controls().contains(&&await_node.node_id));
        assert!(view.controls().contains(&&choice_node.node_id));
        let choice =
            fsl_package::protocol_view::choice_visibility(choice_node).expect("choice visibility");
        assert_eq!(choice.choice, &choice_node.node_id);
        assert_eq!(choice.owner, seller);
        let expected_visible = binding_references(choice_node, "visible");
        assert_eq!(expected_visible.len(), 1);
        assert_eq!(choice.visible, expected_visible);

        let compensation = compensation_id;
        assert_eq!(
            fsl_package::protocol_view::attempts_maximum(package, compensation).unwrap(),
            2
        );

        let buyer_node = node_for_id(package, buyer);
        assert!(matches!(
            fsl_package::protocol_view::protocol_view(package, buyer),
            Err(fsl_package::protocol_view::ViewRefusal::UnknownProtocol { .. })
        ));
        assert!(matches!(
            fsl_package::protocol_view::causal_relation(package, buyer),
            Err(fsl_package::protocol_view::ViewRefusal::UnknownProtocol { .. })
        ));
        assert!(matches!(
            fsl_package::protocol_view::channel_endpoints(buyer_node),
            Err(fsl_package::protocol_view::ViewRefusal::MissingMember { member: "from", .. })
        ));
        assert!(matches!(
            fsl_package::protocol_view::await_anchor(buyer_node),
            Err(fsl_package::protocol_view::ViewRefusal::MissingMember {
                member: "after",
                ..
            })
        ));
        assert!(matches!(
            fsl_package::protocol_view::choice_visibility(await_node),
            Err(fsl_package::protocol_view::ViewRefusal::MalformedMember {
                member: "operator",
                ..
            })
        ));
        assert!(matches!(
            fsl_package::protocol_view::attempts_maximum(package, buyer),
            Err(fsl_package::protocol_view::ViewRefusal::NotACompensation { .. })
        ));
    }

    #[test]
    #[trace(
        "TC-2575",
        "FR-911-AC-2",
        "FSpec-TC-1646",
        "FSpec-FR-661-AC-3",
        "TC-2576",
        "FR-911-AC-3"
    )]
    fn cf_pr_01_01_protocol_view_exposes_clause_members_and_controls() {
        verify_protocol_facts();
    }

    #[test]
    #[trace(
        "TC-2575",
        "FR-911-AC-2",
        "FSpec-TC-1646",
        "FSpec-FR-661-AC-3",
        "TC-2576",
        "FR-911-AC-3"
    )]
    fn cf_pr_02_01_causal_relation_exposes_derived_edges() {
        verify_protocol_facts();
    }

    #[test]
    #[trace(
        "TC-2575",
        "FR-911-AC-2",
        "FSpec-TC-1646",
        "FSpec-FR-661-AC-3",
        "TC-2576",
        "FR-911-AC-3"
    )]
    fn cf_pr_03_01_channel_endpoints_expose_all_three_nodes() {
        verify_protocol_facts();
    }

    #[test]
    #[trace(
        "TC-2575",
        "FR-911-AC-2",
        "FSpec-TC-1646",
        "FSpec-FR-661-AC-3",
        "TC-2576",
        "FR-911-AC-3"
    )]
    fn cf_pr_05_01_await_anchor_exposes_event_target() {
        verify_protocol_facts();
    }

    #[test]
    #[trace(
        "TC-2575",
        "FR-911-AC-2",
        "FSpec-TC-1646",
        "FSpec-FR-661-AC-3",
        "TC-2576",
        "FR-911-AC-3"
    )]
    fn cf_pr_06_01_choice_visibility_exposes_owner_and_visible_set() {
        verify_protocol_facts();
    }

    #[test]
    #[trace(
        "TC-2575",
        "FR-911-AC-2",
        "FSpec-TC-1646",
        "FSpec-FR-661-AC-3",
        "TC-2576",
        "FR-911-AC-3"
    )]
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
profile v = \"filament.value.complete/v1\";\n\
profile t = \"filament.temporal.event-position.false-extension/v1\";\n\
record Reading { level: Int[0, 9]; }\n\
dimension Positions;\n\
unit position : Positions = rational(1, 1);\n\
temporal T using t over (r: Reading) clock \"positions\" on origin { eventually[0, 1] holds(r.level > 0) }\n";
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
    #[ignore = "QSL-1867"]
    #[trace("TC-2575", "FR-911-AC-2")]
    fn cf_rt_02_01_admitted_progress_view_exposes_plan_authority() {
        panic!("pending QSL-1867: CF-RT-02 admit_progress_view consumer contract");
    }
}

mod observation {
    use super::*;

    #[test]
    #[ignore = "OBS-1"]
    #[trace("TC-2575", "FR-911-AC-2")]
    fn cf_ob_01_01_checked_subject_kinds_expose_domain_declarations() {
        panic!("pending OBS-1: CF-OB-01 subject_kinds consumer contract");
    }
}
