// SPDX-License-Identifier: AGPL-3.0-or-later
//! FSpec FR-805 and FR-1010: the strict reader of a plan request carried by
//! a `native-run/1` request's `synthesis` member, and the writer of the
//! `filament.plan-outcome/v1` document.
//!
//! The reader admits the closed member list before any search and names
//! the first defect by its RFC 6901 pointer, prefixed by the pointer of the
//! member that carries the plan request (`/request/synthesis` in a
//! `native-run/1` document). The writer writes every outcome member for
//! every status, `null` where the status gives it no value, with every
//! value in FSpec FR-181's typed value form.

use std::collections::{BTreeMap, BTreeSet};
use std::num::NonZeroU64;

use filament_exact::{CancelCause, Integer, Rational};
use fsl_analyze::plan_search::{
    CertificateOutcome, Direction, PlanLimit, PlanResult, PlanSearchLimits, PlanStatus,
    PlanStopCause,
};
use fsl_foundation::AccountingSetting;
use serde_json::{json, Map, Value};

/// The pointer of the `synthesis` member in a `native-run/1` document.
pub const NATIVE_RUN_POINTER: &str = "/request/synthesis";

/// An admitted plan request (FSpec FR-805 "Request").
#[derive(Clone, Debug, PartialEq)]
pub struct PlanRequest {
    /// The selected `synthesis` declaration; omitted, FSpec FR-823
    /// "Selection" picks the package's only one.
    pub declaration: Option<String>,
    /// The initial-state snapshot file of a state-model plan.
    pub initial: Option<String>,
    /// The policy clause names, in request order, each once.
    pub policy: Vec<String>,
    /// The planner limits in force, defaults filled.
    pub limits: PlanSearchLimits,
    /// The evaluation-meter overrides by counter.
    pub accounting: BTreeMap<AccountingSetting, NonZeroU64>,
    /// The wall-clock deadline in milliseconds.
    pub deadline_ms: Option<u64>,
    /// A hybrid plan's exact positive dwell grid (FSpec FR-819).
    pub dwell_grid: Option<Rational>,
}

/// FSpec FR-805's admission causes, all `invalid_runtime_input`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PlanRequestCause {
    /// The member is not a JSON object.
    MalformedJson,
    /// A required member is absent.
    MissingMember,
    /// A member outside the closed list.
    UnknownMember,
    /// A member of the wrong JSON kind.
    WrongValueKind,
    /// A duplicate policy name, a limit of zero or out of range, a
    /// negative deadline, or a dwell grid that is not positive.
    InvalidValue,
}

impl PlanRequestCause {
    /// The FR-272 cause name.
    pub const fn name(self) -> &'static str {
        match self {
            Self::MalformedJson => "malformed-json",
            Self::MissingMember => "missing-member",
            Self::UnknownMember => "unknown-member",
            Self::WrongValueKind => "wrong-value-kind",
            Self::InvalidValue => "invalid-value",
        }
    }
}

/// A located plan request refusal (FSpec FR-806, exit 20).
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[error("invalid_runtime_input/{}: {pointer}", cause.name())]
pub struct PlanRequestRefusal {
    /// The defect's pointer in the document the caller sent.
    pub pointer: String,
    /// The cause.
    pub cause: PlanRequestCause,
}

const MEMBERS: [&str; 7] = [
    "declaration",
    "initial",
    "policy",
    "limits",
    "accounting",
    "deadline_ms",
    "dwell_grid",
];

