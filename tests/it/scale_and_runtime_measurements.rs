// SPDX-License-Identifier: AGPL-3.0-or-later
//! Measurements of model-intake work, the native build's runtime needs, and
//! front-end scaling over large and deeply nested sources.

use fsl_foundation::diagnostic::StageFailure;
use fsl_foundation::selection::ModelSelection;
use fsl_foundation::{Code, Span};
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
    let package = "test/unit-intake-scale";
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
            "scale_and_runtime_measurements::native_parser_formatter_probe",
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
#[ignore = "runs in make test-deep-measurements (part of make ci)"]
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

/// The selector byte `DeepInput::from_bytes` maps to the shape `shape` (of its
/// six) with depths up to the ceiling of the selector class starting at `base`.
fn selector(base: u8, shape: usize) -> u8 {
    (base..=u8::MAX)
        .find(|selector| usize::from(*selector) % 6 == shape)
        .expect("every class holds all six shapes")
}

/// The same public generator the libFuzzer target uses drives 10,000 complete
/// S1-to-S4 attempts. Their depths are spread evenly through three bands of
/// 1 to 10,000 levels, thinning as they deepen, and every band takes all six
/// shapes. Each input compiles to a unit or stops at a stated limit. No input
/// is deeper than 10,000 levels: emission memory grows with the square of a
/// sum's depth and passes 12 GB well before the generator's 100,000-level
/// ceiling. No mixed input nests both a sum and an `else if`:
/// emission refuses a sum inside an `else` branch, which
/// `a_sum_in_an_else_branch_compiles_or_stops_at_a_limit` reproduces.
#[test]
#[ignore = "runs in make test-deep-measurements (part of make ci)"]
fn deep_input_generator_returns_outcomes_for_ten_thousand_inputs() {
    // (first selector of the class, shallowest, deepest, inputs)
    let bands: [(u8, usize, usize, usize); 3] = [
        (0, 1, 100, 9_010),
        (160, 101, 1_000, 900),
        (208, 1_001, 10_000, 90),
    ];
    let mut inputs = 0;
    for (base, shallowest, deepest, count) in bands {
        let started = Instant::now();
        let (mut compiled, mut limited) = (0, 0);
        for index in 0..count {
            let depth = shallowest + index * (deepest - shallowest) / (count - 1).max(1);
            let raw = u32::try_from(depth - 1).expect("depth fits the generator's bytes");
            let mut bytes = vec![selector(base, index % 6)];
            bytes.extend_from_slice(&raw.to_le_bytes());
            // The mixed shape alternates between cycling sum, bracket and `let`
            // and cycling `else if`, bracket and `let`. A sum is followed by a
            // bracket, since the grammar takes no bare `if` or `let` as a sum's
            // right operand.
            let kinds: [u8; 3] = if (index / 6) % 2 == 0 {
                [1, 0, 3]
            } else {
                [2, 0, 3]
            };
            bytes.extend_from_slice(&kinds);
            let input = fsl_bench::deep_input::DeepInput::from_bytes(&bytes);
            assert_eq!(input.depth(), depth);
            match input.compile() {
                Ok(_) => compiled += 1,
                Err(StageFailure::Limit(_) | StageFailure::LimitWithPayload(..)) => limited += 1,
                Err(StageFailure::Refused(refusal))
                    if refusal.code() == Code::ResourceExhausted =>
                {
                    limited += 1;
                }
                Err(StageFailure::RefusedWithPayload(refusal, _))
                    if refusal.code() == Code::ResourceExhausted =>
                {
                    limited += 1;
                }
                Err(other) => panic!(
                    "depth {depth}, selector {}: neither a result nor a limit: {other:?}",
                    bytes[0]
                ),
            }
        }
        eprintln!(
            "deep-input band {shallowest}..={deepest}: compiled={compiled} limited={limited} \
             in {:?}",
            started.elapsed()
        );
        assert_eq!(compiled + limited, count);
        inputs += count;
    }
    assert_eq!(inputs, 10_000);
}

/// A conditional whose `else` branch adds two integers is a bounded, well-typed
/// unit, so S1 to S4 compile it or stop at a stated limit.
#[test]
#[ignore = "QSL-3283: emission refuses a sum inside an else branch with SourceMapInvariant"]
fn a_sum_in_an_else_branch_compiles_or_stops_at_a_limit() {
    use fsl_bench::deep_input::{DeepInput, Level, Shape};
    // `if true then x else x + x`
    let input = DeepInput::new(Shape::Expression(vec![Level::ElseIf, Level::Sum]));
    match input.compile() {
        Ok(_) | Err(StageFailure::Limit(_) | StageFailure::LimitWithPayload(..)) => {}
        Err(other) => panic!("neither a result nor a limit: {other:?}\n{}", input.source),
    }
}
