// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(
    dead_code,
    reason = "shared liveness fixture helpers span split integration test binaries"
)]
//! FR-126's liveness phase (TP-4) and its weak-fairness filter over the
//! `Counter` subject (`crate::counter`) and its `setTo` variant: the
//! product's SCC phase, the fairness filter, the canonical lasso, the
//! terminal stutter loop, the limits and the refusals of what is not built.
//!
//! Every expected loop below is derived by hand from the model's arithmetic
//! and FR-101's canonical order (see each test), and every lasso is also
//! checked by an oracle written here that reads only the model's successor
//! relation: the loop closes, the claim is false on it, and each weak
//! constraint is taken in it or disabled somewhere in it.
//!
//! ADR-018 §6's ConfigVersion units and FR-126-AC-4's `Health` unit are
//! `example_units`. Not covered here: the `holds` atoms are read through the
//! test's own [`Letters`]; the lasso's replay is FR-128's.

use filament_exact::{Cancel, ScalarLimits};
use fsl_analyze::model_check::{check_model, ModelCheckRefusal, ModelCheckRequest};
use fsl_eval::model_subject::ModelInstance;
use fsl_eval::simulation::model::{ModelSimState, ModelSystem};
use fsl_eval::simulation::TransitionSystem;
use fsl_package::CheckedPackage;
use fsl_replay::{
    check_components, CertificateRequest, ComponentCertificate, Lasso, ModelCheckItem,
    ModelCheckLimits, ModelCheckOutcome, ProofCertificate, SmtEncodingLimits,
    TemporalCertificateCheck, TemporalCounterexample,
};
use fsl_semantics::check::CheckedTemporalClause;
use fsl_semantics::model::observation::DocumentRef;

use crate::counter::{
    clause_text, key, subject, unit_text, value, Documents, Letters, CONTRACT, UNLIMITED,
};

/// A Counter world with one declared clause.
pub(crate) struct Fixture {
    pub(crate) package: CheckedPackage,
    pub(crate) temporal_clock_selections: Vec<fsl_package::TemporalClockSelection>,
    pub(crate) clauses: Vec<CheckedTemporalClause>,
    pub(crate) documents: Documents,
    pub(crate) initial: Vec<DocumentRef>,
    pub(crate) keys: Vec<&'static str>,
    /// The native unit the package was checked from.
    pub(crate) unit: Vec<u8>,
}

