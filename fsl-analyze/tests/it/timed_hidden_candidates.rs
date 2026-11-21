// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-899 "Timed subjects": a timed abstract side with a hidden field whose
//! candidates disagree on the abstract clock behaviour. The candidates live
//! and die per zone: a candidate whose own time invariant stops holding
//! leaves the alive set by a death edge, a discrete step keeps the alive
//! post candidates of the rows whose guard holds, and FR-245's checker
//! recomputes both with code of its own.

use filament_exact::Cancel;
use fsl_analyze::zone_check::{
    certificate_item, check_timed_refinement, TimedLivenessHalf, TimedRefinementRefusal,
    TimedSafetyOutcome, ZoneCheckLimits, ZoneCheckRefusal,
};
use fsl_replay::{
    check_zone_certificate, CertEdge, CertificateCheckLimits, CertificateRule, CertifiedItem,
    DivergentTarget, RefinementFailure, RefinementTraceSettlement, SilentMove,
    TemporalCounterexample, ZoneCertificate, ZoneCertificateRequest, ZoneCertificateVerdict,
};
use fsl_route::deadlock::ItemObligation;
use ix_trace_rs::trace;

use crate::timed_refinement::{obligation, refinement, replay, safety_request, the_refinement};
use crate::timed_world::{document, refining, renamed_package, wire_field, Refining, DENSE};

type Certificate = ZoneCertificate<ItemObligation<&'static str>>;

/// The request limits every test here runs under: a few MiB of zones.
const LIMITS: ZoneCheckLimits = ZoneCheckLimits {
    max_symbolic_states: 2_000,
    max_symbolic_transitions: 20_000,
    max_automaton_states: 1_000,
    max_zone_bytes: 8 << 20,
};

/// An abstract package whose object also has the hidden `h: Boolean`.
fn abstract_document(operations: &[(&str, &[&str])]) -> Vec<u8> {
    let mut document: serde_json::Value =
        serde_json::from_slice(&document(&["y"], operations)).expect("abstract package JSON");
    document["types"][1]["fields"]
        .as_array_mut()
        .expect("abstract object fields")
        .push(wire_field("h", "ix://filament/native/Boolean"));
    renamed_package(&document.to_string().into_bytes(), "test/abstract")
}

/// `TReq`'s `send` (guard `x >= 1`) refined by an abstract `send` over the
/// hidden `h`, free at the initial state: `abstract_guard` is its
/// precondition, `invariant` the abstract time invariant's text (empty for
/// none) and `step` the step row's target.
fn world(abstract_guard: &str, invariant: &str, step: &str) -> Refining {
    let abstract_document = abstract_document(&[("send", &["inflight"])]);
    let concrete = document(&["x"], &[("send", &["inflight"])]);
    refining(
        &abstract_document,
        &concrete,
        &format!(
            "{DENSE}time A dense unit ms;\n\
             post ASend using v on A::Call::send {{ self.inflight }}\n\
             pre ASendable using v on A::Call::send {{ {abstract_guard} }}\n\
             {invariant}\n\
             pre MSend using v on M::Call::send {{ self.x >= 1 ms }}\n\
             post MSent using v on M::Call::send {{ self.inflight }}\n{}",
            refinement("self.inflight")
                .replace(" late = self.late;", "")
                .replace("-> A::Call::send(self)", step)
        ),
    )
    .unwrap_or_else(|refusals| panic!("{refusals:?}"))
    .with_abstract_hidden_initials(&abstract_document, &[false, true])
}

/// The abstract time invariant that holds `y` at 0 while `h` is set.
const TRAP: &str = "time invariant Trap when { self.h } { self.y <= 0 ms }";

fn safety_with_limits(world: &Refining) -> TimedSafetyOutcome<&'static str> {
    let mut request = safety_request(world);
    request.limits = LIMITS;
    check_timed_refinement(request, &Cancel::new())
        .expect("EN-6 takes the refinement")
        .safety
}

