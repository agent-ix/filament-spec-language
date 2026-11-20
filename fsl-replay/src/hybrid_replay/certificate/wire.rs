// SPDX-License-Identifier: AGPL-3.0-or-later
//! FSpec FR-652 "Checking order" steps 1 to 3: a provider's `certificate`
//! member read by the FSpec FR-331 schema (every object closed, every exact
//! number a lowest-terms `Rational` object), matched to its entry's form,
//! and checked against the structural refusals FSpec FR-652 "Structural
//! refusals" lists. A failure is a [`CertificateRefusal`]: the reader cannot
//! read the certificate, which is never a rejection by a rule.
//!
//! Every pointer is rooted at `/certificate`; a replay request prefixes its
//! `/replay`.

use std::collections::BTreeSet;

use filament_exact::{Integer, Rational};
use fsl_semantics::check::CheckedHybridModel;
use serde_json::{Map, Value};

use crate::hybrid_replay::{ExactInterval, HybridTrace};

/// The root of every certificate pointer.
pub(crate) const ROOT: &str = "/certificate";

/// Why the reader refuses a certificate: the FSpec FR-272 `invalid_result`
/// (`invalid_runtime_input` on a replay request) cause.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RefusalCode {
    /// `missing-member`.
    MissingMember,
    /// `unknown-member`.
    UnknownMember,
    /// `wrong-value-kind`.
    WrongValueKind,
    /// `invalid-value`.
    InvalidValue,
}

impl RefusalCode {
    /// The FSpec FR-272 cause spelling.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::MissingMember => "missing-member",
            Self::UnknownMember => "unknown-member",
            Self::WrongValueKind => "wrong-value-kind",
            Self::InvalidValue => "invalid-value",
        }
    }
}

/// A certificate the reader cannot read.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[error("the certificate is refused at {pointer}: {}", .code.as_str())]
pub struct CertificateRefusal {
    /// The RFC 6901 pointer of the member, rooted at `/certificate`.
    pub pointer: String,
    /// The cause.
    pub code: RefusalCode,
}

impl CertificateRefusal {
    fn new(pointer: impl Into<String>, code: RefusalCode) -> Self {
        Self {
            pointer: pointer.into(),
            code,
        }
    }

    /// This refusal with its pointer moved under `prefix`, as a replay
    /// request names it (`/replay/certificate/...`).
    #[must_use]
    pub fn under(mut self, prefix: &str) -> Self {
        self.pointer = format!("{prefix}{}", self.pointer);
        self
    }
}

impl From<CertificateRefusal> for crate::proof_result::PluginResultRefusal {
    fn from(refusal: CertificateRefusal) -> Self {
        use crate::proof_result::{PluginResultRefusal, ValueFault};
        let pointer = refusal.pointer;
        match refusal.code {
            RefusalCode::MissingMember => PluginResultRefusal::MissingMember { pointer },
            RefusalCode::UnknownMember => PluginResultRefusal::UnknownMember { pointer },
            RefusalCode::WrongValueKind => PluginResultRefusal::Malformed {
                pointer,
                cause: ValueFault::WrongValueKind,
            },
            RefusalCode::InvalidValue => PluginResultRefusal::Malformed {
                pointer,
                cause: ValueFault::InvalidValue,
            },
        }
    }
}

/// A box as the wire writes it: one interval per key, the keys unchecked
/// (`box-shape` examines them).
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct WireBox(pub(crate) Vec<(String, ExactInterval)>);

/// One step of a flowpipe or a witness.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct WireStep {
    /// The mode's position in the model.
    pub(crate) mode: usize,
    pub(crate) time: ExactInterval,
    pub(crate) bx: WireBox,
    pub(crate) apriori: WireBox,
    pub(crate) step: Rational,
    /// `{dwell, transition}` of a flowpipe jump step.
    pub(crate) from: Option<(usize, usize)>,
}

/// The certificate's content by form.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum Body {
    /// One box per mode, in the model's mode order.
    Inductive(Vec<WireBox>),
    Flowpipe(Vec<WireStep>),
    Witness(Vec<Vec<WireStep>>),
}

