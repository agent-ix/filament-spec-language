// SPDX-License-Identifier: AGPL-3.0-or-later
//! Public replay-command cells for TC-3242.

use filament_exact::Cancel;
use fsl_command::command::{self, CommandLimits, RunLimits};
use fsl_foundation::{Setting, SettingLimits, SourceIdentity};
use fsl_replay::spine::{self, ParseRequest, SpineLimits};
use fsl_replay::{
    replay, DependencyInput, Identifier, QualifiedName, ReplayLimits, ReplayPackage, ReplayRequest,
    ReplayRequestRefusal, ReplayRequestWire, ReplaySource, RequestTargetWire, StateEnvironment,
};
use ix_trace_rs::trace;
use serde_json::{json, Value};
use std::time::Instant;

const WIDE: u64 = 1 << 42;
const AUTHORITY: &str = "agent-ix";
const SOURCE_ID: &str = "test:qsl-3570-replay-matrix";

#[derive(Clone, Copy, Debug)]
struct CompilePhaseTimes {
    parse_forms_ms: u128,
    model_select_ms: u128,
    check_ms: u128,
    package_emit_ms: u128,
    emitter: fsl_package::test_support::S4PhaseTimes,
    retained_counts: fsl_package::test_support::S4RetainedCounts,
}

fn unlimited_scalar_limits() -> filament_exact::ScalarLimits {
    filament_exact::ScalarLimits {
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
    }
}

fn package_bytes(
    source_text: &str,
    packages: &fsl_semantics::model::intake::PackageInput,
    model_limits: fsl_semantics::model::accounting::ModelNormalizationLimits,
) -> (Vec<u8>, fsl_replay::FunctionSite, usize, CompilePhaseTimes) {
    let identity = SourceIdentity::new(AUTHORITY, SOURCE_ID);
    let source_limits = fsl_cst::Limits {
        source_bytes: 1 << 28,
        tokens: 2_000_000,
        nodes: 2_000_000,
        work_units: 1 << 42,
        diagnostics: 100,
    };
    let limits = SpineLimits {
        source: source_limits,
        model: model_limits,
        checking: filament_semantic_value::checking::CheckingLimits::new(u64::MAX)
            .with_input_bytes(WIDE)
            .with_work_budget(WIDE),
        package: spine::PackageLimits::default().with_output_bytes(WIDE),
        ..SpineLimits::default()
    };
    let parse_started = Instant::now();
    let parsed = spine::parse(
        &ParseRequest {
            source: &identity,
            path: "unit.native",
            bytes: source_text.as_bytes(),
        },
        limits.source,
        &Cancel::new(),
    )
    .expect("the replay source parses")
    .into_value();
    let parse_forms_ms = parse_started.elapsed().as_millis();
    let source_tokens = parsed.syntax().cst().tokens().len();
    let select_started = Instant::now();
    let models = spine::select(&parsed, packages, limits.model, &Cancel::new())
        .expect("the model inputs admit")
        .into_value();
    let model_select_ms = select_started.elapsed().as_millis();
    let check_started = Instant::now();
    let checked = spine::check(
        &parsed,
        "qsl-3570".to_owned(),
        &models,
        &DependencyInput::default(),
        limits,
        &Cancel::new(),
    )
    .expect("the replay source checks")
    .into_value();
    let check_ms = check_started.elapsed().as_millis();
    let name = QualifiedName::new(vec![Identifier::new("ready").expect("function name")])
        .expect("qualified function name");
    let site = fsl_replay::locate_in_checked(&checked, "qsl-3570", &name)
        .expect("the function site is located");
    fsl_package::test_support::reset_s4_phase_times();
    let package_started = Instant::now();
    let emitted = spine::package(&checked, limits.package, &Cancel::new())
        .expect("the checked package emits")
        .into_value();
    let package_emit_ms = package_started.elapsed().as_millis();
    let emitter = fsl_package::test_support::s4_phase_times();
    let retained_counts = fsl_package::test_support::s4_retained_counts();
    (
        emitted.package().bytes().to_vec(),
        site,
        source_tokens,
        CompilePhaseTimes {
            parse_forms_ms,
            model_select_ms,
            check_ms,
            package_emit_ms,
            emitter,
            retained_counts,
        },
    )
}