fn certified(world: &Refining) -> Certificate {
    match safety_with_limits(world) {
        TimedSafetyOutcome::Holds { certificate, .. } => *certificate,
        other => panic!("the half holds: {other:?}"),
    }
}

/// FR-245's request for `world`'s refinement, over both initial candidates.
fn request(world: &Refining) -> ZoneCertificateRequest<'_> {
    ZoneCertificateRequest {
        simulation: world.concrete_request(),
        model: "M",
        max_clocks: fsl_eval::simulation::model::DEFAULT_MAX_CLOCKS,
        item: CertifiedItem::Refinement {
            refinement: the_refinement(world),
            abstract_package: &world.package,
            abstract_initial: &world.abstract_initials,
        },
    }
}

fn checked(world: &Refining, certificate: &Certificate) -> ZoneCertificateVerdict {
    check_zone_certificate(
        request(world),
        &obligation(world),
        certificate_item,
        certificate,
        CertificateCheckLimits::default(),
        &Cancel::new(),
    )
    .expect("the check runs")
}

/// The indices of the certificate's positions with exactly one alive
/// candidate.
fn single_positions(certificate: &Certificate) -> Vec<usize> {
    let part = certificate.refinement.as_ref().expect("a refinement part");
    (0..part.positions.len())
        .filter(|&at| part.positions[at].abstract_states.len() == 1)
        .collect()
}

/// FR-899-AC-9's `h`: `when self.h { y <= 0 }` reads differently for the two
/// initial candidates. The candidate with `h` set dies as soon as time
/// passes, and the sends, which need `x >= 1`, are only the surviving
/// candidate's: the refinement holds and FR-245's checker accepts the
/// certificate, which carries the death edge.
#[trace("TC-2538", "FR-899")]
#[test]
fn timed_hidden_candidates_die_by_time_and_the_certificate_is_accepted() {
    let world = world("self.y >= 1 ms", TRAP, "-> A::Call::send(self)");
    let certificate = certified(&world);
    assert!(
        certificate
            .edges
            .iter()
            .any(|edge| matches!(edge.silent, Some(SilentMove::Dies { .. }))),
        "the candidate with h set dies by time: {:?}",
        certificate.edges
    );
    assert!(!single_positions(&certificate).is_empty());
    assert_eq!(
        checked(&world, &certificate),
        ZoneCertificateVerdict::Accepted
    );
}

/// A certificate that lists the dead candidate as alive at a later node
/// states an alive set the checker does not compute: it is rejected.
#[trace("TC-2538", "FR-899")]
#[test]
fn timed_hidden_a_tampered_alive_set_is_rejected() {
    let world = world("self.y >= 1 ms", TRAP, "-> A::Call::send(self)");
    let certificate = certified(&world);
    let part = certificate.refinement.as_ref().expect("a refinement part");
    let full = part
        .positions
        .iter()
        .max_by_key(|position| position.abstract_states.len())
        .expect("a position")
        .abstract_states
        .clone();
    let single = single_positions(&certificate);
    let mut altered = certificate.clone();
    for &at in &single {
        altered.refinement.as_mut().expect("a part").positions[at]
            .abstract_states
            .clone_from(&full);
    }
    assert!(matches!(
        checked(&world, &altered),
        ZoneCertificateVerdict::Rejected(_)
    ));
}

