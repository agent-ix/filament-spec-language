// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-125 Instancing, FR-107: the production [`ModelLetters`] of a checked
//! temporal clause over a [`ModelSystem`].
//!
//! A `holds` atom is evaluated through the one clause evaluator (FR-107)
//! over the state's synthesized observation, as a state-graph predicate is
//! ([`super::state_predicate`]). The clause's `over` parameter is bound to
//! the state's own reference to the object under the instance's key, matched
//! by key across every population that overlaps the parameter's type, never
//! by comparing the whole reference.
//!
//! Where no such object conforms to the parameter's type, or none is held
//! under the key, the clause is evaluated with the parameter at that key: a
//! read through it is `absent-key` undefined, but its use as the reference
//! operand of a `lookup<T>(p, x)`, the guard, is not, so the guard is false
//! where `p` holds no conforming object under the key (FR-125 Instancing). An
//! atom that does not read the parameter is evaluated normally.

use std::cell::{Cell, RefCell};
use std::collections::BTreeSet;
use std::rc::Rc;

use filament_checked::key::DeclarationKey;
use filament_eval::FamilyOutcome;
use filament_exact::{EffectiveId, Meter, NodeKey, ObjectReference, Outcome, Value, ValueType};
use filament_semantic_value::catalog::InternalFault;
use fsl_semantics::check::footprint::{FootprintRefusal, SymbolicLocation};
use fsl_semantics::check::{
    CheckedNode, CheckedTemporalClause, FormulaNodeId, StatePredicate, TemporalClauseNode,
};
use fsl_semantics::model::observation::Observation;

use super::contracts::{Verdict, verdict};
use super::reference_bindings::ReferenceBinding;
use super::walk::{Row, rows_of, stop};
use super::{BROKEN, ClauseOutcome, ModelSimState, ModelSystem, state_document};
use crate::model_subject::{AtomStop, ModelInstance, ModelLetters};
use crate::simulation::StateKey;
use crate::temporal::{AtomReading, CaptureReading};
use crate::value::{
    evaluate_state_predicate_at_absent_key_with_reference_bindings,
    evaluate_state_predicate_with_reference_bindings,
};

/// Why a clause has no [`ClauseLetters`].
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum LettersRefusal {
    /// Package hyper reference literals need an authenticated binding projection.
    #[error("package hyper reference literal bindings are not implemented")]
    PackageHyperReferences,
    /// The clause's `over` parameter is not an object reference, so the
    /// clause has no instance (FR-125 Instancing).
    #[error("the clause's `over` parameter is not an object reference")]
    OverNotReference,
    /// The `over` parameter's object type is declared by no model of the
    /// subject, so no reference to an object of it can be formed.
    #[error("the clause's `over` object type is declared by no model of the subject")]
    OverTypeUndeclared,
    /// A hyper clause's letters were given other than one system for each of
    /// the clause's trace variables (FR-487).
    #[error("a hyper clause needs one model system for each of its trace variables")]
    ComponentsMismatch,
    /// A hyper clause's components resolve at least one reference literal
    /// differently at subject admission (FR-1000-AC-11).
    #[error(
        "hyper components {components:?} disagree on reference literal {object_type}#{identity}: {bindings:?}"
    )]
    ReferenceBindingsDisagree {
        /// The effective object type named by the literal.
        object_type: String,
        /// The UTF-8 identity bytes, rendered as lowercase hexadecimal.
        identity: String,
        /// The first component and the component with a differing binding.
        components: Vec<usize>,
        /// The resolved binding for each reported component.
        bindings: Vec<String>,
    },
    /// A separately checked hyper clause contains a literal that the
    /// component subject cannot admit.
    #[error("hyper reference literal admission failed: {0}")]
    ReferenceBindingsAdmission(fsl_semantics::model::observation::AdmissionFailure),
}

