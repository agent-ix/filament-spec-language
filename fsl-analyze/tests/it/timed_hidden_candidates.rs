// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-899 "Timed subjects": a timed abstract side with a hidden field whose
//! candidates disagree on the abstract clock behaviour. The candidates live
//! and die per zone: a candidate whose own time invariant stops holding
//! leaves the alive set by a death edge, a discrete step keeps the alive
//! post candidates of the rows whose guard holds, and FR-245's checker
//! recomputes both with code of its own.

use filament_exact::{Cancel, Integer};
use fsl_analyze::zone_check::{
    TimedLivenessHalf, TimedLivenessOutcome, TimedRefinementUndecided, TimedSafetyOutcome,
    ZoneCheckLimits, ZoneUndecided, certificate_item, check_timed_refinement,
    settle_timed_refinement,
};
use fsl_replay::{
    CertEdge, CertificateCheckLimits, CertificateRule, Certification, CertifiedItem,
    DivergentTarget, InconclusiveCause, ProofBasis, RefinementFailure, RefinementTraceSettlement,
    SilentMove, Taken, TemporalCounterexample, TerminalValue, ZoneCertificate,
    ZoneCertificateRequest, ZoneCertificateVerdict, check_zone_certificate,
};
use fsl_route::deadlock::ItemObligation;
use ix_trace_rs::trace;

use crate::timed_refinement::{obligation, refinement, replay, safety_request, the_refinement};
use crate::timed_world::{DENSE, Refining, document, refining, renamed_package, wire_field};

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
#[trace("TC-2538", "FR-899", "FSpec-FR-740-AC-4", "FR-245-AC-12")]
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

    // The certificate carries the union of rows across guard-split zones.
    // Every required row must be present, and each listed row must be
    // independently admitted for the candidate and transition.
    let part = certificate.refinement.as_ref().expect("refinement part");
    let (edge, rows) = part
        .taken
        .iter()
        .enumerate()
        .find_map(|(edge, taken)| match taken {
            Some(Taken::CandidateSteps(rows)) if rows.len() >= 4 => Some((edge, rows)),
            _ => None,
        })
        .expect("a partitioned edge carries both stutter and guarded rows");
    let mut missing_row_rejected = false;
    for remove in 0..rows.len() {
        let mut altered = certificate.clone();
        let Some(Taken::CandidateSteps(altered_rows)) =
            altered.refinement.as_mut().expect("refinement part").taken[edge].as_mut()
        else {
            panic!("the partitioned edge retains candidate rows");
        };
        altered_rows.remove(remove);
        if matches!(
            checked(&world, &altered),
            ZoneCertificateVerdict::Rejected(_)
        ) {
            missing_row_rejected = true;
            break;
        }
    }
    assert!(missing_row_rejected, "a missing required row is rejected");

    let mut forged = certificate.clone();
    let Some(Taken::CandidateSteps(forged_rows)) =
        forged.refinement.as_mut().expect("refinement part").taken[edge].as_mut()
    else {
        panic!("the partitioned edge retains candidate rows");
    };
    let mut extra = rows[0].clone();
    extra.pre_candidate = usize::MAX;
    forged_rows.push(extra);
    assert!(matches!(
        checked(&world, &forged),
        ZoneCertificateVerdict::Rejected(_)
    ));
}

