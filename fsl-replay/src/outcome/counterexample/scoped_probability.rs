// SPDX-License-Identifier: AGPL-3.0-or-later
//! Probability carriers constructed under the original source work owner.

use super::*;
use crate::{ReferenceCodecError, ReferenceScope};

impl ReferenceScope<'_, '_, '_, '_> {
    /// Project an independently reproduced undefined probability path using
    /// the same source owner, reference codec and charged metadata writer.
    ///
    /// # Errors
    /// Preserves malformed path, reference, resource and cancellation failures.
    pub fn probabilistic_undefined_wire(
        &mut self,
        witness: &crate::ProbabilityUndefinedWitness,
        claim: &OccurrenceKey,
        bound: &fsl_semantics::check::Bound,
        system: &fsl_eval::simulation::model::ModelSystem<'_>,
        owner: &fsl_semantics::model::intake::SelectedModels,
        scheduler: &[(
            fsl_eval::simulation::StateKey,
            fsl_eval::simulation::model::ScheduledIdentity,
        )],
    ) -> Result<Json, ReferenceCodecError> {
        let mut out = self.probabilistic_wire(&witness.path, bound, system, owner, scheduler)?;
        let paths = out
            .get_mut("evidence")
            .and_then(|value| value.get_mut("paths"))
            .and_then(Json::as_array_mut)
            .ok_or_else(|| self.carrier_refusal("undefined probability path"))?;
        let [path] = paths.as_mut_slice() else {
            return Err(self.carrier_refusal("one undefined probability path"));
        };
        let path = std::mem::take(path);
        let mut evaluation = self.at_pointer("/evidence/undefined", |s| {
            s.undefined_full(&witness.evaluation, Some(claim))
        })?;
        let key = self.at_pointer("/evidence/undefined/where/product_state", |s| {
            let key = s.opaque_bytes(witness.product_state.as_bytes())?;
            s.record([("key", key)])
        })?;
        let at = evaluation
            .get_mut("where")
            .ok_or_else(|| self.carrier_refusal("undefined probability locus"))?;
        self.put(at, "product_state", key)?;
        let tag = self.text("undefined-path")?;
        let evidence = self.record([("type", tag), ("path", path), ("undefined", evaluation)])?;
        self.put(&mut out, "evidence", evidence)?;
        Ok(out)
    }

