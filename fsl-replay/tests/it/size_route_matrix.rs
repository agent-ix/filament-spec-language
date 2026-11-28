// SPDX-License-Identifier: AGPL-3.0-or-later
//! Public S1/I1/S3 check-and-lower cells for TC-3242.

use filament_exact::Cancel;
use filament_semantic_value::checking::CheckingLimits;
use fsl_foundation::diagnostic::{LimitKind, StageFailure};
use fsl_foundation::Setting;
use fsl_replay::spine::{self, ParseRequest, SpineLimits};
use fsl_replay::{DependencyInput, SourceIdentity};
use ix_trace_rs::trace;

const SOURCE_LIMIT: usize = 2_000_000;
const S3_WORK_LIMIT: u64 = 1 << 42;

fn source_limits() -> fsl_cst::Limits {
    fsl_cst::Limits {
        source_bytes: 1 << 28,
        tokens: SOURCE_LIMIT,
        nodes: SOURCE_LIMIT,
        work_units: 1 << 42,
        diagnostics: 100,
    }
}

fn parse_and_select(
    source_text: &str,
    packages: &fsl_semantics::model::intake::PackageInput,
    model_limits: fsl_semantics::model::accounting::ModelNormalizationLimits,
) -> (spine::ParsedUnitSource, spine::AdmittedModels) {
    let identity = SourceIdentity::new("agent-ix", "test:qsl-3570-check-matrix");
    let parsed = spine::parse(
        &ParseRequest {
            source: &identity,
            path: "matrix.native",
            bytes: source_text.as_bytes(),
        },
        source_limits(),
        &Cancel::new(),
    )
    .expect("the matrix source parses")
    .into_value();
    let models = spine::select(&parsed, packages, model_limits, &Cancel::new())
        .expect("the selected model closure admits")
        .into_value();
    (parsed, models)
}

fn checking_limits(nodes: u64) -> SpineLimits {
    SpineLimits {
        source: source_limits(),
        checking: CheckingLimits::new(nodes)
            .with_input_bytes(1 << 40)
            .with_work_budget(S3_WORK_LIMIT),
        ..SpineLimits::default()
    }
}

fn assert_s3_boundary(parsed: &spine::ParsedUnitSource, models: &spine::AdmittedModels) {
    let check = |nodes| {
        spine::check(
            parsed,
            "qsl-3570".to_owned(),
            models,
            &DependencyInput::default(),
            checking_limits(nodes),
            &Cancel::new(),
        )
    };
    let mut low = 0_u64;
    let mut high = 1_u64;
    loop {
        match check(high) {
            Ok(_) => break,
            Err(StageFailure::Limit(limit)) if limit.setting() == Setting::S3Nodes => {
                low = high;
                high = high
                    .checked_mul(2)
                    .expect("bounded S3 search remains in u64");
            }
            Err(other) => panic!("unexpected refusal while measuring s3.nodes: {other:?}"),
        }
    }
    while low + 1 < high {
        let middle = low + (high - low) / 2;
        match check(middle) {
            Ok(_) => high = middle,
            Err(StageFailure::Limit(limit)) if limit.setting() == Setting::S3Nodes => low = middle,
            Err(other) => panic!("unexpected refusal while measuring s3.nodes: {other:?}"),
        }
    }
    let count = high;
    check(count).expect("the exact S3 node count admits");
    let limit = match check(count - 1) {
        Err(StageFailure::Limit(limit)) => limit,
        Err(other) => panic!("one below s3.nodes returned another refusal: {other:?}"),
        Ok(_) => panic!("one below s3.nodes admitted the checked source"),
    };
    assert_eq!(limit.setting(), Setting::S3Nodes);
    assert_eq!(limit.kind(), LimitKind::NodeCount);
    assert_eq!(limit.configured_bound(), count - 1);
    assert_eq!(limit.actual(), u128::from(count));
}

fn empty_packages() -> fsl_semantics::model::intake::PackageInput {
    fsl_semantics::model::intake::package_input(std::iter::empty::<&[u8]>())
        .expect("the empty package closure admits")
}

fn labeled_model_source(source_text: &str) -> fsl_foundation::Source {
    fsl_cst::diagnostic::read_source(
        SourceIdentity::new("agent-ix", "test:qsl-3570-model-source"),
        "model.fsl",
        source_text.as_bytes(),
        1 << 28,
    )
    .expect("the labeled model source reads")
}

fn simple_function_source(expression: &str, result_type: &str) -> String {
    format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.value.complete/v1\";\n\
         function ready using v(): {result_type} pure {{ {expression} }}\n"
    )
}