/// A caller-budgeted letter preparation stopped in admission or accounting.
#[derive(Debug, thiserror::Error)]
pub enum LettersPreparationError {
    /// The existing clause/reference-binding admission refusal.
    #[error(transparent)]
    Admission(#[from] LettersRefusal),
    /// Original representation accounting, cancellation or storage refusal.
    #[error(transparent)]
    Policy(#[from] crate::automaton::PolicyStateError),
}

/// The `holds` atoms of one checked temporal clause, read at the states of a
/// [`ModelSystem`].
///
/// A refused or faulted `holds` evaluation (for example an authored
/// population bound exceeded, FR-380, or a population a snapshot does not
/// list complete), a state that cannot be observed and a broken invariant are
/// each an [`AtomStop`] of [`ModelLetters::atom`], never a panic (QSL-1039).
///
/// The letters observe a state once, however many atoms and instances read
/// it: the state's key selects the last observation, which is kept for the
/// reads that follow. The observation is not charged to the engine's meter,
/// as `state_predicates` does not charge it; charging it needs a ruling.
pub struct ClauseLetters<'s, 'a> {
    system: &'s ModelSystem<'a>,
    node: ClauseSource<'s>,
    /// Base package bindings extended with literals from this separately
    /// checked temporal clause.
    reference_bindings: std::borrow::Cow<'s, super::reference_bindings::ReferenceBindings>,
    parameter: &'s EffectiveId,
    /// The declaration key of the parameter's object type.
    type_key: &'s DeclarationKey,
    overlapping: Vec<&'s DeclarationKey>,
    /// The last state observed, by its canonical key.
    observed: RefCell<Option<(StateKey, Rc<Observed>)>>,
    /// How many states have been observed (not read from `observed`).
    observations: Cell<u64>,
    /// Original package call stop, retained beyond the letter engine boundary.
    call_failure: RefCell<Option<filament_eval::CallFailure>>,
}

/// The clause a [`ClauseLetters`] reads: the source route's checked node, or
/// the package route's package form keyed by its node (FR-940).
#[derive(Clone, Copy)]
enum ClauseSource<'s> {
    Source(&'s TemporalClauseNode),
    Package {
        clause: &'s CheckedTemporalClause,
        key: &'s NodeKey,
    },
}

impl<'s> ClauseSource<'s> {
    fn clause(self) -> &'s CheckedTemporalClause {
        match self {
            Self::Source(node) => node.clause(),
            Self::Package { clause, .. } => clause,
        }
    }
}

/// A model state observed for the decisive atom of a violation.
#[derive(Clone, Debug)]
pub struct DecisiveObservation {
    /// The object the clause's `over` parameter reads as.
    pub over: ObjectReference,
    /// The objects of the state's observation.
    pub environment: fsl_semantics::model::object_environment::ObjectEnvironment,
    /// The identity of the state's synthesized observation.
    pub identity: fsl_semantics::model::observation::DocumentRef,
}

/// One state as the letters read it: its synthesized observation and its
/// objects.
pub(super) struct Observed {
    pub(super) observation: Observation,
    pub(super) rows: Vec<Row>,
}

impl<'a> ModelSystem<'a> {
    /// The [`ModelLetters`] of `node` over this system.
    ///
    /// # Errors
    ///
    /// [`LettersRefusal::OverNotReference`] when the clause's `over`
    /// parameter is not an object reference, and
    /// [`LettersRefusal::OverTypeUndeclared`] when no model of the subject
    /// declares its type. A clause that activates `on each ... when (g)`
    /// reads `g` through [`ModelLetters::activation`] (FR-337, FR-125-AC-18).
    pub fn clause_letters<'s>(
        &'s self,
        node: &'s TemporalClauseNode,
    ) -> Result<ClauseLetters<'s, 'a>, LettersRefusal> {
        self.letters(ClauseSource::Source(node))
    }