/// The world over `document` with `body`, one start with the counters
/// `starts`, and the clause `fairness` + `formula` over `c`.
pub(crate) fn fixture(
    document: Vec<u8>,
    body: &str,
    fairness: &str,
    formula: &str,
    starts: &[(&'static str, i64)],
) -> Fixture {
    fixture_in_mode(
        document,
        body,
        fairness,
        formula,
        starts,
        filament_semantic_value::checking::CheckMode::Linked,
    )
}

/// A fixture with an explicit function-body mode; declared clauses stay Linked.
pub(crate) fn fixture_in_mode(
    document: Vec<u8>,
    body: &str,
    fairness: &str,
    formula: &str,
    starts: &[(&'static str, i64)],
    mode: filament_semantic_value::checking::CheckMode,
) -> Fixture {
    let (package, clauses) = crate::counter::package_with_clauses_mode(
        &document,
        body,
        &[("C", fairness, formula)],
        mode,
    );
    let temporal_clock_selections = crate::fill::tcb_01_event_position_selection(&package);
    let unit = unit_text(&(body.to_owned() + &clause_text("C", fairness, formula))).into_bytes();
    let mut documents = Documents::new(document);
    let initial = vec![documents.start("s0", starts)];
    Fixture {
        package,
        temporal_clock_selections,
        clauses,
        documents,
        initial,
        keys: starts.iter().map(|(key, _)| *key).collect(),
        unit,
    }
}

/// The one-counter fixture over `document`, started at `c = 0`.
pub(crate) fn counter(document: Vec<u8>, fairness: &str, formula: &str) -> Fixture {
    fixture(document, CONTRACT, fairness, formula, &[("c", 0)])
}

impl Fixture {
    pub(crate) fn letters(&self, atom: fn(i64) -> Option<bool>) -> Letters {
        Letters { atom }
    }

    pub(crate) fn run(
        &self,
        atom: fn(i64) -> Option<bool>,
        max_depth: Option<usize>,
        limits: ModelCheckLimits,
        meter: ScalarLimits,
        cancel: &Cancel,
    ) -> Result<ModelCheckOutcome, ModelCheckRefusal> {
        let letters = self.letters(atom);
        check_model(
            ModelCheckRequest {
                workers: 1,
                temporal_clock_selections: &self.temporal_clock_selections,
                subject: subject(
                    &self.package,
                    &self.documents,
                    self.initial.clone(),
                    &self.keys,
                    meter,
                ),
                item: ModelCheckItem::Clause {
                    clause: &self.clauses[0],
                    letters: &letters,
                },
                max_depth,
                limits,
                smt_encoding: Default::default(),
                smt_run: Default::default(),
                reductions: Default::default(),
            },
            cancel,
        )
    }

    pub(crate) fn decide(
        &self,
        atom: fn(i64) -> Option<bool>,
    ) -> Result<ModelCheckOutcome, ModelCheckRefusal> {
        self.run(
            atom,
            None,
            ModelCheckLimits::default(),
            UNLIMITED,
            &Cancel::new(),
        )
    }

    /// The request FR-339's checker reads this fixture's clause under.
    pub(crate) fn certificate_request<'a>(
        &'a self,
        letters: &'a Letters,
        meter: ScalarLimits,
    ) -> CertificateRequest<'a> {
        CertificateRequest {
            temporal_clock_selections: &self.temporal_clock_selections,
            subject: subject(
                &self.package,
                &self.documents,
                self.initial.clone(),
                &self.keys,
                meter,
            ),
            item: ModelCheckItem::Clause {
                clause: &self.clauses[0],
                letters,
            },
            smt_encoding: SmtEncodingLimits::default(),
        }
    }

    /// The component certificate of a proved run, accepted by the core's
    /// checker, and the run's counts.
    pub(crate) fn certified(
        &self,
        atom: fn(i64) -> Option<bool>,
    ) -> (ComponentCertificate, fsl_replay::RunStats) {
        let outcome = self.decide(atom).expect("the clause is decided");
        let ModelCheckOutcome::Holds {
            certificate: ProofCertificate::Component(certificate),
            stats,
        } = outcome
        else {
            panic!("expected a component certificate: {outcome:?}");
        };
        let letters = self.letters(atom);
        let checked = check_components(
            &self.certificate_request(&letters, UNLIMITED),
            &certificate,
            &ModelCheckLimits::default(),
            &Cancel::new(),
        )
        .expect("the request is checked");
        assert_eq!(checked, TemporalCertificateCheck::Accepted);
        (certificate, stats)
    }

    fn system(&self) -> ModelSystem<'_> {
        ModelSystem::new(
            subject(
                &self.package,
                &self.documents,
                self.initial.clone(),
                &self.keys,
                UNLIMITED,
            )
            .simulation,
        )
        .expect("the subject admits")
    }

    /// The model states a counterexample's steps reach, position by
    /// position: each step's post-state is the successor of its transition
    /// identity whose state key it records.
    fn positions(&self, counterexample: &TemporalCounterexample) -> Vec<ModelSimState> {
        let system = self.system();
        let mut current = system.initial()[counterexample.initial_state].clone();
        let mut states = vec![current.clone()];
        for step in &counterexample.prefix {
            let expansion = system.successors(&current).expect("the state expands");
            current = expansion
                .successors
                .into_iter()
                .find(|(transition, state)| {
                    *transition == step.transition
                        && system.state_key(state).expect("encodes").0.as_slice()
                            == step.post_state.as_bytes()
                })
                .map(|(_, state)| state)
                .expect("the recorded step is a successor");
            states.push(current.clone());
        }
        states
    }

    pub(crate) fn values(&self, counterexample: &TemporalCounterexample, object: &str) -> Vec<i64> {
        self.positions(counterexample)
            .iter()
            .map(|state| value(state, object))
            .collect()
    }
}

/// The instance of the clause that binds the counter `object`.
fn instance(object: &str) -> ModelInstance {
    ModelInstance {
        population: key("counters"),
        key: object.to_owned(),
    }
}

fn violated(outcome: &ModelCheckOutcome) -> &TemporalCounterexample {
    match outcome {
        ModelCheckOutcome::Violated { counterexample, .. } => counterexample,
        other => panic!("expected a violation, got {other:?}"),
    }
}

fn lasso_of(counterexample: &TemporalCounterexample) -> Lasso {
    counterexample
        .lasso
        .as_ref()
        .expect("a liveness refutation is a lasso")
        .clone()
}

/// Whether the transition is an application of the `Counter` operation
/// `operation`.
fn applies(transition: &fsl_replay::ModelStep, operation: &str) -> bool {
    transition
        .transition
        .operation()
        .contains(&format!("Counter/{operation}"))
}

/// The oracle for a lasso over `fixture`, read from the model's own
/// successor relation: the last post-state of the prefix is the state at
/// the loop's entry, and a weak constraint on the `Counter` operation
/// `operation` (all of its transitions together, or each transition alone
/// when `each`) is taken in the loop or disabled at some state of it.
fn assert_weakly_fair(
    fixture: &Fixture,
    counterexample: &TemporalCounterexample,
    operation: &str,
    each: bool,
) {
    let lasso = lasso_of(counterexample);
    assert!(!lasso.terminal_stutter);
    let states = fixture.positions(counterexample);
    let system = fixture.system();
    let state_key = |state: &ModelSimState| system.state_key(state).expect("encodes").1;
    assert_eq!(
        state_key(states.last().expect("a position")),
        state_key(&states[lasso.entry]),
        "the loop re-enters its entry state"
    );
    let loop_steps = &counterexample.prefix[lasso.entry..];
    assert!(!loop_steps.is_empty(), "a model loop has a step");
    let loop_states = &states[lasso.entry..states.len() - 1];
    let enabled: Vec<Vec<fsl_replay::ModelStep>> = loop_states
        .iter()
        .map(|state| {
            system
                .successors(state)
                .expect("the state expands")
                .successors
                .into_iter()
                .map(|(transition, next)| fsl_replay::ModelStep {
                    transition,
                    post_state: state_key(&next),
                    delay: None,
                })
                .collect()
        })
        .collect();
    let groups: Vec<Option<&fsl_replay::ModelStep>> = if each {
        let mut identities: Vec<&fsl_replay::ModelStep> = enabled
            .iter()
            .flatten()
            .filter(|step| applies(step, operation))
            .collect();
        identities.sort_by(|left, right| left.transition.cmp(&right.transition));
        identities.dedup_by(|left, right| left.transition == right.transition);
        identities.into_iter().map(Some).collect()
    } else {
        vec![None]
    };
    for group in groups {
        let covers = |step: &fsl_replay::ModelStep| {
            applies(step, operation) && group.is_none_or(|one| one.transition == step.transition)
        };
        let taken = loop_steps.iter().any(covers);
        let disabled_somewhere = enabled.iter().any(|steps| !steps.iter().any(covers));
        assert!(
            taken || disabled_somewhere,
            "the weak constraint on {operation} ({group:?}) is starved in the loop"
        );
    }
}

pub(crate) fn is_three(value: i64) -> Option<bool> {
    Some(value == 3)
}

pub(crate) fn is_zero(value: i64) -> Option<bool> {
    Some(value == 0)
}