/// A death edge the checker's recomputation does not derive is rejected,
/// whatever it leads to.
#[trace("TC-2538", "FR-899")]
#[test]
fn timed_hidden_an_unjustified_death_edge_is_rejected() {
    let world = world("self.y >= 1 ms", TRAP, "-> A::Call::send(self)");
    let certificate = certified(&world);
    let single = single_positions(&certificate)
        .into_iter()
        .find_map(|at| {
            certificate
                .nodes
                .iter()
                .position(|node| node.automaton_state == at && !node.locked)
        })
        .expect("a node with one alive candidate");
    let mut altered = certificate.clone();
    altered.edges.push(CertEdge {
        from: single,
        transition: None,
        task_transition: None,
        part: Some(0),
        horizon: None,
        to: certificate.initials[0],
        silent: Some(SilentMove::Dies {
            candidates: vec![0],
        }),
    });
    altered
        .refinement
        .as_mut()
        .expect("a part")
        .taken
        .push(None);
    match checked(&world, &altered) {
        ZoneCertificateVerdict::Rejected(rejection) => {
            assert_eq!(rejection.rule, CertificateRule::SuccessorsDiffer);
        }
        other => panic!("an extra death edge is rejected: {other:?}"),
    }
}

/// The surviving candidate's guard `y >= 5` fails for a send at `x` in
/// `[1, 5)`: refuted, the counterexample concretised through the death of
/// the other candidate.
#[trace("TC-2538", "FR-899")]
#[test]
fn timed_hidden_a_failing_survivor_guard_is_refuted_with_a_counterexample() {
    let world = world("self.y >= 5 ms", TRAP, "-> A::Call::send(self)");
    let TimedSafetyOutcome::Violated { counterexample, .. } = safety_with_limits(&world) else {
        panic!("the survivor's guard fails");
    };
    assert!(matches!(
        counterexample.refinement.as_ref(),
        Some(RefinementFailure::AbstractClockGuard { .. })
    ));
    let step = counterexample.prefix.first().expect("the failing send");
    let delay = step.delay.as_ref().expect("an exact delay");
    let (one, five) = (
        filament_exact::Rational::from_integer(filament_exact::Integer::one()),
        filament_exact::Rational::from_integer(filament_exact::Integer::from(5_i64)),
    );
    assert!(delay >= &one && delay < &five, "{delay:?}");
}

/// The KEY soundness case. The candidate with `h` set takes the send with no
/// clock guard, the other needs `y >= 5`. While `h` candidate is alive the
/// send at `x` in `[1, 5)` is covered; but it dies the moment time passes,
/// which every send needs, so the truth is a refutation. Without the time
/// invariant the same world holds.
#[trace("TC-2538", "FR-899")]
#[test]
fn timed_hidden_a_dead_candidate_cannot_cover_a_guard_failure() {
    let guard = "self.h or self.y >= 5 ms";
    let violated = world(guard, TRAP, "-> A::Call::send(self)");
    let outcome = safety_with_limits(&violated);
    let TimedSafetyOutcome::Violated { counterexample, .. } = outcome else {
        panic!("the candidate with h set is dead at every send: {outcome:?}");
    };
    assert!(matches!(
        counterexample.refinement.as_ref(),
        Some(RefinementFailure::AbstractClockGuard { .. })
    ));
    let alive = world(guard, "", "-> A::Call::send(self)");
    let outcome = safety_with_limits(&alive);
    assert!(
        matches!(outcome, TimedSafetyOutcome::Holds { .. }),
        "with no invariant the candidate with h set covers every send: {outcome:?}"
    );
}

/// With `any` the concrete send is matched by a stutter and by the abstract
/// `send`; the step row's guard `y >= 5` splits the zone: for `x` in
/// `[1, 5)` only the stutters survive, from `x >= 5` both. The refinement
/// holds, and FR-245's checker recomputes the split.
#[trace("TC-2538", "FR-899")]
#[test]
fn timed_hidden_a_stutter_and_a_guarded_step_split_the_zone() {
    let abstract_document = abstract_document(&[("send", &["inflight"])]);
    let concrete = document(&["x"], &[("send", &["inflight"])]);
    let world = refining(
        &abstract_document,
        &concrete,
        &format!(
            "{DENSE}time A dense unit ms;\n\
             post ASend using v on A::Call::send {{ self.inflight = false }}\n\
             pre ASendable using v on A::Call::send {{ self.y >= 5 ms }}\n\
             pre MSend using v on M::Call::send {{ self.x >= 1 ms }}\n\
             post MSent using v on M::Call::send {{ self.inflight }}\n{}",
            refinement("false")
                .replace(" late = self.late;", "")
                .replace("-> A::Call::send(self)", "-> any")
        ),
    )
    .unwrap_or_else(|refusals| panic!("{refusals:?}"))
    .with_abstract_hidden_initials(&abstract_document, &[false, true]);
    let certificate = certified(&world);
    assert_eq!(
        checked(&world, &certificate),
        ZoneCertificateVerdict::Accepted
    );
}

