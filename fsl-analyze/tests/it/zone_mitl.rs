// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-242's timed claim automaton on EN-6's liveness product: ADR-026
//! §11's `Settles` (TT-3) over `Rpc`, decided, certified, refuted and
//! replayed (TC-692, TC-694, TC-697).

use filament_exact::{Cancel, CancelCause, Integer, Rational};
use fsl_analyze::zone_check::{ZoneCheckLimit, ZoneCheckLimits, ZoneCheckOutcome, ZoneStop};
use fsl_replay::{CertificateCheckLimits, ZoneCertificateVerdict};
use fsl_route::deadlock::{ItemObligation, ObligationKind};
use ix_trace_rs::trace;

use crate::timed_world::{document_unbounded, World};
use crate::zone_replay::{replay, reproduced};
use crate::zone_search::{
    check, claim, delays, operations, proved, rpc_claim, rpc_world, violated, Outcome, SUBJECT,
    TIMED,
};

/// ADR-026 §11's `Settles` over `Rpc` with deadline `t`, its `eventually`
/// over `interval`: every `Waiting` position is followed within the
/// interval by a position that is not `Waiting`.
pub(crate) fn settles(t: u32, interval: &str) -> World {
    rpc_claim(t, TIMED, &settles_clause(interval))
}

/// `Settles`' clause text, its `eventually` over `interval`.
fn settles_clause(interval: &str) -> String {
    format!(
        "temporal Settles using tt over (c: M::Call) clock \"model-time\" on origin \
         {{ always (holds(c.phase = 1) implies eventually{interval} holds(not (c.phase = 1))) }}\n"
    )
}

fn decide(world: &World) -> Outcome {
    claim(world, ZoneCheckLimits::default(), &Cancel::new())
}

fn whole(value: i64) -> Rational {
    Rational::from_integer(Integer::from(value))
}

/// FR-242's TT-3 behaviour: `Settles` is proved over `Rpc` with `T = 3 ms`
/// and with `T = 4 ms`, each certificate accepted by the core, which
/// rebuilds the claim automaton itself; under `eventually[0 ms, 3 ms)` it
/// is refuted (FR-237-AC-1's `Settles` half) by `send` at 0 then `reply`
/// after delay 3, the exact boundary, and that counterexample reproduces.
#[trace("TC-697", "FR-242")]
#[trace("TC-692", "FR-237-AC-1")]
#[test]
fn settles_is_proved_and_its_strict_variant_refuted() {
    for t in [3, 4] {
        let world = settles(t, "[0 ms, 3 ms]");
        let certificate = proved(decide(&world));
        assert!(certificate.components.is_some());
        assert_eq!(
            check(&world, &certificate, CertificateCheckLimits::default()),
            Ok(ZoneCertificateVerdict::Accepted),
            "T = {t}"
        );
    }
    let strict = settles(4, "[0 ms, 3 ms)");
    let counterexample = violated(decide(&strict));
    assert_eq!(operations(&counterexample)[..2], ["send", "reply"]);
    assert_eq!(delays(&counterexample)[..2], [whole(0), whole(3)]);
    let settlement = replay(&strict, &counterexample).expect("replays");
    assert!(reproduced(&settlement), "{settlement:?}");
}

/// Part of FR-239-AC-4 (TC-694): `Settles` over `Rpc` with
/// `max_symbolic_states` 2 stops naming the value 2; with a cancelled
/// handle it stops `Cancelled`; two runs give equal outcomes, certificates
/// and counterexamples. The `RequiresBound` half is
/// [`an_unbounded_root_requires_a_bound`].
#[trace("TC-694")]
#[test]
fn settles_stops_at_its_budgets_and_repeats() {
    let world = settles(4, "[0 ms, 3 ms]");
    let limits = ZoneCheckLimits {
        max_symbolic_states: 2,
        ..ZoneCheckLimits::default()
    };
    let ZoneCheckOutcome::Stopped { cause, .. } = claim(&world, limits, &Cancel::new()) else {
        panic!("a budget of 2 stops");
    };
    assert_eq!(
        cause,
        ZoneStop::LimitReached(ZoneCheckLimit::MaxSymbolicStates(2))
    );
    let cancel = Cancel::new();
    cancel.cancel(CancelCause::Requested);
    let ZoneCheckOutcome::Stopped { cause, .. } =
        claim(&world, ZoneCheckLimits::default(), &cancel)
    else {
        panic!("a cancelled run stops");
    };
    assert_eq!(cause, ZoneStop::Cancelled);
    assert_eq!(proved(decide(&world)), proved(decide(&world)));
    let strict = settles(4, "[0 ms, 3 ms)");
    assert_eq!(violated(decide(&strict)), violated(decide(&strict)));
}

