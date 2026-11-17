// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-982-AC-4 (TC-2777) and FSpec FR-784: a parenthesized window-aggregate
//! threshold is typed against the aggregate's value.
use filament_semantic_value::checking::{CheckMode, CheckingLimits};
use fsl_forms::build_unit;
use fsl_foundation::{Code, SourceIdentity};
use fsl_semantics::check::{CheckRefusal, CheckedGraph, PackageDeclarations};
use fsl_semantics::model::intake::SelectedModels;
use ix_trace_rs::trace;

/// S1 to S3 of `header` and `declarations` under `mode`; S1 and S2 must
/// accept the unit.
fn check_in(
    header: &str,
    declarations: &str,
    mode: CheckMode,
) -> Result<CheckedGraph, Vec<CheckRefusal>> {
    let text = format!("language \"ix:native\" edition \"1-draft\";\n{header}{declarations}\n");
    let parsed = fsl_cst::parse(
        SourceIdentity::new("test", "w17"),
        "w17.native",
        text.as_bytes(),
        fsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let unit = build_unit(&parsed).expect("S2 builds the unit");
    PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        crate::TEST_PACKAGE_NAME.to_owned(),
        unit,
        SelectedModels::default(),
        Vec::new(),
    )
    .unwrap_or_else(|refusal| panic!("assembly succeeds: {refusal:?}"))
    .check(mode, CheckingLimits::default())
}

/// The first refusal of `declarations` has `code` and `cause`.
fn assert_refused_with(
    header: &str,
    declarations: &str,
    code: Code,
    cause: Option<&str>,
) -> CheckRefusal {
    let refusals =
        check_in(header, declarations, CheckMode::Linked).expect_err("the unit is refused");
    let first = refusals.first().expect("a refusal").clone();
    assert_eq!(first.cause.code(), code, "{declarations}: {refusals:?}");
    if cause.is_some() {
        assert_eq!(first.cause.cause(), cause, "{declarations}: {refusals:?}");
    }
    first
}

const EVENTS: &str = "profile v = \"filament.value.complete/v1\";\n\
     profile ev = \"filament.temporal.event-position.false-extension/v1\";\n";

const TIMESTAMPED: &str = "profile v = \"filament.value.complete/v1\";\n\
     profile ev = \"filament.temporal.timestamped-event.finite-window/v1\";\n";

fn clause(formula: &str) -> String {
    format!(
        "record Reading {{ k: Int[0, 9]; ok: Boolean; done: Boolean; }}\n\
         temporal T using ev over (s: Reading) on origin {{ {formula} }}"
    )
}

/// [`clause`] under the timestamped-event profile, on the clock `"events"`,
/// with the time unit `ms` declared.
fn timestamped_clause(formula: &str) -> String {
    format!(
        "dimension Time;\n\
         unit ms : Time = rational(1, 1000);\n\
         record Reading {{ k: Int[0, 9]; ok: Boolean; done: Boolean; }}\n\
         temporal T using ev over (s: Reading) clock \"events\" on origin {{ {formula} }}"
    )
}

/// FR-982-AC-4 (TC-2777): under event-position and under timestamped-event,
/// with no interval-less temporal operator (FSpec FR-090-AC-7), a numeric
/// parenthesized threshold checks and a Boolean one refuses
/// `ill_typed`/`type-mismatch`.
#[trace("TC-2777", "FR-982-AC-4", "FSpec-TC-1901", "FSpec-FR-784-AC-3")]
#[test]
#[ignore = "QSL-2160: a parenthesized threshold under the sum rule and its type check are not implemented yet"]
fn a_parenthesized_threshold_is_typed_against_the_aggregate() {
    let cases: [(&str, fn(&str) -> String, &str); 2] = [
        (EVENTS, clause, "past[0,1]"),
        (TIMESTAMPED, timestamped_clause, "past[0 ms, 1000 ms]"),
    ];
    for (header, wrap, window) in cases {
        let numeric = wrap(&format!(
            "count where holds(s.ok) over {window} >= (s.k + 1) and holds(s.done)"
        ));
        if let Err(refusals) = check_in(header, &numeric, CheckMode::Linked) {
            panic!("a numeric threshold checks over {window}: {refusals:?}");
        }
        assert_refused_with(
            header,
            &wrap(&format!(
                "count where holds(s.ok) over {window} >= (s.ok and s.done)"
            )),
            Code::IllTyped,
            Some("type-mismatch"),
        );
    }
}
