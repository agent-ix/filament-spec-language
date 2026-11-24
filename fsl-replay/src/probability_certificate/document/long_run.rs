// SPDX-License-Identifier: AGPL-3.0-or-later
//! Independent gain–bias checking of the closed DTMC long-run profiles.

use super::*;
use quire_canonical::{Encode, Error, Sink, Writer};
use std::cell::RefCell;

/// Actual long-run checking phase entered under the caller's control.
#[cfg(feature = "test-support")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LongRunProbabilityPhase {
    /// Canonical document admission.
    Admission,
    /// Complete model/product key encoding.
    Key,
    /// Checked state predicate evaluation.
    Predicate,
    /// Qualified workload expansion.
    Workload,
    /// Checked named weight and selected unit resolution.
    Unit,
    /// Actual outgoing draw, before reading its checked weight magnitude.
    Weight,
    /// Positive-support component traversal.
    Components,
    /// Supplied versus independent membership comparison.
    Membership,
    /// Exact rational sums and equations.
    Arithmetic,
}

const PROFILE: &str = "filament.probability-objective/dtmc-long-run/v1";
const WEIGHTED_PROFILE: &str = "filament.probability-objective/dtmc-weighted-long-run/v1";

/// Candidate evidence for one complete recurrent component.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LongRunProbabilityComponent {
    /// Long-run fraction of the checked indicator in this component.
    pub value: Rational,
    /// Selected long-run ratio marker, equal to `value`.
    pub gain: Rational,
    /// Signed biases in strictly ascending complete-key order, first bias zero.
    pub bias: Vec<ExactProbabilityValue>,
    /// The component's policy: present exactly under `minimum` or `maximum`,
    /// with one row per state of the component in ascending key order (FR-412
    /// "Long-run certificate wire shape and witnesses").
    pub policy: Option<Vec<LongRunPolicyRow>>,
}

/// One state's action in a component's policy.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LongRunPolicyRow {
    /// The complete product key of the state.
    pub state: ProbabilityProductKey,
    /// The action the state takes: its scheduled identity, or a terminal
    /// stutter at a state with no action.
    pub choice: crate::ProductAction,
}

/// The scheduler optimisation a long-run document is written for (FR-412
/// "Long-run certificate wire shape and witnesses").
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LongRunOptimization {
    /// `under W`: the workload's single chain.
    Workload,
    /// `under every scheduler` with a `>=` bound.
    Minimum,
    /// `under every scheduler` with a `<=` bound.
    Maximum,
}

impl LongRunOptimization {
    fn as_str(self) -> &'static str {
        match self {
            Self::Workload => "workload",
            Self::Minimum => "minimum",
            Self::Maximum => "maximum",
        }
    }

    fn of(scheduler: &ProbScheduler, bound: &Bound) -> Self {
        match (scheduler, bound) {
            (ProbScheduler::Workload(_), _) => Self::Workload,
            (ProbScheduler::Every, Bound::AtLeast(_)) => Self::Minimum,
            (ProbScheduler::Every, Bound::AtMost(_)) => Self::Maximum,
        }
    }
}

/// Canonical long-run document; construction alone grants no acceptance.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LongRunProbabilityDocument {
    /// Structural checked claim identity.
    pub identity: ObligationIdentity,
    /// The scheduler optimisation the document is written for.
    pub optimization: LongRunOptimization,
    /// The claim's fairness set, empty unless `optimization` is not `Workload`.
    pub(crate) fairness: Vec<super::fairness::Record>,
    /// Exact reachable keys where the independently checked predicate holds.
    pub predicate_states: Vec<ProbabilityProductKey>,
    /// Exact long-run values on the whole original reachable graph.
    pub values: Vec<ExactProbabilityValue>,
    /// Every bottom component, ordered by its first complete bias key.
    pub components: Vec<LongRunProbabilityComponent>,
}

/// Weighted long-run evidence, retaining the original unweighted row representation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WeightedLongRunProbabilityDocument {
    /// Common identity, predicate, values and recurrent-component evidence.
    pub certificate: LongRunProbabilityDocument,
    /// Actual checked weight identity and selected magnitude unit.
    pub weight: ProbabilityRewardObjective,
}