fn request_wire(
    package: &[u8],
    source_text: &str,
    site: &fsl_replay::FunctionSite,
) -> ReplayRequestWire {
    let name = QualifiedName::new(vec![Identifier::new("ready").expect("function name")])
        .expect("qualified function name");
    ReplayRequestWire {
        certificate: None,
        profile_selections: vec![],
        package: ReplayPackage::read(package, u64::MAX).expect("the checked package reads"),
        dependencies: vec![],
        replay_target: RequestTargetWire::function(name, site.declaration.clone()),
        source: ReplaySource::ClaimFree,
        obligation_identity: site.obligation_identity().clone(),
        backend: "kani-backend-1".to_owned(),
        state_environment: StateEnvironment::new(vec![]),
        accounting_limits: unlimited_scalar_limits(),
        stage_limits: [
            Setting::EmitOutputBytes,
            Setting::I2InputBytes,
            Setting::I2Nodes,
            Setting::I2Edges,
            Setting::I2Occurrences,
            Setting::I2Diagnostics,
            Setting::I2WorkUnits,
            Setting::I2NativeJoinStorage,
            Setting::ModelDeclarationRecords,
            Setting::ModelDerivationFacts,
            Setting::ModelAncestorSteps,
            Setting::ModelSequenceMaximum,
            Setting::S1InputBytes,
            Setting::S1Tokens,
            Setting::S1Nodes,
            Setting::S1WorkUnits,
            Setting::S3InputBytes,
            Setting::S3Nodes,
            Setting::S3WorkUnits,
        ]
        .into_iter()
        .map(|setting| {
            let bound = match setting {
                Setting::ModelDeclarationRecords => 3_000,
                Setting::ModelDerivationFacts => 1_504_502,
                Setting::ModelAncestorSteps => 1_000,
                Setting::ModelSequenceMaximum => 100_000,
                Setting::S1Tokens => 100_001,
                _ => WIDE,
            };
            (setting.name().to_owned(), bound)
        })
        .collect(),
        supplied_limits: None,
        object_environment: None,
        declared_domains: vec![],
        byte_provision: vec![(
            AUTHORITY.to_owned(),
            SOURCE_ID.to_owned(),
            source_text.as_bytes().to_vec(),
        )],
        documents: vec![],
    }
}

fn request_file_value(wire: &ReplayRequestWire) -> Value {
    let package: Value = serde_json::from_slice(wire.package.bytes()).expect("package is JSON");
    let stage_limits = wire
        .stage_limits
        .iter()
        .map(|(setting, bound)| (setting.clone(), json!(bound.to_string())))
        .collect::<serde_json::Map<_, _>>();
    let occurrence = wire
        .replay_target
        .occurrence_key
        .as_ref()
        .expect("function replay has its declaration occurrence");
    let node_json = |node: &filament_exact::NodeKey| {
        fsl_foundation::node_wire::node_key_to_json(node).expect("node key serializes")
    };
    let obligation = &wire.obligation_identity;
    let provision = &wire.byte_provision[0];
    json!({
        "format": "native-replay/1",
        "request": {
            "profile_selections": [],
            "package": package,
            "dependencies": [],
            "replay_target": {
                "members": {"function":"ready"},
                "arguments": false,
                "occurrence_key": {
                    "node_id": node_json(occurrence.node()),
                    "role": occurrence.origin().role().as_str(),
                    "ordinal": occurrence.origin().ordinal().to_string()
                },
                "over": null
            },
            "source": {"kind":"claim_free"},
            "obligation_identity": {
                "node_id": node_json(obligation.node()),
                "model": obligation.model().map(|model| model.as_str()),
                "ordinal": obligation.ordinal()
            },
            "backend": wire.backend.as_str(),
            "state_environment": [],
            "accounting_limits": {},
            "stage_limits": stage_limits,
            "supplied_limits": null,
            "object_environment": null,
            "declared_domains": [],
            "byte_provision": [{
                "authority": provision.0.as_str(),
                "identity": provision.1.as_str(),
                "bytes_path": "unit.native"
            }]
        }
    })
}

