// SPDX-License-Identifier: AGPL-3.0-or-later
//! FSpec FR-820: the `step_for_step` fact of a refinement, the one declared
//! type its writer, the refinement-transfer record and the transfer planner
//! share.

use fsl_foundation::serde_object::{omitted_not_null, required_nullable};
use serde::{Deserialize, Serialize};

/// The arm of the first step row that is not step-for-step.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StepArm {
    /// The row maps the step to a stutter.
    Stutter,
    /// The row maps the step to any abstract step.
    Any,
    /// The row maps the step to a named node.
    Node,
    /// The row maps the step to an abstract operation.
    Abstract,
}

/// The first step row that is not step-for-step.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StepRow {
    /// The concrete operation the row belongs to; `null` when it has none.
    #[serde(deserialize_with = "required_nullable")]
    pub operation: Option<String>,
    /// The row's target arm.
    pub arm: StepArm,
}

/// The only `time` reason: a dense concrete subject.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Time {
    Dense,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Wire {
    holds: bool,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "omitted_not_null"
    )]
    time: Option<Time>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "omitted_not_null"
    )]
    row: Option<StepRow>,
}

/// FSpec FR-820's `step_for_step` fact: `holds`, and when it does not, the
/// dense `time` reason before any row is read, else the first offending
/// `row`.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "Wire", into = "Wire")]
pub enum StepForStep {
    /// The refinement is step-for-step.
    Holds,
    /// The concrete subject is dense, so no row is read.
    Dense,
    /// The first row that is not step-for-step.
    Row(StepRow),
    /// It does not hold and no reason is recorded.
    Unexplained,
}

impl StepForStep {
    /// Whether the refinement is step-for-step.
    pub const fn holds(&self) -> bool {
        matches!(self, Self::Holds)
    }
}

impl From<StepForStep> for Wire {
    fn from(fact: StepForStep) -> Self {
        let (holds, time, row) = match fact {
            StepForStep::Holds => (true, None, None),
            StepForStep::Dense => (false, Some(Time::Dense), None),
            StepForStep::Row(row) => (false, None, Some(row)),
            StepForStep::Unexplained => (false, None, None),
        };
        Self { holds, time, row }
    }
}

impl TryFrom<Wire> for StepForStep {
    type Error = &'static str;

    fn try_from(wire: Wire) -> Result<Self, Self::Error> {
        match (wire.holds, wire.time, wire.row) {
            (true, None, None) => Ok(Self::Holds),
            (true, _, _) => Err("a holding fact carries no reason"),
            (false, Some(Time::Dense), None) => Ok(Self::Dense),
            (false, None, Some(row)) => Ok(Self::Row(row)),
            (false, None, None) => Ok(Self::Unexplained),
            (false, Some(_), Some(_)) => Err("a fact has one reason, `time` or `row`"),
        }
    }
}

#[cfg(test)]
mod tests {
    use ix_trace_rs::trace;
    use serde_json::json;

    use super::{StepArm, StepForStep, StepRow};

    fn read(value: serde_json::Value) -> Result<StepForStep, serde_json::Error> {
        serde_json::from_value(value)
    }

    /// Each of FSpec FR-820's four forms is written as the spec names it
    /// and read back as the same value.
    #[trace("FSpec-TC-2000", "FSpec-FR-820-AC-2")]
    #[test]
    fn each_step_for_step_form_round_trips() {
        let row = StepRow {
            operation: Some("Impl::Counter::beginA".to_owned()),
            arm: StepArm::Stutter,
        };
        let forms = [
            (StepForStep::Holds, json!({"holds": true})),
            (StepForStep::Dense, json!({"holds": false, "time": "dense"})),
            (
                StepForStep::Row(row),
                json!({"holds": false, "row": {"operation": "Impl::Counter::beginA", "arm": "stutter"}}),
            ),
            (StepForStep::Unexplained, json!({"holds": false})),
            (
                StepForStep::Row(StepRow {
                    operation: None,
                    arm: StepArm::Abstract,
                }),
                json!({"holds": false, "row": {"operation": null, "arm": "abstract"}}),
            ),
        ];
        for (fact, wire) in forms {
            assert_eq!(serde_json::to_value(&fact).expect("writes"), wire);
            assert_eq!(read(wire).expect("reads"), fact);
        }
        assert!(StepForStep::Holds.holds());
        assert!(!StepForStep::Dense.holds());
    }

    /// A fact with two reasons, a reason on a holding fact, an empty row,
    /// a `null` reason and an unknown member are refused.
    #[trace("FSpec-TC-2000", "FSpec-FR-820-AC-2")]
    #[test]
    fn a_malformed_step_for_step_fact_is_refused() {
        for wire in [
            json!({"holds": true, "time": "dense"}),
            json!({"holds": false, "time": "dense", "row": {"operation": null, "arm": "any"}}),
            json!({"holds": false, "row": {}}),
            json!({"holds": false, "row": {"arm": "any"}}),
            json!({"holds": false, "row": null}),
            json!({"holds": false, "time": "sparse"}),
            json!({"holds": false, "extra": 1}),
            json!({"time": "dense"}),
        ] {
            assert!(read(wire.clone()).is_err(), "{wire}");
        }
    }
}