impl PlanRequest {
    /// Read `member`, the plan request object, whose own pointer in the
    /// caller's document is `pointer` (FSpec FR-805 "Request").
    ///
    /// # Errors
    /// [`PlanRequestRefusal`] at the first defect, in member order.
    pub fn read(member: &Value, pointer: &str) -> Result<Self, PlanRequestRefusal> {
        let refuse = |path: &str, cause| PlanRequestRefusal {
            pointer: format!("{pointer}{path}"),
            cause,
        };
        let Value::Object(object) = member else {
            return Err(refuse("", PlanRequestCause::MalformedJson));
        };
        if let Some(unknown) = object.keys().find(|key| !MEMBERS.contains(&key.as_str())) {
            return Err(refuse(
                &format!("/{}", escape(unknown)),
                PlanRequestCause::UnknownMember,
            ));
        }
        let string = |name: &str| match object.get(name) {
            None => Ok(None),
            Some(Value::String(text)) => Ok(Some(text.clone())),
            Some(_) => Err(refuse(
                &format!("/{name}"),
                PlanRequestCause::WrongValueKind,
            )),
        };
        let declaration = string("declaration")?;
        let initial = string("initial")?;
        let policy = match object.get("policy") {
            None => Vec::new(),
            Some(Value::Array(names)) => {
                let mut seen = BTreeSet::new();
                let mut policy = Vec::with_capacity(names.len());
                for (index, name) in names.iter().enumerate() {
                    let Value::String(name) = name else {
                        return Err(refuse(
                            &format!("/policy/{index}"),
                            PlanRequestCause::WrongValueKind,
                        ));
                    };
                    if !seen.insert(name.clone()) {
                        return Err(refuse(
                            &format!("/policy/{index}"),
                            PlanRequestCause::InvalidValue,
                        ));
                    }
                    policy.push(name.clone());
                }
                policy
            }
            Some(_) => return Err(refuse("/policy", PlanRequestCause::WrongValueKind)),
        };
        let mut limits = PlanSearchLimits::default();
        match object.get("limits") {
            None => {}
            Some(Value::Object(members)) => {
                for (name, value) in members {
                    let path = format!("/limits/{}", escape(name));
                    let Some(limit) = PlanLimit::from_member(name) else {
                        return Err(refuse(&path, PlanRequestCause::UnknownMember));
                    };
                    limits.set(
                        limit,
                        positive(value).map_err(|cause| refuse(&path, cause))?,
                    );
                }
            }
            Some(_) => return Err(refuse("/limits", PlanRequestCause::WrongValueKind)),
        }
        let mut accounting = BTreeMap::new();
        match object.get("accounting") {
            None => {}
            Some(Value::Object(members)) => {
                for (name, value) in members {
                    let path = format!("/accounting/{}", escape(name));
                    let Some(setting) = AccountingSetting::from_counter(name) else {
                        return Err(refuse(&path, PlanRequestCause::UnknownMember));
                    };
                    accounting.insert(
                        setting,
                        positive(value).map_err(|cause| refuse(&path, cause))?,
                    );
                }
            }
            Some(_) => return Err(refuse("/accounting", PlanRequestCause::WrongValueKind)),
        }
        let deadline_ms = match object.get("deadline_ms") {
            None => None,
            Some(Value::Number(number)) => match number.as_u64() {
                Some(value) => Some(value),
                None => return Err(refuse("/deadline_ms", PlanRequestCause::InvalidValue)),
            },
            Some(_) => return Err(refuse("/deadline_ms", PlanRequestCause::WrongValueKind)),
        };
        let dwell_grid = match object.get("dwell_grid") {
            None => None,
            Some(value) => {
                Some(exact_positive(value).map_err(|cause| refuse("/dwell_grid", cause))?)
            }
        };
        Ok(Self {
            declaration,
            initial,
            policy,
            limits,
            accounting,
            deadline_ms,
            dwell_grid,
        })
    }