/// FR-245 over a timed claim automaton: the checker rebuilds the automaton
/// from the clause it is asked about and trusts nothing of the
/// certificate's. `Settles`' proof at `T = 4 ms`, renamed to the strict
/// variant `eventually[0 ms, 3 ms)`, which `reply` at exactly 3 ms refutes,
/// is rejected.
#[trace("TC-700")]
#[test]
fn a_settles_certificate_is_checked_against_the_rebuilt_automaton() {
    let world = settles(4, "[0 ms, 3 ms]");
    let mut certificate = proved(decide(&world));
    let strict = settles(4, "[0 ms, 3 ms)");
    certificate.obligation = ItemObligation {
        subject: SUBJECT,
        kind: ObligationKind::Authored(fsl_replay::node_occurrence(
            &strict.package.graph().temporal_clauses()[0],
        )),
        reductions: fsl_route::reductions::ObligationReductions::default(),
    };
    let verdict =
        check(&strict, &certificate, CertificateCheckLimits::default()).expect("the check runs");
    assert!(
        matches!(verdict, ZoneCertificateVerdict::Rejected(_)),
        "{verdict:?}"
    );
}

/// Part of FR-239-AC-4 (TC-694): `Settles` over `Rpc` whose `send` also
/// takes `n: Integer`, an unbounded population root, refuses
/// `RequiresBound` (FR-126), naming the unbounded domain; the refusal is
/// read before any state is explored, when the timed subject is built. Two
/// runs refuse alike.
#[trace("TC-694")]
#[test]
fn an_unbounded_root_requires_a_bound() {
    let document = document_unbounded(
        &["x"],
        &[
            ("send", &["phase", "inflight", "late", "x"]),
            ("reply", &["phase", "inflight", "late"]),
            ("timeout", &["phase"]),
            ("reset", &["phase"]),
        ],
    );
    let world = rpc_world(&document, 4, TIMED, &settles_clause("[0 ms, 3 ms]"));
    // A timed subject is built only to be expanded, so the bound check
    // refuses it before EN-6 is asked.
    let refused = || {
        let system = world.system();
        fsl_eval::simulation::model::TimedSystem::new(&system, "M").err()
    };
    let Some(fsl_eval::simulation::model::TimedSubjectRefusal::NotSimulated(
        fsl_eval::simulation::NotSimulated::RequiresBound(roots),
    )) = refused()
    else {
        panic!("expected RequiresBound, got {:?}", refused());
    };
    assert!(!roots.domains.is_empty(), "{roots:?}");
    assert_eq!(refused(), refused());
}

/// The verdict `Settles` gets over `Rpc` at `T = 4 ms` under `interval`.
fn verdict(interval: &str) -> &'static str {
    match decide(&settles(4, interval)) {
        Outcome::Holds { .. } => "holds",
        Outcome::Violated { .. } => "violated",
        other => panic!("{interval}: neither proved nor refuted: {other:?}"),
    }
}

/// QSL-2129 MITL breadth, closed bound: `reply` is forced by the server
/// deadline at 3 ms, so the claim over `[0 ms, 3 ms]` holds.
#[trace("TC-697", "FR-242")]
#[test]
fn a_closed_bound_holds_at_its_boundary() {
    assert_eq!(verdict("[0 ms, 3 ms]"), "holds");
}

/// QSL-2129 MITL breadth, open bound: `reply` may fire at exactly 3 ms, which
/// the open interval `(0 ms, 3 ms)` excludes, so the claim is refuted at the
/// boundary delay.
#[trace("TC-697", "FR-242")]
#[test]
fn an_open_bound_excludes_its_boundary() {
    let world = settles(4, "(0 ms, 3 ms)");
    let counterexample = violated(decide(&world));
    assert_eq!(operations(&counterexample)[..2], ["send", "reply"]);
    assert_eq!(delays(&counterexample)[..2], [whole(0), whole(3)]);
    let settlement = replay(&world, &counterexample).expect("replays");
    assert!(reproduced(&settlement), "{settlement:?}");
}