    /// Prepare source policy letters on the caller's original representation
    /// ledger, borrowing immutable model identities and admitted bindings.
    ///
    /// # Errors
    /// Existing clause admission, accounting, cancellation or storage refusal.
    pub fn clause_letters_with_meter<'s>(
        &'s self,
        node: &'s TemporalClauseNode,
        meter: &mut filament_exact::Meter,
    ) -> Result<ClauseLetters<'s, 'a>, LettersPreparationError> {
        let budget = crate::policy_budget::Budget {
            meter: RefCell::new(meter),
            stopped: RefCell::new(None),
        };
        budget.work(1)?;
        let ValueType::Reference(parameter) = &node.clause().over().1 else {
            return Err(LettersRefusal::OverNotReference.into());
        };
        let type_key = self
            .model
            .declaration_key_with_work(parameter, &mut |units| {
                budget.work(
                    usize::try_from(units)
                        .map_err(|_| crate::automaton::PolicyStateError::Invalid)?,
                )
            })?
            .ok_or(LettersRefusal::OverTypeUndeclared)?;
        let reference_bindings = match self.source_package() {
            Some(package)
                if !self
                    .reference_bindings
                    .contains_temporal_literals_with_budget(package.graph(), node, &budget)? =>
            {
                // Separately checked clauses still need the shared resolver's
                // budgeted extension boundary; do not omit their admission.
                std::borrow::Cow::Owned(self.reference_bindings.with_temporal_clause_with_budget(
                    package.graph(),
                    node,
                    &self.model,
                    self.types,
                    &self.populations,
                    &self.universes,
                    &self.initial_base,
                    &budget,
                )?)
            }
            _ => std::borrow::Cow::Borrowed(&self.reference_bindings),
        };
        let mut overlapping = Vec::new();
        for population in &self.populations {
            budget.work(1)?;
            if self.overlaps_with_budget(population, parameter, &budget)? {
                budget.growth(&mut overlapping, 1)?;
                overlapping.push(&population.key);
            }
        }
        Ok(ClauseLetters {
            system: self,
            node: ClauseSource::Source(node),
            reference_bindings,
            parameter,
            type_key,
            overlapping,
            observed: RefCell::new(None),
            observations: Cell::new(0),
            call_failure: RefCell::new(None),
        })
    }

    /// FR-940: the [`ModelLetters`] of the package form `clause` of the
    /// temporal clause whose node key is `key`, over this system: each `holds`
    /// atom and the activation guard read through the input's evaluator keyed
    /// by node ([`super::ModelEvaluator::evaluate_holds`],
    /// [`super::ModelEvaluator::evaluate_guard`]). The package form lists each
    /// formula node once, so an atom's arena index is its use position.
    ///
    /// # Errors
    ///
    /// As [`Self::clause_letters`].
    pub fn package_clause_letters<'s>(
        &'s self,
        clause: &'s CheckedTemporalClause,
        key: &'s NodeKey,
    ) -> Result<ClauseLetters<'s, 'a>, LettersRefusal> {
        self.letters(ClauseSource::Package { clause, key })
    }

    fn letters<'s>(
        &'s self,
        node: ClauseSource<'s>,
    ) -> Result<ClauseLetters<'s, 'a>, LettersRefusal> {
        let ValueType::Reference(ref parameter) = node.clause().over().1 else {
            return Err(LettersRefusal::OverNotReference);
        };
        let type_key = self
            .model
            .type_key(parameter.clone())
            .ok_or(LettersRefusal::OverTypeUndeclared)?;
        let reference_bindings = match (node, self.source_package()) {
            (ClauseSource::Source(source_node), Some(package)) => {
                super::reference_bindings::ReferenceBindings::with_temporal_clause(
                    &self.reference_bindings,
                    package.graph(),
                    source_node,
                    &self.model,
                    self.types,
                    &self.populations,
                    &self.universes,
                    &self.initial_base,
                )
                .map_err(LettersRefusal::ReferenceBindingsAdmission)?
            }
            (ClauseSource::Package { clause, key }, _) => self
                .reference_bindings
                .with_literals(
                    self.evaluator().temporal_reference_literals(clause, key),
                    &self.model,
                    self.types,
                    &self.populations,
                    &self.universes,
                    &self.initial_base,
                )
                .map_err(LettersRefusal::ReferenceBindingsAdmission)?,
            (ClauseSource::Source(_), None) => self.reference_bindings.clone(),
        };
        Ok(ClauseLetters {
            system: self,
            node,
            reference_bindings: std::borrow::Cow::Owned(reference_bindings),
            parameter,
            type_key,
            overlapping: self
                .populations
                .iter()
                .filter(|population| self.overlaps(population, parameter.clone()))
                .map(|population| &population.key)
                .collect(),
            observed: RefCell::new(None),
            observations: Cell::new(0),
            call_failure: RefCell::new(None),
        })
    }
}

/// One `holds` condition as a predicate of the clause evaluator.
struct Condition<'n> {
    body: CheckedNode<'n>,
    slots: usize,
}

impl StatePredicate for Condition<'_> {
    fn body(&self) -> Option<CheckedNode<'_>> {
        Some(self.body)
    }

    fn slots(&self) -> usize {
        self.slots
    }
}

