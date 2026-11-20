// SPDX-License-Identifier: AGPL-3.0-or-later
//! FSpec FR-652 "Certificate forms" as the wire writes them: every exact
//! number an FSpec FR-331 `Rational` object, every box one interval per
//! continuous variable.

use filament_exact::Rational;
use fsl_replay::{HybridBox, HybridInterval};
use fsl_semantics::check::CheckedHybridModel;
use serde_json::{json, Map, Value};

/// One step of a flowpipe or witness certificate.
#[derive(Clone, Debug)]
pub struct Step {
    /// The mode's position.
    pub mode: usize,
    /// The step's `time`.
    pub time: (Rational, Rational),
    /// The `box`.
    pub bx: HybridBox,
    /// The a-priori box `B`.
    pub ap: HybridBox,
    /// The step length `h`.
    pub h: Rational,
    /// The `from` of a flowpipe jump step: its dwell and transition.
    pub from: Option<(usize, usize)>,
}

/// An FSpec FR-331 `Rational`.
pub fn rational(value: &Rational) -> Value {
    json!({"numerator": value.numerator().to_string(), "denominator": value.denominator().to_string()})
}

fn end(value: Option<&Rational>) -> Value {
    value.map_or(Value::Null, rational)
}

fn interval(value: &HybridInterval) -> Value {
    json!({"lower": end(value.lower.as_ref()), "upper": end(value.upper.as_ref())})
}

/// `bx` keyed by the model's variable names.
pub fn box_value(model: &CheckedHybridModel, bx: &HybridBox) -> Value {
    let mut members = Map::new();
    for (variable, x) in model.variables.iter().zip(bx) {
        members.insert(variable.name.as_str().to_owned(), interval(x));
    }
    Value::Object(members)
}

fn step_value(model: &CheckedHybridModel, step: &Step) -> Value {
    let mut members = Map::new();
    members.insert("mode".into(), model.modes[step.mode].name.as_str().into());
    members.insert(
        "time".into(),
        json!({"lower": rational(&step.time.0), "upper": rational(&step.time.1)}),
    );
    members.insert("box".into(), box_value(model, &step.bx));
    members.insert("apriori".into(), box_value(model, &step.ap));
    members.insert("step".into(), rational(&step.h));
    if let Some((dwell, transition)) = step.from {
        members.insert("from".into(), json!({"dwell": dwell, "transition": transition}));
    }
    Value::Object(members)
}

/// A `flowpipe` certificate.
pub fn flowpipe(model: &CheckedHybridModel, steps: &[Step]) -> Value {
    json!({"type": "flowpipe",
        "steps": steps.iter().map(|step| step_value(model, step)).collect::<Vec<_>>()})
}

/// A `validated-witness` certificate: one list of steps per trace state.
pub fn witness(model: &CheckedHybridModel, dwells: &[Vec<Step>]) -> Value {
    json!({"type": "validated-witness",
        "dwells": dwells.iter()
            .map(|steps| steps.iter().map(|step| step_value(model, step)).collect::<Vec<_>>())
            .collect::<Vec<_>>()})
}

/// An `inductive-enclosure` certificate, one box per mode.
pub fn inductive(model: &CheckedHybridModel, boxes: &[HybridBox]) -> Value {
    let mut members = Map::new();
    for (mode, bx) in model.modes.iter().zip(boxes) {
        members.insert(mode.name.as_str().to_owned(), box_value(model, bx));
    }
    json!({"type": "inductive-enclosure", "boxes": members})
}
