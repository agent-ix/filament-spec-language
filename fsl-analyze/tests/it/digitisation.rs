// SPDX-License-Identifier: AGPL-3.0-or-later
//! FSL FR-243: the digital-clock route declines, before any search, a claim
//! whose dense-time verdict digitisation does not preserve, naming the rule
//! and the part, and decides the rest as the zone route does. TC-2290.

use filament_exact::Cancel;
use fsl_analyze::model_check::digital::{
    check_digital, DigitalLimits, DigitalRefusal, DigitalRequest,
};
use fsl_eval::simulation::model::TimedSystem;
use fsl_foundation::Span;
use fsl_replay::planner::{read_result_entry, EntryCause, EntryOrigin};
use fsl_replay::{NotDigitisable, NotDigitisableRule};
use ix_trace_rs::trace;
use serde_json::{json, Value};

use crate::analyze_cli_support::{analyze, label, request_over, Run, Workspace};
use crate::analyze_family_support::rpc_snapshot;
use crate::timed_world;

/// The two-step subject of TC-2290: `first` runs at `t` in `[0, 1]` ms and
/// resets `x`; `second` runs exactly 2 ms later.
const TWO_STEP: &str = "\
     pre First using v on M::Call::first { self.phase = 0 }\n\
     post FirstDone using v on M::Call::first { self.phase = 1 and self.x = 0 ms }\n\
     pre Second using v on M::Call::second { self.phase = 1 and self.x >= 2 ms }\n\
     post SecondDone using v on M::Call::second { self.phase = 2 }\n\
     time invariant Early when { self.phase = 0 } { self.x <= 1 ms }\n\
     time invariant Exact when { self.phase = 1 } { self.x <= 2 ms }\n";

fn declaration(name: &str, body: &str) -> String {
    format!(
        "temporal {name} using tt over (c: M::Call) clock \"model-time\" on origin {{ {body} }}\n"
    )
}

/// A unit over the dense model `M` with the guards, resets and invariants
/// `subject` and the temporal clauses `C0`, `C1`, ... of `bodies`.
fn unit(subject: &str, bodies: &[&str]) -> String {
    let claims: String = bodies
        .iter()
        .enumerate()
        .map(|(at, body)| declaration(&format!("C{at}"), body))
        .collect();
    timed_world::unit_text(
        timed_world::PACKAGE,
        crate::zone_search::TIMED,
        &format!("{}{subject}{claims}", timed_world::DENSE),
    )
}

/// Each claim of `bodies` analyzed over `subject` twice: item `2n` with
/// `options.method` absent and item `2n + 1` with `digital-clocks`.
fn both_routes(name: &str, subject: &str, operations: &[&str], bodies: &[&str]) -> (String, Run) {
    let unit = unit(subject, bodies);
    let writes: &[&str] = &["phase", "x"];
    let operations: Vec<(&str, &[&str])> = operations.iter().map(|name| (*name, writes)).collect();
    let items: Vec<Value> = (0..bodies.len())
        .flat_map(|at| {
            let claim = format!("C{at}");
            [
                json!({"claim": claim.clone()}),
                json!({"claim": claim, "options": {"method": "digital-clocks"}}),
            ]
        })
        .collect();
    let run = analyze(&request_over(
        &Workspace::new(name),
        &unit,
        &timed_world::document(&["x"], &operations),
        &[rpc_snapshot()],
        json!({"items": items}),
    ));
    (unit, run)
}

/// The byte range of `part` within the declaration of claim `C{at}`.
fn span_of(unit: &str, at: usize, body: &str, part: &str) -> Span {
    let declared = declaration(&format!("C{at}"), body);
    let from = unit.find(&declared).expect("the claim is declared");
    let start = from + declared.find(part).expect("the part is in the claim");
    Span {
        start,
        end: start + part.len(),
    }
}

/// `item` is EN-1's decline for a claim that is not digitisable, carrying
/// `rule` at `span`, with no search.
fn assert_declined(item: &Value, rule: &str, span: Span) {
    assert_eq!(label(item), "declined", "{item}");
    assert_eq!(item["record"]["backend"], "en-1", "{item}");
    assert_eq!(item["record"]["settled_by"], "result", "{item}");
    assert_eq!(item["record"]["cause"]["kind"], "invalid-input", "{item}");
    assert_eq!(
        item["record"]["diagnostic"],
        json!({"code": "invalid_runtime_input", "cause": "invalid-value",
            "detail": {"not_digitisable": {"rule": rule,
                "span": {"start": span.start, "end": span.end}}}}),
        "{item}"
    );
    assert!(item.get("exploration").is_none(), "no search runs: {item}");
}