fn assert_command_replay(wire: ReplayRequestWire) {
    let in_process = replay(wire.clone(), ReplayLimits::default().with_input_bytes(WIDE))
        .expect("in-process replay completes");
    let expected: Value = serde_json::from_str(
        &in_process
            .to_function_json()
            .expect("in-process result serializes"),
    )
    .expect("in-process result is JSON");
    assert_eq!(expected["settlement"], "reproduced-without-witness");

    let actual = command_replay_with_exact_input_bound(&wire);
    assert_eq!(actual, expected);
}

fn command_replay_with_exact_input_bound(wire: &ReplayRequestWire) -> Value {
    let envelope = request_file_value(&wire);
    let bytes = serde_json::to_vec(&envelope).expect("request envelope serializes");
    let directory = tempfile::tempdir().expect("replay workspace");
    let request_path = directory.path().join("request.json");
    std::fs::write(&request_path, &bytes).expect("request file writes");
    std::fs::write(
        directory.path().join("unit.native"),
        &wire.byte_provision[0].2,
    )
    .expect("source provision writes");

    // FR-071 charges the package, target, backend and provided source bytes;
    // the fixed-size JSON envelope punctuation is not part of that counter.
    let exact_input_bytes =
        match ReplayRequest::decode(wire.clone(), ReplayLimits::default().with_input_bytes(0)) {
            Err(ReplayRequestRefusal::BoundExceeded(limit)) => {
                usize::try_from(limit.actual).expect("measured replay bytes fit usize")
            }
            Err(other) => panic!("unexpected zero-bound replay request result: {other:?}"),
            Ok(_) => panic!("zero-byte replay input bound admitted the request"),
        };
    let mut caller = CommandLimits::replay();
    assert!(caller.set_bound(Setting::ReplayInputBytes, exact_input_bytes as u64));
    let run_limits = RunLimits {
        request_bytes: 1 << 30,
        dependent_bytes: 1 << 30,
    };
    let result = command::replay_with_limits(&request_path, run_limits, &caller)
        .expect("command::run admits the exact replay-input byte count");
    assert_eq!(result.exit_code, 0);
    let row = result.value.as_value()["results"][0].clone();

    assert!(caller.set_bound(Setting::ReplayInputBytes, (exact_input_bytes - 1) as u64));
    let refusal = command::replay_with_limits(&request_path, run_limits, &caller)
        .expect_err("one byte below replay.input_bytes refuses");
    let detail = format!("{refusal:?}");
    assert!(detail.contains("replay.input_bytes"), "{detail}");
    assert!(
        detail.contains(&(exact_input_bytes - 1).to_string()),
        "{detail}"
    );
    assert!(detail.contains(&exact_input_bytes.to_string()), "{detail}");
    row
}