/// Alive candidates whose rows apply different clock resets at one step
/// need one clock block per candidate, which the product does not keep: the
/// typed refusal of exactly that sub-case.
#[trace("TC-2538", "FR-899")]
#[test]
fn timed_hidden_candidates_with_different_resets_are_refused_distinctly() {
    let abstract_document =
        abstract_document(&[("send", &["inflight", "y"]), ("send2", &["inflight"])]);
    let concrete = document(&["x"], &[("send", &["inflight"])]);
    let world = refining(
        &abstract_document,
        &concrete,
        &format!(
            "{DENSE}time A dense unit ms;\n\
             pre ASendable using v on A::Call::send {{ self.h }}\n\
             post ASend using v on A::Call::send {{ self.inflight and self.y = 0 ms }}\n\
             pre ASendable2 using v on A::Call::send2 {{ not self.h }}\n\
             post ASend2 using v on A::Call::send2 {{ self.inflight }}\n\
             pre MSend using v on M::Call::send {{ self.x >= 1 ms }}\n\
             post MSent using v on M::Call::send {{ self.inflight }}\n{}",
            refinement("self.inflight")
                .replace(" late = self.late;", "")
                .replace("-> A::Call::send(self)", "-> any")
        ),
    )
    .unwrap_or_else(|refusals| panic!("{refusals:?}"))
    .with_abstract_hidden_initials(&abstract_document, &[false, true]);
    let mut request = safety_request(&world);
    request.limits = LIMITS;
    let refused = check_timed_refinement(request, &Cancel::new())
        .err()
        .expect("rows with different resets are refused");
    assert!(
        matches!(
            refused,
            TimedRefinementRefusal::Zone(ZoneCheckRefusal::HiddenCandidateClockBlocks)
        ),
        "{refused:?}"
    );
}

/// A hidden-candidate world with a settable concrete guard, abstract guard,
/// abstract post row and time invariants, over `candidates` (each the
/// `inflight`, `late` and `retried` flags and the hidden `h`). With
/// `resets`, the abstract `send` resets `y`; `concrete_invariants` are
/// appended to the concrete side.
fn custom(
    candidates: &[([bool; 3], Option<bool>)],
    (concrete_guard, abstract_guard, abstract_post): (&str, &str, &str),
    (invariants, resets): (&str, bool),
) -> Refining {
    let modifies: &[&str] = if resets {
        &["inflight", "y"]
    } else {
        &["inflight"]
    };
    let abstract_document = abstract_document(&[("send", modifies)]);
    let concrete = document(&["x"], &[("send", &["inflight"])]);
    refining(
        &abstract_document,
        &concrete,
        &format!(
            "{DENSE}time A dense unit ms;\n\
             post ASend using v on A::Call::send {{ {abstract_post} }}\n\
             pre ASendable using v on A::Call::send {{ {abstract_guard} }}\n\
             {invariants}\n\
             pre MSend using v on M::Call::send {{ {concrete_guard} }}\n\
             post MSent using v on M::Call::send {{ self.inflight }}\n{}",
            refinement("self.inflight").replace(" late = self.late;", "")
        ),
    )
    .unwrap_or_else(|refusals| panic!("{refusals:?}"))
    .with_abstract_candidates(&abstract_document, candidates)
}

