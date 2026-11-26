// SPDX-License-Identifier: AGPL-3.0-or-later
//! Temporal occurrence emission, row identity and clock-selection refusals.

use filament_contract_model::ClockConfiguration;
use filament_exact::Cancel;
use filament_semantic_value::checking::{CheckMode, CheckingLimits};
use fsl_package::{
    emit_checked, emit_checked_with_temporal_selections,
    emit_dependency_with_temporal_selections_and_cancel, CheckedPackage, EmitLimits, EmitRefusal,
    TemporalClockSelection,
};
use fsl_semantics::check::PackageDeclarations;
use ix_trace_rs::trace;
use serde_json::{json, Value};

const EVENT: &str = "filament.temporal.event-position.false-extension/v1";
const INFINITE: &str = "filament.temporal.infinite-trace/v1";

fn unit(profile: &str, declarations: &str) -> String {
    format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.model.complete/v1\";\n\
         profile t = \"{profile}\";\n\
         model Config = \"acme/orders\";\n\
         {declarations}\n"
    )
}

fn declared(name: &str, body: &str) -> String {
    format!(
        "temporal {name} using t over (c: Config::Widget) clock \"model-steps\" \
         on origin {{ {body} }}"
    )
}

fn graph(source_text: &str) -> fsl_semantics::check::CheckedGraph {
    graph_with_model(
        source_text,
        include_bytes!("fixtures/verification-plan-model.semantic-ir.json"),
    )
}

fn graph_with_model(
    source_text: &str,
    model_document: &[u8],
) -> fsl_semantics::check::CheckedGraph {
    declarations_with_model(source_text, model_document)
        .check(
            CheckMode::Linked,
            CheckingLimits::default(),
            &fsl_semantics::check::test_support::ZeroRewardEvaluator,
        )
        .unwrap_or_else(|refusal| panic!("S3 checks the source: {refusal:?}"))
}

fn declarations_with_model(source_text: &str, model_document: &[u8]) -> PackageDeclarations {
    declarations_named(source_text, model_document, "test/package")
}

fn declarations_named(
    source_text: &str,
    model_document: &[u8],
    package_name: &str,
) -> PackageDeclarations {
    let parsed = fsl_cst::parse(
        fsl_foundation::SourceIdentity::new("agent-ix", "test:qsl-2094-temporal-rows"),
        "unit.native",
        source_text.as_bytes(),
        fsl_cst::Limits::default(),
    )
    .expect("S1 reads the source");
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let unit = fsl_forms::build_unit(&parsed).expect("S2 builds the source");
    let model_packages = fsl_semantics::model::intake::package_input([model_document])
        .expect("the selected model package is admitted");
    let models = fsl_semantics::model::intake::admit_unit(
        &unit.selections().models,
        &model_packages,
        fsl_semantics::model::accounting::ModelNormalizationLimits::default(),
    )
    .expect("the unit's model selection resolves");
    PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        package_name.to_owned(),
        unit,
        models,
        Vec::new(),
    )
    .expect("the unit assembles")
}

fn model_with_version_number() -> Vec<u8> {
    let mut document: Value = serde_json::from_slice(include_bytes!(
        "fixtures/verification-plan-model.semantic-ir.json"
    ))
    .expect("the fixture is JSON");
    let widget = document["types"]
        .as_array_mut()
        .expect("model types")
        .iter_mut()
        .find(|ty| ty["identity"] == "ix://acme/orders/Widget")
        .expect("the Widget type");
    widget["fields"].as_array_mut().unwrap().push(json!({
        "identity":"ix://acme/orders/Widget/versionNumber",
        "name":"versionNumber",
        "typeRef":"ix://filament/native/Integer",
        "presence":"required",
        "nullable":false,
        "defaultKind":"none",
        "multiplicity":{"lower":1,"upper":1,"ordered":false,"unique":false},
        "origin":{"generated":{"generatorIdentity":"ix://acme/orders/spec",
            "generatorVersion":"1.0.0",
            "inputIdentities":["ix://acme/orders/Widget/versionNumber"]}}
    }));
    document.to_string().into_bytes()
}

