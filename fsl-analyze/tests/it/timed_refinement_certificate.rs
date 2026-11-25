// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-244 "Refinement form" (TC-699) and FR-245 "Refinement form" (TC-700):
//! the zone certificate EN-6 returns with a timed refinement's `Holds`
//! (FSpec FR-430), and the core's check of it with `check_initial` and
//! `check_step` of its own.
//!
//! The certificates are the engine's own; each tampering edits one part of
//! a valid certificate.

use filament_exact::{Cancel, Integer};
use fsl_analyze::zone_check::{
    certificate_item, check_free_delay_refinement, TimedRefinementUndecided, TimedSafetyOutcome,
};
use fsl_replay::{
    check_free_delay_zone_certificate, check_zone_certificate, CertificateCheckLimits,
    CertificateLocus, CertificateMember, CertificateRejection, CertificateRule, IncompleteCause,
    InconclusiveCause, ModelCheckLimit, RefinementFailure, Taken, TerminalValue, ZoneCertificate,
    ZoneCertificateRefusal, ZoneCertificateRequest, ZoneCertificateVerdict,
};
use fsl_route::deadlock::{ItemObligation, ObligationKind};
use ix_trace_rs::trace;

use crate::certificate_rejection_support::written_cause;
use crate::provider_schema_support::{assert_valid, provider_definition};
use crate::timed_refinement::{
    certificate_request, dead_undo, dead_undo_with_idle, free_delay_world, obligation, refinement,
    req, safety, safety_request, treq, undo_world, unit, CONTRACTS,
};
use crate::timed_world::{document, refining, refining_from, renamed_package, Refining, DENSE};

type Certificate = ZoneCertificate<ItemObligation<&'static str>>;
type Checked = Result<ZoneCertificateVerdict, ZoneCertificateRefusal<ItemObligation<&'static str>>>;

/// The certificate of `world`'s holding safety half.
fn certified(world: &Refining) -> Certificate {
    match safety(world) {
        TimedSafetyOutcome::Holds { certificate, .. } => *certificate,
        other => panic!("the half holds: {other:?}"),
    }
}

/// FR-245's check of `certificate` under `request`, for `world`'s item.
fn check_with(
    world: &Refining,
    request: ZoneCertificateRequest<'_>,
    certificate: &Certificate,
) -> Checked {
    check_zone_certificate(
        request,
        &obligation(world),
        certificate_item,
        certificate,
        CertificateCheckLimits::default(),
        &Cancel::new(),
    )
}

fn check(world: &Refining, certificate: &Certificate) -> Checked {
    check_with(world, certificate_request(world), certificate)
}

fn verdict(world: &Refining, certificate: &Certificate) -> ZoneCertificateVerdict {
    check(world, certificate).expect("the check runs")
}

fn rejected(rule: CertificateRule, at: CertificateLocus) -> ZoneCertificateVerdict {
    ZoneCertificateVerdict::Rejected(CertificateRejection { rule, at })
}

/// FR-900-AC-1: two timed model sides with one mapped `send` operation.
fn timed_abstract_world() -> Refining {
    let abstract_model =
        renamed_package(&document(&["y"], &[("send", &["inflight"])]), "test/treq2");
    let concrete = treq();
    refining(
        &abstract_model,
        &concrete,
        &format!(
            "{DENSE}time A dense unit ms;\n\
             post ASend using v on A::Call::send {{ self.inflight }}\n\
             pre ASend2 using v on A::Call::send {{ self.y >= 1 ms }}\n\
             pre MSend using v on M::Call::send {{ self.x >= 2 ms }}\n\
             post MSent using v on M::Call::send {{ self.inflight }}\n{}",
            refinement("self.inflight")
        ),
    )
    .unwrap_or_else(|refusals| panic!("{refusals:?}"))
}

/// FR-900-AC-1: the abstract clock is present in the zone certificate and
/// FR-245 recomputes and accepts its guards and resets.
#[trace("TC-2538", "FR-900-AC-1")]
#[test]
fn a_timed_abstract_refinement_returns_a_replayable_certificate() {
    let world = timed_abstract_world();
    let certificate = certified(&world);
    assert_eq!(certificate.nodes[0].zone.dim(), 3);
    assert_eq!(
        verdict(&world, &certificate),
        ZoneCertificateVerdict::Accepted
    );

    let mut altered = certificate.clone();
    altered.nodes[certificate.initials[0]]
        .zone
        .reset(2, &Integer::one())
        .expect("the auxiliary clock is a zone clock");
    assert!(matches!(
        verdict(&world, &altered),
        ZoneCertificateVerdict::Rejected(_)
    ));
}

