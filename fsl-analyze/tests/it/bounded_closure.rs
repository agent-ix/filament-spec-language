// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-338 and FR-127 for a bounded (TP-2) clause over the Counter subject
//! (`crate::counter`) from `c = 0`, where `inc` runs until the deadlock at 3:
//! EN-1's proof of `eventually[0,u] holds(A)` and `always[0,u] holds(A)`
//! carries a closure certificate of the model states under the canonical
//! bounded state each was read to, the core checker accepts or rejects it,
//! and the settlement map settles `proved` on its acceptance.
//!
//! Every expected certificate is written from the Counter arithmetic and the
//! counted-distance rule, not from the engine or the checker. An `on each`
//! clause, guarded or not, carries the one merged window its activations
//! share (FR-338 "Guarded `on each` activation"). Not covered: a lower
//! bound, an interval in an operand and past operators, which have no
//! canonical bounded state yet (Remaining work: QSL-878).

use filament_exact::{Cancel, Meter};
use fsl_analyze::model_check::{check_model, ModelCheckRequest};
use fsl_eval::automaton::{AutomatonStateKey, BoundedState, BoundedTranslation, StateContext};
use fsl_eval::model_subject::{AtomStop, ModelInstance, ModelLetters};
use fsl_eval::simulation::model::{ModelSimState, ModelSystem};
use fsl_eval::simulation::TransitionSystem;
use fsl_eval::temporal::{AtomReading, UndefinedCause};
use fsl_replay::{
    check_closure, settle_model_check, CertificateLocus, CertificateRejection, CertificateRequest,
    CertificateRule, ClauseIdentity, ClosureCertificate, IncompleteCause, ModelCheckItem,
    ModelCheckLimit, ModelCheckLimits, ModelCheckOutcome, ProductStateRef, ResultLabel,
    TemporalCertificateCheck,
};
use ix_trace_rs::trace;

use crate::counter::{reference, subject, value, Letters, UNLIMITED};
use crate::lasso_replay::{bounded_fixture, bounded_fixture_on};
use crate::liveness::Fixture;

/// The letters of a clause over the one counter `c`: its `holds` atom and,
/// for a guarded `on each`, its `when` condition, each a predicate of the
/// counter's value that is `None` where it is undefined.
struct Condition {
    atoms: Letters,
    when: fn(i64) -> Option<bool>,
}

impl ModelLetters for Condition {
    fn atom(
        &self,
        atom: fsl_semantics::check::FormulaNodeId,
        instance: &ModelInstance,
        state: &ModelSimState,
        meter: &mut Meter,
    ) -> Result<AtomReading, AtomStop> {
        self.atoms.atom(atom, instance, state, meter)
    }

    fn activation(
        &self,
        instance: &ModelInstance,
        state: &ModelSimState,
        _meter: &mut Meter,
    ) -> Result<AtomReading, AtomStop> {
        Ok(match (self.when)(value(state, &instance.key)) {
            Some(true) => AtomReading::True,
            Some(false) => AtomReading::False,
            None => AtomReading::Undefined(UndefinedCause::Kernel(
                filament_exact::Undefined::DivisionByZero,
            )),
        })
    }

    fn reads(&self, atom: fsl_semantics::check::FormulaNodeId) -> crate::units::Reads {
        self.atoms.reads(atom)
    }

    fn activation_reads(&self) -> crate::units::Reads {
        crate::units::unread()
    }
}

/// A bounded clause over the one counter `c`, with its letters.
struct Bounded {
    fixture: Fixture,
    letters: Condition,
}

/// The same clause under the infinite-trace profile (FR-125).
fn infinite(formula: &str, atom: fn(i64) -> Option<bool>) -> Bounded {
    Bounded {
        fixture: crate::liveness::counter(crate::counter::document(false), "", formula),
        letters: Condition {
            atoms: Letters { atom },
            when: |_| Some(true),
        },
    }
}

fn bounded(formula: &str, atom: fn(i64) -> Option<bool>) -> Bounded {
    Bounded {
        fixture: bounded_fixture(formula),
        letters: Condition {
            atoms: Letters { atom },
            when: |_| Some(true),
        },
    }
}

/// The same clause activated `on each` of the counter, with no `when`.
fn each(formula: &str, atom: fn(i64) -> Option<bool>) -> Bounded {
    Bounded {
        fixture: bounded_fixture_on("on each (q: M::Counter)", formula),
        letters: Condition {
            atoms: Letters { atom },
            when: |_| Some(true),
        },
    }
}

/// The same clause activated `on each` of the counter `when` its condition
/// holds; the source text's condition is the one `when` reads.
fn guarded(
    formula: &str,
    atom: fn(i64) -> Option<bool>,
    source: &str,
    when: fn(i64) -> Option<bool>,
) -> Bounded {
    Bounded {
        fixture: bounded_fixture_on(&format!("on each (q: M::Counter) when ({source})"), formula),
        letters: Condition {
            atoms: Letters { atom },
            when,
        },
    }
}

