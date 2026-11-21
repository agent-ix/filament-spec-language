// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-751 (TC-2101) and FSpec FR-652 (TC-1605): the core's check of a hybrid
//! provider's certificate. Each test names its FSpec acceptance criterion
//! and builds the certificate the criterion states over the model it states.

use std::collections::BTreeSet;

use serde_json::{json, Map, Value};

use super::*;
use crate::hybrid::BoundDimension;
use crate::hybrid_replay::{
    check_certificate, read_certificate, CertificateOutcome, CertificateRefusal, Expect,
    HybridCertificateRule, ProofCoverage,
};
use crate::proof_result::{InconclusiveCause, TerminalValue};
use crate::result::{DisagreementCause, HybridTraceDisagreement};

fn rat(text: &str) -> Value {
    let (numerator, denominator) = text.split_once('/').unwrap_or((text, "1"));
    json!({"numerator": numerator, "denominator": denominator})
}

fn iv(lower: &str, upper: &str) -> Value {
    json!({"lower": rat(lower), "upper": rat(upper)})
}

fn bx(parts: &[(&str, &str, &str)]) -> Value {
    let mut map = Map::new();
    for (name, lower, upper) in parts {
        map.insert((*name).to_owned(), iv(lower, upper));
    }
    Value::Object(map)
}

fn step(mode: &str, time: (&str, &str), h: &str, b: &Value, apriori: &Value) -> Value {
    json!({"mode": mode, "time": iv(time.0, time.1), "step": rat(h), "box": b, "apriori": apriori})
}

fn jump(
    mode: &str,
    from: (usize, usize),
    time: (&str, &str),
    h: &str,
    b: &Value,
    apriori: &Value,
) -> Value {
    let mut value = step(mode, time, h, b, apriori);
    value["from"] = json!({"dwell": from.0, "transition": from.1});
    value
}

fn flowpipe(steps: Vec<Value>) -> Value {
    json!({"type": "flowpipe", "steps": steps})
}

fn enclosure(boxes: &[(&str, Value)]) -> Value {
    let mut map = Map::new();
    for (name, value) in boxes {
        map.insert((*name).to_owned(), value.clone());
    }
    json!({"type": "inductive-enclosure", "boxes": Value::Object(map)})
}

fn cover(horizon: &str, jumps: u64, dimensions: &[BoundDimension]) -> Option<ProofCoverage> {
    let (numerator, denominator) = horizon.split_once('/').unwrap_or((horizon, "1"));
    Some(ProofCoverage {
        horizon: q(
            numerator.parse().expect("integer"),
            denominator.parse().expect("integer"),
        ),
        jumps,
        exhaustive: dimensions.iter().copied().collect::<BTreeSet<_>>(),
    })
}

const TIME: &[BoundDimension] = &[BoundDimension::Time];

/// What reading and checking a certificate gave.
#[derive(Debug, PartialEq)]
enum Verdict {
    Accepted,
    Rejected(HybridCertificateRule, String),
    Refused(String, String),
}

fn verdict(outcome: Result<CertificateOutcome, CertificateRefusal>) -> Verdict {
    match outcome {
        Ok(CertificateOutcome::Accepted(_)) => Verdict::Accepted,
        Ok(CertificateOutcome::Rejected(rejection)) => {
            Verdict::Rejected(rejection.rule, rejection.at)
        }
        Err(refusal) => Verdict::Refused(refusal.pointer, refusal.code.as_str().to_owned()),
    }
}

fn run_with(
    unit: &Unit,
    name: &str,
    certificate: &Value,
    expect: Expect<'_>,
    coverage: Option<&ProofCoverage>,
    trace: Option<&HybridTrace>,
    work: u64,
) -> Result<CertificateOutcome, CertificateRefusal> {
    let claim = claim(unit, name);
    let model = &unit.hybrid_models()[claim.model.get()];
    let read = read_certificate(model, expect, certificate)?;
    Ok(check_certificate(
        &unit.graph,
        claim,
        &read,
        coverage,
        trace,
        &mut HybridReplayMeter::new(work),
    )
    .expect("the checked model is sound"))
}

fn proof(
    unit: &Unit,
    name: &str,
    certificate: &Value,
    expect: Expect<'_>,
    coverage: Option<ProofCoverage>,
) -> Verdict {
    verdict(run_with(
        unit,
        name,
        certificate,
        expect,
        coverage.as_ref(),
        None,
        u64::MAX,
    ))
}

fn bounded(
    unit: &Unit,
    name: &str,
    certificate: &Value,
    coverage: Option<ProofCoverage>,
) -> Verdict {
    proof(unit, name, certificate, Expect::Bounded, coverage)
}

fn inductive(unit: &Unit, name: &str, certificate: &Value) -> Verdict {
    proof(unit, name, certificate, Expect::Inductive, None)
}

fn rejected(rule: HybridCertificateRule, at: &str) -> Verdict {
    Verdict::Rejected(rule, at.to_owned())
}

fn refused(pointer: &str, code: &str) -> Verdict {
    Verdict::Refused(pointer.to_owned(), code.to_owned())
}

use HybridCertificateRule as R;

fn names(unit: &Unit, name: &str) -> Vec<String> {
    let claim = claim(unit, name);
    unit.hybrid_models()[claim.model.get()]
        .variables
        .iter()
        .map(|variable| variable.name.as_str().to_owned())
        .collect()
}

/// A trace point: mode index, time and the values of the variables by name.
fn tstate(
    unit: &Unit,
    claim_name: &str,
    mode: usize,
    time: (&str, &str),
    values: &[(&str, &str)],
) -> TraceState {
    let variables = names(unit, claim_name);
    let parse = |text: &str| {
        let (numerator, denominator) = text.split_once('/').unwrap_or((text, "1"));
        q(
            numerator.parse().expect("integer"),
            denominator.parse().expect("integer"),
        )
    };
    TraceState {
        mode: ModeIndex::new(mode),
        time: ExactInterval::new(parse(time.0), parse(time.1)),
        enclosure: variables
            .iter()
            .map(|variable| {
                let (_, text) = values
                    .iter()
                    .find(|(name, _)| name == variable)
                    .expect("every variable has a value");
                point(parse(text))
            })
            .collect(),
    }
}

fn one_state(unit: &Unit, claim_name: &str, mode: usize, values: &[(&str, &str)]) -> HybridTrace {
    HybridTrace {
        start: tstate(unit, claim_name, mode, ("0", "0"), values),
        jumps: Vec::new(),
        delta: None,
        loop_to: None,
        end: None,
    }
}

fn witness_verdict(unit: &Unit, name: &str, witness: &Value, trace: &HybridTrace) -> Verdict {
    verdict(run_with(
        unit,
        name,
        witness,
        Expect::Witness(trace),
        None,
        Some(trace),
        u64::MAX,
    ))
}

// -- models -----------------------------------------------------------------

const BRAKING_NO_T: &str = "hybrid Braking using p {\n\
       mode Moving invariant { v >= 0 and v <= 20 and a >= 1 and a <= 2 }\n\
         flow { v' = -a; a' = 0; }\n\
         transition to Stopped when { v = 0 } reset { v = 0 };\n\
       mode Stopped invariant { v = 0 }\n\
         flow { v' = 0; a' = 0; }\n\
     }\n";

fn claim_text(name: &str, model: &str, clock: &str, formula: &str) -> String {
    format!(
        "temporal {name} using p over (h: {model}) clock \"{clock}\" on origin {{ {formula} }}\n"
    )
}

