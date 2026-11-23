// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-338-AC-30 (TC-525 step 17): an infinite-trace TP-3 clause under an
//! unguarded or a guarded `on each` activation is keyed by its monitor's
//! viable configurations, the core checker recomputes every successor, and a
//! certificate is accepted only under the clause's own context.
//!
//! The Counter subject runs the values 0 to 3 and then stutters the terminal
//! state, so every model position and every stutter position is an activation
//! position. The rejection loci are computed from that arithmetic.

use fsl_eval::automaton::{Atom, AutomatonStateKey};
use fsl_eval::model_subject::ModelInstance;
use fsl_eval::temporal::{AtomReading, UndefinedCause};
use fsl_replay::CounterexampleKind;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, OnceLock};

use super::*;
use crate::counter::package_with_activated_clause;

const EACH: &str = "on each (q: M::Counter)";
const EACH_WHEN: &str = "on each (q: M::Counter) when (c.value >= 2)";
const PRIOR_ZERO: &str = "holds(c.value = 1) implies once[1,1] holds(c.value = 0)";
const PRIOR_ZERO_AT_TWO: &str = "holds(c.value = 2) implies once[1,1] holds(c.value = 0)";
const STUTTER_PRIOR: &str = "holds(c.value = 3) implies once[1,1] holds(c.value = 2)";
const OLD_DEADLINE: &str = "holds(c.value <= 1) implies eventually[0,2] holds(c.value = 3)";

/// What one atom of a clause reads from the bound counter's `value`.
type Reading = fn(i64) -> Option<bool>;

/// Letters whose atom `i` (the clause's atom order) reads `reads[i]`, and
/// whose `when` condition, if any, reads `guard`.
struct ValueLetters {
    atoms: Vec<fsl_semantics::check::FormulaNodeId>,
    reads: Vec<Reading>,
    guard: Option<Reading>,
}

impl ValueLetters {
    fn of(clause: &CheckedTemporalClause, reads: Vec<Reading>, guard: Option<Reading>) -> Self {
        let automaton = translate(clause, 64).expect("the clause translates");
        assert_eq!(automaton.atoms().len(), reads.len(), "one reading per atom");
        Self {
            atoms: automaton.atoms().to_vec(),
            reads,
            guard,
        }
    }
}

fn reading(read: Option<bool>) -> AtomReading {
    match read {
        Some(true) => AtomReading::True,
        Some(false) => AtomReading::False,
        None => AtomReading::Undefined(UndefinedCause::Kernel(
            filament_exact::Undefined::DivisionByZero,
        )),
    }
}

impl ModelLetters for ValueLetters {
    fn reads(&self, _atom: fsl_semantics::check::FormulaNodeId) -> crate::units::Reads {
        crate::units::unread()
    }

    fn atom(
        &self,
        atom: fsl_semantics::check::FormulaNodeId,
        instance: &ModelInstance,
        state: &ModelSimState,
        _meter: &mut filament_exact::Meter,
    ) -> Result<AtomReading, fsl_eval::model_subject::AtomStop> {
        let index = self
            .atoms
            .iter()
            .position(|known| *known == atom)
            .expect("the clause's own atom");
        Ok(reading((self.reads[index])(crate::counter::value(
            state,
            &instance.key,
        ))))
    }

    fn activation(
        &self,
        instance: &ModelInstance,
        state: &ModelSimState,
        _meter: &mut filament_exact::Meter,
    ) -> Result<AtomReading, fsl_eval::model_subject::AtomStop> {
        let guard = self
            .guard
            .expect("only a guarded clause reads its condition");
        Ok(reading(guard(crate::counter::value(state, &instance.key))))
    }

    fn activation_reads(&self) -> crate::units::Reads {
        crate::units::unread()
    }
}

fn is(value: i64) -> Reading {
    match value {
        0 => |v| Some(v == 0),
        1 => |v| Some(v == 1),
        2 => |v| Some(v == 2),
        _ => |v| Some(v == 3),
    }
}

/// Two readings, in the clause's atom order.
fn pair(first: Reading, second: Reading) -> Vec<Reading> {
    vec![first, second]
}

