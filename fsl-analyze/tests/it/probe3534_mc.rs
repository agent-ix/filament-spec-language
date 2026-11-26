// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL-3534 route probes: a `step` guard with no value at a reachable state.
//! ADR-018 UE-6 / ADR-022 RU-5: every item over the subject is refuted as an
//! undefined evaluation; none is witnessed or proved.

use filament_exact::Cancel;
use filament_semantic_value::checking::CheckMode;
use fsl_analyze::model_check::check_model;
use fsl_analyze::model_check::digital::{check_digital, DigitalLimits, DigitalRequest};
use fsl_analyze::model_check::ModelCheckRequest;
use fsl_analyze::zone_check::ZoneCheckLimits;
use fsl_eval::simulation::model::TimedSystem;
use fsl_replay::{
    check_closure, check_components, CounterexampleKind, ModelCheckItem, ModelCheckLimits,
    ModelCheckOutcome, ProofCertificate, StateGraphOutcome,
};

use crate::counter::{document, CONTRACT, UNLIMITED};
use crate::graph_world::config_kernel_world_over;
use crate::liveness_support::fixture_in_mode;
use crate::model_world::{self, Atom, Clause, Letters, BOUNDED, INFINITE};
use crate::state_graph::{no_samples, run};
use crate::timed_world::{document as timed_document, world_with_mode, DENSE};

fn short<T: std::fmt::Debug>(value: &T) -> String {
    let mut text = format!("{value:?}");
    text.truncate(300);
    text
}

// State-graph claims: `attemptUpdate`'s guard has no value at version 2.

const GRAPH_GUARD: &str = "pre Guard using v on Config::ConfigVersion::attemptUpdate { \
     quotient(2 - self.versionNumber) >= 1 }\n";

fn graph(claim: &str, name: &str, limits: ModelCheckLimits) {
    let world = config_kernel_world_over(GRAPH_GUARD, claim, &["a"], &[vec![0]]);
    let result = run(world.request(&["a"], vec![world.item(name)], limits));
    let outcome = &result.items[0][0].outcome;
    println!("OBSERVED: {}", short(outcome));
    assert!(
        matches!(outcome, StateGraphOutcome::Undefined { .. }),
        "expected Undefined (UE-6, RU-5): {}",
        short(outcome)
    );
}

#[test]
fn probe_graph_possible_witnessed_at_start_sampled() {
    graph(
        "possible Zero using v over (c: Config::ConfigVersion) { c.versionNumber = 0 }\n",
        "Zero",
        ModelCheckLimits::default(),
    );
}

#[test]
fn probe_graph_possible_witnessed_before_the_state() {
    graph(
        "possible One using v over (c: Config::ConfigVersion) { c.versionNumber = 1 }\n",
        "One",
        no_samples(),
    );
}

#[test]
fn probe_graph_always_possible_holds_everywhere() {
    graph(
        "always possible Any using v over (c: Config::ConfigVersion) { c.versionNumber <= 3 }\n",
        "Any",
        no_samples(),
    );
}

#[test]
fn probe_graph_unique_path_holds_by_empty_path() {
    graph(
        "unique path Self_ using v over (c: Config::ConfigVersion) \
         from (c.versionNumber = 0) to (c.versionNumber = 0);\n",
        "Self_",
        no_samples(),
    );
}

// EN-1 temporal claims: `inc`'s guard has no value at value 2.

const COUNTER_GUARD: &str = "pre CanInc using v on M::Counter::inc { undefined_remainder(2 - self.value) >= 0 }\n\
     post Inc using v on M::Counter::inc { self.value - pre(self.value) = 1 }\n";
const REMAINDER: &str =
    "function undefined_remainder using v(x: Integer): Integer pure { 6 mod x }\n";

fn liveness(formula: &str, atom: fn(i64) -> Option<bool>) {
    let fixture = fixture_in_mode(
        document(false),
        &format!("{REMAINDER}{COUNTER_GUARD}"),
        "",
        formula,
        &[("c", 0)],
        CheckMode::Kernel,
    );
    let outcome = fixture.decide(atom).expect("checked");
    println!("OBSERVED: {}", short(&outcome));
    assert!(
        matches!(&outcome, ModelCheckOutcome::Violated { counterexample, .. }
            if matches!(counterexample.kind, CounterexampleKind::UndefinedEvaluation(_))),
        "expected an undefined evaluation (UE-6): {}",
        short(&outcome)
    );
}

#[test]
fn probe_temporal_inf_eventually_decided_at_start() {
    liveness("eventually holds(c.value = 0)", |v| Some(v == 0));
}

#[test]
fn probe_temporal_inf_position_zero_only() {
    liveness("holds(c.value = 0)", |v| Some(v == 0));
}