fn timed_command_replay_with_exact_input_bound(
    wire: &ReplayRequestWire,
) -> (Value, u128, u128, u128, usize) {
    let setup_started = Instant::now();
    let envelope = request_file_value(wire);
    let bytes = serde_json::to_vec(&envelope).expect("request envelope serializes");
    let directory = tempfile::tempdir().expect("replay workspace");
    let request_path = directory.path().join("request.json");
    std::fs::write(&request_path, &bytes).expect("request file writes");
    std::fs::write(
        directory.path().join("unit.native"),
        &wire.byte_provision[0].2,
    )
    .expect("source provision writes");
    let setup_ms = setup_started.elapsed().as_millis();

    let decode_started = Instant::now();
    let exact_input_bytes =
        match ReplayRequest::decode(wire.clone(), ReplayLimits::default().with_input_bytes(0)) {
            Err(ReplayRequestRefusal::BoundExceeded(limit)) => {
                usize::try_from(limit.actual).expect("measured replay bytes fit usize")
            }
            Err(other) => panic!("unexpected zero-bound replay request result: {other:?}"),
            Ok(_) => panic!("zero-byte replay input bound admitted the request"),
        };
    let decode_ms = decode_started.elapsed().as_millis();

    let mut caller = CommandLimits::replay();
    assert!(caller.set_bound(Setting::ReplayInputBytes, exact_input_bytes as u64));
    let run_limits = RunLimits {
        request_bytes: 1 << 30,
        dependent_bytes: 1 << 30,
    };
    let exact_started = Instant::now();
    let result = command::replay_with_limits(&request_path, run_limits, &caller)
        .expect("command replay admits the exact replay-input byte count");
    let exact_ms = exact_started.elapsed().as_millis();
    assert_eq!(result.exit_code, 0);
    let row = result.value.as_value()["results"][0].clone();

    assert!(caller.set_bound(Setting::ReplayInputBytes, (exact_input_bytes - 1) as u64));
    let refusal = command::replay_with_limits(&request_path, run_limits, &caller)
        .expect_err("one byte below replay.input_bytes refuses");
    let detail = format!("{refusal:?}");
    assert!(detail.contains("replay.input_bytes"), "{detail}");
    assert!(
        detail.contains(&(exact_input_bytes - 1).to_string()),
        "{detail}"
    );
    assert!(detail.contains(&exact_input_bytes.to_string()), "{detail}");
    (row, setup_ms, decode_ms, exact_ms, exact_input_bytes)
}

fn set_s1_input_bytes_from_serialized_request(wire: &mut ReplayRequestWire) -> (usize, u64) {
    let reader_bytes =
        match ReplayRequest::decode(wire.clone(), ReplayLimits::default().with_input_bytes(0)) {
            Err(ReplayRequestRefusal::BoundExceeded(limit)) => {
                usize::try_from(limit.actual).expect("measured replay bytes fit usize")
            }
            Err(other) => panic!("unexpected zero-bound replay request result: {other:?}"),
            Ok(_) => panic!("zero-byte replay input bound admitted the request"),
        };
    let setting = Setting::S1InputBytes.name();
    let mut bound = u64::try_from(
        serde_json::to_vec(&request_file_value(wire))
            .expect("request envelope serializes")
            .len(),
    )
    .expect("serialized request size fits u64")
    .min(u64::try_from(reader_bytes).expect("reader byte count fits u64"));
    let final_request_bytes = loop {
        let (_, value) = wire
            .stage_limits
            .iter_mut()
            .find(|(name, _)| name.as_str() == setting)
            .expect("request carries the s1.input_bytes ceiling");
        *value = bound;
        let request_bytes =
            serde_json::to_vec(&request_file_value(wire)).expect("request envelope serializes");
        let next = u64::try_from(request_bytes.len())
            .expect("serialized request size fits u64")
            .min(u64::try_from(reader_bytes).expect("reader byte count fits u64"));
        if next == bound {
            break request_bytes.len();
        }
        bound = next;
    };
    (final_request_bytes, bound)
}