fn event_clock(name: &str) -> ClockConfiguration {
    ClockConfiguration::EventPosition {
        sequence_authority: format!("authority-{name}").into(),
    }
}

fn timestamp_clock(name: &str) -> ClockConfiguration {
    ClockConfiguration::TimestampedEvent {
        timestamp_unit: format!("unit-{name}").into(),
    }
}

fn selections(
    checked: &fsl_semantics::check::CheckedGraph,
    clocks: impl Fn(&str) -> Option<ClockConfiguration>,
) -> Vec<TemporalClockSelection> {
    checked
        .temporal_clauses()
        .iter()
        .filter_map(|temporal| {
            Some(TemporalClockSelection::new(
                checked.source().clone(),
                temporal.clause().span(),
                clocks(temporal.clause().name())?,
            ))
        })
        .collect()
}

fn emit(
    source_text: &str,
    clocks: impl Fn(&str) -> Option<ClockConfiguration>,
) -> fsl_package::Emission {
    let checked = graph(source_text);
    let selections = selections(&checked, clocks);
    let package = CheckedPackage::link(checked);
    let emission =
        emit_checked_with_temporal_selections(&package, &selections, EmitLimits::new(1 << 32))
            .unwrap_or_else(|refusal| panic!("the temporal package emits: {refusal:?}"));
    emission
}

fn document(emission: &fsl_package::Emission) -> Value {
    serde_json::from_slice(emission.package().bytes()).expect("emitted bytes are JSON")
}

fn rows<'a>(document: &'a Value, member: &str) -> &'a Vec<Value> {
    document[member]
        .as_array()
        .unwrap_or_else(|| panic!("{member} is an array: {}", document[member]))
}

/// TC-524 / FR-337-AC-3: step intervals lower to exact operator identities
/// and literal interval values under both temporal profiles.
#[trace("TC-524", "FR-337-AC-3")]
#[test]
fn temporal_step_intervals_emit_their_exact_operator_and_bounds() {
    let model = model_with_version_number();
    let cases = [
        (
            EVENT,
            "eventually[0,5] holds(c.versionNumber = 2)",
            "filament.op.temporal.eventually",
            json!({"lower":"0","upper":"5"}),
        ),
        (
            EVENT,
            "once[1,1] holds(c.versionNumber = 2)",
            "filament.op.temporal.once",
            json!({"lower":"1","upper":"1"}),
        ),
        (
            INFINITE,
            "eventually[3,*] holds(c.versionNumber = 2)",
            "filament.op.temporal.eventually",
            json!({"lower":"3","upper":null}),
        ),
    ];
    for (profile, body, expected_operation, expected_interval) in cases {
        let source = unit(profile, &declared("Bounded", body));
        let checked = graph_with_model(&source, &model);
        let configured = checked
            .temporal_clauses()
            .iter()
            .map(|clause| {
                TemporalClockSelection::new(
                    checked.source().clone(),
                    clause.clause().span(),
                    if profile == EVENT {
                        event_clock(clause.clause().name())
                    } else {
                        // Infinite-trace has no binding row, but S4 still takes
                        // no selection for this profile.
                        event_clock(clause.clause().name())
                    },
                )
            })
            .collect::<Vec<_>>();
        let package = CheckedPackage::link(checked);
        let selected = if profile == EVENT {
            configured.as_slice()
        } else {
            &[]
        };
        let emission =
            emit_checked_with_temporal_selections(&package, selected, EmitLimits::new(1 << 32))
                .expect("the clause emits");
        let mut wire = document(&emission);
        fsl_semantics::check::wire_id::resolve_operand_slots(&mut wire)
            .expect("every operand slot names its emitted node");
        let node = wire["semantic_graph"]["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|node| {
                node["node_tag"] == "temporal" && node["semantic_form"] == "temporal_clause"
            })
            .expect("the checked temporal clause node");
        let body = &node["body"];
        let formula_ref = &body["arguments"][5];
        assert_eq!(formula_ref["term"], "reference");
        let formula = wire["semantic_graph"]["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|candidate| candidate["node_id"] == formula_ref["target"])
            .expect("the formula binding");
        let operation = formula["body"]["operation"]["identity"]
            .as_str()
            .expect("the temporal operator identity");
        assert_eq!(operation, expected_operation, "{profile}: {body}");
        assert_eq!(
            formula["body"]["operation"]["member"],
            json!({"kind":"temporal_interval","interval":expected_interval}),
            "{profile}: {body}"
        );
        assert_eq!(
            body["arguments"][4],
            json!({"term":"aggregate","members":[]})
        );
    }
}