impl Bounded {
    fn item(&self) -> ModelCheckItem<'_> {
        ModelCheckItem::Clause {
            clause: &self.fixture.clauses[0],
            letters: &self.letters,
        }
    }

    fn request(&self) -> CertificateRequest<'_> {
        let fixture = &self.fixture;
        CertificateRequest {
            temporal_clock_selections: &fixture.temporal_clock_selections,
            subject: subject(
                &fixture.package,
                &fixture.documents,
                fixture.initial.clone(),
                &fixture.keys,
                UNLIMITED,
            ),
            item: self.item(),
            smt_encoding: fsl_replay::SmtEncodingLimits::default(),
        }
    }

    fn run(&self) -> ModelCheckOutcome {
        self.run_under(ModelCheckLimits::default(), None)
    }

    fn run_under(&self, limits: ModelCheckLimits, max_depth: Option<usize>) -> ModelCheckOutcome {
        let fixture = &self.fixture;
        check_model(
            ModelCheckRequest {
                temporal_clock_selections: &fixture.temporal_clock_selections,
                subject: subject(
                    &fixture.package,
                    &fixture.documents,
                    fixture.initial.clone(),
                    &fixture.keys,
                    UNLIMITED,
                ),
                item: self.item(),
                max_depth,
                limits,
                reductions: Default::default(),
            },
            &Cancel::new(),
        )
        .expect("checked")
    }

    fn translation(&self) -> BoundedTranslation {
        let fixture = &self.fixture;
        let identity = ClauseIdentity::resolve(
            &fixture.package,
            &fixture.documents.packages,
            &fixture.clauses[0],
            &fixture.temporal_clock_selections,
            &Cancel::new(),
            fsl_package::EmitLimits::fixture(),
        )
        .expect("the package declares the clause");
        let binding = vec![fsl_eval::automaton::ContextBinding::instance(
            identity.over().expect("an over node"),
            "c",
        )];
        let context =
            StateContext::clause(&fixture.clauses[0], identity.occurrence().clone(), binding);
        BoundedTranslation::clause(&fixture.clauses[0], &context).expect("translates")
    }

    /// The state key of the model state `c = v`.
    fn state_key(&self, v: i64) -> fsl_eval::simulation::StateKey {
        let fixture = &self.fixture;
        let system = ModelSystem::new(
            subject(
                &fixture.package,
                &fixture.documents,
                fixture.initial.clone(),
                &fixture.keys,
                UNLIMITED,
            )
            .simulation,
        )
        .expect("the subject admits");
        let mut state: ModelSimState = system.initial().remove(0);
        while value(&state, "c") != v {
            state = system
                .successors(&state)
                .expect("expands")
                .successors
                .remove(0)
                .1;
        }
        {
            let (bytes, _) = system.state_key(&state).expect("encodes");
            fsl_eval::simulation::StateKey::from_canonical_bytes(bytes)
        }
    }

    /// The product state of `c = v` read to `state`.
    fn at(&self, v: i64, state: BoundedState) -> ProductStateRef {
        ProductStateRef {
            model_state: self.state_key(v),
            automaton_state: self.translation().key(state).expect("encodes"),
        }
    }

    /// The product state of `c = v` read to `state`, its key's octets
    /// edited by `edit` and carried as an imported key.
    fn edited(
        &self,
        v: i64,
        state: BoundedState,
        edit: impl Fn(&str) -> String,
    ) -> ProductStateRef {
        let key = self.translation().key(state).expect("encodes");
        let text = std::str::from_utf8(key.as_bytes()).expect("a key is UTF-8");
        ProductStateRef {
            model_state: self.state_key(v),
            automaton_state: AutomatonStateKey::imported(edit(text).into_bytes()),
        }
    }

    fn certificate_of(&self, states: &[(i64, BoundedState)]) -> ClosureCertificate {
        let mut states: Vec<ProductStateRef> = states
            .iter()
            .map(|(v, state)| self.at(*v, *state))
            .collect();
        states.sort();
        ClosureCertificate { states }
    }
}

fn open(remaining: u64) -> BoundedState {
    BoundedState::Open { remaining }
}

fn is_at_least_one(v: i64) -> Option<bool> {
    Some(v >= 1)
}

fn certificate(outcome: &ModelCheckOutcome) -> ClosureCertificate {
    match outcome {
        ModelCheckOutcome::Holds {
            certificate: fsl_replay::ProofCertificate::Closure(certificate),
            ..
        } => certificate.clone(),
        other => panic!("expected Holds, got {other:?}"),
    }
}

fn check(
    request: &CertificateRequest<'_>,
    certificate: &ClosureCertificate,
) -> TemporalCertificateCheck {
    check_closure(
        request,
        certificate,
        &ModelCheckLimits::default(),
        &Cancel::new(),
    )
    .expect("the request is checked")
}

fn rejected(rule: CertificateRule, at: ProductStateRef) -> TemporalCertificateCheck {
    TemporalCertificateCheck::Rejected(CertificateRejection {
        rule,
        at: CertificateLocus::ProductState(at),
    })
}

/// FR-338-AC-6: `eventually[0,1] holds(c.value >= 1)` from `c = 0` reads p
/// false then true, so the monitor advances `Open(1)` to `Accept` and stays
/// there past the horizon. The proof's certificate is exactly those four
/// product states, the core checker accepts it, and the map settles
/// `proved`.
#[trace("TC-525", "FR-338-AC-6")]
#[test]
fn a_bounded_eventually_that_is_met_carries_a_certificate_the_core_accepts() {
    let clause = bounded("eventually[0,1] holds(c.value >= 1)", is_at_least_one);
    let outcome = clause.run();
    let expected = clause.certificate_of(&[
        (0, open(1)),
        (1, BoundedState::Accept),
        (2, BoundedState::Accept),
        (3, BoundedState::Accept),
    ]);
    assert_eq!(certificate(&outcome), expected);
    let request = clause.request();
    assert_eq!(
        check(&request, &expected),
        TemporalCertificateCheck::Accepted
    );
    let record = settle_model_check(
        &outcome,
        &ModelCheckLimits::default(),
        &request,
        &Cancel::new(),
        |_| panic!("a proof replays nothing"),
    );
    assert_eq!(record.label, ResultLabel::Proved);
}