fn braking_unit_no_t() -> Unit {
    let claims = [
        ("Bounded", "always holds(h.v < 30)"),
        ("Tight", "always holds(h.v < 10)"),
        ("Recip", "always holds(1 / h.v < 100)"),
        ("Nested", "always [0, 10] eventually [0, 1] holds(h.v < 30)"),
        ("SlowEnough", "always [0, 10] holds(h.v < 30)"),
        ("TooSlow", "always [0, 10] holds(h.v > 5)"),
        ("RecipTimed", "always [0, 10] holds(1 / h.v < 100)"),
    ];
    let text: String = claims
        .iter()
        .map(|(name, formula)| claim_text(name, "Braking", "model-time", formula))
        .collect();
    check(&format!("{BRAKING_NO_T}{text}"))
}

fn inductive_braking(moving_upper: &str, stopped_a: (&str, &str)) -> Value {
    enclosure(&[
        ("Moving", bx(&[("v", "0", moving_upper), ("a", "1", "2")])),
        (
            "Stopped",
            bx(&[("v", "0", "0"), ("a", stopped_a.0, stopped_a.1)]),
        ),
    ])
}

/// `n/16` in lowest terms, the form an FSpec FR-331 `Rational` takes.
fn sixteenths(n: u32) -> String {
    let divisor = (1..=16_u32)
        .rev()
        .find(|d| n.is_multiple_of(*d) && 16_u32.is_multiple_of(*d))
        .unwrap_or(1);
    match 16 / divisor {
        1 => (n / divisor).to_string(),
        denominator => format!("{}/{denominator}", n / divisor),
    }
}

fn braking_steps() -> Vec<Value> {
    (0..160_u32)
        .map(|k| {
            let upper = sixteenths(320 - k);
            let b = bx(&[("v", "0", &upper), ("a", "1", "2")]);
            step(
                "Moving",
                (&sixteenths(k), &sixteenths(k + 1)),
                "1/16",
                &b,
                &b,
            )
        })
        .collect()
}

fn braking_jump(time: (&str, &str)) -> Value {
    let b = bx(&[("v", "0", "0"), ("a", "1", "2")]);
    jump("Stopped", (0, 0), time, "10", &b, &b)
}

fn braking_pipe(time: (&str, &str)) -> Value {
    let mut steps = braking_steps();
    steps.push(braking_jump(time));
    flowpipe(steps)
}

fn horizon_ten() -> Option<ProofCoverage> {
    cover("10", 1, TIME)
}

// -- FSpec FR-652-AC-1, AC-4, AC-7, AC-8: inductive-enclosure ---------------

/// FSpec FR-652-AC-1.
#[test]
#[trace("FSpec-TC-1605", "FSpec-FR-652-AC-1")]
#[trace("TC-2101", "FR-751-AC-1")]
fn an_inductive_enclosure_is_accepted_and_one_missing_the_initial_state_is_rejected_at_a() {
    let unit = braking_unit_no_t();
    assert_eq!(
        inductive(&unit, "Bounded", &inductive_braking("20", ("1", "2"))),
        Verdict::Accepted
    );
    assert_eq!(
        inductive(&unit, "Bounded", &inductive_braking("19", ("1", "2"))),
        rejected(R::A, "/certificate/boxes/Moving")
    );
}

/// FSpec FR-652-AC-8, the `Braking` rows.
#[test]
#[trace("FSpec-TC-1605", "FSpec-FR-652-AC-8")]
#[trace("TC-2101", "FR-751-AC-4")]
fn the_c_and_d_rules_name_the_box_they_fail_at() {
    let unit = braking_unit_no_t();
    assert_eq!(
        inductive(&unit, "Bounded", &inductive_braking("20", ("1", "3/2"))),
        rejected(R::C, "/certificate/boxes/Stopped")
    );
    let accepted = inductive_braking("20", ("1", "2"));
    assert_eq!(
        inductive(&unit, "Tight", &accepted),
        rejected(R::D, "/certificate/boxes/Moving")
    );
    assert_eq!(
        inductive(&unit, "Recip", &accepted),
        rejected(R::D, "/certificate/boxes/Moving")
    );
    assert_eq!(
        inductive(&unit, "Nested", &accepted),
        rejected(R::D, "/certificate")
    );
}

/// FSpec FR-652-AC-8, `Hold`.
#[test]
#[trace("FSpec-TC-1605", "FSpec-FR-652-AC-8")]
fn an_atom_read_definitely_true_makes_its_negation_definitely_false() {
    let unit = check(&format!(
        "hybrid Hold using p {{ initial {{ mode H when {{ v >= 40 and v <= 50 }}; }} \
         mode H invariant {{ v >= 0 and v <= 50 }} flow {{ v' = 0; }} }}\n{}{}",
        claim_text("NotLow", "Hold", "model-time", "always not holds(h.v < 30)"),
        claim_text(
            "NotHigh",
            "Hold",
            "model-time",
            "always not holds(h.v >= 30)"
        )
    ));
    let boxes = enclosure(&[("H", bx(&[("v", "40", "50")]))]);
    assert_eq!(inductive(&unit, "NotLow", &boxes), Verdict::Accepted);
    assert_eq!(
        inductive(&unit, "NotHigh", &boxes),
        rejected(R::D, "/certificate/boxes/H")
    );
}

/// FSpec FR-652-AC-7.
#[test]
#[trace("FSpec-TC-1605", "FSpec-FR-652-AC-7")]
#[trace("TC-2101", "FR-751-AC-4")]
fn rule_b_admits_only_faces_that_lie_on_the_invariant_or_have_inward_rates() {
    let rise = check(&format!(
        "hybrid Rise using p {{ initial {{ mode Up when {{ v >= 0 and v <= 20 and w = 0 }}; }} \
         mode Up invariant {{ v >= 0 and v <= 41/2 and w = 0 }} flow {{ v' = 1; w' = 0; }} }}\n{}",
        claim_text("Cap", "Rise", "model-time", "always holds(h.v <= 21)")
    ));
    let up = |upper: &str| enclosure(&[("Up", bx(&[("v", "0", upper), ("w", "0", "0")]))]);
    assert_eq!(inductive(&rise, "Cap", &up("41/2")), Verdict::Accepted);
    assert_eq!(
        inductive(&rise, "Cap", &up("39/2")),
        rejected(R::A, "/certificate/boxes/Up")
    );
    assert_eq!(
        inductive(&rise, "Cap", &up("20")),
        rejected(R::B, "/certificate/boxes/Up")
    );
    let twin = check(&format!(
        "hybrid Twin using p {{ initial {{ mode M when {{ x = 0 and y = 0 and w = 0 }}; }} \
         mode M invariant {{ x = y and w = 0 }} flow {{ x' = 1; y' = 1; w' = 0; }} }}\n{}",
        claim_text("Cap", "Twin", "model-time", "always holds(h.x <= 1)")
    ));
    assert_eq!(
        inductive(
            &twin,
            "Cap",
            &enclosure(&[(
                "M",
                bx(&[("x", "0", "1"), ("y", "0", "1"), ("w", "0", "0")])
            )])
        ),
        rejected(R::B, "/certificate/boxes/M")
    );
}

/// FSpec FR-652-AC-4: `box-shape`.
#[test]
#[trace("FSpec-TC-1605", "FSpec-FR-652-AC-4")]
fn a_box_missing_a_variable_is_rejected_at_box_shape() {
    let unit = braking_unit_no_t();
    let boxes = enclosure(&[
        ("Moving", bx(&[("v", "0", "20")])),
        ("Stopped", bx(&[("v", "0", "0"), ("a", "1", "2")])),
    ]);
    assert_eq!(
        inductive(&unit, "Bounded", &boxes),
        rejected(R::BoxShape, "/certificate/boxes/Moving")
    );
    let mut steps = braking_steps();
    steps.push(braking_jump(("0", "10")));
    steps[3]["apriori"] = json!({"v": iv("1", "0"), "a": iv("1", "2")});
    assert_eq!(
        bounded(&unit, "SlowEnough", &flowpipe(steps), horizon_ten()),
        rejected(R::BoxShape, "/certificate/steps/3/apriori")
    );
}

