// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-642 (TC-1664, TC-1665): a timed clause over a declared record trace
//! subject, its clock role bound by the request's `timed` clock selection,
//! is admitted by the checker and evaluated pointwise over a recorded timed
//! trace (FSpec FR-416, FR-541). Today the checker refuses it as a timed
//! trace subject that is not checked yet.

use ix_trace_rs::trace;

use filament_exact::{Integer, Meter, Rational};
use fsl_eval::temporal::{
    evaluate_timed_at, AtomReading, LetterStop, TemporalLetters, TemporalOutcome, TemporalPosition,
    TimedTrace,
};
use fsl_foundation::SourceIdentity;
use fsl_semantics::check::{
    check_clauses, CheckedTemporalClause, FormulaNode, FormulaNodeId, PackageDeclarations,
    TemporalProfile, SCALAR_LIMITS_UNLIMITED,
};
use fsl_semantics::model::accounting::ModelNormalizationLimits;
use fsl_semantics::model::intake::{admit_unit, package_input};

/// `Ack` over the record trace subject `r: Reading` on the clock role
/// `"plant"` (FR-642-AC-1).
const ACK: &str = "always (holds(r.req) implies eventually[0 ms, 200 ms] holds(r.ack))";

fn unit(clock: &str, body: &str) -> String {
    format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.value.complete/v1\";\n\
         profile t0 = \"filament.temporal.timed/v1\";\n\
         dimension Time;\n\
         unit s : Time = rational(1, 1);\n\
         unit ms : Time = rational(1, 1000) * s;\n\
         record Reading {{ req: Boolean; ack: Boolean; }}\n\
         temporal Ack using t0 over (r: Reading) clock \"{clock}\" on origin {{ {body} }}\n"
    )
}

/// The unit through S1, S2, the assembler and the temporal check, with no
/// model and no time source: the clock role is a trace-subject role, and
/// `timed_unit` is the `unit` of its `{"kind": "timed"}` configuration
/// (FR-933), `None` for no configuration.
///
/// QSL-2136 adds the in-process configuration input to the S3 check and
/// passes `timed_unit` through it; until then the check takes none.
fn check(text: &str, timed_unit: Option<&str>) -> Vec<Result<CheckedTemporalClause, String>> {
    let _configured = timed_unit;
    let parsed = fsl_cst::parse(
        SourceIdentity::new("a", "u"),
        "unit.native",
        text.as_bytes(),
        fsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let built = fsl_forms::build_unit(&parsed).expect("S2 builds the unit");
    let packages = package_input(std::iter::empty::<&[u8]>()).expect("no packages");
    let models = admit_unit(
        &built.selections().models,
        &packages,
        ModelNormalizationLimits::default(),
    )
    .expect("no model selection");
    let package = PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        "fixture".to_owned(),
        built,
        models,
        Vec::new(),
    )
    .expect("the unit assembles");
    check_clauses(&package)
        .into_iter()
        .map(|result| result.map_err(|cause| format!("{cause:?}")))
        .collect()
}

/// Reads `holds(r.req)` and `holds(r.ack)` from the samples at each position.
struct Samples<'a> {
    text: &'a str,
    clause: &'a CheckedTemporalClause,
    samples: Vec<(bool, bool)>,
}

impl TemporalLetters for Samples<'_> {
    fn atom(
        &mut self,
        atom: FormulaNodeId,
        position: TemporalPosition,
        _meter: &mut Meter,
    ) -> Result<AtomReading, LetterStop> {
        let Some(FormulaNode::Holds { span }) = self.clause.formula().nodes().get(atom.index())
        else {
            panic!("the letter reads only holds atoms");
        };
        let at = usize::try_from(position.index()).expect("a small position");
        let (req, ack) = self.samples[at];
        let value = match &self.text[span.start..span.end] {
            "holds(r.req)" => req,
            "holds(r.ack)" => ack,
            other => panic!("unexpected atom {other}"),
        };
        Ok(if value {
            AtomReading::True
        } else {
            AtomReading::False
        })
    }

    fn activation(
        &mut self,
        _position: TemporalPosition,
        _meter: &mut Meter,
    ) -> Result<AtomReading, LetterStop> {
        panic!("no `when` condition");
    }
}

fn ms(value: u64) -> Rational {
    Rational::from_integer(Integer::from(value))
}