struct Decoded {
    document: LongRunProbabilityDocument,
    weight: Option<ProbabilityRewardObjective>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct WeightedObjective {
    profile: String,
    predicate_states: Vec<String>,
    weight: (Box<serde_json::value::RawValue>, String),
    unit: Box<serde_json::value::RawValue>,
    optimization: String,
    fairness: Vec<Box<serde_json::value::RawValue>>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Objective {
    profile: String,
    predicate_states: Vec<String>,
    optimization: String,
    fairness: Vec<Box<serde_json::value::RawValue>>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Component {
    value: WireRational,
    gain: WireRational,
    bias: Vec<WireValue>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    policy: Option<Vec<WirePolicy>>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct WirePolicy {
    state: String,
    choice: Box<serde_json::value::RawValue>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Document<O = Objective> {
    identity: WireIdentity,
    objective: O,
    kind: String,
    values: Vec<WireValue>,
    components: Vec<Component>,
}

pub(super) fn is_profile(profile: &str) -> bool {
    matches!(profile, PROFILE | WEIGHTED_PROFILE)
}

pub(super) fn certificate_identity(
    bytes: &[u8],
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
) -> Result<ObligationIdentity, ProbabilityDocumentError> {
    let mut cx = Context::new(limits, cancel);
    Ok(read(bytes, &mut cx)?.document.identity)
}

/// Read canonical long-run bytes without trusting their component partition.
pub fn read_long_run_probability_certificate(
    bytes: &[u8],
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
) -> Result<LongRunProbabilityDocument, ProbabilityDocumentError> {
    let mut cx = Context::new(limits, cancel);
    let decoded = read(bytes, &mut cx)?;
    if decoded.weight.is_some() {
        return Err(ProbabilityDocumentError::Unsupported);
    }
    Ok(decoded.document)
}

/// Read the strict weighted objective without trusting component evidence.
pub fn read_weighted_long_run_probability_certificate(
    bytes: &[u8],
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
) -> Result<WeightedLongRunProbabilityDocument, ProbabilityDocumentError> {
    let mut cx = Context::new(limits, cancel);
    let decoded = read(bytes, &mut cx)?;
    Ok(WeightedLongRunProbabilityDocument {
        certificate: decoded.document,
        weight: decoded
            .weight
            .ok_or(ProbabilityDocumentError::Unsupported)?,
    })
}

#[ix_trace_rs::string_edge]
fn read(bytes: &[u8], cx: &mut Context<'_>) -> Result<Decoded, ProbabilityDocumentError> {
    #[cfg(feature = "test-support")]
    cx.phase(LongRunProbabilityPhase::Admission)?;
    if profile_dispatch(bytes, cx.limits, cx.cancel)? != ProfileDispatch::LongRun {
        return Err(ProbabilityDocumentError::Unsupported);
    }
    // Account wire strings, parsed rows and decoded key storage before serde growth.
    cx.reserve(bytes.len().saturating_mul(16))?;
    cx.work(bytes.len() as u64)?;
    let parsed = quire_canonical::read_bounded(bytes, cx.limits.bytes, cx.limits.storage_bytes)
        .map_err(|_| ProbabilityDocumentError::Malformed)?;
    admit_members(parsed.root(), cx)?;
    let weighted = matches!(
        parsed
            .root()
            .get("objective")
            .and_then(|o| o.get("profile"))
            .map(|p| p.node()),
        Some(quire_canonical::Node::String(WEIGHTED_PROFILE))
    );
    drop(parsed);
    let (wire, weight) = if weighted {
        let wire: Document<WeightedObjective> =
            serde_json::from_slice(bytes).map_err(|_| ProbabilityDocumentError::Malformed)?;
        if canonical(&wire, cx.limits.bytes)? != bytes {
            return Err(ProbabilityDocumentError::Noncanonical);
        }
        let weight = ProbabilityRewardObjective {
            model: fsl_foundation::node_wire::node_key_from_text(wire.objective.weight.0.get())
                .map_err(|_| ProbabilityDocumentError::Malformed)?,
            name: filament_exact::Identifier::new(wire.objective.weight.1)
                .map_err(|_| ProbabilityDocumentError::Malformed)?,
            unit: unit::read(wire.objective.unit.get().as_bytes(), cx.limits, cx.cancel)?,
        };
        (
            Document {
                identity: wire.identity,
                objective: Objective {
                    profile: wire.objective.profile,
                    predicate_states: wire.objective.predicate_states,
                    optimization: wire.objective.optimization,
                    fairness: wire.objective.fairness,
                },
                kind: wire.kind,
                values: wire.values,
                components: wire.components,
            },
            Some(weight),
        )
    } else {
        let wire: Document =
            serde_json::from_slice(bytes).map_err(|_| ProbabilityDocumentError::Malformed)?;
        if canonical(&wire, cx.limits.bytes)? != bytes {
            return Err(ProbabilityDocumentError::Noncanonical);
        }
        (wire, None)
    };
    if wire.kind != "long-run"
        || !matches!(wire.objective.profile.as_str(), PROFILE | WEIGHTED_PROFILE)
    {
        return Err(ProbabilityDocumentError::Unsupported);
    }
    let optimization = match wire.objective.optimization.as_str() {
        "workload" => LongRunOptimization::Workload,
        "minimum" => LongRunOptimization::Minimum,
        "maximum" => LongRunOptimization::Maximum,
        _ => return Err(ProbabilityDocumentError::Unsupported),
    };
    // A fairness set belongs to `every scheduler`, and to the unweighted form.
    if !wire.objective.fairness.is_empty()
        && (optimization == LongRunOptimization::Workload || weight.is_some())
    {
        return Err(ProbabilityDocumentError::Unsupported);
    }
    let fairness = super::fairness::read(
        &wire.objective.fairness,
        cx.limits,
        cx.cancel,
        &mut cx.meter,
        &mut cx.storage,
    )?;
    cx.rows(wire.values.len())?;
    cx.rows(wire.objective.predicate_states.len())?;
    cx.rows(wire.components.len())?;
    let mut total = 0usize;
    for component in &wire.components {
        total = total.saturating_add(component.bias.len());
    }
    cx.rows(total)?;
    let node = fsl_foundation::node_wire::node_key_from_text(wire.identity.node_id.get())
        .map_err(|_| ProbabilityDocumentError::Malformed)?;
    let identity = ObligationIdentity::new(
        node,
        wire.identity.model.map(crate::ModelIdentity::new),
        wire.identity.ordinal,
    );
    let predicate_states = ordered(
        wire.objective
            .predicate_states
            .iter()
            .map(|key| decode_key(key, cx.limits))
            .collect::<Result<Vec<_>, _>>()?,
    )?;
    let values = read_rows(wire.values, true, cx)?;
    let mut components = Vec::new();
    for component in wire.components {
        cx.work(1)?;
        let value = ratio(
            component.value,
            cx.limits.scalar_bits,
            &mut cx.meter,
            cx.cancel,
        )?;
        let gain = ratio_checked(
            component.gain,
            cx.limits.scalar_bits,
            &mut cx.meter,
            cx.cancel,
            false,
        )?;
        let bias = read_rows(component.bias, false, cx)?;
        if bias.is_empty()
            || components
                .last()
                .is_some_and(|previous: &LongRunProbabilityComponent| {
                    previous.bias[0].state >= bias[0].state
                })
        {
            return Err(ProbabilityDocumentError::MalformedMember("components"));
        }
        // Whether `policy` is present exactly under `minimum` or `maximum`
        // (FR-412) is decided against the checked claim, after its objective.
        let policy = match component.policy {
            None => None,
            Some(rows) => {
                cx.rows(rows.len())?;
                let mut policy: Vec<LongRunPolicyRow> = Vec::new();
                for row in rows {
                    cx.work(1)?;
                    let state = decode_key(&row.state, cx.limits)?;
                    // A duplicated or unordered row is not a policy.
                    if policy.last().is_some_and(|last| last.state >= state) {
                        return Err(ProbabilityDocumentError::MalformedMember("policy"));
                    }
                    let choice = crate::ProductAction::parse(row.choice.get().as_bytes())
                        .map_err(|_| ProbabilityDocumentError::MalformedMember("policy"))?;
                    policy.push(LongRunPolicyRow { state, choice });
                }
                Some(policy)
            }
        };
        components.push(LongRunProbabilityComponent {
            value,
            gain,
            bias,
            policy,
        });
    }
    Ok(Decoded {
        document: LongRunProbabilityDocument {
            identity,
            optimization,
            fairness,
            predicate_states,
            values,
            components,
        },
        weight,
    })
}

// Keep structural rejection loci separate from JSON and scalar syntax errors.
fn admit_members(
    root: quire_canonical::NodeRef<'_>,
    cx: &mut Context<'_>,
) -> Result<(), ProbabilityDocumentError> {
    fn rows(
        node: Option<quire_canonical::NodeRef<'_>>,
        member: &'static str,
        cx: &mut Context<'_>,
    ) -> Result<(), ProbabilityDocumentError> {
        let Some(quire_canonical::Node::Array(rows)) = node.map(|node| node.node()) else {
            return Err(ProbabilityDocumentError::MalformedMember(member));
        };
        for row in rows {
            cx.work(1)?;
            if row.get("state").is_none() || row.get("value").is_none() {
                return Err(ProbabilityDocumentError::MalformedMember(member));
            }
        }
        Ok(())
    }
    rows(root.get("values"), "values", cx)?;
    let components = root
        .get("components")
        .ok_or(ProbabilityDocumentError::MissingMember("components"))?;
    let quire_canonical::Node::Array(components) = components.node() else {
        return Err(ProbabilityDocumentError::MalformedMember("components"));
    };
    for component in components {
        cx.work(1)?;
        if component.get("value").is_none() || component.get("gain").is_none() {
            return Err(ProbabilityDocumentError::MalformedMember("components"));
        }
        rows(component.get("bias"), "components", cx)?;
    }
    Ok(())
}

fn read_rows(
    rows: Vec<WireValue>,
    probability: bool,
    cx: &mut Context<'_>,
) -> Result<Vec<ExactProbabilityValue>, ProbabilityDocumentError> {
    let mut values = Vec::new();
    for row in rows {
        cx.work(1)?;
        let state = decode_key(&row.state, cx.limits)?;
        if values
            .last()
            .is_some_and(|previous: &ExactProbabilityValue| previous.state >= state)
        {
            return Err(ProbabilityDocumentError::MalformedMember(if probability {
                "values"
            } else {
                "components"
            }));
        }
        let value = ratio_checked(
            row.value,
            cx.limits.scalar_bits,
            &mut cx.meter,
            cx.cancel,
            probability,
        )?;
        values.push(ExactProbabilityValue { state, value });
    }
    Ok(values)
}

struct Events<'a> {
    document: &'a LongRunProbabilityDocument,
    weight: Option<&'a ProbabilityRewardObjective>,
    text: writer::Text<'a>,
    /// Each component's policy choices as canonical documents, in row order.
    choices: Vec<Vec<quire_canonical::Document>>,
}

/// Every policy choice of `document` as a canonical document, ready to embed.
fn policy_choices(
    document: &LongRunProbabilityDocument,
    limits: ProbabilityDocumentLimits,
) -> Result<Vec<Vec<quire_canonical::Document>>, ProbabilityDocumentError> {
    let mut all = Vec::new();
    for component in &document.components {
        let mut rows = Vec::new();
        for row in component.policy.iter().flatten() {
            let bytes = row
                .choice
                .to_bytes()
                .map_err(|_| ProbabilityDocumentError::MalformedMember("policy"))?;
            rows.push(
                quire_canonical::read_bounded(&bytes, limits.bytes, limits.storage_bytes)
                    .map_err(|_| ProbabilityDocumentError::MalformedMember("policy"))?,
            );
        }
        all.push(rows);
    }
    Ok(all)
}
impl Events<'_> {
    fn ratio<S: Sink + ?Sized>(
        &self,
        value: &Rational,
        writer: &mut Writer<'_, S>,
    ) -> Result<(), Error> {
        writer.begin_object()?;
        writer.name("numerator")?;
        self.text.integer(value.numerator(), writer)?;
        writer.name("denominator")?;
        self.text.integer(value.denominator(), writer)?;
        writer.end_object()
    }
    fn rows<S: Sink + ?Sized>(
        &self,
        rows: &[ExactProbabilityValue],
        writer: &mut Writer<'_, S>,
    ) -> Result<(), Error> {
        writer.begin_array()?;
        for row in rows {
            self.text.poll()?;
            writer.begin_object()?;
            writer.name("state")?;
            self.text.key(&row.state, writer, false)?;
            writer.name("value")?;
            self.ratio(&row.value, writer)?;
            writer.end_object()?;
        }
        writer.end_array()
    }
}
impl Encode for Events<'_> {
    fn encode_into<S: Sink + ?Sized>(&self, writer: &mut Writer<'_, S>) -> Result<(), Error> {
        self.text.poll()?;
        writer.begin_object()?;
        writer.name("identity")?;
        writer.begin_object()?;
        writer.name("node_id")?;
        fsl_foundation::node_wire::encode_node_key(self.document.identity.node(), writer)?;
        writer.name("model")?;
        match self.document.identity.model() {
            Some(model) => writer.string(model.as_str())?,
            None => writer.null()?,
        }
        writer.name("ordinal")?;
        writer.integer(i128::from(self.document.identity.ordinal()))?;
        writer.end_object()?;
        writer.name("objective")?;
        writer.begin_object()?;
        writer.name("profile")?;
        writer.string(if self.weight.is_some() {
            WEIGHTED_PROFILE
        } else {
            PROFILE
        })?;
        if let Some(weight) = self.weight {
            writer.name("weight")?;
            writer.begin_array()?;
            fsl_foundation::node_wire::encode_node_key(&weight.model, writer)?;
            writer.string(weight.name.as_str())?;
            writer.end_array()?;
            writer.name("unit")?;
            self.text.unit(weight.unit.as_ref(), writer)?;
        }
        writer.name("predicate_states")?;
        writer.begin_array()?;
        for (index, key) in self.document.predicate_states.iter().enumerate() {
            self.text.key(key, writer, index != 0)?;
        }
        writer.end_array()?;
        writer.name("optimization")?;
        writer.string(self.document.optimization.as_str())?;
        writer.name("fairness")?;
        writer.begin_array()?;
        for record in &self.document.fairness {
            self.text.poll()?;
            record.encode_into(writer)?;
        }
        writer.end_array()?;
        writer.end_object()?;
        writer.name("kind")?;
        writer.string("long-run")?;
        writer.name("values")?;
        self.rows(&self.document.values, writer)?;
        writer.name("components")?;
        writer.begin_array()?;
        for (index, component) in self.document.components.iter().enumerate() {
            writer.begin_object()?;
            writer.name("value")?;
            self.ratio(&component.value, writer)?;
            writer.name("gain")?;
            self.ratio(&component.gain, writer)?;
            writer.name("bias")?;
            self.rows(&component.bias, writer)?;
            if let (Some(policy), Some(choices)) = (&component.policy, self.choices.get(index)) {
                writer.name("policy")?;
                writer.begin_array()?;
                for (row, choice) in policy.iter().zip(choices) {
                    self.text.poll()?;
                    writer.begin_object()?;
                    writer.name("state")?;
                    self.text.key(&row.state, writer, false)?;
                    writer.name("choice")?;
                    choice.root().encode_into(writer)?;
                    writer.end_object()?;
                }
                writer.end_array()?;
            }
            writer.end_object()?;
        }
        writer.end_array()?;
        writer.end_object()
    }
}

/// Write only the canonical, structurally admitted long-run representation.
pub fn write_long_run_probability_certificate(
    document: &LongRunProbabilityDocument,
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
) -> Result<Vec<u8>, ProbabilityDocumentError> {
    poll(cancel)?;
    let events = Events {
        document,
        weight: None,
        text: writer::Text::new(limits, cancel),
        choices: policy_choices(document, limits)?,
    };
    let admission = RefCell::new(writer::Admission::new(limits.storage_bytes, cancel));
    let result = writer::encode_reserved(&events, limits.bytes, &admission);
    if let Some(error) = events.text.take_stop() {
        return Err(error);
    }
    let bytes = result?;
    read_long_run_probability_certificate(&bytes, limits, cancel)?;
    Ok(bytes)
}

/// Write a canonical weighted long-run document; construction grants no verdict.
pub fn write_weighted_long_run_probability_certificate(
    document: &WeightedLongRunProbabilityDocument,
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
) -> Result<Vec<u8>, ProbabilityDocumentError> {
    poll(cancel)?;
    let events = Events {
        document: &document.certificate,
        weight: Some(&document.weight),
        text: writer::Text::new(limits, cancel),
        choices: policy_choices(&document.certificate, limits)?,
    };
    let admission = RefCell::new(writer::Admission::new(limits.storage_bytes, cancel));
    let result = writer::encode_reserved(&events, limits.bytes, &admission);
    if let Some(error) = events.text.take_stop() {
        return Err(error);
    }
    let bytes = result?;
    read_weighted_long_run_probability_certificate(&bytes, limits, cancel)?;
    Ok(bytes)
}

struct Subject {
    graph: CheckerGraph,
    optimization: LongRunOptimization,
    /// The claim's fairness set as canonical records, and as checked constraints.
    fairness: Vec<super::fairness::Record>,
    constraints: Vec<filament_semantic_value::fairness::FairnessConstraint>,
    reference: Option<ObjectReference>,
    bound: Bound,
    weight: Option<ProbabilityRewardObjective>,
    weights: BTreeMap<ProbabilityProductKey, Rational>,
}

struct Context<'a> {
    limits: ProbabilityDocumentLimits,
    cancel: &'a Cancel,
    meter: filament_exact::Meter,
    storage: u64,
    identity: fsl_foundation::IdentityLimits,
    /// Compare the initial value strictly against the failing side of the bound.
    refutation: bool,
    /// The chain is the one a scheduler witness induces over every scheduler.
    scheduled: bool,
    #[cfg(feature = "test-support")]
    observe: Option<&'a mut dyn FnMut(LongRunProbabilityPhase)>,
}
impl<'a> Context<'a> {
    fn new(limits: ProbabilityDocumentLimits, cancel: &'a Cancel) -> Self {
        Self {
            limits,
            cancel,
            meter: filament_exact::Meter::new(limits.accounting).with_cancel(cancel.clone()),
            storage: 0,
            identity: fsl_foundation::IdentityLimits::default(),
            refutation: false,
            scheduled: false,
            #[cfg(feature = "test-support")]
            observe: None,
        }
    }
    #[cfg(feature = "test-support")]
    fn phase(&mut self, phase: LongRunProbabilityPhase) -> Result<(), ProbabilityDocumentError> {
        if let Some(observe) = self.observe.as_mut() {
            observe(phase);
        }
        poll(self.cancel)
    }
    fn rows(&self, n: usize) -> Result<(), ProbabilityDocumentError> {
        if n > self.limits.states {
            return Err(exceeded(
                ProbabilityDocumentLimit::States,
                self.limits.states as u64,
                n as u64,
            ));
        }
        Ok(())
    }
    fn reserve(&mut self, bytes: usize) -> Result<(), ProbabilityDocumentError> {
        poll(self.cancel)?;
        let actual = self.storage.saturating_add(bytes as u64);
        if actual > self.limits.storage_bytes {
            return Err(exceeded(
                ProbabilityDocumentLimit::StorageBytes,
                self.limits.storage_bytes,
                actual,
            ));
        }
        self.storage = actual;
        Ok(())
    }
    fn work(&mut self, n: u64) -> Result<(), ProbabilityDocumentError> {
        poll(self.cancel)?;
        self.meter
            .charge(
                filament_exact::Charge::new(filament_exact::ChargePoint::GraphEdge)
                    .work(Integer::from(n)),
            )
            .map_err(|error| match self.cancel.tripped() {
                Some(cause) => ProbabilityDocumentError::Cancelled(cause),
                None => ProbabilityDocumentError::Accounting(Box::new(error)),
            })
    }
    /// Every enabled action of `state` as one distribution each, under the
    /// document's meter and storage.
    fn distributions(
        &mut self,
        system: &ModelSystem<'_>,
        state: &ModelSimState,
    ) -> Result<Vec<Vec<(Rational, ModelSimState)>>, ProbabilityDocumentError> {
        let mut admission = ProbabilityKeyAdmission::new(
            &mut self.meter,
            &mut self.storage,
            self.limits.storage_bytes,
            self.cancel,
        );
        super::super::model_distributions_with_budget(
            system,
            state,
            None,
            self.identity,
            &mut admission,
            self.cancel,
        )
    }