/// FSpec FR-652-AC-4 and FR-751-AC-8: a check the work limit stops.
#[test]
#[trace("FSpec-TC-1605", "FSpec-FR-193-AC-29")]
#[trace("FSpec-TC-1605", "FSpec-FR-652-AC-18")]
#[trace("TC-2101", "FR-751-AC-8")]
fn a_check_the_work_limit_stops_is_rejected_at_work_limit_never_accepted() {
    let unit = braking_unit_no_t();
    let outcome = verdict(run_with(
        &unit,
        "Bounded",
        &inductive_braking("20", ("1", "2")),
        Expect::Inductive,
        None,
        None,
        1,
    ));
    assert_eq!(outcome, rejected(R::WorkLimit, "/certificate/boxes/Moving"));
}

// -- FSpec FR-652-AC-5, AC-9, AC-10: the flowpipe core -----------------------

/// FSpec FR-652-AC-5.
#[test]
#[trace("FSpec-TC-1605", "FSpec-FR-652-AC-5")]
#[trace("TC-2101", "FR-751-AC-5")]
fn a_flowpipe_jump_step_carries_the_narrowed_jump_time() {
    let unit = braking_unit_no_t();
    assert_eq!(
        bounded(
            &unit,
            "SlowEnough",
            &braking_pipe(("0", "10")),
            horizon_ten()
        ),
        Verdict::Accepted
    );
    assert_eq!(
        bounded(
            &unit,
            "SlowEnough",
            &braking_pipe(("1", "10")),
            horizon_ten()
        ),
        rejected(R::Jump, "/certificate/steps/160")
    );
    assert_eq!(
        bounded(
            &unit,
            "SlowEnough",
            &braking_pipe(("0", "20")),
            horizon_ten()
        ),
        Verdict::Accepted
    );
    assert_eq!(
        bounded(&unit, "TooSlow", &braking_pipe(("0", "10")), horizon_ten()),
        rejected(R::Claim, "/certificate/steps/0")
    );
}

/// FSpec FR-652-AC-9, `initial`.
#[test]
#[trace("FSpec-TC-1605", "FSpec-FR-652-AC-9")]
#[trace("TC-2101", "FR-751-AC-5")]
fn the_first_box_must_cover_the_initial_states() {
    let unit = braking_unit_no_t();
    let mut narrow = braking_pipe(("0", "10"));
    narrow["steps"][0]["box"] = json!({"v": iv("0", "1/32"), "a": iv("1", "2")});
    assert_eq!(
        bounded(&unit, "SlowEnough", &narrow, horizon_ten()),
        rejected(R::Initial, "/certificate/steps/0")
    );
    let mut early = braking_pipe(("0", "10"));
    early["steps"][0]["time"] = iv("0", "1/8");
    assert_eq!(
        bounded(&unit, "SlowEnough", &early, horizon_ten()),
        rejected(R::Initial, "/certificate/steps/0")
    );
}

/// FSpec FR-652-AC-9, two initial modes.
#[test]
#[trace("FSpec-TC-1605", "FSpec-FR-652-AC-9")]
fn every_initial_mode_needs_an_initial_dwell() {
    let unit = check(&format!(
        "hybrid Braking using p {{\n\
           initial {{ mode Moving when {{ v = 20 and a >= 1 and a <= 2 }}; \
                     mode Stopped when {{ v = 0 and a = 1 }}; }}\n\
           mode Moving invariant {{ v >= 0 and v <= 20 and a >= 1 and a <= 2 }}\n\
             flow {{ v' = -a; a' = 0; }}\n\
             transition to Stopped when {{ v = 0 }} reset {{ v = 0 }};\n\
           mode Stopped invariant {{ v = 0 }} flow {{ v' = 0; a' = 0; }}\n\
         }}\n{}",
        claim_text(
            "SlowEnough",
            "Braking",
            "model-time",
            "always [0, 10] holds(h.v < 30)"
        )
    ));
    let stopped = bx(&[("v", "0", "0"), ("a", "1", "1")]);
    let mut steps = braking_steps();
    steps.push(step("Stopped", ("0", "10"), "10", &stopped, &stopped));
    steps.push(braking_jump(("0", "10")));
    assert_eq!(
        bounded(&unit, "SlowEnough", &flowpipe(steps), horizon_ten()),
        Verdict::Accepted
    );
    let mut without = braking_steps();
    without.push(braking_jump(("0", "10")));
    assert_eq!(
        bounded(&unit, "SlowEnough", &flowpipe(without), horizon_ten()),
        rejected(R::Initial, "/certificate/steps/0")
    );
}

/// FSpec FR-652-AC-10, `Braking`.
#[test]
#[trace("FSpec-TC-1605", "FSpec-FR-652-AC-10")]
#[trace("TC-2101", "FR-751-AC-5")]
fn picard_and_successor_name_the_step_they_fail_at() {
    let unit = braking_unit_no_t();
    // The contracted tube is included although the uncontracted one is not.
    let mut contracted = braking_pipe(("0", "10"));
    contracted["steps"][0]["apriori"] = json!({"v": iv("0", "20"), "a": iv("1", "2")});
    assert_eq!(
        bounded(&unit, "SlowEnough", &contracted, horizon_ten()),
        Verdict::Accepted
    );
    let mut picard = braking_pipe(("0", "10"));
    picard["steps"][3]["apriori"] = json!({"v": iv("0", "633/32"), "a": iv("1", "2")});
    assert_eq!(
        bounded(&unit, "SlowEnough", &picard, horizon_ten()),
        rejected(R::Picard, "/certificate/steps/3")
    );
    let mut successor = braking_pipe(("0", "10"));
    successor["steps"][6]["box"] = json!({"v": iv("0", "627/32"), "a": iv("1", "2")});
    assert_eq!(
        bounded(&unit, "SlowEnough", &successor, horizon_ten()),
        rejected(R::Successor, "/certificate/steps/6")
    );
    let mut time = braking_pipe(("0", "10"));
    time["steps"][1]["time"] = iv("1/8", "3/16");
    assert_eq!(
        bounded(&unit, "SlowEnough", &time, horizon_ten()),
        rejected(R::Successor, "/certificate/steps/1")
    );
}

/// FSpec FR-652-AC-10, `Cond` and `Lift`.
#[test]
#[trace("FSpec-TC-1605", "FSpec-FR-652-AC-10")]
fn modes_that_are_not_contraction_admitted_check_the_uncontracted_tube() {
    let cond = check(&format!(
        "hybrid Cond using p {{ initial {{ mode M when {{ x = 0 and y = 0 }}; }} \
         mode M invariant {{ x <= (if y > 0 then 10 else 0) }} flow {{ x' = 1; y' = x; }} }}\n{}",
        claim_text("Cap", "Cond", "model-time", "always [0, 1] holds(h.x <= 0)")
    ));
    let b = bx(&[("x", "0", "0"), ("y", "0", "0")]);
    let apriori = bx(&[("x", "-1", "0"), ("y", "-1", "0")]);
    assert_eq!(
        bounded(
            &cond,
            "Cap",
            &flowpipe(vec![step("M", ("0", "1"), "1", &b, &apriori)]),
            cover("1", 0, TIME)
        ),
        rejected(R::Picard, "/certificate/steps/0")
    );
    let lift = check(&format!(
        "hybrid Lift using p {{ initial {{ mode M when {{ x = 0 and y = 0 }}; }} \
         mode M invariant {{ x >= -1 and x <= 1 and y >= -1 }} \
         flow {{ x' = 1; y' = if y > 0 then x else 0; }} }}\n{}{}",
        claim_text("Cap", "Lift", "model-time", "always [0, 1] holds(h.y <= 0)"),
        claim_text("Whole", "Lift", "model-time", "always holds(h.y <= 0)")
    ));
    let b = bx(&[("x", "0", "0"), ("y", "0", "0")]);
    let apriori = bx(&[("x", "0", "1"), ("y", "0", "0")]);
    assert_eq!(
        bounded(
            &lift,
            "Cap",
            &flowpipe(vec![step("M", ("0", "1"), "1", &b, &apriori)]),
            cover("1", 0, TIME)
        ),
        rejected(R::Picard, "/certificate/steps/0")
    );
    assert_eq!(
        inductive(
            &lift,
            "Whole",
            &enclosure(&[("M", bx(&[("x", "-1", "1"), ("y", "-1", "0")]))])
        ),
        rejected(R::B, "/certificate/boxes/M")
    );
}