fn assert_settled(item: &Value, backend: &str, value: &str) {
    assert_eq!(item["record"]["backend"], backend, "{item}");
    assert_eq!(label(item), value, "{item}");
}

/// FR-243-AC-5, TC-2290 step 4: each claim of the table that digitisation
/// does not preserve is declined by EN-1 naming the rule and the part, and
/// EN-6 decides it with the hand value.
#[trace("TC-2290", "FR-243-AC-5")]
#[test]
fn digital_clocks_decline_what_one_rounding_does_not_preserve() {
    // (claim, hand value, rule, the named part)
    let table: [(&str, &str, &str, &str); 14] = [
        // No position lies in [1, 2] when `t` is strictly inside (0, 1).
        (
            "eventually[1 ms, 2 ms] holds(c.phase >= 0)",
            "refuted",
            "two-sided-interval",
            "eventually[1 ms, 2 ms] holds(c.phase >= 0)",
        ),
        (
            "holds(c.phase >= 0) until[1 ms, 2 ms] holds(c.phase >= 0)",
            "refuted",
            "two-sided-interval",
            "holds(c.phase >= 0) until[1 ms, 2 ms] holds(c.phase >= 0)",
        ),
        (
            "eventually[0 ms, 0 ms] eventually[1 ms, 2 ms] holds(c.phase >= 0)",
            "refuted",
            "second-bounded-operator",
            "eventually[1 ms, 2 ms] holds(c.phase >= 0)",
        ),
        (
            "always[0 ms, 0 ms] eventually[1 ms, 2 ms] holds(c.phase >= 0)",
            "refuted",
            "two-sided-interval",
            "eventually[1 ms, 2 ms] holds(c.phase >= 0)",
        ),
        (
            "always (holds(c.phase = 0) implies eventually[1 ms, 2 ms] holds(c.phase >= 0))",
            "refuted",
            "two-sided-interval",
            "eventually[1 ms, 2 ms] holds(c.phase >= 0)",
        ),
        (
            "eventually (holds(c.phase = 0) and eventually[1 ms, 2 ms] holds(c.phase >= 0))",
            "refuted",
            "two-sided-interval",
            "eventually[1 ms, 2 ms] holds(c.phase >= 0)",
        ),
        (
            "eventually[1 ms, 1 ms] holds(c.phase >= 0)",
            "refuted",
            "two-sided-interval",
            "eventually[1 ms, 1 ms] holds(c.phase >= 0)",
        ),
        // `t + 2` is in [2, 3]: true, declined conservatively.
        (
            "eventually[2 ms, 3 ms] holds(c.phase = 2)",
            "proved",
            "two-sided-interval",
            "eventually[2 ms, 3 ms] holds(c.phase = 2)",
        ),
        (
            "eventually[0 ms, 1 ms] holds(c.phase = 2) or eventually[0 ms, 2 ms] holds(c.phase = 1)",
            "proved",
            "second-bounded-operator",
            "eventually[0 ms, 2 ms] holds(c.phase = 1)",
        ),
        // (e): `phase` 1 is reached at `t <= 1`, so each of these is true.
        (
            "eventually eventually[0 ms, 2 ms] holds(c.phase = 1)",
            "proved",
            "nested-bounded-operator",
            "eventually[0 ms, 2 ms] holds(c.phase = 1)",
        ),
        (
            "holds(c.phase < 2) until eventually[0 ms, 2 ms] holds(c.phase = 1)",
            "proved",
            "nested-bounded-operator",
            "eventually[0 ms, 2 ms] holds(c.phase = 1)",
        ),
        (
            "(eventually[0 ms, 2 ms] holds(c.phase = 1)) release holds(c.phase < 3)",
            "proved",
            "nested-bounded-operator",
            "eventually[0 ms, 2 ms] holds(c.phase = 1)",
        ),
        // The normal form is `eventually eventually[0 ms, 2 ms] ...`.
        (
            "not always always[0 ms, 2 ms] holds(c.phase < 1)",
            "proved",
            "nested-bounded-operator",
            "always[0 ms, 2 ms] holds(c.phase < 1)",
        ),
        // `not (a until[0, 2] b)` is a release; `not release` is an until,
        // two-sided here: position 1 is at `t <= 1`, outside [1, 2] only
        // when `t < 1`, and `phase` 2 is at `t + 2`, so no position is in
        // the window for `0 < t < 1`.
        (
            "not (holds(c.phase < 0) release[1 ms, 2 ms] holds(c.phase < 0))",
            "refuted",
            "two-sided-interval",
            "holds(c.phase < 0) release[1 ms, 2 ms] holds(c.phase < 0)",
        ),
    ];
    let bodies: Vec<&str> = table.iter().map(|row| row.0).collect();
    let (unit, run) = both_routes(
        "digitisation-declined",
        TWO_STEP,
        &["first", "second"],
        &bodies,
    );
    for (at, (body, value, rule, part)) in table.iter().enumerate() {
        assert_settled(&run.item(2 * at), "en-6", value);
        assert_declined(&run.item(2 * at + 1), rule, span_of(&unit, at, body, part));
    }
}