/// Each hidden candidate takes its own mapped row and updates its own clock
/// block; the independently recomputed certificate retains both blocks.
#[trace(
    "TC-2538",
    "FSpec-FR-740-AC-4",
    "FSpec-FR-418-AC-22",
    "FR-244-AC-8",
    "FR-245-AC-12"
)]
#[test]
fn timed_hidden_candidates_with_different_resets_keep_independent_clock_blocks() {
    let abstract_document = abstract_document(&[
        ("send", &["inflight", "y", "h"]),
        ("send2", &["inflight", "h"]),
        ("poll", &[]),
    ]);
    let concrete = document(&["x"], &[("send", &["inflight"]), ("poll", &[])]);
    let world = refining(
        &abstract_document,
        &concrete,
        &format!(
            "{DENSE}time A dense unit ms;\n\
             pre ASendable using v on A::Call::send {{ self.h }}\n\
             post ASend using v on A::Call::send {{ self.inflight and self.y = 0 ms and not self.h }}\n\
             pre ASendable2 using v on A::Call::send2 {{ not self.h }}\n\
             post ASend2 using v on A::Call::send2 {{ self.inflight and not self.h }}\n\
             pre APoll using v on A::Call::poll {{ not self.h and self.y >= 1 ms }}\n\
             pre MSend using v on M::Call::send {{ not self.inflight and self.x >= 2 ms }}\n\
             post MSent using v on M::Call::send {{ self.inflight }}\n\
             pre MPoll using v on M::Call::poll {{ self.inflight }}\n{}",
            refinement("self.inflight")
                .replace(" late = self.late;", "")
                .replace(
                    "-> A::Call::send(self)",
                    "-> any;\nstep M::Call::poll -> A::Call::poll(self)",
                )
        ),
    )
    .unwrap_or_else(|refusals| panic!("{refusals:?}"))
    .with_abstract_hidden_initials(&abstract_document, &[false, true]);
    let certificate = certified(&world);
    assert_eq!(
        checked(&world, &certificate),
        ZoneCertificateVerdict::Accepted
    );
    let part = certificate.refinement.as_ref().expect("refinement part");
    assert!(part.positions.iter().any(|position| {
        position.abstract_states.len() == 2
            && position.abstract_states[0] == position.abstract_states[1]
            && position
                .candidate_blocks
                .as_ref()
                .is_some_and(|blocks| blocks.len() == 2)
    }));
    assert!(part.positions.iter().any(|position| {
        position.abstract_states.len() == 1
            && !position
                .candidate_blocks
                .as_ref()
                .expect("timed abstract blocks")
                .is_empty()
    }));
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

/// A concrete send reaches the candidate's dead region, after which the
/// concrete invariant (`x <= 3`) prevents time divergence. The locked entry
/// is therefore reachable and required by the checker.
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

/// A stutter beside a guarded step (the mixed rows of the world of
/// [`timed_hidden_a_stutter_and_a_guarded_step_split_the_zone`]) with
/// `rows` after the step row.
fn mixed_rows(rows: &str) -> Refining {
    let abstract_document = abstract_document(&[("send", &["inflight"])]);
    let concrete = document(&["x"], &[("send", &["inflight"])]);
    refining(
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
                    &format!("step M::Call::send -> any;\n{rows}")
                )
        ),
    )
    .unwrap_or_else(|refusals| panic!("{refusals:?}"))
    .with_abstract_hidden_initials(&abstract_document, &[false, true])
}

/// Candidates whose rows disagree (a stutter beside a guarded step) have their
/// liveness half decided, beside the safety half. FR-899 "Hidden fields": "The
/// half holds exactly when, for every concrete behaviour fair under `F_C`, some
/// candidate run along it satisfies `F_A`." With `F_C` empty the concrete
/// model may idle for ever, never taking `send`, which the abstract `send`
/// enables from `y >= 5`: violated. Assuming the concrete `send` weakly fair
/// leaves only behaviours that send, and the candidate that takes the abstract
/// `send` at `y >= 5` satisfies the claim: holds.
#[trace("TC-2538", "FR-899")]
#[test]
fn timed_hidden_a_stutter_and_a_guarded_step_decide_the_liveness_half() {
    let idle = liveness_violation(&mixed_rows("ensure fair weak A::Call::send;"));
    assert!(matches!(
        idle.refinement,
        Some(RefinementFailure::HiddenUnfair { .. })
    ));
    assert_liveness_holds(&mixed_rows(
        "assume fair weak M::Call::send;\nensure fair weak A::Call::send;",
    ));
}

