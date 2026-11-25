// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL-2097 measured-work regressions for model intake.

use fsl_foundation::selection::ModelSelection;
use fsl_foundation::Span;
use fsl_semantics::model::accounting::{ModelNormalizationLimits, NormalizationLimitKind};
use fsl_semantics::model::intake::{admit_unit, meaning, PackageInput};
use ix_trace_rs::trace;
use serde_json::Value;
use std::process::Command;
use std::time::Instant;

fn wire_field(identity: &str) -> Value {
    serde_json::json!({
        "identity": format!("{identity}/f"), "name": "f",
        "typeRef": "ix://filament/native/Boolean", "presence": "optional",
        "nullable": false, "defaultKind": "none",
        "multiplicity": {"lower": 0, "ordered": false, "unique": true},
        "origin": {"generated": {"generatorIdentity": identity,
            "generatorVersion": "1.0.0", "inputIdentities": [identity]}}
    })
}

fn scale_document(package: &str, count: usize) -> Vec<u8> {
    let types: Vec<Value> = (0..count)
        .map(|index| {
            let identity = format!("ix://{package}/R{index:05}");
            serde_json::json!({
                "identity": identity, "displayName": identity,
                "kind": {"module": package, "name": "record_value_type"},
                "roles": [], "origin": {"generated": {
                    "generatorIdentity": identity, "generatorVersion": "1.0.0",
                    "inputIdentities": [identity]}},
                "constraints": [], "extensions": [], "unknownPolicy": "reject",
                "supertypes": [], "fields": [wire_field(&identity)]
            })
        })
        .collect();
    let document = serde_json::json!({
        "contractVersion": "2.0.0", "canonicalization": {"algorithm": "rfc8785-v1"},
        "source": {"identity": format!("ix://{package}/spec"), "version": "1.0.0", "dialect": "spec-bundle"},
        "package": {"identity": package, "version": "1.0.0", "mappingVersions": [], "profileVersions": []},
        "occurrences": [], "extensions": [],
        "constructs": [{"kind": {"module": package, "name": "record_value_type"},
            "moduleVersion": "1.0.0", "construct": {"identity": "none", "shape": "record",
            "members": {}, "meaning": meaning::RECORD_VALUE_TYPE}}],
        "types": types
    });
    serde_json::to_vec(&document).expect("serialize scale input")
}

/// The input has N record-value declarations and N owned fields. Each
/// fourfold increase must remain within the specified fivefold work ceiling.
#[test]
#[trace("NFR-012-M-11", "TC-434")]
fn unit_intake_projection_work_scales_with_declarations_and_fields() {
    let package = "test/qsl2097-unit-intake-scale";
    let selection = ModelSelection {
        alias: "M".to_owned(),
        identity: package.to_owned(),
        span: Span { start: 0, end: 0 },
        identity_span: Span { start: 0, end: 0 },
    };
    let mut charged = Vec::new();
    for count in [1_000, 4_000, 16_000] {
        let bytes = scale_document(package, count);
        let packages = PackageInput::from([(package.to_owned(), bytes)]);
        let selected = admit_unit(
            std::slice::from_ref(&selection),
            &packages,
            ModelNormalizationLimits::default(),
        )
        .unwrap_or_else(|error| panic!("{count} declaration intake failed: {error:?}"));
        assert_eq!(selected.len(), 1);
        assert_eq!(selected.exact_field_rows().len(), count);
        charged.push(selected.consumed(NormalizationLimitKind::WorkUnits));
    }
    assert!(charged[1] <= 5 * charged[0], "work {charged:?}");
    assert!(charged[2] <= 5 * charged[1], "work {charged:?}");
}

#[test]
fn native_parser_formatter_probe() {
    let source = fsl_foundation::SourceIdentity::new("test", "native-without-runtimes");
    let bytes = include_bytes!("../fixtures/spine-compile.native");
    let parsed = fsl_cst::parse(
        source.clone(),
        "unit.native",
        bytes,
        fsl_cst::Limits::default(),
    )
    .expect("native parser accepts its fixture");
    let formatted = fsl_cst::format::format_with_limit(&parsed, usize::MAX)
        .expect("native formatter formats the parsed fixture");
    let reparsed = fsl_cst::parse(
        source,
        "formatted.native",
        formatted.as_bytes(),
        fsl_cst::Limits::default(),
    )
    .expect("formatted native source parses");
    assert!(reparsed.is_admissible());
}

