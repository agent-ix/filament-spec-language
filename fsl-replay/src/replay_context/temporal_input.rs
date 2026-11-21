// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-364 metadata retained before admitted native transition reconstruction.

use filament_exact::{Integer, Outcome, Rational};
use fsl_eval::simulation::model::{ModelKeyError, ModelRestoreBudget};
use serde::Deserialize;
use serde_json::Value;

use super::{ContextError, ReferenceContextError, TemporalReadbackError};
use crate::Lasso;
use fsl_eval::simulation::{StateKey, SubjectTransition};

/// Semantic metadata restored against the actual admitted family subject.
pub(crate) struct TemporalMetadata<T: SubjectTransition> {
    pub(crate) over_binding: Option<fsl_eval::model_subject::ModelInstance>,
    pub(crate) fairness: Vec<T::Fairness>,
    pub(crate) kind: crate::CounterexampleKind,
    pub(crate) blocked: Vec<fsl_eval::simulation::protocol::BlockedThread>,
    pub(crate) refinement: Option<crate::refinement::RefinementFailure>,
}

/// Decoded transport bytes awaiting comparison with an actual native state key.
/// This is deliberately not an admitted StateKey constructor.
pub(crate) struct StateKeyInput(Vec<u8>);

impl StateKeyInput {
    pub(crate) fn read(
        value: &Value,
        pointer: &str,
        budget: &mut dyn ModelRestoreBudget,
    ) -> Result<Self, TemporalReadbackError> {
        use base64::Engine;
        let denied = || {
            ReferenceContextError::Codec(crate::reference_codec::ReferenceCodecError::Key {
                source: ModelKeyError::Denied,
                pointer: pointer.to_owned(),
            })
        };
        budget.work(1).map_err(|_| denied())?;
        let object = value
            .as_object()
            .filter(|object| object.len() == 1)
            .ok_or_else(|| {
                ContextError::invalid(pointer, "expected the closed native state-key form")
            })?;
        let text = object
            .get("key")
            .and_then(Value::as_str)
            .ok_or_else(|| ContextError::invalid(pointer, "expected native state-key text"))?;
        let reserve = text
            .len()
            .checked_add(3)
            .map(|n| n / 4)
            .and_then(|n| n.checked_mul(3))
            .ok_or_else(denied)?;
        budget.reserve(reserve).map_err(|_| denied())?;
        budget
            .work(u64::try_from(text.len()).map_err(|_| denied())?)
            .map_err(|_| denied())?;
        budget
            .work(u64::try_from(reserve).map_err(|_| denied())?)
            .map_err(|_| denied())?;
        let mut bytes = Vec::new();
        bytes
            .try_reserve(reserve)
            .map_err(|_| ContextError::Allocation { requested: reserve })?;
        bytes.resize(reserve, 0);
        let used = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode_slice(text, &mut bytes)
            .map_err(|_| ContextError::invalid(pointer, "invalid native state-key base64"))?;
        bytes.truncate(used);
        Ok(Self(bytes))
    }