#[test]
fn probe_temporal_inf_invariant_shape() {
    liveness("always holds(c.value >= 0)", |v| Some(v >= 0));
}

fn bounded(profile: &str, formula: &str, atom: fn(i64) -> Option<bool>) {
    let world = model_world::World::with_kernel_functions(
        model_world::counter(),
        "pre CanInc using v on M::Counter::inc { q(2 - self.value) >= 0 }\n\
         post Inc using v on M::Counter::inc { self.value - pre(self.value) = 1 }\n",
        "function q using v(x: Integer): Integer pure { 6 mod x }\n",
        profile,
        &[Clause {
            activation: "on origin",
            formula,
        }],
        &[("c", serde_json::json!({"value": {"integer": "0"}}))],
    );
    let domain = model_world::counter();
    let atoms: Vec<Atom> = vec![Box::new(move |state, object| {
        atom(domain.integer(state, object, "value"))
    })];
    let letters = Letters::new(&world.clauses[0], atoms);
    let selections = crate::fill::tcb_01_event_position_selection(&world.package);
    let outcome = check_model(
        ModelCheckRequest {
            workers: 1,
            temporal_clock_selections: &selections,
            subject: world.subject(&["c"], UNLIMITED),
            item: ModelCheckItem::Clause {
                clause: &world.clauses[0],
                letters: &letters,
            },
            max_depth: None,
            limits: ModelCheckLimits::default(),
            smt_encoding: Default::default(),
            smt_run: Default::default(),
            reductions: Default::default(),
        },
        &Cancel::new(),
    )
    .expect("checked");
    println!("OBSERVED: {}", short(&outcome));
    assert!(
        matches!(&outcome, ModelCheckOutcome::Violated { counterexample, .. }
            if matches!(counterexample.kind, CounterexampleKind::UndefinedEvaluation(_))),
        "expected an undefined evaluation (UE-6): {}",
        short(&outcome)
    );
}

#[test]
fn probe_temporal_bounded_horizon_before_the_state() {
    bounded(BOUNDED, "eventually[0,0] holds(c.value = 0)", |v| Some(v == 0));
}

#[test]
fn probe_temporal_bounded_inf_profile_control() {
    bounded(INFINITE, "always holds(c.value >= 0)", |v| Some(v >= 0));
}

// EN-1's certificate checkers, given a certificate of the same model without
// the guard.

#[test]
fn probe_checker_component_certificate_past_the_guard() {
    let plain = fixture_in_mode(
        document(false),
        &format!("{REMAINDER}{CONTRACT}"),
        "",
        "always eventually holds(c.value = 3)",
        &[("c", 0)],
        CheckMode::Kernel,
    );
    let (certificate, _) = plain.certified(|v| Some(v == 3));
    let guarded = fixture_in_mode(
        document(false),
        &format!("{REMAINDER}{COUNTER_GUARD}"),
        "",
        "always eventually holds(c.value = 3)",
        &[("c", 0)],
        CheckMode::Kernel,
    );
    let letters = guarded.letters(|v| Some(v == 3));
    let checked = check_components(
        &guarded.certificate_request(&letters, UNLIMITED),
        &certificate,
        &ModelCheckLimits::default(),
        &Cancel::new(),
    )
    .expect("checked");
    println!("OBSERVED: {}", short(&checked));
    assert!(
        !matches!(checked, fsl_replay::TemporalCertificateCheck::Accepted),
        "expected a rejection (UE-6): {}",
        short(&checked)
    );
}

#[test]
fn probe_checker_closure_certificate_past_the_guard() {
    let plain = fixture_in_mode(
        document(false),
        &format!("{REMAINDER}{CONTRACT}"),
        "",
        "always holds(c.value >= 0)",
        &[("c", 0)],
        CheckMode::Kernel,
    );
    let outcome = plain.decide(|v| Some(v >= 0)).expect("checked");
    let ModelCheckOutcome::Holds {
        certificate: ProofCertificate::Closure(certificate),
        ..
    } = outcome
    else {
        panic!("a closure proof without the guard: {}", short(&outcome));
    };
    let guarded = fixture_in_mode(
        document(false),
        &format!("{REMAINDER}{COUNTER_GUARD}"),
        "",
        "always holds(c.value >= 0)",
        &[("c", 0)],
        CheckMode::Kernel,
    );
    let letters = guarded.letters(|v| Some(v >= 0));
    let checked = check_closure(
        &guarded.certificate_request(&letters, UNLIMITED),
        &certificate,
        &ModelCheckLimits::default(),
        &Cancel::new(),
    )
    .expect("checked");
    println!("OBSERVED: {}", short(&checked));
    assert!(
        !matches!(checked, fsl_replay::TemporalCertificateCheck::Accepted),
        "expected a rejection (UE-6): {}",
        short(&checked)
    );
}