/// A hybrid provider's certificate, read and structurally admitted
/// (FSpec FR-652 "Checking order" steps 1 to 3), with the member exactly as
/// the provider returned it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HybridCertificate {
    raw: Value,
    pub(crate) body: Body,
}

impl HybridCertificate {
    /// The certificate as the provider returned it (FR-621 `form`).
    pub const fn raw(&self) -> &Value {
        &self.raw
    }

    /// The form's `type` spelling.
    pub const fn form(&self) -> &'static str {
        match self.body {
            Body::Inductive(_) => "inductive-enclosure",
            Body::Flowpipe(_) => "flowpipe",
            Body::Witness(_) => "validated-witness",
        }
    }
}

/// The form the entry the certificate rides on takes (FSpec FR-652 "Checking
/// order" step 2): by basis type, never by `method`.
#[derive(Clone, Copy, Debug)]
pub enum Expect<'t> {
    /// A `bounded-solver` proof: `flowpipe`.
    Bounded,
    /// An `inductive-solver` proof: `inductive-enclosure`.
    Inductive,
    /// A `refuted` entry or a `solver-inconclusive` entry carrying this
    /// trace: `validated-witness`.
    Witness(&'t HybridTrace),
}

fn child(base: &str, name: &str) -> String {
    format!("{base}/{}", name.replace('~', "~0").replace('/', "~1"))
}

fn index(base: &str, position: usize) -> String {
    format!("{base}/{position}")
}

fn refuse<T>(pointer: &str, code: RefusalCode) -> Result<T, CertificateRefusal> {
    Err(CertificateRefusal::new(pointer, code))
}

fn name_order(left: &str, right: &str) -> std::cmp::Ordering {
    left.encode_utf16().cmp(right.encode_utf16())
}

fn object<'a>(
    value: &'a Value,
    allowed: &[&str],
    pointer: &str,
) -> Result<&'a Map<String, Value>, CertificateRefusal> {
    let Value::Object(members) = value else {
        return refuse(pointer, RefusalCode::WrongValueKind);
    };
    let extra = members
        .keys()
        .map(String::as_str)
        .filter(|name| !allowed.contains(name))
        .min_by(|left, right| name_order(left, right));
    if let Some(extra) = extra {
        return refuse(&child(pointer, extra), RefusalCode::UnknownMember);
    }
    Ok(members)
}

fn required<'a>(
    members: &'a Map<String, Value>,
    name: &str,
    pointer: &str,
) -> Result<&'a Value, CertificateRefusal> {
    members.get(name).map_or_else(
        || refuse(&child(pointer, name), RefusalCode::MissingMember),
        Ok,
    )
}

fn canonical_integer(text: &str) -> Option<Integer> {
    let digits = text.strip_prefix('-').unwrap_or(text);
    let canonical = !digits.is_empty()
        && digits.bytes().all(|byte| byte.is_ascii_digit())
        && (digits == "0" || !digits.starts_with('0'))
        && text != "-0";
    if canonical {
        text.parse::<Integer>().ok()
    } else {
        None
    }
}

/// An FSpec FR-331 `Rational`: `{numerator, denominator}` decimal strings in
/// lowest terms.
fn rational(value: &Value, pointer: &str) -> Result<Rational, CertificateRefusal> {
    let members = object(value, &["denominator", "numerator"], pointer)?;
    let part = |name: &str| {
        members
            .get(name)
            .and_then(Value::as_str)
            .and_then(canonical_integer)
    };
    let (Some(numerator), Some(denominator)) = (part("numerator"), part("denominator")) else {
        return refuse(pointer, RefusalCode::InvalidValue);
    };
    let Ok(reduced) = Rational::new(numerator.clone(), denominator.clone()) else {
        return refuse(pointer, RefusalCode::InvalidValue);
    };
    if reduced.numerator() != &numerator || reduced.denominator() != &denominator {
        return refuse(pointer, RefusalCode::InvalidValue);
    }
    Ok(reduced)
}