/// FR-900-AC-5 and FR-245: the free-delay concrete side contributes no
/// clock placement, while the abstract auxiliary clock is certified and
/// checked independently.
#[trace("TC-2539", "FR-900-AC-5", "FR-245-AC-1")]
#[test]
fn a_free_delay_refinement_certificate_has_only_the_abstract_clock() {
    let world = free_delay_world("self.y >= 0 ms");
    let certificate = match check_free_delay_refinement(safety_request(&world), &Cancel::new())
        .expect("the free-delay refinement is admitted")
        .safety
    {
        TimedSafetyOutcome::Holds { certificate, .. } => *certificate,
        other => panic!("the guard always holds: {other:?}"),
    };
    assert_eq!(certificate.nodes[0].zone.dim(), 2);
    let checked = check_free_delay_zone_certificate(
        certificate_request(&world),
        &obligation(&world),
        certificate_item,
        &certificate,
        CertificateCheckLimits::default(),
        &Cancel::new(),
    )
    .expect("FR-245 checks the free-delay certificate");
    assert_eq!(checked, ZoneCertificateVerdict::Accepted);

    let mut altered = certificate.clone();
    altered.nodes[certificate.initials[0]]
        .zone
        .reset(1, &Integer::from(-1_i64))
        .expect("the abstract clock is a zone clock");
    let rejected = check_free_delay_zone_certificate(
        certificate_request(&world),
        &obligation(&world),
        certificate_item,
        &altered,
        CertificateCheckLimits::default(),
        &Cancel::new(),
    )
    .expect("the altered certificate is checked");
    assert!(matches!(rejected, ZoneCertificateVerdict::Rejected(_)));
}

fn node(index: usize) -> CertificateLocus {
    CertificateLocus::ZoneNode {
        node: u64::try_from(index).expect("a small index"),
    }
}

fn edge(index: usize) -> CertificateLocus {
    CertificateLocus::ZoneEdge {
        edge: u64::try_from(index).expect("a small index"),
        verdict: None,
    }
}

/// The operation an edge's transition names.
fn operation(certificate: &Certificate, index: usize) -> &'static str {
    let transition = certificate.edges[index]
        .transition
        .as_ref()
        .expect("every refinement edge is a step");
    let operation = transition.operation();
    ["send", "enter", "undo"]
        .into_iter()
        .find(|name| operation.contains(&format!("/Call/{name}\"")))
        .unwrap_or_else(|| panic!("an operation of the fixture: {operation}"))
}

/// The reserved index of the locked region: `positions.len()`.
fn reserved(certificate: &Certificate) -> usize {
    certificate
        .refinement
        .as_ref()
        .expect("the refinement part")
        .positions
        .len()
}

fn holds(row: &str) -> Refining {
    unit(row).unwrap_or_else(|refusals| panic!("{refusals:?}"))
}

/// FR-244-AC-5 (TC-699): the safety half of FR-430-AC-1's `TReq` returns a
/// certificate with `refinement` present, one initial node, each node's
/// position an index into `positions`, every edge a discrete `send` step
/// that takes `send`, an `admitted` path from the initial node, and the
/// refinement item's obligation; FR-245 accepts it (FR-245-AC-8), and two
/// runs give equal certificates.
#[trace("TC-699", "FR-244-AC-5")]
#[trace("TC-1197", "FR-245-AC-7")]
#[trace("TC-700", "FR-245-AC-8")]
#[test]
fn the_treq_certificate_is_a_refinement_form_the_checker_accepts() {
    let world = holds("self.inflight");
    let certificate = certified(&world);
    let part = certificate
        .refinement
        .as_ref()
        .expect("the refinement part");
    // FSpec FR-418 `refinement_node`: the refinement node's id, the node
    // of the refinement record's occurrence.
    let ObligationKind::Refinement { occurrence, .. } = &obligation(&world).kind else {
        panic!("a refinement obligation");
    };
    assert_eq!(&part.refinement_node, occurrence.node());
    assert_eq!(certificate.initials.len(), 1);
    assert_eq!(certificate.obligation, obligation(&world));
    assert!(certificate
        .nodes
        .iter()
        .all(|listed| listed.automaton_state < part.positions.len() && !listed.locked));
    assert!(!certificate.edges.is_empty());
    assert_eq!(part.taken.len(), certificate.edges.len());
    for (index, taken) in part.taken.iter().enumerate() {
        assert_eq!(operation(&certificate, index), "send");
        assert!(
            matches!(taken, Some(Taken::Abstract(steps)) if steps.len() == 1),
            "{taken:?}"
        );
    }
    assert_eq!(
        certificate.admitted.as_ref().unwrap().start,
        certificate.initials[0]
    );
    assert_eq!(
        verdict(&world, &certificate),
        ZoneCertificateVerdict::Accepted
    );
    assert_eq!(certified(&world), certificate, "two runs agree");
}