fn one_undefined_at_two(v: i64) -> Option<bool> {
    (v != 2).then_some(v == 1)
}

fn at_most_one(v: i64) -> Option<bool> {
    Some(v <= 1)
}

fn at_least_two(v: i64) -> Option<bool> {
    Some(v >= 2)
}

/// The clause `name` over the world's counter under `activation`, read
/// through `reads` and `guard`.
fn claim(
    world: &World,
    activation: &str,
    formula: &str,
    reads: Vec<Reading>,
    guard: Option<Reading>,
) -> Invariant {
    let (package, clause) = package_with_activated_clause(
        &world.documents.document,
        &world.body,
        "C",
        activation,
        formula,
    );
    let letters = ValueLetters::of(&clause, reads, guard);
    world.declared_with(package, clause, Box::new(letters))
}

/// The accepted clause: the activation at value 1 reads value 0 one
/// position back.
fn prior_zero(world: &World, activation: &str, guard: Option<Reading>) -> Invariant {
    claim(world, activation, PRIOR_ZERO, pair(is(1), is(0)), guard)
}

/// The product states of the behaviour `values`, whose last position is the
/// first the clause rejects, keyed as the clause's monitor reaches them: the
/// certificate of a violated clause that holds every state its walk reached
/// before the violation, under the clause's own context.
fn walked_before_violation(
    world: &World,
    claim: &Invariant,
    object: &ObjectReference,
    values: &[i64],
    reads: &[Reading],
    guard: Option<Reading>,
) -> ClosureCertificate {
    let keys = claim.monitor_keys(object);
    let mut monitor = Monitor::bad_prefix(translate(&claim.clause, 64).expect("translates"), 64)
        .expect("a safety automaton");
    let when = monitor.automaton().when();
    let mut state = monitor.initial().expect("initial state");
    let mut states = Vec::new();
    for value in values {
        let mut letter = Letter::default();
        for (index, read) in reads.iter().enumerate() {
            if read(*value) == Some(true) {
                letter = letter.with(Atom::new(index).expect("an atom"));
            }
        }
        if let (Some(when), Some(guard)) = (when, guard) {
            if guard(*value) == Some(true) {
                letter = letter.with(when);
            }
        }
        state = monitor.step(state, letter).expect("steps");
        assert!(
            !monitor.is_rejecting(state).expect("a known state"),
            "the walk rejects only after {values:?}"
        );
        let configurations = monitor
            .configurations(state, keys.max_key_bytes())
            .expect("resolves");
        states.push(ProductStateRef {
            model_state: world.state_key(&[*value]),
            automaton_state: keys.key(&configurations).expect("encodes"),
        });
    }
    states.sort();
    states.dedup();
    ClosureCertificate { states }
}

fn entries(
    certificate: &ClosureCertificate,
    model: &fsl_eval::simulation::StateKey,
) -> Vec<ProductStateRef> {
    certificate
        .states
        .iter()
        .filter(|state| &state.model_state == model)
        .cloned()
        .collect()
}

/// The key `state` carries, as JSON.
fn key_json(state: &ProductStateRef) -> serde_json::Value {
    serde_json::from_slice(state.automaton_state.as_bytes()).expect("engine JSON key")
}