fn interval(value: &Value, pointer: &str) -> Result<ExactInterval, CertificateRefusal> {
    let members = object(value, &["lower", "upper"], pointer)?;
    let end = |name: &str| rational(required(members, name, pointer)?, &child(pointer, name));
    Ok(ExactInterval::new(end("lower")?, end("upper")?))
}

fn read_box(value: &Value, pointer: &str) -> Result<WireBox, CertificateRefusal> {
    let Value::Object(members) = value else {
        return refuse(pointer, RefusalCode::WrongValueKind);
    };
    members
        .iter()
        .map(|(name, bound)| Ok((name.clone(), interval(bound, &child(pointer, name))?)))
        .collect::<Result<Vec<_>, CertificateRefusal>>()
        .map(WireBox)
}

/// A step before its mode name is resolved.
struct RawStep {
    mode: String,
    time: ExactInterval,
    bx: WireBox,
    apriori: WireBox,
    step: Rational,
    from: Option<(u64, u64)>,
}

fn count(value: &Value, pointer: &str) -> Result<u64, CertificateRefusal> {
    value
        .as_u64()
        .map_or_else(|| refuse(pointer, RefusalCode::WrongValueKind), Ok)
}

fn read_step(value: &Value, pointer: &str) -> Result<RawStep, CertificateRefusal> {
    let members = object(
        value,
        &["apriori", "box", "from", "mode", "step", "time"],
        pointer,
    )?;
    let mode_pointer = child(pointer, "mode");
    let mode = match required(members, "mode", pointer)? {
        Value::String(name) => name.clone(),
        _ => return refuse(&mode_pointer, RefusalCode::WrongValueKind),
    };
    let time = interval(required(members, "time", pointer)?, &child(pointer, "time"))?;
    let bx = read_box(required(members, "box", pointer)?, &child(pointer, "box"))?;
    let apriori = read_box(
        required(members, "apriori", pointer)?,
        &child(pointer, "apriori"),
    )?;
    let step = rational(required(members, "step", pointer)?, &child(pointer, "step"))?;
    let from = match members.get("from") {
        None => None,
        Some(source) => {
            let from_pointer = child(pointer, "from");
            let fields = object(source, &["dwell", "transition"], &from_pointer)?;
            let dwell = count(
                required(fields, "dwell", &from_pointer)?,
                &child(&from_pointer, "dwell"),
            )?;
            let transition = count(
                required(fields, "transition", &from_pointer)?,
                &child(&from_pointer, "transition"),
            )?;
            Some((dwell, transition))
        }
    };
    Ok(RawStep {
        mode,
        time,
        bx,
        apriori,
        step,
        from,
    })
}

fn read_steps(value: &Value, pointer: &str) -> Result<Vec<RawStep>, CertificateRefusal> {
    let Value::Array(items) = value else {
        return refuse(pointer, RefusalCode::WrongValueKind);
    };
    items
        .iter()
        .enumerate()
        .map(|(position, step)| read_step(step, &index(pointer, position)))
        .collect()
}

fn mode_of(model: &CheckedHybridModel, name: &str) -> Option<usize> {
    model
        .modes
        .iter()
        .position(|mode| mode.name.as_str() == name)
}

fn zero() -> Rational {
    Rational::from_integer(Integer::from(0_i64))
}

/// A step's own structural refusals (FSpec FR-652 "Structural refusals"):
/// its mode, its `step` and its `time`.
fn resolve_step(
    model: &CheckedHybridModel,
    raw: RawStep,
    pointer: &str,
) -> Result<WireStep, CertificateRefusal> {
    let Some(mode) = mode_of(model, &raw.mode) else {
        return refuse(&child(pointer, "mode"), RefusalCode::InvalidValue);
    };
    if raw.step.numerator().is_negative() {
        return refuse(&child(pointer, "step"), RefusalCode::InvalidValue);
    }
    if raw.time.lower < zero() || raw.time.lower > raw.time.upper {
        return refuse(&child(pointer, "time"), RefusalCode::InvalidValue);
    }
    let from = raw.from.map(|(dwell, transition)| {
        (
            usize::try_from(dwell).unwrap_or(usize::MAX),
            usize::try_from(transition).unwrap_or(usize::MAX),
        )
    });
    Ok(WireStep {
        mode,
        time: raw.time,
        bx: raw.bx,
        apriori: raw.apriori,
        step: raw.step,
        from,
    })
}

