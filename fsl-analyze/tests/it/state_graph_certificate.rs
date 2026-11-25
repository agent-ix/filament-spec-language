// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-169's state-graph certificate (ADR-022 GC-1 to GC-4): the certificate
//! EN-1 returns with `Witnessed` and `Holds`, and the core checker
//! `fsl_replay::check_state_graph`, which accepts the engine's certificates
//! of ADR-022 §7's true claims and rejects altered or false ones.
//!
//! Every expected closure, rank, count and locus is computed here from a
//! breadth-first enumeration of the subject's `ModelSystem`
//! (`crate::state_graph::reachable`) and each world's arithmetic, not read
//! from the engine.
//!
//! The terminal record each check settles to (FR-169's map) is
//! `state_graph_settle`'s. Not covered: the reduced-run `Uncertified` proof,
//! since no reduction exists yet.

use std::collections::{BTreeMap, BTreeSet};

use filament_exact::{Cancel, CancelCause, ObjectReference};
use fsl_eval::simulation::model::ModelSimState;
use fsl_eval::simulation::StateKey;
use fsl_eval::simulation::TransitionSystem;
use fsl_replay::{
    check_state_graph, CertificateLocus, CertificateRejection, CertificateRule, ClosureRefusal,
    IncompleteCause, ModelCheckLimit, ModelCheckLimits, StateGraphCertificate,
    StateGraphCertificateRequest, StateGraphOutcome, TemporalCertificateCheck,
};
use ix_trace_rs::trace;

use crate::counter::UNLIMITED;
use crate::graph_world::{
    config_kernel_world, config_kernel_world_over, config_version, config_world,
    counter_kernel_world, fork_kernel_world, fork_world, game_world, job_world, World,
    CONFIG_CONTRACT, DIVIDES, REACHES_TWO,
};
use crate::state_graph::{no_samples, reachable, run, system, AB};

/// Check `certificate` for `claim`'s instance bound to `over` over `world`
/// with universe `keys`, under `limits`.
fn check(
    world: &World,
    keys: &[&str],
    claim: &str,
    over: Option<ObjectReference>,
    certificate: &StateGraphCertificate,
    limits: &ModelCheckLimits,
) -> TemporalCertificateCheck {
    check_cancelled(
        world,
        keys,
        claim,
        over,
        certificate,
        limits,
        &Cancel::new(),
    )
}

/// [`check`] polling `cancel`.
fn check_cancelled(
    world: &World,
    keys: &[&str],
    claim: &str,
    over: Option<ObjectReference>,
    certificate: &StateGraphCertificate,
    limits: &ModelCheckLimits,
    cancel: &Cancel,
) -> TemporalCertificateCheck {
    let request = StateGraphCertificateRequest {
        subject: world.subject(keys, UNLIMITED),
        claim: world.claim(claim),
        over,
    };
    check_state_graph(&request, certificate, limits, cancel).expect("the request admits")
}

fn rejected(rule: CertificateRule, at: StateKey) -> TemporalCertificateCheck {
    TemporalCertificateCheck::Rejected(CertificateRejection {
        rule,
        at: CertificateLocus::ModelKey(at),
    })
}

/// The state key of the one reachable state where `select` holds.
pub(crate) fn state_where(
    world: &World,
    keys: &[&str],
    select: impl Fn(&ModelSimState) -> bool,
) -> StateKey {
    let found: Vec<StateKey> = reachable(world, keys)
        .0
        .into_iter()
        .filter(|(_, state)| select(state))
        .map(|(state_key, _)| state_key)
        .collect();
    assert_eq!(found.len(), 1, "one state selected");
    found[0].clone()
}

/// §7.1's state `(va, vb)`.
pub(crate) fn config(world: &World, va: i64, vb: i64) -> StateKey {
    state_where(world, AB, |state| {
        world.value(state, "a", "versionNumber") == va
            && world.value(state, "b", "versionNumber") == vb
    })
}

/// A certificate's ranks by state.
pub(crate) fn ranks_of(certificate: &StateGraphCertificate) -> BTreeMap<StateKey, u32> {
    certificate
        .ranks
        .iter()
        .map(|(index, d)| {
            let at = usize::try_from(*index).expect("fits");
            (certificate.closure[at].clone(), *d)
        })
        .collect()
}