/// FR-243-AC-5, TC-2290 step 4: the admitted claims of the table, nested
/// shapes included, are decided by both routes. Two nested shapes the digital
/// product reads wrongly settle `inconclusive` `replay-parity` on EN-1
/// until QSL-3480 is fixed.
#[trace("TC-2290", "FR-243-AC-5")]
#[test]
fn digital_clocks_decide_the_admitted_claims_as_zones_do() {
    // (claim, EN-6, EN-1)
    let table: [(&str, &str, &str); 14] = [
        (
            "eventually[0 ms, 2 ms] holds(c.phase >= 1)",
            "proved",
            "proved",
        ),
        (
            "holds(c.phase < 2) until[0 ms, 2 ms] holds(c.phase = 1)",
            "proved",
            "proved",
        ),
        (
            "always (holds(c.phase = 1) implies eventually[0 ms, 2 ms] holds(c.phase = 2))",
            "proved",
            "proved",
        ),
        (
            "always eventually[0 ms, 1 ms] holds(c.phase >= 0)",
            "proved",
            "proved",
        ),
        (
            "always[1 ms, 2 ms] holds(c.phase = 0)",
            "refuted",
            "refuted",
        ),
        (
            "holds(c.phase < 0) release[1 ms, 2 ms] holds(c.phase < 0)",
            "refuted",
            "refuted",
        ),
        (
            "always[0 ms, 4 ms] eventually[0 ms, 1 ms] holds(c.phase >= 1)",
            "proved",
            "inconclusive",
        ),
        (
            "always[0 ms, 4 ms] eventually[0 ms, 1 ms] holds(c.phase = 1)",
            "refuted",
            "refuted",
        ),
        (
            "eventually[0 ms, 4 ms] always[0 ms, 1 ms] holds(c.phase = 2)",
            "proved",
            "inconclusive",
        ),
        (
            "always (holds(c.phase < 1) until[0 ms, 2 ms] holds(c.phase >= 1))",
            "proved",
            "proved",
        ),
        (
            "always (holds(c.phase < 2) until[0 ms, 2 ms] holds(c.phase = 1))",
            "refuted",
            "refuted",
        ),
        // A negation that leaves a `[0, b]` eventually, and one that turns a
        // two-sided eventually into an `always`, which is not counted.
        (
            "not always[0 ms, 2 ms] holds(c.phase < 1)",
            "proved",
            "proved",
        ),
        (
            "not eventually[0 ms, 2 ms] holds(c.phase = 0)",
            "refuted",
            "refuted",
        ),
        // `second` runs at `t + 2`, after 2 ms whenever `t > 0`.
        (
            "eventually[0 ms, 2 ms] holds(c.phase = 2)",
            "refuted",
            "refuted",
        ),
    ];
    let bodies: Vec<&str> = table.iter().map(|row| row.0).collect();
    let (_, run) = both_routes(
        "digitisation-admitted",
        TWO_STEP,
        &["first", "second"],
        &bodies,
    );
    for (at, (_, zone, digital)) in table.iter().enumerate() {
        assert_settled(&run.item(2 * at), "en-6", zone);
        let item = run.item(2 * at + 1);
        assert_settled(&item, "en-1", digital);
        assert!(
            item["diagnostic"]["detail"]
                .get("not_digitisable")
                .is_none(),
            "{item}"
        );
        if *digital == "inconclusive" {
            assert_eq!(item["record"]["cause"]["kind"], "replay-parity", "{item}");
        }
    }
}