fn flowpipe_structure(
    model: &CheckedHybridModel,
    raw: Vec<RawStep>,
) -> Result<Vec<WireStep>, CertificateRefusal> {
    let base = child(ROOT, "steps");
    if raw.is_empty() {
        return refuse(&base, RefusalCode::InvalidValue);
    }
    if raw[0].from.is_some() {
        return refuse(&child(&index(&base, 0), "from"), RefusalCode::InvalidValue);
    }
    let steps = raw
        .into_iter()
        .enumerate()
        .map(|(position, step)| resolve_step(model, step, &index(&base, position)))
        .collect::<Result<Vec<_>, _>>()?;
    let mut dwell_first: Vec<usize> = Vec::new();
    let mut dwell_of: Vec<usize> = Vec::with_capacity(steps.len());
    for (k, step) in steps.iter().enumerate() {
        let starts = k == 0 || step.from.is_some() || step.mode != steps[k - 1].mode;
        if starts {
            dwell_first.push(k);
        }
        dwell_of.push(dwell_first.len() - 1);
    }
    let mut jumped: BTreeSet<(usize, usize)> = BTreeSet::new();
    let mut initial_modes: BTreeSet<usize> = BTreeSet::new();
    for (k, step) in steps.iter().enumerate() {
        let dwell = dwell_of[k];
        if dwell_first[dwell] != k {
            continue;
        }
        let pointer = index(&base, k);
        let Some((from_dwell, transition)) = step.from else {
            if !initial_modes.insert(step.mode) {
                return refuse(&child(&pointer, "mode"), RefusalCode::InvalidValue);
            }
            continue;
        };
        let from_pointer = child(&pointer, "from");
        if from_dwell >= dwell {
            return refuse(&child(&from_pointer, "dwell"), RefusalCode::InvalidValue);
        }
        let source = &model.modes[steps[dwell_first[from_dwell]].mode];
        let Some(taken) = source.transitions.get(transition) else {
            return refuse(
                &child(&from_pointer, "transition"),
                RefusalCode::InvalidValue,
            );
        };
        if !jumped.insert((from_dwell, transition)) {
            return refuse(&from_pointer, RefusalCode::InvalidValue);
        }
        if taken.target.get() != step.mode {
            return refuse(&child(&pointer, "mode"), RefusalCode::InvalidValue);
        }
    }
    Ok(steps)
}

fn witness_structure(
    model: &CheckedHybridModel,
    trace: &HybridTrace,
    raw: Vec<Vec<RawStep>>,
) -> Result<Vec<Vec<WireStep>>, CertificateRefusal> {
    if trace.loop_to.is_some() {
        return refuse(ROOT, RefusalCode::InvalidValue);
    }
    let base = child(ROOT, "dwells");
    if raw.len() != trace.state_count() {
        return refuse(&base, RefusalCode::InvalidValue);
    }
    let mut dwells = Vec::with_capacity(raw.len());
    for (d, steps) in raw.into_iter().enumerate() {
        let dwell_pointer = index(&base, d);
        let expected = trace.state(d).map(|state| state.mode.get());
        let mut resolved = Vec::with_capacity(steps.len());
        for (k, step) in steps.into_iter().enumerate() {
            let pointer = index(&dwell_pointer, k);
            let step = resolve_step(model, step, &pointer)?;
            if Some(step.mode) != expected {
                return refuse(&child(&pointer, "mode"), RefusalCode::InvalidValue);
            }
            if step.from.is_some() {
                return refuse(&child(&pointer, "from"), RefusalCode::InvalidValue);
            }
            resolved.push(step);
        }
        dwells.push(resolved);
    }
    Ok(dwells)
}