// -- FSpec FR-652-AC-11, AC-12, AC-13: jumps and coverage --------------------

const FORK: &str = "hybrid Fork using p { initial { mode A when { x = 0 }; } \
     mode A invariant { x >= 0 and x <= 10 } flow { x' = 1; } \
     transition to B when { x >= 5 } reset { x = 100 }; \
     transition to C when { x >= 7 } reset { x = 0 }; \
     mode B invariant { x >= 0 and x <= 200 } flow { x' = 0; } \
     mode C invariant { x >= 0 and x <= 1 } flow { x' = 0; } }\n";

fn fork_unit() -> Unit {
    check(&format!(
        "{FORK}{}{}",
        claim_text(
            "NonNegative",
            "Fork",
            "model-time",
            "always [0, 10] holds(h.x >= 0)"
        ),
        claim_text(
            "Small",
            "Fork",
            "model-time",
            "always [0, 10] holds(h.x < 50)"
        )
    ))
}

fn fork_a_steps(count: u32) -> Vec<Value> {
    (0..count)
        .map(|k| {
            step(
                "A",
                (&k.to_string(), &(k + 1).to_string()),
                "1",
                &bx(&[("x", &k.to_string(), &k.to_string())]),
                &bx(&[("x", &k.to_string(), &(k + 1).to_string())]),
            )
        })
        .collect()
}

fn fork_pipe(b_time: (&str, &str), b_box: &str, with_c: bool) -> Value {
    let mut steps = fork_a_steps(10);
    let b = bx(&[("x", b_box, b_box)]);
    steps.push(jump("B", (0, 0), b_time, "6", &b, &b));
    if with_c {
        let c = bx(&[("x", "0", "0")]);
        steps.push(jump("C", (0, 1), ("6", "10"), "4", &c, &c));
    }
    flowpipe(steps)
}

/// FSpec FR-652-AC-11.
#[test]
#[trace("FSpec-TC-1605", "FSpec-FR-652-AC-11")]
#[trace("TC-2101", "FR-751-AC-5")]
fn every_transition_that_can_fire_needs_its_jump_step() {
    let unit = fork_unit();
    let coverage = || cover("10", 1, TIME);
    let full = fork_pipe(("4", "10"), "100", true);
    assert_eq!(
        bounded(&unit, "NonNegative", &full, coverage()),
        Verdict::Accepted
    );
    assert_eq!(
        bounded(&unit, "Small", &full, coverage()),
        rejected(R::Claim, "/certificate/steps/10")
    );
    let alone = flowpipe(fork_a_steps(10));
    for claim in ["NonNegative", "Small"] {
        assert_eq!(
            bounded(&unit, claim, &alone, coverage()),
            rejected(R::Jump, "/certificate/steps/4")
        );
    }
    assert_eq!(
        bounded(
            &unit,
            "NonNegative",
            &fork_pipe(("4", "10"), "100", false),
            coverage()
        ),
        rejected(R::Jump, "/certificate/steps/6")
    );
    assert_eq!(
        bounded(
            &unit,
            "NonNegative",
            &fork_pipe(("5", "10"), "100", true),
            coverage()
        ),
        rejected(R::Jump, "/certificate/steps/10")
    );
    assert_eq!(
        bounded(
            &unit,
            "NonNegative",
            &fork_pipe(("4", "10"), "99", true),
            coverage()
        ),
        rejected(R::Jump, "/certificate/steps/10")
    );
}

/// FSpec FR-652-AC-12.
#[test]
#[trace("FSpec-TC-1605", "FSpec-FR-652-AC-12")]
fn a_self_loop_needs_from_to_continue_as_a_child_dwell() {
    let unit = check(&format!(
        "hybrid Tick using p {{ initial {{ mode T when {{ x = 0 and w = 0 }}; }} \
         mode T invariant {{ x >= 0 and x <= 1 and w = 0 }} flow {{ x' = 1; w' = 0; }} \
         transition to T when {{ x = 1 }} reset {{ x = 0 }}; }}\n{}",
        "temporal Cap using t over (h: Tick) clock \"model-time\" on origin { always [0 s, 1500 ms] holds(h.x <= 1) }\n"
    ));
    let first = step(
        "T",
        ("0", "1"),
        "1",
        &bx(&[("x", "0", "0"), ("w", "0", "0")]),
        &bx(&[("x", "0", "1"), ("w", "0", "0")]),
    );
    let second = step(
        "T",
        ("1", "3/2"),
        "1/2",
        &bx(&[("x", "1", "1"), ("w", "0", "0")]),
        &bx(&[("x", "1", "1"), ("w", "0", "0")]),
    );
    let third = jump(
        "T",
        (0, 0),
        ("1", "1"),
        "1/2",
        &bx(&[("x", "0", "0"), ("w", "0", "0")]),
        &bx(&[("x", "0", "1/2"), ("w", "0", "0")]),
    );
    let coverage = || cover("3/2", 1, TIME);
    assert_eq!(
        bounded(
            &unit,
            "Cap",
            &flowpipe(vec![first.clone(), second.clone(), third.clone()]),
            coverage()
        ),
        Verdict::Accepted
    );
    let mut plain = third;
    plain.as_object_mut().expect("object").remove("from");
    assert_eq!(
        bounded(
            &unit,
            "Cap",
            &flowpipe(vec![first, second, plain]),
            coverage()
        ),
        rejected(R::Successor, "/certificate/steps/2")
    );
}

/// FSpec FR-652-AC-13.
#[test]
#[trace("FSpec-TC-1605", "FSpec-FR-193-AC-29")]
#[trace("FSpec-TC-1605", "FSpec-FR-652-AC-13")]
#[trace("TC-2101", "FR-751-AC-5")]
fn coverage_is_checked_against_jumps_and_exhaustive() {
    let unit = braking_unit_no_t();
    assert_eq!(
        bounded(
            &unit,
            "SlowEnough",
            &flowpipe(braking_steps()),
            cover("10", 0, TIME)
        ),
        rejected(R::Coverage, "/certificate/steps/0")
    );
    assert_eq!(
        bounded(
            &unit,
            "SlowEnough",
            &braking_pipe(("0", "10")),
            cover("10", 1, &[BoundDimension::Time, BoundDimension::Jumps])
        ),
        rejected(R::Coverage, "/certificate/steps/159")
    );
    let fork = fork_unit();
    let mut short = fork_a_steps(5);
    let b = bx(&[("x", "100", "100")]);
    short.push(jump("B", (0, 0), ("4", "5"), "6", &b, &b));
    assert_eq!(
        bounded(&fork, "NonNegative", &flowpipe(short), cover("10", 1, TIME)),
        rejected(R::Coverage, "/certificate/steps/4")
    );
    let drop = check(&format!(
        "hybrid Drop using p {{ initial {{ mode A when {{ x = 1 }}; }} \
         mode A invariant {{ x >= 0 and x <= 1 }} flow {{ x' = -1; }} \
         transition to B when {{ x = 0 }} reset {{ x = 0 }}; \
         mode B invariant {{ x = 0 }} flow {{ x' = 0; }} }}\n{}",
        claim_text("Cap", "Drop", "model-time", "always [0, 2] holds(h.x <= 1)")
    ));
    let steps = vec![
        step(
            "A",
            ("0", "1"),
            "1",
            &bx(&[("x", "1", "1")]),
            &bx(&[("x", "0", "1")]),
        ),
        step(
            "A",
            ("1", "2"),
            "1",
            &bx(&[("x", "0", "0")]),
            &bx(&[("x", "0", "0")]),
        ),
        jump(
            "B",
            (0, 0),
            ("1", "1"),
            "1",
            &bx(&[("x", "0", "0")]),
            &bx(&[("x", "0", "0")]),
        ),
    ];
    assert_eq!(
        bounded(
            &drop,
            "Cap",
            &flowpipe(steps),
            cover("2", 1, &[BoundDimension::Time, BoundDimension::Jumps])
        ),
        Verdict::Accepted
    );
}