/// Distinct abstract transitions from one hidden candidate can reach the same
/// abstract state. Their relation rows share one authoritative post ordinal;
/// liveness must retain that ordinal instead of ranking transitions as
/// separate target candidates.
#[trace("TC-2538", "FR-899", "FSpec-FR-740-AC-4")]
#[test]
fn timed_hidden_liveness_keeps_shared_post_ordinal_for_same_source_transitions() {
    let abstract_document = abstract_document(&[("send", &["inflight"]), ("send2", &["inflight"])]);
    let concrete = document(&["x"], &[("send", &["inflight"])]);
    let world = refining(
        &abstract_document,
        &concrete,
        &format!(
            "{DENSE}time A dense unit ms;\n\
             pre ASendable using v on A::Call::send {{ self.y >= 0 ms }}\n\
             post ASend using v on A::Call::send {{ self.inflight }}\n\
             pre ASendable2 using v on A::Call::send2 {{ self.y >= 0 ms }}\n\
             post ASend2 using v on A::Call::send2 {{ self.inflight }}\n\
             pre MSend using v on M::Call::send {{ self.x >= 1 ms }}\n\
             post MSent using v on M::Call::send {{ self.inflight }}\n{}",
            refinement("self.inflight")
                .replace(" late = self.late;", "")
                .replace(
                    "-> A::Call::send(self)",
                    "-> any;\nensure fair weak A::Call::send",
                )
        ),
    )
    .unwrap_or_else(|refusals| panic!("{refusals:?}"))
    .with_abstract_hidden_initials(&abstract_document, &[false, true]);

    let certificate = certified(&world);
    let part = certificate.refinement.as_ref().expect("refinement part");
    assert!(
        part.taken.iter().any(|taken| {
            let Some(Taken::CandidateSteps(rows)) = taken else {
                return false;
            };
            rows.iter().enumerate().any(|(index, row)| {
                rows[index + 1..].iter().any(|other| {
                    row.pre_candidate == other.pre_candidate
                        && row.post_candidate == other.post_candidate
                        && row.post_state == other.post_state
                        && row.transition.is_some()
                        && other.transition.is_some()
                        && row.transition != other.transition
                })
            })
        }),
        "two transitions from one candidate share the same post candidate ordinal"
    );

    let outcome = both_halves(&world);
    assert!(
        matches!(&outcome.liveness, Some(TimedLivenessHalf::Checked(_))),
        "candidate liveness checks the shared-ordinal relation rows: {:?}",
        outcome.liveness
    );
}

/// A looping world: the concrete `send` needs `x = 1`, resets `x` and is held
/// by `x <= 1`; the abstract `send` has no guard and does not reset `y`, so
/// `y` grows by one per loop. `invariants` are the abstract time invariants,
/// over `candidates`.
fn looping(candidates: &[([bool; 3], Option<bool>)], invariants: &str) -> Refining {
    let abstract_document = abstract_document(&[("send", &["inflight"])]);
    let concrete = document(&["x"], &[("send", &["inflight", "x"])]);
    refining(
        &abstract_document,
        &concrete,
        &format!(
            "{DENSE}time A dense unit ms;\n\
             post ASend using v on A::Call::send {{ self.inflight }}\n\
             pre ASendable using v on A::Call::send {{ self.y >= 0 ms }}\n\
             {invariants}\n\
             time invariant MInv when {{ true }} {{ self.x <= 1 ms }}\n\
             pre MSend using v on M::Call::send {{ self.x = 1 ms }}\n\
             post MSent using v on M::Call::send {{ self.inflight and self.x = 0 ms }}\n{}",
            refinement("self.inflight").replace(" late = self.late;", "")
        ),
    )
    .unwrap_or_else(|refusals| panic!("{refusals:?}"))
    .with_abstract_candidates(&abstract_document, candidates)
}

/// Abstract invariants are upper bounds, but the delay past one is found by
/// their negation, a lower-bound comparison: coverage must keep every
/// abstract invariant constant on both sides, or a zone with a smaller `y`
/// covers one closer to dying and the loop looks harmless.
#[trace("TC-2538", "FR-900")]
#[test]
fn timed_hidden_a_looping_delay_past_a_single_invariant_is_refuted() {
    let world = looping(
        &[([false; 3], Some(false))],
        "time invariant Y when { true } { self.y <= 3 ms }",
    );
    assert_time_invariant(safety_with_limits(&world));
}

#[trace("TC-2538", "FR-900")]
#[test]
fn timed_hidden_a_looping_delay_past_two_invariants_is_refuted() {
    let world = looping(
        &[([false; 3], Some(true)), ([false; 3], Some(false))],
        "time invariant Y3 when { self.h } { self.y <= 3 ms }\n\
         time invariant Y5 when { not self.h } { self.y <= 5 ms }",
    );
    assert_time_invariant(safety_with_limits(&world));
}

