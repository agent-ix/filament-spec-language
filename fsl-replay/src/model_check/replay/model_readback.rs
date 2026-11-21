// SPDX-License-Identifier: AGPL-3.0-or-later
//! Source-model readback against actual native successors and population binding.

use filament_exact::Cancel;
use filament_semantic_value::model_transition::ModelTransition;
use fsl_eval::model_subject::ModelInstance;
use fsl_eval::simulation::model::{ModelRestoreBudget, ModelSimState, ModelSystem};
use fsl_eval::simulation::{ConstraintValue, SubjectSystem};

use super::protocol_readback::{ClauseInput, ResolvedPrefix};
use crate::reference_codec::ReferenceCodec;
use crate::replay_context::{
    ContextError, ReferenceContextError, TemporalInput, TemporalMetadata, TemporalReadbackError,
};

pub(crate) struct ModelReadback<'a, 'runtime> {
    pub(crate) compiled: &'a crate::execute::Recompiled,
    pub(crate) system: &'a ModelSystem<'runtime>,
    pub(crate) subject: &'a crate::ModelTraceSubject,
    pub(crate) over: Option<&'a ModelInstance>,
    pub(crate) claim: &'a fsl_foundation::source::provenance::OccurrenceKey,
    pub(crate) supplied: &'a serde_json::Value,
}

pub(super) struct ModelMetadata<'a> {
    pub(super) over: Option<ModelInstance>,
    pub(super) node: Option<&'a fsl_semantics::check::TemporalClauseNode>,
    pub(super) letters: Option<&'a dyn fsl_eval::model_subject::ModelLetters>,
    pub(super) trace: Option<&'a fsl_eval::temporal::TimedTrace>,
}

impl ModelReadback<'_, '_> {
    pub(crate) fn admit(
        &self,
        codec: &ReferenceCodec<'_, '_, '_>,
        budget: &mut dyn ModelRestoreBudget,
        cancel: &Cancel,
    ) -> Result<crate::TemporalCounterexample, TemporalReadbackError> {
        match &self.subject.timed {
            Some(reading) => super::timed::readback::admit(self, reading, codec, budget, cancel),
            None => self.untimed(codec, budget, cancel),
        }
    }

    pub(super) fn restore_over(
        &self,
        input: &TemporalInput,
        initial_model_index: usize,
        codec: &ReferenceCodec<'_, '_, '_>,
        budget: &mut dyn ModelRestoreBudget,
        cancel: &Cancel,
    ) -> Result<Option<ModelInstance>, TemporalReadbackError> {
        match (self.over, input.over_binding.is_null()) {
            (None, true) => Ok(None),
            (Some(selected), false) => Ok(Some(
                codec
                    .restore_model_over_binding(
                        &input.over_binding,
                        Some(selected),
                        initial_model_index,
                        budget,
                        cancel,
                        usize::MAX,
                    )
                    .map_err(ReferenceContextError::from)?,
            )),
            _ => Err(ContextError::invalid(
                "/subject/over",
                "original selected instance and payload binding differ",
            )
            .into()),
        }
    }
    pub(crate) fn untimed(
        &self,
        codec: &ReferenceCodec<'_, '_, '_>,
        budget: &mut dyn ModelRestoreBudget,
        cancel: &Cancel,
    ) -> Result<crate::TemporalCounterexample, TemporalReadbackError> {
        let input = TemporalInput::read(self.supplied)?;
        let package = self.compiled.checked.package();
        let node = package.graph().temporal_clauses().iter().find(|node| {
            node.identity() == *self.claim.node() && node.claim() == self.claim.origin()
        });
        let letters = node
            .map(|node| self.system.clause_letters(node))
            .transpose()
            .map_err(|_| {
                ContextError::invalid("/subject", "actual model clause letters could not be built")
            })?;
        let over = self.restore_over(&input, input.initial_state, codec, budget, cancel)?;
        let steps = super::protocol_readback::native_steps(&input, budget)?;
        let resolved = super::protocol_readback::resolve(
            self.system,
            input.initial_state,
            &steps,
            budget,
            |transition, _, _, budget| {
                let mut scope = codec.scope(
                    budget,
                    cancel,
                    usize::MAX,
                    quire_canonical::Limits::new(u64::MAX),
                );
                Ok((scope.encode_task_transition(transition)?, None))
            },
        )?;
        let metadata = self.metadata(
            &input,
            &resolved,
            ModelMetadata {
                over,
                node,
                letters: letters
                    .as_ref()
                    .map(|letters| letters as &dyn fsl_eval::model_subject::ModelLetters),
                trace: None,
            },
            budget,
        )?;
        let terminal = self
            .system
            .canonical_state_key(&resolved.final_state)
            .map_err(ReferenceContextError::from)?;
        let over_reference = metadata.over_binding.as_ref().and_then(|over| {
            letters
                .as_ref()
                .and_then(|letters| letters.instance_reference(over))
        });
        let payload = input.reconstruct(resolved.prefix, &terminal, metadata, budget)?;
        let written = codec
            .scope(
                budget,
                cancel,
                usize::MAX,
                quire_canonical::Limits::new(u64::MAX),
            )
            .temporal_wire(
                &payload,
                over_reference.as_ref(),
                Some(self.claim),
                Some(&terminal),
            )
            .map_err(ReferenceContextError::from)?;
        if &written != self.supplied {
            return Err(ContextError::invalid(
                "/payload",
                "native model reconstruction differs from the complete supplied family wire",
            )
            .into());
        }
        Ok(payload)
    }

