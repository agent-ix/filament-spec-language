// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-393's timed-interval values and stage boundaries through public APIs.

use std::collections::BTreeMap;

use filament_exact::Cancel;
use fsl_forms::{
    build_unit, DeclarationForm, ExactValueKind, OperatorInterval, TemporalNodeForm,
    TimeSourceSyntax, TimedUpper,
};
use fsl_foundation::{SourceIdentity, Span};
use fsl_replay::spine::{
    check_with_execution_points_and_source_inputs, parse, select, CompileRefusal, DependencyInput,
    ParseRequest, SourceInputs, SpineLimits,
};
use fsl_semantics::check::{CheckCause, CheckedInterval, TemporalCause};
use fsl_semantics::model::accounting::ModelNormalizationLimits;
use fsl_semantics::model::intake::package_input;
use ix_trace_rs::trace;

const MODEL: &[u8] = br#"language "ix:native" edition "1-draft";
domain "acme/timed-counter";
dimension Time;
unit s : Time = rational(1, 1);
object Counter identity (id) { id: Int[0,3]; }
population counters: Counter;
"#;

const PREFIX: &str = "language \"ix:native\" edition \"1-draft\";\n\
profile v = \"filament.model.complete/v1\";\n\
profile tc = \"filament.temporal.complete/v1\";\n\
profile t = \"filament.temporal.timed/v1\";\n\
model M = \"acme/timed-counter\";\n\
dimension Time;\n\
unit s : Time = rational(1, 1);\n\
unit ms : Time = rational(1, 1000) * s;\n\
time M dense unit ms;\n";

fn source(formula: &str) -> String {
    format!(
        "{PREFIX}temporal T using t over (c: M::Counter) clock \"model-time\" on origin {{ {formula} }}\n"
    )
}

fn forms(text: &str) -> fsl_forms::ParsedUnit {
    let parsed = fsl_forms::cst::parse(
        SourceIdentity::new("agent-ix", "test:qsl-2094-fr393-ac4"),
        "unit.native",
        text.as_bytes(),
        fsl_forms::cst::Limits::default(),
    )
    .expect("S1 reads the source");
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    build_unit(&parsed).expect("S2 builds the forms")
}

fn model_bytes() -> Vec<u8> {
    let base = package_input([MODEL]).expect("the base domain model admits");
    let mut document: serde_json::Value =
        serde_json::from_slice(&base["acme/timed-counter"]).expect("model JSON");
    let mut milliseconds = document["units"][0].clone();
    milliseconds["identity"] = serde_json::json!("ix://acme/timed-counter/unit/ms");
    milliseconds["name"] = serde_json::json!("ms");
    milliseconds["target"] = serde_json::json!("ix://acme/timed-counter/unit/s");
    milliseconds["scale"] = serde_json::json!("1/1000");
    document["units"]
        .as_array_mut()
        .expect("units are an array")
        .push(milliseconds);
    document.to_string().into_bytes()
}

fn s3_result(text: &str) -> Result<fsl_replay::spine::CheckedUnit, Box<CompileRefusal>> {
    let cancel = Cancel::new();
    let source = SourceIdentity::new("agent-ix", "test:qsl-2094-fr393-ac4");
    let parsed = parse(
        &ParseRequest {
            source: &source,
            path: "unit.native",
            bytes: text.as_bytes(),
        },
        fsl_cst::Limits::default(),
        &cancel,
    )
    .expect("S1 and S2 admit the source")
    .into_value();
    let bytes = model_bytes();
    let packages = package_input([bytes.as_slice()]).expect("the model fixture admits");
    let models = select(
        &parsed,
        &packages,
        ModelNormalizationLimits::default(),
        &cancel,
    )
    .expect("I1 selects the domain model")
    .into_value();
    check_with_execution_points_and_source_inputs(
        &parsed,
        "acme/timed-claims".to_owned(),
        &BTreeMap::new(),
        SourceInputs::default(),
        &models,
        &DependencyInput::default(),
        SpineLimits::default(),
        &cancel,
    )
    .map(|checked| checked.into_value())
    .map_err(|failure| failure.into_refused().expect("the result is a refusal"))
}

fn s3_refusal(text: &str) -> CompileRefusal {
    *s3_result(text).expect_err("S3 refuses the invalid timed form")
}

fn interval(text: &str) -> fsl_forms::TemporalOperatorForm {
    let unit = forms(text);
    let form = unit
        .forms()
        .iter()
        .find(|form| matches!(form.form(), DeclarationForm::Temporal(_)))
        .expect("the temporal declaration");
    let DeclarationForm::Temporal(clause) = form.form() else {
        unreachable!("the selected form is temporal");
    };
    let Some(TemporalNodeForm::Operator(operator)) = clause.formula.node(clause.formula.root())
    else {
        panic!("the temporal formula root is an operator");
    };
    operator.clone()
}

