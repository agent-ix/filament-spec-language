// SPDX-License-Identifier: AGPL-3.0-or-later
//! Hyper carriers borrowing each original trace's runtime-bound converter.

use super::*;
use crate::{ReferenceCodecError, ReferenceScope};

impl ReferenceScope<'_, '_, '_, '_> {
    /// Write one trace under its original quantifier's runtime and owner.
    ///
    /// # Errors
    /// Preserves canonical reference, metadata, resource and cancellation errors.
    pub fn hyper_trace_wire(
        &mut self,
        trace: &crate::HyperTrace,
    ) -> Result<Json, ReferenceCodecError> {
        use crate::{HyperStep, StepMark};
        let mut steps = self.array(trace.steps.len())?;
        for (index, found) in trace.steps.iter().enumerate() {
            let step = self.at_index("/steps", index, |s| {
                let mut out = match found {
                    HyperStep::Model(found) => {
                        let tag = s.text("step")?;
                        let transition = s.at_pointer("/transition", |s| {
                            s.encode_task_transition(&found.transition)
                        })?;
                        let state = s.opaque_key(&found.post_state)?;
                        let mut out = s.record([
                            ("type", tag),
                            ("transition", transition),
                            ("post_state", state),
                        ])?;
                        if let Some(delay) = &found.delay {
                            let delay = s.rational(delay)?;
                            s.put(&mut out, "delay", delay)?;
                        }
                        out
                    }
                    HyperStep::TerminalStutter => {
                        let tag = s.text("terminal-stutter")?;
                        s.record([("type", tag)])?
                    }
                };
                if let Some(mark) = trace.marks.get(index) {
                    let mark = s.text(match mark {
                        StepMark::Skipped => "skipped",
                        StepMark::Visible => "visible",
                    })?;
                    s.put(&mut out, "mark", mark)?;
                }
                Ok::<_, ReferenceCodecError>(out)
            })?;
            steps.push(step);
        }
        let tag = self.text("hyper-trace")?;
        let variable = self.text(&trace.variable)?;
        let alias = self.text(&trace.alias)?;
        let initial = self.display(&trace.initial)?;
        self.record([
            ("type", tag),
            ("variable", variable),
            ("alias", alias),
            ("initial", initial),
            ("steps", Json::Array(steps)),
        ])
    }

    /// Write full hyper metadata with trace/source conversion supplied by the
    /// original owning caller. The metadata scope debits the original source
    /// unit's aggregate owner, never an allowance selected from a trace model.
    ///
    /// # Errors
    /// Preserves metadata errors and the exact owning callback's refusals.
    pub fn hyper_counterexample_wire<E: From<ReferenceCodecError>>(
        &mut self,
        found: &crate::HyperCounterexample,
        claim: Option<&OccurrenceKey>,
        mut trace: impl FnMut(usize, &crate::HyperTrace) -> Result<Json, E>,
        mut guard: impl FnMut(&crate::HyperUndefined, &ModelTransition) -> Result<Json, E>,
    ) -> Result<Json, E> {
        use crate::HyperCounterexampleKind as Kind;
        let mut traces = self.array(found.traces.len())?;
        for (index, found) in found.traces.iter().enumerate() {
            self.work(1)?;
            traces.push(trace(index, found)?);
        }
        let mut instance = self.array(found.instance.len())?;
        for (parameter, key) in &found.instance {
            let parameter = self.text(parameter)?;
            let tag = self.text("string")?;
            let value = self.text(key)?;
            let key = self.record([("type", tag), ("value", value)])?;
            instance.push(self.record([("parameter", parameter), ("key", key)])?);
        }
        let tag = self.text("hyper-counterexample")?;
        let position = self.display(&found.trace_position)?;
        let mut out = self.record([
            ("type", tag),
            ("instance", Json::Array(instance)),
            ("traces", Json::Array(traces)),
            ("trace_position", position),
        ])?;
        let kind = match &found.kind {
            Kind::Lockstep { loop_entry } => {
                let entry = self.display(loop_entry)?;
                self.put(&mut out, "loop_entry", entry)?;
                "lockstep"
            }
            Kind::WitnessExhausted {
                loop_entry,
                position,
            } => {
                let entry = self.display(loop_entry)?;
                let position = self.display(position)?;
                self.put(&mut out, "loop_entry", entry)?;
                self.put(&mut out, "position", position)?;
                "witness-exhausted"
            }
            Kind::Projected => "projected",
            Kind::Closed => "closed",
            Kind::StepTuple => "step-tuple",
            Kind::Undefined(undefined) => {
                let mut variables = self.array(undefined.variables.len())?;
                for (variable, alias) in &undefined.variables {
                    let variable = self.text(variable)?;
                    let alias = self.text(alias)?;
                    variables.push(self.record([("variable", variable), ("alias", alias)])?);
                }
                let mut states = self.array(undefined.state_keys.len())?;
                for key in &undefined.state_keys {
                    states.push(self.opaque_key(key)?);
                }
                let position = self.display(&undefined.position)?;
                let locus = self.at_pointer("/undefined/where/locus", |s| s.claim_locus(claim))?;
                let mut at = self.record([
                    ("position", position),
                    ("state_keys", Json::Array(states)),
                    ("variables", Json::Array(variables)),
                    ("locus", locus),
                ])?;
                if let Some(transition) = &undefined.transition {
                    let transition = guard(undefined, transition)?;
                    self.put(&mut at, "transition", transition)?;
                }
                let tag = self.text("undefined-evaluation")?;
                let cause = self.text(undefined_reason(&undefined.cause))?;
                let undefined = self.record([("type", tag), ("where", at), ("cause", cause)])?;
                self.put(&mut out, "undefined", undefined)?;
                "undefined"
            }
        };
        let kind = self.text(kind)?;
        self.put(&mut out, "kind", kind)?;
        Ok(out)
    }
}