/// Read `value`, the `certificate` member of an entry that takes `expect`'s
/// form, against `model` (FSpec FR-652 "Checking order" steps 1 to 3).
///
/// # Errors
///
/// [`CertificateRefusal`] at the first member the schema, the form match or
/// a structural refusal rejects.
pub fn read_certificate(
    model: &CheckedHybridModel,
    expect: Expect<'_>,
    value: &Value,
) -> Result<HybridCertificate, CertificateRefusal> {
    let Value::Object(members) = value else {
        return refuse(ROOT, RefusalCode::WrongValueKind);
    };
    let type_pointer = child(ROOT, "type");
    let form = match members.get("type") {
        None => return refuse(&type_pointer, RefusalCode::MissingMember),
        Some(Value::String(form)) => form.as_str(),
        Some(_) => return refuse(&type_pointer, RefusalCode::WrongValueKind),
    };
    enum Parsed {
        Inductive(Vec<(String, WireBox)>),
        Flowpipe(Vec<RawStep>),
        Witness(Vec<Vec<RawStep>>),
    }
    let parsed = match form {
        "inductive-enclosure" => {
            let fields = object(value, &["boxes", "type"], ROOT)?;
            let boxes_pointer = child(ROOT, "boxes");
            let boxes = required(fields, "boxes", ROOT)?;
            let Value::Object(entries) = boxes else {
                return refuse(&boxes_pointer, RefusalCode::WrongValueKind);
            };
            Parsed::Inductive(
                entries
                    .iter()
                    .map(|(name, bound)| {
                        Ok((name.clone(), read_box(bound, &child(&boxes_pointer, name))?))
                    })
                    .collect::<Result<_, CertificateRefusal>>()?,
            )
        }
        "flowpipe" => {
            let fields = object(value, &["steps", "type"], ROOT)?;
            Parsed::Flowpipe(read_steps(
                required(fields, "steps", ROOT)?,
                &child(ROOT, "steps"),
            )?)
        }
        "validated-witness" => {
            let fields = object(value, &["dwells", "type"], ROOT)?;
            let dwells_pointer = child(ROOT, "dwells");
            let Value::Array(lists) = required(fields, "dwells", ROOT)? else {
                return refuse(&dwells_pointer, RefusalCode::WrongValueKind);
            };
            let mut dwells = Vec::with_capacity(lists.len());
            for (d, list) in lists.iter().enumerate() {
                let pointer = index(&dwells_pointer, d);
                let steps = read_steps(list, &pointer)?;
                if steps.is_empty() {
                    return refuse(&pointer, RefusalCode::InvalidValue);
                }
                dwells.push(steps);
            }
            Parsed::Witness(dwells)
        }
        _ => return refuse(&type_pointer, RefusalCode::InvalidValue),
    };
    let matches = matches!(
        (&parsed, expect),
        (Parsed::Flowpipe(_), Expect::Bounded)
            | (Parsed::Inductive(_), Expect::Inductive)
            | (Parsed::Witness(_), Expect::Witness(_))
    );
    if !matches {
        return refuse(ROOT, RefusalCode::InvalidValue);
    }
    let body = match (parsed, expect) {
        (Parsed::Inductive(entries), _) => {
            let boxes_pointer = child(ROOT, "boxes");
            let mut by_mode: Vec<Option<WireBox>> = vec![None; model.modes.len()];
            for (name, bound) in entries {
                let Some(mode) = mode_of(model, &name) else {
                    return refuse(&child(&boxes_pointer, &name), RefusalCode::InvalidValue);
                };
                by_mode[mode] = Some(bound);
            }
            let mut boxes = Vec::with_capacity(by_mode.len());
            for (mode, bound) in by_mode.into_iter().enumerate() {
                let Some(bound) = bound else {
                    return refuse(
                        &child(&boxes_pointer, model.modes[mode].name.as_str()),
                        RefusalCode::MissingMember,
                    );
                };
                boxes.push(bound);
            }
            Body::Inductive(boxes)
        }
        (Parsed::Flowpipe(steps), _) => Body::Flowpipe(flowpipe_structure(model, steps)?),
        (Parsed::Witness(dwells), Expect::Witness(trace)) => {
            Body::Witness(witness_structure(model, trace, dwells)?)
        }
        (Parsed::Witness(_), _) => return refuse(ROOT, RefusalCode::InvalidValue),
    };
    Ok(HybridCertificate {
        raw: value.clone(),
        body,
    })
}