/// The certificate over `closure` with `ranks`, re-indexed; a rank of a
/// state outside `closure` is dropped.
pub(crate) fn with_ranks(
    closure: Vec<StateKey>,
    ranks: &BTreeMap<StateKey, u32>,
) -> StateGraphCertificate {
    let ranks = closure
        .iter()
        .enumerate()
        .filter_map(|(index, state)| {
            ranks
                .get(state)
                .map(|d| (u64::try_from(index).expect("fits"), *d))
        })
        .collect();
    StateGraphCertificate {
        closure,
        ranks,
        counts: Vec::new(),
    }
}

/// `ReachesTwo`'s instance for `c = a` with phase 0 off: its binding and
/// the certificate of its `Witnessed`.
fn reaches_two(world: &World) -> (Option<ObjectReference>, StateGraphCertificate) {
    let result = run(world.request(AB, vec![world.item("ReachesTwo")], no_samples()));
    let instance = &result.items[0][0];
    let StateGraphOutcome::Witnessed { certificate, .. } = &instance.outcome else {
        panic!("witnessed: {:?}", instance.outcome);
    };
    (instance.over.clone(), certificate.clone())
}

pub(crate) const GUARDED: &str =
    "always possible Guarded using v over (x: G::Game) from (x.phase != 2) { x.phase = 3 }\n";

/// FR-169-AC-9 (check half): `ReachesTwo` for `c = a` returns a certificate
/// whose closure is §7.1's 9 states and whose ranks are `d = 0` at `va = 2`,
/// 1 at `va = 1` and 2 at `va = 0`, and `check_state_graph` accepts it;
/// §7.2's `CanStillWin` `from (x.phase != Lost)` returns ranks `Won` 0,
/// `Mid` 1, `Start` 2 and none for `Lost`, accepted.
#[trace("TC-643", "FR-169-AC-9")]
#[test]
fn the_checker_accepts_the_engines_rank_certificates() {
    let world = config_world(REACHES_TWO, &[(0, 0)]);
    let (over, certificate) = reaches_two(&world);
    let states = reachable(&world, AB).0;
    assert_eq!(states.len(), 9);
    assert_eq!(
        certificate.closure,
        states.keys().cloned().collect::<Vec<_>>()
    );
    let expected: BTreeMap<StateKey, u32> = states
        .iter()
        .map(|(state_key, state)| {
            let d = match world.value(state, "a", "versionNumber") {
                2 => 0,
                1 => 1,
                0 => 2,
                other => panic!("va = {other}"),
            };
            (state_key.clone(), d)
        })
        .collect();
    assert_eq!(ranks_of(&certificate), expected);
    assert!(certificate.counts.is_empty());
    assert_eq!(
        check(
            &world,
            AB,
            "ReachesTwo",
            over,
            &certificate,
            &ModelCheckLimits::default()
        ),
        TemporalCertificateCheck::Accepted
    );

    let game = game_world(false, false, GUARDED);
    let result = run(game.request(&["g"], vec![game.item("Guarded")], no_samples()));
    let instance = &result.items[0][0];
    let StateGraphOutcome::Holds { certificate } = &instance.outcome else {
        panic!("holds: {:?}", instance.outcome);
    };
    let phase = |value| {
        state_where(&game, &["g"], |state| {
            game.value(state, "g", "phase") == value
        })
    };
    // Start 0, Mid 1, Lost 2, Won 3.
    assert_eq!(
        ranks_of(certificate),
        BTreeMap::from([(phase(3), 0), (phase(1), 1), (phase(0), 2)])
    );
    assert_eq!(certificate.closure.len(), 4);
    assert_eq!(
        check(
            &game,
            &["g"],
            "Guarded",
            instance.over.clone(),
            certificate,
            &ModelCheckLimits::default()
        ),
        TemporalCertificateCheck::Accepted
    );
}

