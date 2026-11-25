// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-169 (ADR-022 GC-1 to GC-4): the certificate EN-1 returns with a
//! `Witnessed` or `Holds` state-graph outcome from an unreduced run, and
//! [`check_state_graph`], the core's checker of it.
//!
//! The checker reads nothing from the engine but the certificate. It
//! re-admits the subject through `ModelSystem` (FR-120), re-derives the
//! instance's `over` binding from the subject's universes, recomputes every
//! successor and evaluates every claim predicate with `fsl-eval`'s code. It
//! applies its rules in this order and rejects at the first failing state in
//! certificate order (ascending state key):
//!
//! 1. `Malformed`, before any recomputation: a closure entry is not a
//!    `filament.simulation.state-key/v1` encoding or is not strictly greater
//!    than the one before it, or a rank or count entry is unsorted,
//!    duplicated, names no closure state, counts above 2 or belongs to the
//!    list the claim's form does not carry;
//! 2. `InitialMissing`: an initial state is not in the closure;
//! 3. `SuccessorMissing`: a closure state the checker reached has a successor
//!    outside the closure; `BadState`: its expansion evaluated a contract
//!    conjunction undefined or refused, so its successors are not known, or
//!    it has no successor and FR-124 classifies it undefined or undecided,
//!    its `terminal when` predicates having no value there;
//! 4. `PredicateUndefined`: a claim predicate is undefined or refused at a
//!    reached state;
//! 5. by form: `possible` and `always possible` check ranks (`RankMissing`,
//!    `RankBroken`); `unique path` checks `H`'s order and counts
//!    (`OrderBroken`, `CountBroken`).
//!
//! Only states reached from the initial states are checked; an unreached
//! closure entry is never used, so it cannot help a false claim pass. The
//! reached states count against `max_states` and the recomputed successors
//! against `max_transitions`; `Malformed` reads the certificate once,
//! linearly, before any of that.
//!
//! **Counts.** A reached state with no count entry counts 0, so the states
//! with count 0 are closed under successors and hold no `to` node: none
//! reaches `to`. A state with count 0 or 1 that is no `to` node must have
//! every edge to a counted state go to an earlier state in the order, so its
//! count is well founded and exact; a count of 2 needs no order, since it
//! only refutes a `from` node and so is never trusted for acceptance. That
//! lets a cycle of `H` that no `from` node reaches be certified.

use ix_trace_rs::implements;
use std::collections::{BTreeMap, BTreeSet, VecDeque};

use filament_exact::{Cancel, ObjectReference, ValueType};
use fsl_eval::model_subject::ModelSubject;
use fsl_eval::simulation::model::{ModelSimState, ModelSystem, PredicateReading, TerminalClass};
use fsl_eval::simulation::{StateKey, TransitionSystem};
use fsl_semantics::check::{CheckedStateGraphClaim, PredicateRole, StateGraphForm};

use crate::certificate::check::{
    admit, expansion_halt, finish, poll, stopped, widen, Budgets, Checker, Halt, Step,
};
use crate::certificate::closure::{CertificateRejection, ClosureRefusal, TemporalCertificateCheck};
use crate::certificate::{CertificateLocus, CertificateRule};
use crate::model_check::limits::{ModelCheckLimit, ModelCheckLimits};
use crate::probability_certificate::is_model_state_key;
use crate::proof_result::IncompleteCause;

/// FR-169's state-graph certificate of one item instance.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct StateGraphCertificate {
    /// Every explored model state's full canonical key, strictly ascending.
    pub closure: Vec<StateKey>,
    /// `possible` and `always possible`: `(closure index, d)`, the state's
    /// distance to the nearest target state, for every state that reaches
    /// one; ascending by index.
    pub ranks: Vec<(u64, u32)>,
    /// `unique path`: `(closure index, count, order)` for every node of `H`:
    /// its path count to `to`, saturated at 2, and its position in a reverse
    /// topological order of `H`; ascending by index.
    pub counts: Vec<(u64, u8, u64)>,
}

/// What [`check_state_graph`] checks: the model subject, re-admitted by the
/// checker, the claim and the instance's `over` binding.
pub struct StateGraphCertificateRequest<'a> {
    /// The model subject (FR-125).
    pub subject: ModelSubject<'a>,
    /// The checked claim.
    pub claim: &'a CheckedStateGraphClaim,
    /// The object the instance binds the claim's `over` parameter to; `None`
    /// for a claim with no `over`.
    pub over: Option<ObjectReference>,
}