/// A landing state held by `x <= 5` with a step `b` (guard `x >= 3`) into a
/// state with no invariant: the abstract `send` guard fails at landing, and
/// time diverges after the landing by `b`, so the failure refutes.
fn landing(abstract_guard: &str) -> Refining {
    let abstract_document = abstract_document(&[("send", &["inflight"])]);
    let concrete = document(&["x"], &[("send", &["inflight"]), ("b", &["late"])]);
    refining(
        &abstract_document,
        &concrete,
        &format!(
            "{DENSE}time A dense unit ms;\n\
             post ASend using v on A::Call::send {{ self.inflight }}\n\
             pre ASendable using v on A::Call::send {{ {abstract_guard} }}\n\
             time invariant Cap when {{ self.inflight and not self.late }} {{ self.x <= 5 ms }}\n\
             pre MSend using v on M::Call::send {{ self.x >= 1 ms }}\n\
             post MSent using v on M::Call::send {{ self.inflight }}\n\
             pre MB using v on M::Call::b {{ self.inflight and not self.late and self.x >= 3 ms }}\n\
             post MBd using v on M::Call::b {{ self.late }}\n{}",
            refinement("self.inflight")
                .replace(" late = self.late;", "")
                .replace(
                    "step M::Call::send -> A::Call::send(self);",
                    "step M::Call::send -> A::Call::send(self);\nstep M::Call::b -> stutter;"
                )
        ),
    )
    .unwrap_or_else(|refusals| panic!("{refusals:?}"))
    .with_abstract_hidden_initials(&abstract_document, &[false])
}

/// A concrete `send` is enabled at zero, fails its mapped abstract guard over
/// the whole source zone, and lands in an invariant-locked successor. The
/// producer records the failure as completeness loss because no
/// time-divergent behaviour is admitted.
fn undelayed_failure_into_locked_successor() -> Refining {
    let abstract_document = abstract_document(&[("send", &["inflight"])]);
    let concrete = document(&["x"], &[("send", &["inflight"])]);
    refining(
        &abstract_document,
        &concrete,
        &format!(
            "{DENSE}time A dense unit ms;\n\
             post ASend using v on A::Call::send {{ self.inflight }}\n\
             pre ASendable using v on A::Call::send {{ self.y < 0 ms }}\n\
             time invariant MCap when {{ true }} {{ self.x <= 0 ms }}\n\
             pre MSend using v on M::Call::send {{ self.x >= 0 ms }}\n\
             post MSent using v on M::Call::send {{ self.inflight }}\n{}",
            refinement("self.inflight").replace(" late = self.late;", "")
        ),
    )
    .unwrap_or_else(|refusals| panic!("{refusals:?}"))
    .with_abstract_hidden_initials(&abstract_document, &[false])
}

/// An undelayed failing mapped edge into a time-locked successor does not
/// produce a refutation or a proof: the producer settles only the explicit
/// `NoAdmittedBehaviour` completeness outcome.
#[trace(
    "TC-699",
    "TC-700",
    "FSpec-FR-740-AC-6",
    "FSpec-FR-418-AC-22",
    "FR-244-AC-8",
    "FR-245-AC-12"
)]
#[test]
fn an_undelayed_failure_into_a_locked_successor_is_completeness_only() {
    let world = undelayed_failure_into_locked_successor();
    let mut request = safety_request(&world);
    request.limits = LIMITS;
    let outcome =
        check_timed_refinement(request, &Cancel::new()).expect("EN-6 checks the timed refinement");
    assert!(
        matches!(
            outcome.safety,
            TimedSafetyOutcome::Undecided {
                cause: TimedRefinementUndecided::Zone(ZoneUndecided::NoAdmittedBehaviour),
                ..
            }
        ),
        "the producer must preserve this as completeness loss: {outcome:?}"
    );
    let record = settle_timed_refinement(
        &outcome,
        &Cancel::new(),
        |_| panic!("an undecided producer outcome has no proof certificate"),
        |_| panic!("an undecided producer outcome has no counterexample"),
    );
    assert_eq!(
        record.value,
        TerminalValue::Inconclusive(InconclusiveCause::NoAdmittedBehaviour)
    );
    assert!(
        record.counterexample.is_none(),
        "the edge is not a refutation"
    );
}