impl ClauseLetters<'_, '_> {
    /// Take the original checked-package call failure after the letter engine
    /// stops. Its admission diagnostic and package locus are kept intact.
    pub fn take_call_failure(&self) -> Option<filament_eval::CallFailure> {
        self.call_failure.borrow_mut().take()
    }

    fn call_stop(&self, failure: filament_eval::CallFailure) -> AtomStop {
        if let filament_eval::CallFailure::Fault(fault) = failure {
            return AtomStop::Fault(fault);
        }
        *self.call_failure.borrow_mut() = Some(failure);
        // The engine accepts letter stops only; the caller reads the original
        // typed failure before interpreting this boundary stop.
        fault("package-call-stopped")
    }

    /// Evaluate one `holds` atom with the capture values fixed for its
    /// activation position.
    pub(super) fn atom_with_captures(
        &self,
        atom: FormulaNodeId,
        instance: &ModelInstance,
        state: &ModelSimState,
        captures: &[Value],
        meter: &mut Meter,
    ) -> Result<AtomReading, AtomStop> {
        let node = match self.node {
            ClauseSource::Source(node) => node,
            ClauseSource::Package { key, .. } => {
                let observed = self.observe(state)?;
                let bound = self.bound(&observed.rows, instance);
                let absent = bound.is_none().then_some(instance.population.node.as_str());
                let over = match bound {
                    Some(bound) => bound,
                    None => self.key_reference(instance)?,
                };
                return reading(
                    &self
                        .system
                        .evaluator()
                        .evaluate_model_holds(
                            &crate::value::HoldsAtom {
                                clause: key.clone(),
                                position: atom.index(),
                            },
                            Value::Reference(over),
                            absent,
                            captures,
                            &observed.observation.environment,
                            &self.reference_bindings,
                            meter,
                        )
                        .map_err(|failure| self.call_stop(failure))?
                        .outcome,
                );
            }
        };
        let (body, slots) = node
            .condition(atom)
            .ok_or_else(|| fault("engine-reads-only-holds-atoms"))?;
        let observed = self.observe(state)?;
        let condition = Condition { body, slots };
        let further = captures.to_vec();
        let evaluation = if let Some(package) = self.system.source_package() {
            match self.bound(&observed.rows, instance) {
                Some(bound) => evaluate_state_predicate_with_reference_bindings(
                    package,
                    &condition,
                    &observed.observation.environment,
                    Some(bound),
                    further,
                    Some(&self.reference_bindings),
                    meter,
                ),
                None => evaluate_state_predicate_at_absent_key_with_reference_bindings(
                    package,
                    &condition,
                    &observed.observation.environment,
                    self.key_reference(instance)?,
                    instance.population.node.clone(),
                    Some(&self.reference_bindings),
                    meter,
                ),
            }
        } else {
            match self.bound(&observed.rows, instance) {
                Some(bound) => self.system.evaluator().evaluate_state_predicate(
                    &condition,
                    &observed.observation.environment,
                    Some(bound),
                    further,
                    meter,
                ),
                None => self
                    .system
                    .evaluator()
                    .evaluate_state_predicate_at_absent_key(
                        &condition,
                        &observed.observation.environment,
                        self.key_reference(instance)?,
                        instance.population.node.clone(),
                        meter,
                    ),
            }
        }
        .map_err(AtomStop::Fault)?;
        reading(&evaluation.outcome)
    }