#[trace("TC-1308", "FR-337-AC-20")]
#[test]
fn required_clock_selection_refusals_are_typed_and_located() {
    let source = unit(
        EVENT,
        &declared("Bound", "eventually[0,1] holds(c.code = 2)"),
    );

    let missing_graph = graph(&source);
    let missing_span = missing_graph.temporal_clauses()[0].clause().span();
    let missing = CheckedPackage::link(missing_graph);
    let refusal = emit_checked_with_temporal_selections(&missing, &[], EmitLimits::new(1 << 32))
        .expect_err("event-position profile requires a selection");
    assert!(matches!(
        &refusal,
        EmitRefusal::MissingTemporalClockSelection { span, .. } if *span == missing_span
    ));
    assert_eq!(refusal.code(), fsl_foundation::Code::MissingDeclaration);
    assert_eq!(refusal.cause(), Some("missing-selection"));

    let surplus_graph = graph(&source);
    let temporal = &surplus_graph.temporal_clauses()[0];
    let selection = TemporalClockSelection {
        declaration_span: fsl_foundation::Span { start: 0, end: 1 },
        ..TemporalClockSelection::new(
            surplus_graph.source().clone(),
            temporal.clause().span(),
            event_clock(temporal.clause().name()),
        )
    };
    let surplus_span = selection.declaration_span;
    let surplus = CheckedPackage::link(surplus_graph);
    let refusal =
        emit_checked_with_temporal_selections(&surplus, &[selection], EmitLimits::new(1 << 32))
            .expect_err("a selection must name a checked occurrence");
    assert!(matches!(
        &refusal,
        EmitRefusal::SurplusTemporalClockSelection { span, .. } if *span == surplus_span
    ));
    assert_eq!(refusal.code(), fsl_foundation::Code::InvalidPackage);
    assert_eq!(refusal.cause(), Some("timed-node-reference"));

    let wrong_graph = graph(&source);
    let temporal = &wrong_graph.temporal_clauses()[0];
    let selection = TemporalClockSelection::new(
        wrong_graph.source().clone(),
        temporal.clause().span(),
        timestamp_clock(temporal.clause().name()),
    );
    let wrong_span = selection.declaration_span;
    let wrong = CheckedPackage::link(wrong_graph);
    let refusal =
        emit_checked_with_temporal_selections(&wrong, &[selection], EmitLimits::new(1 << 32))
            .expect_err("the selected kind must agree with event-position");
    assert!(matches!(
        &refusal,
        EmitRefusal::TemporalClockProfileMismatch { span, .. } if *span == wrong_span
    ));
    assert_eq!(refusal.code(), fsl_foundation::Code::InvalidPackage);
    assert_eq!(refusal.cause(), Some("timed-node-reference"));
}