fn profile_native_size_replay(count: usize) {
    let values = (0..count)
        .map(|value| value.to_string())
        .collect::<Vec<_>>()
        .join(",");
    let source = model_function(
        None,
        &format!(
            "let values: Sequence<Integer>[0, {count}] = sequence[{values}] in \
             size<Integer>(values) = {count}"
        ),
        "Boolean",
    );
    let mut limits = fsl_semantics::model::accounting::ModelNormalizationLimits::default();
    limits.intake = limits.intake.with_sequence_maximum(count as u32);
    let compile_started = Instant::now();
    let (package, site, source_tokens, phases) = package_bytes(&source, &empty_packages(), limits);
    let compile_ms = compile_started.elapsed().as_millis();
    let mut wire = request_wire(&package, &source, &site);
    let (request_json_bytes, s1_input_limit) =
        set_s1_input_bytes_from_serialized_request(&mut wire);
    let (result, setup_ms, decode_ms, exact_ms, input_bytes) =
        timed_command_replay_with_exact_input_bound(&wire);

    assert_eq!(result["arm"], "input");
    assert_eq!(result["settlement"], "reproduced-without-witness");
    assert_eq!(result["category"], "success");
    assert_eq!(result["value"], true);
    eprintln!(
        "QSL3570_NATIVE_REPLAY_PHASE_PROFILE values={count} source_tokens={source_tokens} source_bytes={} package_bytes={} request_json_bytes={request_json_bytes} s1_input_bytes={s1_input_limit} replay_input_bytes={input_bytes} compile_total_ms={compile_ms} parse_forms_ms={} model_select_ms={} check_ms={} package_emit_ms={} s4_pre_lock_graph_temporal_us={} s4_lock_us={} s4_inspection_us={} s4_index_us={} s4_producer_temporal_claim_term_us={} s4_source_requirement_operand_us={} s4_final_encode_us={} s4_retained_summary_us={} s4_retained_disposition_us={} s4_retained_emission_assembly_us={} s4_retained_identity_selection_us={} s4_retained_post_encode_us={} graph_nodes={} exact_declarations={} exact_fields={} keep_nodes={} document_nodes={} setup_ms={setup_ms} decode_ms={decode_ms} command_exact_ms={exact_ms}",
        source.len(),
        package.len(),
        phases.parse_forms_ms,
        phases.model_select_ms,
        phases.check_ms,
        phases.package_emit_ms,
        phases.emitter.pre_lock_graph_temporal_us,
        phases.emitter.lock_us,
        phases.emitter.inspection_us,
        phases.emitter.index_us,
        phases.emitter.producer_temporal_claim_term_us,
        phases.emitter.source_requirement_operand_us,
        phases.emitter.final_encode_us,
        phases.emitter.retained_summary_us,
        phases.emitter.retained_disposition_us,
        phases.emitter.retained_emission_assembly_us,
        phases.emitter.retained_identity_selection_us,
        phases.emitter.retained_post_encode_us,
        phases.retained_counts.graph_node_count,
        phases.retained_counts.exact_declaration_count,
        phases.retained_counts.exact_field_count,
        phases.retained_counts.keep_nodes,
        phases.retained_counts.document_node_count,
    );
}

fn model_function(model_identity: Option<&str>, expression: &str, result_type: &str) -> String {
    model_function_with_declarations(model_identity, "", expression, result_type)
}

fn model_function_with_declarations(
    model_identity: Option<&str>,
    declarations: &str,
    expression: &str,
    result_type: &str,
) -> String {
    let model = model_identity.map_or_else(String::new, |identity| {
        format!("model M = \"{identity}\";\n")
    });
    format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.value.complete/v1\";\n\
         {model}{declarations}\nfunction ready using v(): {result_type} pure {{ {expression} }}\n"
    )
}

fn empty_packages() -> fsl_semantics::model::intake::PackageInput {
    fsl_semantics::model::intake::package_input(std::iter::empty::<&[u8]>())
        .expect("the empty model closure admits")
}

/// FR-862-AC-5: the 10,000-term request-file replay matches the in-process
/// result, and its exact replay input count refuses one byte below.
#[trace("TC-3242", "FR-862-AC-5")]
#[test]
fn command_replays_the_ten_thousand_term_sum_with_exact_input_bytes() {
    let sum = std::iter::repeat_n("1", 10_000)
        .collect::<Vec<_>>()
        .join(" + ");
    let source = model_function(None, &format!("({sum}) = 10000"), "Boolean");
    let (package, site, _, _) = package_bytes(
        &source,
        &empty_packages(),
        fsl_semantics::model::accounting::ModelNormalizationLimits::default(),
    );
    assert_command_replay(request_wire(&package, &source, &site));
}

fn enum_declarations() -> String {
    let mut declarations = String::new();
    for index in 0..1_000 {
        declarations.push_str(&format!("enum E{index:04} {{ A, B }}\n"));
    }
    declarations
}

/// FR-862-AC-6: a replay request carrying 1,000 enum declarations returns
/// the same reproduced result as in-process replay.
#[trace("TC-3242", "FR-862-AC-6")]
#[test]
fn command_replays_one_thousand_enum_declarations() {
    let source = model_function_with_declarations(None, &enum_declarations(), "true", "Boolean");
    let limits = fsl_semantics::model::accounting::ModelNormalizationLimits::default()
        .with_declaration_records(3_000);
    let (package, site, _, _) = package_bytes(&source, &empty_packages(), limits);
    assert_command_replay(request_wire(&package, &source, &site));
}

