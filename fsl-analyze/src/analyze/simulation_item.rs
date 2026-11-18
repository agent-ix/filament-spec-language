// SPDX-License-Identifier: AGPL-3.0-or-later
//! The `simulation` item of a `native-analyze/1` request (FR-634): an
//! enumeration or a seeded sampling of the subject's nondeterministic
//! behaviour through FR-120's simulator, within explicit bounds (FSpec
//! FR-181). It settles no claim and is not negotiated: it runs in process,
//! and its item document carries the traces, the coverage and the
//! unexplored frontier in place of a record.

use std::num::NonZeroU64;

use filament_exact::{length_amount, Cancel, Charge, ChargePoint, Incomplete, Meter};
use filament_semantic_value::catalog::Category;
use fsl_eval::simulation::model::{enumerate_model, sample_model_runs, ModelSystem, SampledRuns};
use fsl_eval::simulation::{
    Coverage, EnumerationBounds, EnumerationEnd, NotSimulated, StateKey, BOUND_POSITIONS,
};
use fsl_foundation::selection::DefinitionRef;
use fsl_foundation::RequestIndex;
use fsl_package::CheckedPackage;
use fsl_semantics::model::observation::AdmissionFailure;
use serde::Deserialize;
use serde_json::{json, Map, Value};

use super::{AnalyzeLimits, SubjectBinding};

/// FR-634 `simulation.mode`.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum SimulationMode {
    /// Enumerate the bounded behaviours breadth-first.
    Enumerate,
    /// Draw seeded samples.
    Sample,
}

/// FR-634's `simulation` member: the mode and its bounds, each a positive
/// count. `seed` and `samples` belong to `sample` only.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SimulationSelection {
    /// `enumerate` or `sample`.
    pub mode: SimulationMode,
    /// The sampling seed.
    #[serde(default)]
    pub seed: Option<u64>,
    /// How many traces a `sample` item draws.
    #[serde(default, deserialize_with = "read_samples")]
    pub samples: Option<NonZeroU64>,
    /// The step bound: one trace's length for `sample`, the search horizon
    /// for `enumerate`.
    #[serde(default, deserialize_with = "read_max_steps")]
    pub max_steps: Option<NonZeroU64>,
    /// The most distinct states the run visits.
    #[serde(default, deserialize_with = "read_max_states")]
    pub max_states: Option<NonZeroU64>,
    /// The longest canonical state key, in octets, the run expands.
    #[serde(default, deserialize_with = "read_max_state_key_bytes")]
    pub max_state_key_bytes: Option<NonZeroU64>,
}

/// Read a positive count member named `member`: a JSON integer from 1 to
/// 2^53 - 1. Zero, or a larger value, refuses the whole request naming the
/// member (FR-634: every bound is a positive count).
fn positive<'de, D: serde::Deserializer<'de>>(
    decoder: D,
    member: &str,
) -> Result<Option<NonZeroU64>, D::Error> {
    const MAX_COUNT: u64 = 9_007_199_254_740_991;
    let value = u64::deserialize(decoder)?;
    if value > MAX_COUNT {
        return Err(serde::de::Error::custom(format!(
            "{member} {value} is not a count from 1 to {MAX_COUNT}"
        )));
    }
    NonZeroU64::new(value).map(Some).ok_or_else(|| {
        serde::de::Error::custom(format!("{member} 0 is not a count from 1 to {MAX_COUNT}"))
    })
}

fn read_samples<'de, D: serde::Deserializer<'de>>(
    decoder: D,
) -> Result<Option<NonZeroU64>, D::Error> {
    positive(decoder, "simulation.samples")
}

fn read_max_steps<'de, D: serde::Deserializer<'de>>(
    decoder: D,
) -> Result<Option<NonZeroU64>, D::Error> {
    positive(decoder, "simulation.max_steps")
}

fn read_max_states<'de, D: serde::Deserializer<'de>>(
    decoder: D,
) -> Result<Option<NonZeroU64>, D::Error> {
    positive(decoder, "simulation.max_states")
}

fn read_max_state_key_bytes<'de, D: serde::Deserializer<'de>>(
    decoder: D,
) -> Result<Option<NonZeroU64>, D::Error> {
    positive(decoder, "simulation.max_state_key_bytes")
}