/// The guard failure is judged on the delayed landing zone, so it refutes;
/// a certificate that locks the landing node on an undelayed zone is
/// rejected: the successor's delayed zone is not covered.
#[trace("TC-2538", "FR-900")]
#[test]
fn timed_hidden_a_landing_lock_is_judged_after_the_delay() {
    let failing = landing("self.y < 0 ms");
    let TimedSafetyOutcome::Violated { counterexample, .. } = safety_with_limits(&failing) else {
        panic!("time diverges after the landing, by b");
    };
    assert!(matches!(
        counterexample.refinement.as_ref(),
        Some(RefinementFailure::AbstractClockGuard { .. })
    ));
    let holding = landing("self.y >= 0 ms");
    let certificate = certified(&holding);
    let reserved = certificate
        .refinement
        .as_ref()
        .expect("a refinement part")
        .positions
        .len();
    let landed = certificate
        .edges
        .iter()
        .find(|edge| edge.transition.is_some() && edge.from == certificate.initials[0])
        .expect("the send edge")
        .to;
    let mut forged = certificate.clone();
    let node = &mut forged.nodes[landed];
    node.locked = true;
    node.automaton_state = reserved;
    // The undelayed landing zone: x at most 2.
    let two = Integer::from_big(forged.scale.as_big() + forged.scale.as_big());
    forged.nodes[landed]
        .zone
        .constrain(1, 0, &two, false)
        .expect("a clock");
    assert!(
        matches!(
            checked(&failing, &forged),
            ZoneCertificateVerdict::Rejected(_)
        ),
        "a lock on an undelayed zone is rejected"
    );
}

/// A world whose only concrete step is never enabled and whose abstract
/// candidate is dead at zero: time diverges by delay alone.
#[trace("TC-2538", "FR-900")]
#[test]
fn timed_hidden_a_dead_initial_state_with_pure_delay_is_refuted() {
    let abstract_document = abstract_document(&[("send", &["inflight"])]);
    let concrete = document(&["x"], &[("send", &["inflight"])]);
    let world = refining(
        &abstract_document,
        &concrete,
        &format!(
            "{DENSE}time A dense unit ms;\n\
             post ASend using v on A::Call::send {{ self.inflight }}\n\
             pre ASendable using v on A::Call::send {{ self.y >= 0 ms }}\n\
             time invariant Dead when {{ true }} {{ self.y < 0 ms }}\n\
             pre MSend using v on M::Call::send {{ self.x < 0 ms }}\n\
             post MSent using v on M::Call::send {{ self.inflight }}\n{}",
            refinement("self.inflight").replace(" late = self.late;", "")
        ),
    )
    .unwrap_or_else(|refusals| panic!("{refusals:?}"))
    .with_abstract_hidden_initials(&abstract_document, &[false]);
    let counterexample = assert_time_invariant(safety_with_limits(&world));
    assert!(counterexample.prefix.is_empty());
}