/// FR-338-AC-30: over the Counter subject the unguarded `on each` clause
/// `holds(c.value = 1) implies once[1,1] holds(c.value = 0)` returns `Holds`
/// whose certificate keys every product state with activation `"infinite"` and
/// a `["safety", branches]` body under the clause's own root, which `check_closure`
/// accepts; FR-127 settles it `proved`, `Certified`. The same body `on origin`
/// reads its own root, so no key of one is a key of the other. The guarded
/// variant `when (c.value >= 2)` with the body `holds(c.value = 2) implies
/// once[1,1] holds(c.value = 1)`, whose activations at 2 and 3 read the
/// position before, is accepted under activation `"each-when"`.
#[trace("TC-525", "FR-338-AC-30")]
#[trace("TC-525", "FR-127-AC-1")]
#[test]
fn an_unguarded_on_each_tp3_proof_carries_monitor_keys_the_core_accepts() {
    let world = world(CONTRACT, &[("c", 0)]);
    let each = prior_zero(&world, EACH, None);
    let outcome = each.run(&world);
    let full = certificate(&outcome);
    assert!(!full.states.is_empty());
    let request = each.request(&world, UNLIMITED);
    assert_eq!(check(&request, &full), TemporalCertificateCheck::Accepted);
    assert_eq!(
        settled(&world, &each, &outcome).value,
        TerminalValue::Proved {
            basis: ProofBasis::Exhaustive,
            certification: Certification::Certified,
        }
    );
    for state in &full.states {
        let key = key_json(state);
        assert_eq!(key[1][2], "infinite", "{key}");
        assert_eq!(key[2][0], "safety", "{key}");
    }

    let origin = claim(&world, "on origin", PRIOR_ZERO, pair(is(1), is(0)), None);
    let on_origin = certificate(&origin.run(&world));
    for state in &on_origin.states {
        assert!(
            !full
                .states
                .iter()
                .any(|each| each.automaton_state == state.automaton_state),
            "an `on each` key never equals an `on origin` key of the same body"
        );
    }

    // The guarded form: activation positions are the values 2 and 3 and the
    // terminal stutter; the activation at 2 reads value 1 one position back.
    let guarded = claim(
        &world,
        EACH_WHEN,
        "holds(c.value = 2) implies once[1,1] holds(c.value = 1)",
        pair(is(2), is(1)),
        Some(at_least_two),
    );
    let outcome = guarded.run(&world);
    let full = certificate(&outcome);
    for state in &full.states {
        assert_eq!(key_json(state)[1][2], "each-when");
    }
    assert_eq!(
        check(&guarded.request(&world, UNLIMITED), &full),
        TemporalCertificateCheck::Accepted
    );
    assert_eq!(
        settled(&world, &guarded, &outcome).value,
        TerminalValue::Proved {
            basis: ProofBasis::Exhaustive,
            certification: Certification::Certified,
        }
    );
}

/// FR-338-AC-30: each of three unguarded `on each` clauses is violated, and
/// the certificate keyed under the clause's own context that holds the states
/// its walk reached is rejected `BadState` at the stated model state under the
/// rejecting key `["safety",[]]`:
/// - `holds(c.value = 2) implies once[1,1] holds(c.value = 0)`, at model state
///   2, since the activation at position 2 reads position 1 (value 1);
/// - `holds(c.value = 3) implies once[1,1] holds(c.value = 2)`, at the terminal
///   stutter at model state 3, whose previous position holds 3;
/// - `holds(c.value <= 1) implies eventually[0,2] holds(c.value = 3)`, at model
///   state 2, because the activation at position 0 keeps its deadline 2
///   although the activation at position 1 would be met at position 3.
#[trace("TC-525", "FR-338-AC-30")]
#[test]
fn a_violated_on_each_tp3_clause_is_rejected_bad_state_under_its_own_context() {
    let world = world(CONTRACT, &[("c", 0)]);
    let object = world.objects().remove(0);

    let jump = claim(&world, EACH, PRIOR_ZERO_AT_TWO, pair(is(2), is(0)), None);
    assert!(matches!(
        jump.run(&world),
        ModelCheckOutcome::Violated { .. }
    ));
    let prefix = walked_before_violation(&world, &jump, &object, &[0, 1], &[is(2), is(0)], None);
    assert_eq!(
        check(&jump.request(&world, UNLIMITED), &prefix),
        rejected(
            CertificateRule::BadState,
            jump.rejecting(&world, &object, 2)
        )
    );

    let stutter = claim(&world, EACH, STUTTER_PRIOR, pair(is(3), is(2)), None);
    assert!(matches!(
        stutter.run(&world),
        ModelCheckOutcome::Violated { .. }
    ));
    let prefix = walked_before_violation(
        &world,
        &stutter,
        &object,
        &[0, 1, 2, 3],
        &[is(3), is(2)],
        None,
    );
    assert_eq!(
        check(&stutter.request(&world, UNLIMITED), &prefix),
        rejected(
            CertificateRule::BadState,
            stutter.rejecting(&world, &object, 3)
        )
    );

    let older = claim(&world, EACH, OLD_DEADLINE, pair(at_most_one, is(3)), None);
    assert!(matches!(
        older.run(&world),
        ModelCheckOutcome::Violated { .. }
    ));
    let prefix = walked_before_violation(
        &world,
        &older,
        &object,
        &[0, 1],
        &[at_most_one, is(3)],
        None,
    );
    assert_eq!(
        check(&older.request(&world, UNLIMITED), &prefix),
        rejected(
            CertificateRule::BadState,
            older.rejecting(&world, &object, 2)
        )
    );

    // The accepted clause's certificate is a foreign context to each of
    // them: `ShapeMismatch` at the least refused product state, never
    // `BadState`.
    let accepted = prior_zero(&world, EACH, None);
    let full = certificate(&accepted.run(&world));
    for other in [&jump, &stutter, &older] {
        assert_eq!(
            check(&other.request(&world, UNLIMITED), &full),
            rejected(CertificateRule::ShapeMismatch, least(&full.states))
        );
    }
}

