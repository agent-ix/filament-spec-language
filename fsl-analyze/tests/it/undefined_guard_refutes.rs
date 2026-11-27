// SPDX-License-Identifier: AGPL-3.0-or-later
//! ADR-018 UE-6: a step guard with no value at a reachable state refutes the
//! timed claim as an undefined evaluation on every route (FR-239-AC-11,
//! FR-239-AC-12, FR-243-AC-9, FR-235-AC-9, FR-245-AC-13); none is witnessed
//! or proved.

use filament_exact::Cancel;
use filament_semantic_value::checking::CheckMode;
use fsl_analyze::model_check::digital::{check_digital, DigitalLimits, DigitalRequest};
use fsl_analyze::zone_check::{settle_zone_check, ZoneCheckLimits, ZoneItem};
use fsl_eval::simulation::model::TimedSystem;
use fsl_replay::{
    CertificateCheckLimits, CertificateRule, CounterexampleKind, ModelCheckOutcome,
    ModelTraceReplayResult, TerminalValue, ZoneCertificateVerdict,
};
use ix_trace_rs::trace;

use crate::timed_world::{document as timed_document, world_with_mode, DENSE};

fn short<T: std::fmt::Debug>(value: &T) -> String {
    let mut text = format!("{value:?}");
    text.truncate(300);
    text
}

// Timed routes: `bad`'s data precondition has no value once `go` has set
// `phase = 1`.

fn timed(claim: &str) -> crate::timed_world::World {
    timed_guard("1 - self.phase", claim)
}

/// [`timed`] with `bad`'s precondition `q(argument) >= 0`: `argument` is
/// `0` at `phase = 1` for `1 - self.phase`, so `6 mod 0` has no value there.
fn timed_guard(argument: &str, claim: &str) -> crate::timed_world::World {
    let body = format!(
        "{DENSE}\
         function q using v(x: Integer): Integer pure {{ 6 mod x }}\n\
         pre Go using v on M::Call::go {{ self.phase = 0 }}\n\
         post Went using v on M::Call::go {{ self.phase = 1 }}\n\
         pre Bad using v on M::Call::bad {{ q({argument}) >= 0 }}\n\
         post Stayed using v on M::Call::bad {{ self.late = pre(self.late) }}\n\
         {claim}"
    );
    world_with_mode(
        &timed_document(&["x"], &[("go", &["phase"]), ("bad", &["late"])]),
        "profile inf = \"filament.temporal.infinite-trace/v1\";\n",
        &body,
        CheckMode::Kernel,
    )
}

fn temporal(formula: &str) -> String {
    format!(
        "temporal C using inf over (c: M::Call) clock \"model-steps\" on origin {{ {formula} }}\n"
    )
}

fn zone_refutes(outcome: &crate::zone_search::Outcome) {
    let crate::zone_search::Outcome::Violated { counterexample, .. } = outcome else {
        panic!(
            "expected an undefined evaluation (UE-6): {}",
            short(outcome)
        );
    };
    let CounterexampleKind::UndefinedEvaluation(undefined) = &counterexample.kind else {
        panic!(
            "expected an undefined evaluation (UE-6): {}",
            short(outcome)
        );
    };
    // FR-239-AC-11: the prefix is `go`; `where` names position 1, the
    // precondition of `bad` and the division by zero.
    assert_eq!(counterexample.prefix.len(), 1, "{}", short(outcome));
    assert!(
        format!("{:?}", counterexample.prefix[0].transition).contains("go"),
        "{}",
        short(outcome)
    );
    assert_eq!(undefined.site.position, 1, "{}", short(outcome));
    assert!(
        matches!(
            &undefined.site.locus,
            fsl_replay::UndefinedLocus::Contract(_)
        ),
        "{:?}",
        undefined.site.locus
    );
    assert!(
        format!("{:?}", undefined.site.transition).contains("bad"),
        "{:?}",
        undefined.site.transition
    );
    assert!(
        format!("{:?}", undefined.cause)
            .to_lowercase()
            .contains("divis"),
        "{:?}",
        undefined.cause
    );
}

#[trace("TC-710", "FR-239-AC-11")]
#[test]
fn zone_safety() {
    let world = timed(&temporal("always holds(c.phase <= 3)"));
    zone_refutes(&crate::zone_search::claim(
        &world,
        ZoneCheckLimits::default(),
        &Cancel::new(),
    ));
}

#[trace("TC-710", "FR-239-AC-11")]
#[test]
fn zone_liveness() {
    let world = timed(&temporal("eventually holds(c.phase = 0)"));
    zone_refutes(&crate::zone_search::claim(
        &world,
        ZoneCheckLimits::default(),
        &Cancel::new(),
    ));
}

#[trace("TC-710", "FR-239-AC-11")]
#[test]
fn zone_deadlock_freedom() {
    let world = timed(&temporal("always holds(c.phase <= 3)"));
    zone_refutes(&crate::zone_search::deadlock(&world));
}