/// FR-243-AC-7, TC-2290 step 5: with fairness declared over a subject with a
/// clock constraint, the named part is the first `fair` constraint in source
/// order.
#[trace("TC-2290", "FR-243-AC-7")]
#[test]
fn digital_clocks_decline_fairness_over_a_clock_constraint() {
    let body = "fair strong M::Call::second; fair weak M::Call::first; \
                eventually[0 ms, 2 ms] holds(c.phase >= 1)";
    let (unit, run) = both_routes("digitisation-fair", TWO_STEP, &["first", "second"], &[body]);
    assert_settled(&run.item(0), "en-6", "proved");
    assert_declined(
        &run.item(1),
        "fairness-over-clock-constraint",
        span_of(&unit, 0, body, "fair strong M::Call::second;"),
    );
}

/// FR-243-AC-7, rule (f): the two subjects whose fair dense behaviour the
/// integer grid cannot express. `spin` keeps time moving while `fire` is
/// disabled at some instant of every period, so `eventually` its effect is
/// false; every integer-clock behaviour enables `fire` and would prove it.
#[trace("TC-2290", "FR-243-AC-7")]
#[test]
fn digital_clocks_decline_the_fairness_counterexamples_zones_refute() {
    for (name, kind, spin, fire) in [
        (
            "digitisation-strong",
            "strong",
            "self.x <= 1 ms",
            "self.x >= 1 ms",
        ),
        (
            "digitisation-weak",
            "weak",
            "self.x >= 1 ms",
            "(self.x <= 0 ms or self.x >= 1 ms)",
        ),
    ] {
        let subject = format!(
            "pre Spin using v on M::Call::spin {{ self.phase = 0 and {spin} }}\n\
             post Spun using v on M::Call::spin {{ self.phase = 0 and self.x = 0 ms }}\n\
             pre Fire using v on M::Call::fire {{ self.phase = 0 and {fire} }}\n\
             post Fired using v on M::Call::fire {{ self.phase = 1 }}\n\
             time invariant Period when {{ self.phase = 0 }} {{ self.x <= 1 ms }}\n"
        );
        let constraint = format!("fair {kind} M::Call::fire;");
        let body = format!("{constraint} eventually holds(c.phase = 1)");
        let (unit, run) = both_routes(name, &subject, &["spin", "fire"], &[&body]);
        assert_settled(&run.item(0), "en-6", "refuted");
        assert_declined(
            &run.item(1),
            "fairness-over-clock-constraint",
            span_of(&unit, 0, &body, &constraint),
        );
    }
}

/// FR-243-AC-7: fairness over a subject with no clock constraint is
/// digitisable, and both routes decide it.
#[trace("TC-2290", "FR-243-AC-7")]
#[test]
fn digital_clocks_decide_fairness_over_a_subject_with_no_clock_constraint() {
    let subject = "pre Spin using v on M::Call::spin { self.phase = 0 }\n\
                   post Spun using v on M::Call::spin { self.phase = 0 }\n\
                   pre Fire using v on M::Call::fire { self.phase = 0 }\n\
                   post Fired using v on M::Call::fire { self.phase = 1 }\n";
    let (_, run) = both_routes(
        "digitisation-fair-unconstrained",
        subject,
        &["spin", "fire"],
        &["fair strong M::Call::fire; eventually holds(c.phase = 1)"],
    );
    assert_settled(&run.item(0), "en-6", "proved");
    assert_settled(&run.item(1), "en-1", "proved");
}

/// FR-243 rule (b): a claim atom reading a clock does not check, so no such
/// claim reaches the route.
#[trace("TC-2290")]
#[test]
fn a_claim_atom_reading_a_clock_does_not_check() {
    let (_, run) = both_routes(
        "digitisation-clock-atom",
        TWO_STEP,
        &["first", "second"],
        &["eventually holds(c.x <= 1 ms)"],
    );
    assert_eq!(run.exit, 20, "{}", run.document());
}