    /// Evaluate a clause's captures at an activation and return their typed
    /// values for the following `holds` reads.
    pub(super) fn captures_with_values(
        &self,
        instance: &ModelInstance,
        state: &ModelSimState,
        meter: &mut Meter,
    ) -> Result<(CaptureReading, Vec<Value>), AtomStop> {
        let clause = self.node.clause();
        if clause.captures().len() == 0 {
            return Ok((CaptureReading::Fixed, Vec::new()));
        }
        let ClauseSource::Source(node) = self.node else {
            return Err(fault("package-form-reads-no-capture-yet"));
        };
        let observed = self.observe(state)?;
        let binders = 1 + usize::from(clause.trigger().is_some());
        let bound = self.bound(&observed.rows, instance);
        let object = match &bound {
            Some(bound) => bound.clone(),
            None => self.key_reference(instance)?,
        };
        let mut values: Vec<Value> = Vec::new();
        for (index, (name, _)) in clause.captures().enumerate() {
            let (body, slots) = node
                .capture(index)
                .ok_or_else(|| fault("engine-reads-the-typed-captures-of-a-clause"))?;
            let capture = Condition { body, slots };
            let mut arguments = vec![Value::Reference(object.clone()); binders];
            arguments.extend(values.iter().cloned());
            let absent = bound
                .is_none()
                .then(|| ((0..binders).collect(), instance.population.node.clone()));
            let evaluation = self
                .system
                .evaluator()
                .evaluate_capture(
                    &capture,
                    &observed.observation.environment,
                    arguments,
                    absent,
                    meter,
                )
                .map_err(AtomStop::Fault)?;
            if let FamilyOutcome::Evaluated(Outcome::Completed(value)) = &evaluation.outcome {
                values.push(value.clone());
                continue;
            }
            let reading = match verdict(&evaluation.outcome) {
                Verdict::Undecided(outcome) => {
                    if let Some(cause) = outcome.undefined_cause() {
                        CaptureReading::Undefined {
                            capture: name.to_owned(),
                            cause,
                        }
                    } else if let ClauseOutcome::Incomplete(incomplete) = outcome {
                        return Err(AtomStop::Incomplete(incomplete));
                    } else {
                        return Err(AtomStop::Refused(outcome));
                    }
                }
                Verdict::True | Verdict::False => {
                    return Err(fault("a-capture-completes-no-boolean-verdict"));
                }
            };
            return Ok((reading, values));
        }
        Ok((CaptureReading::Fixed, values))
    }

    /// The reference the `over` parameter reads as under `instance`'s key in
    /// the observed state: the first object of an overlapping population held
    /// under the key whose most-specific type conforms to the parameter's.
    fn bound(&self, rows: &[Row], instance: &ModelInstance) -> Option<ObjectReference> {
        rows.iter()
            .find(|row| {
                row.key == instance.key
                    && self
                        .overlapping
                        .iter()
                        .any(|population| **population == row.population)
                    && self.system.types.conforms(
                        row.reference.object_type().clone(),
                        (*self.parameter).clone(),
                    )
            })
            .map(|row| row.reference.clone())
    }

    /// Add the subject-dependent membership reads of CreatedObject literals
    /// from a separately checked temporal expression. The model evaluator's
    /// graph-only footprint cannot see these literals in the base package.
    fn add_reference_literal_reads(
        &self,
        root: CheckedNode<'_>,
        reads: &mut BTreeSet<SymbolicLocation>,
    ) {
        let Some(package) = self.system.source_package() else {
            return;
        };
        let mut literals = BTreeSet::new();
        super::reference_bindings::collect_literals(package.graph(), root, &mut literals);
        for literal in literals {
            if let Some(ReferenceBinding::CreatedObject { populations, .. }) = self
                .reference_bindings
                .get(&literal.object_type, &literal.identity)
            {
                reads.extend(
                    populations
                        .iter()
                        .cloned()
                        .map(|population| SymbolicLocation::AnyMembership { population }),
                );
            }
        }
    }
}

impl ClauseLetters<'_, '_> {
    /// The reference of the parameter's type under `instance`'s key, which no
    /// object of the state need hold.
    ///
    /// # Errors
    ///
    /// A fault when the model refuses a reference under the key (an empty
    /// key), which the subject's admission refuses before it forms an
    /// instance.
    fn key_reference(&self, instance: &ModelInstance) -> Result<ObjectReference, AtomStop> {
        self.system
            .model
            .object_reference(self.type_key, &instance.key)
            .map_err(|_| fault("instance-key-names-a-reference"))
    }

    /// The written binding of the clause's `over` parameter to `instance`
    /// (FSpec FR-364's `over_binding`; [`ModelSystem::written_reference`]), or
    /// `None` when the model refuses a reference under the key.
    pub fn instance_reference(&self, instance: &ModelInstance) -> Option<ObjectReference> {
        self.system
            .written_reference(self.parameter.clone(), instance)
            .ok()
    }