    pub(super) fn metadata(
        &self,
        input: &TemporalInput,
        resolved: &ResolvedPrefix<ModelTransition, ModelSimState>,
        metadata: ModelMetadata<'_>,
        budget: &mut dyn ModelRestoreBudget,
    ) -> Result<TemporalMetadata<ModelTransition>, TemporalReadbackError> {
        let ModelMetadata {
            over,
            node,
            letters,
            trace,
        } = metadata;
        let kind = match input.kind.as_str() {
            Some("formula") => crate::CounterexampleKind::Formula,
            Some("deadlock") => crate::CounterexampleKind::Deadlock,
            Some("time-lock") if self.subject.timed.is_some() => {
                crate::CounterexampleKind::TimeLock
            }
            Some("time-lock") => {
                return Err(super::refused(super::ModelTraceRefusal::TimeLockUntimed).into())
            }
            Some("race") => {
                return Err(super::refused(super::ModelTraceRefusal::RaceOverModel).into())
            }
            None if input.kind.get("type").and_then(serde_json::Value::as_str)
                == Some("undefined-evaluation") =>
            {
                let package = self.compiled.checked.package();
                let invariant = package.graph().state_clauses().iter().find(|clause| {
                    clause.is_invariant()
                        && clause.identity() == *self.claim.node()
                        && clause.claim() == self.claim.origin()
                });
                let failure = super::protocol_readback::undefined_metadata(
                    self.system,
                    input,
                    resolved,
                    ClauseInput {
                        over: over.as_ref(),
                        node,
                        letters,
                        trace,
                    },
                    budget,
                    |state| {
                        for clause in package.graph().state_clauses() {
                            if invariant
                                .is_none_or(|selected| selected.identity() != clause.identity())
                                && !self.system.has_constraint(clause.name())
                            {
                                continue;
                            }
                            if let ConstraintValue::Undefined { cause, .. } = self
                                .system
                                .constraint_value(state, clause.name())
                                .map_err(|stop| TemporalReadbackError::Expansion { stop })?
                            {
                                let locus = if invariant.is_some_and(|selected| {
                                    selected.identity() == clause.identity()
                                }) {
                                    crate::UndefinedLocus::Invariant(clause.name().to_owned())
                                } else {
                                    crate::UndefinedLocus::Constraint(clause.name().to_owned())
                                };
                                return Ok(Some((locus, cause)));
                            }
                        }
                        Ok(None)
                    },
                )?;
                crate::CounterexampleKind::UndefinedEvaluation(failure)
            }
            _ => {
                return Err(ContextError::invalid(
                    "/payload/kind",
                    "unknown or inapplicable native model kind",
                )
                .into())
            }
        };
        Ok(TemporalMetadata {
            over_binding: over,
            fairness: node
                .map(|node| node.clause().fairness().to_vec())
                .unwrap_or_default(),
            kind,
            blocked: Vec::new(),
            refinement: None,
        })
    }
}