/// The successor state keys of each reachable state of `world`, one per edge.
fn successors(world: &World, keys: &[&str]) -> BTreeMap<StateKey, Vec<StateKey>> {
    let system = system(world, keys);
    reachable(world, keys)
        .0
        .into_iter()
        .map(|(state_key, state)| {
            let next = system
                .successors(&state)
                .expect("expands")
                .successors
                .into_iter()
                .map(|(_, next)| system.state_key(&next).expect("encodes").0.into())
                .collect();
            (state_key, next)
        })
        .collect()
}

/// FR-169-AC-9 (check half): §7.3's sequenced `InOneWay` returns count 1 at
/// every node of `H` with a reverse topological order, accepted.
#[trace("TC-643", "FR-169-AC-9")]
#[test]
fn the_checker_accepts_the_engines_count_certificate() {
    let world = job_world(true, false);
    let result = run(world.request(&["j"], vec![world.item("InOneWay")], no_samples()));
    let instance = &result.items[0][0];
    let StateGraphOutcome::Holds { certificate } = &instance.outcome else {
        panic!("holds: {:?}", instance.outcome);
    };
    let graph = successors(&world, &["j"]);
    // Sequenced: 000 -> 100 -> 110 -> 111, every node reaching `done`.
    assert_eq!(graph.len(), 4);
    assert_eq!(
        certificate.closure,
        graph.keys().cloned().collect::<Vec<_>>()
    );
    assert_eq!(certificate.counts.len(), 4);
    let order: BTreeMap<StateKey, u64> = certificate
        .counts
        .iter()
        .map(|(index, count, order)| {
            assert_eq!(*count, 1);
            (
                certificate.closure[usize::try_from(*index).expect("fits")].clone(),
                *order,
            )
        })
        .collect();
    let done = state_where(&world, &["j"], |state| world.value(state, "j", "done") == 1);
    for (source, targets) in &graph {
        if *source == done {
            continue;
        }
        for target in targets {
            assert!(order[target] < order[source], "reverse topological order");
        }
    }
    assert_eq!(
        check(
            &world,
            &["j"],
            "InOneWay",
            instance.over.clone(),
            certificate,
            &ModelCheckLimits::default()
        ),
        TemporalCertificateCheck::Accepted
    );
}

/// FR-169-AC-10 (check half): AC-9's `ReachesTwo` certificate with `(2, 2)`
/// removed is rejected `SuccessorMissing` at the first, in certificate
/// order, of the states that reach it; with the rank of `(1, 0)` changed to
/// 2 it is rejected `RankBroken` at `(0, 0)`, the one state left with no
/// successor ranked one less (`(1, 0)` itself still has `(1, 1)` at rank
/// 1); with `(0, 0)` unranked it is rejected `RankMissing` at `(0, 0)`; and
/// checked with `max_states` 2 it stops naming `max_states` and 2.
#[trace("TC-644", "FR-169-AC-10")]
#[test]
fn the_checker_rejects_altered_rank_certificates() {
    let world = config_world(REACHES_TWO, &[(0, 0)]);
    let (over, certificate) = reaches_two(&world);
    let ranks = ranks_of(&certificate);
    let checked = |certificate: &StateGraphCertificate, limits: &ModelCheckLimits| {
        check(&world, AB, "ReachesTwo", over.clone(), certificate, limits)
    };
    let defaults = ModelCheckLimits::default();

    let removed = config(&world, 2, 2);
    let closure: Vec<_> = certificate
        .closure
        .iter()
        .filter(|&state| *state != removed)
        .cloned()
        .collect();
    let first_reaching = config(&world, 1, 2).min(config(&world, 2, 1));
    assert_eq!(
        checked(&with_ranks(closure, &ranks), &defaults),
        rejected(CertificateRule::SuccessorMissing, first_reaching)
    );

    let mut changed = ranks.clone();
    changed.insert(config(&world, 1, 0), 2);
    assert_eq!(
        checked(
            &with_ranks(certificate.closure.clone(), &changed),
            &defaults
        ),
        rejected(CertificateRule::RankBroken, config(&world, 0, 0))
    );

    let mut unranked = ranks.clone();
    unranked.remove(&config(&world, 0, 0));
    assert_eq!(
        checked(
            &with_ranks(certificate.closure.clone(), &unranked),
            &defaults
        ),
        rejected(CertificateRule::RankMissing, config(&world, 0, 0))
    );

    let mut limits = ModelCheckLimits::default();
    limits.limits.max_states = 2;
    let stopped = TemporalCertificateCheck::Stopped(IncompleteCause::LimitReached {
        limit: ModelCheckLimit::MaxStates,
        value: 2,
        required: None,
    });
    assert_eq!(checked(&certificate, &limits), stopped);
}