    /// FR-330 "The decisive atom" over a model state: the observation the
    /// letters read at `state`, the environment the atom is evaluated over,
    /// with the reference the `over` parameter reads as under `instance`'s
    /// key in it. `None` when the state holds no conforming object under
    /// the key.
    ///
    /// # Errors
    ///
    /// [`AtomStop::Unobservable`] when `state` is no state of the system.
    pub fn decisive_observation(
        &self,
        instance: &ModelInstance,
        state: &ModelSimState,
    ) -> Result<Option<DecisiveObservation>, AtomStop> {
        let observed = self.observe(state)?;
        Ok(self
            .bound(&observed.rows, instance)
            .map(|over| DecisiveObservation {
                over,
                environment: observed.observation.environment.clone(),
                identity: observed.observation.identity.clone(),
            }))
    }

    /// How many states these letters have observed: a state read again, by
    /// another atom or instance, is not observed again.
    pub fn observations(&self) -> u64 {
        self.observations.get()
    }

    /// `state`, observed once however many reads follow.
    fn observe(&self, state: &ModelSimState) -> Result<Rc<Observed>, AtomStop> {
        let key = self
            .system
            .canonical_state_key_ref(state)
            .map_err(|_| AtomStop::Unobservable(stop(BROKEN)))?;
        let known = self
            .observed
            .borrow()
            .as_ref()
            .filter(|(known, _)| known == key)
            .map(|(_, observed)| Rc::clone(observed));
        match known {
            Some(observed) => Ok(observed),
            None => {
                let observation = self
                    .system
                    .model
                    .observation(state.state(), state_document())
                    .map_err(|_| AtomStop::Unobservable(stop(BROKEN)))?;
                let rows = rows_of(self.system, state.state()).map_err(AtomStop::Unobservable)?;
                let observed = Rc::new(Observed { observation, rows });
                self.observations
                    .set(self.observations.get().saturating_add(1));
                *self.observed.borrow_mut() = Some((key.clone(), Rc::clone(&observed)));
                Ok(observed)
            }
        }
    }
}

/// The footprint refusal of a package-form clause whose activation holds
/// captures: the package form reads no capture initializer yet (the
/// evaluation faults at it too), so a reduction that needs its reads is not
/// applied to it.
const PACKAGE_FOOTPRINT: FootprintRefusal = FootprintRefusal::Unresolved {
    what: "package-form clause footprint",
};

/// A broken invariant of the letters.
fn fault(invariant: &'static str) -> AtomStop {
    AtomStop::Fault(InternalFault::new("model-letters", invariant))
}

/// The reading of a condition's clause `outcome`.
fn reading(outcome: &FamilyOutcome<Value>) -> Result<AtomReading, AtomStop> {
    match verdict(outcome) {
        Verdict::True => Ok(AtomReading::True),
        Verdict::False => Ok(AtomReading::False),
        Verdict::Undecided(outcome) => {
            if let Some(cause) = outcome.undefined_cause() {
                Ok(AtomReading::Undefined(cause))
            } else if let ClauseOutcome::Incomplete(incomplete) = outcome {
                Err(AtomStop::Incomplete(incomplete))
            } else {
                Err(AtomStop::Refused(outcome))
            }
        }
    }
}

impl ModelLetters for ClauseLetters<'_, '_> {
    /// # Errors
    ///
    /// [`AtomStop::Incomplete`] when the meter runs out in the evaluation; [`AtomStop::Refused`] when the
    /// evaluation is refused or completes no Boolean;
    /// [`AtomStop::Unobservable`] when `state` is no state of the system; and
    /// [`AtomStop::Fault`] when `atom` is no `holds` atom of the clause,
    /// `instance`'s key names no reference, or the evaluator faults.
    fn atom(
        &self,
        atom: FormulaNodeId,
        instance: &ModelInstance,
        state: &ModelSimState,
        meter: &mut Meter,
    ) -> Result<AtomReading, AtomStop> {
        self.atom_with_captures(atom, instance, state, &[], meter)
    }