/// Execute the real native parse/format control in a child with an empty
/// executable search path, so Node and JVM cannot satisfy the operation.
#[test]
#[trace("NFR-002-M-2")]
fn native_parse_and_format_run_without_node_or_jvm_on_path() {
    let empty_path = tempfile::tempdir().expect("empty runtime path");
    let output = Command::new(std::env::current_exe().expect("test binary path"))
        .args([
            "--exact",
            "qsl2097_measurements::native_parser_formatter_probe",
            "--nocapture",
        ])
        .env("PATH", empty_path.path())
        .output()
        .expect("run Rust-only parser/formatter probe");
    let report = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "native probe failed without external runtimes: {report}{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        report.contains("test result: ok. 1 passed"),
        "the child ran exactly the probe: {report}"
    );
}

fn complete_v1_parse_and_forms(count: usize) -> std::time::Duration {
    use std::fmt::Write as _;

    let mut source = String::from(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.value.complete/v1\";\n\
         type Digit = Int[0, 9];\n",
    );
    for index in 0..count {
        writeln!(source, "function f{index} using v(): Digit pure {{ 7 }}")
            .expect("write generated declaration");
    }
    let limits = fsl_cst::Limits {
        source_bytes: usize::MAX,
        tokens: 10_000_000,
        nodes: 10_000_000,
        work_units: usize::MAX,
        diagnostics: 100,
    };
    let identity = fsl_foundation::SourceIdentity::new("test", "nfr001-syntax-scaling");
    let started = Instant::now();
    let parsed = fsl_cst::parse(identity, "scale.native", source.as_bytes(), limits)
        .expect("raised S1 limits admit the generated declarations");
    assert!(parsed.is_admissible());
    fsl_forms::build_unit(&parsed).expect("S2 builds all generated declarations");
    started.elapsed()
}

/// S1+S2 work is measured on the literal 2,000 and 200,000 declaration
/// populations; per-declaration time at the larger population stays within
/// the authored 2x threshold.
#[test]
#[trace("NFR-001-M-5")]
fn parser_and_forms_per_declaration_time_stays_bounded_at_200k() {
    let small = complete_v1_parse_and_forms(2_000);
    let large = complete_v1_parse_and_forms(200_000);
    let small_per_decl = small.as_nanos() / 2_000;
    let large_per_decl = large.as_nanos() / 200_000;
    eprintln!("nfr001-syntax-scaling small={small:?} large={large:?}");
    assert!(
        large_per_decl <= small_per_decl * 2,
        "per-declaration time small={small_per_decl}ns large={large_per_decl}ns"
    );
}

/// The same public generator the libFuzzer target uses drives 10,000 complete
/// S1-to-S4 attempts. The deterministic population includes every shape,
/// shallow depths throughout the range, and one 100,000-level input.
#[test]
#[trace("FR-356-AC-7")]
fn deep_input_generator_returns_outcomes_for_ten_thousand_inputs() {
    for index in 0..9_999_u32 {
        let mut bytes = [0_u8; 5];
        bytes[0] = (index % 240) as u8;
        bytes[1..].copy_from_slice(&(index % 64).to_le_bytes());
        let input = fsl_bench::deep_input::DeepInput::from_bytes(&bytes);
        assert!((1..=fsl_bench::deep_input::MAX_DEPTH).contains(&input.depth()));
        let _outcome = input.compile();
    }
    let maximal = fsl_bench::deep_input::DeepInput::from_bytes(&[255, 0x9f, 0x86, 0x01, 0x00]);
    assert_eq!(maximal.depth(), fsl_bench::deep_input::MAX_DEPTH);
    let _outcome = maximal.compile();
}