/// FR-169-AC-14 (TC-1197): each malformed closure, rank or count shape is
/// rejected at its typed state or entry locus before the certificate can
/// prove the item.
#[trace("TC-1197", "FR-169-AC-14")]
#[test]
fn malformed_state_graph_certificate_members_have_typed_loci() {
    let world = config_world(REACHES_TWO, &[(0, 0)]);
    let (over, certificate) = reaches_two(&world);
    let check_reaches = |candidate: &StateGraphCertificate| {
        check(
            &world,
            AB,
            "ReachesTwo",
            over.clone(),
            candidate,
            &ModelCheckLimits::default(),
        )
    };

    let mut swapped = certificate.clone();
    swapped.closure.swap(0, 1);
    assert_eq!(
        check_reaches(&swapped),
        rejected(CertificateRule::Malformed, certificate.closure[0].clone())
    );

    let mut duplicate = certificate.clone();
    duplicate.closure[1] = duplicate.closure[0].clone();
    assert_eq!(
        check_reaches(&duplicate),
        rejected(CertificateRule::Malformed, certificate.closure[0].clone())
    );

    let mut invalid_key = certificate.clone();
    invalid_key.closure[0] = StateKey::from_canonical_bytes(b"not-a-state-key".to_vec());
    assert_eq!(
        check_reaches(&invalid_key),
        rejected(CertificateRule::Malformed, invalid_key.closure[0].clone())
    );

    let mut out_of_range_rank = certificate.clone();
    let closure_len = u64::try_from(certificate.closure.len()).expect("closure length");
    out_of_range_rank.ranks.push((closure_len, 0));
    assert_eq!(
        check_reaches(&out_of_range_rank),
        TemporalCertificateCheck::Rejected(CertificateRejection {
            rule: CertificateRule::Malformed,
            at: CertificateLocus::Entry { index: closure_len },
        })
    );

    let mut descending = certificate.clone();
    descending.ranks.reverse();
    let second = usize::try_from(descending.ranks[1].0).expect("rank index");
    assert_eq!(
        check_reaches(&descending),
        rejected(
            CertificateRule::Malformed,
            certificate.closure[second].clone()
        )
    );

    let world = job_world(true, false);
    let result = run(world.request(&["j"], vec![world.item("InOneWay")], no_samples()));
    let instance = &result.items[0][0];
    let StateGraphOutcome::Holds { certificate } = &instance.outcome else {
        panic!("the sequenced count claim holds: {:?}", instance.outcome);
    };
    let count_index = usize::try_from(certificate.counts[0].0).expect("count index");
    let state = certificate.closure[count_index].clone();
    let mut bad_count = certificate.clone();
    bad_count.counts[0].1 = 3;
    assert_eq!(
        check(
            &world,
            &["j"],
            "InOneWay",
            instance.over.clone(),
            &bad_count,
            &ModelCheckLimits::default()
        ),
        rejected(CertificateRule::Malformed, state)
    );
}