/// FR-338-AC-30: the accepted certificate with one reached product state
/// removed is rejected `SuccessorMissing` at it, and with every key's activation
/// replaced by `"each"`, `ShapeMismatch` at the least refused product state.
#[trace("TC-525", "FR-338-AC-30")]
#[test]
fn a_tampered_on_each_tp3_certificate_is_rejected_at_the_tampered_state() {
    let world = world(CONTRACT, &[("c", 0)]);
    let each = prior_zero(&world, EACH, None);
    let full = certificate(&each.run(&world));
    let request = each.request(&world, UNLIMITED);

    let one = entries(&full, &world.state_key(&[1]));
    assert_eq!(one.len(), 1, "value 1 is reached at one position");
    let mut removed = full.clone();
    removed.states.retain(|state| *state != one[0]);
    assert_eq!(
        check(&request, &removed),
        rejected(CertificateRule::SuccessorMissing, one[0].clone())
    );

    let mut other_activation = full.clone();
    for state in &mut other_activation.states {
        let mut key = key_json(state);
        key[1][2] = serde_json::json!("each");
        state.automaton_state =
            AutomatonStateKey::imported(serde_json::to_vec(&key).expect("array-only key"));
    }
    assert_eq!(
        check(&request, &other_activation),
        rejected(
            CertificateRule::ShapeMismatch,
            least(&other_activation.states)
        )
    );
}