/// FR-277-AC-8: the exact entered S3 node count for the 1,000-level source
/// admits, and the adjacent lower bound refuses with `s3.nodes`.
#[trace("TC-3242", "FR-277-AC-8")]
#[test]
fn check_lower_admits_the_thousand_nested_expression_at_exact_s3_nodes() {
    let source = simple_function_source(
        &format!("{}true{}", "not (".repeat(1_000), ")".repeat(1_000)),
        "Boolean",
    );
    let (parsed, models) = parse_and_select(
        &source,
        &empty_packages(),
        fsl_semantics::model::accounting::ModelNormalizationLimits::default(),
    );
    assert_s3_boundary(&parsed, &models);
}

/// FR-277-AC-9: the left-associated 10,000-term sum uses its counted S3 node
/// bound and refuses exactly one node below it.
#[trace("TC-3242", "FR-277-AC-9")]
#[test]
fn check_lower_admits_the_ten_thousand_term_sum_at_exact_s3_nodes() {
    let sum = std::iter::repeat_n("1", 10_000)
        .collect::<Vec<_>>()
        .join(" + ");
    let source = simple_function_source(&sum, "Integer");
    let (parsed, models) = parse_and_select(
        &source,
        &empty_packages(),
        fsl_semantics::model::accounting::ModelNormalizationLimits::default(),
    );
    assert_s3_boundary(&parsed, &models);
}

fn enum_model() -> String {
    let mut model = String::from(
        "language \"ix:native\" edition \"1-draft\";\n\
         domain \"test/qsl-3570-enums\";\n",
    );
    for index in 0..1_000 {
        model.push_str(&format!("enum E{index:04} {{ A, B }}\n"));
    }
    model
}

fn link_model(redefines: bool) -> String {
    let mut model = String::from(
        "language \"ix:native\" edition \"1-draft\";\ndomain \"test/qsl-3570-links\";\n",
    );
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
            .map_or_else(String::new, |value| format!(" extends {value}"));
        let operation = if redefines {
            let clause = parent
                .as_ref()
                .map_or_else(String::new, |value| format!(" redefines {value}::query"));
            format!(" operation query(): Integer{clause};")
        } else {
            String::new()
        };
        model.push_str(&format!("{kind} {name}{extends} {{{operation} }}\n"));
    }
    model
}

fn model_select_source(identity: &str) -> String {
    format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.value.complete/v1\";\n\
         model M = \"{identity}\";\n\
         function ready using v(): Boolean pure {{ true }}\n"
    )
}

/// FR-277-AC-10: 1,000 enum declarations charge 3,000 declaration records;
/// the one-under selection refuses at I1, while an independent exact/one-under
/// S3 node pair retains its own `LimitExceeded` result.
#[trace("TC-3242", "FR-277-AC-10")]
#[test]
fn check_lower_keeps_enum_intake_and_s3_nodes_independent() {
    let model_source = enum_model();
    let labeled_source = labeled_model_source(&model_source);
    let packages = fsl_semantics::model::intake::package_input_from_sources([(
        "test/qsl-3570-enums",
        &labeled_source,
    )])
    .expect("the enum model source compiles");
    let source = model_select_source("test/qsl-3570-enums");
    let model_limits = fsl_semantics::model::accounting::ModelNormalizationLimits::default()
        .with_declaration_records(3_000);
    let (parsed, models) = parse_and_select(&source, &packages, model_limits);
    assert_s3_boundary(&parsed, &models);

    let identity = SourceIdentity::new("agent-ix", "test:qsl-3570-check-matrix");
    let parsed = spine::parse(
        &ParseRequest {
            source: &identity,
            path: "matrix.native",
            bytes: source.as_bytes(),
        },
        source_limits(),
        &Cancel::new(),
    )
    .expect("the source parses")
    .into_value();
    let refusal = spine::select(
        &parsed,
        &packages,
        fsl_semantics::model::accounting::ModelNormalizationLimits::default()
            .with_declaration_records(2_999),
        &Cancel::new(),
    )
    .expect_err("2,999 records refuse the 3,000th declaration record");
    let detail = format!("{refusal:?}");
    assert!(detail.contains("DeclarationRecords"), "{detail}");
    assert!(detail.contains("2999"), "{detail}");
    assert!(detail.contains("3000"), "{detail}");
}