/// The reviewer's three candidates: `h` set, `late` set and neither.
const THREE: [([bool; 3], Option<bool>); 3] = [
    ([false, false, false], Some(true)),
    ([false, true, false], Some(false)),
    ([false, false, false], Some(false)),
];

/// The first two of [`THREE`] die at `y > 1`; the third has no invariant.
const PAIR_DIES: &str = "time invariant IA when { self.h } { self.y <= 1 ms }\n\
                         time invariant IB when { self.late } { self.y <= 1 ms }";

/// The reviewer's finding-1 world: the first two candidates die together at
/// `y = 1`, the third survives, and the concrete send needs `x >= 2`.
fn pair_dies(abstract_guard: &str) -> Refining {
    custom(
        &THREE,
        ("self.x >= 2 ms", abstract_guard, "self.inflight"),
        (PAIR_DIES, false),
    )
}

/// Two candidates that die at the same instant must not hide the third: the
/// survivor's guard `y < 1` fails at the send, which needs `x >= 2`. A death
/// edge that removed one candidate at a time never left the node, and the
/// violation was missed.
#[trace("TC-2538", "FR-899")]
#[test]
fn timed_hidden_simultaneous_deaths_expose_the_survivors_guard_failure() {
    let world = pair_dies("self.h or self.late or self.y < 1 ms");
    let TimedSafetyOutcome::Violated { counterexample, .. } = safety_with_limits(&world) else {
        panic!("the survivor's guard fails after the simultaneous deaths");
    };
    assert!(matches!(
        counterexample.refinement.as_ref(),
        Some(RefinementFailure::AbstractClockGuard { .. })
    ));
}

/// The same world where the survivor's guard holds: the refinement holds, the
/// certificate carries one death edge naming both dying candidates, and
/// FR-245's checker accepts it.
#[trace("TC-2538", "FR-899")]
#[test]
fn timed_hidden_simultaneous_deaths_are_one_edge_the_checker_accepts() {
    let world = pair_dies("self.h or self.late or self.y >= 0 ms");
    let certificate = certified(&world);
    assert!(
        certificate.edges.iter().any(|edge| matches!(
            &edge.silent,
            Some(SilentMove::Dies { candidates }) if candidates.len() == 2
        )),
        "{:?}",
        certificate.edges
    );
    assert_eq!(
        checked(&world, &certificate),
        ZoneCertificateVerdict::Accepted
    );
}

/// The certificate without edge `index`, its `taken` row and the witness
/// indices shifted with it.
fn without_edge(certificate: &Certificate, index: usize) -> Certificate {
    let mut altered = certificate.clone();
    altered.edges.remove(index);
    altered
        .refinement
        .as_mut()
        .expect("a refinement part")
        .taken
        .remove(index);
    let shift = |edge: &mut usize| {
        assert_ne!(*edge, index, "the divergence witness avoids the edge");
        if *edge > index {
            *edge -= 1;
        }
    };
    if let Some(admitted) = altered.admitted.as_mut() {
        admitted.path.iter_mut().for_each(shift);
        if let DivergentTarget::Cycle(cycle) = &mut admitted.target {
            cycle.iter_mut().for_each(shift);
        }
    }
    altered
}

/// The index of the first edge of `certificate` for which `wanted` holds
/// that no divergence witness path uses.
fn unwitnessed_edge(certificate: &Certificate, wanted: impl Fn(&CertEdge) -> bool) -> usize {
    let used = certificate
        .admitted
        .as_ref()
        .map(|admitted| admitted.path.clone())
        .unwrap_or_default();
    certificate
        .edges
        .iter()
        .enumerate()
        .position(|(index, edge)| wanted(edge) && !used.contains(&index))
        .expect("an edge no witness uses")
}

