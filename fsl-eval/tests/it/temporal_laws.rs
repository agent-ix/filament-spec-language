// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-327-AC-14 and FR-611 (FSpec FR-092 "Derived forms", "Positions before
//! the origin"): the pre-origin reading of closed past intervals, the
//! origin-bounded reading of unbounded past operators, the derived forms of
//! strong previous, next and weak until, and the S1 refusal of `previous`,
//! `next` and `weak` written as operators.

use fsl_eval::temporal::{TemporalOutcome, TemporalTrace};
use fsl_foundation::SourceIdentity;
use ix_trace_rs::trace;

use crate::temporal_trace::{
    counter, counters, finite, fixture, lasso, outcome, violated, EVENT, INFINITE,
};

/// The closed `Counter` trace with `c.level` 1, 1, 1 under event-position
/// false-extension, `on origin`.
fn closed_ones(body: &str) -> TemporalOutcome {
    outcome(&counter(EVENT, body), finite(3), counters(&[1, 1, 1]))
}

/// The lasso with an empty prefix and loop `c.level` 1, 1, 1 under
/// infinite-trace, `on origin`.
fn lasso_ones(body: &str) -> TemporalOutcome {
    outcome(
        &fixture(INFINITE, "c: Counter", "on origin", body),
        lasso(0, 3),
        counters(&[1, 1, 1]),
    )
}

/// FR-327-AC-14: a closed past interval reads the pre-origin extension (atoms
/// false, constants unchanged, operators evaluated recursively there), under
/// event-position and infinite-trace alike; `eventually[1,1]` moves the read
/// to position 1.
#[trace("TC-1544", "FR-327-AC-14", "FSpec-TC-1041", "FSpec-FR-092-AC-9")]
#[test]
fn a_closed_past_interval_reads_the_pre_origin_extension() {
    for read in [closed_ones as fn(&str) -> TemporalOutcome, lasso_ones] {
        assert_eq!(read("once[1,1] holds(c.level = 1)"), violated(0));
        assert_eq!(
            read("once[1,1] not holds(c.level = 1)"),
            TemporalOutcome::Holds
        );
        assert_eq!(
            read("eventually[1,1] historically[0,2] holds(c.level = 1)"),
            violated(0)
        );
        assert_eq!(
            read("eventually[1,1] historically[0,2] true"),
            TemporalOutcome::Holds
        );
        assert_eq!(
            read("true since[1,1] not holds(c.level = 1)"),
            TemporalOutcome::Holds
        );
    }
}

/// FR-327-AC-14: under infinite-trace the unbounded and lower-bounded past
/// operators range over positions 0 through `i - a` only.
#[trace("TC-1544", "FR-327-AC-14", "FSpec-TC-1041", "FSpec-FR-092-AC-9")]
#[test]
fn an_unbounded_past_operator_ranges_over_the_positions_that_exist() {
    assert_eq!(
        lasso_ones("eventually[1,1] historically holds(c.level = 1)"),
        TemporalOutcome::Holds
    );
    assert_eq!(
        lasso_ones("eventually[1,1] once[2,*] not holds(c.level = 1)"),
        violated(0)
    );
}

/// Every `Counter` trace of length 1 to 4 over the values 0, 1 and 2.
fn traces() -> Vec<Vec<i64>> {
    let mut all = Vec::new();
    for len in 1..=4u32 {
        for code in 0..3i64.pow(len) {
            let mut code = code;
            let mut values = Vec::new();
            for _ in 0..len {
                values.push(code % 3);
                code /= 3;
            }
            all.push(values);
        }
    }
    all
}

/// The outcome an `on each` clause has when `holds_at(i)` is its truth at
/// each activation `i`: true everywhere, or false at the first position.
fn reference(len: usize, holds_at: impl Fn(usize) -> bool) -> TemporalOutcome {
    match (0..len).find(|i| !holds_at(*i)) {
        None => TemporalOutcome::Holds,
        Some(i) => violated(u64::try_from(i).expect("a small position")),
    }
}