/// The library entry [`check_digital`] declines too, and what `analyze`
/// writes is what the strict result-entry reader reads back.
#[trace("TC-2290", "FR-243-AC-5")]
#[test]
fn the_not_digitisable_member_round_trips_through_the_strict_reader() {
    let body = "eventually[1 ms, 2 ms] holds(c.phase >= 0)";
    let (unit, run) = both_routes(
        "digitisation-round-trip",
        TWO_STEP,
        &["first", "second"],
        &[body],
    );
    let expected = NotDigitisable {
        rule: NotDigitisableRule::TwoSidedInterval,
        span: span_of(&unit, 0, body, body),
    };
    let diagnostic = run.item(1)["record"]["diagnostic"].clone();

    // The published outcome schema takes the member and refuses another rule.
    let schema: Value =
        serde_json::from_str(fsl_replay::outcome::OUTCOME_SCHEMA_1).expect("schema JSON");
    let schema = jsonschema::JSONSchema::options()
        .with_draft(jsonschema::Draft::Draft202012)
        .compile(&schema)
        .expect("the schema compiles");
    let mut document = run.document();
    assert!(schema.is_valid(&document), "{document}");
    document["items"][1]["record"]["diagnostic"]["detail"]["not_digitisable"]["rule"] =
        json!("interval-not-closed");
    assert!(!schema.is_valid(&document), "{document}");
    let entry = |diagnostic: &Value| {
        json!({"result": "declined", "kind": "temporal-satisfaction", "backend": "en-1",
            "request_index": 1, "cause": {"type": "invalid-input"}, "diagnostic": diagnostic})
    };
    for origin in [EntryOrigin::FirstParty, EntryOrigin::Plugin] {
        let read = read_result_entry(&entry(&diagnostic), 0, origin).expect("a declined entry");
        assert_eq!(read.not_digitisable, Some(expected));
        assert_eq!(read.diagnostic.as_ref(), Some(&diagnostic));
    }

    // Each departure from the member's shape, or from its one code and
    // cause, is refused.
    let member = "/diagnostic/detail/not_digitisable";
    let mut other_rule = diagnostic.clone();
    other_rule["detail"]["not_digitisable"]["rule"] = json!("interval-not-closed");
    let mut extra = diagnostic.clone();
    extra["detail"]["not_digitisable"]["unit"] = json!("program");
    let mut extra_in_span = diagnostic.clone();
    extra_in_span["detail"]["not_digitisable"]["span"]["unit"] = json!("program");
    let mut no_span = diagnostic.clone();
    no_span["detail"]["not_digitisable"] = json!({"rule": "two-sided-interval"});
    let mut other_cause = diagnostic.clone();
    other_cause["cause"] = json!("strict-clock-constraint");
    for (written, cause) in [
        (other_rule, EntryCause::InvalidValue),
        (extra, EntryCause::InvalidValue),
        (extra_in_span, EntryCause::InvalidValue),
        (no_span, EntryCause::InvalidValue),
        (other_cause, EntryCause::UnknownMember),
    ] {
        let refusal = read_result_entry(&entry(&written), 0, EntryOrigin::FirstParty)
            .expect_err("a departure is refused");
        assert_eq!(refusal.pointer, member, "{written}");
        assert_eq!(refusal.cause, cause, "{written}");
    }

    let world = timed_world::world_over(
        &timed_world::document(
            &["x"],
            &[("first", &["phase", "x"]), ("second", &["phase"])],
        ),
        crate::zone_search::TIMED,
        &format!(
            "{}{TWO_STEP}{}",
            timed_world::DENSE,
            declaration("Window", body)
        ),
        &["c"],
    );
    let system = world.system();
    let timed = TimedSystem::new(&system, "M").expect("timed model");
    let node = &world.package.graph().temporal_clauses()[0];
    let letters = system.clause_letters(node).expect("clause letters");
    let refusal = check_digital(
        &DigitalRequest {
            subject: &timed,
            clause: node.clause(),
            letters: &letters,
            limits: DigitalLimits {
                max_states: u64::MAX,
                max_transitions: u64::MAX,
                max_automaton_states: u64::MAX,
            },
        },
        &Cancel::new(),
    )
    .expect_err("the library entry declines");
    assert!(
        matches!(
            refusal,
            DigitalRefusal::NotDigitisable(declined)
                if declined.rule == NotDigitisableRule::TwoSidedInterval
        ),
        "{refusal:?}"
    );
}