/// A death edge the checker derives cannot be dropped from a certificate:
/// its successor then has no edge that covers it.
#[trace("TC-2538", "FR-899")]
#[test]
fn timed_hidden_a_removed_death_edge_is_rejected() {
    let world = pair_dies("self.h or self.late or self.y >= 0 ms");
    let certificate = certified(&world);
    let index = unwitnessed_edge(&certificate, |edge| {
        matches!(edge.silent, Some(SilentMove::Dies { .. }))
    });
    let altered = without_edge(&certificate, index);
    match checked(&world, &altered) {
        ZoneCertificateVerdict::Rejected(rejection) => {
            assert_eq!(rejection.rule, CertificateRule::SuccessorMissing);
        }
        other => panic!("a missing death edge is rejected: {other:?}"),
    }
}

/// The refutation `AbstractTimeInvariant` of `outcome`.
fn assert_time_invariant(outcome: TimedSafetyOutcome<&'static str>) -> TemporalCounterexample {
    let TimedSafetyOutcome::Violated { counterexample, .. } = outcome else {
        panic!("the delay leaves the abstract invariant: {outcome:?}");
    };
    assert!(
        matches!(
            counterexample.refinement.as_ref(),
            Some(RefinementFailure::AbstractTimeInvariant { .. })
        ),
        "{:?}",
        counterexample.refinement
    );
    *counterexample
}

/// FR-900 "Delay": a delay that would leave the abstract invariant fails
/// with `AbstractTimeInvariant`, here after a step that resets `y`, when
/// every alive candidate dies at once.
#[trace("TC-2538", "FR-900")]
#[test]
fn timed_hidden_all_candidates_dying_is_an_abstract_time_invariant_failure() {
    let world = custom(
        &[([false; 3], Some(true)), ([false; 3], Some(false))],
        (
            "self.x >= 1 ms",
            "self.y >= 0 ms",
            "self.inflight and self.y = 0 ms",
        ),
        (
            "time invariant J1 when { self.inflight and self.h } { self.y <= 1 ms }\n\
             time invariant J2 when { self.inflight and not self.h } { self.y <= 1 ms }",
            true,
        ),
    );
    assert_time_invariant(safety_with_limits(&world));
}

/// The same for one candidate whose invariant runs out during a delay, and
/// FR-145 reproduces the counterexample.
#[trace("TC-2538", "FR-900")]
#[test]
fn timed_hidden_a_single_candidate_running_out_of_invariant_is_refuted() {
    let world = custom(
        &[([false; 3], Some(false))],
        (
            "self.x >= 1 ms",
            "self.y >= 0 ms",
            "self.inflight and self.y = 0 ms",
        ),
        (
            "time invariant K when { self.inflight } { self.y <= 1 ms }",
            true,
        ),
    );
    let counterexample = assert_time_invariant(safety_with_limits(&world));
    assert!(matches!(
        replay(&world, &counterexample),
        RefinementTraceSettlement::Reproduced {
            failure: RefinementFailure::AbstractTimeInvariant { .. },
            ..
        }
    ));
}

/// The world of the previous test where the concrete side is time-locked
/// beyond the abstract invariant (`x <= 3` once sent): no time-divergent
/// behaviour reaches the dead region, so it is a `locked` entry node, which
/// the checker requires.
fn locked_dead_region() -> Refining {
    custom(
        &[([false; 3], Some(false))],
        (
            "self.x >= 1 ms",
            "self.y >= 0 ms",
            "self.inflight and self.y = 0 ms",
        ),
        (
            "time invariant K when { self.inflight } { self.y <= 1 ms }\n\
             time invariant MInv when { self.inflight } { self.x <= 3 ms }",
            true,
        ),
    )
}