#[trace("TC-1267", "FR-337-AC-29")]
#[test]
fn a_model_owned_clock_unit_types_both_duration_bounds() {
    let model_source = br#"language "ix:native" edition "1-draft";
domain "acme/rpc";
dimension Time;
unit s : Time = rational(1, 1);
event Tick { timestamp: Timestamp; occurs timestamp; }
"#;
    let model_input = fsl_semantics::model::intake::package_input([model_source.as_slice()])
        .expect("the model's root time unit compiles");
    let mut model_document: Value =
        serde_json::from_slice(&model_input["acme/rpc"]).expect("compiled model package is JSON");
    let mut millisecond = model_document["units"][0].clone();
    millisecond["identity"] = json!("ix://acme/rpc/unit/ms");
    millisecond["name"] = json!("ms");
    millisecond["target"] = json!("ix://acme/rpc/unit/s");
    millisecond["scale"] = json!("1/1000");
    model_document["units"]
        .as_array_mut()
        .expect("compiled model units")
        .push(millisecond);
    let model = model_document.to_string().into_bytes();
    let source = r#"
language "ix:native" edition "1-draft";
profile v = "filament.model.complete/v1";
profile t = "filament.temporal.timestamped-event.finite-window/v1";
model M = "acme/rpc";
temporal Window using t over (e: M::Tick) clock "events" on origin {
  count over past[0 M::s, 300000 M::ms] >= 1
}
"#;
    let probe = declarations_named(source, &model, "acme/orders")
        .check(
            CheckMode::Linked,
            CheckingLimits::default(),
            &fsl_semantics::check::test_support::ZeroRewardEvaluator,
        )
        .expect("the clause checks before its explicit clock is applied");
    let [temporal] = probe.temporal_clauses() else {
        panic!("one temporal clause");
    };
    let span = temporal.clause().span();
    let clock = ClockConfiguration::TimestampedEvent {
        timestamp_unit: "ix://acme/rpc/unit/ms".into(),
    };
    let graph = declarations_named(source, &model, "acme/orders")
        .with_clock_units(vec![fsl_semantics::check::ClockUnitSelection {
            declaration_span: span,
            clock: clock.clone(),
        }])
        .check(
            CheckMode::Linked,
            CheckingLimits::default(),
            &fsl_semantics::check::test_support::ZeroRewardEvaluator,
        )
        .expect("S3 selects the model-owned timestamp unit");
    let selection = TemporalClockSelection::new(graph.source().clone(), span, clock);
    let package = CheckedPackage::link(graph);
    let emission =
        emit_checked_with_temporal_selections(&package, &[selection], EmitLimits::new(1 << 32))
            .expect("the clocked package emits");
    let wire = document(&emission);
    let unit_path = json!(["scalar_type", "unit", "ix://acme/rpc/unit/ms"]);
    let units: Vec<_> = wire["semantic_graph"]["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|node| node["node_id"]["path"] == unit_path)
        .collect();
    let [unit_node] = units.as_slice() else {
        panic!("one model-owned `ms` node: {units:?}");
    };
    assert_eq!(unit_node["node_id"]["package"], "acme/rpc");
    assert_eq!(unit_node["owner"]["kind"], "model");
    assert_eq!(unit_node["owner"]["identity"], "acme/rpc");
    assert_eq!(unit_node["owner"]["node"], "ix://acme/rpc/unit/ms");

    let clauses: Vec<_> = wire["semantic_graph"]["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|node| node["node_tag"] == "temporal" && node["semantic_form"] == "temporal_clause")
        .collect();
    let [clause] = clauses.as_slice() else {
        panic!("one emitted temporal clause: {clauses:?}");
    };
    assert_eq!(clause["node_id"]["package"], "acme/orders");
    let aggregate = wire["semantic_graph"]["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|node| node["body"]["operation"]["identity"] == "filament.op.temporal.aggregate")
        .expect("the checked count aggregate");
    let bounds = aggregate["body"]["arguments"][0]["members"]
        .as_array()
        .expect("the aggregate arguments");
    for (name, expected) in [("lower", "0"), ("upper", "300000")] {
        let bound = bounds
            .iter()
            .find(|member| member["name"] == name)
            .map(|member| &member["value"])
            .unwrap_or_else(|| panic!("aggregate has {name}: {bounds:?}"));
        assert_eq!(bound["value_kind"], "quantity", "{name}");
        assert_eq!(bound["value"], expected, "{name}");
        assert_eq!(bound["type"]["package"], "acme/rpc", "{name}");
        assert_eq!(bound["type"]["path"], unit_path, "{name}");
    }
}

#[trace("TC-1326", "FR-337-AC-16", "FR-337-AC-19")]
#[test]
fn equal_clauses_keep_each_occurrence_name_and_clock() {
    let declarations = format!(
        "{}\n{}",
        declared("Alpha", "eventually[0,1] holds(true)"),
        declared("Beta", "eventually[0,1] holds(true)")
    );
    let emission = emit(&unit(EVENT, &declarations), |name| Some(event_clock(name)));
    let wire = document(&emission);
    let [first, second] = rows(&wire, "temporal_clause_names").as_slice() else {
        panic!(
            "two occurrence name rows: {}",
            wire["temporal_clause_names"]
        );
    };
    assert_eq!(first["occurrence"], 0);
    assert_eq!(first["name"], "Alpha");
    assert_eq!(second["occurrence"], 1);
    assert_eq!(second["name"], "Beta");
    assert_eq!(first["declaration"], second["declaration"]);
    let clause_nodes = wire["semantic_graph"]["nodes"]
        .as_array()
        .expect("semantic graph nodes")
        .iter()
        .filter(|node| node["node_tag"] == "temporal" && node["semantic_form"] == "temporal_clause")
        .count();
    assert_eq!(clause_nodes, 1);
    let clauses = emission.temporal_rows().clauses();
    assert_eq!(clauses.len(), 2);
    assert_eq!(clauses[0].declaration, clauses[1].declaration);
    assert_eq!([clauses[0].occurrence, clauses[1].occurrence], [0, 1]);
    let bindings = rows(&wire, "temporal_bindings");
    assert_eq!(bindings.len(), 2);
    assert_eq!(bindings[0]["occurrence"], 0);
    assert_eq!(
        bindings[0]["clock"]["sequence_authority"],
        "authority-Alpha"
    );
    assert_eq!(bindings[1]["occurrence"], 1);
    assert_eq!(bindings[1]["clock"]["sequence_authority"], "authority-Beta");
}

#[trace("TC-1326", "FR-337-AC-19")]
#[test]
fn timestamped_event_keeps_each_occurrence_clock_configuration() {
    let declarations = format!(
        "{}\n{}",
        declared("Alpha", "eventually[0,1] holds(true)"),
        declared("Beta", "eventually[0,1] holds(true)")
    );
    let emission = emit(
        &unit(
            "filament.temporal.timestamped-event.finite-window/v1",
            &declarations,
        ),
        |name| Some(timestamp_clock(name)),
    );
    let wire = document(&emission);
    let bindings = rows(&wire, "temporal_bindings");
    assert_eq!(bindings.len(), 2);
    assert_eq!(bindings[0]["occurrence"], 0);
    assert_eq!(bindings[0]["clock"]["timestamp_unit"], "unit-Alpha");
    assert_eq!(bindings[1]["occurrence"], 1);
    assert_eq!(bindings[1]["clock"]["timestamp_unit"], "unit-Beta");
}

#[trace("TC-1326", "FR-337-AC-16")]
#[test]
fn equal_clauses_under_infinite_profile_keep_both_names_on_one_node() {
    let declarations = format!(
        "{}\n{}",
        declared("Alpha", "eventually holds(true)"),
        declared("Bravo", "eventually holds(true)")
    );
    let emission = emit(&unit(INFINITE, &declarations), |_| None);
    let wire = document(&emission);
    let names = rows(&wire, "temporal_clause_names");
    assert_eq!(names.len(), 2);
    assert_eq!(
        [
            names[0]["occurrence"].as_u64(),
            names[1]["occurrence"].as_u64()
        ],
        [Some(0), Some(1)]
    );
    assert_eq!(
        [names[0]["name"].as_str(), names[1]["name"].as_str()],
        [Some("Alpha"), Some("Bravo")]
    );
    assert_eq!(names[0]["declaration"], names[1]["declaration"]);
    assert!(wire.get("temporal_bindings").is_none());
}

#[trace("TC-1326", "FR-337-AC-17", "FR-337-AC-18", "FR-337-AC-21")]
#[test]
fn changing_a_clause_name_changes_only_its_name_row() {
    let body = "eventually[0,1] holds(true)";
    let alpha = emit(&unit(EVENT, &declared("Alpha", body)), |name| {
        Some(event_clock(name))
    });
    let beta = emit(&unit(EVENT, &declared("Beta", body)), |name| {
        Some(event_clock(name))
    });
    let alpha_wire = document(&alpha);
    let beta_wire = document(&beta);
    let alpha_names = rows(&alpha_wire, "temporal_clause_names");
    let beta_names = rows(&beta_wire, "temporal_clause_names");
    assert_eq!(alpha_names.len(), 1);
    assert_eq!(beta_names.len(), 1);
    assert_eq!(alpha_names[0]["name"], "Alpha");
    assert_eq!(beta_names[0]["name"], "Beta");
    assert_eq!(
        alpha_wire["semantic_graph"]["nodes"]
            .as_array()
            .expect("nodes")
            .iter()
            .map(|node| &node["node_id"])
            .collect::<Vec<_>>(),
        beta_wire["semantic_graph"]["nodes"]
            .as_array()
            .expect("nodes")
            .iter()
            .map(|node| &node["node_id"])
            .collect::<Vec<_>>()
    );
    assert_eq!(alpha.package().name(), beta.package().name());

    let one = emit(&unit(INFINITE, &declared("Reaches", body)), |_| None);
    let one_wire = document(&one);
    assert_eq!(rows(&one_wire, "temporal_clause_names").len(), 1);
    assert_eq!(rows(&one_wire, "temporal_clause_names")[0]["occurrence"], 0);
    assert!(one_wire.get("temporal_bindings").is_none());

    let none = emit(
        &unit(INFINITE, "function noop using v(): Boolean pure { true }"),
        |_| None,
    );
    assert!(document(&none).get("temporal_clause_names").is_none());
}

/// FR-640-AC-16: a package emitted again as a dependency under other clock
/// selections keeps the later emission, which a following `emit_checked`
/// returns.
#[trace("TC-2212", "FR-640-AC-16")]
#[test]
fn a_later_dependency_emission_replaces_the_remembered_one() {
    let source = unit(
        EVENT,
        &declared("Bound", "eventually[0,1] holds(c.code = 2)"),
    );
    let graph = graph(&source);
    let span = graph.temporal_clauses()[0].clause().span();
    let source_ref = graph.source().clone();
    let package = CheckedPackage::link(graph);
    let limits = EmitLimits::new(1 << 32);
    let emit_under = |authority: &str| {
        let selection =
            TemporalClockSelection::new(source_ref.clone(), span, event_clock(authority));
        emit_dependency_with_temporal_selections_and_cancel(
            &package,
            &[selection],
            &Cancel::new(),
            limits,
        )
        .expect("the selection emits")
    };

    let first = emit_under("first");
    let second = emit_under("second");
    assert_ne!(first.package().bytes(), second.package().bytes());
    let held = emit_checked(&package, limits).expect("the remembered emission");
    assert_eq!(held.package().bytes(), second.package().bytes());
    let wire: Value = serde_json::from_slice(held.package().bytes()).expect("the wire is JSON");
    assert_eq!(
        wire["temporal_bindings"][0]["clock"]["sequence_authority"],
        "authority-second"
    );
}