/// FR-338-AC-6: p false then false rejects. `eventually[0,1]` over a letter
/// that reads false at 0 and 1 is refuted by EN-1, which carries no
/// certificate.
#[trace("TC-525", "FR-338-AC-6")]
#[test]
fn a_bounded_eventually_that_is_missed_is_refuted() {
    let clause = bounded("eventually[0,1] holds(c.value >= 2)", |v| Some(v >= 2));
    assert!(matches!(clause.run(), ModelCheckOutcome::Violated { .. }));
}

/// FR-338-AC-6: a certificate missing a reached product state is rejected at
/// it, `SuccessorMissing` for a successor and `InitialMissing` for the
/// initial state; a certificate that holds a state under another bounded
/// state than it was read to is missing the one it was read to.
#[trace("TC-525", "FR-338-AC-6")]
#[test]
fn a_bounded_certificate_missing_a_reached_state_is_rejected_at_it() {
    let clause = bounded("eventually[0,1] holds(c.value >= 1)", is_at_least_one);
    let full = certificate(&clause.run());
    let request = clause.request();
    let without = |v: i64, state: BoundedState| {
        let at = clause.at(v, state);
        let mut certificate = full.clone();
        certificate.states.retain(|held| *held != at);
        assert_eq!(certificate.states.len(), full.states.len() - 1);
        (certificate, at)
    };
    let (successor, at) = without(2, BoundedState::Accept);
    assert_eq!(
        check(&request, &successor),
        rejected(CertificateRule::SuccessorMissing, at)
    );
    let (initial, at) = without(0, open(1));
    assert_eq!(
        check(&request, &initial),
        rejected(CertificateRule::InitialMissing, at)
    );
    let mut renamed = full.clone();
    renamed
        .states
        .retain(|held| *held != clause.at(1, BoundedState::Accept));
    renamed.states.push(clause.at(1, open(0)));
    assert_eq!(
        check(&request, &renamed),
        rejected(
            CertificateRule::SuccessorMissing,
            clause.at(1, BoundedState::Accept)
        )
    );
}

/// FR-338-AC-6: the final counted-distance region and the terminal false
/// extension give the evaluator's result. `always[0,3] holds(c.value >= 0)`
/// ends its window at the terminal state `c = 3` (`Open(0)`), which the
/// false extension discharges, so it holds and its certificate is accepted;
/// `always[0,4]` leaves a counted step in the window at `c = 3` (`Open(1)`),
/// where the false extension's false letter rejects, so a certificate that
/// holds the states is rejected `BadState` at the terminal state.
#[trace("TC-525", "FR-338-AC-6")]
#[test]
fn the_terminal_false_extension_closes_the_last_region() {
    let closes = bounded("always[0,3] holds(c.value >= 0)", |v| Some(v >= 0));
    let outcome = closes.run();
    let expected = closes.certificate_of(&[(0, open(3)), (1, open(2)), (2, open(1)), (3, open(0))]);
    assert_eq!(certificate(&outcome), expected);
    assert_eq!(
        check(&closes.request(), &expected),
        TemporalCertificateCheck::Accepted
    );

    let open_at_end = bounded("always[0,4] holds(c.value >= 0)", |v| Some(v >= 0));
    assert!(matches!(
        open_at_end.run(),
        ModelCheckOutcome::Violated { .. }
    ));
    let held =
        open_at_end.certificate_of(&[(0, open(4)), (1, open(3)), (2, open(2)), (3, open(1))]);
    assert_eq!(
        check(&open_at_end.request(), &held),
        rejected(CertificateRule::BadState, open_at_end.at(3, open(1)))
    );
}

/// FR-338-AC-6 and ADR-018 UE-1: the claim reads positions 0 to its horizon,
/// the positions after the monitor accepts included, and no later one. With
/// the atom undefined at `c = 2`, `eventually[0,1]` never reads position 2
/// and holds, while `eventually[0,2]` reads it after accepting at 1: EN-1
/// finds the undefined evaluation and the checker rejects a certificate of
/// the states `BadState` at the state read to undefined.
#[trace("TC-525", "FR-338-AC-6")]
#[test]
fn the_positions_of_the_horizon_are_read_after_the_monitor_accepts() {
    fn undefined_at_two(v: i64) -> Option<bool> {
        (v != 2).then_some(v >= 1)
    }
    let unread = bounded("eventually[0,1] holds(c.value >= 1)", undefined_at_two);
    let proof = certificate(&unread.run());
    assert_eq!(
        check(&unread.request(), &proof),
        TemporalCertificateCheck::Accepted
    );

    let read = bounded("eventually[0,2] holds(c.value >= 1)", undefined_at_two);
    assert!(matches!(read.run(), ModelCheckOutcome::Violated { .. }));
    let held = read.certificate_of(&[
        (0, open(2)),
        (1, BoundedState::Accept),
        (2, BoundedState::Accept),
        (3, BoundedState::Accept),
    ]);
    assert_eq!(
        check(&read.request(), &held),
        rejected(CertificateRule::BadState, read.at(2, BoundedState::Reject))
    );
}

