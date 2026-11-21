// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-338's canonical translation state for a bounded (TP-2) clause: the
//! exact key octets of `eventually[0,u] holds(A)` and `always[0,u] holds(A)`
//! on origin, the counted-distance rules a step reads them by, and the
//! admission of imported keys against the clause's context.
//!
//! The expected octets are written out here as ASCII text, independent of
//! the encoder under test. Not covered: the `on each`, lower-bound, past,
//! nested-interval and Büchi alternatives, which no EN-1 phase certifies
//! yet (Remaining work: QSL-878).

use filament_exact::{Origin, Role};
use fsl_eval::automaton::{
    bounded_shape, closure_shape, BoundedKind, BoundedState, BoundedTranslation, KeyRefusal,
    StateContext, TranslationRefusal,
};
use fsl_foundation::source::provenance::OccurrenceKey;
use ix_trace_rs::trace;

use crate::temporal_trace::{fixture, EVENT, INFINITE};

fn occurrence(ordinal: u64) -> OccurrenceKey {
    OccurrenceKey::new(
        crate::support::ids::node_key([0xab]),
        Origin::new(Role::new("temporal"), ordinal),
    )
}

/// The bounded clause `body` over `c: Counter` under event-position
/// false-extension at `occurrence(ordinal)`, activated as `activation`.
fn clause_in(body: &str, activation: &str, ordinal: u64) -> BoundedTranslation {
    let fixture = fixture(EVENT, "c: Counter", activation, body);
    let context = StateContext::clause(&fixture.clause, occurrence(ordinal), Vec::new());
    BoundedTranslation::clause(&fixture.clause, &context)
        .unwrap_or_else(|why| panic!("{body}: {why}"))
}

fn clause(body: &str) -> BoundedTranslation {
    clause_in(body, "on origin", 0)
}

fn text(translation: &BoundedTranslation, state: BoundedState) -> String {
    let key = translation.key(state).expect("the key encodes");
    String::from_utf8(key.as_bytes().to_vec()).expect("a key is UTF-8")
}

/// The context of `clause(_)` as FR-338 spells it.
fn context() -> String {
    r#"[[{"ordinal":0,"package":"fixture","path":["[171]"]},"temporal","0"],"filament.temporal.event-position.false-extension/v1","origin",[]]"#.to_owned()
}

fn key(body: &str) -> String {
    format!(
        r#"["filament.automaton-state-key/v1",{},{body}]"#,
        context()
    )
}