    /// Check the members the selected declaration's grammar requires: a
    /// state-model plan needs `initial` and admits no `dwell_grid`; a
    /// hybrid plan needs `dwell_grid` and admits no `initial` (FSpec
    /// FR-805, FR-819).
    ///
    /// # Errors
    /// [`PlanRequestRefusal`] naming the member.
    pub fn check_grammar(&self, hybrid: bool, pointer: &str) -> Result<(), PlanRequestRefusal> {
        let refuse = |member: &str, cause| {
            Err(PlanRequestRefusal {
                pointer: format!("{pointer}/{member}"),
                cause,
            })
        };
        match (hybrid, &self.initial, &self.dwell_grid) {
            (false, None, _) => refuse("initial", PlanRequestCause::MissingMember),
            (false, Some(_), Some(_)) => refuse("dwell_grid", PlanRequestCause::UnknownMember),
            (true, Some(_), _) => refuse("initial", PlanRequestCause::UnknownMember),
            (true, None, None) => refuse("dwell_grid", PlanRequestCause::MissingMember),
            (false, Some(_), None) | (true, None, Some(_)) => Ok(()),
        }
    }
}

/// An RFC 6901 reference token.
fn escape(token: &str) -> String {
    token.replace('~', "~0").replace('/', "~1")
}

/// A limit value: an unsigned 64-bit integer of at least 1 (FSpec FR-801
/// "Invalid values").
fn positive(value: &Value) -> Result<NonZeroU64, PlanRequestCause> {
    let Value::Number(number) = value else {
        return Err(PlanRequestCause::WrongValueKind);
    };
    number
        .as_u64()
        .and_then(NonZeroU64::new)
        .ok_or(PlanRequestCause::InvalidValue)
}

/// An exact positive rational in FSpec FR-181's typed `rational` or
/// `decimal` form (FSpec FR-805 `dwell_grid`).
fn exact_positive(value: &Value) -> Result<Rational, PlanRequestCause> {
    let Value::Object(object) = value else {
        return Err(PlanRequestCause::WrongValueKind);
    };
    let text = |name: &str| match object.get(name) {
        Some(Value::String(text)) => text.parse::<Integer>().ok(),
        _ => None,
    };
    let closed = |names: &[&str]| object.keys().all(|key| names.contains(&key.as_str()));
    let rational = match object.get("type").and_then(Value::as_str) {
        Some("rational") if closed(&["type", "numerator", "denominator"]) => {
            let (Some(numerator), Some(denominator)) = (text("numerator"), text("denominator"))
            else {
                return Err(PlanRequestCause::InvalidValue);
            };
            if denominator.is_zero() || denominator.is_negative() {
                return Err(PlanRequestCause::InvalidValue);
            }
            Rational::new(numerator, denominator).map_err(|_| PlanRequestCause::InvalidValue)?
        }
        Some("decimal") if closed(&["type", "coefficient", "scale"]) => {
            let (Some(coefficient), Some(scale)) = (text("coefficient"), text("scale")) else {
                return Err(PlanRequestCause::InvalidValue);
            };
            let scale: u64 = scale
                .to_string()
                .parse()
                .map_err(|_| PlanRequestCause::InvalidValue)?;
            Rational::from_integer(coefficient).divided_by_power_of_ten(scale)
        }
        _ => return Err(PlanRequestCause::WrongValueKind),
    };
    if rational.is_zero() || rational.numerator().is_negative() {
        return Err(PlanRequestCause::InvalidValue);
    }
    Ok(rational)
}

/// The FR-181 typed form an objective value is written in.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ValueForm {
    /// `{"type": "integer", "value": ...}`.
    Integer,
    /// `{"type": "decimal", "coefficient": ..., "scale": ...}` at `scale`.
    Decimal {
        /// The retained scale.
        scale: u32,
    },
    /// The normalized `{"type": "rational", ...}` form.
    Rational,
    /// The normalized rational form with its `unit`.
    Quantity {
        /// The unit symbol, e.g. `min`.
        unit: String,
    },
}