/// A budget stops the checker with the typed cause and never accepts: a
/// certificate padded past `max_states` entries stops at `MaxStates`, and
/// `max_transitions` 1 stops the first expansion at `MaxTransitions` naming
/// the limit and value.
#[trace("TC-525", "FR-338-AC-6")]
#[test]
fn a_stopped_bounded_check_names_what_stopped_it() {
    let clause = bounded("eventually[0,1] holds(c.value >= 1)", is_at_least_one);
    let proof = certificate(&clause.run());
    let request = clause.request();
    let stop = |limit, value| {
        TemporalCertificateCheck::Stopped(IncompleteCause::LimitReached {
            limit,
            value,
            required: None,
        })
    };
    let mut states = ModelCheckLimits::default();
    states.limits.max_states = 2;
    let mut transitions = ModelCheckLimits::default();
    transitions.limits.max_transitions = 1;
    for (limits, expected) in [
        (states, stop(ModelCheckLimit::MaxStates, 2)),
        (transitions, stop(ModelCheckLimit::MaxTransitions, 1)),
    ] {
        assert_eq!(
            check_closure(&request, &proof, &limits, &Cancel::new()).expect("checked"),
            expected
        );
    }
}

#[trace("FR-196-AC-6", "FR-201-AC-9")]
#[test]
fn a_formula_product_embeds_the_real_complete_monitor_without_inner_transport() {
    use fsl_replay::{ProbabilityDocumentIoLimits, ProbabilityProductKey};
    let bounded = bounded("eventually[0,2] holds(c.value = 2)", |n| Some(n == 2));
    let translation = bounded.translation();
    let monitor = translation
        .key(translation.start(false))
        .expect("actual FR338 key");
    let model = bounded.state_key(0);
    let binding = reference(&bounded.fixture.package, &bounded.fixture.documents, "c");
    let key = ProbabilityProductKey::for_formula_state(
        Some(&binding),
        model.as_bytes(),
        &monitor,
        ProbabilityDocumentIoLimits::default(),
        &Cancel::new(),
    )
    .expect("both owner keys admit");
    let parsed: serde_json::Value = serde_json::from_slice(key.as_bytes()).unwrap();
    assert_eq!(
        parsed[4],
        serde_json::json!([
            "filament.simulation.state-key/v1",
            serde_json::from_slice::<serde_json::Value>(model.as_bytes()).unwrap()
        ])
    );
    assert_eq!(
        parsed[5],
        serde_json::json!([
            "formula",
            serde_json::from_slice::<serde_json::Value>(monitor.as_bytes()).unwrap()
        ])
    );
    let document = fsl_replay::ProbabilityCertificateDocument {
        cyclic_dtmc: false,
        reward: None,
        event: None,
        timed: None,
        failures: Vec::new(),
        identity: fsl_package::ObligationIdentity::new(
            filament_exact::NodeKey::new(
                "fixture",
                vec![filament_exact::PathSegment::Text("formula".to_owned())],
                0,
            )
            .unwrap(),
            None,
            0,
        ),
        optimization: fsl_replay::ProbabilityObjective::Minimum,
        targets: vec![key.clone()],
        values: vec![fsl_replay::ExactProbabilityValue {
            state: key.clone(),
            value: filament_exact::Rational::from_integer(filament_exact::Integer::one()),
        }],
        ranking: vec![(key, 0)],
    };
    let limits = fsl_replay::ProbabilityDocumentLimits {
        bytes: 1_000_000,
        storage_bytes: 8_000_000,
        states: 100,
        edges: 100,
        scalar_bits: 128,
        accounting: fsl_replay::AccountingLimits::default().0,
    };
    let bytes = fsl_replay::write_probability_certificate(&document, limits, &Cancel::new())
        .expect("structural formula keys admit");
    assert_eq!(
        fsl_replay::read_probability_certificate(&bytes, limits, &Cancel::new()).unwrap(),
        document
    );
}

/// FR-338-AC-6 under the infinite-trace profile (FR-125): the terminal
/// state `c = 3` stutters for ever, each stutter a counted step that
/// re-reads its letter, so the window runs out on the stutter and
/// `always[0,4]` is met where the false extension would refute it.
#[trace("TC-525", "FR-338-AC-6")]
#[test]
fn an_infinite_always_closes_its_window_over_the_terminal_stutter() {
    let clause = infinite("always[0,4] holds(c.value >= 0)", |v| Some(v >= 0));
    let outcome = clause.run();
    let expected = clause.certificate_of(&[
        (0, open(4)),
        (1, open(3)),
        (2, open(2)),
        (3, open(1)),
        (3, open(0)),
        (3, BoundedState::Accept),
    ]);
    assert_eq!(certificate(&outcome), expected);
    let request = clause.request();
    assert_eq!(
        check(&request, &expected),
        TemporalCertificateCheck::Accepted
    );
    let record = settle_model_check(
        &outcome,
        &ModelCheckLimits::default(),
        &request,
        &Cancel::new(),
        |_| panic!("a proof replays nothing"),
    );
    assert_eq!(record.label, ResultLabel::Proved);
    // The tampered certificates: each stutter state is a reached state, and
    // a state held under another monitor state is the one it was read to
    // missing.
    for (v, state) in [
        (3, open(1)),
        (3, open(0)),
        (3, BoundedState::Accept),
        (0, open(4)),
    ] {
        let at = clause.at(v, state);
        let mut without = expected.clone();
        without.states.retain(|held| *held != at);
        let rule = if v == 0 {
            CertificateRule::InitialMissing
        } else {
            CertificateRule::SuccessorMissing
        };
        assert_eq!(check(&request, &without), rejected(rule, at));
    }
    let mut renamed = expected.clone();
    let accepted = clause.at(3, BoundedState::Accept);
    renamed.states.retain(|held| *held != accepted);
    renamed.states.push(clause.at(3, open(2)));
    assert_eq!(
        check(&request, &renamed),
        rejected(CertificateRule::SuccessorMissing, accepted)
    );
}