/// FR-244-AC-5 and FR-430-AC-8 (TC-699, TC-700): FR-430-AC-6's `Holds`
/// certificate lists the post-state of `undo` as a `locked` node with the
/// reserved index, entered by the failing `undo` edge, which takes
/// `Taken::Failing` (FR-244-AC-8; the byte and `candidate_blocks` clauses of
/// that criterion are not covered here);
/// FR-245 accepts it. With the `locked` mark removed from that node it is
/// `Rejected` `StepFails` at the `undo` edge (FR-245-AC-8), written as FSpec
/// FR-331's zone edge step with the failing verdict.
#[trace("TC-699", "FR-244-AC-5")]
#[trace("TC-699", "FR-244-AC-8")]
#[trace("TC-1197", "FR-245-AC-7")]
#[trace("TC-700", "FR-245-AC-8")]
#[trace("TC-700", "FR-245-AC-12")]
#[trace("TC-1170", "FR-430-AC-8")]
#[test]
fn the_locked_undo_post_state_is_a_locked_entry_node() {
    let world = dead_undo(true);
    let certificate = certified(&world);
    let reserved = reserved(&certificate);
    let part = certificate
        .refinement
        .as_ref()
        .expect("the refinement part");
    let entries: Vec<usize> = (0..certificate.nodes.len())
        .filter(|&at| certificate.nodes[at].locked)
        .collect();
    let [entry] = entries.as_slice() else {
        panic!("one locked entry node: {entries:?}");
    };
    assert_eq!(certificate.nodes[*entry].automaton_state, reserved);
    let into: Vec<usize> = (0..certificate.edges.len())
        .filter(|&at| certificate.edges[at].to == *entry && certificate.edges[at].from != *entry)
        .collect();
    let [undo] = into.as_slice() else {
        panic!("one edge into the entry node: {into:?}");
    };
    assert_eq!(operation(&certificate, *undo), "undo");
    // FR-244-AC-8: the failing edge into the entry lists `Taken::Failing`.
    assert_eq!(part.taken[*undo], Some(Taken::Failing));
    let source = certificate.edges[*undo].from;
    assert!(certificate.nodes[source].automaton_state < reserved);
    assert_eq!(
        verdict(&world, &certificate),
        ZoneCertificateVerdict::Accepted
    );
    assert_eq!(certified(&world), certificate, "two runs agree");

    let mut unmarked = certificate.clone();
    unmarked.nodes[*entry].locked = false;
    let ZoneCertificateVerdict::Rejected(rejection) = verdict(&world, &unmarked) else {
        panic!("the unmarked entry rejects");
    };
    assert_eq!(rejection.rule, CertificateRule::StepFails);
    let at = u64::try_from(*undo).expect("a small index");
    assert!(
        matches!(
            &rejection.at,
            CertificateLocus::ZoneEdge { edge, verdict: Some(_) } if *edge == at
        ),
        "{:?}",
        rejection.at
    );
    let cause = written_cause(TerminalValue::Inconclusive(
        InconclusiveCause::CertificateRejected {
            rule: rejection.rule,
            at: rejection.at,
        },
    ));
    assert_valid(
        &provider_definition("CertificateRejected"),
        &cause,
        "the zone step-fails rejection",
    );
    assert_eq!(
        cause["at"],
        serde_json::json!({
            "edge": at,
            "verdict": {"kind": "StutterChanged", "position": at.to_string()},
        })
    );

    // FR-245-AC-12 (TC-700 step 9): `None` or another value for the failing
    // edge into the entry, `Taken::Failing` on an edge inside the locked
    // region and on a passing edge are each `Rejected` `PostDiffers` at that
    // edge.
    let tamper = |at: usize, taken: Option<Taken>| {
        let mut forged = certificate.clone();
        forged.refinement.as_mut().expect("the part").taken[at] = taken;
        verdict(&world, &forged)
    };
    for taken in [None, Some(Taken::Stutter)] {
        assert_eq!(
            tamper(*undo, taken.clone()),
            rejected(CertificateRule::PostDiffers, edge(*undo)),
            "{taken:?} on the failing edge"
        );
    }
    let passing: Vec<usize> = (0..certificate.edges.len())
        .filter(|&at| {
            at != *undo && certificate.nodes[certificate.edges[at].from].automaton_state < reserved
        })
        .collect();
    assert!(!passing.is_empty(), "a passing edge exists");
    for at in passing {
        assert_eq!(
            tamper(at, Some(Taken::Failing)),
            rejected(CertificateRule::PostDiffers, edge(at)),
            "Taken::Failing on passing edge {at}"
        );
    }
}