impl ValueForm {
    /// `value` in this form; `None` for an integer or decimal form that
    /// cannot hold it exactly.
    pub fn write(&self, value: &Rational) -> Option<Value> {
        match self {
            Self::Integer => value
                .is_integer()
                .then(|| json!({"type": "integer", "value": value.numerator().to_string()})),
            Self::Decimal { scale } => {
                let shifted = value.mul(&Rational::from_integer(ten_to(*scale)));
                shifted.is_integer().then(|| {
                    json!({"type": "decimal", "coefficient": shifted.numerator().to_string(),
                        "scale": scale.to_string()})
                })
            }
            Self::Rational => Some(rational(value)),
            Self::Quantity { unit } => {
                let mut written = rational(value);
                written["unit"] = Value::String(unit.clone());
                Some(written)
            }
        }
    }
}

fn ten_to(exponent: u32) -> Integer {
    (0..exponent).fold(Integer::from(1_i64), |power, _| {
        power.mul(&Integer::from(10_i64))
    })
}

fn rational(value: &Rational) -> Value {
    json!({"type": "rational", "numerator": value.numerator().to_string(),
        "denominator": value.denominator().to_string()})
}

/// The objective as the outcome names it (FSpec FR-805 `objective`).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlanObjective {
    /// `minimize` or `maximize`.
    pub direction: Direction,
    /// The objective's source text; `length` when the declaration has none.
    pub expression: String,
    /// The form its value is written in.
    pub form: ValueForm,
}

/// One plan step as the outcome writes it.
#[derive(Clone, Debug, PartialEq)]
pub struct RenderedStep {
    /// The operation's name.
    pub operation: String,
    /// `{parameter, value}` pairs in parameter order.
    pub arguments: Vec<(String, Value)>,
    /// The native FR-181 typed receiver, absent only for receiverless operations.
    pub receiver: Option<Value>,
    /// Canonical outcome ordinal, present only for multiple native outcomes.
    pub outcome_ordinal: Option<u64>,
    /// A timed model's exact delay, in its typed quantity form.
    pub delay: Option<Value>,
}

/// A reported plan as the outcome writes it.
#[derive(Clone, Debug, PartialEq)]
pub struct RenderedPlan {
    /// The steps, in plan order.
    pub steps: Vec<RenderedStep>,
    /// The `length + 1` states from the initial state.
    pub states: Vec<Value>,
}

/// FSpec FR-805 "Categories and exits": the exit of `status`.
pub const fn exit_code(status: PlanStatus) -> u8 {
    match status {
        PlanStatus::Optimal | PlanStatus::Feasible => 0,
        PlanStatus::Infeasible => 10,
        PlanStatus::Incomplete => 22,
    }
}

/// The exit of an `unsupported` plan outcome (FSpec FR-819).
pub const UNSUPPORTED_EXIT: u8 = 21;

const fn status_name(status: PlanStatus) -> &'static str {
    match status {
        PlanStatus::Optimal => "optimal",
        PlanStatus::Feasible => "feasible",
        PlanStatus::Infeasible => "infeasible",
        PlanStatus::Incomplete => "incomplete",
    }
}

/// The identity members every outcome document carries.
#[derive(Clone, Copy, Debug)]
pub struct OutcomeHeader<'h> {
    /// The checked package identity.
    pub package: &'h str,
    /// The selected declaration.
    pub declaration: &'h str,
    /// The objective.
    pub objective: &'h PlanObjective,
    /// The limits in force.
    pub limits: &'h PlanSearchLimits,
}