// -- FSpec FR-652-AC-14: the claim rule ---------------------------------------

/// FSpec FR-652-AC-14, `Hold`, `Ramp` and `Decay`.
#[test]
#[trace("FSpec-TC-1605", "FSpec-FR-652-AC-14")]
#[trace("TC-2101", "FR-751-AC-5")]
fn the_claim_is_read_per_instant_not_per_box() {
    let hold = check(&format!(
        "hybrid Hold using p {{ initial {{ mode H when {{ v >= 40 and v <= 50 }}; }} \
         mode H invariant {{ v >= 0 and v <= 50 }} flow {{ v' = 0; }} }}\n{}{}{}",
        claim_text(
            "High",
            "Hold",
            "model-time",
            "always [0, 10] holds(h.v >= 30)"
        ),
        claim_text(
            "Not",
            "Hold",
            "model-time",
            "always [0, 10] not holds(h.v >= 30)"
        ),
        claim_text(
            "Implies",
            "Hold",
            "model-time",
            "always [0, 10] (holds(h.v < 45) implies holds(h.v < 30))"
        )
    ));
    let b = bx(&[("v", "40", "50")]);
    let one = flowpipe(vec![step("H", ("0", "10"), "10", &b, &b)]);
    assert_eq!(
        bounded(&hold, "High", &one, cover("10", 0, TIME)),
        Verdict::Accepted
    );
    for claim in ["Not", "Implies"] {
        assert_eq!(
            bounded(&hold, claim, &one, cover("10", 0, TIME)),
            rejected(R::Claim, "/certificate/steps/0")
        );
    }
    let unit = braking_unit_no_t();
    assert_eq!(
        bounded(
            &unit,
            "RecipTimed",
            &braking_pipe(("0", "10")),
            horizon_ten()
        ),
        rejected(R::Claim, "/certificate/steps/0")
    );
    let steps_claims = |model: &str, source: &str, formula: &str| {
        check(&format!(
            "{source}{}",
            claim_text("Cap", model, "model-steps", formula)
        ))
    };
    let drop_source = "hybrid Drop using p { initial { mode A when { x = 1 }; } \
         mode A invariant { x >= 0 and x <= 1 } flow { x' = -1; } \
         transition to B when { x = 0 } reset { x = 0 }; \
         mode B invariant { x = 0 } flow { x' = 0; } }\n";
    let drop_steps = flowpipe(vec![
        step(
            "A",
            ("0", "1"),
            "1",
            &bx(&[("x", "1", "1")]),
            &bx(&[("x", "0", "1")]),
        ),
        step(
            "A",
            ("1", "2"),
            "1",
            &bx(&[("x", "0", "0")]),
            &bx(&[("x", "0", "0")]),
        ),
        jump(
            "B",
            (0, 0),
            ("1", "1"),
            "1",
            &bx(&[("x", "0", "0")]),
            &bx(&[("x", "0", "0")]),
        ),
    ]);
    let jumps_only = || cover("2", 1, &[BoundDimension::Jumps]);
    let held = steps_claims("Drop", drop_source, "always [0, 1] holds(h.x <= 1)");
    assert_eq!(
        bounded(&held, "Cap", &drop_steps, jumps_only()),
        Verdict::Accepted
    );
    let exact = steps_claims("Drop", drop_source, "always [0, 1] holds(h.x = 1)");
    assert_eq!(
        bounded(&exact, "Cap", &drop_steps, jumps_only()),
        rejected(R::Claim, "/certificate/steps/1")
    );
    let ramp = steps_claims(
        "Ramp",
        "hybrid Ramp using p { initial { mode R when { x = 0 and w = 0 }; } \
         mode R invariant { x >= 0 and x <= 1 and w = 0 } flow { x' = 1; w' = 0; } }\n",
        "always [0, 1] holds(h.x < 1)",
    );
    let ramp_steps = flowpipe(vec![
        step(
            "R",
            ("0", "1"),
            "1",
            &bx(&[("x", "0", "0"), ("w", "0", "0")]),
            &bx(&[("x", "0", "1"), ("w", "0", "0")]),
        ),
        step(
            "R",
            ("1", "2"),
            "1",
            &bx(&[("x", "1", "1"), ("w", "0", "0")]),
            &bx(&[("x", "1", "1"), ("w", "0", "0")]),
        ),
    ]);
    assert_eq!(
        bounded(&ramp, "Cap", &ramp_steps, jumps_only()),
        rejected(R::Claim, "/certificate/steps/1")
    );
    let decay = steps_claims(
        "Decay",
        "hybrid Decay using p { initial { mode D when { x = 1 }; } \
         mode D invariant { x >= 0 and x <= 1 } flow { x' = -x; } }\n",
        "always [1, 1] holds(h.x < 1)",
    );
    let decay_steps = flowpipe(vec![
        step(
            "D",
            ("0", "1/2"),
            "1/2",
            &bx(&[("x", "1", "1")]),
            &bx(&[("x", "1/2", "1")]),
        ),
        step(
            "D",
            ("1/2", "1"),
            "1/2",
            &bx(&[("x", "0", "3/4")]),
            &bx(&[("x", "0", "3/4")]),
        ),
    ]);
    assert_eq!(
        bounded(
            &decay,
            "Cap",
            &decay_steps,
            cover("1", 1, &[BoundDimension::Jumps])
        ),
        rejected(R::Claim, "/certificate/steps/0")
    );
}

// -- FSpec FR-652-AC-3, AC-15, AC-16, AC-19: validated-witness ---------------

const GROW: &str = "hybrid Grow using p { initial { mode Up when { v = 1 }; } \
     mode Up invariant { v >= 1 and v <= 100 } flow { v' = 1 - 1 / (1 + v * v); } }\n";

fn grow_unit(model: &str) -> Unit {
    check(&format!(
        "{model}{}",
        claim_text(
            "Below",
            "Grow",
            "model-time",
            "always [0, 40] holds(h.v < 30)"
        )
    ))
}

const GROW_LOWER: [&str; 9] = [
    "1", "3", "33/5", "21/2", "72/5", "183/10", "111/5", "261/10", "30",
];

fn grow_dwell(steps: usize) -> Vec<Value> {
    GROW_LOWER
        .iter()
        .take(steps)
        .enumerate()
        .map(|(k, lower)| {
            let k = u32::try_from(k).expect("nine steps");
            step(
                "Up",
                (&(4 * k).to_string(), &(4 * k + 4).to_string()),
                "4",
                &bx(&[("v", lower, &(1 + 4 * k).to_string())]),
                &bx(&[("v", lower, &(5 + 4 * k).to_string())]),
            )
        })
        .collect()
}

fn witness(dwells: Vec<Vec<Value>>) -> Value {
    json!({"type": "validated-witness", "dwells": dwells})
}