/// FR-244-AC-8 and FR-245-AC-12 (TC-699, TC-700): over `dead_undo` with a
/// further `idle` step the locked entry node has an edge of its own; the
/// certificate lists it with `None`, and `Taken::Failing` there is `Rejected`
/// `PostDiffers` at that edge.
#[trace("TC-699", "FR-244-AC-8")]
#[trace("TC-700", "FR-245-AC-12")]
#[test]
fn a_step_inside_the_locked_region_lists_no_taken() {
    let world = dead_undo_with_idle();
    let certificate = certified(&world);
    let reserved = reserved(&certificate);
    let inside: Vec<usize> = (0..certificate.edges.len())
        .filter(|&at| certificate.nodes[certificate.edges[at].from].automaton_state == reserved)
        .collect();
    assert!(
        !inside.is_empty(),
        "an edge inside the locked region exists"
    );
    let part = certificate.refinement.as_ref().expect("the part");
    for &at in &inside {
        assert_eq!(part.taken[at], None, "inside edge {at}");
    }
    assert_eq!(
        verdict(&world, &certificate),
        ZoneCertificateVerdict::Accepted
    );
    for at in inside {
        let mut forged = certificate.clone();
        forged.refinement.as_mut().expect("the part").taken[at] = Some(Taken::Failing);
        assert_eq!(
            verdict(&world, &forged),
            rejected(CertificateRule::PostDiffers, edge(at)),
            "Taken::Failing at edge {at}"
        );
    }
}

/// FR-245-AC-8 and FR-430-AC-8 (TC-700): the `TReq` certificate with its
/// `positions` emptied, or with no refinement part, is `Rejected`
/// `Malformed` at that member before any successor is recomputed; with one
/// edge's target position replaced, or its `taken` changed to `Stutter`, it
/// is `Rejected` `PostDiffers` at that edge.
#[trace("TC-1197", "FR-245-AC-7")]
#[trace("TC-700", "FR-245-AC-8")]
#[trace("TC-1170", "FR-430-AC-8")]
#[test]
fn a_tampered_treq_certificate_is_rejected() {
    let world = holds("self.inflight");
    let certificate = certified(&world);

    let mut no_positions = certificate.clone();
    no_positions
        .refinement
        .as_mut()
        .expect("the part")
        .positions
        .clear();
    assert_eq!(
        verdict(&world, &no_positions),
        rejected(
            CertificateRule::Malformed,
            CertificateLocus::CertificateMember(CertificateMember::Positions)
        )
    );
    let mut no_part = certificate.clone();
    no_part.refinement = None;
    assert_eq!(
        verdict(&world, &no_part),
        rejected(
            CertificateRule::Malformed,
            CertificateLocus::CertificateMember(CertificateMember::Refinement)
        )
    );

    // The first edge leaves the initial node for a node of another
    // position.
    let first = 0;
    let target = certificate.edges[first].to;
    let initial = certificate.initials[0];
    assert_ne!(
        certificate.nodes[target].automaton_state,
        certificate.nodes[initial].automaton_state
    );
    let mut moved = certificate.clone();
    moved.nodes[target].automaton_state = certificate.nodes[initial].automaton_state;
    assert_eq!(
        verdict(&world, &moved),
        rejected(CertificateRule::PostDiffers, edge(first))
    );
    let mut stutter = certificate.clone();
    stutter.refinement.as_mut().expect("the part").taken[first] = Some(Taken::Stutter);
    assert_eq!(
        verdict(&world, &stutter),
        rejected(CertificateRule::PostDiffers, edge(first))
    );
    // A passing step never enters the locked region: its target marked a
    // `locked` entry node with the reserved index is `PostDiffers`.
    let mut locked = certificate.clone();
    locked.nodes[target].automaton_state = reserved(&certificate);
    locked.nodes[target].locked = true;
    assert_eq!(
        verdict(&world, &locked),
        rejected(CertificateRule::PostDiffers, edge(first))
    );
}