    fn model<T>(
        &mut self,
        operation: impl FnOnce(
            &mut dyn fsl_eval::simulation::model::ModelKeyBudget,
        )
            -> Result<T, fsl_eval::simulation::model::ModelKeyExpansionError>,
    ) -> Result<T, ProbabilityDocumentError> {
        let mut admission = ProbabilityKeyAdmission::new(
            &mut self.meter,
            &mut self.storage,
            self.limits.storage_bytes,
            self.cancel,
        );
        operation(&mut admission).map_err(|error| admission.expansion_refusal(error))
    }

    fn arithmetic(
        &mut self,
        operation: filament_exact::RationalArithmetic<'_>,
    ) -> Result<Rational, ProbabilityDocumentError> {
        #[cfg(feature = "test-support")]
        self.phase(LongRunProbabilityPhase::Arithmetic)?;
        let value = arithmetic(operation, &mut self.meter, self.cancel)?;
        check_scalar(&value, self.limits)?;
        Ok(value)
    }
}

fn subject(
    system: &ModelSystem<'_>,
    claims: &[CheckedProbabilisticClaim],
    item: &ObligationIdentity,
    cx: &mut Context<'_>,
) -> Result<Subject, ProbabilityDocumentError> {
    use fsl_eval::simulation::model::{TimedSubjectRefusal, TimedSystem};
    if !std::ptr::eq(system.input().probabilistic.claims(), claims) {
        return Err(ProbabilityDocumentError::IdentityMismatch);
    }
    let claim = bind_claim(claims, item, cx.cancel)?;
    let ProbForm::LongRunFraction {
        bound,
        predicate,
        weight,
    } = &claim.form
    else {
        return Err(ProbabilityDocumentError::Unsupported);
    };
    if fsl_semantics::check::TemporalProfile::from_identity(claim.profile.identity())
        != Some(fsl_semantics::check::TemporalProfile::InfiniteTrace)
        || !matches!(
            TimedSystem::new(system, &claim.subject.alias),
            Err(TimedSubjectRefusal::NoTimedModel { .. })
        )
    {
        return Err(ProbabilityDocumentError::Unsupported);
    }
    let binding = claim_binding(system, claim)?;
    let reference = binding.as_ref().map(|b| &b.reference);
    // `None` is `under every scheduler`: the graph keeps every action as a choice.
    let workload = claim_workload(system, claim)?;
    if workload.is_none() && weight.is_some() {
        // Per-action weights are the MDP weighted profile's, not this one's.
        return Err(ProbabilityDocumentError::Unsupported);
    }
    let optimization = LongRunOptimization::of(&claim.scheduler, bound);
    let fairness = if claim.fairness.is_empty() {
        Vec::new()
    } else {
        super::fairness::records(
            system,
            claim,
            cx.limits,
            cx.cancel,
            &mut cx.meter,
            &mut cx.storage,
            &super::fairness::Control::new(cx.cancel),
        )?
    };
    let limits = cx.limits;
    let cancel = cx.cancel;
    let identity = cx.identity;
    let objective = match weight {
        // FR-186 delay weighting reads a timed subject, which the guard above
        // already refused: this certificate covers untimed workloads only.
        Some(fsl_semantics::check::Weighting::Delay) => {
            return Err(ProbabilityDocumentError::Unsupported);
        }
        Some(fsl_semantics::check::Weighting::Reward(name)) => {
            #[cfg(feature = "test-support")]
            cx.phase(LongRunProbabilityPhase::Unit)?;
            cx.work(1)?;
            Some(super::super::reward::objective(
                system,
                claim,
                name,
                limits,
                cancel,
                &mut cx.storage,
            )?)
        }
        None => None,
    };
    let mut weights = BTreeMap::new();
    let mut remaining_weight_edges = limits.edges;
    let cx = RefCell::new(cx);
    let stopped = RefCell::new(None);
    let key_of = |state: &ModelSimState| {
        let encoded = (|| {
            #[cfg(feature = "test-support")]
            cx.borrow_mut().phase(LongRunProbabilityPhase::Key)?;
            cx.borrow_mut().work(1)?;
            cx.borrow_mut().model(|budget| {
                use fsl_eval::simulation::model::ModelKeyExpansionError as Error;
                let bytes = system
                    .state_key_bytes_with_budget(state, identity, budget, cancel)
                    .map_err(Error::Key)?;
                ProbabilityProductKey::for_model_state_with_budget(
                    reference, &bytes, identity, budget, cancel,
                )
                .map_err(Error::Key)
            })
        })();
        encoded.map_err(|error| {
            *stopped.borrow_mut() = Some(error);
            ExactProbabilityGraphError::ExpansionRefused
        })
    };
    let mut predicate_keys = BTreeSet::new();
    let mut issue = None;
    let graph = enumerate_model_graph_with_distributions(
        system,
        None::<&fsl_semantics::check::CheckedPredicate>,
        reference,
        limits.states,
        limits.edges,
        cancel,
        false,
        &key_of,
        &mut |bytes| {
            let result = {
                let mut context = cx.borrow_mut();
                context.reserve(bytes).and_then(|()| context.work(1))
            };
            result.map_err(|error| {
                *stopped.borrow_mut() = Some(error);
                ExactProbabilityGraphError::ExpansionRefused
            })
        },
        &mut |state, failure| {
            let evaluated = (|| {
                #[cfg(feature = "test-support")]
                cx.borrow_mut().phase(LongRunProbabilityPhase::Workload)?;
                let Some(workload) = workload else {
                    return cx.borrow_mut().distributions(system, state);
                };
                let steps = cx.borrow_mut().model(|budget| {
                    system.weighted_steps_with_budget(state, workload, identity, budget, cancel)
                })?;
                let steps = steps.map_err(|stop| {
                    failure(match stop {
                        fsl_eval::simulation::model::Unsteppable::NotMarkov(cause) => {
                            crate::ExactProbBuildIssue::NotMarkov(cause)
                        }
                        fsl_eval::simulation::model::Unsteppable::UndecidedSuccessor(_) => {
                            crate::ExactProbBuildIssue::UndecidedSuccessor
                        }
                    });
                    ProbabilityDocumentError::Graph(ExactProbabilityGraphError::ExpansionRefused)
                })?;
                if steps.iter().any(|step| {
                    matches!(
                        step.transition,
                        fsl_eval::simulation::model::TransitionId::Stutter
                    )
                }) {
                    if steps.len() == 1 {
                        return Ok(Vec::new());
                    }
                    return Err(ProbabilityDocumentError::Graph(
                        ExactProbabilityGraphError::UnsupportedDraw,
                    ));
                }
                Ok(vec![steps
                    .into_iter()
                    .map(|step| (step.probability, step.post))
                    .collect()])
            })();
            evaluated.map_err(|error| {
                *stopped.borrow_mut() = Some(error);
                ExactProbabilityGraphError::ExpansionRefused
            })
        },
        &mut |key, state| {
            let evaluated = (|| {
                if let Some(objective) = &objective {
                    let workload = workload.ok_or(ProbabilityDocumentError::Unsupported)?;
                    let mut context = cx.borrow_mut();
                    let steps = context
                        .model(|budget| {
                            system.weighted_steps_with_budget(
                                state, workload, identity, budget, cancel,
                            )
                        })?
                        .map_err(|stop| {
                            ProbabilityDocumentError::Build(Box::new(match stop {
                                fsl_eval::simulation::model::Unsteppable::NotMarkov(cause) => {
                                    crate::ExactProbBuildIssue::NotMarkov(cause)
                                }
                                fsl_eval::simulation::model::Unsteppable::UndecidedSuccessor(_) => {
                                    crate::ExactProbBuildIssue::UndecidedSuccessor
                                }
                            }))
                        })?;
                    let mut total = rational(0);
                    for step in steps {
                        context.work(1)?;
                        if matches!(
                            step.transition,
                            fsl_eval::simulation::model::TransitionId::Stutter
                        ) {
                            continue;
                        }
                        #[cfg(feature = "test-support")]
                        context.phase(LongRunProbabilityPhase::Weight)?;
                        let magnitude = {
                            let context = &mut **context;
                            super::super::reward::magnitude(
                                &step.rewards,
                                objective,
                                limits,
                                cancel,
                                &mut context.storage,
                                &mut remaining_weight_edges,
                                &mut context.meter,
                            )?
                        };
                        if step.probability == rational(0) {
                            continue;
                        }
                        let term =
                            context.arithmetic(filament_exact::RationalArithmetic::Multiply(
                                &step.probability,
                                &magnitude,
                            ))?;
                        total = context
                            .arithmetic(filament_exact::RationalArithmetic::Add(&total, &term))?;
                    }
                    context.reserve(key.as_bytes().len().saturating_add(std::mem::size_of::<(
                        ProbabilityProductKey,
                        Rational,
                    )>(
                    )))?;
                    weights.insert(key.clone(), total);
                }
                #[cfg(feature = "test-support")]
                cx.borrow_mut().phase(LongRunProbabilityPhase::Predicate)?;
                cx.borrow_mut().work(1)?;
                let readings = system
                    .state_predicates_with_cancel(state, &[(predicate, reference)], cancel)
                    .map_err(|stop| {
                        ProbabilityDocumentError::Build(Box::new(
                            crate::ExactProbBuildIssue::Expansion(stop),
                        ))
                    })?;
                poll(cancel)?;
                match readings
                    .into_iter()
                    .next()
                    .ok_or(ProbabilityDocumentError::Malformed)?
                {
                    fsl_eval::simulation::model::PredicateReading::True => {
                        cx.borrow_mut().reserve(
                            key.as_bytes()
                                .len()
                                .saturating_add(std::mem::size_of::<ProbabilityProductKey>()),
                        )?;
                        predicate_keys.insert(key.clone());
                    }
                    fsl_eval::simulation::model::PredicateReading::False => {}
                    fsl_eval::simulation::model::PredicateReading::Undefined { cause, .. } => {
                        return Err(ProbabilityDocumentError::UndefinedPredicate(cause))
                    }
                    fsl_eval::simulation::model::PredicateReading::Refused(outcome) => {
                        return Err(ProbabilityDocumentError::Build(Box::new(
                            crate::ExactProbBuildIssue::Predicate(outcome),
                        )))
                    }
                }
                Ok(())
            })();
            evaluated.map_err(|error| {
                *stopped.borrow_mut() = Some(error);
                ExactProbabilityGraphError::ExpansionRefused
            })
        },
        &mut |error| issue = Some(error),
    );
    if let Some(error) = stopped.borrow_mut().take() {
        return Err(error);
    }
    if let Some(issue) = issue {
        return Err(ProbabilityDocumentError::Build(Box::new(issue)));
    }
    let mut graph = graph.map_err(|error| match error {
        ExactProbabilityGraphError::LimitReached => exceeded(
            ProbabilityDocumentLimit::States,
            limits.states as u64,
            (limits.states as u64).saturating_add(1),
        ),
        ExactProbabilityGraphError::EdgeLimitReached => exceeded(
            ProbabilityDocumentLimit::Edges,
            limits.edges as u64,
            (limits.edges as u64).saturating_add(1),
        ),
        other => other.into(),
    })?;
    for (key, state) in &mut graph.states {
        cx.borrow_mut().work(1)?;
        if predicate_keys.contains(key) {
            state.terminal = Some(true);
        }
    }
    Ok(Subject {
        graph,
        optimization,
        fairness,
        constraints: claim.fairness.clone(),
        reference: reference.cloned(),
        bound: bound.clone(),
        weight: objective,
        weights,
    })
}

/// Build candidate transport with independently derived predicate keys.
/// Component membership and equations still require the independent checker.
pub fn long_run_probability_certificate_document(
    system: &ModelSystem<'_>,
    claims: &[CheckedProbabilisticClaim],
    item: &ObligationIdentity,
    values: Vec<ExactProbabilityValue>,
    components: Vec<LongRunProbabilityComponent>,
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
) -> Result<LongRunProbabilityDocument, ProbabilityDocumentError> {
    let mut cx = Context::new(limits, cancel);
    let subject = subject(system, claims, item, &mut cx)?;
    if subject.weight.is_some() {
        return Err(ProbabilityDocumentError::Unsupported);
    }
    let graph = subject.graph;
    let predicate_bytes = graph
        .states
        .iter()
        .filter(|(_, state)| state.terminal == Some(true))
        .fold(0usize, |bytes, (key, _)| {
            bytes
                .saturating_add(key.as_bytes().len())
                .saturating_add(std::mem::size_of::<ProbabilityProductKey>())
        });
    cx.reserve(predicate_bytes)?;
    Ok(LongRunProbabilityDocument {
        identity: item.clone(),
        optimization: subject.optimization,
        fairness: subject.fairness,
        predicate_states: target_keys(&graph),
        values,
        components,
    })
}

/// Construct weighted transport from independently admitted checked objectives.
/// The returned rows still require independent component/equation checking.
pub fn weighted_long_run_probability_certificate_document(
    system: &ModelSystem<'_>,
    claims: &[CheckedProbabilisticClaim],
    item: &ObligationIdentity,
    values: Vec<ExactProbabilityValue>,
    components: Vec<LongRunProbabilityComponent>,
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
) -> Result<WeightedLongRunProbabilityDocument, ProbabilityDocumentError> {
    let mut cx = Context::new(limits, cancel);
    let subject = subject(system, claims, item, &mut cx)?;
    let weight = subject
        .weight
        .ok_or(ProbabilityDocumentError::Unsupported)?;
    let predicate_bytes = subject
        .graph
        .states
        .iter()
        .filter(|(_, s)| s.terminal == Some(true))
        .fold(0usize, |n, (key, _)| {
            n.saturating_add(key.as_bytes().len())
                .saturating_add(std::mem::size_of::<ProbabilityProductKey>())
        });
    cx.reserve(predicate_bytes)?;
    Ok(WeightedLongRunProbabilityDocument {
        certificate: LongRunProbabilityDocument {
            identity: item.clone(),
            optimization: subject.optimization,
            fairness: subject.fairness.clone(),
            predicate_states: target_keys(&subject.graph),
            values,
            components,
        },
        weight,
    })
}

fn malformed(member: &'static str) -> ExactProbabilityCertificateCheck {
    ExactProbabilityCertificateCheck::Rejected(ProbabilityCertificateRejection {
        rule: ProbabilityCertificateRule::Malformed,
        at: ProbabilityCertificateLocus::Member(member),
    })
}

pub(super) fn check(
    system: &ModelSystem<'_>,
    claims: &[CheckedProbabilisticClaim],
    item: &ObligationIdentity,
    bytes: &[u8],
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
) -> Result<ExactProbabilityCertificateCheck, ProbabilityDocumentError> {
    let mut cx = Context::new(limits, cancel);
    check_with_context(system, claims, item, bytes, &mut cx)
}

/// FR-202's counterexample role: every proof-side condition over the rebuilt
/// workload chain, then the strict refutation comparison instead of the
/// inclusive claim-side threshold. Acceptance means the claim is refuted.
///
/// Under `every scheduler` the chain is the one `scheduler` induces (FR-412
/// "Long-run certificate wire shape and witnesses"); under a workload it must
/// be empty.
pub(in crate::probability_certificate) fn check_refutation(
    system: &ModelSystem<'_>,
    claims: &[CheckedProbabilisticClaim],
    item: &ObligationIdentity,
    bytes: &[u8],
    scheduler: &[(ProbabilityProductKey, crate::ProductAction)],
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
) -> Result<LongRunRefutation, ProbabilityDocumentError> {
    let mut cx = Context::new(limits, cancel);
    cx.refutation = true;
    check_scheduled(system, claims, item, bytes, scheduler, &mut cx)
}

/// The outcome of checking long-run evidence in its counterexample role.
pub(in crate::probability_certificate) enum LongRunRefutation {
    /// The rebuilt chain's check, accepted or rejected.
    Checked(ExactProbabilityCertificateCheck),
    /// The scheduler is missing, extra or names no choice of the rebuilt graph.
    SchedulerRefused,
}

/// Admit canonical long-run or weighted long-run bytes without checking them.
pub(in crate::probability_certificate) fn admit(
    bytes: &[u8],
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
) -> Result<(), ProbabilityDocumentError> {
    let mut cx = Context::new(limits, cancel);
    read(bytes, &mut cx).map(|_| ())
}

/// Check with a phase observer while retaining the original caller controls.
#[cfg(feature = "test-support")]
pub fn check_long_run_probability_certificate_observing(
    system: &ModelSystem<'_>,
    claims: &[CheckedProbabilisticClaim],
    item: &ObligationIdentity,
    bytes: &[u8],
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
    observe: &mut dyn FnMut(LongRunProbabilityPhase),
) -> Result<ExactProbabilityCertificateCheck, ProbabilityDocumentError> {
    let mut cx = Context::new(limits, cancel);
    cx.observe = Some(observe);
    check_with_context(system, claims, item, bytes, &mut cx)
}

/// Check using an explicit raw/full identity byte ceiling for resource controls.
#[cfg(feature = "test-support")]
pub fn check_long_run_probability_certificate_with_identity(
    system: &ModelSystem<'_>,
    claims: &[CheckedProbabilisticClaim],
    item: &ObligationIdentity,
    bytes: &[u8],
    limits: ProbabilityDocumentLimits,
    identity: fsl_foundation::IdentityLimits,
    cancel: &Cancel,
) -> Result<ExactProbabilityCertificateCheck, ProbabilityDocumentError> {
    let mut cx = Context::new(limits, cancel);
    cx.identity = identity;
    check_with_context(system, claims, item, bytes, &mut cx)
}

fn check_with_context(
    system: &ModelSystem<'_>,
    claims: &[CheckedProbabilisticClaim],
    item: &ObligationIdentity,
    bytes: &[u8],
    cx: &mut Context<'_>,
) -> Result<ExactProbabilityCertificateCheck, ProbabilityDocumentError> {
    match check_scheduled(system, claims, item, bytes, &[], cx)? {
        LongRunRefutation::Checked(check) => Ok(check),
        LongRunRefutation::SchedulerRefused => Err(ProbabilityDocumentError::Unsupported),
    }
}

/// The chain `rows` induce from the initial state: only the reached states,
/// each keeping its one scheduled choice. `None` when the rows are not exactly
/// one enabled action for every reached choice state.
fn induced_graph(
    system: &ModelSystem<'_>,
    graph: CheckerGraph,
    reference: Option<&ObjectReference>,
    rows: &[(ProbabilityProductKey, crate::ProductAction)],
    with_enabled: bool,
    cx: &mut Context<'_>,
) -> Result<Option<Induced>, ProbabilityDocumentError> {
    let mut enabled = BTreeMap::new();
    let Some(chosen) = super::refutation::scheduled_choices(
        system,
        &graph,
        reference,
        rows,
        cx.limits,
        cx.cancel,
        &mut cx.meter,
        &mut cx.storage,
        with_enabled.then_some(&mut enabled),
    )?
    else {
        return Ok(None);
    };
    let mut states = BTreeMap::new();
    let mut pending = VecDeque::from([graph.initial.clone()]);
    while let Some(key) = pending.pop_front() {
        cx.work(1)?;
        if states.contains_key(&key) {
            continue;
        }
        let mut state = graph
            .states
            .get(&key)
            .ok_or(ProbabilityDocumentError::Malformed)?
            .clone();
        if !state.choices.is_empty() {
            let Some(&index) = chosen.get(&key) else {
                return Ok(None);
            };
            let choice = state
                .choices
                .get(index)
                .ok_or(ProbabilityDocumentError::Malformed)?
                .clone();
            for edge in &choice {
                cx.work(1)?;
                if edge.probability > rational(0) {
                    pending.push_back(edge.target.clone());
                }
            }
            state.choices = vec![choice];
        }
        cx.reserve(
            key.as_bytes()
                .len()
                .saturating_add(std::mem::size_of::<(ProbabilityProductKey, CheckerState)>()),
        )?;
        states.insert(key, state);
    }
    Ok(Some(Induced {
        graph: CheckerGraph {
            initial: graph.initial,
            states,
            scheduler_actions: Default::default(),
        },
        chosen,
        enabled,
    }))
}

/// The chain a scheduler induces, with what it chose and what it left enabled.
struct Induced {
    graph: CheckerGraph,
    /// The action index each reached choice state takes.
    chosen: BTreeMap<ProbabilityProductKey, usize>,
    /// Every enabled identity at each reached choice state, in action order;
    /// filled only when a fairness set is checked.
    enabled: BTreeMap<ProbabilityProductKey, Vec<fsl_eval::simulation::model::ScheduledIdentity>>,
}

/// Whether some bottom component of the induced chain is unfair for the set
/// (FR-411): a weak constraint whose operation or identity is enabled at every
/// state of the component, or a strong one enabled at some state, yet taken
/// at none, is violated. The checker recomputes the components itself.
fn has_unfair_bottom(
    system: &ModelSystem<'_>,
    induced: &Induced,
    constraints: &[filament_semantic_value::fairness::FairnessConstraint],
    cx: &mut Context<'_>,
) -> Result<bool, ProbabilityDocumentError> {
    use filament_semantic_value::fairness::{FairnessGranularity, FairnessKind};
    let graph = &induced.graph;
    let keys = graph.states.keys().collect::<Vec<_>>();
    let positions = keys
        .iter()
        .enumerate()
        .map(|(index, key)| (*key, index))
        .collect::<BTreeMap<_, _>>();
    let mut edges = vec![Vec::new(); keys.len()];
    let mut reverse = vec![Vec::new(); keys.len()];
    for (index, key) in keys.iter().enumerate() {
        cx.work(1)?;
        let state = &graph.states[*key];
        let Some(choice) = state.choices.first() else {
            edges[index].push(index);
            reverse[index].push(index);
            continue;
        };
        for edge in choice {
            cx.work(1)?;
            if edge.probability > rational(0) {
                let next = *positions
                    .get(&edge.target)
                    .ok_or(ProbabilityDocumentError::Malformed)?;
                edges[index].push(next);
                reverse[next].push(index);
            }
        }
    }
    let (_, components, bottom) =
        super::super::bottom_components(&edges, &reverse, &mut |_| cx.work(1))?;
    for (index, component) in components.iter().enumerate() {
        if !bottom[index] {
            continue;
        }
        for constraint in constraints {
            cx.work(1)?;
            let Some(text) = system.operation_text(
                constraint.operation().declaring_type(),
                constraint.operation().name(),
            ) else {
                // An operation the model does not declare is never enabled.
                continue;
            };
            let identities = |state: usize| -> &[fsl_eval::simulation::model::ScheduledIdentity] {
                induced
                    .enabled
                    .get(keys[state])
                    .map_or(&[][..], Vec::as_slice)
            };
            let taken_identity = |state: usize| {
                let index = induced.chosen.get(keys[state])?;
                identities(state).get(*index)
            };
            // One obligation per operation, or per scheduled identity.
            let mut obligations: Vec<Option<&fsl_eval::simulation::model::ScheduledIdentity>> =
                Vec::new();
            if constraint.granularity() == FairnessGranularity::Whole {
                obligations.push(None);
            } else {
                for &state in component {
                    for identity in identities(state) {
                        cx.work(1)?;
                        if identity.operation == text
                            && !obligations.iter().any(|seen| *seen == Some(identity))
                        {
                            obligations.push(Some(identity));
                        }
                    }
                }
            }
            for obligation in obligations {
                let matches = |identity: &fsl_eval::simulation::model::ScheduledIdentity| {
                    identity.operation == text && obligation.is_none_or(|wanted| identity == wanted)
                };
                let mut enabled_all = true;
                let mut enabled_any = false;
                let mut taken = false;
                for &state in component {
                    cx.work(1)?;
                    let here = identities(state).iter().any(matches);
                    enabled_all &= here;
                    enabled_any |= here;
                    taken |= taken_identity(state).is_some_and(matches);
                }
                let required = match constraint.kind() {
                    FairnessKind::Weak => enabled_all,
                    FairnessKind::Strong => enabled_any,
                };
                if required && !taken {
                    return Ok(true);
                }
            }
        }
    }
    Ok(false)
}

fn check_scheduled(
    system: &ModelSystem<'_>,
    claims: &[CheckedProbabilisticClaim],
    item: &ObligationIdentity,
    bytes: &[u8],
    scheduler: &[(ProbabilityProductKey, crate::ProductAction)],
    cx: &mut Context<'_>,
) -> Result<LongRunRefutation, ProbabilityDocumentError> {
    let decoded = match read(bytes, cx) {
        Ok(document) => document,
        Err(ProbabilityDocumentError::MissingMember("components")) => {
            return Ok(LongRunRefutation::Checked(malformed("components")))
        }
        Err(ProbabilityDocumentError::MalformedMember(member)) => {
            return Ok(LongRunRefutation::Checked(malformed(member)))
        }
        Err(error) => return Err(error),
    };
    let document = decoded.document;
    if &document.identity != item {
        return Err(ProbabilityDocumentError::IdentityMismatch);
    }
    let subject = subject(system, claims, item, cx)?;
    if decoded.weight != subject.weight || document.optimization != subject.optimization {
        return Err(ProbabilityDocumentError::ObjectiveMismatch);
    }
    if !super::fairness::same(
        &subject.fairness,
        &document.fairness,
        cx.limits,
        cx.cancel,
        &mut cx.meter,
        &mut cx.storage,
    )? {
        return Err(ProbabilityDocumentError::ObjectiveMismatch);
    }
    let workload = subject.optimization == LongRunOptimization::Workload;
    if workload && document.components.iter().any(|c| c.policy.is_some()) {
        // Under a workload no choice exists to name.
        return Ok(LongRunRefutation::Checked(malformed("policy")));
    }
    if workload && !scheduler.is_empty() {
        // A workload has no scheduler to rebuild a chain from.
        return Ok(LongRunRefutation::SchedulerRefused);
    }
    if !workload && !cx.refutation && !subject.constraints.is_empty() {
        // The holds side over fair schedulers needs FR-1041's progress ranking
        // over fair end components, which no long-run member carries.
        return Err(ProbabilityDocumentError::Unsupported);
    }
    let graph = subject.graph;
    if !document.predicate_states.iter().eq(graph
        .states
        .iter()
        .filter(|(_, state)| state.terminal == Some(true))
        .map(|(key, _)| key))
    {
        return Err(ProbabilityDocumentError::ObjectiveMismatch);
    }
    if !workload && !cx.refutation {
        return check_mdp(
            system,
            &graph,
            &document,
            &subject.bound,
            subject.reference.as_ref(),
            cx,
        )
        .map(|mut check| {
            if let ExactProbabilityCertificateCheck::Accepted(accepted) = &mut check {
                accepted.document = Some(bytes.into());
            }
            LongRunRefutation::Checked(check)
        });
    }
    if !workload && !policy_matches_scheduler(&document, scheduler) {
        return Ok(LongRunRefutation::Checked(malformed("policy")));
    }
    cx.scheduled = !workload;
    let graph = if workload {
        graph
    } else {
        let fair = !subject.constraints.is_empty();
        let Some(induced) = induced_graph(
            system,
            graph,
            subject.reference.as_ref(),
            scheduler,
            fair,
            cx,
        )?
        else {
            return Ok(LongRunRefutation::SchedulerRefused);
        };
        // FR-412-AC-27: under a fairness set a deterministic witness counts
        // only if every bottom component of its induced chain is fair.
        if fair && has_unfair_bottom(system, &induced, &subject.constraints, cx)? {
            return Ok(LongRunRefutation::Checked(rejected(
                ProbabilityCertificateRule::InequalityFails,
                induced.graph.initial.clone(),
            )));
        }
        induced.graph
    };
    check_graph(
        &graph,
        &document,
        &subject.bound,
        subject.weight.as_ref().map(|_| &subject.weights),
        cx,
    )
    .map(|mut check| {
        if let ExactProbabilityCertificateCheck::Accepted(accepted) = &mut check {
            accepted.document = Some(bytes.into());
        }
        LongRunRefutation::Checked(check)
    })
}

fn check_graph(
    graph: &CheckerGraph,
    document: &LongRunProbabilityDocument,
    bound: &Bound,
    weights: Option<&BTreeMap<ProbabilityProductKey, Rational>>,
    cx: &mut Context<'_>,
) -> Result<ExactProbabilityCertificateCheck, ProbabilityDocumentError> {
    cx.reserve(
        graph.states.len().saturating_mul(256).saturating_add(
            graph
                .states
                .values()
                .map(|s| s.choices.iter().map(Vec::len).sum::<usize>().max(1))
                .sum::<usize>()
                .saturating_mul(32),
        ),
    )?;
    let keys = graph.states.keys().collect::<Vec<_>>();
    let positions = keys
        .iter()
        .enumerate()
        .map(|(index, key)| (*key, index))
        .collect::<BTreeMap<_, _>>();
    let mut edges = vec![Vec::new(); keys.len()];
    let mut reverse = vec![Vec::new(); keys.len()];
    for (i, key) in keys.iter().enumerate() {
        cx.work(1)?;
        let state = &graph.states[*key];
        if state.choices.is_empty() {
            edges[i].push(i);
            reverse[i].push(i);
            continue;
        }
        let [choice] = state.choices.as_slice() else {
            return Err(ProbabilityDocumentError::Unsupported);
        };
        let mut mass = rational(0);
        for edge in choice {
            cx.work(1)?;
            check_scalar(&edge.probability, cx.limits)?;
            if edge.probability < rational(0) {
                return Err(ProbabilityDocumentError::Malformed);
            }
            mass = cx.arithmetic(filament_exact::RationalArithmetic::Add(
                &mass,
                &edge.probability,
            ))?;
            if edge.probability > rational(0) {
                let next = *positions
                    .get(&edge.target)
                    .ok_or(ProbabilityDocumentError::Malformed)?;
                edges[i].push(next);
                reverse[next].push(i);
            }
        }
        if mass != rational(1) {
            return Err(ProbabilityDocumentError::Malformed);
        }
    }
    let (_, components, bottom) = super::super::bottom_components(&edges, &reverse, &mut |pass| {
        #[cfg(feature = "test-support")]
        if pass == super::super::ComponentPass::Order {
            cx.phase(LongRunProbabilityPhase::Components)?;
        }
        #[cfg(not(feature = "test-support"))]
        let _ = pass;
        cx.work(1)
    })?;
    let mut expected = vec![None; keys.len()];
    let mut unweighted_zero_component = vec![false; keys.len()];
    for (index, component) in components.iter().enumerate() {
        if bottom[index] {
            cx.work(component.len() as u64)?;
            let first = *component
                .iter()
                .min()
                .ok_or(ProbabilityDocumentError::Malformed)?;
            if let Some(weights) = weights {
                let mut positive = false;
                for &state in component {
                    cx.work(1)?;
                    if weights
                        .get(keys[state])
                        .ok_or(ProbabilityDocumentError::Malformed)?
                        > &rational(0)
                    {
                        positive = true;
                    }
                }
                if !positive {
                    // FR-199 defines a weighted component with no positive
                    // expected weight by its unweighted state occupancy.
                    // Derive that case from the independently evaluated
                    // rewards; never accept a producer-supplied fallback flag.
                    for &state in component {
                        cx.work(1)?;
                        unweighted_zero_component[state] = true;
                    }
                }
            }
            for &state in component {
                expected[state] = Some(first);
            }
        }
    }
    let mut offered = vec![None; keys.len()];
    let mut source = vec![None; keys.len()];
    for (index, component) in document.components.iter().enumerate() {
        let Some(first) = component.bias.first() else {
            return Ok(malformed("components"));
        };
        let Some(&first_index) = positions.get(&first.state) else {
            return Ok(malformed("components"));
        };
        for row in &component.bias {
            cx.work(1)?;
            let Some(&state) = positions.get(&row.state) else {
                return Ok(malformed("components"));
            };
            if offered[state].replace(first_index).is_some() {
                return Ok(malformed("components"));
            }
            source[state] = Some(index);
        }
    }
    for (state, key) in keys.iter().enumerate() {
        #[cfg(feature = "test-support")]
        cx.phase(LongRunProbabilityPhase::Membership)?;
        cx.work(1)?;
        if expected[state] != offered[state] {
            return Ok(rejected(
                ProbabilityCertificateRule::InequalityFails,
                (*key).clone(),
            ));
        }
    }
    let mut values = BTreeMap::new();
    for row in &document.values {
        cx.work(1)?;
        if !graph.states.contains_key(&row.state) || values.insert(&row.state, &row.value).is_some()
        {
            return Ok(malformed("values"));
        }
    }
    if values.len() != keys.len() {
        return Ok(malformed("values"));
    }
    for (state, key) in keys.iter().enumerate() {
        cx.work(1)?;
        let value = values.get(key).ok_or(ProbabilityDocumentError::Malformed)?;
        let current = &graph.states[*key];
        let expected_value = if let Some(index) = source[state] {
            let component = &document.components[index];
            let first = &component.bias[0];
            if *key == &first.state
                && (component.gain != component.value || first.value != rational(0))
            {
                return Ok(rejected(
                    ProbabilityCertificateRule::InequalityFails,
                    (*key).clone(),
                ));
            }
            let bias = component
                .bias
                .binary_search_by(|row| row.state.cmp(key))
                .ok()
                .and_then(|index| component.bias.get(index))
                .ok_or(ProbabilityDocumentError::Malformed)?;
            let mut sum = if current.terminal == Some(true) {
                rational(1)
            } else {
                rational(0)
            };
            sum = cx.arithmetic(filament_exact::RationalArithmetic::Subtract(
                &sum,
                &component.value,
            ))?;
            if let Some(weights) = weights {
                let effective_weight = if unweighted_zero_component[state] {
                    rational(1)
                } else {
                    weights
                        .get(*key)
                        .ok_or(ProbabilityDocumentError::Malformed)?
                        .clone()
                };
                sum = cx.arithmetic(filament_exact::RationalArithmetic::Multiply(
                    &sum,
                    &effective_weight,
                ))?;
            }
            if current.choices.is_empty() {
                sum = cx.arithmetic(filament_exact::RationalArithmetic::Add(&sum, &bias.value))?;
            } else {
                for edge in &current.choices[0] {
                    cx.work(1)?;
                    let next = component
                        .bias
                        .binary_search_by(|row| row.state.cmp(&edge.target))
                        .ok()
                        .and_then(|index| component.bias.get(index))
                        .ok_or(ProbabilityDocumentError::Malformed)?;
                    let term = cx.arithmetic(filament_exact::RationalArithmetic::Multiply(
                        &edge.probability,
                        &next.value,
                    ))?;
                    sum = cx.arithmetic(filament_exact::RationalArithmetic::Add(&sum, &term))?;
                }
            }
            if sum != bias.value {
                return Ok(rejected(
                    ProbabilityCertificateRule::InequalityFails,
                    (*key).clone(),
                ));
            }
            component.value.clone()
        } else {
            let mut sum = rational(0);
            for edge in &current.choices[0] {
                cx.work(1)?;
                let next = values
                    .get(&edge.target)
                    .ok_or(ProbabilityDocumentError::Malformed)?;
                let term = cx.arithmetic(filament_exact::RationalArithmetic::Multiply(
                    &edge.probability,
                    next,
                ))?;
                sum = cx.arithmetic(filament_exact::RationalArithmetic::Add(&sum, &term))?;
            }
            sum
        };
        if **value != expected_value {
            return Ok(rejected(
                ProbabilityCertificateRule::InequalityFails,
                (*key).clone(),
            ));
        }
    }
    let initial = values
        .get(&graph.initial)
        .ok_or(ProbabilityDocumentError::Malformed)?;
    let threshold = match bound {
        Bound::AtLeast(v) | Bound::AtMost(v) => v,
    };
    check_scalar(threshold, cx.limits)?;
    let on_side = if cx.refutation {
        match bound {
            Bound::AtLeast(v) => *initial < v,
            Bound::AtMost(v) => *initial > v,
        }
    } else {
        match bound {
            Bound::AtLeast(v) => *initial >= v,
            Bound::AtMost(v) => *initial <= v,
        }
    };
    if !on_side {
        // FR-412-AC-27: a scheduler witness whose induced value meets the
        // bound fails the comparison like any other condition, at the initial state.
        let rule = if cx.scheduled {
            ProbabilityCertificateRule::InequalityFails
        } else {
            ProbabilityCertificateRule::ThresholdMissed
        };
        return Ok(rejected(rule, graph.initial.clone()));
    }
    poll(cx.cancel)?;
    Ok(ExactProbabilityCertificateCheck::Accepted(
        ExactProbabilityAcceptance {
            initial_values: None,
            bound: None,
            certificate: ExactProbabilityCertificate {
                values: document.values.clone(),
            },
            document: None,
            value: (*initial).clone(),
            side: match bound {
                Bound::AtLeast(_) => ProbabilityBoundSide::AtLeast,
                Bound::AtMost(_) => ProbabilityBoundSide::AtMost,
            },
        },
    ))
}

/// The maximal end components of an MDP given as, per state, each action's
/// successor states, by repeated strongly-connected-component refinement:
/// drop every action that leaves its state's component, drop states left with
/// no action, and repeat until nothing changes. Components are ordered by
/// their least state.
fn maximal_end_components(
    actions: &[Vec<Vec<usize>>],
    cx: &mut Context<'_>,
) -> Result<Vec<Vec<usize>>, ProbabilityDocumentError> {
    let count = actions.len();
    let mut alive_state = vec![true; count];
    let mut alive_action = actions
        .iter()
        .map(|state| vec![true; state.len()])
        .collect::<Vec<_>>();
    loop {
        let mut changed = false;
        // An action with a dead successor is gone; a state with no action dies.
        loop {
            let mut again = false;
            for state in 0..count {
                cx.work(1)?;
                if !alive_state[state] {
                    continue;
                }
                for (index, successors) in actions[state].iter().enumerate() {
                    if alive_action[state][index]
                        && successors.iter().any(|next| !alive_state[*next])
                    {
                        alive_action[state][index] = false;
                        again = true;
                    }
                }
                if !alive_action[state].iter().any(|alive| *alive) {
                    alive_state[state] = false;
                    again = true;
                }
            }
            changed |= again;
            if !again {
                break;
            }
        }
        let mut edges = vec![Vec::new(); count];
        let mut reverse = vec![Vec::new(); count];
        for state in 0..count {
            if !alive_state[state] {
                continue;
            }
            for (index, successors) in actions[state].iter().enumerate() {
                if !alive_action[state][index] {
                    continue;
                }
                for &next in successors {
                    cx.work(1)?;
                    edges[state].push(next);
                    reverse[next].push(state);
                }
            }
        }
        let (component_of, components, _) =
            super::super::bottom_components(&edges, &reverse, &mut |_| cx.work(1))?;
        for state in 0..count {
            if !alive_state[state] {
                continue;
            }
            for (index, successors) in actions[state].iter().enumerate() {
                cx.work(1)?;
                if alive_action[state][index]
                    && successors
                        .iter()
                        .any(|next| component_of[*next] != component_of[state])
                {
                    alive_action[state][index] = false;
                    changed = true;
                }
            }
            if !alive_action[state].iter().any(|alive| *alive) {
                alive_state[state] = false;
                changed = true;
            }
        }
        if !changed {
            let mut result = components
                .into_iter()
                .filter(|component| component.iter().all(|state| alive_state[*state]))
                .map(|mut component| {
                    component.sort_unstable();
                    component
                })
                .collect::<Vec<_>>();
            result.sort_by_key(|component| component[0]);
            return Ok(result);
        }
    }
}

/// FR-412-AC-26 over every scheduler with no fairness set: one entry per
/// maximal end component of the rebuilt graph, each with its gain-bias pair
/// and policy; the equalities under the policy and the inequalities over every
/// action; and the value of every other state as the optimal Bellman value
/// over the components. The checker rebuilds the components, every action and
/// its schedule itself, and reads the supplied policy rows only as hints it
/// verifies.
fn check_mdp(
    system: &ModelSystem<'_>,
    graph: &CheckerGraph,
    document: &LongRunProbabilityDocument,
    bound: &Bound,
    reference: Option<&ObjectReference>,
    cx: &mut Context<'_>,
) -> Result<ExactProbabilityCertificateCheck, ProbabilityDocumentError> {
    let minimum = document.optimization == LongRunOptimization::Minimum;
    let keys = graph.states.keys().collect::<Vec<_>>();
    let positions = keys
        .iter()
        .enumerate()
        .map(|(index, key)| (*key, index))
        .collect::<BTreeMap<_, _>>();
    let one = rational(1);
    // Per state, per action: the successors with positive probability. A state
    // with no action stutters forever: one action to itself.
    let mut actions: Vec<Vec<Vec<(usize, Rational)>>> = Vec::with_capacity(keys.len());
    for (index, key) in keys.iter().enumerate() {
        cx.work(1)?;
        let state = &graph.states[*key];
        if state.choices.is_empty() {
            actions.push(vec![vec![(index, one.clone())]]);
            continue;
        }
        let mut per_action = Vec::with_capacity(state.choices.len());
        for choice in &state.choices {
            let mut successors = Vec::new();
            for edge in choice {
                cx.work(1)?;
                check_scalar(&edge.probability, cx.limits)?;
                if edge.probability > rational(0) {
                    let next = *positions
                        .get(&edge.target)
                        .ok_or(ProbabilityDocumentError::Malformed)?;
                    successors.push((next, edge.probability.clone()));
                }
            }
            per_action.push(successors);
        }
        actions.push(per_action);
    }
    let shape = actions
        .iter()
        .map(|state| {
            state
                .iter()
                .map(|action| action.iter().map(|(next, _)| *next).collect())
                .collect()
        })
        .collect::<Vec<Vec<Vec<usize>>>>();
    let mecs = maximal_end_components(&shape, cx)?;
    let mut mec_of = vec![None; keys.len()];
    for (index, mec) in mecs.iter().enumerate() {
        for &state in mec {
            mec_of[state] = Some(index);
        }
    }
    // One entry per component; a missing or an extra one is malformed.
    let mut entry_of = vec![None; mecs.len()];
    for (index, component) in document.components.iter().enumerate() {
        let membership = (|| {
            let first = positions.get(&component.bias.first()?.state)?;
            let mec = mec_of[*first]?;
            if entry_of[mec].is_some() || component.bias.len() != mecs[mec].len() {
                return None;
            }
            for row in &component.bias {
                if mec_of[*positions.get(&row.state)?] != Some(mec) {
                    return None;
                }
            }
            Some(mec)
        })();
        let Some(mec) = membership else {
            return Ok(malformed("components"));
        };
        entry_of[mec] = Some(index);
    }
    if entry_of.iter().any(Option::is_none) {
        return Ok(malformed("components"));
    }
    let schedules = super::refutation::enabled_schedules(
        system,
        graph,
        reference,
        cx.limits,
        cx.cancel,
        &mut cx.meter,
        &mut cx.storage,
    )?;
    let mut values = vec![None; keys.len()];
    for row in &document.values {
        cx.work(1)?;
        let Some(&state) = positions.get(&row.state) else {
            return Ok(malformed("values"));
        };
        if values[state].replace(&row.value).is_some() {
            return Ok(malformed("values"));
        }
    }
    if values.iter().any(Option::is_none) {
        return Ok(malformed("values"));
    }
    let values = values.into_iter().flatten().collect::<Vec<_>>();
    // Each component, in ascending order of its least state.
    for (mec, entry) in entry_of.iter().enumerate() {
        let component = &document.components[entry.ok_or(ProbabilityDocumentError::Malformed)?];
        let first = &component.bias[0];
        // The policy: one row per state of the component, each an action of
        // the state that stays inside it.
        let Some(rows) = &component.policy else {
            return Ok(malformed("policy"));
        };
        if rows.len() != mecs[mec].len() {
            return Ok(malformed("policy"));
        }
        let mut policy = BTreeMap::new();
        for row in rows {
            cx.work(1)?;
            let Some(&state) = positions.get(&row.state) else {
                return Ok(malformed("policy"));
            };
            if mec_of[state] != Some(mec) {
                return Ok(malformed("policy"));
            }
            let index = match (&row.choice, schedules.get(keys[state])) {
                (crate::ProductAction::TerminalStutter, None) => 0,
                (crate::ProductAction::Identity(schedule), Some(row)) => {
                    match row.iter().position(|offered| offered == schedule) {
                        Some(index) => index,
                        None => return Ok(malformed("policy")),
                    }
                }
                _ => return Ok(malformed("policy")),
            };
            let stays = actions[state]
                .get(index)
                .is_some_and(|action| action.iter().all(|(next, _)| mec_of[*next] == Some(mec)));
            if !stays || policy.insert(state, index).is_some() {
                return Ok(malformed("policy"));
            }
        }
        if component.gain != component.value || first.value != rational(0) {
            return Ok(rejected(
                ProbabilityCertificateRule::InequalityFails,
                first.state.clone(),
            ));
        }
        let bias = |state: usize| -> Option<&Rational> {
            component
                .bias
                .binary_search_by(|row| row.state.cmp(keys[state]))
                .ok()
                .map(|index| &component.bias[index].value)
        };
        for &state in &mecs[mec] {
            cx.work(1)?;
            let own = bias(state).ok_or(ProbabilityDocumentError::Malformed)?;
            let indicator = if graph.states[keys[state]].terminal == Some(true) {
                rational(1)
            } else {
                rational(0)
            };
            let reward = cx.arithmetic(filament_exact::RationalArithmetic::Subtract(
                &indicator,
                &component.value,
            ))?;
            for (index, successors) in actions[state].iter().enumerate() {
                if !successors
                    .iter()
                    .all(|(next, _)| mec_of[*next] == Some(mec))
                {
                    continue;
                }
                let mut sum = reward.clone();
                for (next, probability) in successors {
                    cx.work(1)?;
                    let next = bias(*next).ok_or(ProbabilityDocumentError::Malformed)?;
                    let term = cx.arithmetic(filament_exact::RationalArithmetic::Multiply(
                        probability,
                        next,
                    ))?;
                    sum = cx.arithmetic(filament_exact::RationalArithmetic::Add(&sum, &term))?;
                }
                let optimal = if minimum { *own <= sum } else { *own >= sum };
                let attained = policy.get(&state) != Some(&index) || *own == sum;
                if !optimal || !attained {
                    return Ok(rejected(
                        ProbabilityCertificateRule::InequalityFails,
                        keys[state].clone(),
                    ));
                }
            }
        }
    }
    // A component's states share one value, which may be better than the
    // component's own `rho` because a scheduler can leave it for another
    // component: it is on the right side of `rho` and of every action that
    // leaves the component, and equals one of them (staying or an exit).
    for (mec, entry) in entry_of.iter().enumerate() {
        let component = &document.components[entry.ok_or(ProbabilityDocumentError::Malformed)?];
        let first = mecs[mec][0];
        let shared = values[first];
        let mut attained = *shared == component.value;
        let on_side = |sum: &Rational| {
            if minimum {
                shared <= sum
            } else {
                shared >= sum
            }
        };
        if !on_side(&component.value) {
            return Ok(rejected(
                ProbabilityCertificateRule::InequalityFails,
                keys[first].clone(),
            ));
        }
        for &state in &mecs[mec] {
            cx.work(1)?;
            if values[state] != shared {
                return Ok(rejected(
                    ProbabilityCertificateRule::InequalityFails,
                    keys[state].clone(),
                ));
            }
            for successors in &actions[state] {
                if successors
                    .iter()
                    .all(|(next, _)| mec_of[*next] == Some(mec))
                {
                    continue;
                }
                let mut sum = rational(0);
                for (next, probability) in successors {
                    cx.work(1)?;
                    let term = cx.arithmetic(filament_exact::RationalArithmetic::Multiply(
                        probability,
                        values[*next],
                    ))?;
                    sum = cx.arithmetic(filament_exact::RationalArithmetic::Add(&sum, &term))?;
                }
                if !on_side(&sum) {
                    return Ok(rejected(
                        ProbabilityCertificateRule::InequalityFails,
                        keys[state].clone(),
                    ));
                }
                attained |= *shared == sum;
            }
        }
        if !attained {
            return Ok(rejected(
                ProbabilityCertificateRule::InequalityFails,
                keys[first].clone(),
            ));
        }
    }
    // The value of a state outside every component is its optimal value over
    // the actions.
    for (state, key) in keys.iter().enumerate() {
        cx.work(1)?;
        let expected = match mec_of[state] {
            Some(_) => continue,
            None => {
                let mut best: Option<Rational> = None;
                for successors in &actions[state] {
                    let mut sum = rational(0);
                    for (next, probability) in successors {
                        cx.work(1)?;
                        let term = cx.arithmetic(filament_exact::RationalArithmetic::Multiply(
                            probability,
                            values[*next],
                        ))?;
                        sum =
                            cx.arithmetic(filament_exact::RationalArithmetic::Add(&sum, &term))?;
                    }
                    best = Some(match best {
                        Some(best) if (minimum && best <= sum) || (!minimum && best >= sum) => best,
                        _ => sum,
                    });
                }
                best.ok_or(ProbabilityDocumentError::Malformed)?
            }
        };
        if *values[state] != expected {
            return Ok(rejected(
                ProbabilityCertificateRule::InequalityFails,
                (*key).clone(),
            ));
        }
    }
    let initial = values[*positions
        .get(&graph.initial)
        .ok_or(ProbabilityDocumentError::Malformed)?];
    let threshold = match bound {
        Bound::AtLeast(v) | Bound::AtMost(v) => v,
    };
    check_scalar(threshold, cx.limits)?;
    let holds = match bound {
        Bound::AtLeast(v) => initial >= v,
        Bound::AtMost(v) => initial <= v,
    };
    if !holds {
        return Ok(rejected(
            ProbabilityCertificateRule::ThresholdMissed,
            graph.initial.clone(),
        ));
    }
    poll(cx.cancel)?;
    Ok(ExactProbabilityCertificateCheck::Accepted(
        ExactProbabilityAcceptance {
            initial_values: None,
            bound: None,
            certificate: ExactProbabilityCertificate {
                values: document.values.clone(),
            },
            document: None,
            value: initial.clone(),
            side: match bound {
                Bound::AtLeast(_) => ProbabilityBoundSide::AtLeast,
                Bound::AtMost(_) => ProbabilityBoundSide::AtMost,
            },
        },
    ))
}

/// FR-412-AC-27: a refutation document's component policy rows are the
/// witness scheduler's rows (and a terminal stutter at a state with none), one
/// per state of the component.
fn policy_matches_scheduler(
    document: &LongRunProbabilityDocument,
    scheduler: &[(ProbabilityProductKey, crate::ProductAction)],
) -> bool {
    let rows = scheduler
        .iter()
        .map(|(key, action)| (key, action))
        .collect::<BTreeMap<_, _>>();
    document.components.iter().all(|component| {
        component.policy.as_ref().is_some_and(|policy| {
            policy.len() == component.bias.len()
                && policy.iter().zip(&component.bias).all(|(row, bias)| {
                    row.state == bias.state
                        && match rows.get(&row.state) {
                            Some(action) => **action == row.choice,
                            None => row.choice == crate::ProductAction::TerminalStutter,
                        }
                })
        })
    })
}