/// Write `result` as a `filament.plan-outcome/v1` document, `plan` being
/// its reported plan rendered (FSpec FR-805 "Members by status").
pub fn plan_outcome(
    header: OutcomeHeader<'_>,
    result: &PlanResult,
    plan: Option<&RenderedPlan>,
) -> Value {
    let reported = result.plan.is_some();
    let objective_value = result
        .objective
        .as_ref()
        .filter(|_| reported)
        .and_then(|value| header.objective.form.write(value));
    let mut objective = objective_member(header.objective, objective_value);
    if let Some(step) = result.negative_step {
        objective["negative_step"] = json!(step);
    }
    let certificate = result.certificate.as_ref().and_then(|report| {
        // An `incomplete` result keeps only a rejected closure certificate
        // (FSpec FR-805 "Members by status").
        if result.status == PlanStatus::Incomplete
            && !matches!(
                result.cause,
                Some(PlanStopCause::CertificateRejected { .. })
            )
        {
            return None;
        }
        let mut written = json!({"type": report.kind.name(), "entries": report.entries});
        match &report.outcome {
            CertificateOutcome::Accepted => written["outcome"] = json!("accepted"),
            CertificateOutcome::Rejected { rule, entry } => {
                written["outcome"] = json!("rejected");
                written["rule"] = json!(rule.name());
                written["entry"] = json!(entry);
            }
        }
        Some(written)
    });
    let certification = match (result.certified, reported) {
        (true, _) => json!("certified"),
        (false, true) => json!("uncertified"),
        (false, false) => Value::Null,
    };
    let plan_member = match plan.filter(|_| reported) {
        Some(plan) => json!({
            "length": plan.steps.len(),
            "steps": plan.steps.iter().enumerate().map(|(index, step)| {
                let mut written = json!({"index": index, "operation": step.operation,
                    "arguments": step.arguments.iter().map(|(parameter, value)|
                        json!({"parameter": parameter, "value": value})).collect::<Vec<_>>()});
                if let Some(receiver) = &step.receiver { written["receiver"] = receiver.clone(); }
                if let Some(ordinal) = step.outcome_ordinal { written["outcome_ordinal"] = json!(ordinal); }
                if let Some(delay) = &step.delay {
                    written["delay"] = delay.clone();
                }
                written
            }).collect::<Vec<_>>(),
            "states": plan.states,
        }),
        None => Value::Null,
    };
    let replay = match plan.filter(|_| reported) {
        Some(plan) => json!({"outcome": "reproduced", "steps": plan.steps.len()}),
        None => Value::Null,
    };
    let mut document = header_members(header, status_name(result.status), objective);
    document.insert("plan".to_owned(), plan_member);
    document.insert("replay".to_owned(), replay);
    document.insert("certificate".to_owned(), certificate.unwrap_or(Value::Null));
    document.insert("certification".to_owned(), certification);
    document.insert("bound".to_owned(), Value::Null);
    document.insert(
        "explored".to_owned(),
        json!({"states": result.explored.states, "work": result.explored.work,
            "nan_objectives": result.explored.nan_objectives,
            "undefined_objectives": result.explored.undefined_objectives}),
    );
    document.insert("limits".to_owned(), limits_member(header.limits));
    document.insert(
        "cause".to_owned(),
        result.cause.as_ref().map_or(Value::Null, cause_member),
    );
    Value::Object(document)
}

/// Write an `unsupported` plan outcome with the FR-193 `cause`, every
/// no-plan member `null` and zero explored counts (FSpec FR-805, FR-819).
pub fn unsupported_outcome(header: OutcomeHeader<'_>, cause: Value) -> Value {
    let objective = objective_member(header.objective, None);
    let mut document = header_members(header, "unsupported", objective);
    for member in ["plan", "replay", "certificate", "certification", "bound"] {
        document.insert(member.to_owned(), Value::Null);
    }
    document.insert(
        "explored".to_owned(),
        json!({"states": 0, "work": 0, "nan_objectives": 0, "undefined_objectives": 0}),
    );
    document.insert("limits".to_owned(), limits_member(header.limits));
    document.insert("cause".to_owned(), cause);
    Value::Object(document)
}

fn objective_member(objective: &PlanObjective, value: Option<Value>) -> Value {
    json!({
        "direction": match objective.direction {
            Direction::Minimize => "minimize",
            Direction::Maximize => "maximize",
        },
        "expression": objective.expression,
        "value": value.unwrap_or(Value::Null),
    })
}

fn header_members(header: OutcomeHeader<'_>, status: &str, objective: Value) -> Map<String, Value> {
    let mut document = Map::new();
    document.insert("format".to_owned(), json!("filament.plan-outcome/v1"));
    document.insert("package".to_owned(), json!(header.package));
    document.insert("declaration".to_owned(), json!(header.declaration));
    document.insert("status".to_owned(), json!(status));
    document.insert("objective".to_owned(), objective);
    document
}