/// FR-169-AC-15 (TC-644): certificates reject the other proof form's list,
/// while the engine's certificates with that list empty remain accepted.
#[trace("TC-644", "FR-169-AC-15")]
#[test]
fn certificates_reject_the_other_forms_list_and_accept_an_empty_one() {
    let world = config_world(REACHES_TWO, &[(0, 0)]);
    let (over, certificate) = reaches_two(&world);
    let mut wrong_counts = certificate.clone();
    wrong_counts.counts.push((0, 1, 0));
    assert_eq!(
        check(
            &world,
            AB,
            "ReachesTwo",
            over.clone(),
            &wrong_counts,
            &ModelCheckLimits::default()
        ),
        rejected(CertificateRule::Malformed, certificate.closure[0].clone())
    );
    let mut missing_count_entry = certificate.clone();
    missing_count_entry.counts.push((9, 1, 0));
    assert_eq!(
        check(
            &world,
            AB,
            "ReachesTwo",
            over.clone(),
            &missing_count_entry,
            &ModelCheckLimits::default()
        ),
        TemporalCertificateCheck::Rejected(CertificateRejection {
            rule: CertificateRule::Malformed,
            at: CertificateLocus::Entry { index: 9 },
        })
    );
    assert_eq!(
        check(
            &world,
            AB,
            "ReachesTwo",
            over,
            &certificate,
            &ModelCheckLimits::default()
        ),
        TemporalCertificateCheck::Accepted
    );

    let world = job_world(true, false);
    let result = run(world.request(&["j"], vec![world.item("InOneWay")], no_samples()));
    let instance = &result.items[0][0];
    let StateGraphOutcome::Holds { certificate } = &instance.outcome else {
        panic!("the sequenced count claim holds: {:?}", instance.outcome);
    };
    let mut wrong_ranks = certificate.clone();
    wrong_ranks.ranks.push((0, 0));
    assert_eq!(
        check(
            &world,
            &["j"],
            "InOneWay",
            instance.over.clone(),
            &wrong_ranks,
            &ModelCheckLimits::default()
        ),
        rejected(CertificateRule::Malformed, certificate.closure[0].clone())
    );
    assert_eq!(
        check(
            &world,
            &["j"],
            "InOneWay",
            instance.over.clone(),
            certificate,
            &ModelCheckLimits::default()
        ),
        TemporalCertificateCheck::Accepted
    );
}

/// FR-169-AC-10 (check half): a hand-built certificate for FR-168-AC-5's
/// `Divides` over §7.1's 9 states, ranked as if the target held at `va = 1`,
/// is rejected `PredicateUndefined` at the first, in certificate order, of
/// the states where `va = 2`, where the target divides by zero; `(2, 0)` is
/// one of them.
#[trace("TC-644", "FR-169-AC-10")]
#[test]
fn the_checker_rejects_a_certificate_over_an_undefined_target() {
    let world = config_kernel_world(DIVIDES, &[(0, 0)]);
    let states = reachable(&world, AB).0;
    let va = |state: &ModelSimState| world.value(state, "a", "versionNumber");
    let ranks: BTreeMap<StateKey, u32> = states
        .iter()
        .map(|(state_key, state)| {
            (
                state_key.clone(),
                [1, 0, 2][usize::try_from(va(state)).expect("fits")],
            )
        })
        .collect();
    let undefined: BTreeSet<StateKey> = states
        .iter()
        .filter(|(_, state)| va(state) == 2)
        .map(|(state_key, _)| state_key.clone())
        .collect();
    assert!(undefined.contains(&config(&world, 2, 0)));
    let certificate = with_ranks(states.keys().cloned().collect(), &ranks);
    assert_eq!(
        check(
            &world,
            AB,
            "Divides",
            Some(world.reference("a")),
            &certificate,
            &ModelCheckLimits::default()
        ),
        rejected(
            CertificateRule::PredicateUndefined,
            undefined.first().expect("a state with va = 2").clone()
        )
    );
}