/// FR-338-AC-30, failure domain over the accepted clause: with `max_states`
/// one below the product states its closure holds the check returns
/// `Stopped(LimitReached{MaxStates, ..})`, which FR-127 settles `incomplete`;
/// cancelled with source `Requested` it returns `Stopped(Cancelled{Requested})`,
/// settled `incomplete`; neither is `Rejected` nor `Accepted`. With the atom
/// `c.value = 1` read undefined at model state 2 the engine ends the run with
/// an undefined evaluation and a certificate under the clause's own context
/// is rejected `BadState` at model state 2 under the rejecting key.
#[trace("TC-525", "FR-338-AC-30")]
#[test]
fn an_on_each_tp3_check_stops_or_rejects_an_undefined_atom_with_typed_results() {
    let world = world(CONTRACT, &[("c", 0)]);
    let each = prior_zero(&world, EACH, None);
    let outcome = each.run(&world);
    let full = certificate(&outcome);
    let request = each.request(&world, UNLIMITED);

    let below = u64::try_from(full.states.len() - 1).expect("a small closure");
    let mut limits = ModelCheckLimits::default();
    limits.limits.max_states = usize::try_from(below).expect("a small closure");
    assert_eq!(
        check_closure(&request, &full, &limits, &Cancel::new()).expect("checked"),
        TemporalCertificateCheck::Stopped(IncompleteCause::LimitReached {
            limit: ModelCheckLimit::MaxStates,
            value: below,
            required: None,
        })
    );
    let record = settle_model_check(&outcome, &limits, &request, &Cancel::new(), |_| {
        panic!("a proof replays nothing")
    });
    assert_eq!(
        record.value,
        TerminalValue::Incomplete(IncompleteCause::LimitReached {
            limit: ModelCheckLimit::MaxStates,
            value: below,
            required: None,
        })
    );

    // Cancelled by the first charge the check makes, not before the call.
    let polls = Arc::new(AtomicU32::new(0));
    let handle: Arc<OnceLock<Cancel>> = Arc::new(OnceLock::new());
    let (seen, target) = (Arc::clone(&polls), Arc::clone(&handle));
    let cancel = Cancel::observing(move || {
        if seen.fetch_add(1, Ordering::Relaxed) == 0 {
            if let Some(cancel) = target.get() {
                cancel.cancel(CancelCause::Requested);
            }
        }
    });
    assert!(handle.set(cancel.clone()).is_ok());
    assert_eq!(cancel.cause(), None, "not cancelled before the check");
    let cancelled_record = settle_model_check(
        &outcome,
        &ModelCheckLimits::default(),
        &request,
        &cancel,
        |_| panic!("a proof replays nothing"),
    );
    assert!(polls.load(Ordering::Relaxed) >= 1, "the check polled");
    assert_eq!(cancel.cause(), Some(CancelCause::Requested));
    assert_eq!(
        cancelled_record.value,
        TerminalValue::Incomplete(IncompleteCause::Cancelled {
            source: CancelCause::Requested
        })
    );

    let object = world.objects().remove(0);
    let undefined = claim(
        &world,
        EACH,
        PRIOR_ZERO,
        pair(one_undefined_at_two, is(0)),
        None,
    );
    let ModelCheckOutcome::Violated { counterexample, .. } = undefined.run(&world) else {
        panic!("an undefined atom ends the run");
    };
    let CounterexampleKind::UndefinedEvaluation(evaluation) = &counterexample.kind else {
        panic!("an undefined atom ends the run");
    };
    assert_eq!(evaluation.site.position, 2);
    let prefix = walked_before_violation(
        &world,
        &undefined,
        &object,
        &[0, 1],
        &[one_undefined_at_two, is(0)],
        None,
    );
    assert_eq!(
        check(&undefined.request(&world, UNLIMITED), &prefix),
        rejected(
            CertificateRule::BadState,
            undefined.rejecting(&world, &object, 2)
        )
    );
}

/// FR-1030-AC-11 (clause-root half) for an `on each` clause: a request whose
/// clause root the request's package does not declare has no root. The core
/// refuses it with the unresolved-root refusal, it never rejects a
/// certificate, and the settlement is `Failed`, not `Certified` and not
/// `Inconclusive`; the accepted `on each` certificate alone is no proof of it.
#[trace("TC-2920", "FR-1030-AC-11")]
#[test]
fn an_on_each_clause_the_package_does_not_declare_settles_failed() {
    let world = world(CONTRACT, &[("c", 0)]);
    let each = prior_zero(&world, EACH, None);
    let outcome = each.run(&world);
    let full = certificate(&outcome);
    assert_eq!(
        check(&each.request(&world, UNLIMITED), &full),
        TemporalCertificateCheck::Accepted
    );
    // The same clause under another name is declared by a package of its
    // own, not by the request's.
    let (_, other) = package_with_activated_clause(
        &world.documents.document,
        &world.body,
        "Other",
        EACH,
        PRIOR_ZERO,
    );
    let foreign = ModelCheckItem::Clause {
        clause: &other,
        letters: &*each.letters,
    };
    let request = each.request_with(&world, foreign, UNLIMITED);
    match check_closure(
        &request,
        &full,
        &ModelCheckLimits::default(),
        &Cancel::new(),
    ) {
        Err(ClosureRefusal::Item(ItemRefusal::ClauseRoot(ClauseRootRefusal::Undeclared {
            name,
        }))) => assert_eq!(name, "Other"),
        other => panic!("expected the clause-root-unresolved refusal, got {other:?}"),
    }
    let record = settle_model_check(
        &outcome,
        &ModelCheckLimits::default(),
        &request,
        &Cancel::new(),
        |_| panic!("a proof replays nothing"),
    );
    assert_eq!(record.value, TerminalValue::Failed);
}