/// FR-169: check `certificate` for `request`'s claim instance under
/// `limits` and the subject's own evaluation meter and candidate bound,
/// polling `cancel` at every state.
///
/// # Errors
///
/// [`ClosureRefusal`] when the subject does not admit, a root is unbounded,
/// the `over` binding is no instance of the subject, the subject has no
/// initial state, or a state cannot be encoded. A refusal is never an
/// acceptance.
pub fn check_state_graph(
    request: &StateGraphCertificateRequest<'_>,
    certificate: &StateGraphCertificate,
    limits: &ModelCheckLimits,
    cancel: &Cancel,
) -> Result<TemporalCertificateCheck, ClosureRefusal> {
    let shape = malformed(certificate, Carried::of(request.claim.form()), cancel);
    if shape.is_err() {
        return finish(shape);
    }
    let simulation = request.subject.simulation.clone();
    let budgets = Budgets::of(&simulation);
    let system = admit(simulation)?;
    // FR-125 Instancing orders the refusals: an unbounded root (`admit`, and
    // `bound_over`'s instancing), then no initial state, then no instance.
    let over = match bound_over(request, &system) {
        Ok(_) | Err(ClosureRefusal::NoInstance) if system.initial().is_empty() => {
            return Err(ClosureRefusal::NoInitialState);
        }
        over => over?,
    };
    let predicates = predicates(request.claim.form());
    let check = Check {
        system: &system,
        certificate,
        limits,
        budgets,
        cancel,
        over: over.as_ref(),
        claim: request.claim,
        predicates: &predicates,
    };
    finish(check.run(request.claim.form()))
}

/// The request's `over` binding, when it is one of the subject's instances
/// of the claim's `over` type (FR-125).
fn bound_over(
    request: &StateGraphCertificateRequest<'_>,
    system: &ModelSystem<'_>,
) -> Result<Option<ObjectReference>, ClosureRefusal> {
    match (request.claim.over(), &request.over) {
        (None, None) => Ok(None),
        (Some(parameter), Some(reference)) => {
            let instances = system
                .instances(&ValueType::Reference(parameter.object.clone()))
                .map_err(|refusal| ClosureRefusal::NotSimulated(Box::new(refusal)))?;
            for instance in &instances {
                let bound = system
                    .written_reference(parameter.object.clone(), instance)
                    .map_err(ClosureRefusal::Fault)?;
                if bound == *reference {
                    return Ok(Some(bound));
                }
            }
            Err(ClosureRefusal::NoInstance)
        }
        (None, Some(_)) | (Some(_), None) => Err(ClosureRefusal::NoInstance),
    }
}

/// The roles of the claim's written predicates; an omitted `from` holds
/// everywhere and has none. Each is read through the evaluator port, keyed
/// by the claim's occurrence and the role.
fn predicates(form: &StateGraphForm) -> Vec<PredicateRole> {
    let all = match form {
        StateGraphForm::Possible { target } => vec![(PredicateRole::Target, target)],
        StateGraphForm::AlwaysPossible { from, target } => {
            vec![(PredicateRole::From, from), (PredicateRole::Target, target)]
        }
        StateGraphForm::UniquePath { from, to } => {
            vec![(PredicateRole::From, from), (PredicateRole::Target, to)]
        }
    };
    all.into_iter()
        .filter(|(_, predicate)| predicate.is_written())
        .map(|(role, _)| role)
        .collect()
}

/// The one entry list a certificate carries, by its claim's form.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Carried {
    /// `possible` and `always possible`.
    Ranks,
    /// `unique path`.
    Counts,
}

impl Carried {
    const fn of(form: &StateGraphForm) -> Self {
        match form {
            StateGraphForm::Possible { .. } | StateGraphForm::AlwaysPossible { .. } => Self::Ranks,
            StateGraphForm::UniquePath { .. } => Self::Counts,
        }
    }
}