fn source_span(text: &str, span: Span) -> &str {
    &text[span.start..span.end]
}

#[trace("TC-1078", "FR-393-AC-4")]
#[test]
fn decimal_timed_magnitude_builds_and_invalid_meanings_refuse_only_at_s3() {
    let positive = source("eventually[0 s, 0.003 s] holds(true)");
    let operator = interval(&positive);
    let Some(OperatorInterval::Timed(timed)) = operator.interval else {
        panic!("a timed interval");
    };
    let TimedUpper::Finite(upper) = timed.upper else {
        panic!("a finite upper bound");
    };
    assert_eq!(upper.magnitude.kind, ExactValueKind::DecimalLiteral);
    assert_eq!(upper.magnitude.first.to_string(), "3");
    assert_eq!(upper.magnitude.second.to_string(), "3");
    assert_eq!(upper.unit.name, "s");
    assert_eq!(source_span(&positive, upper.span), "0.003 s");
    let checked = s3_result(&positive).expect("the positive interval checks");
    let [clause] = checked.package().graph().temporal_clauses() else {
        panic!("one checked temporal clause");
    };
    let intervals: Vec<_> = clause
        .clause()
        .operators()
        .iter()
        .filter_map(|operator| match &operator.interval {
            Some(CheckedInterval::Timed(interval)) => Some(interval),
            _ => None,
        })
        .collect();
    let [checked_interval] = intervals.as_slice() else {
        panic!("one checked timed interval: {intervals:?}");
    };
    let expected = filament_exact::Rational::new(
        filament_exact::Integer::from(3_u64),
        filament_exact::Integer::from(1_u64),
    )
    .expect("the denominator is nonzero");
    assert_eq!(checked_interval.upper(), Some(&expected));

    for (formula, expected_reason) in [
        (
            "eventually(2 ms, 2 ms] holds(true)",
            fsl_foundation::bound::InvalidTimedInterval::EmptyOpenEnd,
        ),
        (
            "eventually[3 ms, 2 ms] holds(true)",
            fsl_foundation::bound::InvalidTimedInterval::Inverted,
        ),
    ] {
        let text = source(formula);
        let operator = interval(&text);
        assert!(matches!(
            operator.interval,
            Some(OperatorInterval::Timed(_))
        ));
        let CompileRefusal::Check { refusals, .. } = s3_refusal(&text) else {
            panic!("the interval is refused by S3: {formula}");
        };
        assert!(
            matches!(
                refusals.as_slice(),
                [refusal]
                    if matches!(
                        &refusal.cause,
                        CheckCause::Temporal(cause)
                            if matches!(
                                **cause,
                                TemporalCause::InvalidTimedInterval { reason, .. }
                                    if reason == expected_reason
                            )
                    )
            ),
            "{formula}: {refusals:?}"
        );
    }

    let duplicate = format!("{PREFIX}time M dense unit s;\n");
    let built = forms(&duplicate);
    let time_sources: Vec<_> = built
        .forms()
        .iter()
        .filter_map(|form| match form.form() {
            DeclarationForm::TimeSource(source) => Some(source),
            _ => None,
        })
        .collect();
    assert_eq!(time_sources.len(), 2);
    assert!(time_sources
        .iter()
        .all(|source| source.source == TimeSourceSyntax::Dense));
    let CompileRefusal::Check { refusals, .. } = s3_refusal(&duplicate) else {
        panic!("the duplicate time source is refused by S3");
    };
    assert!(
        matches!(
            refusals.as_slice(),
            [refusal]
                if matches!(
                    &refusal.cause,
                    CheckCause::Temporal(cause) if matches!(**cause, TemporalCause::DuplicateTimeSource { .. })
                )
        ),
        "{refusals:?}"
    );

    let malformed = source("eventually[0 ms, x] holds(true)");
    let parsed = fsl_forms::cst::parse(
        SourceIdentity::new("agent-ix", "test:qsl-2094-fr393-ac4"),
        "unit.native",
        malformed.as_bytes(),
        fsl_forms::cst::Limits::default(),
    )
    .expect("S1 completes with a diagnostic");
    assert!(!parsed.is_admissible(), "the upper bound token is invalid");
    let diagnostic = parsed.diagnostics()[0]
        .byte_span()
        .expect("the parse diagnostic has a byte span");
    let x = malformed.find(", x]").expect("the malformed upper bound") + 2;
    assert_eq!(source_span(&malformed, diagnostic), "x");
    assert_eq!(
        diagnostic,
        Span {
            start: x,
            end: x + 1
        }
    );
    assert!(
        build_unit(&parsed).is_err(),
        "S2 builds no form after S1 refusal"
    );
}