    pub(crate) fn matches(
        &self,
        native: &StateKey,
        budget: &mut dyn ModelRestoreBudget,
    ) -> Result<bool, TemporalReadbackError> {
        let denied = || {
            ReferenceContextError::Codec(crate::reference_codec::ReferenceCodecError::Key {
                source: ModelKeyError::Denied,
                pointer: "/payload".to_owned(),
            })
        };
        budget
            .work(u64::try_from(self.0.len().max(native.as_bytes().len())).map_err(|_| denied())?)
            .map_err(|_| denied())?;
        Ok(self.0.as_slice() == native.as_bytes())
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Delay {
    numerator: String,
    denominator: String,
}

impl Delay {
    fn admit(
        &self,
        pointer: &str,
        budget: &mut dyn ModelRestoreBudget,
    ) -> Result<Rational, TemporalReadbackError> {
        let stopped = || {
            ReferenceContextError::Codec(crate::reference_codec::ReferenceCodecError::Key {
                source: ModelKeyError::Denied,
                pointer: pointer.to_owned(),
            })
        };
        let mut integer = |text: &str, signed: bool| -> Result<Integer, TemporalReadbackError> {
            let n = text.len();
            budget
                .reserve(n.checked_mul(4).ok_or_else(stopped)?)
                .map_err(|_| stopped())?;
            let work = n
                .checked_mul(n)
                .and_then(|work| u64::try_from(work).ok())
                .ok_or_else(stopped)?;
            budget.work(work).map_err(|_| stopped())?;
            let digits = if signed {
                text.strip_prefix('-').unwrap_or(text)
            } else {
                text
            };
            if digits.is_empty()
                || (digits.len() > 1 && digits.starts_with('0'))
                || text == "-0"
                || !digits.bytes().all(|byte| byte.is_ascii_digit())
            {
                return Err(
                    ContextError::invalid(pointer, "expected canonical delay integer").into(),
                );
            }
            let value: Integer = text
                .parse()
                .map_err(|_| ContextError::invalid(pointer, "invalid delay integer"))?;
            budget.integer(&value).map_err(|_| stopped())?;
            Ok(value)
        };
        let numerator = Rational::from_integer(integer(&self.numerator, true)?);
        let denominator = Rational::from_integer(integer(&self.denominator, false)?);
        match filament_exact::evaluate_rational_arithmetic(
            filament_exact::RationalArithmetic::Divide(&numerator, &denominator),
            None,
            budget.meter(),
        ) {
            Outcome::Completed(value) => Ok(value),
            Outcome::Incomplete(incomplete) => {
                Err(TemporalReadbackError::Accounting { incomplete })
            }
            Outcome::Undefined(_) | Outcome::Refused(_) => {
                Err(ContextError::invalid(pointer, "invalid delay rational").into())
            }
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct StepInput {
    pub(crate) transition: Value,
    pub(crate) post_state: Value,
    delay: Option<Delay>,
    pub(crate) send_record: Option<Value>,
}

impl StepInput {
    pub(crate) fn delay(
        &self,
        pointer: &str,
        budget: &mut dyn ModelRestoreBudget,
    ) -> Result<Option<Rational>, TemporalReadbackError> {
        self.delay
            .as_ref()
            .map(|delay| delay.admit(pointer, budget))
            .transpose()
    }

    pub(crate) fn has_delay(&self) -> bool {
        self.delay.is_some()
    }
}

/// Raw members stay separate from semantic identities. The admitted subject
/// must restore metadata and every transition, then compare the complete
/// owning-family wire output with the supplied document before replay.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct TemporalInput {
    trace: String,
    pub(crate) kind: Value,
    pub(crate) initial_state: usize,
    pub(crate) prefix: Vec<StepInput>,
    #[serde(rename = "loop")]
    pub(crate) looped: Vec<StepInput>,
    pub(crate) over_binding: Value,
    #[serde(rename = "fairness")]
    _fairness: Vec<Value>,
    interval: Value,
    pub(crate) trace_position: Value,
    final_delay: Option<Delay>,
    #[serde(rename = "subject")]
    _subject: Option<Value>,
    #[serde(rename = "max_live_instances")]
    _max_live_instances: Option<u64>,
    #[serde(rename = "blocked")]
    _blocked: Option<Vec<Value>>,
    pub(crate) race: Option<Value>,
    pub(crate) loop_proof: Option<Value>,
    #[serde(rename = "refinement_failure")]
    _refinement_failure: Option<Value>,
}

impl TemporalInput {
    pub(crate) fn read(value: &Value) -> Result<Self, ContextError> {
        let input: Self = serde_json::from_value(value.clone())?;
        if input.trace != "model" || !input.interval.is_null() {
            return Err(ContextError::invalid(
                "/payload/trace",
                "invalid native temporal trace or interval",
            )
            .into());
        }
        for (segment, steps) in [("prefix", &input.prefix), ("loop", &input.looped)] {
            for (index, step) in steps.iter().enumerate() {
                if !step.transition.is_object() || !step.post_state.is_object() {
                    return Err(ContextError::invalid(
                        format!("/payload/{segment}/{index}"),
                        "expected native transition and state objects",
                    ));
                }
                if step.transition.get("type").and_then(Value::as_str) == Some("terminal-stutter")
                    && (segment != "loop"
                        || steps.len() != 1
                        || step.transition.as_object().is_none_or(|map| map.len() != 1)
                        || step.send_record.is_some())
                {
                    return Err(ContextError::invalid(
                        format!("/payload/{segment}/{index}/transition"),
                        "terminal stutter must be the sole loop step",
                    ));
                }
            }
        }
        Ok(input)
    }

    pub(crate) fn is_terminal_stutter(&self) -> bool {
        self.looped.len() == 1 && self.looped[0].transition["type"] == "terminal-stutter"
    }

    pub(crate) fn temporal_trace(
        &self,
        positions: usize,
    ) -> Result<fsl_eval::temporal::TemporalTrace, ContextError> {
        let count = |n| {
            u64::try_from(n).map_err(|_| {
                ContextError::invalid("/payload", "native trace size cannot be represented")
            })
        };
        if self.looped.is_empty() {
            return Ok(fsl_eval::temporal::TemporalTrace::Finite {
                len: count(positions)?,
            });
        }
        let loop_len = if self.is_terminal_stutter() {
            1
        } else {
            positions
                .checked_sub(1)
                .and_then(|steps| steps.checked_sub(self.prefix.len()))
                .ok_or_else(|| {
                    ContextError::invalid("/payload/loop", "native loop entry is not represented")
                })?
        };
        Ok(fsl_eval::temporal::TemporalTrace::Lasso {
            prefix: count(self.prefix.len())?,
            loop_len: count(loop_len)?,
        })
    }

    pub(crate) fn lasso(
        &self,
        budget: &mut dyn ModelRestoreBudget,
    ) -> Result<Option<Lasso>, TemporalReadbackError> {
        if self.looped.is_empty() {
            return Ok(None);
        }
        let stutter = self.is_terminal_stutter();
        let delay = if stutter {
            self.looped[0].delay("/payload/loop/0/delay", budget)?
        } else {
            None
        };
        if delay
            .as_ref()
            .is_some_and(|value| value.is_zero() || value.numerator().is_negative())
        {
            return Err(ContextError::invalid(
                "/payload/loop/0/delay",
                "terminal stutter delay must be positive",
            )
            .into());
        }
        Ok(Some(Lasso {
            entry: self.prefix.len(),
            terminal_stutter: stutter,
            terminal_stutter_delay: delay,
        }))
    }

    pub(crate) fn final_delay(
        &self,
        budget: &mut dyn ModelRestoreBudget,
    ) -> Result<Option<Rational>, TemporalReadbackError> {
        self.final_delay
            .as_ref()
            .map(|delay| delay.admit("/payload/final_delay", budget))
            .transpose()
    }

    /// Finish only after the native builder has uniquely resolved all genuine
    /// transitions, selected bindings and metadata. The owning wire comparison
    /// remains mandatory and is performed by the caller before packet admission.
    pub(crate) fn reconstruct<T: SubjectTransition>(
        &self,
        prefix: Vec<crate::ModelStep<T>>,
        terminal_state: &StateKey,
        metadata: TemporalMetadata<T>,
        budget: &mut dyn ModelRestoreBudget,
    ) -> Result<crate::TemporalCounterexample<T>, TemporalReadbackError> {
        let lasso = self.lasso(budget)?;
        let expected = self
            .prefix
            .len()
            .checked_add(
                if lasso.as_ref().is_some_and(|found| found.terminal_stutter) {
                    0
                } else {
                    self.looped.len()
                },
            )
            .ok_or_else(|| ContextError::invalid("/payload/loop", "step count overflow"))?;
        if prefix.len() != expected {
            return Err(ContextError::invalid(
                "/payload/prefix",
                "native reconstruction omitted or added a step",
            )
            .into());
        }
        if lasso.as_ref().is_some_and(|found| found.terminal_stutter) {
            let expected = StateKeyInput::read(
                &self.looped[0].post_state,
                "/payload/loop/0/post_state",
                budget,
            )?;
            if !expected.matches(terminal_state, budget)? {
                return Err(ContextError::invalid(
                    "/payload/loop/0/post_state",
                    "terminal stutter state differs from the actual admitted prefix",
                )
                .into());
            }
        }
        let payload = crate::TemporalCounterexample {
            initial_state: self.initial_state,
            prefix,
            over_binding: metadata.over_binding,
            fairness: metadata.fairness,
            kind: metadata.kind,
            lasso,
            blocked: metadata.blocked,
            refinement: metadata.refinement,
            final_delay: self.final_delay(budget)?,
        };
        if serde_json::to_value(payload.trace_position()).map_err(ContextError::from)?
            != self.trace_position
        {
            return Err(ContextError::invalid(
                "/payload/trace_position",
                "position differs from the native trace",
            )
            .into());
        }
        Ok(payload)
    }
}