fn limits_member(limits: &PlanSearchLimits) -> Value {
    Value::Object(
        limits
            .by_setting()
            .into_iter()
            .map(|(setting, value)| (setting.to_owned(), json!(value)))
            .collect(),
    )
}

fn cause_member(cause: &PlanStopCause) -> Value {
    match cause {
        PlanStopCause::LimitReached {
            limit,
            value,
            setting,
        } => json!({"type": "limit-reached", "limit": limit, "value": value, "setting": setting}),
        PlanStopCause::Cancelled(source) => json!({"type": "cancelled", "source": match source {
            CancelCause::Requested => "requested",
            CancelCause::Deadline => "deadline",
        }}),
        PlanStopCause::CertificateRejected { rule, entry } => {
            json!({"type": "certificate-rejected", "rule": rule.name(), "entry": entry})
        }
        PlanStopCause::InvariantUndetermined { invariant, .. } => {
            json!({"type": "invariant-undetermined", "invariant": invariant})
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{PlanRequest, PlanRequestCause, ValueForm};
    use filament_exact::{Integer, Rational};
    use serde_json::{json, Value};

    fn refusal(member: Value) -> (String, &'static str) {
        let refusal = PlanRequest::read(&member, "/request/synthesis").unwrap_err();
        (refusal.pointer, refusal.cause.name())
    }

    /// FSpec FR-805-AC-4, FR-801-AC-7 and FR-805-AC-8: each malformed member
    /// refuses at its pointer under the host member.
    #[test]
    fn malformed_members_refuse_at_their_pointer() {
        let base = |extra: Value| {
            let mut member = json!({"declaration": "Reach", "initial": "s.json"});
            for (key, value) in extra.as_object().unwrap() {
                member[key] = value.clone();
            }
            member
        };
        for (extra, pointer, cause) in [
            (
                json!({"strategy": "greedy"}),
                "/request/synthesis/strategy",
                "unknown-member",
            ),
            (
                json!({"replay_plan": []}),
                "/request/synthesis/replay_plan",
                "unknown-member",
            ),
            (
                json!({"engines": "x"}),
                "/request/synthesis/engines",
                "unknown-member",
            ),
            (
                json!({"policy": ["P", "P"]}),
                "/request/synthesis/policy/1",
                "invalid-value",
            ),
            (
                json!({"policy": "P"}),
                "/request/synthesis/policy",
                "wrong-value-kind",
            ),
            (
                json!({"limits": {"max_work": 0}}),
                "/request/synthesis/limits/max_work",
                "invalid-value",
            ),
            (
                json!({"limits": {"max_work": -1}}),
                "/request/synthesis/limits/max_work",
                "invalid-value",
            ),
            (
                json!({"limits": {"max_work": 1.5}}),
                "/request/synthesis/limits/max_work",
                "invalid-value",
            ),
            (
                json!({"limits": {"max_work": "10"}}),
                "/request/synthesis/limits/max_work",
                "wrong-value-kind",
            ),
            (
                json!({"limits": {"max_steps": 1}}),
                "/request/synthesis/limits/max_steps",
                "unknown-member",
            ),
            (
                json!({"accounting": {"steps": 1}}),
                "/request/synthesis/accounting/steps",
                "unknown-member",
            ),
            (
                json!({"dwell_grid": 1}),
                "/request/synthesis/dwell_grid",
                "wrong-value-kind",
            ),
            (
                json!({"dwell_grid": {"type": "rational", "numerator": "0", "denominator": "1"}}),
                "/request/synthesis/dwell_grid",
                "invalid-value",
            ),
            (
                json!({"deadline_ms": -1}),
                "/request/synthesis/deadline_ms",
                "invalid-value",
            ),
        ] {
            assert_eq!(
                refusal(base(extra.clone())),
                (pointer.to_owned(), cause),
                "{extra}"
            );
        }
        let above: Value = serde_json::from_str(
            r#"{"declaration": "Reach", "initial": "s.json", "limits": {"max_work": 18446744073709551616}}"#,
        )
        .unwrap();
        assert_eq!(
            refusal(above),
            (
                "/request/synthesis/limits/max_work".to_owned(),
                "invalid-value"
            )
        );
        assert_eq!(refusal(json!([])).1, PlanRequestCause::MalformedJson.name());
    }

    /// FSpec FR-801-AC-7: an override replaces only its member.
    #[test]
    fn a_limit_override_replaces_only_its_member() {
        let request = PlanRequest::read(
            &json!({"declaration": "Reach", "initial": "s.json", "limits": {"max_states": 500}}),
            "/request/synthesis",
        )
        .unwrap();
        let limits = request.limits.by_setting();
        assert_eq!(limits["plan_search.max_states"], 500);
        assert_eq!(limits["plan_search.max_work"], 100_000_000);
        assert_eq!(limits["plan_search.max_certificate_entries"], 16_777_216);
        assert_eq!(limits["plan_search.max_check_work"], 100_000_000);
        assert_eq!(limits["plan_search.max_state_key_bytes"], 1_048_576);
    }

    /// FSpec FR-805-AC-8: a half time unit is admitted exactly, and the
    /// grammar decides which of `initial` and `dwell_grid` is required.
    #[test]
    fn the_dwell_grid_is_exact_and_the_grammar_selects_its_members() {
        let hybrid = PlanRequest::read(
            &json!({"declaration": "Reach",
                "dwell_grid": {"type": "rational", "numerator": "1", "denominator": "2"}}),
            "/request/synthesis",
        )
        .unwrap();
        let half = Rational::new(Integer::from(1_i64), Integer::from(2_i64)).unwrap();
        assert_eq!(hybrid.dwell_grid, Some(half));
        assert!(hybrid.check_grammar(true, "/request/synthesis").is_ok());
        assert_eq!(
            hybrid
                .check_grammar(false, "/request/synthesis")
                .unwrap_err()
                .pointer,
            "/request/synthesis/initial"
        );
        let with_initial = PlanRequest::read(
            &json!({"declaration": "Reach", "initial": "s.json",
                "dwell_grid": {"type": "decimal", "coefficient": "5", "scale": "1"}}),
            "/request/synthesis",
        )
        .unwrap();
        let refusal = with_initial
            .check_grammar(true, "/request/synthesis")
            .unwrap_err();
        assert_eq!(refusal.pointer, "/request/synthesis/initial");
        assert_eq!(refusal.cause, PlanRequestCause::UnknownMember);
    }

    /// FSpec FR-805-AC-3 and AC-7: values are written in the typed form,
    /// never as JSON numbers.
    #[test]
    fn objective_values_are_written_in_the_typed_form() {
        let tenth =
            |count: i64| Rational::new(Integer::from(count), Integer::from(10_i64)).unwrap();
        assert_eq!(
            ValueForm::Decimal { scale: 2 }.write(&tenth(3)),
            Some(json!({"type": "decimal", "coefficient": "30", "scale": "2"}))
        );
        assert_eq!(
            ValueForm::Integer.write(&Rational::from_integer(Integer::from(3_i64))),
            Some(json!({"type": "integer", "value": "3"}))
        );
        assert_eq!(
            ValueForm::Rational
                .write(&Rational::new(Integer::from(6_i64), Integer::from(8_i64)).unwrap()),
            Some(json!({"type": "rational", "numerator": "3", "denominator": "4"}))
        );
        assert_eq!(
            ValueForm::Quantity {
                unit: "min".to_owned()
            }
            .write(&Rational::from_integer(Integer::from(60_i64))),
            Some(json!({"type": "rational", "numerator": "60", "denominator": "1", "unit": "min"}))
        );
    }
}