    /// Project reproduced probability evidence without unbounded native value
    /// trees. `owner` is the original selected-source admission; each borrowed
    /// value writer debits that same cumulative owner.
    ///
    /// # Errors
    /// Preserves typed wire, reference, resource and cancellation failures.
    pub fn probabilistic_wire(
        &mut self,
        witness: &crate::ProbabilisticCounterexample,
        bound: &fsl_semantics::check::Bound,
        system: &fsl_eval::simulation::model::ModelSystem<'_>,
        owner: &fsl_semantics::model::intake::SelectedModels,
        scheduler: &[(
            fsl_eval::simulation::StateKey,
            fsl_eval::simulation::model::ScheduledIdentity,
        )],
    ) -> Result<Json, ReferenceCodecError> {
        use fsl_eval::simulation::protocol::ProtocolWireSink;
        self.model_runtime(system)?;
        let package = system
            .source_package()
            .ok_or_else(|| self.carrier_refusal("probabilistic source package"))?;
        let paths = self.at_pointer("/evidence/paths", |s| {
            let mut paths = Json::Array(ReferenceScope::array(s, witness.paths.len())?);
            for (index, path) in witness.paths.iter().enumerate() {
                s.element(&mut paths, index, |s| {
                    let mut steps = Json::Array(ReferenceScope::array(s, path.steps.len())?);
                    for (index, step) in path.steps.iter().enumerate() {
                        s.element(&mut steps, index, |s| {
                            let mut out = s.record([])?;
                            s.field(&mut out, "transition", |s| match &step.transition {
                                Some(transition) => s.encode_transition(transition),
                                None => Ok(Json::Null),
                            })?;
                            s.field(&mut out, "post_state", |s| {
                                s.native_state(&step.post, system, owner)
                            })?;
                            s.field(&mut out, "random", |s| match &step.draw {
                                None => Ok(Json::Null),
                                Some(draw) => {
                                    let mut out =
                                        Json::Array(ReferenceScope::array(s, draw.len())?);
                                    for (index, value) in draw.iter().enumerate() {
                                        s.element(&mut out, index, |s| {
                                            s.native_value(value, package, owner)
                                        })?;
                                    }
                                    Ok(out)
                                }
                            })?;
                            s.work(1)?;
                            if step.elapsed
                                != Rational::from_integer(filament_exact::Integer::from(0_i64))
                            {
                                s.field(&mut out, "delay", |s| s.rational(&step.elapsed))?;
                            }
                            Ok(out)
                        })?;
                    }
                    Ok(steps)
                })?;
            }
            Ok(paths)
        })?;
        let (comparison, threshold) = match bound {
            fsl_semantics::check::Bound::AtLeast(threshold) => (">=", threshold),
            fsl_semantics::check::Bound::AtMost(threshold) => ("<=", threshold),
        };
        let mut out = self.record([])?;
        self.field(&mut out, "type", |s| s.text("probabilistic"))?;
        self.put(&mut out, "initial_state", Json::from(0))?;
        self.field(&mut out, "binding", |s| match &witness.binding {
            Some(binding) => s.encode_reference(binding),
            None => Ok(Json::Null),
        })?;
        self.field(&mut out, "bound", |s| {
            let comparison = s.text(comparison)?;
            let threshold = s.rational(threshold)?;
            s.record([("comparison", comparison), ("threshold", threshold)])
        })?;
        self.field(&mut out, "evidence", |s| {
            if let Some(subsystem) = &witness.subsystem {
                if !witness.paths.is_empty() {
                    return Err(s.carrier_refusal("mixed probabilistic evidence"));
                }
                let certificate = s.native_json(subsystem.as_bytes())?;
                let mut certificates = s.array(1)?;
                certificates.push(certificate);
                let tag = s.text("subsystem")?;
                s.record([("type", tag), ("certificates", Json::Array(certificates))])
            } else {
                let tag = s.text("path-set")?;
                s.record([("type", tag), ("paths", paths)])
            }
        })?;
        if witness.subsystem.is_some() {
            self.put(&mut out, "instance", Json::Null)?;
        }
        let mut entries = Json::Array(ReferenceScope::array(self, scheduler.len())?);
        for (index, (at, choice)) in scheduler.iter().enumerate() {
            self.element(&mut entries, index, |s| {
                let mut entry = s.record([])?;
                s.field(&mut entry, "state", |s| s.state(at))?;
                s.field(&mut entry, "choice", |s| {
                    let mut out = s.record([])?;
                    s.field(&mut out, "type", |s| s.text("transition"))?;
                    s.field(&mut out, "operation", |s| {
                        s.native_json(choice.operation.as_bytes())
                    })?;
                    s.field(&mut out, "receiver", |s| {
                        s.encode_reference(&choice.receiver)
                    })?;
                    s.field(&mut out, "arguments", |s| {
                        let mut out =
                            Json::Array(ReferenceScope::array(s, choice.arguments.len())?);
                        for (index, argument) in choice.arguments.iter().enumerate() {
                            s.element(&mut out, index, |s| {
                                s.native_value(argument, package, owner)
                            })?;
                        }
                        Ok(out)
                    })?;
                    Ok(out)
                })?;
                Ok(entry)
            })?;
        }
        for (at, choice) in &witness.scheduler {
            self.work(1)?;
            let tag = match choice {
                crate::ProductAction::PostState(_) => "post-state",
                crate::ProductAction::TerminalStutter => "terminal-stutter",
                crate::ProductAction::Identity(_)
                | crate::ProductAction::UnitDelay
                | crate::ProductAction::EnterIdleDelay
                | crate::ProductAction::IdleDelay
                | crate::ProductAction::IdleObserve
                | crate::ProductAction::WorkloadResolve { .. }
                | crate::ProductAction::FreeDelays(_) => continue,
            };
            let mut action = self.array(1)?;
            action.push(self.text(tag)?);
            if let crate::ProductAction::PostState(key) = choice {
                let key = self.native_json(key.as_bytes())?;
                self.append(&mut action, key)?;
            }
            let key = self.opaque_bytes(at.as_bytes())?;
            let key = self.record([("key", key)])?;
            let entry = self.record([("state", key), ("choice", Json::Array(action))])?;
            let Json::Array(entries) = &mut entries else {
                return Err(self.carrier_refusal("scheduler entries"));
            };
            self.append(entries, entry)?;
        }
        if !scheduler.is_empty()
            || entries
                .as_array()
                .is_some_and(|entries| !entries.is_empty())
        {
            let scheduler = self.record([("entries", entries)])?;
            self.put(&mut out, "scheduler", scheduler)?;
        }
        Ok(out)
    }
}