/// Stop at the first `Malformed` entry, reading the certificate once: closure
/// domain and order, then ranks, then counts, polling `cancel` at every
/// closure state. Every entry of the list not `carried` is malformed.
fn malformed(certificate: &StateGraphCertificate, carried: Carried, cancel: &Cancel) -> Step<()> {
    let mut previous: Option<&StateKey> = None;
    for state in &certificate.closure {
        poll(cancel)?;
        let encoded = is_model_state_key(state.as_bytes()).map_err(|_| {
            Halt::Check(TemporalCertificateCheck::Stopped(
                IncompleteCause::ResourceExhausted,
            ))
        })?;
        let ordered = previous.is_none_or(|before| before < state);
        if !encoded || !ordered {
            return Err(reject(CertificateRule::Malformed, state.clone()));
        }
        previous = Some(state);
    }
    let entry = |index: u64| match usize::try_from(index)
        .ok()
        .and_then(|at| certificate.closure.get(at))
    {
        Some(state) => CertificateLocus::ModelKey(state.clone()),
        None => CertificateLocus::Entry { index },
    };
    let indices = certificate
        .ranks
        .iter()
        .map(|(index, _)| (*index, false))
        .collect::<Vec<_>>();
    let counted = certificate
        .counts
        .iter()
        .map(|(index, count, _)| (*index, *count > 2))
        .collect::<Vec<_>>();
    for (list, section) in [(Carried::Ranks, indices), (Carried::Counts, counted)] {
        let mut previous: Option<u64> = None;
        for (index, over) in section {
            let in_range = usize::try_from(index).is_ok_and(|at| at < certificate.closure.len());
            let ordered = previous.is_none_or(|before| before < index);
            if list != carried || !in_range || !ordered || over {
                return Err(Halt::Check(TemporalCertificateCheck::Rejected(
                    CertificateRejection {
                        rule: CertificateRule::Malformed,
                        at: entry(index),
                    },
                )));
            }
            previous = Some(index);
        }
    }
    Ok(())
}

fn reject(rule: CertificateRule, at: StateKey) -> Halt {
    Halt::Check(TemporalCertificateCheck::Rejected(CertificateRejection {
        rule,
        at: CertificateLocus::ModelKey(at),
    }))
}

/// A reached closure state: the successor of each of its edges, in
/// expansion order, one entry per edge.
struct Reached {
    state: ModelSimState,
    successors: Vec<StateKey>,
    /// Each written predicate's truth there.
    holds: Vec<(PredicateRole, bool)>,
}

impl Reached {
    /// Whether `role` holds; an unwritten predicate holds everywhere.
    fn holds(&self, role: PredicateRole) -> bool {
        self.holds
            .iter()
            .find(|(written, _)| *written == role)
            .is_none_or(|(_, holds)| *holds)
    }
}

struct Check<'c, 'a> {
    system: &'c ModelSystem<'a>,
    certificate: &'c StateGraphCertificate,
    limits: &'c ModelCheckLimits,
    budgets: Budgets,
    cancel: &'c Cancel,
    over: Option<&'c ObjectReference>,
    claim: &'c CheckedStateGraphClaim,
    predicates: &'c [PredicateRole],
}

impl Check<'_, '_> {
    fn poll(&self) -> Step<()> {
        poll(self.cancel)
    }

    fn halt(&self, stop: &fsl_eval::simulation::ExpansionStop) -> Halt {
        expansion_halt(stop, self.budgets, Checker::StateGraph)
    }

    fn member(&self, state: &StateKey) -> bool {
        self.certificate.closure.binary_search(state).is_ok()
    }

    /// The closure state an entry's index names; `malformed` admitted it.
    fn state_at(&self, index: u64) -> Step<StateKey> {
        usize::try_from(index)
            .ok()
            .and_then(|at| self.certificate.closure.get(at))
            .cloned()
            .ok_or_else(|| {
                Checker::StateGraph
                    .fault("admitted-entry-names-a-closure-state")
                    .into()
            })
    }

    fn run(&self, form: &StateGraphForm) -> Step<()> {
        self.poll()?;
        let mut initial = BTreeMap::new();
        for state in self.system.initial() {
            let (bytes, _) = self.system.state_key(&state)?;
            let key = StateKey::from_canonical_bytes(bytes);
            initial.entry(key).or_insert(state);
        }
        if let Some(missing) = initial.keys().find(|state_key| !self.member(state_key)) {
            return Err(reject(CertificateRule::InitialMissing, (*missing).clone()));
        }
        let mut reached = self.explore(&initial)?;
        self.read_predicates(reached.iter_mut())?;
        match form {
            StateGraphForm::Possible { .. } => {
                let ranks = self.ranks()?;
                if let Some(missing) = initial
                    .keys()
                    .find(|state_key| !ranks.contains_key(state_key))
                {
                    return Err(reject(CertificateRule::RankMissing, missing.clone()));
                }
                self.check_ranks(&reached, &ranks)
            }
            StateGraphForm::AlwaysPossible { .. } => {
                let ranks = self.ranks()?;
                if let Some((missing, _)) = reached.iter().find(|(state_key, state)| {
                    state.holds(PredicateRole::From) && !ranks.contains_key(state_key)
                }) {
                    return Err(reject(CertificateRule::RankMissing, missing.clone()));
                }
                self.check_ranks(&reached, &ranks)
            }
            StateGraphForm::UniquePath { .. } => self.check_counts(&reached),
        }
    }

