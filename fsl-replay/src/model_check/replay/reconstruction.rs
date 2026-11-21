// SPDX-License-Identifier: AGPL-3.0-or-later
//! Native temporal replay over a freshly reconstructed genuine source graph.

use filament_exact::ScalarLimits;
use fsl_eval::simulation::model::ModelSystem;

use super::{
    invariant, resolve_invariant, resolve_item, settle, timed, DerivedAnchors, FormulaReplay,
    ModelTraceRefusal, ModelTraceReplayResult, ModelTraceSubject,
};
use crate::execute::Recompiled;
use crate::{ReplayRefusal, ReplayRequest, TemporalCounterexample, WitnessEnvelope};

/// Derived protocol items have no authored atom or activation to evaluate.
pub(crate) struct NoLetters;

impl fsl_eval::model_subject::ModelLetters for NoLetters {
    fn atom(
        &self,
        _: fsl_semantics::check::FormulaNodeId,
        _: &fsl_eval::model_subject::ModelInstance,
        _: &fsl_eval::simulation::model::ModelSimState,
        _: &mut filament_exact::Meter,
    ) -> Result<fsl_eval::temporal::AtomReading, fsl_eval::model_subject::AtomStop> {
        Err(fsl_eval::model_subject::AtomStop::Fault(
            filament_semantic_value::catalog::InternalFault::new(
                "replay-context",
                "a-derived-item-has-no-authored-atom",
            ),
        ))
    }
    fn activation(
        &self,
        _: &fsl_eval::model_subject::ModelInstance,
        _: &fsl_eval::simulation::model::ModelSimState,
        _: &mut filament_exact::Meter,
    ) -> Result<fsl_eval::temporal::AtomReading, fsl_eval::model_subject::AtomStop> {
        Err(fsl_eval::model_subject::AtomStop::Fault(
            filament_semantic_value::catalog::InternalFault::new(
                "replay-context",
                "a-derived-item-has-no-authored-activation",
            ),
        ))
    }
    fn activation_reads(
        &self,
    ) -> Result<
        std::collections::BTreeSet<fsl_semantics::check::footprint::SymbolicLocation>,
        fsl_semantics::check::footprint::FootprintRefusal,
    > {
        Ok(Default::default())
    }
    fn reads(
        &self,
        _: fsl_semantics::check::FormulaNodeId,
    ) -> Result<
        std::collections::BTreeSet<fsl_semantics::check::footprint::SymbolicLocation>,
        fsl_semantics::check::footprint::FootprintRefusal,
    > {
        Ok(Default::default())
    }
}

/// Re-evaluate an admitted protocol payload using the existing native protocol
/// settlement, without a second compiler or rewriting the proving envelope.
pub(crate) fn protocol<M: fsl_eval::simulation::protocol::MemoryModel>(
    compiled: &Recompiled,
    subject: &super::ProtocolTraceSubject,
    system: &fsl_eval::simulation::protocol::ProtocolSystem<'_, M>,
    envelope: &WitnessEnvelope<
        TemporalCounterexample<fsl_eval::simulation::protocol::ProtocolTransition>,
    >,
    letters: &dyn fsl_eval::model_subject::ModelLetters,
    meter: ScalarLimits,
) -> Result<ModelTraceReplayResult, ReplayRefusal> {
    let package = compiled.checked.package();
    let clause = resolve_item(
        envelope,
        package,
        DerivedAnchors::Protocol(&subject.protocol),
        |_| Ok(()),
    )?;
    let settlement = super::ProtocolReplay {
        terminal_key: None,
        payload: envelope.family_payload(),
        clause,
        letters,
        meter,
    }
    .settle(system)?;
    Ok(ModelTraceReplayResult {
        settlement,
        hints: Vec::new(),
        terminals: package.graph().terminal_keys(),
    })
}

pub(crate) fn model(
    compiled: &Recompiled,
    original: &ReplayRequest,
    system: &ModelSystem<'_>,
    subject: &ModelTraceSubject,
    envelope: &WitnessEnvelope<TemporalCounterexample>,
    meter: ScalarLimits,
) -> Result<ModelTraceReplayResult, ReplayRefusal> {
    let package = compiled.checked.package();
    if let Some(clause) = resolve_invariant(envelope, package)? {
        let settlement = invariant(system, envelope.family_payload(), clause)?;
        return Ok(ModelTraceReplayResult {
            settlement,
            hints: Vec::new(),
            terminals: package.graph().terminal_keys(),
        });
    }
    let clause = resolve_item(envelope, package, DerivedAnchors::Model, |clause| {
        if envelope.family_payload().fairness == clause.fairness() {
            return Ok(());
        }
        Err(super::refused(ModelTraceRefusal::FairnessSet {
            payload: envelope.family_payload().fairness.clone(),
            recompiled: clause.fairness().to_vec(),
        }))
    })?;
    let replay = |formula| match &subject.timed {
        Some(reading) => timed::settle(
            &timed::timed_subject(system, &reading.model, reading.max_clocks)?,
            envelope.family_payload(),
            formula,
            &reading.exploration,
        ),
        None => settle(system, envelope.family_payload(), formula),
    };
    let (settlement, hints) = match clause {
        None => replay(None)?,
        Some(clause) => {
            let node = package
                .graph()
                .temporal_clauses()
                .iter()
                .find(|node| {
                    node.identity() == *envelope.clause_node()
                        && node.claim() == envelope.occurrence_key().origin()
                })
                .ok_or_else(|| super::fault("reconstructed-checked-clause-exists"))?;
            if original
                .replay_target()
                .and_then(crate::ReplayTarget::as_state_clause)
                .is_none_or(|target| target.to_string() != clause.name())
            {
                return Err(super::refused(ModelTraceRefusal::UnknownTemporalClause {
                    clause_node: envelope.clause_node().clone(),
                }));
            }
            let letters = system
                .clause_letters(node)
                .map_err(|_| super::fault("reconstructed-clause-letters"))?;
            replay(Some(FormulaReplay {
                clause,
                letters: &letters,
                meter,
            }))?
        }
    };
    Ok(ModelTraceReplayResult {
        settlement,
        hints,
        terminals: package.graph().terminal_keys(),
    })
}