#[trace("TC-2538", "FR-900")]
#[test]
fn timed_hidden_a_time_locked_dead_region_is_a_required_locked_node() {
    let world = locked_dead_region();
    let certificate = certified(&world);
    let index = unwitnessed_edge(&certificate, |edge| {
        matches!(edge.silent, Some(SilentMove::Dies { .. })) && certificate.nodes[edge.to].locked
    });
    assert_eq!(
        checked(&world, &certificate),
        ZoneCertificateVerdict::Accepted
    );
    let altered = without_edge(&certificate, index);
    match checked(&world, &altered) {
        ZoneCertificateVerdict::Rejected(rejection) => {
            assert_eq!(rejection.rule, CertificateRule::SuccessorMissing);
        }
        other => panic!("a missing locked entry edge is rejected: {other:?}"),
    }
}

/// An initial state whose abstract candidate is dead at the zero valuation
/// (`y < 0`) beside a healthy one refutes: it is not cut away.
#[trace("TC-2538", "FR-900")]
#[test]
fn timed_hidden_an_initial_state_with_no_candidate_alive_at_zero_is_refuted() {
    let abstract_document = abstract_document(&[("send", &["inflight"])]);
    let concrete = document(&["x"], &[("send", &["inflight"])]);
    let world = refining(
        &abstract_document,
        &concrete,
        &format!(
            "{DENSE}time A dense unit ms;\n\
             post ASend using v on A::Call::send {{ self.inflight }}\n\
             pre ASendable using v on A::Call::send {{ self.y >= 0 ms }}\n\
             time invariant Late when {{ self.inflight and self.h }} {{ self.y < 0 ms }}\n\
             pre MSend using v on M::Call::send {{ self.x >= 0 ms }}\n\
             post MSent using v on M::Call::send {{ self.inflight }}\n{}",
            refinement("self.inflight").replace(" late = self.late;", "")
        ),
    )
    .unwrap_or_else(|refusals| panic!("{refusals:?}"))
    .with_concrete_initials(&concrete, &[[false; 3], [true, false, false]])
    .with_abstract_candidates(
        &abstract_document,
        &[
            ([false; 3], Some(false)),
            ([true, false, false], Some(true)),
        ],
    );
    assert_time_invariant(safety_with_limits(&world));
}

/// The liveness half's refusal is its own: the decided safety half is still
/// reported when candidates' rows disagree (a stutter beside a guarded step).
#[trace("TC-2538", "FR-899")]
#[test]
fn timed_hidden_safety_is_reported_beside_the_liveness_refusal() {
    let abstract_document = abstract_document(&[("send", &["inflight"])]);
    let concrete = document(&["x"], &[("send", &["inflight"])]);
    let world = refining(
        &abstract_document,
        &concrete,
        &format!(
            "{DENSE}time A dense unit ms;\n\
             post ASend using v on A::Call::send {{ self.inflight = false }}\n\
             pre ASendable using v on A::Call::send {{ self.y >= 5 ms }}\n\
             pre MSend using v on M::Call::send {{ self.x >= 1 ms }}\n\
             post MSent using v on M::Call::send {{ self.inflight }}\n{}",
            refinement("false")
                .replace(" late = self.late;", "")
                .replace(
                    "step M::Call::send -> A::Call::send(self);",
                    "step M::Call::send -> any;\nensure fair weak A::Call::send;"
                )
        ),
    )
    .unwrap_or_else(|refusals| panic!("{refusals:?}"))
    .with_abstract_hidden_initials(&abstract_document, &[false, true]);
    let mut request = safety_request(&world);
    request.limits = LIMITS;
    let outcome = check_timed_refinement(request, &Cancel::new()).expect("EN-6 takes it");
    assert!(
        matches!(outcome.safety, TimedSafetyOutcome::Holds { .. }),
        "{:?}",
        outcome.safety
    );
    assert!(
        matches!(
            outcome.liveness,
            Some(TimedLivenessHalf::Refused(
                ZoneCheckRefusal::HiddenCandidateLiveness
            ))
        ),
        "{:?}",
        outcome.liveness
    );
}
