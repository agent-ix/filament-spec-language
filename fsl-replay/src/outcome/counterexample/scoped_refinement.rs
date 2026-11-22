// SPDX-License-Identifier: AGPL-3.0-or-later
//! Concrete traces and abstract mapped-state/failure output retain both owners.

use super::*;
use crate::{ReferenceCodecError, ReferenceScope};

impl ReferenceScope<'_, '_, '_, '_> {
    /// Write a reproduced refinement while its actual concrete and abstract
    /// runtimes remain borrowed. Mapped-state carriers use the abstract
    /// catalog; trace transitions use the concrete catalog. Native state keys
    /// and the independent refinement replay remain unchanged.
    ///
    /// # Errors
    /// Preserves reference, metadata, resource and cancellation failures.
    #[allow(
        clippy::too_many_arguments,
        reason = "the original two runtimes, scopes and mapped states must remain explicit"
    )]
    pub fn refinement_temporal_wire(
        &mut self,
        counterexample: &TemporalCounterexample,
        refinement: &fsl_semantics::check::CheckedRefinement,
        systems: &crate::RefinementSystems<'_>,
        mapped: &[fsl_semantics::model::state::ModelState],
        owner: &fsl_semantics::model::intake::SelectedModels,
        abstract_references: &mut ReferenceScope<'_, '_, '_, '_>,
    ) -> Result<Json, ReferenceCodecError> {
        self.model_runtime(&systems.concrete)?;
        abstract_references.model_runtime(&systems.abstract_system)?;
        let failure = counterexample
            .refinement
            .as_ref()
            .map(|failure| {
                abstract_references
                    .at_pointer("/refinement_failure", |s| s.refinement_full(failure))
            })
            .transpose()?;
        let terminal = if counterexample.prefix.is_empty()
            && counterexample
                .lasso
                .as_ref()
                .is_some_and(|lasso| lasso.terminal_stutter)
        {
            self.work(1)?;
            let initial = systems
                .concrete
                .initial_states()
                .get(counterexample.initial_state)
                .ok_or_else(|| self.carrier_refusal("refinement initial state"))?;
            Some(self.at_pointer("/loop/0/post_state", |s| {
                s.native_state_key(initial, &systems.concrete, owner)
            })?)
        } else {
            None
        };
        let claim = refinement.requirement_occurrence();
        let mut out = self.temporal_full(
            counterexample,
            None,
            claim,
            terminal.as_ref(),
            false,
            failure,
            |s, transition| s.encode_task_transition(transition),
            |_, _| Ok(None),
            |s, fairness| s.fairness_wire(fairness),
        )?;
        let mut steps = abstract_references.array(mapped.len())?;
        for (position, state) in mapped.iter().enumerate() {
            steps.push(abstract_references.at_index("/steps", position, |s| {
                let state = s.at_pointer("/abstract_state", |s| {
                    s.encode_model_state(state, &systems.abstract_system, owner)
                })?;
                s.record([
                    ("position", Json::from(position)),
                    ("abstract_state", state),
                ])
            })?);
        }
        let mut simulations = self.array(refinement.step_simulations().len())?;
        for record in refinement.step_simulations() {
            let role = self.display(record.occurrence().origin().role())?;
            let ordinal = Json::from(record.occurrence().origin().ordinal());
            simulations.push(self.record([("role", role), ("ordinal", ordinal)])?);
        }
        self.put(&mut out, "steps", Json::Array(steps))?;
        self.put(&mut out, "step_simulations", Json::Array(simulations))?;
        Ok(out)
    }
}