/// FR-125: an infinite `eventually` whose atom the stutter never makes true
/// is refuted; a certificate of the states its window passes through is
/// rejected `BadState` where the stutter exhausts the window, so no
/// certificate of a false claim is accepted.
#[trace("TC-525", "FR-338-AC-6")]
#[test]
fn an_infinite_eventually_the_stutter_never_meets_has_no_accepted_certificate() {
    let clause = infinite("eventually[0,5] holds(c.value >= 4)", |v| Some(v >= 4));
    assert!(matches!(clause.run(), ModelCheckOutcome::Violated { .. }));
    let held = clause.certificate_of(&[
        (0, open(5)),
        (1, open(4)),
        (2, open(3)),
        (3, open(2)),
        (3, open(1)),
        (3, open(0)),
        (3, BoundedState::Reject),
    ]);
    assert_eq!(
        check(&clause.request(), &held),
        rejected(
            CertificateRule::BadState,
            clause.at(3, BoundedState::Reject)
        )
    );
    // The same window with the atom true at the terminal state is met on
    // the first stutter-free position it holds.
    let met = infinite("eventually[0,5] holds(c.value >= 3)", |v| Some(v >= 3));
    assert!(matches!(met.run(), ModelCheckOutcome::Holds { .. }));
}

fn is_three(v: i64) -> Option<bool> {
    Some(v == 3)
}

fn is_at_least_two(v: i64) -> Option<bool> {
    Some(v >= 2)
}

fn is_at_most_three(v: i64) -> Option<bool> {
    Some(v <= 3)
}

fn is_at_most_one(v: i64) -> Option<bool> {
    Some(v <= 1)
}

fn is_zero(v: i64) -> Option<bool> {
    Some(v == 0)
}

/// What a certificate of `clause` is checked as, and settled as when the
/// engine's own proof is accepted.
fn proved_and_accepted(clause: &Bounded, expected: &[(i64, BoundedState)]) -> ClosureCertificate {
    let outcome = clause.run();
    let expected = clause.certificate_of(expected);
    assert_eq!(certificate(&outcome), expected);
    let request = clause.request();
    assert_eq!(
        check(&request, &expected),
        TemporalCertificateCheck::Accepted
    );
    let record = settle_model_check(
        &outcome,
        &ModelCheckLimits::default(),
        &request,
        &Cancel::new(),
        |_| panic!("a proof replays nothing"),
    );
    assert_eq!(record.label, ResultLabel::Proved);
    expected
}