/// A dead initial state beside a healthy one, time-locked by the concrete
/// invariant `x <= 0`: a `locked` entry node the checker requires.
fn dead_locked_initial() -> Refining {
    let abstract_document = abstract_document(&[("send", &["inflight"])]);
    let concrete = document(&["x"], &[("send", &["inflight"])]);
    refining(
        &abstract_document,
        &concrete,
        &format!(
            "{DENSE}time A dense unit ms;\n\
             post ASend using v on A::Call::send {{ self.inflight }}\n\
             pre ASendable using v on A::Call::send {{ self.y >= 0 ms }}\n\
             time invariant Dead when {{ self.inflight and self.h }} {{ self.y < 0 ms }}\n\
             time invariant Lock when {{ self.inflight }} {{ self.x <= 0 ms }}\n\
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
    )
}

#[trace("TC-2538", "FR-900")]
#[test]
fn timed_hidden_a_dead_initial_entry_is_required_and_locked() {
    let world = dead_locked_initial();
    let certificate = certified(&world);
    assert_eq!(
        checked(&world, &certificate),
        ZoneCertificateVerdict::Accepted
    );
    let dead = *certificate.initials.last().expect("two initial nodes");
    assert!(certificate.nodes[dead].locked);
    let mut unlocked = certificate.clone();
    unlocked.nodes[dead].locked = false;
    match checked(&world, &unlocked) {
        ZoneCertificateVerdict::Rejected(rejection) => {
            assert_eq!(rejection.rule, CertificateRule::InitialMismatch);
        }
        other => panic!("an unlocked dead initial entry is rejected: {other:?}"),
    }
    let mut omitted = certificate.clone();
    omitted.initials.pop();
    match checked(&world, &omitted) {
        ZoneCertificateVerdict::Rejected(rejection) => {
            assert_eq!(rejection.rule, CertificateRule::InitialMissing);
        }
        other => panic!("an omitted dead initial entry is rejected: {other:?}"),
    }
}

/// A polling world over the hidden `h` (candidates `h = false` and
/// `h = true`, both initial): the concrete `poll` loop never takes anything,
/// the abstract `idle` is enabled only where `h` holds and `send` is enabled
/// while the call is not in flight. `invariants` are the abstract time
/// invariants and `ensures` the abstract fairness rows. `y` is never reset, so
/// a candidate with `y <= 2` dies two units into the loop.
fn polling(invariants: &str, ensures: &str) -> Refining {
    let abstract_document = abstract_document(&[("send", &["inflight"]), ("idle", &["inflight"])]);
    let concrete = document(&["x"], &[("send", &["inflight"]), ("poll", &["x"])]);
    refining(
        &abstract_document,
        &concrete,
        &format!(
            "{DENSE}time A dense unit ms;\n\
             pre ASendable using v on A::Call::send {{ not self.inflight }}\n\
             post ASend using v on A::Call::send {{ self.inflight }}\n\
             pre AIdleable using v on A::Call::idle {{ self.h }}\n\
             post AIdled using v on A::Call::idle {{ self.inflight = pre(self.inflight) }}\n\
             {invariants}\n\
             pre MSend using v on M::Call::send {{ not self.inflight }}\n\
             post MSent using v on M::Call::send {{ self.inflight }}\n\
             pre MPoll using v on M::Call::poll {{ not self.inflight }}\n\
             post MPolled using v on M::Call::poll {{ self.x = 0 ms }}\n{}",
            refinement("self.inflight")
                .replace(" late = self.late;", "")
                .replace(
                    "step M::Call::send -> A::Call::send(self);",
                    &format!("step M::Call::send -> A::Call::send(self);\nstep M::Call::poll -> stutter;\n{ensures}"),
                )
        ),
    )
    .unwrap_or_else(|refusals| panic!("{refusals:?}"))
    .with_abstract_hidden_initials(&abstract_document, &[false, true])
}

/// Both halves of `world` under the tests' limits.
fn both_halves(world: &Refining) -> fsl_analyze::zone_check::TimedRefinementOutcome<&'static str> {
    let mut request = safety_request(world);
    request.limits = LIMITS;
    check_timed_refinement(request, &Cancel::new()).expect("EN-6 takes the refinement")
}

/// The liveness half's violation beside a holding safety half.
fn liveness_violation(world: &Refining) -> TemporalCounterexample {
    let outcome = both_halves(world);
    assert!(
        matches!(outcome.safety, TimedSafetyOutcome::Holds { .. }),
        "{:?}",
        outcome.safety
    );
    match outcome.liveness {
        Some(TimedLivenessHalf::Checked(TimedLivenessOutcome::Violated {
            counterexample, ..
        })) => *counterexample,
        other => panic!("the liveness half is violated: {other:?}"),
    }
}

fn assert_liveness_holds(world: &Refining) {
    let outcome = both_halves(world);
    assert!(
        matches!(outcome.safety, TimedSafetyOutcome::Holds { .. }),
        "{:?}",
        outcome.safety
    );
    assert!(
        matches!(
            outcome.liveness,
            Some(TimedLivenessHalf::Checked(
                TimedLivenessOutcome::Holds { .. }
            ))
        ),
        "{:?}",
        outcome.liveness
    );
}

const DIES_TRUE: &str = "time invariant IT when { self.h } { self.y <= 2 ms }";
const DIES_FALSE: &str = "time invariant IF when { not self.h } { self.y <= 2 ms }";
const FAIR_IDLE: &str = "ensure fair weak A::Call::idle;";