/// FSpec FR-652-AC-3 and AC-15.
#[test]
#[trace("FSpec-TC-1605", "FSpec-FR-652-AC-3")]
#[trace("FSpec-TC-1605", "FSpec-FR-652-AC-15")]
#[trace("TC-2101", "FR-751-AC-6")]
fn a_witness_is_accepted_only_over_a_decisive_box() {
    let unit = grow_unit(GROW);
    let trace = one_state(&unit, "Below", 0, &[("v", "1")]);
    assert_eq!(
        witness_verdict(&unit, "Below", &witness(vec![grow_dwell(9)]), &trace),
        Verdict::Accepted
    );
    assert_eq!(
        witness_verdict(&unit, "Below", &witness(vec![grow_dwell(8)]), &trace),
        rejected(R::WitnessViolation, "/certificate/dwells/0/7")
    );
    let single = step(
        "Up",
        ("0", "40"),
        "40",
        &bx(&[("v", "1", "1")]),
        &bx(&[("v", "1", "41")]),
    );
    assert_eq!(
        witness_verdict(&unit, "Below", &witness(vec![vec![single]]), &trace),
        rejected(R::WitnessViolation, "/certificate/dwells/0/0")
    );
    let mut start = grow_dwell(9);
    start[0]["box"] = json!({"v": iv("2", "3")});
    assert_eq!(
        witness_verdict(&unit, "Below", &witness(vec![start]), &trace),
        rejected(R::WitnessStart, "/certificate/dwells/0/0")
    );
    let mut picard = grow_dwell(9);
    picard[3]["apriori"] = json!({"v": iv("21/2", "15")});
    assert_eq!(
        witness_verdict(&unit, "Below", &witness(vec![picard]), &trace),
        rejected(R::WitnessPicard, "/certificate/dwells/0/3")
    );
    let low = grow_unit(&GROW.replace("v <= 100", "v <= 31"));
    assert_eq!(
        witness_verdict(&low, "Below", &witness(vec![grow_dwell(9)]), &trace),
        rejected(R::WitnessPicard, "/certificate/dwells/0/7")
    );
    let mut ended = trace.clone();
    ended.end = Some(TraceBefore {
        time: ExactInterval::new(n(32), n(32)),
        enclosure: vec![ExactInterval::new(n(30), n(33))],
    });
    assert_eq!(
        witness_verdict(&unit, "Below", &witness(vec![grow_dwell(9)]), &ended),
        Verdict::Accepted
    );
    ended.end = Some(TraceBefore {
        time: ExactInterval::new(n(31), n(31)),
        enclosure: vec![ExactInterval::new(n(30), n(33))],
    });
    assert_eq!(
        witness_verdict(&unit, "Below", &witness(vec![grow_dwell(9)]), &ended),
        rejected(R::WitnessViolation, "/certificate/dwells/0/8")
    );
}

/// FSpec FR-652-AC-15, `Shift`.
#[test]
#[trace("FSpec-TC-1605", "FSpec-FR-652-AC-15")]
fn a_witness_through_an_undecided_conditional_is_not_unique() {
    let source = |initial: &str| {
        check(&format!(
            "hybrid Shift using p {{ initial {{ mode Up when {{ v = {initial} }}; }} \
             mode Up invariant {{ v >= 1 and v <= 100 }} \
             flow {{ v' = if v > 2 then 1 else 1/2; }} }}\n{}",
            claim_text(
                "Below",
                "Shift",
                "model-time",
                "always [0, 10] holds(h.v < 5)"
            )
        ))
    };
    let unit = source("3");
    let trace = one_state(&unit, "Below", 0, &[("v", "3")]);
    let accepted = witness(vec![vec![
        step(
            "Up",
            ("0", "2"),
            "2",
            &bx(&[("v", "3", "3")]),
            &bx(&[("v", "3", "5")]),
        ),
        step(
            "Up",
            ("2", "3"),
            "1",
            &bx(&[("v", "5", "5")]),
            &bx(&[("v", "5", "6")]),
        ),
    ]]);
    assert_eq!(
        witness_verdict(&unit, "Below", &accepted, &trace),
        Verdict::Accepted
    );
    let low = source("1");
    let trace = one_state(&low, "Below", 0, &[("v", "1")]);
    let undecided = witness(vec![vec![
        step(
            "Up",
            ("0", "4"),
            "4",
            &bx(&[("v", "1", "1")]),
            &bx(&[("v", "1", "5")]),
        ),
        step(
            "Up",
            ("4", "5"),
            "1",
            &bx(&[("v", "3", "5")]),
            &bx(&[("v", "3", "6")]),
        ),
    ]]);
    assert_eq!(
        witness_verdict(&low, "Below", &undecided, &trace),
        rejected(R::WitnessUnique, "/certificate/dwells/0/0")
    );
}

fn grow_jump(guard: &str, reset: &str) -> Unit {
    grow_unit(&format!(
        "hybrid Grow using p {{ initial {{ mode Up when {{ v = 1 }}; }} \
         mode Up invariant {{ v >= 1 and v <= 100 }} flow {{ v' = 1 - 1 / (1 + v * v); }} \
         transition to Hold when {{ {guard} }} reset {{ {reset} }}; \
         mode Hold invariant {{ v >= 1 and v <= 100 }} flow {{ v' = 0; }} }}\n"
    ))
}

fn grow_jump_trace(unit: &Unit, jump_time: (&str, &str)) -> HybridTrace {
    HybridTrace {
        start: tstate(unit, "Below", 0, ("0", "0"), &[("v", "1")]),
        jumps: vec![TraceJump {
            transition: 0,
            before: None,
            state: tstate(unit, "Below", 1, jump_time, &[("v", "50")]),
        }],
        delta: None,
        loop_to: None,
        end: None,
    }
}

fn grow_jump_witness() -> Value {
    witness(vec![
        vec![
            step(
                "Up",
                ("0", "4"),
                "4",
                &bx(&[("v", "1", "1")]),
                &bx(&[("v", "1", "5")]),
            ),
            step(
                "Up",
                ("4", "4"),
                "0",
                &bx(&[("v", "3", "5")]),
                &bx(&[("v", "3", "5")]),
            ),
        ],
        vec![step(
            "Hold",
            ("4", "8"),
            "4",
            &bx(&[("v", "50", "50")]),
            &bx(&[("v", "50", "50")]),
        )],
    ])
}

/// FSpec FR-652-AC-16.
#[test]
#[trace("FSpec-TC-1605", "FSpec-FR-652-AC-16")]
fn a_witness_jump_instant_is_derived_from_the_witness_not_the_trace() {
    let unit = grow_jump("v >= 3", "v = 50");
    assert_eq!(
        witness_verdict(
            &unit,
            "Below",
            &grow_jump_witness(),
            &grow_jump_trace(&unit, ("4", "4"))
        ),
        Verdict::Accepted
    );
    assert_eq!(
        witness_verdict(
            &unit,
            "Below",
            &grow_jump_witness(),
            &grow_jump_trace(&unit, ("5", "5"))
        ),
        rejected(R::WitnessGuard, "/certificate/dwells/0/1")
    );
    let strict = grow_jump("v >= 4", "v = 50");
    assert_eq!(
        witness_verdict(
            &strict,
            "Below",
            &grow_jump_witness(),
            &grow_jump_trace(&strict, ("4", "4"))
        ),
        rejected(R::WitnessGuard, "/certificate/dwells/0/1")
    );
}

