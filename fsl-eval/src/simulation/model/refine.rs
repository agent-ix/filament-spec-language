// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-140 (ADR-020 RM-7): the seam a refinement reads a model system through.
//!
//! A refinement's mapping rows are evaluated over a concrete state's
//! synthesized observation (ADR-016 ID-10) by the one clause evaluator
//! (FR-107), and the mapped abstract state is compared with the abstract
//! system's own states by FR-101's state key. Both need the system's private
//! state model, so this module is the one place the refinement reads it.

use filament_checked::key::DeclarationKey;
use filament_eval::{FamilyOutcome, FamilyResult};
use filament_exact::{Meter, ObjectReference, Outcome, Value};
use fsl_semantics::check::CheckedRow;
use fsl_semantics::model::observation::AdmissionFailure;
use fsl_semantics::model::state::{ModelState, StateField};

use std::collections::BTreeMap;

use filament_exact::ValueType;

use super::domain::{Domain, Odometer};
use super::walk::{exhausted, stop};
use super::{
    encode, identity_limits, state_document, ClauseOutcome, ModelSimState, ModelSystem, BROKEN,
};
use crate::simulation::expansion::{ExpansionLimit, ExpansionStop};
use crate::simulation::key;
use crate::temporal::UndefinedCause;

/// What one row evaluation gave (ADR-020 RE-4, RE-5).
#[derive(Clone, Debug)]
pub enum RowReading {
    /// The row completed with this value.
    Value(Value),
    /// The row evaluated undefined, for this cause (RE-5).
    Undefined(UndefinedCause),
    /// The row was refused or incomplete: no value, and no refutation
    /// (RE-4).
    Undetermined(ClauseOutcome),
}

/// The observation of one state, which [`ModelSystem::read_row`] reads.
#[derive(Clone, Debug)]
pub struct StateObservation(fsl_semantics::model::observation::Observation);