/// FR-899 "Hidden fields": the half holds when some candidate run along every
/// fair concrete behaviour satisfies `F_A`. The `h` candidate dies two units
/// in, and the surviving `not h` candidate never enables `idle`, so its run is
/// fair: both halves hold, the safety certificate carries the death edge and
/// FR-245's checker accepts it.
#[trace("TC-2538", "FR-899")]
#[test]
fn timed_hidden_a_survivor_that_satisfies_the_claim_holds_both_halves() {
    let world = polling(DIES_TRUE, FAIR_IDLE);
    assert_liveness_holds(&world);
    let certificate = certified(&world);
    assert!(
        certificate
            .edges
            .iter()
            .any(|edge| matches!(&edge.silent, Some(SilentMove::Dies { .. }))),
        "a candidate dies"
    );
    assert_eq!(
        checked(&world, &certificate),
        ZoneCertificateVerdict::Accepted
    );
    // ADR-020 CT-4: no core checker covers the liveness half.
    let outcome = both_halves(&world);
    let record = settle_timed_refinement(
        &outcome,
        &Cancel::new(),
        |certificate| {
            check_zone_certificate(
                request(&world),
                &obligation(&world),
                certificate_item,
                certificate,
                CertificateCheckLimits::default(),
                &Cancel::new(),
            )
        },
        |_| panic!("a holding half replays no lasso"),
    );
    assert_eq!(
        record.value,
        TerminalValue::Proved {
            basis: ProofBasis::Exhaustive,
            certification: Certification::Uncertified,
        }
    );
}

/// The surviving candidate is the only run once the other is dead: it keeps
/// `send` enabled and the polling loop never takes it. The refuting lasso
/// passes the death edge, so its first delay runs past `y = 2`.
#[trace("TC-2538", "FR-899")]
#[test]
fn timed_hidden_a_survivor_that_violates_the_claim_is_a_lasso_through_the_death() {
    let world = polling(DIES_TRUE, "ensure fair weak A::Call::send;");
    let counterexample = liveness_violation(&world);
    assert!(matches!(
        counterexample.refinement,
        Some(RefinementFailure::HiddenUnfair { .. })
    ));
    let first = counterexample
        .prefix
        .first()
        .and_then(|step| step.delay.clone())
        .expect("a delayed first step");
    assert!(
        first > filament_exact::Rational::from_integer(Integer::from(2_i64)),
        "the lasso passes the death at y = 2: {first:?}"
    );
    assert_reproduced(&world, &counterexample);
}

/// A free poll may occur any time after 1 and is bounded by 3. The candidate
/// dies just after its invariant boundary at 2, so replay must retain the
/// death instant on a loop whose first delay is greater than 2.
#[trace(
    "TC-2538",
    "FSpec-FR-740-AC-4",
    "FSpec-FR-418-AC-22",
    "FR-244-AC-8",
    "FR-245-AC-12"
)]
#[test]
fn free_poll_replays_a_lasso_past_the_candidate_death_instant() {
    let world = free_polling(DIES_FALSE, FAIR_IDLE);
    let counterexample = liveness_violation(&world);
    let delay = counterexample
        .prefix
        .first()
        .and_then(|step| step.delay.clone())
        .expect("first poll has a delay");
    assert!(delay > filament_exact::Rational::from_integer(Integer::from(2_i64)));
    assert_reproduced(&world, &counterexample);
}

fn free_polling(invariants: &str, ensures: &str) -> Refining {
    let abstract_document = abstract_document(&[("send", &["inflight"]), ("idle", &["inflight"])]);
    let concrete = document(&["x"], &[("send", &["inflight"]), ("poll", &["x"])]);
    refining(
        &abstract_document,
        &concrete,
        &format!(
            "{DENSE}time A dense unit ms;\n\
             pre ASendable using v on A::Call::send {{ not self.inflight }}\n\
             post ASend using v on A::Call::send {{ self.inflight }}\n\
             pre AIdleable using v on A::Call::idle {{ self.h }}\n\
             post AIdled using v on A::Call::idle {{ self.inflight = pre(self.inflight) }}\n\
             {invariants}\n\
             pre MSend using v on M::Call::send {{ not self.inflight }}\n\
             post MSent using v on M::Call::send {{ self.inflight }}\n\
             time invariant MBound when {{ true }} {{ self.x <= 3 ms }}\n\
             pre MPoll using v on M::Call::poll {{ not self.inflight and self.x >= 1 ms }}\n\
             post MPolled using v on M::Call::poll {{ self.x = 0 ms }}\n{}",
            refinement("self.inflight")
                .replace(" late = self.late;", "")
                .replace(
                    "step M::Call::send -> A::Call::send(self);",
                    &format!("step M::Call::send -> A::Call::send(self);\nstep M::Call::poll -> stutter;\n{ensures}"),
                )
        ),
    )
    .unwrap_or_else(|refusals| panic!("{refusals:?}"))
    .with_abstract_hidden_initials(&abstract_document, &[false, true])
}