/// FR-245-AC-8 (TC-700): FR-430-AC-7's refinement (abstract initial `sent`
/// true, so its half is `Violated`) with a forged certificate, the `TReq`
/// one, whose initial node lists the abstract state `sent` false and no
/// `locked` mark, is `Rejected` `InitialMismatch` at the initial node.
#[trace("TC-1197", "FR-245-AC-7")]
#[trace("TC-700", "FR-245-AC-8")]
#[test]
fn a_forged_initial_position_is_an_initial_mismatch() {
    let honest = holds("self.inflight");
    let mut forged = certified(&honest);
    let violated = refining_from(
        &req(),
        &treq(),
        &format!("{DENSE}{CONTRACTS}{}", refinement("self.inflight")),
        true,
    )
    .unwrap_or_else(|refusals| panic!("{refusals:?}"));
    assert!(matches!(
        safety(&violated),
        TimedSafetyOutcome::Violated { .. }
    ));
    forged.obligation = obligation(&violated);
    let initial = forged.initials[0];
    assert_eq!(
        verdict(&violated, &forged),
        rejected(CertificateRule::InitialMismatch, node(initial))
    );
}

/// FR-245-AC-8 (TC-700): the `TReq` certificate checked with an evaluation
/// meter of no work units stops `LimitReached` naming the evaluation meter
/// at the first `check_initial` or `check_step`, never `Rejected`; checked
/// against the refinement over another abstract package it refuses
/// `stale_dependency`.
#[trace("TC-1197", "FR-245-AC-7")]
#[trace("TC-700", "FR-245-AC-8")]
#[test]
fn a_meter_stop_is_no_rejection_and_another_package_is_stale() {
    let world = holds("self.inflight");
    let certificate = certified(&world);
    let mut request = certificate_request(&world);
    request.simulation.meter.work_units = 0;
    match check_with(&world, request, &certificate) {
        Ok(ZoneCertificateVerdict::Stopped(IncompleteCause::LimitReached {
            limit: ModelCheckLimit::EvaluationMeter { .. },
            ..
        })) => {}
        other => panic!("a meter stop: {other:?}"),
    }

    let other = dead_undo(true);
    let mut request = certificate_request(&world);
    let fsl_replay::CertifiedItem::Refinement {
        refinement,
        abstract_initial,
        ..
    } = request.item
    else {
        panic!("a refinement item");
    };
    request.item = fsl_replay::CertifiedItem::Refinement {
        refinement,
        abstract_package: &other.package,
        abstract_initial,
    };
    let refusal = check_with(&world, request, &certificate).expect_err("a stale certificate");
    assert!(
        matches!(refusal, ZoneCertificateRefusal::RefinementMismatch { .. }),
        "{refusal:?}"
    );
    assert_eq!(
        refusal.code(),
        fsl_foundation::diagnostic::Code::StaleDependency
    );
}

/// FR-245-AC-9's `Init3`: abstract `Req` refined by the dense `TInit`, its
/// three initial states (`sent`, here `inflight`, false, true, false) told
/// apart by `k`, here the flags `late` (the locked index) and `retried`
/// (index 2), which map to the abstract `false`; `send` guarded `x >= 1`
/// with precondition `not sent and not late` (`k <> 1`); the time
/// invariant `when late { x <= 0 }` (`k = 1`). With `locked` 0 (`Init3L`)
/// index 0 carries `late` and `sent` and index 1 neither. With no invariant
/// the locked index is time-divergent.
fn init3(locked: usize, invariant: bool) -> Refining {
    let invariant = if invariant {
        "time invariant Stuck when { self.late } { self.x <= 0 ms }\n"
    } else {
        ""
    };
    let contracts = format!(
        "post ASend using v on A::Call::send {{ self.inflight }}\n\
         pre MSend using v on M::Call::send {{ not self.inflight and not self.late and self.x >= 1 ms }}\n\
         post MSent using v on M::Call::send {{ self.inflight }}\n{invariant}"
    );
    let body = refinement("self.inflight")
        .replace("late = self.late", "late = false")
        .replace("retried = self.retried", "retried = false");
    let concrete = treq();
    let state = |index: usize| [index == locked, index == locked, index == 2];
    refining(&req(), &concrete, &format!("{DENSE}{contracts}{body}"))
        .unwrap_or_else(|refusals| panic!("{refusals:?}"))
        .with_concrete_initials(&concrete, &[state(0), state(1), state(2)])
}