impl ModelSystem<'_> {
    /// The identity of the one domain package this system is over; `None`
    /// for a system over several selected models.
    pub fn model(&self) -> Option<&str> {
        self.over.as_deref()
    }

    /// The evaluation meter budget each clause evaluation gets.
    pub fn meter(&self) -> filament_exact::ScalarLimits {
        self.meter
    }

    /// The most candidates one expansion may generate.
    pub fn max_candidates(&self) -> u64 {
        self.limits.max_candidates
    }

    /// The initial states, in the order the system was given them.
    pub fn initial_states(&self) -> &[ModelSimState] {
        &self.initial
    }

    /// Evaluate `row` over `observed`, with `bindings` for
    /// its parameters (`self` first) and a fresh meter of this system's
    /// budget.
    ///
    /// # Errors
    ///
    /// `runtime_invariant`/`established-invariant-broken` when the evaluator
    /// faults.
    pub fn read_row(
        &self,
        row: &CheckedRow,
        observed: &StateObservation,
        bindings: Vec<Value>,
    ) -> Result<RowReading, ExpansionStop> {
        let evaluation = self.evaluator().evaluate_refinement_row(
            row,
            &observed.0.environment,
            bindings,
            &mut Meter::new(self.meter),
        )
        .map_err(|_| stop(BROKEN))?;
        reading(&evaluation.outcome)
    }

    /// The synthesized observation of `state`, built once for every row read
    /// over it.
    ///
    /// # Errors
    ///
    /// `runtime_invariant`/`established-invariant-broken` when `state` is not
    /// a state of this system.
    pub fn observe(&self, state: &ModelState) -> Result<StateObservation, ExpansionStop> {
        self.observe_with_history(state, BTreeMap::new())
    }

    /// [`Self::observe`], with the history values `history` a refinement
    /// keeps beside `state` (FR-138), by object and field name, which a row
    /// reads as it reads a field.
    ///
    /// # Errors
    ///
    /// As [`Self::observe`].
    pub fn observe_with_history(
        &self,
        state: &ModelState,
        history: BTreeMap<(ObjectReference, String), Value>,
    ) -> Result<StateObservation, ExpansionStop> {
        let _key = self.state_key_of(state).map_err(|_| stop(BROKEN))?;
        let mut observation = self
            .model
            .observation(state, state_document())
            .map_err(|_| stop(BROKEN))?;
        observation.environment = observation.environment.with_history(history);
        Ok(StateObservation(observation))
    }

    /// Evaluate the history update `row` of one step (FR-138): its reads of
    /// `self` and of the step's arguments observe `post`, and `pre(..)`
    /// operands observe `pre`. `bindings` fill the row's parameters, the
    /// receiver first, and the evaluation gets a fresh meter of this
    /// system's budget.
    ///
    /// # Errors
    ///
    /// `runtime_invariant`/`established-invariant-broken` when the evaluator
    /// faults.
    pub fn read_history_update(
        &self,
        row: &CheckedRow,
        (pre, post): (&StateObservation, &StateObservation),
        bindings: Vec<Value>,
    ) -> Result<RowReading, ExpansionStop> {
        let evaluation = self.evaluator().evaluate_history_update(
            row,
            (&pre.0.environment, &post.0.environment),
            bindings,
            &mut Meter::new(self.meter),
        )
        .map_err(|_| stop(BROKEN))?;
        reading(&evaluation.outcome)
    }

    /// FR-181's typed canonical bytes of `value`, which key a history value
    /// in a refinement's product (FR-142).
    ///
    /// # Errors
    ///
    /// [`crate::simulation::EncodingRefusal`] when `value` has no encoding.
    pub fn value_key(&self, value: &Value) -> Result<Vec<u8>, crate::simulation::EncodingRefusal> {
        key::canonical_bytes(&encode::value_enc(value, self.typing()), identity_limits())
    }

    /// Whether the object type `concrete` is `ancestor` or a subtype of it;
    /// `false` when either is not a type of this system's model.
    pub fn type_conforms(&self, concrete: &DeclarationKey, ancestor: &DeclarationKey) -> bool {
        match (
            self.model.type_identity(concrete),
            self.model.type_identity(ancestor),
        ) {
            (Some(concrete), Some(ancestor)) => self.types.conforms(concrete, ancestor),
            _ => false,
        }
    }

    /// Every value of the finite domain of `value_type`, in FR-120's order,
    /// where a `Reference<T>` ranges over the members of `objects` whose type
    /// conforms to `T`.
    ///
    /// # Errors
    ///
    /// `MaxCandidates` when the domain is larger than this system's
    /// candidate bound, or `value_type` has no enumerable domain.
    pub fn domain_values(
        &self,
        value_type: &ValueType,
        objects: &[ObjectReference],
    ) -> Result<Vec<Value>, ExpansionStop> {
        let domain = Domain::of(value_type, objects, self.types, &self.record_orders)
            .ok_or_else(|| exhausted(ExpansionLimit::MaxCandidates))?;
        if domain.len() > u128::from(self.limits.max_candidates) {
            return Err(exhausted(ExpansionLimit::MaxCandidates));
        }
        Ok((0..domain.len())
            .map_while(|index| domain.nth(index))
            .collect())
    }

    /// FR-101's canonical key of `state`, for a state built outside this system's expansion
    /// (a mapped abstract state).
    ///
    /// # Errors
    ///
    /// [`crate::simulation::EncodingRefusal`] when the key has no RFC 8785
    /// encoding.
    pub fn state_key_of(
        &self,
        state: &ModelState,
    ) -> Result<(Vec<u8>, crate::simulation::StateKey), crate::simulation::EncodingRefusal> {
        key::state_key(
            &encode::model_key(&self.model, state, self.typing()),
            identity_limits(),
        )
    }

    /// The reference to the object `object` of the type `type_key`.
    ///
    /// # Errors
    ///
    /// As [`fsl_semantics::model::state::StateModel::object_reference`].
    pub fn object_reference(
        &self,
        type_key: &DeclarationKey,
        object: &str,
    ) -> Result<ObjectReference, AdmissionFailure> {
        self.model.object_reference(type_key, object)
    }

    /// The fields of the object type `type_key`, in its effective attribute
    /// order; `None` when this system's model does not declare the type.
    pub fn fields_of(&self, type_key: &DeclarationKey) -> Option<&[StateField]> {
        self.fields.get(type_key).map(Vec::as_slice)
    }

    /// The declaration key of the type of `reference`, when this system's
    /// model declares it.
    pub fn type_key_of(&self, reference: &ObjectReference) -> Option<&DeclarationKey> {
        self.model.type_key(reference.object_type().clone())
    }
}

/// Read a row evaluation's outcome.
///
/// A `match` over [`FamilyOutcome`] would have to name its probe-only
/// variant (FR-063), so the two real arms are tested in turn.
fn reading(outcome: &FamilyOutcome<Value>) -> Result<RowReading, ExpansionStop> {
    if let FamilyOutcome::Evaluated(kernel) = outcome {
        return Ok(match kernel {
            Outcome::Completed(value) => RowReading::Value(value.clone()),
            Outcome::Undefined(undefined) => {
                RowReading::Undefined(UndefinedCause::Kernel(undefined.clone()))
            }
            Outcome::Refused(refusal) => {
                RowReading::Undetermined(ClauseOutcome::KernelRefused(refusal.clone()))
            }
            Outcome::Incomplete(incomplete) => {
                RowReading::Undetermined(ClauseOutcome::Incomplete(incomplete.clone()))
            }
        });
    }
    if let FamilyOutcome::FamilyEvaluated(family) = outcome {
        return Ok(match family {
            FamilyResult::Refused(cause) => {
                RowReading::Undetermined(ClauseOutcome::FamilyRefused(cause.catalog_code()))
            }
            FamilyResult::Undefined(cause) => {
                RowReading::Undefined(UndefinedCause::Family(cause.undefined_record()))
            }
        });
    }
    // The probe-only variant is never built by a real evaluation.
    Err(stop(BROKEN))
}