fn link_declarations(redefines: bool) -> String {
    let mut declarations = String::new();
    for index in 0_u64..=1_000 {
        let name = format!("Link{index:04}");
        let kind = if index == 1_000 {
            "object"
        } else {
            "abstract object"
        };
        let parent = index.checked_sub(1).map(|value| format!("Link{value:04}"));
        let extends = parent
            .as_ref()
            .map_or_else(String::new, |parent| format!(" extends {parent}"));
        let operation = if redefines {
            let clause = parent
                .as_ref()
                .map_or_else(String::new, |parent| format!(" redefines {parent}::query"));
            format!(" operation query(): Integer{clause};")
        } else {
            String::new()
        };
        declarations.push_str(&format!("{kind} {name}{extends} {{{operation} }}\n"));
    }
    declarations
}

/// FR-862-AC-7: both valid 1,000-link fixtures replay to their in-process
/// values, with distinct inheritance and operation-redefinition packages.
#[trace("TC-3242", "FR-862-AC-7")]
#[test]
fn command_replays_both_thousand_link_variants() {
    for redefines in [false, true] {
        let declarations = link_declarations(redefines);
        let source = model_function_with_declarations(None, &declarations, "true", "Boolean");
        let limits = fsl_semantics::model::accounting::ModelNormalizationLimits::default()
            .with_ancestor_steps(1_000)
            .with_derivation_facts(1_504_502);
        let (package, site, _, _) = package_bytes(&source, &empty_packages(), limits);
        assert_command_replay(request_wire(&package, &source, &site));
    }
}

/// FR-862-AC-8: the 100,000-value replay returns its named semantic result
/// and applies the independent exact/one-under replay.input_bytes boundary.
#[trace("TC-3242", "FR-862-AC-8")]
#[test]
fn command_replays_one_hundred_thousand_ordered_values() {
    const COUNT: usize = 100_000;
    let values = (0..COUNT)
        .map(|value| value.to_string())
        .collect::<Vec<_>>()
        .join(",");
    let source = model_function(
        None,
        &format!(
            "let values: Sequence<Integer>[0, {COUNT}] = sequence[{values}] in \
             size<Integer>(values) = {COUNT}"
        ),
        "Boolean",
    );
    let mut limits = fsl_semantics::model::accounting::ModelNormalizationLimits::default();
    limits.intake = limits.intake.with_sequence_maximum(COUNT as u32);
    let (package, site, _, _) = package_bytes(&source, &empty_packages(), limits);
    let result = command_replay_with_exact_input_bound(&request_wire(&package, &source, &site));
    assert_eq!(result["arm"], "input");
    assert_eq!(result["settlement"], "reproduced-without-witness");
    assert_eq!(result["category"], "success");
    assert_eq!(result["value"], true);
}

#[trace("TC-3242", "FR-862-AC-8")]
#[test]
fn profile_native_replay_at_ten_thousand_values() {
    profile_native_size_replay(10_000);
}

#[trace("TC-3242", "FR-862-AC-8")]
#[test]
fn profile_native_replay_at_twenty_thousand_values() {
    profile_native_size_replay(20_000);
}

#[trace("TC-3242", "FR-862-AC-8")]
#[test]
fn profile_native_replay_at_forty_thousand_values() {
    profile_native_size_replay(40_000);
}

/// FR-862-AC-8: profile S4 emission subphases while preserving the 20,000-value
/// native command replay result and exact/one-under replay-input checks.
#[trace("TC-3242", "FR-862-AC-8")]
#[test]
fn profile_s4_emission_at_twenty_thousand_values() {
    profile_native_size_replay(20_000);
}

/// FR-862-AC-8: profile S4 emission subphases while preserving the 40,000-value
/// native command replay result and exact/one-under replay-input checks.
#[trace("TC-3242", "FR-862-AC-8")]
#[test]
fn profile_s4_emission_at_forty_thousand_values() {
    profile_native_size_replay(40_000);
}