#[trace("TC-710", "FR-243-AC-9")]
#[test]
fn digital_safety() {
    let world = timed(&temporal("always holds(c.phase <= 3)"));
    let system = world.system();
    let timed = TimedSystem::new(&system, "M").expect("a timed subject");
    let node = &world.package.graph().temporal_clauses()[0];
    let letters = system.clause_letters(node).expect("letters");
    let outcome = check_digital(
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
    .map(|outcome| outcome.outcome);
    assert!(
        matches!(&outcome, Ok(ModelCheckOutcome::Violated { counterexample, .. })
            if matches!(counterexample.kind, CounterexampleKind::UndefinedEvaluation(_))),
        "expected an undefined evaluation (UE-6): {}",
        short(&outcome)
    );
}

/// Control: the timed fixture's guard really has no value after `go`. It
/// asserts the fixture, not a criterion, so it carries no tag.
#[test]
fn timed_fixture_control() {
    use fsl_eval::simulation::TransitionSystem;
    let world = timed(&temporal("always holds(c.phase <= 3)"));
    let system = world.system();
    let initial = system.try_initial().expect("initial").remove(0);
    let first = system.successors(&initial).expect("expands");
    let (_, next) = first
        .successors
        .into_iter()
        .find(|(step, _)| format!("{step:?}").contains("Call/go"))
        .expect("go");
    let second = system.successors(&next).expect("expands");
    let findings = format!("{:?}", second.findings);
    assert!(findings.contains("ContractUndetermined"), "{findings}");
}

/// FR-235-AC-9: the zone refutation settles `refuted` after FR-237's replay
/// reproduces `where`, the guarded transition included.
#[trace("TC-710", "FR-235-AC-9")]
#[test]
fn zone_refutation_settles_refuted_after_replay() {
    let world = timed(&temporal("always holds(c.phase <= 3)"));
    let outcome = crate::zone_search::claim(&world, ZoneCheckLimits::default(), &Cancel::new());
    let record = settle_zone_check(
        ZoneItem::Outcome(&outcome),
        &Cancel::new(),
        |_| Ok(ZoneCertificateVerdict::Accepted),
        |payload| {
            crate::zone_replay::replay(&world, payload).map(|settlement| ModelTraceReplayResult {
                settlement,
                hints: Vec::new(),
                terminals: Default::default(),
            })
        },
    );
    assert_eq!(record.value, TerminalValue::Refuted);
}

/// FR-243-AC-9: the digital counterexample replays, the guarded transition
/// included.
#[trace("TC-710", "FR-243-AC-9")]
#[test]
fn digital_refutation_replays() {
    let world = timed(&temporal("always holds(c.phase <= 3)"));
    let system = world.system();
    let timed = TimedSystem::new(&system, "M").expect("a timed subject");
    let node = &world.package.graph().temporal_clauses()[0];
    let letters = system.clause_letters(node).expect("letters");
    let outcome = check_digital(
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
    .expect("checked")
    .outcome;
    let ModelCheckOutcome::Violated { counterexample, .. } = outcome else {
        panic!("a violation: {}", short(&outcome));
    };
    let settlement = crate::zone_replay::replay(&world, &counterexample).expect("replays");
    assert!(crate::zone_replay::reproduced(&settlement));
}

/// FR-239-AC-12: under a weak fairness constraint the zone route never
/// returns `Holds` past the undefined guard (it stays `Undecided` until the
/// refutation through a fair behaviour lands).
#[trace("TC-710", "FR-239-AC-12")]
#[test]
fn zone_liveness_under_fairness_is_not_holds() {
    let world = timed(&temporal(
        "fair weak M::Call::go; eventually holds(c.phase = 0)",
    ));
    let outcome = crate::zone_search::claim(&world, ZoneCheckLimits::default(), &Cancel::new());
    // Never `Holds` or `HoldsReduced`: `Undecided` until the refutation
    // through a fair time-divergent behaviour lands, and then `Violated`
    // with a formula or undefined-evaluation kind, never a deadlock.
    assert!(
        match &outcome {
            crate::zone_search::Outcome::Undecided { .. } => true,
            crate::zone_search::Outcome::Violated { counterexample, .. } =>
                !matches!(counterexample.kind, CounterexampleKind::Deadlock),
            _ => false,
        },
        "expected Undecided or Violated, never Holds (UE-3, UE-6): {}",
        short(&outcome)
    );
}

/// FR-245-AC-13: a certificate for the same subject with `bad`'s guard
/// defined, replayed against the subject whose guard has no value at the
/// `phase = 1` node, is rejected `BadState`.
#[trace("TC-710", "FR-245-AC-13")]
#[test]
fn checker_rejects_bad_state_at_an_undefined_guard() {
    let claim = temporal("always holds(c.phase <= 3)");
    let defined = timed_guard("2 - self.phase", &claim);
    let certificate = crate::zone_search::proved(crate::zone_search::claim(
        &defined,
        ZoneCheckLimits::default(),
        &Cancel::new(),
    ));
    let undefined = timed(&claim);
    let verdict =
        crate::zone_search::check(&undefined, &certificate, CertificateCheckLimits::default())
            .expect("the check runs");
    assert!(
        matches!(&verdict, ZoneCertificateVerdict::Rejected(rejection)
            if rejection.rule == CertificateRule::BadState),
        "expected BadState (UE-6, FR-245): {}",
        short(&verdict)
    );
}