    /// The guard of the clause's `on each` activation (FR-337) for
    /// `instance` at `state`: the `over` parameter and the trigger binder are
    /// both the instance's object. A clause with no guard has no condition to
    /// read, and the engine does not ask.
    ///
    /// # Errors
    ///
    /// As [`Self::atom`]; [`AtomStop::Fault`] also when the clause has no
    /// guard.
    fn activation(
        &self,
        instance: &ModelInstance,
        state: &ModelSimState,
        meter: &mut Meter,
    ) -> Result<AtomReading, AtomStop> {
        let node = match self.node {
            ClauseSource::Source(node) => node,
            ClauseSource::Package { key, .. } => {
                let observed = self.observe(state)?;
                let bound = self.bound(&observed.rows, instance);
                let absent = bound.is_none().then_some(instance.population.node.as_str());
                let over = match bound {
                    Some(bound) => bound,
                    None => self.key_reference(instance)?,
                };
                return reading(
                    &self
                        .system
                        .evaluator()
                        .evaluate_model_guard(
                            key,
                            Value::Reference(over),
                            absent,
                            &observed.observation.environment,
                            &self.reference_bindings,
                            meter,
                        )
                        .map_err(|failure| self.call_stop(failure))?
                        .outcome,
                );
            }
        };
        let (body, slots) = node
            .guard()
            .ok_or_else(|| fault("engine-reads-the-guard-of-a-guarded-clause"))?;
        let observed = self.observe(state)?;
        let condition = Condition { body, slots };
        let evaluation = match self.bound(&observed.rows, instance) {
            Some(bound) => self.system.evaluator().evaluate_activation_guard(
                &condition,
                &observed.observation.environment,
                bound,
                meter,
            ),
            None => self
                .system
                .evaluator()
                .evaluate_activation_guard_at_absent_key(
                    &condition,
                    &observed.observation.environment,
                    self.key_reference(instance)?,
                    instance.population.node.clone(),
                    meter,
                ),
        }
        .map_err(AtomStop::Fault)?;
        reading(&evaluation.outcome)
    }

    /// FR-337 "Capture observation": the captures in source order, each
    /// through the clause evaluator over the observation at the activation,
    /// the `over` parameter and the trigger binder the instance's object and
    /// each earlier capture its value. A capture with no value ends the
    /// reading at it.
    ///
    /// # Errors
    ///
    /// As [`Self::atom`].
    fn captures(
        &self,
        instance: &ModelInstance,
        state: &ModelSimState,
        meter: &mut Meter,
    ) -> Result<CaptureReading, AtomStop> {
        self.captures_with_values(instance, state, meter)
            .map(|(reading, _)| reading)
    }

    fn captured_read(&self) -> Option<usize> {
        match self.node {
            ClauseSource::Source(node) => node.captured_read(),
            ClauseSource::Package { .. } => None,
        }
    }

    fn activation_reads(&self) -> Result<BTreeSet<SymbolicLocation>, FootprintRefusal> {
        let node = match self.node {
            ClauseSource::Source(node) => node,
            ClauseSource::Package { clause, key } => {
                if clause.captures().len() > 0 {
                    return Err(PACKAGE_FOOTPRINT);
                }
                return self.system.activation_guard_reads(key);
            }
        };
        let clause = node.clause();
        let mut reads = BTreeSet::new();
        let mut binders = vec![clause.over().clone()];
        binders.extend(
            clause
                .trigger()
                .map(|(name, value_type)| (name.to_owned(), value_type.clone())),
        );
        if let Some((guard, _)) = node.guard() {
            reads.extend(self.system.guard_reads(guard, &binders)?);
            self.add_reference_literal_reads(guard, &mut reads);
        }
        for (index, (name, declared)) in clause.captures().enumerate() {
            let (initializer, _) = node
                .capture(index)
                .ok_or(FootprintRefusal::Unresolved { what: "capture" })?;
            reads.extend(self.system.guard_reads(initializer, &binders)?);
            self.add_reference_literal_reads(initializer, &mut reads);
            binders.push((name.to_owned(), declared.clone()));
        }
        Ok(reads)
    }

    fn reads(&self, atom: FormulaNodeId) -> Result<BTreeSet<SymbolicLocation>, FootprintRefusal> {
        let node = match self.node {
            ClauseSource::Source(node) => node,
            ClauseSource::Package { key, .. } => {
                return self.system.holds_reads(key, atom.index());
            }
        };
        let (condition, _) = node
            .condition(atom)
            .ok_or(FootprintRefusal::Unresolved { what: "holds atom" })?;
        let mut reads = self.system.atom_reads(node.clause().over(), condition)?;
        self.add_reference_literal_reads(condition, &mut reads);
        Ok(reads)
    }
}