/// FR-245-AC-9 and FR-244-AC-6 (TC-700, TC-699): `Init3`'s index 1 fails
/// `check_initial` and is time-locked, so EN-6 returns `Holds`; the
/// certificate lists three initial nodes, index 1's a `locked` entry node
/// with the reserved index and the others positioned, and the checker
/// accepts it. Each tampering is rejected at the initial node of the index
/// it names: the `locked` mark removed from index 1, or added to index 0,
/// is `InitialMismatch`; index 2's node replaced by one that does not
/// cover its symbolic state, or dropped, is `InitialMissing` at that
/// state.
#[trace("TC-700", "FR-245-AC-9")]
#[trace("TC-699", "FR-244-AC-6")]
#[test]
fn three_initial_states_one_time_locked_are_certified() {
    let world = init3(1, true);
    let certificate = certified(&world);
    let reserved = reserved(&certificate);
    let [first, locked, last] = certificate.initials[..] else {
        panic!("three initial nodes: {:?}", certificate.initials);
    };
    assert!(certificate.nodes[locked].locked);
    assert_eq!(certificate.nodes[locked].automaton_state, reserved);
    for positioned in [first, last] {
        assert!(!certificate.nodes[positioned].locked);
        assert!(certificate.nodes[positioned].automaton_state < reserved);
    }
    assert_eq!(
        verdict(&world, &certificate),
        ZoneCertificateVerdict::Accepted
    );
    assert_eq!(certified(&world), certificate, "two runs agree");

    let mut unmarked = certificate.clone();
    unmarked.nodes[locked].locked = false;
    assert_eq!(
        verdict(&world, &unmarked),
        rejected(CertificateRule::InitialMismatch, node(locked))
    );
    let mut marked = certificate.clone();
    marked.nodes[first].locked = true;
    assert_eq!(
        verdict(&world, &marked),
        rejected(CertificateRule::InitialMismatch, node(first))
    );
    let missing = CertificateLocus::ModelState(certificate.nodes[last].state_key.clone());
    let mut replaced = certificate.clone();
    replaced.initials[2] = first;
    assert_eq!(
        verdict(&world, &replaced),
        rejected(CertificateRule::InitialMissing, missing.clone())
    );
    let mut dropped = certificate.clone();
    dropped.initials.truncate(2);
    assert_eq!(
        verdict(&world, &dropped),
        rejected(CertificateRule::InitialMissing, missing)
    );
}

/// FR-245-AC-9 (TC-700): in `Init3L` index 0 is the locked one; the
/// certificate is accepted with `admitted` starting at index 1's node, and
/// `admitted` moved to start at index 0's locked node is `WitnessFails`
/// there.
#[trace("TC-700", "FR-245-AC-9")]
#[test]
fn a_locked_first_initial_state_does_not_decide_admitted() {
    let world = init3(0, true);
    let certificate = certified(&world);
    let [locked, second, _] = certificate.initials[..] else {
        panic!("three initial nodes: {:?}", certificate.initials);
    };
    assert!(certificate.nodes[locked].locked);
    assert_eq!(certificate.admitted.as_ref().unwrap().start, second);
    assert_eq!(
        verdict(&world, &certificate),
        ZoneCertificateVerdict::Accepted
    );
    let mut moved = certificate.clone();
    moved.admitted.as_mut().unwrap().start = locked;
    assert_eq!(
        verdict(&world, &moved),
        rejected(CertificateRule::WitnessFails, node(locked))
    );
}