    /// Every closure state reachable from `initial`, expanded, checking
    /// `SuccessorMissing` then `BadState` at the first failing state in
    /// certificate order.
    #[implements("FR-169-AC-11", "FR-169-AC-18")]
    fn explore(
        &self,
        initial: &BTreeMap<StateKey, ModelSimState>,
    ) -> Step<BTreeMap<StateKey, Reached>> {
        let mut reached: BTreeMap<StateKey, Reached> = BTreeMap::new();
        let mut queue = VecDeque::new();
        let mut leaves = BTreeSet::new();
        let mut bad = BTreeSet::new();
        let mut edges: usize = 0;
        for (state_key, state) in initial {
            self.admit(&reached)?;
            reached.insert(
                state_key.clone(),
                Reached {
                    state: state.clone(),
                    successors: Vec::new(),
                    holds: Vec::new(),
                },
            );
            queue.push_back(state_key.clone());
        }
        while let Some(at) = queue.pop_front() {
            self.poll()?;
            let state = reached
                .get(&at)
                .map(|entry| entry.state.clone())
                .ok_or_else(|| Checker::StateGraph.fault("queued-state-is-reached"))?;
            let expansion = self
                .system
                .successors(&state)
                .map_err(|stop| self.halt(&stop))?;
            if expansion.findings.iter().any(|finding| {
                finding.undefined_contract().is_some() || finding.is_refused_contract()
            }) {
                bad.insert(at.clone());
            } else if expansion.successors.is_empty() {
                // FR-124: `terminal when` is read where no transition is
                // enabled, and must have a value there.
                let class = self
                    .system
                    .terminal_class(&state, false)
                    .map_err(|stop| self.halt(&stop))?;
                if matches!(
                    class,
                    TerminalClass::Undefined { .. } | TerminalClass::Undecided { .. }
                ) {
                    bad.insert(at.clone());
                }
            }
            let mut successors = Vec::with_capacity(expansion.successors.len());
            for (_, next) in expansion.successors {
                if edges >= self.limits.limits.max_transitions {
                    return Err(stopped(
                        ModelCheckLimit::MaxTransitions,
                        widen(self.limits.limits.max_transitions),
                    ));
                }
                edges = edges.saturating_add(1);
                let (bytes, _) = self.system.state_key(&next)?;
                let key = StateKey::from_canonical_bytes(bytes);
                successors.push(key.clone());
                if !self.member(&key) {
                    leaves.insert(at.clone());
                    continue;
                }
                if !reached.contains_key(&key) {
                    self.admit(&reached)?;
                    reached.insert(
                        key.clone(),
                        Reached {
                            state: next,
                            successors: Vec::new(),
                            holds: Vec::new(),
                        },
                    );
                    queue.push_back(key);
                }
            }
            if let Some(entry) = reached.get_mut(&at) {
                entry.successors = successors;
            }
        }
        if let Some(first) = leaves.first() {
            return Err(reject(CertificateRule::SuccessorMissing, first.clone()));
        }
        if let Some(first) = bad.first() {
            return Err(reject(CertificateRule::BadState, first.clone()));
        }
        Ok(reached)
    }

    /// Charge one more reached state against `max_states`.
    fn admit(&self, reached: &BTreeMap<StateKey, Reached>) -> Step<()> {
        if reached.len() >= self.limits.limits.max_states {
            return Err(stopped(
                ModelCheckLimit::MaxStates,
                widen(self.limits.limits.max_states),
            ));
        }
        Ok(())
    }