// Timed routes: `bad`'s data precondition has no value once `go` has set
// `phase = 1`.

fn timed(claim: &str) -> crate::timed_world::World {
    let body = format!(
        "{DENSE}\
         function q using v(x: Integer): Integer pure {{ 6 mod x }}\n\
         pre Go using v on M::Call::go {{ self.phase = 0 }}\n\
         post Went using v on M::Call::go {{ self.phase = 1 }}\n\
         pre Bad using v on M::Call::bad {{ q(1 - self.phase) >= 0 }}\n\
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
    let text = format!("{outcome:?}");
    let head: String = text.chars().take(80).collect();
    let kind = match outcome {
        crate::zone_search::Outcome::Violated { counterexample, .. } => format!(
            "kind={:?} prefix={}",
            counterexample.kind,
            counterexample.prefix.len()
        ),
        _ => String::new(),
    };
    println!("OBSERVED: {head} {kind:.200}");
    assert!(
        text.starts_with("Violated") && kind.contains("UndefinedEvaluation"),
        "expected an undefined evaluation (UE-6): {}",
        short(outcome)
    );
}

#[test]
fn probe_zone_safety() {
    let world = timed(&temporal("always holds(c.phase <= 3)"));
    zone_refutes(&crate::zone_search::claim(
        &world,
        ZoneCheckLimits::default(),
        &Cancel::new(),
    ));
}

#[test]
fn probe_zone_liveness() {
    let world = timed(&temporal("eventually holds(c.phase = 0)"));
    zone_refutes(&crate::zone_search::claim(
        &world,
        ZoneCheckLimits::default(),
        &Cancel::new(),
    ));
}

#[test]
fn probe_zone_deadlock_freedom() {
    let world = timed(&temporal("always holds(c.phase <= 3)"));
    zone_refutes(&crate::zone_search::deadlock(&world));
}

#[test]
fn probe_digital_safety() {
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
    println!("OBSERVED: {}", short(&outcome));
    assert!(
        matches!(&outcome, Ok(ModelCheckOutcome::Violated { counterexample, .. })
            if matches!(counterexample.kind, CounterexampleKind::UndefinedEvaluation(_))),
        "expected an undefined evaluation (UE-6): {}",
        short(&outcome)
    );
}

// FR-124 deadlock-freedom and FR-951 invariant items on EN-1.

fn derived(name: Option<&str>) {
    let world = model_world::World::with_kernel_functions(
        model_world::counter(),
        "pre CanInc using v on M::Counter::inc { q(2 - self.value) >= 0 }\n\
         post Inc using v on M::Counter::inc { self.value - pre(self.value) = 1 }\n\
         invariant AtMostThree using v on M::Counter at current { self.value <= 3 }\n",
        "function q using v(x: Integer): Integer pure { 6 mod x }\n",
        INFINITE,
        &[],
        &[("c", serde_json::json!({"value": {"integer": "0"}}))],
    );
    let item = match name {
        Some(name) => ModelCheckItem::Invariant {
            clause: fsl_replay::InvariantClause::of(
                world
                    .package
                    .graph()
                    .state_clauses()
                    .iter()
                    .find(|clause| clause.name() == name)
                    .expect("declared"),
            ),
        },
        None => ModelCheckItem::DeadlockFreedom,
    };
    let outcome = check_model(
        ModelCheckRequest {
            workers: 1,
            temporal_clock_selections: &[],
            subject: world.subject(&["c"], UNLIMITED),
            item,
            max_depth: None,
            limits: ModelCheckLimits::default(),
            smt_encoding: Default::default(),
            smt_run: Default::default(),
            reductions: Default::default(),
        },
        &Cancel::new(),
    )
    .expect("checked");
    println!("OBSERVED: {}", short(&outcome));
    assert!(
        matches!(&outcome, ModelCheckOutcome::Violated { counterexample, .. }
            if matches!(counterexample.kind, CounterexampleKind::UndefinedEvaluation(_))),
        "expected an undefined evaluation (UE-6): {}",
        short(&outcome)
    );
}

#[test]
fn probe_deadlock_freedom_item() {
    derived(None);
}

#[test]
fn probe_invariant_item() {
    derived(Some("AtMostThree"));
}

/// Control: the timed fixture's guard really has no value after `go`.
#[test]
fn probe_timed_fixture_control() {
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
    println!(
        "OBSERVED: successors={} findings={findings:.300}",
        second.successors.len()
    );
    assert!(findings.contains("ContractUndetermined"), "{findings}");
}