/// FR-245-AC-9 (TC-700): without the time invariant `Init3`'s index 1 can
/// diverge, and EN-6 returns `Violated` with `InitialNotAbstract{initial:
/// 1}` and an empty prefix. `Init3`'s certificate forged for it, index 1's
/// `locked` node widened to cover its symbolic state, is `BadState` at that
/// node: time diverges from it.
#[trace("TC-700", "FR-245-AC-9")]
#[test]
fn a_forged_locked_initial_state_that_diverges_is_a_bad_state() {
    let free = init3(1, false);
    let TimedSafetyOutcome::Violated { counterexample, .. } = safety(&free) else {
        panic!("index 1 refutes where it can diverge");
    };
    assert!(counterexample.prefix.is_empty());
    assert_eq!(
        counterexample.refinement,
        Some(RefinementFailure::InitialNotAbstract { initial: 1 })
    );

    let mut forged = certified(&init3(1, true));
    let [first, locked, _] = forged.initials[..] else {
        panic!("three initial nodes");
    };
    forged.obligation = obligation(&free);
    forged.nodes[locked].zone = forged.nodes[first].zone.clone();
    assert_eq!(
        verdict(&free, &forged),
        rejected(CertificateRule::BadState, node(locked))
    );
}

/// FSpec FR-430 with FR-244: a half whose every initial state is
/// time-locked has no certificate: it settles TD-6's
/// `NoAdmittedBehaviour`.
#[trace("TC-1170", "FR-430-AC-7")]
#[test]
fn no_positioned_initial_state_is_no_admitted_behaviour() {
    let outcome = safety(&init3(1, true).with_concrete_initials(&treq(), &[[true, true, false]]));
    assert!(
        matches!(
            outcome,
            TimedSafetyOutcome::Undecided {
                cause: TimedRefinementUndecided::Zone(
                    fsl_analyze::zone_check::ZoneUndecided::NoAdmittedBehaviour
                ),
                ..
            }
        ),
        "{outcome:?}"
    );
}

/// FR-245 "Refinement form" (TC-700, SR-1757002 FND-001): a step-level
/// `locked` entry, entered by a failing `check_step`, whose forward closure
/// is not time-locked is `BadState`. FR-430-AC-6's `Holds` certificate,
/// forged for the variant whose `undo` post-state has no time invariant
/// (its half `Violated`), with the entry widened to the universe so it
/// covers that post-state, is rejected `BadState` at the entry.
#[trace("TC-1197", "FR-245-AC-7")]
#[trace("TC-700", "FR-245-AC-8")]
#[test]
fn a_forged_step_level_locked_entry_that_diverges_is_a_bad_state() {
    use fsl_replay::Zone;
    let live = undo_world(true, false);
    assert!(matches!(safety(&live), TimedSafetyOutcome::Violated { .. }));
    let mut forged = certified(&dead_undo(true));
    let entry = (0..forged.nodes.len())
        .find(|&at| forged.nodes[at].locked)
        .expect("the locked entry node");
    forged.obligation = obligation(&live);
    let dim = forged.nodes[entry].zone.dim();
    forged.nodes[entry].zone = Zone::universe(dim);
    assert_eq!(
        verdict(&live, &forged),
        rejected(CertificateRule::BadState, node(entry))
    );
}

/// FR-900-AC-2 with a hidden abstract field: the abstract clock guard of the
/// step the concrete send maps to is applied through the hidden candidate's
/// step rows, the half holds, and FR-245 recomputes the rows with its own
/// `check_step` and accepts the certificate; an edit to the abstract clock
/// is rejected.
#[trace("TC-2538", "FR-900-AC-2")]
#[test]
fn a_hidden_field_timed_abstract_refinement_returns_a_replayable_certificate() {
    let abstract_model =
        renamed_package(&document(&["y"], &[("send", &["inflight"])]), "test/treq2");
    let world = refining(
        &abstract_model,
        &treq(),
        &format!(
            "{DENSE}time A dense unit ms;\n\
             post ASend using v on A::Call::send {{ self.inflight }}\n\
             pre ASend2 using v on A::Call::send {{ self.y >= 1 ms }}\n\
             pre MSend using v on M::Call::send {{ self.x >= 2 ms }}\n\
             post MSent using v on M::Call::send {{ self.inflight }}\n{}",
            refinement("self.inflight").replace(" late = self.late;", "")
        ),
    )
    .unwrap_or_else(|refusals| panic!("{refusals:?}"));
    let certificate = certified(&world);
    assert_eq!(certificate.nodes[0].zone.dim(), 3);
    assert_eq!(
        verdict(&world, &certificate),
        ZoneCertificateVerdict::Accepted
    );

    let mut altered = certificate.clone();
    altered.nodes[certificate.initials[0]]
        .zone
        .reset(2, &Integer::one())
        .expect("the auxiliary clock is a zone clock");
    assert!(matches!(
        verdict(&world, &altered),
        ZoneCertificateVerdict::Rejected(_)
    ));
}