/// QSL-2129 MITL breadth, punctual bound: `[3 ms, 3 ms]` needs a position at
/// exactly 3 ms, and `reply` at 1 ms leaves none, so the claim is refuted; the
/// punctual `[0 ms, 0 ms]` is the waiting position itself, which is waiting.
#[trace("TC-697", "FR-242")]
#[test]
fn a_punctual_bound_needs_a_position_at_its_instant() {
    assert_eq!(verdict("[3 ms, 3 ms]"), "violated");
    assert_eq!(verdict("[0 ms, 0 ms]"), "violated");
}

/// QSL-2129 MITL breadth, nested operators: `eventually[0,3]` wrapping the
/// same `eventually[0,3]` holds where the flat claim does, and wrapping the
/// open inner bound is refuted where the open flat claim is.
#[trace("TC-697", "FR-242")]
#[test]
fn nested_timed_operators_compose() {
    let nested = |outer: &str, inner: &str| {
        rpc_claim(
            4,
            TIMED,
            &format!(
                "temporal Nested using tt over (c: M::Call) clock \"model-time\" on origin \
                 {{ always (holds(c.phase = 1) implies eventually{outer} \
                 (eventually{inner} holds(not (c.phase = 1)))) }}\n"
            ),
        )
    };
    assert!(matches!(
        decide(&nested("[0 ms, 3 ms]", "[0 ms, 3 ms]")),
        Outcome::Holds { .. }
    ));
    assert!(matches!(
        decide(&nested("[0 ms, 3 ms]", "(0 ms, 3 ms)")),
        Outcome::Violated { .. }
    ));
}

/// QSL-2129 MITL breadth, nested `until`: a timed `until` under a timed
/// `eventually` composes the same way. The reply is forced at 3 ms, so the
/// closed bounds hold and the open inner bound, which excludes 3 ms, does not.
#[trace("TC-697", "FR-242")]
#[test]
fn nested_until_operators_compose() {
    let nested = |outer: &str, inner: &str| {
        rpc_claim(
            4,
            TIMED,
            &format!(
                "temporal NestedUntil using tt over (c: M::Call) clock \"model-time\" on origin \
                 {{ always (holds(c.phase = 1) implies eventually{outer} \
                 (holds(c.phase = 1) until{inner} holds(not (c.phase = 1)))) }}\n"
            ),
        )
    };
    assert!(matches!(
        decide(&nested("[0 ms, 3 ms]", "[0 ms, 3 ms]")),
        Outcome::Holds { .. }
    ));
    assert!(matches!(
        decide(&nested("[0 ms, 3 ms]", "(0 ms, 3 ms)")),
        Outcome::Violated { .. }
    ));
}

/// FR-1034-AC-3 (TC-2969): over `Rpc` at `T = 4 ms`, claims with a lower
/// bound and no upper bound are decided by the zone search: the
/// `eventually[1 ms, *]` and `once[1 ms, *]` claims hold with accepted
/// certificates, `once(1 ms, *]` is refuted at the exact lower bound and
/// replays, and `until[4 ms, *]` is refuted by the server deadline.
#[trace("TC-2969", "FR-1034-AC-3")]
#[test]
fn open_upper_intervals_are_decided_by_zone_search() {
    let world = |body: &str| {
        rpc_claim(
            4,
            TIMED,
            &format!(
                "temporal Open using tt over (c: M::Call) clock \"model-time\" on origin \
                 {{ {body} }}\n"
            ),
        )
    };
    for body in [
        "always (holds(c.phase = 1) implies eventually[1 ms, *] holds(not (c.phase = 1)))",
        "always (holds(c.phase = 2) implies once[1 ms, *] holds(c.phase = 1))",
    ] {
        let world = world(body);
        let certificate = proved(decide(&world));
        assert_eq!(
            check(&world, &certificate, CertificateCheckLimits::default()),
            Ok(ZoneCertificateVerdict::Accepted),
            "{body}"
        );
    }
    let strict = world("always (holds(c.phase = 2) implies once(1 ms, *] holds(c.phase = 1))");
    let counterexample = violated(decide(&strict));
    assert_eq!(operations(&counterexample)[..2], ["send", "reply"]);
    assert_eq!(delays(&counterexample)[..2], [whole(0), whole(1)]);
    let settlement = replay(&strict, &counterexample).expect("replays");
    assert!(reproduced(&settlement), "{settlement:?}");
    let late = world(
        "always (holds(c.phase = 1) implies \
         (holds(c.phase = 1) until[4 ms, *] holds(c.phase = 2)))",
    );
    let _ = violated(decide(&late));
}