/// The published defaults the simulator applies to a bound the item leaves
/// unset: the exploration engine's `explore.states` (FR-255), the sampler's
/// step budget, one sample, and the identity limit on one canonical key.
const DEFAULT_MAX_STATES: u64 = 10_000_000;
const DEFAULT_MAX_STEPS: u64 = 16_777_216;
const DEFAULT_SAMPLES: u64 = 1;

impl SimulationSelection {
    /// The member a `sample`-only setting names on an `enumerate` item, if
    /// any (FR-634 "Refusals").
    pub fn misplaced_member(&self) -> Option<&'static str> {
        match self.mode {
            SimulationMode::Enumerate if self.seed.is_some() => Some("seed"),
            SimulationMode::Enumerate if self.samples.is_some() => Some("samples"),
            SimulationMode::Enumerate | SimulationMode::Sample => None,
        }
    }

    fn max_states(&self) -> usize {
        to_usize(self.max_states.map_or(DEFAULT_MAX_STATES, NonZeroU64::get))
    }

    fn max_steps(&self) -> usize {
        to_usize(self.max_steps.map_or(DEFAULT_MAX_STEPS, NonZeroU64::get))
    }

    fn max_state_key_bytes(&self) -> u64 {
        self.max_state_key_bytes.map_or(
            fsl_foundation::IdentityLimits::default().input_bytes,
            NonZeroU64::get,
        )
    }

    /// The simulation member as the item echoes it.
    fn echo(&self) -> Value {
        let mut members = Map::new();
        members.insert(
            "mode".to_owned(),
            json!(match self.mode {
                SimulationMode::Enumerate => "enumerate",
                SimulationMode::Sample => "sample",
            }),
        );
        if self.mode == SimulationMode::Sample {
            // FSpec FR-181-AC-32: a `sample` run without a seed reports seed 0.
            let seed = self.seed.unwrap_or_default();
            members.insert("seed".to_owned(), json!(seed.to_string()));
            members.insert("samples".to_owned(), json!(self.samples()));
        }
        members.insert("max_steps".to_owned(), json!(self.max_steps()));
        members.insert("max_states".to_owned(), json!(self.max_states()));
        members.insert(
            "max_state_key_bytes".to_owned(),
            json!(self.max_state_key_bytes()),
        );
        Value::Object(members)
    }

    fn samples(&self) -> u64 {
        self.samples.map_or(DEFAULT_SAMPLES, NonZeroU64::get)
    }
}

fn to_usize(value: u64) -> usize {
    usize::try_from(value).unwrap_or(usize::MAX)
}

/// One resolved simulation item: its request index and its settings.
#[derive(Clone, Debug)]
pub struct SimulationItem {
    /// The item's position in the request.
    pub index: RequestIndex,
    /// The item's `simulation` member.
    pub settings: SimulationSelection,
    /// The item's own subject; the request's when absent.
    pub subject: Option<super::document::SubjectSelection>,
}

/// Why a simulation item has no document.
pub(super) enum SimulationFault {
    /// The subject does not admit: the whole request refuses (FR-624).
    Binding(AdmissionFailure),
    /// An internal invariant broke.
    Fault,
}

/// The `{category, cause}` of a run that ended at `end`.
fn ending(end: &EnumerationEnd, settings: &SimulationSelection) -> (Category, Option<Value>) {
    let limit = |limit: &str, value: u64| {
        Some(json!({"kind": "limit-reached", "limit": limit, "value": value}))
    };
    match end {
        EnumerationEnd::Exhausted => (Category::Success, None),
        EnumerationEnd::MaxStates => (
            Category::Incomplete,
            limit(
                "max_states",
                settings
                    .max_states
                    .map_or(DEFAULT_MAX_STATES, NonZeroU64::get),
            ),
        ),
        EnumerationEnd::MaxSteps => (
            Category::Incomplete,
            limit(
                "max_steps",
                settings
                    .max_steps
                    .map_or(DEFAULT_MAX_STEPS, NonZeroU64::get),
            ),
        ),
        EnumerationEnd::StateKeyLimit => (
            Category::Incomplete,
            Some(
                json!({"kind": "state-key-limit-exceeded", "limit": "max_state_key_bytes",
                "value": settings.max_state_key_bytes()}),
            ),
        ),
        EnumerationEnd::Meter(incomplete) => (Category::Incomplete, Some(meter_cause(incomplete))),
        EnumerationEnd::Cancelled => (
            Category::Incomplete,
            Some(json!({"kind": "cancelled", "code": "cancelled"})),
        ),
        EnumerationEnd::Stopped(cause) => (
            fsl_foundation::diagnostic::category_of(cause).unwrap_or(Category::InternalFailure),
            Some(json!({"code": cause.code(), "kind": cause.cause()})),
        ),
    }
}