/// FR-338's `eventually[0,1] holds(p)` vector: on origin with p false at
/// position zero the state is `["bounded",["origin","1"],[[[],[],[[[],
/// "positive","deadline","1"]],[]]]]`; p true at counted distance one gives
/// `["accept"]`, and p false at the final region's close gives `["reject"]`.
#[trace("TC-525", "FR-338-AC-6")]
#[test]
fn eventually_one_on_origin_walks_to_accept_and_to_reject() {
    let translation = clause("eventually[0,1] holds(true)");
    assert_eq!(translation.kind(), BoundedKind::Eventually);
    assert_eq!(translation.horizon(), 1);
    let first = translation.start(false);
    assert_eq!(first, BoundedState::Open { remaining: 1 });
    assert_eq!(
        text(&translation, first),
        key(r#"["bounded",["origin","1"],[[[],[],[[[],"positive","deadline","1"]],[]]]]"#)
    );
    let accepted = translation.step(first, true, true);
    assert_eq!(accepted, BoundedState::Accept);
    assert_eq!(text(&translation, accepted), key(r#"["accept"]"#));
    // False at distance one leaves the region open until the next counted
    // step crosses it; the crossing reads no letter.
    let last = translation.step(first, true, false);
    assert_eq!(last, BoundedState::Open { remaining: 0 });
    assert_eq!(
        text(&translation, last),
        key(r#"["bounded",["origin","0"],[[[],[],[[[],"positive","deadline","0"]],[]]]]"#)
    );
    let rejected = translation.step(last, true, true);
    assert_eq!(rejected, BoundedState::Reject);
    assert_eq!(text(&translation, rejected), key(r#"["reject"]"#));
    assert_eq!(
        translation.step(translation.start(true), true, false),
        BoundedState::Accept
    );
    assert_eq!(translation.step(rejected, true, true), BoundedState::Reject);
}

/// FSpec FR-091 on a protocol subject (ADR-027 PB-3): a step that does not
/// count leaves the debt where it was and its letter is still read, so a
/// false fork before the counted step does not reject, a true letter after
/// an uncounted one accepts, and the crossing counted step rejects.
#[trace("TC-525", "FR-338-AC-6")]
#[test]
fn a_step_that_does_not_count_reads_its_letter_without_reducing_the_debt() {
    let translation = clause("eventually[0,1] holds(true)");
    let origin = translation.start(false);
    let after_fork = translation.step(origin, false, false);
    assert_eq!(after_fork, origin);
    assert_eq!(
        translation.step(after_fork, true, true),
        BoundedState::Accept
    );
    // The counted step that follows a false counted one closes the region.
    let counted = translation.step(origin, true, false);
    assert_eq!(translation.step(counted, true, true), BoundedState::Reject);
    // At distance one a false letter followed by an uncounted true one
    // accepts: the zero-distance carry keeps the region open.
    assert_eq!(translation.step(counted, false, true), BoundedState::Accept);
    assert_eq!(translation.step(counted, false, false), counted);
}

/// `always[0,u] holds(A)`: A must hold at every position of the window, the
/// extent is the furthest required distance, and the window's closed upper
/// boundary discharges it on the counted step that crosses it.
#[trace("TC-525", "FR-338-AC-6")]
#[test]
fn always_carries_its_extent_and_discharges_at_the_closed_boundary() {
    let translation = clause("always[0,1] holds(true)");
    assert_eq!(translation.kind(), BoundedKind::Always);
    assert_eq!(translation.start(false), BoundedState::Reject);
    let first = translation.start(true);
    assert_eq!(
        text(&translation, first),
        key(r#"["bounded",["origin","1"],[[[],[],[[[],"positive","extent","1"]],[]]]]"#)
    );
    let last = translation.step(first, true, true);
    assert_eq!(last, BoundedState::Open { remaining: 0 });
    assert_eq!(translation.step(first, true, false), BoundedState::Reject);
    // The region at the upper boundary still reads zero-distance letters.
    assert_eq!(translation.step(last, false, false), BoundedState::Reject);
    assert_eq!(translation.step(last, false, true), last);
    // The crossing step reads no letter and discharges.
    assert_eq!(translation.step(last, true, false), BoundedState::Accept);
}

/// FSpec FR-091 and FR-338: a terminal state's false extension supplies the
/// counted false letters through the remaining horizon and closes the last
/// region, so an open `eventually` debt rejects and an open `always`
/// obligation rejects unless its window already ended at the terminal
/// state.
#[trace("TC-525", "FR-338-AC-6")]
#[test]
fn the_terminal_false_extension_closes_the_open_region() {
    let eventually = clause("eventually[0,2] holds(true)");
    for remaining in 0..=2 {
        assert_eq!(
            eventually.terminal(BoundedState::Open { remaining }),
            BoundedState::Reject
        );
    }
    assert_eq!(
        eventually.terminal(BoundedState::Accept),
        BoundedState::Accept
    );
    let always = clause("always[0,2] holds(true)");
    assert_eq!(
        always.terminal(BoundedState::Open { remaining: 0 }),
        BoundedState::Accept
    );
    assert_eq!(
        always.terminal(BoundedState::Open { remaining: 1 }),
        BoundedState::Reject
    );
    assert_eq!(
        always.terminal(BoundedState::Open { remaining: 2 }),
        BoundedState::Reject
    );
}

/// u64::MAX bounds stay exact: the horizon and the remaining distance are
/// decimal strings of the checked u64 domain, with no saturation.
#[trace("TC-525", "FR-338-AC-7")]
#[test]
fn a_u64_max_horizon_stays_exact() {
    let translation = clause("eventually[0,18446744073709551615] holds(true)");
    let first = translation.start(false);
    assert_eq!(
        first,
        BoundedState::Open {
            remaining: u64::MAX
        }
    );
    assert!(text(&translation, first).contains(r#"["origin","18446744073709551615"]"#));
    assert_eq!(
        translation.admit(text(&translation, first).as_bytes()),
        Ok((translation.key(first).expect("encodes"), first))
    );
    assert_eq!(
        translation.step(first, true, false),
        BoundedState::Open {
            remaining: u64::MAX - 1
        }
    );
}

/// Every key a translation encodes is admitted back as the same state; a
/// key of another occurrence, profile or activation is `ForeignContext`,
/// and a `bounded` state with a mismatched clock and window, an `each`
/// clock, a window of the other kind, a remaining distance past the
/// horizon, a leading zero or an extra member is refused before it is used.
#[trace("TC-525", "FR-338-AC-5")]
#[test]
fn a_bounded_key_is_admitted_against_its_own_context_only() {
    let translation = clause("eventually[0,2] holds(true)");
    for state in [
        BoundedState::Open { remaining: 0 },
        BoundedState::Open { remaining: 2 },
        BoundedState::Accept,
        BoundedState::Reject,
    ] {
        let key = translation.key(state).expect("encodes");
        assert_eq!(translation.admit(key.as_bytes()), Ok((key.clone(), state)));
    }
    let other = clause_in("eventually[0,2] holds(true)", "on origin", 1);
    let foreign = other.key(BoundedState::Accept).expect("encodes");
    assert_eq!(
        translation.admit(foreign.as_bytes()),
        Err(KeyRefusal::ForeignContext)
    );
    let each = text(&translation, BoundedState::Accept).replace(r#""origin""#, r#""each""#);
    assert_eq!(
        translation.admit(each.as_bytes()),
        Err(KeyRefusal::ForeignContext)
    );
    let infinite = text(&translation, BoundedState::Accept).replace(
        "filament.temporal.event-position.false-extension/v1",
        INFINITE,
    );
    assert_eq!(
        translation.admit(infinite.as_bytes()),
        Err(KeyRefusal::ForeignContext)
    );

    let open = |clock: &str, window: &str| {
        key(&format!(
            r#"["bounded",["origin",{clock}],[[[],[],[[[],"positive",{window}]],[]]]]"#
        ))
    };
    let refused = [
        (open(r#""1""#, r#""deadline","2""#), KeyRefusal::Unreachable),
        (open(r#""3""#, r#""deadline","3""#), KeyRefusal::Unreachable),
        (open(r#""1""#, r#""extent","1""#), KeyRefusal::Unresolved),
        (
            open(r#""01""#, r#""deadline","01""#),
            KeyRefusal::Decimal("01".into()),
        ),
        (
            open(
                r#""18446744073709551616""#,
                r#""deadline","18446744073709551616""#,
            ),
            KeyRefusal::Decimal("18446744073709551616".into()),
        ),
        (open("1", r#""deadline","1""#), KeyRefusal::Members),
        (
            key(r#"["bounded",["origin","1"],[]]"#),
            KeyRefusal::Unreachable,
        ),
        (
            key(r#"["bounded",["origin","1"],[[[],[],[],[]]]]"#),
            KeyRefusal::Unresolved,
        ),
        (
            key(
                r#"["bounded",["origin","1"],[[[[[],"positive"]],[],[[[],"positive","deadline","1"]],[]]]]"#,
            ),
            KeyRefusal::Unresolved,
        ),
        (
            key(r#"["bounded",["origin","1"],[[[],[],[[[],"negative","deadline","1"]],[]]]]"#),
            KeyRefusal::Unresolved,
        ),
        (
            key(r#"["bounded",["origin","1"],[[[],[],[[["0"],"positive","deadline","1"]],[]]]]"#),
            KeyRefusal::Unresolved,
        ),
        (key(r#"["accept","extra"]"#), KeyRefusal::Members),
        (
            key(r#"["safety",[]]"#),
            KeyRefusal::WrongForm("safety".into()),
        ),
        (
            key(r#"["pending"]"#),
            KeyRefusal::UnknownTag("pending".into()),
        ),
    ];
    for (octets, why) in refused {
        assert_eq!(translation.admit(octets.as_bytes()), Err(why), "{octets}");
    }
    let spaced = text(&translation, BoundedState::Accept).replacen("[", "[ ", 1);
    assert_eq!(
        translation.admit(spaced.as_bytes()),
        Err(KeyRefusal::NotCanonical)
    );
}

/// Only `eventually[0,u]` and `always[0,u]` over one `holds(A)`, on origin
/// under a step-measured bounded profile or the infinite-trace one, have a
/// bounded translation.
#[trace("TC-525")]
#[test]
fn another_shape_has_no_bounded_translation() {
    for (profile, activation, body) in [
        (EVENT, "on origin", "eventually[1,2] holds(true)"),
        (
            EVENT,
            "on origin",
            "eventually[0,2] (holds(true) and holds(false))",
        ),
        (EVENT, "on origin", "once[0,2] holds(true)"),
        (EVENT, "on origin", "holds(true) until[0,2] holds(true)"),
    ] {
        let fixture = fixture(profile, "c: Counter", activation, body);
        assert!(bounded_shape(&fixture.clause).is_none(), "{body}");
        let context = StateContext::clause(&fixture.clause, occurrence(0), Vec::new());
        assert!(
            matches!(
                BoundedTranslation::clause(&fixture.clause, &context),
                Err(TranslationRefusal::NotTranslated(_))
            ),
            "{body}"
        );
    }
}

/// FR-125 and FR-338: the infinite-trace profile translates the same two
/// operators, keyed under the `infinite` activation, and its terminal state
/// stutters where a bounded profile closes with false letters.
#[trace("TC-525")]
#[test]
fn the_infinite_trace_profile_translates_the_bounded_operators_and_stutters() {
    let fixture = fixture(
        INFINITE,
        "c: Counter",
        "on origin",
        "eventually[0,2] holds(true)",
    );
    assert!(bounded_shape(&fixture.clause).is_none());
    assert!(closure_shape(&fixture.clause).is_some());
    let context = StateContext::clause(&fixture.clause, occurrence(0), Vec::new());
    let translation =
        BoundedTranslation::clause(&fixture.clause, &context).expect("an infinite translation");
    assert!(translation.stutters());
}

/// FR-338-AC-10: the keys of an `on each` bounded monitor, whose clock is
/// `["each"]` with no remaining distance, and whose one branch carries the
/// merged window or none; each is admitted back as its own state, and an
/// `["each", remaining]` clock, an `["origin", ..]` clock, a second window,
/// a window of the other operator, an obligation, a past or an acceptance
/// member and a distance past the horizon are refused.
#[trace("TC-525", "FR-338-AC-10", "FR-338-AC-28")]
#[test]
fn an_each_key_has_a_clock_without_a_remaining_distance() {
    let each = |body: &str| {
        let translation = clause_in(body, "on each (q: Counter)", 0);
        let context = context().replace(r#""origin""#, r#""each""#);
        (translation, context)
    };
    let (eventually, context) = each("eventually[0,1] holds(true)");
    let key = |body: &str| format!(r#"["filament.automaton-state-key/v1",{context},{body}]"#);
    let idle = eventually.start_each(true, true);
    assert_eq!(idle, BoundedState::Idle);
    assert_eq!(
        text(&eventually, idle),
        key(r#"["bounded",["each"],[[[],[],[],[]]]]"#)
    );
    let owing = eventually.start_each(false, true);
    assert_eq!(owing, BoundedState::Open { remaining: 1 });
    assert_eq!(
        text(&eventually, owing),
        key(r#"["bounded",["each"],[[[],[],[[[],"positive","deadline","1"]],[]]]]"#)
    );
    for state in [idle, owing, BoundedState::Accept, BoundedState::Reject] {
        let octets = eventually.key(state).expect("encodes");
        assert_eq!(
            eventually.admit(octets.as_bytes()),
            Ok((octets.clone(), state))
        );
    }
    let refused = [
        // A stray remaining distance in the clock.
        (
            key(r#"["bounded",["each","1"],[[[],[],[[[],"positive","deadline","1"]],[]]]]"#),
            KeyRefusal::Members,
        ),
        (
            key(r#"["bounded",["each","1"],[[[],[],[],[]]]]"#),
            KeyRefusal::Members,
        ),
        (
            key(r#"["bounded",[],[[[],[],[],[]]]]"#),
            KeyRefusal::Members,
        ),
        // The clock of another activation.
        (
            key(r#"["bounded",["origin","1"],[[[],[],[[[],"positive","deadline","1"]],[]]]]"#),
            KeyRefusal::ForeignContext,
        ),
        // A distance past the horizon, a window of the other operator and a
        // second window.
        (
            key(r#"["bounded",["each"],[[[],[],[[[],"positive","deadline","2"]],[]]]]"#),
            KeyRefusal::Unreachable,
        ),
        (
            key(r#"["bounded",["each"],[[[],[],[[[],"positive","extent","1"]],[]]]]"#),
            KeyRefusal::Unresolved,
        ),
        (
            key(
                r#"["bounded",["each"],[[[],[],[[[],"positive","deadline","0"],[[],"positive","deadline","1"]],[]]]]"#,
            ),
            KeyRefusal::Unresolved,
        ),
        // A member the clause has no occurrence for, and no branch at all.
        (
            key(r#"["bounded",["each"],[[[[[],"positive"]],[],[],[]]]]"#),
            KeyRefusal::Unresolved,
        ),
        (key(r#"["bounded",["each"],[]]"#), KeyRefusal::Unreachable),
    ];
    for (octets, why) in refused {
        assert_eq!(eventually.admit(octets.as_bytes()), Err(why), "{octets}");
    }
    // An `on origin` translation refuses the idle key, which it cannot reach.
    let origin = clause("eventually[0,1] holds(true)");
    assert_eq!(origin.key(BoundedState::Idle), Err(KeyRefusal::Unreachable));
    assert_eq!(
        origin.admit(key(r#"["bounded",["each"],[[[],[],[],[]]]]"#).as_bytes()),
        Err(KeyRefusal::ForeignContext)
    );
    let (always, context) = each("always[0,2] holds(true)");
    let extent = always.start_each(true, true);
    assert_eq!(
        text(&always, extent),
        format!(
            r#"["filament.automaton-state-key/v1",{context},["bounded",["each"],[[[],[],[[[],"positive","extent","2"]],[]]]]]"#
        )
    );
}

/// The metered policy facade preserves existing canonical identities, rather
/// than introducing search-local state numbers or another key namespace.
#[test]
#[trace("FSpec-FR-803", "FR-1011", "FR-1012")]
fn source_policy_keys_match_the_shared_canonical_translations() {
    use filament_exact::Meter;
    use fsl_eval::automaton::{
        ContextBinding, PolicyContext, PolicyState, SafetyTranslation, StepPolicy,
    };
    use fsl_semantics::check::SCALAR_LIMITS_UNLIMITED;
    let root = crate::support::ids::node_key([0xab]);
    let parameter = crate::support::ids::node_key([0xcd]);
    let claim = Origin::new(Role::new("temporal"), 0);
    let contexts = [PolicyContext {
        node: &root,
        claim: &claim,
        parameter: &parameter,
        instance: "c",
    }];
    for formula in [
        "always holds(c.level >= 1)",
        "eventually[0,4] holds(c.level >= 1)",
        "always[0,4] holds(c.level >= 1)",
    ] {
        let fixture = fixture(INFINITE, "c: Counter", "on origin", formula);
        let policy = StepPolicy::new(&fixture.clause).unwrap();
        let context = StateContext::clause(
            &fixture.clause,
            occurrence(0),
            vec![ContextBinding::instance(parameter.clone(), "c")],
        );
        for holds in [false, true] {
            let mut meter = Meter::new(SCALAR_LIMITS_UNLIMITED);
            let state = policy.advance(None, holds, &mut meter).unwrap();
            let expected = match state {
                PolicyState::Safety(state) => {
                    SafetyTranslation::reachable_invariant(&fixture.clause, &context)
                        .unwrap()
                        .key(state)
                        .unwrap()
                }
                PolicyState::Bounded(state) => {
                    BoundedTranslation::clause(&fixture.clause, &context)
                        .unwrap()
                        .key(state)
                        .unwrap()
                }
            };
            let bytes = policy.encode(&contexts, &[state], &mut meter).unwrap();
            assert_eq!(&bytes[16..], expected.as_bytes());
            assert_eq!(
                policy.admit(&contexts, &bytes, &mut meter).unwrap(),
                [state]
            );
            let foreign = [PolicyContext {
                instance: "other",
                ..contexts[0]
            }];
            assert!(policy.admit(&foreign, &bytes, &mut meter).is_err());
            let mut trailing = bytes.clone();
            trailing.push(0);
            assert!(policy.admit(&contexts, &trailing, &mut meter).is_err());
            let mut omitted = bytes.clone();
            omitted[0] = 0;
            assert!(policy.admit(&contexts, &omitted, &mut meter).is_err());
        }
    }
}

/// Infinite-trace completion repeats the final letter, whereas the bounded
/// profile's false extension would reject an unexhausted always window.
#[test]
#[trace("FSpec-FR-803", "FR-1012")]
fn source_policy_terminal_stutter_matches_the_batch_lasso() {
    use crate::temporal_trace::{counters, outcome};
    use filament_exact::Meter;
    use fsl_eval::automaton::StepPolicy;
    use fsl_eval::temporal::{TemporalOutcome, TemporalTrace};
    use fsl_semantics::check::SCALAR_LIMITS_UNLIMITED;
    for formula in [
        "always holds(c.level >= 1)",
        "eventually[0,4] holds(c.level >= 1)",
        "always[0,4] holds(c.level >= 1)",
        "always[0,0] holds(c.level >= 1)",
        "eventually[0,0] holds(c.level >= 1)",
    ] {
        let fixture = fixture(INFINITE, "c: Counter", "on origin", formula);
        let policy = StepPolicy::new(&fixture.clause).unwrap();
        for value in [0, 1] {
            let mut meter = Meter::new(SCALAR_LIMITS_UNLIMITED);
            let state = policy.advance(None, value >= 1, &mut meter).unwrap();
            let expected = outcome(
                &fixture,
                TemporalTrace::Lasso {
                    prefix: 0,
                    loop_len: 1,
                },
                counters(&[value]),
            );
            assert_eq!(
                policy.terminal(state, value >= 1, &mut meter).unwrap(),
                matches!(expected, TemporalOutcome::Holds),
                "{formula}, {value}: {expected:?}"
            );
        }
    }
}

/// Representation work and full capacity reservations use the caller's
/// original scalar ledger; cancellation is distinct from ordinary shortage.
#[test]
#[trace("FR-1011", "FSpec-FR-803")]
fn source_policy_key_budget_preserves_work_capacity_and_cancel_causes() {
    use filament_exact::{Cancel, CancelCause, ChargePoint, LimitKind, Meter, ScalarLimits};
    use fsl_eval::automaton::{PolicyContext, PolicyStateError, StepPolicy};
    use fsl_semantics::check::SCALAR_LIMITS_UNLIMITED;
    let fixture = fixture(
        INFINITE,
        "c: Counter",
        "on origin",
        "always holds(c.level >= 1)",
    );
    let policy = StepPolicy::new(&fixture.clause).unwrap();
    let root = crate::support::ids::node_key([0xab]);
    let parameter = root.clone();
    let claim = Origin::new(Role::new("temporal"), 0);
    let contexts = [PolicyContext {
        node: &root,
        claim: &claim,
        parameter: &parameter,
        instance: "c",
    }];
    let state = policy
        .advance(None, true, &mut Meter::new(SCALAR_LIMITS_UNLIMITED))
        .unwrap();
    let mut ample = Meter::new(SCALAR_LIMITS_UNLIMITED);
    let expected = policy.encode(&contexts, &[state], &mut ample).unwrap();
    for counter in [LimitKind::WorkUnits, LimitKind::ResultUnits] {
        let exact = ample.consumed(counter);
        for (limit, succeeds) in [(exact, true), (exact - 1, false)] {
            let mut limits = SCALAR_LIMITS_UNLIMITED;
            match counter {
                LimitKind::WorkUnits => limits.work_units = limit,
                LimitKind::ResultUnits => limits.result_units = limit,
                _ => unreachable!(),
            }
            let result = policy.encode(&contexts, &[state], &mut Meter::new(limits));
            if succeeds {
                assert_eq!(result.unwrap(), expected);
            } else {
                let PolicyStateError::Accounting(denial) = result.unwrap_err() else {
                    panic!("original scalar denial");
                };
                assert_eq!(denial.limit_kind, counter);
                assert_eq!(denial.limit, limit);
                assert_eq!(denial.charge_point, ChargePoint::ModelCandidate);
            }
        }
    }
    for cause in [CancelCause::Requested, CancelCause::Deadline] {
        let cancel = Cancel::new();
        cancel.cancel(cause);
        let mut meter = Meter::new(ScalarLimits {
            work_units: 0,
            result_units: 0,
            ..SCALAR_LIMITS_UNLIMITED
        })
        .with_cancel(cancel);
        assert!(
            matches!(policy.encode(&contexts, &[state], &mut meter), Err(PolicyStateError::Cancelled(actual)) if actual == cause)
        );
        assert!(
            matches!(policy.admit(&contexts, &expected, &mut meter), Err(PolicyStateError::Cancelled(actual)) if actual == cause)
        );
    }
}