/// The soundness case: the `not h` candidate would run fair for ever (it never
/// enables `idle`), but it dies at `y > 2`, and the `h` candidate that
/// survives enables `idle` for ever. With nobody dying the half holds; with
/// the death it is violated.
#[trace("TC-2538", "FR-899")]
#[test]
fn timed_hidden_a_dead_candidate_cannot_satisfy_the_liveness_claim() {
    assert_liveness_holds(&polling("", FAIR_IDLE));
    let world = polling(DIES_FALSE, FAIR_IDLE);
    let counterexample = liveness_violation(&world);
    assert!(matches!(
        counterexample.refinement,
        Some(RefinementFailure::HiddenUnfair { .. })
    ));
    assert_reproduced(&world, &counterexample);
}

/// FR-145 reproduces the refutation: replay drops each candidate by its own
/// invariant along the loop's delays, so a dead candidate is no fair run.
fn assert_reproduced(world: &Refining, counterexample: &TemporalCounterexample) {
    assert!(
        matches!(
            replay(world, counterexample),
            RefinementTraceSettlement::Reproduced {
                failure: RefinementFailure::HiddenUnfair { .. },
                ..
            }
        ),
        "{:?}",
        replay(world, counterexample)
    );
}

/// The concrete `poll` fires every 3 units (guard `x >= 3`, invariant
/// `x <= 3`, resets `x`) and maps to the abstract `tick`, which resets `y` and
/// leaves `h` free: two post rows, `h' = true` and `h' = false`. `idle` needs
/// `h`; `invariant` is the abstract time invariant of the `not h` candidate.
fn ticking(invariant: &str) -> Refining {
    let abstract_document = abstract_document(&[("tick", &["y", "h"]), ("idle", &["inflight"])]);
    let concrete = document(&["x"], &[("poll", &["x"])]);
    refining(
        &abstract_document,
        &concrete,
        &format!(
            "{DENSE}time A dense unit ms;\n\
             pre ATickable using v on A::Call::tick {{ self.y >= 0 ms }}\n\
             post ATick using v on A::Call::tick {{ self.y = 0 ms }}\n\
             pre AIdleable using v on A::Call::idle {{ self.h }}\n\
             post AIdled using v on A::Call::idle {{ self.inflight = pre(self.inflight) }}\n\
             {invariant}\n\
             time invariant MInv when {{ true }} {{ self.x <= 3 ms }}\n\
             pre MPoll using v on M::Call::poll {{ self.x >= 3 ms }}\n\
             post MPolled using v on M::Call::poll {{ self.x = 0 ms }}\n{}",
            refinement("self.inflight")
                .replace(" late = self.late;", "")
                .replace(
                    "step M::Call::send -> A::Call::send(self);",
                    &format!("step M::Call::poll -> A::Call::tick(self);\n{FAIR_IDLE}"),
                )
        ),
    )
    .unwrap_or_else(|refusals| panic!("{refusals:?}"))
    .with_abstract_hidden_initials(&abstract_document, &[false, true])
}

/// A candidate born at every tick and dead before the next one is no witness
/// of an infinite run, and the search cannot chain a different candidate into
/// each iteration: with `y <= 2` the `not h` run dies at `y > 2` before the
/// tick at `y = 3`, so every infinite run is the `h` run, which enables `idle`
/// for ever; with `y <= 3` the `not h` run is reborn at each tick and never
/// enables `idle`. Both are replayed.
#[trace("TC-2538", "FR-899")]
#[test]
fn timed_hidden_a_candidate_born_at_each_tick_and_dead_before_the_next_is_no_run() {
    let dying = ticking("time invariant IF when { not self.h } { self.y <= 2 ms }");
    let counterexample = liveness_violation(&dying);
    let Some(RefinementFailure::HiddenUnfair { constraint }) = &counterexample.refinement else {
        panic!("{:?}", counterexample.refinement);
    };
    assert_eq!(constraint.operation().name(), "idle");
    assert_reproduced(&dying, &counterexample);
    let surviving = ticking("time invariant IF when { not self.h } { self.y <= 3 ms }");
    assert_liveness_holds(&surviving);
}