/// A state's FSpec FR-181 typed canonical form, read back from its key.
fn state_value(key: &StateKey) -> Result<Value, SimulationFault> {
    serde_json::from_slice(key.as_bytes()).map_err(|_| SimulationFault::Fault)
}

/// A transition identity's typed canonical form.
fn transition_value(
    transition: &filament_semantic_value::model_transition::ModelTransition,
) -> Result<Value, SimulationFault> {
    let bytes = quire_canonical::to_vec(transition, quire_canonical::Limits::new(u64::MAX))
        .map_err(|_| SimulationFault::Fault)?;
    serde_json::from_slice(&bytes).map_err(|_| SimulationFault::Fault)
}

fn coverage_value(coverage: &Coverage) -> Value {
    json!({
        "states": coverage.states.to_string(),
        "transitions": coverage.transitions.to_string(),
        "choices": coverage.choices.to_string(),
    })
}

/// FR-634: run `item` over the binding and write its item document.
pub(super) fn simulate(
    package: &CheckedPackage,
    binding: &SubjectBinding<'_>,
    limits: &AnalyzeLimits,
    item: &SimulationItem,
    cancel: &Cancel,
) -> Result<(Value, Category), SimulationFault> {
    let own = item
        .subject
        .as_ref()
        .map(|subject| subject.binding(binding.packages, binding.snapshots));
    let binding = own.as_ref().unwrap_or(binding);
    let system = ModelSystem::new(super::request::simulation(package, binding, limits))
        .map_err(SimulationFault::Binding)?;
    let settings = &item.settings;
    let mut document = Map::new();
    document.insert("request_index".to_owned(), json!(item.index.get()));
    document.insert("kind".to_owned(), json!("simulation"));
    document.insert("simulation".to_owned(), settings.echo());
    let category = match settings.mode {
        SimulationMode::Enumerate => enumerate(&system, settings, limits, cancel, &mut document)?,
        SimulationMode::Sample => sample(&system, settings, limits, cancel, &mut document)?,
    };
    document.insert("category".to_owned(), json!(category.as_str()));
    Ok((Value::Object(document), category))
}

/// A refusal of the requires-bound pre-check or an encoding: the run's
/// `{category, cause}`.
fn not_simulated(refused: &NotSimulated) -> (Category, Value) {
    match refused.catalog_code() {
        Some(code) => (
            fsl_foundation::diagnostic::category_of(&code).unwrap_or(Category::Refusal),
            json!({"code": code.code(), "kind": code.cause()}),
        ),
        // FR-101: an unbounded root, or an unbound capacity, requires a bound.
        None => (
            Category::Refusal,
            json!({"code": "requires_bound", "kind": "requires-bound"}),
        ),
    }
}

/// The FR-109 `limit-reached` cause of a refused charge, naming the
/// counter, its limit and the charge point.
fn meter_cause(incomplete: &Incomplete) -> Value {
    json!({"kind": "limit-reached", "limit": incomplete.limit_kind.as_str(),
        "value": incomplete.limit, "charge_point": incomplete.charge_point.as_str()})
}

/// The bound on what one item writes: every emitted trace step is charged
/// to the item's FR-109 meter before the trace is built, and the written
/// traces' bytes are held under the request's `emit.output_bytes`.
struct Emission<'m> {
    meter: &'m mut Meter,
    written: u64,
    limit: u64,
}

impl Emission<'_> {
    /// Admit a trace of `positions` states before it is built.
    fn admit(&mut self, positions: usize) -> Result<(), Value> {
        self.meter
            .charge(Charge::new(ChargePoint::GraphResultRetain).results(length_amount(positions)))
            .map_err(|incomplete| meter_cause(&incomplete))
    }

    /// Account for the built trace `trace`; refuse it when the item's
    /// written bytes would pass `emit.output_bytes`.
    fn account(&mut self, trace: &Value) -> Result<(), Value> {
        let bytes = u64::try_from(trace.to_string().len()).unwrap_or(u64::MAX);
        let written = self.written.saturating_add(bytes);
        if written > self.limit {
            return Err(
                json!({"kind": "limit-reached", "limit": "emit.output_bytes",
                "value": self.limit}),
            );
        }
        self.written = written;
        Ok(())
    }
}