/// Two refinement positions whose mapped abstract steps differ in clock
/// behaviour: `send` (position 0) has the guard `y >= 1` and no reset, `enter`
/// (position 1) has the guard `y >= 2` and resets `y`.
fn two_positions_world() -> Refining {
    two_positions(true)
}

/// [`two_positions_world`], whose `enter` resets the abstract clock only with
/// `resets`.
fn two_positions(resets: bool) -> Refining {
    let written: &[&str] = if resets { &["late", "y"] } else { &["late"] };
    let entered = if resets {
        "self.late and self.y = 0 ms"
    } else {
        "self.late"
    };
    let abstract_model = renamed_package(
        &document(&["y"], &[("send", &["inflight"]), ("enter", written)]),
        "test/treq3",
    );
    let concrete = document(&["x"], &[("send", &["inflight"]), ("enter", &["late"])]);
    refining(
        &abstract_model,
        &concrete,
        &format!(
            "{DENSE}time A dense unit ms;\n\
             pre ASendable using v on A::Call::send {{ not self.inflight and self.y >= 1 ms }}\n\
             post ASend using v on A::Call::send {{ self.inflight }}\n\
             pre AEnterable using v on A::Call::enter {{ self.inflight and not self.late and self.y >= 2 ms }}\n\
             post AEntered using v on A::Call::enter {{ {entered} }}\n\
             pre MSend using v on M::Call::send {{ not self.inflight and self.x >= 2 ms }}\n\
             post MSent using v on M::Call::send {{ self.inflight }}\n\
             pre MEnter using v on M::Call::enter {{ self.inflight and not self.late and self.x >= 3 ms }}\n\
             post MEntered using v on M::Call::enter {{ self.late }}\n{}",
            refinement("self.inflight").replace(
                "step M::Call::send -> A::Call::send(self);",
                "step M::Call::send -> A::Call::send(self);\n\
                 step M::Call::enter -> A::Call::enter(self);"
            )
        ),
    )
    .unwrap_or_else(|refusals| panic!("{refusals:?}"))
}

/// FR-245 "Refinement form": the checker judges each node at its own
/// refinement position. The second position's mapped `enter` resets the
/// abstract clock, which the first position's `send` does not, so a checker
/// that read every node at position 0 would compute a larger successor zone
/// for the `enter` edge and reject the engine's own certificate.
#[trace("TC-700", "FR-245")]
#[test]
fn the_checker_reads_each_refinement_node_at_its_own_position() {
    let world = two_positions_world();
    let certificate = certified(&world);
    assert!(
        certificate
            .refinement
            .as_ref()
            .expect("a refinement part")
            .positions
            .len()
            >= 3,
        "the product has a position per concrete state"
    );
    assert_eq!(
        verdict(&world, &certificate),
        ZoneCertificateVerdict::Accepted
    );
}

/// The forged form of the same rule: a certificate whose nodes carry the
/// zones of the world where `enter` does not reset the abstract clock is
/// valid only for a checker that reads the second position's `enter` at the
/// first position, which has no such step. The checker judges each node at
/// its own position and rejects it.
#[trace("TC-700", "FR-245")]
#[test]
fn a_certificate_valid_only_at_position_zero_is_rejected() {
    let world = two_positions(true);
    let honest = certified(&world);
    let unreset = certified(&two_positions(false));
    assert_eq!(honest.nodes.len(), unreset.nodes.len());
    let mut forged = honest.clone();
    let mut changed = 0;
    for (node, other) in forged.nodes.iter_mut().zip(&unreset.nodes) {
        if node.zone != other.zone {
            node.zone = other.zone.clone();
            changed += 1;
        }
    }
    assert!(changed > 0, "the reset changes a node's zone");
    assert!(matches!(
        verdict(&world, &forged),
        ZoneCertificateVerdict::Rejected(_)
    ));
}