/// FSpec FR-652-AC-19.
#[test]
#[trace("FSpec-TC-1605", "FSpec-FR-652-AC-19")]
fn a_term_that_is_not_definitely_defined_is_rejected() {
    let recip = check(&format!(
        "hybrid Recip using p {{ initial {{ mode M when {{ x = 1 }}; }} \
         mode M invariant {{ x >= 0 and x <= 2 }} flow {{ x' = 1 / x; }} }}\n{}",
        claim_text(
            "Cap",
            "Recip",
            "model-time",
            "always [0, 1] holds(h.x <= 2)"
        )
    ));
    assert_eq!(
        bounded(
            &recip,
            "Cap",
            &flowpipe(vec![step(
                "M",
                ("0", "1"),
                "1",
                &bx(&[("x", "1", "1")]),
                &bx(&[("x", "0", "2")])
            )]),
            cover("1", 0, TIME)
        ),
        rejected(R::Picard, "/certificate/steps/0")
    );
    let fork_div = check(&format!(
        "{}{}",
        FORK.replace("reset { x = 100 }", "reset { x = 100 / (x - 6) }"),
        claim_text(
            "NonNegative",
            "Fork",
            "model-time",
            "always [0, 10] holds(h.x >= 0)"
        )
    ));
    assert_eq!(
        bounded(
            &fork_div,
            "NonNegative",
            &fork_pipe(("4", "10"), "100", true),
            cover("10", 1, TIME)
        ),
        rejected(R::Jump, "/certificate/steps/5")
    );
    let div = grow_jump("v >= 3", "v = 50 / (v - 4)");
    assert_eq!(
        witness_verdict(
            &div,
            "Below",
            &grow_jump_witness(),
            &grow_jump_trace(&div, ("4", "4"))
        ),
        rejected(R::WitnessGuard, "/certificate/dwells/0/1")
    );
}

// -- FSpec FR-652-AC-17: structural refusals ----------------------------------

/// FSpec FR-652-AC-17.
#[test]
#[trace("FSpec-TC-1605", "FSpec-FR-652-AC-17")]
#[trace("TC-2101", "FR-751-AC-7")]
fn structural_mismatches_are_refusals_never_rejections() {
    let unit = braking_unit_no_t();
    let halted = enclosure(&[
        ("Halted", bx(&[("v", "0", "0"), ("a", "1", "2")])),
        ("Moving", bx(&[("v", "0", "20"), ("a", "1", "2")])),
        ("Stopped", bx(&[("v", "0", "0"), ("a", "1", "2")])),
    ]);
    assert_eq!(
        inductive(&unit, "Bounded", &halted),
        refused("/certificate/boxes/Halted", "invalid-value")
    );
    let missing = enclosure(&[("Moving", bx(&[("v", "0", "20"), ("a", "1", "2")]))]);
    assert_eq!(
        inductive(&unit, "Bounded", &missing),
        refused("/certificate/boxes/Stopped", "missing-member")
    );
    let pipe = |edit: &dyn Fn(&mut Value)| {
        let mut value = braking_pipe(("0", "10"));
        edit(&mut value);
        bounded(&unit, "SlowEnough", &value, horizon_ten())
    };
    assert_eq!(
        pipe(&|v| v["steps"][0]["step"] = rat("-1/16")),
        refused("/certificate/steps/0/step", "invalid-value")
    );
    assert_eq!(
        pipe(&|v| v["steps"][0]["time"] = iv("1", "0")),
        refused("/certificate/steps/0/time", "invalid-value")
    );
    assert_eq!(
        pipe(&|v| v["steps"][160]["from"]["dwell"] = json!(1)),
        refused("/certificate/steps/160/from/dwell", "invalid-value")
    );
    assert_eq!(
        pipe(&|v| v["steps"][160]["from"]["transition"] = json!(1)),
        refused("/certificate/steps/160/from/transition", "invalid-value")
    );
    assert_eq!(
        pipe(&|v| v["steps"][160]["mode"] = json!("Moving")),
        refused("/certificate/steps/160/mode", "invalid-value")
    );
    let grow = grow_unit(GROW);
    let trace = one_state(&grow, "Below", 0, &[("v", "1")]);
    assert_eq!(
        witness_verdict(
            &grow,
            "Below",
            &witness(vec![grow_dwell(9), grow_dwell(1)]),
            &trace
        ),
        refused("/certificate/dwells", "invalid-value")
    );
    let mut with_from = grow_dwell(9);
    with_from[0]["from"] = json!({"dwell": 0, "transition": 0});
    assert_eq!(
        witness_verdict(&grow, "Below", &witness(vec![with_from]), &trace),
        refused("/certificate/dwells/0/0/from", "invalid-value")
    );
    let mut looped = trace.clone();
    looped.loop_to = Some(0);
    assert_eq!(
        witness_verdict(&grow, "Below", &witness(vec![grow_dwell(9)]), &looped),
        refused("/certificate", "invalid-value")
    );
}

/// FSpec FR-652-AC-4: a certificate form the entry's basis does not take.
#[test]
#[trace("FSpec-TC-1605", "FSpec-FR-652-AC-4")]
fn the_form_follows_the_basis_never_the_method() {
    let unit = braking_unit_no_t();
    let pipe = flowpipe(Vec::new());
    assert_eq!(
        inductive(&unit, "Bounded", &pipe),
        refused("/certificate", "invalid-value")
    );
}

/// FSpec FR-652-AC-3 and FR-751-AC-8: a witness check the work limit stops.
#[test]
#[trace("FSpec-TC-1605", "FSpec-FR-652-AC-18")]
#[trace("TC-2101", "FR-751-AC-8")]
fn a_witness_check_the_work_limit_stops_is_rejected_at_work_limit() {
    let unit = grow_unit(GROW);
    let trace = one_state(&unit, "Below", 0, &[("v", "1")]);
    let outcome = verdict(run_with(
        &unit,
        "Below",
        &witness(vec![grow_dwell(9)]),
        Expect::Witness(&trace),
        None,
        Some(&trace),
        20,
    ));
    assert!(
        matches!(&outcome, Verdict::Rejected(R::WorkLimit, at) if at.starts_with("/certificate")),
        "{outcome:?}"
    );
}

// -- FSpec FR-652 "Settlement": FR-450-AC-11, FR-450-AC-13 -------------------

fn settle_provider(
    unit: &Unit,
    kind: HybridCounterexampleKind,
    trace: HybridTrace,
    certificate: &Value,
    work: u64,
) -> crate::hybrid::HybridSettlement {
    let claim = claim(unit, "Below");
    let model = &unit.hybrid_models()[claim.model.get()];
    let read = read_certificate(model, Expect::Witness(&trace), certificate).expect("readable");
    crate::hybrid::HybridSettlement::from_provider_certified(
        &unit.graph,
        claim,
        HybridCounterexample { kind, trace },
        Some(&read),
        &mut HybridReplayMeter::new(work),
    )
    .expect("the checked model is sound")
}