mod interval_boundaries {
    use super::*;
    use crate::timed_world::{document, unit_text, world, DENSE, PACKAGE};
    use filament_semantic_value::checking::{CheckMode, CheckingLimits};
    use fsl_analyze::model_check::{check_model, ModelCheckRequest};
    use fsl_eval::model_subject::ModelSubject;
    use fsl_eval::temporal::TemporalOutcome;
    use fsl_foundation::bound::InvalidTimedInterval;
    use fsl_foundation::diagnostic::Code;
    use fsl_foundation::{SourceIdentity, Span};
    use fsl_replay::{
        evaluate_model_trace, ModelCheckItem, ModelCheckLimits, ModelCheckOutcome, ModelTraceEnd,
    };
    use fsl_semantics::check::{CheckCause, PackageDeclarations, TemporalCause};
    use fsl_semantics::model::accounting::ModelNormalizationLimits;
    use fsl_semantics::model::intake::{admit_unit, package_input};

    // A pulse is true only in phase 1. Entry is forced at x=event and its
    // location is urgent, so clearing cannot leave it true at a later instant.
    fn pulse(event: u32, formula: &str) -> World {
        let model = document(
            &["x"],
            &[
                ("pulse", &["phase"]),
                ("clear", &["phase"]),
                ("idle", &["phase"]),
            ],
        );
        world(
            &model,
            TIMED,
            &format!(
                "{DENSE}
pre AtInstant using v on M::Call::pulse {{ self.phase = 0 and self.x >= {event} ms }}
post Pulsed using v on M::Call::pulse {{ self.phase = 1 }}
pre ClearPulse using v on M::Call::clear {{ self.phase = 1 }}
post Cleared using v on M::Call::clear {{ self.phase = 2 }}
pre IdleAllowed using v on M::Call::idle {{ self.phase = 2 }}
post Idled using v on M::Call::idle {{ self.phase = 2 }}
time invariant ReachInstant when {{ self.phase = 0 }} {{ self.x <= {event} ms }}
time invariant InstantOnly when {{ self.phase = 1 }} {{ self.x <= {event} ms }}
temporal Boundary using tt over (c: M::Call) clock \"model-time\" on origin {{ {formula} }}
"
            ),
        )
    }

    fn checked_verdict(world: &World, holds: bool) {
        if holds {
            let certificate = proved(decide(world));
            assert_eq!(
                check(world, &certificate, CertificateCheckLimits::default()),
                Ok(ZoneCertificateVerdict::Accepted)
            );
        } else {
            let counterexample = violated(decide(world));
            let settlement =
                replay(world, &counterexample).expect("actual timed counterexample re-executes");
            assert!(reproduced(&settlement), "{settlement:?}");
        }
    }

    fn eventual(interval: &str, event: u32, holds: bool) {
        let world = pulse(event, &format!("eventually{interval} holds(c.phase = 1)"));
        checked_verdict(&world, holds);
    }

    #[trace("FR-242", "FR-234", "FR-237")]
    #[test]
    fn open_lower_bound_excludes_the_boundary_instant() {
        eventual("(1 ms, 3 ms]", 2, true);
        eventual("(1 ms, 3 ms]", 1, false);
    }

    #[trace("FR-242", "FR-234", "FR-237")]
    #[test]
    fn closed_lower_bound_includes_the_boundary_instant() {
        eventual("[1 ms, 3 ms]", 1, true);
        eventual("[1 ms, 3 ms]", 0, false);
    }

    #[trace("FR-242", "FR-234", "FR-237")]
    #[test]
    fn open_upper_bound_excludes_the_boundary_instant() {
        eventual("[1 ms, 3 ms)", 2, true);
        eventual("[1 ms, 3 ms)", 3, false);
    }