/// FR-642-AC-1 and AC-2: the clause is admitted with a role clock; over the
/// finite trace `req`@0, `ack`@150 the unbounded `always` is pending, and
/// over the timed lasso of FR-642-AC-3 it holds at period 200 and is
/// violated at period 400.
#[test]
#[trace(
    "TC-1664",
    "FR-642-AC-1",
    "FR-642-AC-2",
    "FR-642-AC-3",
    "FSpec-TC-1161",
    "FSpec-FR-541-AC-1"
)]
#[ignore = "QSL-2136: a timed clause over a record trace subject with a role clock is not admitted yet"]
fn a_timed_clause_over_a_record_trace_subject_is_checked_and_evaluated_pointwise() {
    let text = unit("plant", ACK);
    let mut checked = check(&text, Some("ms"));
    let clause = match checked.pop() {
        Some(Ok(clause)) if checked.is_empty() => clause,
        other => panic!("one admitted clause, not {other:?}"),
    };
    assert_eq!(clause.profile(), TemporalProfile::Timed);

    let run = |trace: TimedTrace, samples: Vec<(bool, bool)>| {
        let mut letters = Samples {
            text: &text,
            clause: &clause,
            samples,
        };
        let mut meter = Meter::new(SCALAR_LIMITS_UNLIMITED);
        evaluate_timed_at(
            &clause,
            &trace,
            TemporalPosition::new(0),
            &mut letters,
            &mut meter,
        )
        .expect("the trace is readable")
    };

    let finite = TimedTrace::new(vec![ms(0), ms(150)]).unwrap();
    assert_eq!(
        run(finite, vec![(true, false), (false, true)]),
        TemporalOutcome::Pending
    );

    let samples = vec![(true, false), (false, true), (true, false)];
    let held = TimedTrace::lasso(vec![ms(0), ms(100), ms(120)], 1, ms(200)).unwrap();
    assert_eq!(run(held, samples.clone()), TemporalOutcome::Holds);
    let late = TimedTrace::lasso(vec![ms(0), ms(100), ms(120)], 1, ms(400)).unwrap();
    assert!(
        matches!(run(late, samples), TemporalOutcome::Violated { .. }),
        "a loop request answered 380 ms later violates the 200 ms window"
    );
}

/// FR-642 "Checking": `clock "model-time"` over a record trace subject is
/// refused as `model-time-on-trace-subject`, not as a timed trace subject
/// that is not checked yet.
#[test]
#[trace("TC-1664", "FR-642-AC-1")]
#[ignore = "QSL-2136: model-time over a record trace subject is not refused as model-time-on-trace-subject yet"]
fn model_time_over_a_record_trace_subject_names_the_missing_time_source() {
    let mut checked = check(&unit("model-time", ACK), Some("ms"));
    let refusal = match checked.pop() {
        Some(Err(refusal)) if checked.is_empty() => refusal,
        other => panic!("one refused clause, not {other:?}"),
    };
    assert!(
        refusal.contains("ModelTimeOnTraceSubject"),
        "expected the model-time-on-trace-subject refusal, got {refusal}"
    );
}

/// FR-642-AC-4, FSpec FR-541-AC-2 (TC-1664, FSpec TC-1161): a timed interval
/// is compared exactly on rational stamps: `eventually[0 ms, 500 us]` holds
/// over stamps 0 and 1/3 ms with the answer at the second, and over stamps 0
/// and 2/3 ms it does not.
#[test]
#[trace("TC-1664", "FR-642-AC-4", "FSpec-TC-1161", "FSpec-FR-541-AC-2")]
#[ignore = "QSL-2136: a timed clause over a record trace subject with a role clock is not admitted yet"]
fn timed_intervals_compare_rational_stamps_exactly() {
    let text = unit("plant", "eventually[0 ms, 500 us] holds(r.ack)").replace(
        "unit ms :",
        "unit us : Time = rational(1, 1000000) * s;\nunit ms :",
    );
    let mut checked = check(&text, Some("ms"));
    let clause = match checked.pop() {
        Some(Ok(clause)) if checked.is_empty() => clause,
        other => panic!("one admitted clause, not {other:?}"),
    };
    let third = |numerator: i64| {
        Rational::new(Integer::from(numerator), Integer::from(3_i64)).expect("a rational stamp")
    };
    let run = |late: Rational| {
        let trace = TimedTrace::new(vec![ms(0), late]).unwrap();
        let mut letters = Samples {
            text: &text,
            clause: &clause,
            samples: vec![(false, false), (false, true)],
        };
        let mut meter = Meter::new(SCALAR_LIMITS_UNLIMITED);
        evaluate_timed_at(
            &clause,
            &trace,
            TemporalPosition::new(0),
            &mut letters,
            &mut meter,
        )
        .expect("the trace is readable")
    };
    assert_eq!(run(third(1)), TemporalOutcome::Holds);
    // FR-933-AC-1: the binding unit is `ms`, so `500 us` is `1/2`; the stamp
    // `2/3` completes the window `[0, 1/2]` with no `ack` in it.
    assert!(
        matches!(run(third(2)), TemporalOutcome::Violated { .. }),
        "the window [0, 1/2] ms is complete and unanswered"
    );
}

