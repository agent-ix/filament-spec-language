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

use super::domain::Odometer;
use super::walk::{exhausted, stop};
use super::{
    encode, identity_limits, state_document, BoundedDomain, ClauseOutcome, ModelSimState,
    ModelSystem, BROKEN,
};
use crate::simulation::expansion::{ExpansionLimit, ExpansionStop};
use crate::simulation::key;
use crate::simulation::subject::InitialStateRefusal;
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

impl StateObservation {
    /// The admitted object environment used to evaluate model reads at this
    /// state.
    pub fn environment(&self) -> &fsl_semantics::model::object_environment::ObjectEnvironment {
        &self.0.environment
    }
}

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
        self.initial_all
            .get_or_init(|| self.generated_initial().collect())
    }

    /// The initial states, generated one at a time from the documents' states
    /// and the clock-count and presence-bit ranges, in the order
    /// [`Self::initial_states`] lists them. No state past the last one pulled is
    /// cloned or built, so an exponential set is never held.
    pub fn generated_initial(&self) -> Box<dyn Iterator<Item = ModelSimState> + '_> {
        super::generated(&self.initial_base, &self.initial_stages)
    }

    /// Admit model initial-state recipes without materializing a generated
    /// `ModelSimState` delivered past the distinct-root budget.
    pub(crate) fn generated_initial_for_check<'s>(
        &'s self,
        max_states: usize,
        symmetry: Option<&'s crate::simulation::SymmetryGroup>,
    ) -> Box<dyn Iterator<Item = Result<ModelSimState, InitialStateRefusal>> + 's> {
        struct Factor {
            stage: usize,
            population: DeclarationKey,
            object: String,
            cardinality: u128,
        }
        struct Plan {
            base_index: usize,
            factors: Vec<Factor>,
            strides: Vec<u128>,
            combinations: u128,
        }

        let limits = identity_limits();
        let prepare = |base_index: usize| -> Result<Option<Plan>, InitialStateRefusal> {
            let base = &self.initial_base[base_index];
            let mut factors = Vec::new();
            let mut combinations = 1_u128;
            for (stage_index, stage) in self.initial_stages.iter().enumerate() {
                let holders: Vec<_> = base
                    .state
                    .populations
                    .iter()
                    .flat_map(|(population, objects)| {
                        objects
                            .iter()
                            .filter(|(_, object)| object.type_identity == stage.type_key)
                            .map(move |(key, _)| (population.clone(), key.clone()))
                    })
                    .collect();
                if holders.is_empty() {
                    continue;
                }
                let cardinality = stage.values.len();
                if cardinality == 0 {
                    combinations = 0;
                    continue;
                }
                for (population, object) in holders {
                    factors.push(Factor {
                        stage: stage_index,
                        population,
                        object,
                        cardinality,
                    });
                    combinations = combinations.saturating_mul(cardinality);
                }
            }
            if combinations > 0 {
                let mut strides = vec![1_u128; factors.len()];
                for index in (0..factors.len().saturating_sub(1)).rev() {
                    strides[index] =
                        strides[index + 1].saturating_mul(factors[index + 1].cardinality);
                }
                Ok(Some(Plan {
                    base_index,
                    factors,
                    strides,
                    combinations,
                }))
            } else {
                Ok(None)
            }
        };
        let mut base_index = 0;
        let mut current: Option<(Plan, u128)> = None;
        let mut admitted = std::collections::BTreeSet::<Vec<u8>>::new();
        let mut done = false;
        Box::new(std::iter::from_fn(move || {
            if done {
                return None;
            }
            let exhausted = current
                .as_ref()
                .is_some_and(|(plan, next)| *next >= plan.combinations);
            if current.is_none() || exhausted {
                current = None;
                loop {
                    if base_index >= self.initial_base.len() {
                        return None;
                    }
                    let plan_index = base_index;
                    base_index += 1;
                    match prepare(plan_index) {
                        Ok(Some(plan)) => {
                            current = Some((plan, 0));
                            break;
                        }
                        Ok(None) => continue,
                        Err(refusal) => {
                            done = true;
                            return Some(Err(refusal));
                        }
                    }
                }
            }
            let (plan, index) = current.as_mut().expect("plan was initialized");
            let mut values = Vec::with_capacity(plan.factors.len());
            for (factor, stride) in plan.factors.iter().zip(&plan.strides) {
                let digit = (*index / stride) % factor.cardinality;
                let stage = &self.initial_stages[factor.stage];
                let value = stage
                    .values
                    .nth(digit)
                    .expect("recipe digit is within its initial stage");
                values.push(value.clone());
            }
            *index += 1;
            let base = &self.initial_base[plan.base_index];
            let mut overrides = encode::InitialOverrides::new();
            for (factor, value) in plan.factors.iter().zip(&values) {
                if let Some(object) = base
                    .state
                    .populations
                    .get(&factor.population)
                    .and_then(|objects| objects.get(&factor.object))
                {
                    overrides
                        .entry(object as *const _ as usize)
                        .or_default()
                        .insert(
                            self.initial_stages[factor.stage].field.clone(),
                            value.clone(),
                        );
                }
            }
            let permutation = if let Some(group) = symmetry.filter(|group| !group.is_trivial()) {
                let limits = quire_canonical::Limits::new(identity_limits().input_bytes);
                let mut placeholders = Vec::new();
                for class in &group.classes {
                    let text = match encode::declaration_text(&class.population, limits) {
                        Ok(text) => text,
                        Err(_) => {
                            done = true;
                            return Some(Err(InitialStateRefusal::Expansion(stop(BROKEN))));
                        }
                    };
                    placeholders.push((
                        self.universes_of(&class.population),
                        &class.population,
                        &class.keys,
                        encode::Enc::Object(vec![
                            ("type", encode::Enc::text("symmetric")),
                            ("population", encode::Enc::text(text)),
                        ]),
                    ));
                }
                let placeholder = |reference: &ObjectReference| {
                    placeholders
                        .iter()
                        .find(|(universes, _, keys, _)| {
                            universes.contains(reference.universe())
                                && keys.iter().any(|key| key == reference.object().as_str())
                        })
                        .map(|(_, _, _, enc)| enc.clone())
                };
                let mut permutation = crate::simulation::Permutation::identity();
                for class in &group.classes {
                    let objects = base.state.populations.get(&class.population);
                    let mut sorted = Vec::with_capacity(class.keys.len());
                    for key in &class.keys {
                        let Some(original) = objects.and_then(|objects| objects.get(key)) else {
                            sorted.push((Vec::new(), key));
                            continue;
                        };
                        let bytes = match quire_canonical::to_vec(
                            &encode::initial_object_enc(
                                &self.model,
                                original,
                                self.typing(),
                                &placeholder,
                                &overrides,
                                &Vec::new(),
                            ),
                            limits,
                        ) {
                            Ok(bytes) => bytes,
                            Err(_) => {
                                done = true;
                                return Some(Err(InitialStateRefusal::Expansion(stop(BROKEN))));
                            }
                        };
                        sorted.push((bytes, key));
                    }
                    sorted.sort_by(|left, right| left.0.cmp(&right.0));
                    permutation.insert(
                        &class.population,
                        sorted
                            .into_iter()
                            .zip(&class.keys)
                            .map(|((_, from), to)| (from.clone(), to.clone())),
                    );
                }
                permutation
            } else {
                crate::simulation::Permutation::identity()
            };
            let renaming: encode::InitialRenaming = permutation
                .maps()
                .iter()
                .map(|(population, mapping)| (self.universes_of(population), mapping.clone()))
                .collect();
            let projected_key = key::state_key(
                &encode::initial_model_key(
                    &self.model,
                    &base.state,
                    self.typing(),
                    &overrides,
                    &renaming,
                ),
                limits,
            )
            .map(|(bytes, _)| bytes)
            .map_err(InitialStateRefusal::Encoding);
            let key = match projected_key {
                Ok(key) => key,
                Err(refusal) => {
                    done = true;
                    return Some(Err(refusal));
                }
            };
            if !admitted.contains(&key) {
                if admitted.len() >= max_states {
                    done = true;
                    return Some(Err(InitialStateRefusal::OverBudget(max_states)));
                }
                admitted.insert(key.clone());
            }
            let mut state = base.state.clone();
            for (factor, value) in plan.factors.iter().zip(values) {
                if let Some(object) = state
                    .populations
                    .get_mut(&factor.population)
                    .and_then(|objects| objects.get_mut(&factor.object))
                {
                    object
                        .fields
                        .insert(self.initial_stages[factor.stage].field.clone(), value);
                }
            }
            let generated = base.without_effect(state);
            if cfg!(debug_assertions) {
                let comparison = match symmetry.filter(|group| !group.is_trivial()) {
                    Some(group) => self
                        .canonicalise(&generated, group)
                        .ok()
                        .and_then(|(canonical, _)| self.state_key(&canonical).ok())
                        .map(|(bytes, _)| bytes),
                    _ => self.state_key(&generated).ok().map(|(bytes, _)| bytes),
                };
                debug_assert_eq!(
                    comparison.as_ref(),
                    Some(&key),
                    "recipe projection key agrees"
                );
            }
            super::count_generated_initial_state();
            Some(Ok(generated))
        }))
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
        let evaluation = self
            .evaluator()
            .evaluate_refinement_row(
                row,
                &observed.0.environment,
                bindings,
                &mut Meter::new(self.meter),
            )
            .map_err(|_| stop(BROKEN))?;
        reading(&evaluation.outcome)
    }

    /// Read an admitted package mapping expression against this system's
    /// generated observation, borrowing the already constructed model world.
    ///
    /// # Errors
    /// Returns the same evaluator invariant fault as [`Self::read_row`].
    pub fn read_package_refinement_row(
        &self,
        world: &fsl_package::BareWorld,
        expression: &fsl_package::ReaderProtocolProjection,
        observed: &StateObservation,
        bindings: Vec<Value>,
    ) -> Result<RowReading, ExpansionStop> {
        let evaluation = crate::value::evaluate_bare_refinement_row(
            world,
            expression,
            observed.environment(),
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
        let evaluation = self
            .evaluator()
            .evaluate_history_update(
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
        Ok(self.bounded_domain(value_type, objects)?.iter().collect())
    }

    pub(in crate::simulation) fn bounded_domain(
        &self,
        value_type: &ValueType,
        objects: &[ObjectReference],
    ) -> Result<BoundedDomain, ExpansionStop> {
        let domain = self
            .domain_with_clocks(value_type, objects)
            .ok_or_else(|| exhausted(ExpansionLimit::MaxCandidates))?;
        let cardinality = domain.len();
        if cardinality > u128::from(self.limits.max_candidates) {
            return Err(exhausted(ExpansionLimit::MaxCandidates));
        }
        Ok(BoundedDomain {
            domain,
            cardinality,
        })
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
        self.initial_base = states
            .into_iter()
            .map(|(state, anchor)| ModelSimState::anchored(state, anchor))
            .collect();
        self.initial_stages.clear();
        self.initial_all = std::sync::OnceLock::new();
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
            let domain = self
                .domain_with_clocks(&declared.value_type, &references)
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
            let candidate = ModelSimState::anchored(state, anchor.clone());
            if self.invariant_findings_at(&candidate)?.is_empty() {
                completed.push(candidate.state);
            }
        }
        Ok(completed)
    }
}