/// FR-277-AC-11: both 1,000-edge link shapes retain their exact intake
/// boundary; each accepted model then has an independent S3 exact/one-under
/// node boundary.
#[trace("TC-3242", "FR-277-AC-11")]
#[test]
fn check_lower_keeps_inheritance_and_redefinition_limits_independent() {
    for redefines in [false, true] {
        let model_source = link_model(redefines);
        let labeled_source = labeled_model_source(&model_source);
        let packages = fsl_semantics::model::intake::package_input_from_sources([(
            "test/qsl-3570-links",
            &labeled_source,
        )])
        .expect("the link model source compiles");
        let source = model_select_source("test/qsl-3570-links");
        let identity = SourceIdentity::new("agent-ix", "test:qsl-3570-check-matrix");
        let parsed = spine::parse(
            &ParseRequest {
                source: &identity,
                path: "matrix.native",
                bytes: source.as_bytes(),
            },
            source_limits(),
            &Cancel::new(),
        )
        .expect("the model-selection source parses")
        .into_value();
        let mut one_under = fsl_semantics::model::accounting::ModelNormalizationLimits::default();
        let (setting, bound, next) = if redefines {
            one_under = one_under.with_derivation_facts(1_504_501);
            ("DerivationFacts", 1_504_501, 1_504_502)
        } else {
            one_under = one_under.with_ancestor_steps(999);
            ("AncestorSteps", 999, 1)
        };
        let refusal = spine::select(&parsed, &packages, one_under, &Cancel::new())
            .expect_err("one below the selected model-link count refuses in I1");
        let detail = format!("{refusal:?}");
        assert!(detail.contains(setting), "{detail}");
        assert!(detail.contains(&bound.to_string()), "{detail}");
        assert!(detail.contains(&next.to_string()), "{detail}");
        let (parsed, models) = parse_and_select(
            &source,
            &packages,
            fsl_semantics::model::accounting::ModelNormalizationLimits::default()
                .with_ancestor_steps(1_000)
                .with_derivation_facts(1_504_502),
        );
        assert_s3_boundary(&parsed, &models);
    }
}

/// FR-277-AC-12: sequence intake's `model.sequence_maximum` refusal is
/// separate from S3's exact and one-under entered-node boundary.
#[trace("TC-3242", "FR-277-AC-12")]
#[test]
fn check_lower_keeps_hundred_thousand_sequence_intake_and_s3_limits_independent() {
    const COUNT: usize = 100_000;
    let values = (0..COUNT)
        .map(|value| value.to_string())
        .collect::<Vec<_>>()
        .join(",");
    let source = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.value.complete/v1\";\n\
         model M = \"example/config-version\";\n\
         function sequenceValue using v(): Sequence<Integer>[0, {COUNT}] pure {{ sequence[{values}] }}\n"
    );
    let mut model: serde_json::Value = serde_json::from_slice(include_bytes!(
        "../../../examples/config-version/model.semantic-ir.json"
    ))
    .expect("the checked-in semantic model is JSON");
    let config_version = model["types"]
        .as_array_mut()
        .expect("model types")
        .iter_mut()
        .find(|ty| ty["identity"] == "ix://example/config-version/ConfigVersion")
        .expect("ConfigVersion exists");
    config_version["fields"][0]["multiplicity"] = serde_json::json!({
        "lower": 0, "upper": COUNT, "ordered": true, "unique": false
    });
    let model_bytes = serde_json::to_vec(&model).expect("semantic model serializes");
    let packages = fsl_semantics::model::intake::package_input([model_bytes.as_slice()])
        .expect("the semantic model package reads");
    let identity = SourceIdentity::new("agent-ix", "test:qsl-3570-check-matrix");
    let parsed = spine::parse(
        &ParseRequest {
            source: &identity,
            path: "matrix.native",
            bytes: source.as_bytes(),
        },
        source_limits(),
        &Cancel::new(),
    )
    .expect("the 100,000-value source parses")
    .into_value();
    let default_refusal = spine::select(
        &parsed,
        &packages,
        fsl_semantics::model::accounting::ModelNormalizationLimits::default(),
        &Cancel::new(),
    )
    .expect_err("the default sequence maximum refuses 100,000 values before S3");
    let detail = format!("{default_refusal:?}");
    assert!(detail.contains("SequenceMaximum"), "{detail}");
    assert!(detail.contains("10000"), "{detail}");
    assert!(detail.contains("100000"), "{detail}");

    let mut raised = fsl_semantics::model::accounting::ModelNormalizationLimits::default();
    raised.intake = raised.intake.with_sequence_maximum(100_000);
    let (parsed, models) = parse_and_select(&source, &packages, raised);
    assert_s3_boundary(&parsed, &models);
}