/// FR-338-AC-10: `on each eventually[0,1] holds(c.value >= 1)` from `c = 0`
/// opens a debt at position 0 that the letter at 1 discharges, and every
/// later activation is satisfied where it is made, so the monitor idles
/// from position 1. The certificate is those four product states, with the
/// `["each"]` clock and no live window at `Idle`; the checker accepts it
/// and rejects each tampering of it: a missing state, an initial state
/// missing, a deadline or idle state under another key, a clock with a
/// stray remaining distance or another activation's, a second branch, and
/// a key that is no state at all.
#[trace("TC-525", "FR-338-AC-28")]
#[test]
fn an_each_eventually_idles_once_its_debt_is_discharged() {
    let clause = each("eventually[0,1] holds(c.value >= 1)", is_at_least_one);
    let full = proved_and_accepted(
        &clause,
        &[
            (0, open(1)),
            (1, BoundedState::Idle),
            (2, BoundedState::Idle),
            (3, BoundedState::Idle),
        ],
    );
    let request = clause.request();
    let without = |v: i64, state: BoundedState| {
        let mut certificate = full.clone();
        certificate
            .states
            .retain(|held| *held != clause.at(v, state));
        assert_eq!(certificate.states.len(), full.states.len() - 1);
        certificate
    };
    assert_eq!(
        check(&request, &without(1, BoundedState::Idle)),
        rejected(
            CertificateRule::SuccessorMissing,
            clause.at(1, BoundedState::Idle)
        )
    );
    assert_eq!(
        check(&request, &without(0, open(1))),
        rejected(CertificateRule::InitialMissing, clause.at(0, open(1)))
    );
    // A deadline of another distance is another state: the initial state is
    // then missing, and an idle state held as a debt is a missing successor.
    let mut shifted = without(0, open(1));
    shifted.states.push(clause.at(0, open(0)));
    shifted.states.sort();
    assert_eq!(
        check(&request, &shifted),
        rejected(CertificateRule::InitialMissing, clause.at(0, open(1)))
    );
    let mut owing = without(2, BoundedState::Idle);
    owing.states.push(clause.at(2, open(1)));
    owing.states.sort();
    assert_eq!(
        check(&request, &owing),
        rejected(
            CertificateRule::SuccessorMissing,
            clause.at(2, BoundedState::Idle)
        )
    );
    // A key that is not a state of the translation rejects the whole
    // certificate `ShapeMismatch`, at the least refused state.
    let idle_keys: [(&str, &str); 5] = [
        (r#"["each"],"#, r#"["each","1"],"#),
        (r#"["each"],"#, r#"["origin","1"],"#),
        (r#"["each"],"#, r#"[],"#),
        (r#"[[[],[],[],[]]]"#, r#"[[[],[],[],[]],[[],[],[],[]]]"#),
        (r#"[[[],[],[],[]]]"#, r#"[]"#),
    ];
    for (from, to) in idle_keys {
        let bad = clause.edited(2, BoundedState::Idle, |key| key.replace(from, to));
        let mut certificate = without(2, BoundedState::Idle);
        certificate.states.push(bad.clone());
        certificate.states.sort();
        assert_eq!(
            check(&request, &certificate),
            rejected(CertificateRule::ShapeMismatch, bad),
            "{from} to {to}"
        );
    }
    let stray = clause.edited(0, open(1), |key| {
        key.replace(r#"["each"]"#, r#"["each","1"]"#)
    });
    let mut certificate = without(0, open(1));
    certificate.states.push(stray.clone());
    certificate.states.sort();
    assert_eq!(
        check(&request, &certificate),
        rejected(CertificateRule::ShapeMismatch, stray)
    );
}

/// FR-338-AC-10: a debt the window closes on is a rejecting state, which no
/// certificate holds. `on each eventually[0,0] holds(c.value >= 1)` opens a
/// debt at 0 that its one position cannot discharge, so the step to 1 closes
/// the window and EN-1 refutes it; a certificate holding the states the
/// debt would pass through is rejected `BadState` at the product state the
/// closed window reaches, and an `on each` clause whose window outlasts the
/// terminal state is rejected there too.
#[trace("TC-525", "FR-338-AC-28")]
#[test]
fn an_each_debt_the_window_closes_on_is_a_rejecting_state() {
    let missed = each("eventually[0,0] holds(c.value >= 1)", is_at_least_one);
    assert!(matches!(missed.run(), ModelCheckOutcome::Violated { .. }));
    let held = missed.certificate_of(&[
        (0, open(0)),
        (1, BoundedState::Idle),
        (2, BoundedState::Idle),
        (3, BoundedState::Idle),
    ]);
    assert_eq!(
        check(&missed.request(), &held),
        rejected(
            CertificateRule::BadState,
            missed.at(1, BoundedState::Reject)
        )
    );
    // Every position is an activation, so at the terminal state a window of
    // one more position is opened and the false extension fails it.
    let outlasting = each("eventually[0,1] holds(c.value >= 0)", |v| Some(v >= 0));
    let held = outlasting.certificate_of(&[
        (0, BoundedState::Idle),
        (1, BoundedState::Idle),
        (2, BoundedState::Idle),
        (3, BoundedState::Idle),
    ]);
    assert_eq!(
        check(&outlasting.request(), &held),
        TemporalCertificateCheck::Accepted
    );
    let late = each("eventually[0,2] holds(c.value >= 2)", is_at_least_two);
    let held = late.certificate_of(&[
        (0, open(2)),
        (1, open(1)),
        (2, BoundedState::Idle),
        (3, BoundedState::Idle),
    ]);
    assert_eq!(
        check(&late.request(), &held),
        TemporalCertificateCheck::Accepted
    );
    let never = each("eventually[0,1] holds(c.value = 3)", is_three);
    assert!(matches!(never.run(), ModelCheckOutcome::Violated { .. }));
    let held = never.certificate_of(&[(0, open(1)), (1, open(0)), (2, open(0)), (3, open(0))]);
    assert_eq!(
        check(&never.request(), &held),
        rejected(CertificateRule::BadState, never.at(2, BoundedState::Reject))
    );
}

/// FR-338-AC-10: `on each eventually[0,2] holds(c.value >= 2)` has two live
/// activations at position 1, and the one window it keeps is the earlier
/// deadline, `Open(1)`; the letter at 2 discharges both.
#[trace("TC-525", "FR-338-AC-28")]
#[test]
fn overlapping_each_activations_keep_the_earlier_deadline() {
    let clause = each("eventually[0,2] holds(c.value >= 2)", is_at_least_two);
    let full = proved_and_accepted(
        &clause,
        &[
            (0, open(2)),
            (1, open(1)),
            (2, BoundedState::Idle),
            (3, BoundedState::Idle),
        ],
    );
    // The later deadline, `Open(2)`, which a second activation would give if
    // it replaced the earlier one, is not the state the position reads to.
    let mut later = full.clone();
    later.states.retain(|held| *held != clause.at(1, open(1)));
    later.states.push(clause.at(1, open(2)));
    later.states.sort();
    assert_eq!(
        check(&clause.request(), &later),
        rejected(CertificateRule::SuccessorMissing, clause.at(1, open(1)))
    );
}

/// FR-338-AC-28: a state with the same window entry twice in its slot is no
/// state of the translation, whose windows merge to one per slot.
#[trace("TC-525", "FR-338-AC-28")]
#[test]
fn an_each_key_that_duplicates_its_window_entry_is_rejected() {
    let clause = each("eventually[0,2] holds(c.value >= 2)", is_at_least_two);
    let full = proved_and_accepted(
        &clause,
        &[
            (0, open(2)),
            (1, open(1)),
            (2, BoundedState::Idle),
            (3, BoundedState::Idle),
        ],
    );
    let entry = r#"[[],"positive","deadline","1"]"#;
    let doubled = |key: &str| key.replace(entry, &format!("{entry},{entry}"));
    let mut twice = full.clone();
    twice.states.retain(|held| *held != clause.at(1, open(1)));
    twice.states.push(clause.edited(1, open(1), doubled));
    twice.states.sort();
    assert_eq!(
        check(&clause.request(), &twice),
        rejected(
            CertificateRule::ShapeMismatch,
            clause.edited(1, open(1), doubled)
        )
    );
}

/// FR-338-AC-10: `on each always[0,0] holds(c.value <= 3)` activates at
/// every position and requires only that one, so each state is `Open(0)`,
/// restored by the next activation after the step out of it discharges the
/// extent. A longer window, which the terminal state's false extension
/// fails, and a letter that fails inside an open window are rejecting
/// states.
#[trace("TC-525", "FR-338-AC-28")]
#[test]
fn an_each_always_keeps_the_furthest_extent_and_rejects_a_failing_letter() {
    let clause = each("always[0,0] holds(c.value <= 3)", is_at_most_three);
    let full = proved_and_accepted(
        &clause,
        &[(0, open(0)), (1, open(0)), (2, open(0)), (3, open(0))],
    );
    let request = clause.request();
    let mut idle = full.clone();
    idle.states.retain(|held| *held != clause.at(3, open(0)));
    idle.states.push(clause.at(3, BoundedState::Idle));
    idle.states.sort();
    assert_eq!(
        check(&request, &idle),
        rejected(CertificateRule::SuccessorMissing, clause.at(3, open(0)))
    );
    let mut extent = full.clone();
    extent.states.retain(|held| *held != clause.at(2, open(0)));
    extent.states.push(clause.edited(2, open(0), |key| {
        key.replace(r#""extent","0""#, r#""extent","1""#)
    }));
    extent.states.sort();
    // An extent past the clause's horizon is no state of its translation.
    assert_eq!(
        check(&request, &extent),
        rejected(
            CertificateRule::ShapeMismatch,
            clause.edited(2, open(0), |key| key
                .replace(r#""extent","0""#, r#""extent","1""#))
        )
    );
    // A window past the terminal state is failed by the false extension.
    let long = each("always[0,1] holds(c.value <= 3)", is_at_most_three);
    assert!(matches!(long.run(), ModelCheckOutcome::Violated { .. }));
    let held = long.certificate_of(&[(0, open(1)), (1, open(1)), (2, open(1)), (3, open(1))]);
    assert_eq!(
        check(&long.request(), &held),
        rejected(CertificateRule::BadState, long.at(3, open(1)))
    );
    // A failing letter at 2 inside the window opened at 1.
    let failing = each("always[0,1] holds(c.value <= 1)", is_at_most_one);
    assert!(matches!(failing.run(), ModelCheckOutcome::Violated { .. }));
    let held = failing.certificate_of(&[(0, open(1)), (1, open(1)), (2, open(0))]);
    assert_eq!(
        check(&failing.request(), &held),
        rejected(
            CertificateRule::BadState,
            failing.at(2, BoundedState::Reject)
        )
    );
}

/// FR-338-AC-10 (guarded `on each`): `eventually[0,1] holds(c.value = 3)`
/// when `c.value >= 2` idles at values 0 and 1, where its condition is
/// false, opens a debt at 2 and discharges it at 3. A certificate whose
/// value-2 state is read as idle (the condition misread as false) is
/// rejected `SuccessorMissing` at the state with the deadline, and is
/// accepted when the misread state is added beside it; every key's
/// activation replaced by `each` rejects the whole certificate
/// `ShapeMismatch` at the least refused state.
#[trace("TC-525", "FR-338-AC-10")]
#[test]
fn a_guarded_each_reads_its_condition_through_the_seam() {
    let clause = guarded(
        "eventually[0,1] holds(c.value = 3)",
        is_three,
        "q.value >= 2",
        is_at_least_two,
    );
    let full = proved_and_accepted(
        &clause,
        &[
            (0, BoundedState::Idle),
            (1, BoundedState::Idle),
            (2, open(1)),
            (3, BoundedState::Idle),
        ],
    );
    let request = clause.request();
    let mut misread = full.clone();
    misread.states.retain(|held| *held != clause.at(2, open(1)));
    misread.states.push(clause.at(2, BoundedState::Idle));
    misread.states.sort();
    assert_eq!(
        check(&request, &misread),
        rejected(CertificateRule::SuccessorMissing, clause.at(2, open(1)))
    );
    let mut beside = full.clone();
    beside.states.push(clause.at(2, BoundedState::Idle));
    beside.states.sort();
    assert_eq!(check(&request, &beside), TemporalCertificateCheck::Accepted);
    let each_keys: Vec<ProductStateRef> = [
        (0, BoundedState::Idle),
        (1, BoundedState::Idle),
        (2, open(1)),
        (3, BoundedState::Idle),
    ]
    .iter()
    .map(|(v, state)| clause.edited(*v, *state, |key| key.replace("each-when", "each")))
    .collect();
    let least = each_keys
        .iter()
        .min_by_key(|state| {
            (
                state.model_state.as_bytes().to_vec(),
                state.automaton_state.as_bytes().to_vec(),
            )
        })
        .expect("a state")
        .clone();
    let renamed = ClosureCertificate {
        states: each_keys.clone(),
    };
    assert_eq!(
        check(&request, &renamed),
        rejected(CertificateRule::ShapeMismatch, least)
    );
}

/// FR-338-AC-10 (guarded `on each`): a condition that reads undefined at a
/// model state is a violation, as an undefined atom is (ADR-018 UE-1): EN-1
/// refutes the clause and the checker rejects `BadState` at the state read
/// to undefined, whatever the certificate holds.
#[trace("TC-525", "FR-338-AC-10")]
#[test]
fn a_guarded_each_condition_that_reads_undefined_is_rejected() {
    fn undefined_at_two(v: i64) -> Option<bool> {
        (v != 2).then_some(false)
    }
    let clause = guarded(
        "eventually[0,1] holds(c.value = 3)",
        is_three,
        "q.value >= 2",
        undefined_at_two,
    );
    assert!(matches!(clause.run(), ModelCheckOutcome::Violated { .. }));
    let held = clause.certificate_of(&[(0, BoundedState::Idle), (1, BoundedState::Idle)]);
    assert_eq!(
        check(&clause.request(), &held),
        rejected(
            CertificateRule::BadState,
            clause.at(2, BoundedState::Reject)
        )
    );
}

/// FR-338-AC-10 (guarded `on each`): an extent that survives to its closed
/// boundary discharges, and the letter after it is read against no
/// obligation. `always[0,1] holds(c.value <= 1)` when `c.value = 0`
/// activates once, at position 0, so the extent runs `Open(1)`, `Open(0)`,
/// then idles at 2, where the letter fails without consequence.
#[trace("TC-525", "FR-338-AC-10")]
#[test]
fn a_guarded_each_extent_discharges_at_its_boundary() {
    let clause = guarded(
        "always[0,1] holds(c.value <= 1)",
        is_at_most_one,
        "q.value = 0",
        |v| Some(v == 0),
    );
    let full = proved_and_accepted(
        &clause,
        &[
            (0, open(1)),
            (1, open(0)),
            (2, BoundedState::Idle),
            (3, BoundedState::Idle),
        ],
    );
    let request = clause.request();
    let mut longer = full.clone();
    longer.states.retain(|held| *held != clause.at(1, open(0)));
    longer.states.push(clause.at(1, open(1)));
    longer.states.sort();
    assert_eq!(
        check(&request, &longer),
        rejected(CertificateRule::SuccessorMissing, clause.at(1, open(0)))
    );
    let mut owing = full.clone();
    owing
        .states
        .retain(|held| *held != clause.at(2, BoundedState::Idle));
    owing.states.push(clause.at(2, open(0)));
    owing.states.sort();
    assert_eq!(
        check(&request, &owing),
        rejected(
            CertificateRule::SuccessorMissing,
            clause.at(2, BoundedState::Idle)
        )
    );
}

/// FR-338-AC-10 (guarded `on each`), FR-337: the producer keeps `Open { 0 }`
/// and `Idle` apart at a model state reached both ways. The counter goes
/// `0 -inc-> 1 -setTo-> 3` and `0 -setTo-> 3`; `on each when (q.value = 0)
/// always[0,1] holds(c.value >= 0)` activates at 0 only, so `3` is read
/// `Open { 0 }` by the direct step and `Idle` by the two-step path. Merging
/// the two by the general monitor state left a certificate its own checker
/// refused `SuccessorMissing`; the proof is accepted and settles proved.
#[trace("TC-525", "FR-338-AC-10")]
#[test]
fn a_guarded_each_window_reached_two_ways_is_one_certificate_the_checker_accepts() {
    fn always_true(_: i64) -> Option<bool> {
        Some(true)
    }
    let contract = "pre Step using v on M::Counter::inc { self.value = 0 }\n\
         post Stepped using v on M::Counter::inc { self.value - pre(self.value) = 1 }\n\
         pre Jump using v on M::Counter::setTo { (self.value = 0 or self.value = 1) and n = 3 }\n\
         post Jumped using v on M::Counter::setTo { self.value = n }\n";
    let clause = Bounded {
        fixture: crate::lasso_replay::bounded_fixture_over(
            crate::counter::document_with_ranged_parameter(),
            contract,
            "on each (q: M::Counter) when (q.value = 0)",
            "always[0,1] holds(c.value >= 0)",
        ),
        letters: Condition {
            atoms: Letters { atom: always_true },
            when: is_zero,
        },
    };
    let outcome = clause.run();
    let held = certificate(&outcome);
    let request = clause.request();
    assert_eq!(check(&request, &held), TemporalCertificateCheck::Accepted);
    let record = settle_model_check(
        &outcome,
        &ModelCheckLimits::default(),
        &request,
        &Cancel::new(),
        |_| panic!("a proof replays nothing"),
    );
    assert_eq!(record.label, ResultLabel::Proved);
}