/// FR-169-AC-10 (check half): the `InOneWay` certificate over §7.3's
/// unsequenced job with every count 1 is rejected `CountBroken` at `000`,
/// the node with two paths; the sequenced certificate with its order
/// reversed is rejected `OrderBroken` at the first, in certificate order, of
/// its nodes other than `to`.
#[trace("TC-644", "FR-169-AC-10")]
#[test]
fn the_checker_rejects_altered_count_certificates() {
    let world = job_world(false, false);
    let flags = |state: &ModelSimState| {
        (
            world.value(state, "j", "a"),
            world.value(state, "j", "b"),
            world.value(state, "j", "done"),
        )
    };
    let states = reachable(&world, &["j"]).0;
    assert_eq!(states.len(), 5);
    let closure: Vec<StateKey> = states.keys().cloned().collect();
    // A reverse topological order of `H`: `done` first, `000` last.
    let order = |state: &ModelSimState| match flags(state) {
        (1, 1, 1) => 0,
        (1, 1, 0) => 1,
        (1, 0, 0) => 2,
        (0, 1, 0) => 3,
        (0, 0, 0) => 4,
        other => panic!("unreachable {other:?}"),
    };
    let counts = states
        .values()
        .enumerate()
        .map(|(index, state)| (u64::try_from(index).expect("fits"), 1, order(state)))
        .collect();
    let all_one = StateGraphCertificate {
        closure,
        ranks: Vec::new(),
        counts,
    };
    let start = state_where(&world, &["j"], |state| flags(state) == (0, 0, 0));
    assert_eq!(
        check(
            &world,
            &["j"],
            "InOneWay",
            Some(world.reference("j")),
            &all_one,
            &ModelCheckLimits::default()
        ),
        rejected(CertificateRule::CountBroken, start)
    );

    let world = job_world(true, false);
    let result = run(world.request(&["j"], vec![world.item("InOneWay")], no_samples()));
    let instance = &result.items[0][0];
    let StateGraphOutcome::Holds { certificate } = &instance.outcome else {
        panic!("holds: {:?}", instance.outcome);
    };
    let last = certificate
        .counts
        .iter()
        .map(|(_, _, order)| *order)
        .max()
        .expect("counts");
    let mut reversed = certificate.clone();
    for entry in &mut reversed.counts {
        entry.2 = last - entry.2;
    }
    let first_not_to = reachable(&world, &["j"])
        .0
        .into_iter()
        .find(|(_, state)| world.value(state, "j", "done") == 0)
        .map(|(state_key, _)| state_key)
        .expect("a node other than `to`");
    assert_eq!(
        check(
            &world,
            &["j"],
            "InOneWay",
            instance.over.clone(),
            &reversed,
            &ModelCheckLimits::default()
        ),
        rejected(CertificateRule::OrderBroken, first_not_to)
    );
}

/// The checker's budgets and cancellation over AC-9's `ReachesTwo`
/// certificate: `max_states` 8 stops at the ninth reached state, although
/// the closure is read whole; `max_transitions` 1 stops at the second
/// recomputed successor; a cancelled handle stops it `Cancelled`. None of
/// them is an acceptance.
#[trace("TC-644")]
#[test]
fn the_checker_stops_at_its_budgets_and_on_cancellation() {
    let world = config_world(REACHES_TWO, &[(0, 0)]);
    let (over, certificate) = reaches_two(&world);
    assert_eq!(certificate.closure.len(), 9);
    let mut limits = ModelCheckLimits::default();
    limits.limits.max_states = 8;
    assert_eq!(
        check(
            &world,
            AB,
            "ReachesTwo",
            over.clone(),
            &certificate,
            &limits
        ),
        TemporalCertificateCheck::Stopped(IncompleteCause::LimitReached {
            limit: ModelCheckLimit::MaxStates,
            value: 8,
            required: None,
        })
    );
    limits.limits.max_states = 9;
    assert_eq!(
        check(
            &world,
            AB,
            "ReachesTwo",
            over.clone(),
            &certificate,
            &limits
        ),
        TemporalCertificateCheck::Accepted
    );

    let mut limits = ModelCheckLimits::default();
    limits.limits.max_transitions = 1;
    assert_eq!(
        check(
            &world,
            AB,
            "ReachesTwo",
            over.clone(),
            &certificate,
            &limits
        ),
        TemporalCertificateCheck::Stopped(IncompleteCause::LimitReached {
            limit: ModelCheckLimit::MaxTransitions,
            value: 1,
            required: None,
        })
    );

    let cancel = Cancel::new();
    cancel.cancel(CancelCause::Requested);
    assert_eq!(
        check_cancelled(
            &world,
            AB,
            "ReachesTwo",
            over,
            &certificate,
            &ModelCheckLimits::default(),
            &cancel
        ),
        TemporalCertificateCheck::Stopped(IncompleteCause::Cancelled {
            source: CancelCause::Requested,
        })
    );
}