    /// Read every written predicate at every reached state, in certificate
    /// order: `PredicateUndefined` at the first where one has no value.
    fn read_predicates<'r>(
        &self,
        states: impl Iterator<Item = (&'r StateKey, &'r mut Reached)>,
    ) -> Step<()> {
        let bodies: Vec<_> = self
            .predicates
            .iter()
            .map(|role| (self.claim, *role, self.over))
            .collect();
        for (state_key, entry) in states {
            self.poll()?;
            let readings = self
                .system
                .state_graph_predicates(&entry.state, &bodies)
                .map_err(|stop| self.halt(&stop))?;
            let mut holds = Vec::with_capacity(readings.len());
            for (role, reading) in self.predicates.iter().zip(readings) {
                match reading {
                    PredicateReading::True => holds.push((*role, true)),
                    PredicateReading::False => holds.push((*role, false)),
                    PredicateReading::Undefined { .. } | PredicateReading::Refused(_) => {
                        return Err(reject(
                            CertificateRule::PredicateUndefined,
                            state_key.clone(),
                        ));
                    }
                }
            }
            entry.holds = holds;
        }
        Ok(())
    }

    fn ranks(&self) -> Step<BTreeMap<StateKey, u32>> {
        self.certificate
            .ranks
            .iter()
            .map(|(index, d)| Ok((self.state_at(*index)?, *d)))
            .collect()
    }

    /// `RankBroken`: a reached ranked state with `d = 0` where the target
    /// fails, or with `d > 0` and no successor ranked `d - 1`.
    fn check_ranks(
        &self,
        reached: &BTreeMap<StateKey, Reached>,
        ranks: &BTreeMap<StateKey, u32>,
    ) -> Step<()> {
        for (state_key, state) in reached {
            let Some(d) = ranks.get(state_key) else {
                continue;
            };
            let holds = match d.checked_sub(1) {
                None => state.holds(PredicateRole::Target),
                Some(next) => state
                    .successors
                    .iter()
                    .any(|successor| ranks.get(successor) == Some(&next)),
            };
            if !holds {
                return Err(reject(CertificateRule::RankBroken, state_key.clone()));
            }
        }
        Ok(())
    }

    /// `OrderBroken`, then `CountBroken`, over the reached states in
    /// certificate order (module docs, "Counts").
    fn check_counts(&self, reached: &BTreeMap<StateKey, Reached>) -> Step<()> {
        let counts: BTreeMap<StateKey, (u8, u64)> = self
            .certificate
            .counts
            .iter()
            .map(|(index, count, order)| Ok((self.state_at(*index)?, (*count, *order))))
            .collect::<Step<_>>()?;
        for (state_key, state) in reached {
            let Some((count, order)) = counts.get(state_key) else {
                continue;
            };
            if *count >= 2 || state.holds(PredicateRole::Target) {
                continue;
            }
            let backward = state.successors.iter().any(|successor| {
                counts
                    .get(successor)
                    .is_some_and(|(_, later)| later >= order)
            });
            if backward {
                return Err(reject(CertificateRule::OrderBroken, state_key.clone()));
            }
        }
        for (state_key, state) in reached {
            let count = counts.get(state_key).map_or(0, |(count, _)| *count);
            let expected = if state.holds(PredicateRole::Target) {
                1
            } else {
                state
                    .successors
                    .iter()
                    .map(|successor| counts.get(successor).map_or(0, |(count, _)| *count))
                    .fold(0u8, |sum, count| sum.saturating_add(count).min(2))
            };
            if count != expected || (state.holds(PredicateRole::From) && count != 1) {
                return Err(reject(CertificateRule::CountBroken, state_key.clone()));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use ix_trace_rs::trace;

    use super::*;

    /// A state-key envelope ordered by `byte`, its first member.
    fn key(byte: u8) -> StateKey {
        StateKey::from_canonical_bytes(
            format!(
                "{{\"bounds\":{byte},\"control\":0,\"memory\":0,\"observations\":0,\
                 \"queues\":0,\"roles\":0,\"semantic\":0,\"type\":\"simulation-state\"}}"
            )
            .into_bytes(),
        )
    }

    /// The `Malformed` rule and locus `certificate` is rejected at, as a
    /// certificate carrying the `carried` list.
    fn at(
        carried: Carried,
        certificate: StateGraphCertificate,
    ) -> Option<(CertificateRule, CertificateLocus)> {
        match finish(malformed(&certificate, carried, &Cancel::new())) {
            Ok(TemporalCertificateCheck::Accepted) => None,
            Ok(TemporalCertificateCheck::Rejected(rejection)) => {
                Some((rejection.rule, rejection.at))
            }
            other => panic!("the shape check neither stops nor refuses: {other:?}"),
        }
    }

    fn malformed_at(locus: CertificateLocus) -> Option<(CertificateRule, CertificateLocus)> {
        Some((CertificateRule::Malformed, locus))
    }

    /// FR-169's `Malformed` rule, applied before any recomputation: an
    /// unsorted or duplicated closure, a rank naming no closure state, an
    /// unsorted rank and a count above 2 each reject at their entry.
    #[trace("TC-644")]
    #[test]
    fn malformed_entries_reject_before_recomputation() {
        let closure = vec![key(1), key(2)];
        assert_eq!(
            at(
                Carried::Ranks,
                StateGraphCertificate {
                    closure: vec![key(2), key(1)],
                    ..StateGraphCertificate::default()
                }
            ),
            malformed_at(CertificateLocus::ModelKey(key(1)))
        );
        assert_eq!(
            at(
                Carried::Ranks,
                StateGraphCertificate {
                    closure: vec![key(1), key(1)],
                    ..StateGraphCertificate::default()
                }
            ),
            malformed_at(CertificateLocus::ModelKey(key(1)))
        );
        assert_eq!(
            at(
                Carried::Ranks,
                StateGraphCertificate {
                    closure: closure.clone(),
                    ranks: vec![(0, 1), (2, 0)],
                    counts: Vec::new(),
                }
            ),
            malformed_at(CertificateLocus::Entry { index: 2 })
        );
        assert_eq!(
            at(
                Carried::Ranks,
                StateGraphCertificate {
                    closure: closure.clone(),
                    ranks: vec![(1, 0), (0, 1)],
                    counts: Vec::new(),
                }
            ),
            malformed_at(CertificateLocus::ModelKey(key(1)))
        );
        assert_eq!(
            at(
                Carried::Counts,
                StateGraphCertificate {
                    closure: closure.clone(),
                    ranks: Vec::new(),
                    counts: vec![(1, 3, 0)],
                }
            ),
            malformed_at(CertificateLocus::ModelKey(key(2)))
        );
        assert_eq!(
            at(
                Carried::Ranks,
                StateGraphCertificate {
                    closure: closure.clone(),
                    ranks: vec![(0, 1), (1, 0)],
                    counts: Vec::new(),
                }
            ),
            None
        );
        assert_eq!(
            at(
                Carried::Counts,
                StateGraphCertificate {
                    closure,
                    ranks: Vec::new(),
                    counts: vec![(0, 1, 1), (1, 2, 0)],
                }
            ),
            None
        );
    }

    /// FR-169-AC-15: an entry of the list the claim's form does not carry
    /// rejects at the state its index names, after the carried list's
    /// entries and, for `unique path`, before the counts.
    #[trace("TC-644", "FR-169-AC-15")]
    #[test]
    fn the_other_forms_list_is_malformed_in_stored_order() {
        let both = || StateGraphCertificate {
            closure: vec![key(1), key(2)],
            ranks: vec![(1, 0)],
            counts: vec![(0, 1, 1)],
        };
        assert_eq!(
            at(Carried::Ranks, both()),
            malformed_at(CertificateLocus::ModelKey(key(1)))
        );
        assert_eq!(
            at(Carried::Counts, both()),
            malformed_at(CertificateLocus::ModelKey(key(2)))
        );
        assert_eq!(
            at(
                Carried::Ranks,
                StateGraphCertificate {
                    ranks: vec![(2, 0)],
                    ..both()
                }
            ),
            malformed_at(CertificateLocus::Entry { index: 2 })
        );
    }

    /// FR-169-AC-14: a closure entry that is not a state-key encoding
    /// rejects at that entry, whatever its order: bytes that are no JSON,
    /// JSON that is no model state, and a model state not in RFC 8785 text.
    #[trace("TC-1197", "FR-169-AC-14")]
    #[test]
    fn a_closure_entry_that_is_no_state_key_is_malformed() {
        let spaced = [b" ".as_slice(), key(2).as_bytes()].concat();
        for bytes in [b"not-a-state-key".to_vec(), b"2".to_vec(), spaced] {
            let entry = StateKey::from_canonical_bytes(bytes);
            let certificate = StateGraphCertificate {
                closure: vec![entry.clone(), key(3)],
                ..StateGraphCertificate::default()
            };
            assert_eq!(
                at(Carried::Ranks, certificate),
                malformed_at(CertificateLocus::ModelKey(entry))
            );
        }
    }
}