/// FR-642-AC-6 (TC-2444): with no `through`, a later stamp completes a
/// window: `req`@0 and `ack`@250 violate `[0, 200]` at position 0, while
/// `ack`@200, and `req`@0 alone, leave the clause pending.
#[test]
#[trace("TC-2444", "FR-642-AC-6")]
#[ignore = "QSL-2136: a timed clause over a record trace subject with a role clock is not admitted yet"]
fn a_later_stamp_completes_a_timed_window_without_through() {
    let text = unit("plant", ACK);
    let mut checked = check(&text, Some("ms"));
    let clause = match checked.pop() {
        Some(Ok(clause)) if checked.is_empty() => clause,
        other => panic!("one admitted clause, not {other:?}"),
    };
    let run = |stamps: Vec<Rational>, samples: Vec<(bool, bool)>| {
        let mut letters = Samples {
            text: &text,
            clause: &clause,
            samples,
        };
        let mut meter = Meter::new(SCALAR_LIMITS_UNLIMITED);
        evaluate_timed_at(
            &clause,
            &TimedTrace::new(stamps).unwrap(),
            TemporalPosition::new(0),
            &mut letters,
            &mut meter,
        )
        .expect("the trace is readable")
    };
    assert!(
        matches!(
            run(vec![ms(0), ms(250)], vec![(true, false), (false, true)]),
            TemporalOutcome::Violated { .. }
        ),
        "the stamp 250 completes the window [0, 200], which holds no ack"
    );
    assert_eq!(
        run(vec![ms(0), ms(200)], vec![(true, false), (false, true)]),
        TemporalOutcome::Pending
    );
    assert_eq!(
        run(vec![ms(0)], vec![(true, false)]),
        TemporalOutcome::Pending
    );
}

/// FR-933-AC-1 (TC-2605): checked in process with the `timed` configuration
/// `{"kind": "timed", "unit": "ms"}` for `plant`, every bound is read in `ms`,
/// unit-free bounds as magnitudes in it; with no configuration the check
/// refuses `missing-clock-selection` and infers no unit.
#[test]
#[trace("TC-2605", "FR-933-AC-1")]
#[ignore = "QSL-2136: a timed clause over a record trace subject with a role clock is not admitted yet"]
fn a_role_clock_clause_reads_its_bounds_in_its_binding_unit() {
    let text = |body: &str| {
        unit("plant", body).replace(
            "unit ms :",
            "unit us : Time = rational(1, 1000000) * s;\nunit ms :",
        )
    };
    let outcome = |body: &str, stamps: Vec<Rational>| {
        let text = text(body);
        let mut checked = check(&text, Some("ms"));
        let clause = match checked.pop() {
            Some(Ok(clause)) if checked.is_empty() => clause,
            other => panic!("{body}: one admitted clause, not {other:?}"),
        };
        let mut letters = Samples {
            text: &text,
            clause: &clause,
            samples: vec![(false, false), (false, true)],
        };
        let mut meter = Meter::new(SCALAR_LIMITS_UNLIMITED);
        evaluate_timed_at(
            &clause,
            &TimedTrace::new(stamps).unwrap(),
            TemporalPosition::new(0),
            &mut letters,
            &mut meter,
        )
        .expect("the trace is readable")
    };
    let named = "eventually[0 ms, 200 ms] holds(r.ack)";
    let bare = "eventually[0, 200] holds(r.ack)";
    for body in [named, bare] {
        // Bounds `0` and `200` in `ms`: a stamp 150 answers and 250 does not.
        assert_eq!(outcome(body, vec![ms(0), ms(150)]), TemporalOutcome::Holds);
        assert!(matches!(
            outcome(body, vec![ms(0), ms(250)]),
            TemporalOutcome::Violated { .. }
        ));
    }
    // No configuration: refused, never a unit taken from the bounds.
    for body in [named, bare, "eventually[0 ms, 500 us] holds(r.ack)"] {
        let mut checked = check(&text(body), None);
        let refusal = match checked.pop() {
            Some(Err(refusal)) if checked.is_empty() => refusal,
            other => panic!("{body}: one refused clause, not {other:?}"),
        };
        assert!(
            refusal.contains("MissingClockSelection") && refusal.contains("plant"),
            "{body}: missing-clock-selection naming plant, got {refusal}"
        );
    }
}