/// Departure 4: over `Fork` with `fromB`'s second precondition
/// `remainder(2 - self.pos) >= 0`, whose Kernel function computes
/// `6 mod divisor` and has no value at `pos = 2`, a
/// certificate holding every reachable state is rejected `BadState` at
/// `pos = 2`: its successors are not known.
#[trace("TC-644")]
#[test]
fn an_undetermined_contract_at_a_reached_state_is_bad() {
    let world = fork_kernel_world(
        "function remainder using v(divisor: Integer): Integer pure { 6 mod divisor }\n\
         pre Odd using v on F::Fork::fromB { remainder(2 - self.pos) >= 0 }\n\
         possible ReachesOne using v over (x: F::Fork) { x.pos = 1 }\n",
    );
    let states = reachable(&world, &["f"]).0;
    let ranks: BTreeMap<StateKey, u32> = BTreeMap::new();
    let certificate = with_ranks(states.keys().cloned().collect(), &ranks);
    let two = state_where(&world, &["f"], |state| world.value(state, "f", "pos") == 2);
    assert_eq!(
        check(
            &world,
            &["f"],
            "ReachesOne",
            Some(world.reference("f")),
            &certificate,
            &ModelCheckLimits::default()
        ),
        rejected(CertificateRule::BadState, two)
    );
}

/// Departure 2, both sides, over `Fork` (`0 -> 1 -> 3 -> 0` and
/// `0 -> 2 -> 4`) with `to (x.pos = 4)`: `H` holds the cycle `{0, 1, 3}`,
/// counted 2 with no topological order. With `from (x.pos = 2)` the claim
/// holds, and the engine's certificate, cycle included, is accepted. The
/// same certificate checked for `from (x.pos = 0)`, a node on the cycle, is
/// rejected `CountBroken` at `pos = 0`: a count of 2 never certifies a
/// `from` node.
#[trace("TC-643", "TC-644")]
#[test]
fn a_count_two_cycle_certifies_only_where_no_from_node_sits() {
    let world = fork_world(
        "unique path FromTwo using v over (x: F::Fork) from (x.pos = 2) to (x.pos = 4);\n\
         unique path FromZero using v over (x: F::Fork) from (x.pos = 0) to (x.pos = 4);\n",
    );
    let result = run(world.request(&["f"], vec![world.item("FromTwo")], no_samples()));
    let instance = &result.items[0][0];
    let StateGraphOutcome::Holds { certificate } = &instance.outcome else {
        panic!("holds: {:?}", instance.outcome);
    };
    let pos = |value| {
        state_where(&world, &["f"], |state| {
            world.value(state, "f", "pos") == value
        })
    };
    let counts: BTreeMap<StateKey, u8> = certificate
        .counts
        .iter()
        .map(|(index, count, _)| {
            (
                certificate.closure[usize::try_from(*index).expect("fits")].clone(),
                *count,
            )
        })
        .collect();
    assert_eq!(
        counts,
        BTreeMap::from([
            (pos(0), 2),
            (pos(1), 2),
            (pos(3), 2),
            (pos(2), 1),
            (pos(4), 1)
        ])
    );
    assert_eq!(
        check(
            &world,
            &["f"],
            "FromTwo",
            instance.over.clone(),
            certificate,
            &ModelCheckLimits::default()
        ),
        TemporalCertificateCheck::Accepted
    );
    assert_eq!(
        check(
            &world,
            &["f"],
            "FromZero",
            instance.over.clone(),
            certificate,
            &ModelCheckLimits::default()
        ),
        rejected(CertificateRule::CountBroken, pos(0))
    );
}