fn enumerate(
    system: &ModelSystem<'_>,
    settings: &SimulationSelection,
    limits: &AnalyzeLimits,
    cancel: &Cancel,
    document: &mut Map<String, Value>,
) -> Result<Category, SimulationFault> {
    let bounds = EnumerationBounds {
        max_states: settings.max_states(),
        max_steps: settings.max_steps(),
        max_state_key_bytes: settings.max_state_key_bytes(),
    };
    let mut meter = Meter::new(limits.accounting);
    let run = match enumerate_model(system, BOUND_POSITIONS, bounds, &mut meter, || {
        cancel.poll()
    }) {
        Ok(run) => run,
        Err(refused) => {
            let (category, cause) = not_simulated(&refused);
            document.insert("cause".to_owned(), cause);
            document.insert("traces".to_owned(), json!([]));
            document.insert("coverage".to_owned(), coverage_value(&Coverage::default()));
            document.insert("frontier".to_owned(), json!([]));
            return Ok(category);
        }
    };
    let position = |at: usize| -> Result<Value, SimulationFault> {
        let state = run.states.get(at).ok_or(SimulationFault::Fault)?;
        if state.oversized {
            Ok(json!({"position": at}))
        } else {
            state_value(&state.key)
        }
    };
    let mut emission = Emission {
        meter: &mut meter,
        written: 0,
        limit: limits.emit.output_bytes,
    };
    let mut traces = Vec::new();
    let mut cut = None;
    for leaf in run.leaves() {
        let depth = run.states.get(leaf).map_or(0, |state| state.depth);
        if let Err(cause) = emission.admit(depth.saturating_add(1)) {
            cut = Some(cause);
            break;
        }
        let (positions, transitions) = run.path(leaf);
        let states = positions
            .iter()
            .map(|at| position(*at))
            .collect::<Result<Vec<_>, _>>()?;
        let choices = transitions
            .iter()
            .map(|transition| Ok(json!({"transition": transition_value(transition)?})))
            .collect::<Result<Vec<_>, SimulationFault>>()?;
        let trace = json!({"states": states, "steps": choices});
        if let Err(cause) = emission.account(&trace) {
            cut = Some(cause);
            break;
        }
        traces.push(trace);
    }
    let frontier = run
        .frontier
        .iter()
        .map(|at| {
            let state = run.states.get(*at).ok_or(SimulationFault::Fault)?;
            Ok(if state.oversized {
                json!({"position": at})
            } else {
                json!({"state_key": base64url(&state.key)})
            })
        })
        .collect::<Result<Vec<_>, SimulationFault>>()?;
    let (category, cause) = match cut {
        // A cut list of traces leaves the run incomplete whatever else
        // stopped it.
        Some(cause) => (Category::Incomplete, Some(cause)),
        None => ending(&run.end, settings),
    };
    if let Some(cause) = cause {
        document.insert("cause".to_owned(), cause);
    }
    document.insert("traces".to_owned(), Value::Array(traces));
    document.insert("coverage".to_owned(), coverage_value(&run.coverage));
    document.insert("frontier".to_owned(), Value::Array(frontier));
    Ok(category)
}

/// The unpadded base64url spelling of a state key, FR-621's spelling, which
/// the document writer reads back into the state it names.
fn base64url(key: &StateKey) -> String {
    use base64::Engine;
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(key.as_bytes())
}