impl<'a> ModelSystem<'a> {
    /// This system with `states` as its initial states, in the order given,
    /// each anchored as its paired anchor says. A refinement whose abstract
    /// subject names no initial snapshots builds its abstract system this
    /// way, from the states its mapping gives the concrete initial states
    /// (FSL FR-630 "Request item"), not from snapshot documents.
    #[must_use]
    pub fn with_initial_states(
        mut self,
        states: impl IntoIterator<
            Item = (
                ModelState,
                fsl_semantics::model::observation::SelectedAnchor,
            ),
        >,
    ) -> Self {
        self.initial = states
            .into_iter()
            .map(|(state, anchor)| ModelSimState {
                state,
                anchor,
                reached_by: None,
            })
            .collect();
        self
    }
}

impl ModelSystem<'_> {
    /// Every state that agrees with `visible` and gives each open `slot`
    /// (population, object key, field) a value of its field's type, and
    /// whose invariants and unit bounds all hold (FSL FR-630 "Request item":
    /// hidden fields range over the values the abstract model's initial
    /// constraint admits). An absent `?` field is one of the values. The
    /// result is empty when no assignment satisfies the constraint.
    ///
    /// # Errors
    ///
    /// An [`ExpansionStop`] when a slot names no object or field of this
    /// system, a field's type is not enumerable, the assignments exceed
    /// the remaining `budget` of assignments (the caller's `max_candidates`,
    /// shared by every call of one opening), or an evaluation stops.
    pub fn complete_open_fields(
        &self,
        visible: &ModelState,
        anchor: &fsl_semantics::model::observation::SelectedAnchor,
        slots: &[(DeclarationKey, String, DeclarationKey)],
        budget: &mut u128,
    ) -> Result<Vec<ModelState>, ExpansionStop> {
        let mut references = Vec::new();
        for objects in visible.populations.values() {
            for (key, object) in objects {
                references.push(
                    self.object_reference(&object.type_identity, key)
                        .map_err(|_| stop(BROKEN))?,
                );
            }
        }
        let mut domains = Vec::new();
        for (population, key, field) in slots {
            let object = visible
                .populations
                .get(population)
                .and_then(|objects| objects.get(key))
                .ok_or_else(|| stop(BROKEN))?;
            let declared = self
                .fields
                .get(&object.type_identity)
                .and_then(|fields| fields.iter().find(|declared| declared.key == *field))
                .ok_or_else(|| stop(BROKEN))?;
            let domain = Domain::of(
                &declared.value_type,
                &references,
                self.types,
                &self.record_orders,
            )
            .ok_or_else(|| exhausted(ExpansionLimit::MaxCandidates))?;
            domains.push((domain, declared.optional));
        }
        let radices: Vec<u128> = domains
            .iter()
            .map(|(domain, optional)| domain.len().saturating_add(u128::from(*optional)))
            .collect();
        let total = radices
            .iter()
            .try_fold(1u128, |total, radix| total.checked_mul(*radix));
        match total {
            Some(total) if total <= *budget => *budget -= total,
            _ => return Err(exhausted(ExpansionLimit::MaxCandidates)),
        }
        let mut completed = Vec::new();
        let mut assignments = Odometer::new(radices);
        while let Some(digits) = assignments.next_digits() {
            let mut state = visible.clone();
            for (((population, key, field), (domain, optional)), digit) in
                slots.iter().zip(&domains).zip(&digits)
            {
                let object = state
                    .populations
                    .get_mut(population)
                    .and_then(|objects| objects.get_mut(key))
                    .ok_or_else(|| stop(BROKEN))?;
                // With an optional field, digit 0 is the absent value.
                let index = if *optional {
                    match digit.checked_sub(1) {
                        Some(index) => index,
                        None => {
                            object.fields.remove(field);
                            continue;
                        }
                    }
                } else {
                    *digit
                };
                let value = domain.nth(index).ok_or_else(|| stop(BROKEN))?;
                object.fields.insert(field.clone(), value);
            }
            let candidate = ModelSimState {
                state,
                anchor: anchor.clone(),
                reached_by: None,
            };
            if self.invariant_findings_at(&candidate)?.is_empty() {
                completed.push(candidate.state);
            }
        }
        Ok(completed)
    }
}