/// FR-169-AC-11: over §7.1's subject with universe `{a}`, initial `va = 0`
/// and the guard `quotient(2 - self.versionNumber) >= 1`, whose Kernel
/// function computes `2 div divisor`, a hand-built
/// certificate of the 3 states ranked 2, 1 and 0 for `ReachesTwo` is
/// rejected `BadState` at `va = 2`, whose guard has no value; the same
/// ranks over the subject without the guard are accepted.
#[trace("TC-644", "FR-169-AC-11")]
#[test]
fn an_undefined_guard_at_a_reached_state_rejects_bad_state() {
    const GUARD: &str = "pre Guard using v on Config::ConfigVersion::attemptUpdate { \
                         quotient(2 - self.versionNumber) >= 1 }\n";
    let checked = |extra: &str| {
        let world = config_kernel_world_over(extra, REACHES_TWO, &["a"], &[vec![0]]);
        let states = reachable(&world, &["a"]).0;
        assert_eq!(states.len(), 3);
        let va = |value| {
            state_where(&world, &["a"], |state| {
                world.value(state, "a", "versionNumber") == value
            })
        };
        let ranks = BTreeMap::from([(va(0), 2), (va(1), 1), (va(2), 0)]);
        let certificate = with_ranks(states.keys().cloned().collect(), &ranks);
        let result = check(
            &world,
            &["a"],
            "ReachesTwo",
            Some(world.reference("a")),
            &certificate,
            &ModelCheckLimits::default(),
        );
        (result, va(2))
    };
    let (guarded, two) = checked(GUARD);
    assert_eq!(guarded, rejected(CertificateRule::BadState, two));
    assert_eq!(checked("").0, TemporalCertificateCheck::Accepted);
}

/// FR-169-AC-18: a hand-built certificate of the 4 states of a counter that
/// stops at 3, ranked 3, 2, 1 and 0 for `AtThree`, is rejected `BadState` at
/// `value = 3` when `terminal when` is undefined there, refused there, or
/// both; with a predicate that has a value there, true or false, it is
/// accepted.
#[trace("TC-644", "FR-169-AC-18")]
#[test]
fn a_terminal_predicate_without_a_value_rejects_bad_state() {
    const UNDEFINED: &str = "terminal Done using v on C when { remainder(3 - self.value) = 0 };\n";
    const REFUSED: &str = "terminal Done using v on C when { narrow(self.value) = 0 };\n";
    const BESIDE: &str = "terminal Stop using v on N when { remainder(3 - self.value) = 0 };\n";
    let checked = |terminals: &str| {
        let world = counter_kernel_world(
            "model N = \"test/counter\";\n",
            &format!(
                "{terminals}possible AtThree using v over (c: C::Counter) {{ c.value = 3 }}\n"
            ),
        );
        let states = reachable(&world, &["c"]).0;
        assert_eq!(states.len(), 4);
        let at = |value| {
            state_where(&world, &["c"], |state| {
                world.value(state, "c", "value") == value
            })
        };
        let ranks = BTreeMap::from([(at(0), 3), (at(1), 2), (at(2), 1), (at(3), 0)]);
        let certificate = with_ranks(states.keys().cloned().collect(), &ranks);
        let result = check(
            &world,
            &["c"],
            "AtThree",
            Some(world.reference("c")),
            &certificate,
            &ModelCheckLimits::default(),
        );
        (result, at(3))
    };
    for terminals in [
        UNDEFINED.to_owned(),
        REFUSED.to_owned(),
        format!("{REFUSED}{BESIDE}"),
    ] {
        let (result, three) = checked(&terminals);
        assert_eq!(
            result,
            rejected(CertificateRule::BadState, three),
            "{terminals}"
        );
    }
    for terminals in [
        "terminal Done using v on C when { self.value = 3 };\n",
        "terminal Done using v on C when { false };\n",
    ] {
        assert_eq!(
            checked(terminals).0,
            TemporalCertificateCheck::Accepted,
            "{terminals}"
        );
    }
}

/// FR-125-AC-13: over a subject with no initial state an `over` claim has no
/// instance either; the checker refuses `NoInitialState`, never
/// `NoInstance`.
#[trace("TC-520", "FR-125-AC-13")]
#[test]
fn the_checker_refuses_no_initial_state_before_no_instance() {
    let (over, certificate) = reaches_two(&config_world(REACHES_TWO, &[(0, 0)]));
    let empty = World::new(
        config_version(),
        &format!("{CONFIG_CONTRACT}{REACHES_TWO}"),
        &[],
    );
    let request = StateGraphCertificateRequest {
        subject: empty.subject(AB, UNLIMITED),
        claim: empty.claim("ReachesTwo"),
        over,
    };
    let checked = check_state_graph(
        &request,
        &certificate,
        &ModelCheckLimits::default(),
        &Cancel::new(),
    );
    assert!(
        matches!(checked, Err(ClosureRefusal::NoInitialState)),
        "{checked:?}"
    );
}