/// FR-611-AC-3: the derived forms equal a reference strong previous, next and
/// bounded weak until over every small trace, with false extension past
/// closure and before the origin.
#[trace("TC-1542", "FR-611-AC-3", "FSpec-TC-1041", "FSpec-FR-092-AC-10")]
#[test]
fn the_derived_forms_carry_previous_next_and_weak_until() {
    const PREVIOUS: &str = "holds(c.level = 1) implies once[1,1] holds(c.level = 0)";
    const NEXT: &str = "eventually[1,1] holds(c.level = 1)";
    const WEAK_UNTIL: &str = "(holds(c.level = 0) until[0,2] holds(c.level = 2)) \
                              or always[0,2] holds(c.level = 0)";
    let each = |body: &str| fixture(EVENT, "c: Counter", "on each (q: Counter)", body);
    let (previous, next, weak) = (each(PREVIOUS), each(NEXT), each(WEAK_UNTIL));
    for values in traces() {
        let len = values.len();
        let at = |j: usize| values.get(j).copied();
        let run = |fx| {
            outcome(
                fx,
                TemporalTrace::Finite {
                    len: u64::try_from(len).expect("a small trace"),
                },
                counters(&values),
            )
        };
        assert_eq!(
            run(&previous),
            reference(len, |i| at(i) != Some(1) || (i > 0 && at(i - 1) == Some(0))),
            "{values:?}: previous"
        );
        assert_eq!(
            run(&next),
            reference(len, |i| at(i + 1) == Some(1)),
            "{values:?}: next"
        );
        let weak_until = |i: usize| {
            let p = |j: usize| at(j) == Some(0);
            let q = |j: usize| at(j) == Some(2);
            (0..=2).any(|d| q(i + d) && (0..d).all(|e| p(i + e))) || (0..=2).all(|d| p(i + d))
        };
        assert_eq!(
            run(&weak),
            reference(len, weak_until),
            "{values:?}: weak until"
        );
    }
}

/// The S1 diagnostics of a unit with one temporal clause whose formula is
/// `body`, over a record with a field `next`.
fn parse_diagnostics(body: &str) -> Vec<fsl_cst::CompleteDiagnostic> {
    let text = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.value.complete/v1\";\n\
         profile t0 = \"{EVENT}\";\n\
         record Counter {{ level: Int[0, 9]; next: Int[0, 9]; }}\n\
         temporal C using t0 over (c: Counter) clock \"events\" on origin {{ {body} }}\n"
    );
    fsl_cst::parse(
        SourceIdentity::new("a", "u"),
        "unit.native",
        text.as_bytes(),
        fsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit")
    .diagnostics()
    .to_vec()
}

/// The one `invalid_syntax` refusal of `body`, its message and the text its
/// region covers.
fn refused_word(body: &str) -> (String, String) {
    let diagnostics = parse_diagnostics(body);
    let [diagnostic] = diagnostics.as_slice() else {
        panic!("{body}: one diagnostic, not {diagnostics:?}");
    };
    assert_eq!(diagnostic.code.as_str(), "invalid_syntax", "{body}");
    let span = diagnostic.byte_span().expect("a located refusal");
    let text = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.value.complete/v1\";\n\
         profile t0 = \"{EVENT}\";\n\
         record Counter {{ level: Int[0, 9]; next: Int[0, 9]; }}\n\
         temporal C using t0 over (c: Counter) clock \"events\" on origin {{ {body} }}\n"
    );
    (
        diagnostic.message.clone(),
        text[span.start..span.end].to_owned(),
    )
}

/// FR-611-AC-1: `previous` and `next` written as operators refuse at the
/// word, naming the derived form.
#[trace("TC-1542", "FR-611-AC-1", "FSpec-TC-1041", "FSpec-FR-092-AC-10")]
#[test]
fn previous_and_next_refuse_naming_their_derived_forms() {
    for (body, word, form) in [
        ("previous holds(c.level = 0)", "previous", "once[1,1]"),
        (
            "always (holds(c.level = 1) implies previous holds(c.level = 0))",
            "previous",
            "once[1,1]",
        ),
        ("next holds(c.level = 1)", "next", "eventually[1,1]"),
    ] {
        let (message, at) = refused_word(body);
        assert_eq!(at, word, "{body}");
        assert!(message.contains(form), "{body}: {message}");
    }
}

/// FR-611-AC-2: `weak until` refuses at `weak`, naming the weak-until form,
/// and `next` stays an ordinary field name.
#[trace("TC-1542", "FR-611-AC-2", "FSpec-TC-1041", "FSpec-FR-092-AC-10")]
#[test]
fn weak_until_refuses_naming_its_derived_form_and_next_stays_an_identifier() {
    let (message, at) = refused_word("holds(c.level = 0) weak until holds(c.level = 2)");
    assert_eq!(at, "weak");
    assert!(message.contains("or always"), "{message}");
    assert_eq!(parse_diagnostics("always[0,1] holds(c.next = 1)"), []);
}