/// FR-450-AC-11: an accepted witness refutes a trace exact replay cannot
/// decide; a rejected one leaves `ReplayParity(HybridTrace(NotExact))`; a
/// claimed kind that disagrees with the witness's kind does not refute.
#[test]
#[trace("TC-1221", "FR-450-AC-11")]
#[trace("TC-1221", "FR-450-AC-13")]
#[trace("TC-2101", "FR-751-AC-8")]
fn a_witness_settles_a_not_exact_provider_refutation() {
    use crate::hybrid::CertificateStatus;
    let unit = grow_unit(GROW);
    let trace = one_state(&unit, "Below", 0, &[("v", "1")]);
    let accepted = settle_provider(
        &unit,
        HybridCounterexampleKind::Formula,
        trace.clone(),
        &witness(vec![grow_dwell(9)]),
        u64::MAX,
    );
    assert_eq!(accepted.value, TerminalValue::Refuted);
    assert_eq!(
        accepted.counterexample.as_ref().map(|found| &found.kind),
        Some(&HybridCounterexampleKind::Formula)
    );
    assert_eq!(
        accepted.certificate.as_ref().map(|record| &record.outcome),
        Some(&CertificateStatus::Accepted)
    );
    let cut = settle_provider(
        &unit,
        HybridCounterexampleKind::Formula,
        trace.clone(),
        &witness(vec![grow_dwell(8)]),
        u64::MAX,
    );
    assert!(cut.counterexample.is_none());
    assert!(matches!(
        &cut.value,
        TerminalValue::Inconclusive(InconclusiveCause::ReplayParity(
            DisagreementCause::HybridTrace(HybridTraceDisagreement::NotExact { .. })
        ))
    ));
    assert!(matches!(
        cut.certificate.as_ref().map(|record| &record.outcome),
        Some(CertificateStatus::Rejected {
            rule: R::WitnessViolation,
            ..
        })
    ));
    // The witness's decisive reading gives kind `formula`: a claimed
    // `undefined-evaluation` does not agree with it.
    let claimed = HybridCounterexampleKind::UndefinedEvaluation {
        at: Box::new(UndefinedInstant {
            state: 0,
            time: ExactReal::Rational(n(0)),
            time_interval: None,
            locus: ExpressionLocus {
                site: ExpressionSite::ClaimAtom { atom: 0 },
                term: TermLocus::default(),
            },
        }),
        cause: UndefinedCause::DivisionByZero,
    };
    let disagreeing = settle_provider(
        &unit,
        claimed,
        trace.clone(),
        &witness(vec![grow_dwell(9)]),
        u64::MAX,
    );
    assert!(disagreeing.counterexample.is_none());
    assert!(matches!(
        &disagreeing.value,
        TerminalValue::Inconclusive(InconclusiveCause::ReplayParity(_))
    ));
    // FR-450-AC-13: a witness check the work limit stops is `incomplete`,
    // never `refuted` and never `replay-parity`.
    let stopped = settle_provider(
        &unit,
        HybridCounterexampleKind::Formula,
        trace,
        &witness(vec![grow_dwell(9)]),
        40,
    );
    assert!(
        matches!(&stopped.value, TerminalValue::Incomplete(_)),
        "{:?}",
        stopped.value
    );
    assert!(matches!(
        stopped.certificate.as_ref().map(|record| &record.outcome),
        Some(CertificateStatus::Rejected {
            rule: R::WorkLimit,
            ..
        })
    ));
}

// -- Soundness regressions (review of the first PR head) ---------------------

/// A closing dwell's last step covers every later instant, so a jump step
/// whose `time` stops at the dwell's last step omits later jumps.
#[test]
#[trace("FSpec-TC-1605", "FSpec-FR-652-AC-22")]
fn a_jump_step_must_cover_the_later_jumps_of_a_closing_dwell() {
    let unit = check(&format!(
        "hybrid Late using p {{ initial {{ mode A when {{ x = 1 and y = 0 }}; }} \
         mode A invariant {{ x = 1 and y = 0 }} flow {{ x' = 0; y' = 0; }} \
         transition to B when {{ y = 0 }} reset {{ x = 0 }}; \
         mode B invariant {{ x >= 0 and x <= 100 and y = 0 }} flow {{ x' = 1; y' = 0; }} }}\n{}",
        claim_text(
            "Late",
            "Late",
            "model-time",
            "always [5, 10] holds(h.x >= 1)"
        )
    ));
    let a = bx(&[("x", "1", "1"), ("y", "0", "0")]);
    let build = |jump_time: (&str, &str)| {
        let mut steps = vec![step("A", ("0", "1"), "1", &a, &a)];
        let b0 = bx(&[("x", "0", "0"), ("y", "0", "0")]);
        let b1 = bx(&[("x", "0", "1"), ("y", "0", "0")]);
        steps.push(jump("B", (0, 0), jump_time, "1", &b0, &b1));
        for i in 1..10_u32 {
            steps.push(step(
                "B",
                (&i.to_string(), &(i + 2).to_string()),
                "1",
                &bx(&[("x", &i.to_string(), &i.to_string()), ("y", "0", "0")]),
                &bx(&[("x", &i.to_string(), &(i + 1).to_string()), ("y", "0", "0")]),
            ));
        }
        flowpipe(steps)
    };
    assert_eq!(
        bounded(&unit, "Late", &build(("0", "1")), cover("10", 1, TIME)),
        rejected(R::Jump, "/certificate/steps/1")
    );
}

/// A last step of length 0 does not close its dwell, so its `B` holds only at
/// its own instant.
#[test]
#[trace("FSpec-TC-1605", "FSpec-FR-652-AC-20")]
fn a_zero_length_last_step_does_not_close_its_dwell() {
    let unit = check(&format!(
        "hybrid Z using p {{ initial {{ mode M when {{ x = 0 }}; }} \
         mode M invariant {{ x >= 0 and x <= 100 }} flow {{ x' = 2; }} }}\n{}",
        claim_text("Z", "Z", "model-time", "always [0, 10] holds(h.x <= 0)")
    ));
    let b = bx(&[("x", "0", "0")]);
    assert_eq!(
        bounded(
            &unit,
            "Z",
            &flowpipe(vec![step("M", ("0", "0"), "0", &b, &b)]),
            cover("10", 0, TIME)
        ),
        rejected(R::Coverage, "/certificate/steps/0")
    );
    // Sibling: a zero-length step after a real one ends the dwell the same way.
    let wide = bx(&[("x", "0", "2")]);
    let unit_claim = check(&format!(
        "hybrid Z using p {{ initial {{ mode M when {{ x = 0 }}; }} \
         mode M invariant {{ x >= 0 and x <= 100 }} flow {{ x' = 2; }} }}\n{}",
        claim_text("Z", "Z", "model-time", "always [0, 10] holds(h.x <= 2)")
    ));
    assert_eq!(
        bounded(
            &unit_claim,
            "Z",
            &flowpipe(vec![
                step("M", ("0", "1"), "1", &b, &wide),
                step("M", ("1", "1"), "0", &wide, &wide),
            ]),
            cover("10", 0, TIME)
        ),
        rejected(R::Coverage, "/certificate/steps/1")
    );
}

/// Rule (d) reads only the current state, so an `on each` atom comparing the
/// state with the activation's `q.x` is not decided over a mode box.
#[test]
#[trace("FSpec-TC-1605", "FSpec-FR-652-AC-21")]
fn an_atom_reading_the_activation_state_is_rejected_at_d() {
    let unit = check(
        "hybrid Sw using p { initial { mode A when { x = 0 }; } \
         mode A invariant { x = 0 } flow { x' = 0; } \
         transition to B when { x = 0 } reset { x = 10 }; \
         mode B invariant { x = 10 } flow { x' = 0; } }\n\
         temporal Same using p over (h: Sw) clock \"model-time\" \
         on each (q: Sw) { always holds(h.x = q.x) }\n",
    );
    let boxes = enclosure(&[
        ("A", bx(&[("x", "0", "0")])),
        ("B", bx(&[("x", "10", "10")])),
    ]);
    assert_eq!(
        inductive(&unit, "Same", &boxes),
        rejected(R::D, "/certificate")
    );
    let guarded = check(
        "hybrid Sw using p { initial { mode A when { x = 0 }; } \
         mode A invariant { x = 0 } flow { x' = 0; } \
         transition to B when { x = 0 } reset { x = 10 }; \
         mode B invariant { x = 10 } flow { x' = 0; } }\n\
         temporal Guarded using p over (h: Sw) clock \"model-time\" \
         on each (q: Sw) when (q.x = 0) { always holds(h.x >= 0) }\n",
    );
    assert_eq!(inductive(&guarded, "Guarded", &boxes), Verdict::Accepted);
}

/// An unreduced `Rational` is a schema-valid value (FSpec FR-331).
#[test]
#[trace("FSpec-TC-1605", "FSpec-FR-652-AC-6")]
fn an_unreduced_rational_is_read_and_normalised() {
    let unit = braking_unit_no_t();
    let mut pipe = braking_pipe(("0", "10"));
    pipe["steps"][1]["step"] = json!({"numerator": "2", "denominator": "32"});
    pipe["steps"][1]["time"] = json!({
        "lower": {"numerator": "2", "denominator": "32"},
        "upper": {"numerator": "4", "denominator": "32"}});
    assert_eq!(
        bounded(&unit, "SlowEnough", &pipe, horizon_ten()),
        Verdict::Accepted
    );
}