fn sample(
    system: &ModelSystem<'_>,
    settings: &SimulationSelection,
    limits: &AnalyzeLimits,
    cancel: &Cancel,
    document: &mut Map<String, Value>,
) -> Result<Category, SimulationFault> {
    let seed = settings.seed.unwrap_or_default();
    document.insert("seed".to_owned(), json!(seed.to_string()));
    let sampler = DefinitionRef::new("filament", "filament.simulation.sampler/v1")
        .map_err(|_| SimulationFault::Fault)?;
    let runs = SampledRuns {
        seed,
        samples: settings.samples(),
        max_steps: settings.max_steps(),
    };
    let mut meter = Meter::new(limits.accounting);
    let draw = match sample_model_runs(system, BOUND_POSITIONS, &sampler, runs, &mut meter, || {
        cancel.poll()
    }) {
        Ok(draw) => draw,
        Err(refused) => {
            let (category, cause) = not_simulated(&refused);
            document.insert("cause".to_owned(), cause);
            document.insert("traces".to_owned(), json!([]));
            document.insert("coverage".to_owned(), coverage_value(&Coverage::default()));
            document.insert("frontier".to_owned(), json!([]));
            return Ok(category);
        }
    };
    let mut emission = Emission {
        meter: &mut meter,
        written: 0,
        limit: limits.emit.output_bytes,
    };
    let limit = settings.max_state_key_bytes();
    let mut oversized = false;
    let mut cut = draw.stopped.as_ref().map(meter_cause);
    let mut written = Vec::new();
    for (index, trace) in draw.traces.iter().enumerate() {
        if let Err(cause) = emission.admit(trace.steps.len().saturating_add(1)) {
            cut = Some(cause);
            break;
        }
        let mut states = Vec::new();
        let mut steps = Vec::new();
        let mut keep = |key: &StateKey| -> Result<bool, SimulationFault> {
            if !u64::try_from(key.as_bytes().len()).is_ok_and(|len| len <= limit) {
                states.push(json!({"position": states.len()}));
                oversized = true;
                return Ok(false);
            }
            states.push(state_value(key)?);
            Ok(true)
        };
        if keep(&trace.initial)? {
            for step in &trace.steps {
                steps.push(json!({"transition": transition_value(&step.transition)?}));
                if !keep(&step.key)? {
                    break;
                }
            }
        }
        let trace = json!({"seed": seed.to_string(), "trace_index": index.to_string(),
            "states": states, "steps": steps});
        if let Err(cause) = emission.account(&trace) {
            cut = Some(cause);
            break;
        }
        written.push(trace);
    }
    let drawn = u64::try_from(written.len()).unwrap_or(u64::MAX);
    let (category, cause) = if let Some(cause) = cut {
        (Category::Incomplete, Some(cause))
    } else if oversized {
        (
            Category::Incomplete,
            Some(
                json!({"kind": "state-key-limit-exceeded", "limit": "max_state_key_bytes",
                "value": limit}),
            ),
        )
    } else if drawn < settings.samples() {
        (
            Category::Incomplete,
            Some(json!({"kind": "cancelled", "code": "cancelled"})),
        )
    } else {
        (Category::Success, None)
    };
    if let Some(cause) = cause {
        document.insert("cause".to_owned(), cause);
    }
    document.insert("traces".to_owned(), Value::Array(written));
    document.insert("coverage".to_owned(), coverage_value(&draw.coverage));
    document.insert("frontier".to_owned(), json!([]));
    Ok(category)
}

#[cfg(test)]
mod tests {
    use filament_exact::{Meter, ScalarLimits};
    use ix_trace_rs::trace;
    use serde_json::json;

    use super::Emission;

    fn limits(result_units: u64) -> ScalarLimits {
        ScalarLimits {
            integer_bits: u64::MAX,
            decimal_digits: u64::MAX,
            scale_expansion: u64::MAX,
            text_input_bytes: u64::MAX,
            text_scalars: u64::MAX,
            normalized_scalars: u64::MAX,
            unit_edges: u64::MAX,
            value_occurrences: u64::MAX,
            work_units: u64::MAX,
            result_units,
        }
    }

    /// FR-634, FR-109: a trace is charged by its length before it is built,
    /// so a run whose leaves times depth pass `result_units` stops naming
    /// the counter, and the written traces' bytes stop at
    /// `emit.output_bytes`.
    #[trace("TC-1594", "FR-634")]
    #[test]
    fn traces_stop_at_the_result_and_emit_limits() {
        let mut meter = Meter::new(limits(5));
        let mut emission = Emission {
            meter: &mut meter,
            written: 0,
            limit: 40,
        };
        assert!(emission.admit(3).is_ok());
        let cause = emission.admit(3).expect_err("3 + 3 positions pass 5");
        assert_eq!(cause["kind"], "limit-reached");
        assert_eq!(cause["limit"], "result_units");
        assert_eq!(cause["value"], 5);

        let trace = json!({"states": [1, 2, 3], "steps": [4, 5]});
        assert!(emission.account(&trace).is_ok());
        let cause = emission
            .account(&trace)
            .expect_err("two traces pass 40 bytes");
        assert_eq!(cause["limit"], "emit.output_bytes");
        assert_eq!(cause["value"], 40);
    }
}