    #[trace("FR-242", "FR-234", "FR-237")]
    #[test]
    fn closed_upper_bound_includes_the_boundary_instant() {
        eventual("[1 ms, 3 ms]", 3, true);
        eventual("[1 ms, 3 ms]", 4, false);
    }

    #[trace("FR-242-AC-2", "FR-234-AC-3", "FR-237")]
    #[test]
    fn punctual_bound_requires_the_exact_instant() {
        eventual("[2 ms, 2 ms]", 2, true);
        eventual("[2 ms, 2 ms]", 1, false);
        eventual("[2 ms, 2 ms]", 3, false);
    }

    #[trace("FR-242", "FR-234", "FR-237")]
    #[test]
    fn always_with_each_admitted_bound_kind() {
        for (interval, outside) in [
            ("[1 ms, 3 ms]", 4),
            ("(1 ms, 3 ms]", 1),
            ("[1 ms, 3 ms)", 3),
            ("(1 ms, 3 ms)", 1),
            ("[2 ms, 2 ms]", 1),
        ] {
            let formula = format!("always{interval} holds(not (c.phase = 1))");
            checked_verdict(&pulse(outside, &formula), true);
            checked_verdict(&pulse(2, &formula), false);
        }
    }

    /// A model with a `late` step only after the claim's outer window has
    /// closed, so only the subject's own product, which no claim prunes,
    /// reaches phase 3, where the claim's first atom is undefined.
    fn nested_with_a_late_phase() -> World {
        let model = document(
            &["x"],
            &[
                ("rise", &["phase"]),
                ("pulse", &["phase"]),
                ("delay", &["phase"]),
                ("idle", &["phase"]),
            ],
        );
        crate::timed_world::world_with_mode(
            &model,
            TIMED,
            &format!(
                "{DENSE}
function mod_defined using v(n: Int[0, 3]): Boolean pure {{ 6 mod (3 - n) >= 0 }}
pre RiseAtOne using v on M::Call::rise {{ self.phase = 0 and self.x >= 1 ms }}
post Rose using v on M::Call::rise {{ self.phase = 1 }}
pre PulseAtInstant using v on M::Call::pulse {{ self.phase = 1 and self.x >= 3 ms }}
post Pulsed using v on M::Call::pulse {{ self.phase = 2 }}
pre DelayAllowed using v on M::Call::delay {{ self.phase = 2 and self.x >= 7 ms }}
post Delayed using v on M::Call::delay {{ self.phase = 3 }}
pre IdleAllowed using v on M::Call::idle {{ self.phase = 3 }}
post Idled using v on M::Call::idle {{ self.phase = 3 }}
time invariant FirstDeadline when {{ self.phase = 0 }} {{ self.x <= 1 ms }}
time invariant SecondDeadline when {{ self.phase = 1 }} {{ self.x <= 3 ms }}
temporal NestedIntervals using tt over (c: M::Call) clock \"model-time\" on origin {{
    holds(c.phase = 0 or (c.phase = 3 and mod_defined(c.phase))) until[0 ms, 5 ms] (holds(c.phase = 1) until[1 ms, 2 ms] holds(c.phase = 2))
}}
",
            ),
            CheckMode::Kernel,
        )
    }

    /// FR-245: the admission instance reads no claim, so an atom undefined
    /// at a state only the subject's own product reaches does not reject a
    /// certificate the search proved.
    #[trace("FR-245", "FR-244")]
    #[test]
    fn an_atom_undefined_only_past_the_claims_window_leaves_the_certificate_accepted() {
        checked_verdict(&nested_with_a_late_phase(), true);
    }

    /// FR-245: the admission instance is evidence for `admitted` alone. A
    /// certificate whose witness is moved off its initial node, ends on a
    /// node that is not quiescent, or whose admission nodes are relabelled
    /// as the claim's own is rejected, never accepted on trust.
    #[trace("FR-245", "FR-244")]
    #[test]
    fn a_tampered_admission_instance_is_rejected() {
        use fsl_replay::DivergentTarget;
        let world = nested(3);
        let certificate = proved(decide(&world));
        let admission: Vec<usize> = certificate
            .nodes
            .iter()
            .enumerate()
            .filter(|(_, node)| node.product.is_some_and(|mark| mark.instance > 0))
            .map(|(index, _)| index)
            .collect();
        assert!(!admission.is_empty(), "an admission instance is carried");
        let rejects = |tampered: &crate::zone_search::Certificate| {
            matches!(
                check(&world, tampered, CertificateCheckLimits::default()),
                Ok(ZoneCertificateVerdict::Rejected(_))
            )
        };
        let mut moved = certificate.clone();
        let witness = moved.admitted.as_mut().expect("a witness");
        witness.start = (0..moved.nodes.len())
            .find(|node| !moved.initials.contains(node))
            .expect("a non-initial node");
        assert!(rejects(&moved));
        let mut unmet = certificate.clone();
        let witness = unmet.admitted.as_mut().expect("a witness");
        witness.target = DivergentTarget::Quiescent(witness.start);
        assert!(rejects(&unmet));
        let mut relabelled = certificate.clone();
        for &node in &admission {
            if let Some(mark) = relabelled.nodes[node].product.as_mut() {
                mark.instance = 0;
            }
        }
        assert!(rejects(&relabelled));
        let mut dropped = certificate.clone();
        dropped.initials.retain(|node| !admission.contains(node));
        assert!(rejects(&dropped));
    }

    fn nested(c_event: u32) -> World {
        let model = document(
            &["x"],
            &[
                ("rise", &["phase"]),
                ("pulse", &["phase"]),
                ("clear", &["phase"]),
                ("idle", &["phase"]),
            ],
        );
        world(
            &model,
            TIMED,
            &format!(
                "{DENSE}
pre RiseAtOne using v on M::Call::rise {{ self.phase = 0 and self.x >= 1 ms }}
post Rose using v on M::Call::rise {{ self.phase = 1 }}
pre PulseAtInstant using v on M::Call::pulse {{ self.phase = 1 and self.x >= {c_event} ms }}
post Pulsed using v on M::Call::pulse {{ self.phase = 2 }}
pre ClearPulse using v on M::Call::clear {{ self.phase = 2 }}
post Cleared using v on M::Call::clear {{ self.phase = 3 }}
pre IdleAllowed using v on M::Call::idle {{ self.phase = 3 }}
post Idled using v on M::Call::idle {{ self.phase = 3 }}
time invariant FirstDeadline when {{ self.phase = 0 }} {{ self.x <= 1 ms }}
time invariant SecondDeadline when {{ self.phase = 1 }} {{ self.x <= {c_event} ms }}
time invariant InstantOnly when {{ self.phase = 2 }} {{ self.x <= {c_event} ms }}
temporal NestedIntervals using tt over (c: M::Call) clock \"model-time\" on origin {{
    holds(c.phase = 0) until[0 ms, 5 ms] (holds(c.phase = 1) until[1 ms, 2 ms] holds(c.phase = 2))
}}
"
            ),
        )
    }

    #[trace("FR-242", "FR-234", "FR-237")]
    #[test]
    fn nested_until_with_inner_and_outer_bounds() {
        checked_verdict(&nested(3), true);
        checked_verdict(&nested(4), false);
    }

    fn untimed(formula: &str) -> World {
        let model = document(
            &[],
            &[
                ("pulse", &["phase"]),
                ("clear", &["phase"]),
                ("idle", &["phase"]),
            ],
        );
        world(
            &model,
            crate::zone_search::INFINITE,
            &format!(
                "
pre PulseAllowed using v on M::Call::pulse {{ self.phase = 0 }}
post Pulsed using v on M::Call::pulse {{ self.phase = 1 }}
pre ClearPulse using v on M::Call::clear {{ self.phase = 1 }}
post Cleared using v on M::Call::clear {{ self.phase = 2 }}
pre IdleAllowed using v on M::Call::idle {{ self.phase = 2 }}
post Idled using v on M::Call::idle {{ self.phase = 2 }}
temporal Projection using inf over (c: M::Call) clock \"model-steps\" on origin {{ {formula} }}
"
            ),
        )
    }

    fn untimed_verdict(world: &World) -> bool {
        let system = world.system();
        let node = &world.package.graph().temporal_clauses()[0];
        let letters = system.clause_letters(node).expect("actual model letters");
        let outcome = check_model(
            ModelCheckRequest {
                subject: ModelSubject {
                    simulation: world.request(),
                    proof_bounds: Vec::new(),
                },
                temporal_clock_selections: &[],
                item: ModelCheckItem::Clause {
                    clause: node.clause(),
                    letters: &letters,
                },
                max_depth: None,
                limits: ModelCheckLimits::default(),
                reductions: Default::default(),
            },
            &Cancel::new(),
        )
        .expect("the actual untimed checker admits the projected model");
        match outcome {
            ModelCheckOutcome::Holds { .. } => true,
            ModelCheckOutcome::Violated { counterexample, .. } => {
                let replayed = evaluate_model_trace(
                    &system,
                    &counterexample,
                    node.clause(),
                    &letters,
                    system.meter_limits(),
                )
                .expect("untimed counterexample re-executes");
                let ModelTraceEnd::Evaluated(run) = replayed else {
                    panic!("untimed replay stopped: {replayed:?}");
                };
                assert!(
                    matches!(run.outcome, Ok(TemporalOutcome::Violated { .. })),
                    "{:?}",
                    run.outcome
                );
                false
            }
            other => panic!("untimed projection must decide, got {other:?}"),
        }
    }

    #[trace("FR-1034", "FR-242", "FR-237")]
    #[test]
    fn unbounded_upper_bound_matches_the_actual_untimed_checker() {
        // All pulse events in this model occur at 2 ms, beyond the 1 ms
        // lower bound. Dropping clocks preserves the same discrete phase path.
        // No general equivalence of a positive timed lower bound is assumed.
        for (target, expected) in [(1, true), (3, false)] {
            let timed = pulse(2, &format!("eventually[1 ms, *] holds(c.phase = {target})"));
            let projected = untimed(&format!("eventually holds(c.phase = {target})"));
            let actual_untimed = untimed_verdict(&projected);
            assert_eq!(actual_untimed, expected);
            checked_verdict(&timed, actual_untimed);
        }
    }

    #[trace("FR-234-AC-2")]
    #[test]
    fn reversed_and_empty_open_intervals_are_typed_refusals_before_a_verdict() {
        for (interval, reason) in [
            ("[3 ms, 1 ms]", InvalidTimedInterval::Inverted),
            ("(2 ms, 2 ms]", InvalidTimedInterval::EmptyOpenEnd),
        ] {
            let model = document(&["x"], &[]);
            let text = unit_text(PACKAGE,TIMED,&format!("{DENSE}temporal Bad using tt over (c: M::Call) clock \"model-time\" on origin {{ eventually{interval} holds(c.phase = 1) }}"));
            let parsed = fsl_cst::parse(
                SourceIdentity::new("test", "zone-interval-refusal"),
                "zone-interval.native",
                text.as_bytes(),
                fsl_cst::Limits::default(),
            )
            .expect("source read");
            assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
            let built = fsl_forms::build_unit(&parsed).expect("interval syntax builds");
            let packages = package_input([model.as_slice()]).expect("model input");
            let models = admit_unit(
                &built.selections().models,
                &packages,
                ModelNormalizationLimits::default(),
            )
            .expect("model selection");
            let declarations = PackageDeclarations::assemble(
                parsed.source().reference().clone(),
                "zone-refusal".to_owned(),
                built,
                models,
                Vec::new(),
            )
            .expect("package assembly");
            let Err(refusals) = declarations.with_unwritten_timed_members().check(
                CheckMode::Linked,
                CheckingLimits::default(),
                &fsl_eval::value::S6aRewardEvaluator,
            ) else {
                panic!("{interval} must refuse, not produce a checked package");
            };
            let [refusal] = refusals.as_slice() else {
                panic!("one typed interval refusal: {refusals:?}");
            };
            let CheckCause::Temporal(cause) = &refusal.cause else {
                panic!("expected temporal refusal, got {refusal:?}");
            };
            let start = text.find(interval).expect("interval source");
            assert_eq!(
                cause.as_ref(),
                &TemporalCause::InvalidTimedInterval {
                    reason,
                    span: Span {
                        start,
                        end: start + interval.len()
                    }
                }
            );
            assert_eq!(cause.code(), Code::InvalidTimedModel);
            assert_eq!(cause.tag(), "invalid-interval");
        }
    }
}
