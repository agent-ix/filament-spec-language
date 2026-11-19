// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-944: derive and admit the checked temporal trace through its owner.

use std::collections::{BTreeMap, BTreeSet};

use filament_eval::CallFailure;
use filament_observation::authority::{
    observation, partial, population, AuthoritySelection, Context, History, Limits,
    SubjectSelection,
};
use filament_observation::{
    admit, AdmissionOutcome, AdmissionRequest, AdmittedRecord, Anchor, ClockRange, Digest,
    Identity, Member, ObservationBinding, PackageSelection, QualifiedSubject, ResourceLimits,
    ScopeKind, ScopeSelection, SubjectIdentity, SubjectKind, ValueState, Visibility,
};
use filament_semantic_value::catalog::InternalFault;
use fsl_package::{CheckedNodeId, ClockConfiguration, ClockExactNumber};
use serde_json::{json, Map, Value};
use sha2::{Digest as _, Sha256};

use super::binding::AuthenticatedBinding;
use super::call::{fault, refusal};
use super::request::{EvidencePosition, EvidenceRef, EvidenceRequest, EvidenceSubject};
use super::trigger::SemanticTriggerIdentity;
use crate::temporal::finite_window::{ClockCover, TimestampedTrace, Valuation};
use crate::temporal::position::TemporalPosition;
use crate::temporal::window::AggregateSample;

const VALUE_TYPE: &str = "fsl-eval/trace-position/1";
const PACKAGE_REVISION: &str = fsl_package::CHECKED_PACKAGE_V2;

struct TraceDefinition<'a> {
    package_identity: &'a str,
    canonical_package_bytes: &'a [u8],
    time_unit_scale: Option<&'a filament_exact::Rational>,
    declaration: &'a CheckedNodeId,
    signal: &'a CheckedNodeId,
    occurrence: u64,
}

/// Owner-qualified records and the many-to-one mapping back to FR-106 labels.
pub(super) struct OwnerObservation {
    pub(super) qualified: Box<filament_observation::QualifiedObservation>,
    pub(super) side_map: BTreeMap<Identity, Vec<String>>,
    pub(super) partial_facts: Vec<PartialFact>,
}

/// One Boolean partial fact derived and strictly read through the owner.
pub(super) struct PartialFact {
    pub(super) record_identity: Identity,
    pub(super) atom: u64,
    pub(super) instance: Option<SemanticTriggerIdentity>,
    pub(super) view: partial::View,
}

/// Admit every position in canonical trace order. Selection data is read only
/// from the checked subject and the trace, never from an owner field in input.
pub(super) fn admit_trace(
    subject: &EvidenceSubject<'_>,
    request: &EvidenceRequest,
    binding: &AuthenticatedBinding,
    occurrence: u64,
    trace: &TimestampedTrace,
) -> Result<OwnerObservation, CallFailure> {
    let definition = TraceDefinition {
        package_identity: &subject.package_identity,
        canonical_package_bytes: subject
            .canonical_package_bytes
            .ok_or_else(|| fault("observation-admission-has-canonical-package-bytes"))?,
        time_unit_scale: subject.time_unit_scale.as_ref(),
        declaration: subject.declaration,
        signal: subject.signal,
        occurrence,
    };
    admit_defined(
        &definition,
        &request.positions,
        request.cover,
        binding,
        trace,
    )
}

fn admit_defined(
    subject: &TraceDefinition<'_>,
    positions: &[EvidencePosition],
    cover: ClockCover,
    binding: &AuthenticatedBinding,
    trace: &TimestampedTrace,
) -> Result<OwnerObservation, CallFailure> {
    let owner_limits = Limits::owner_max();
    if positions.len() > owner_limits.max_population_entries {
        return Err(owner_limit(
            "node-count-exceeded",
            owner_limits.max_population_entries,
            positions.len(),
            "filament_observation::authority::Limits.max_population_entries",
            None,
        ));
    }
    let bytes = subject.canonical_package_bytes;
    let package_digest: [u8; 32] = Sha256::digest(bytes).into();
    let package_digest = Digest::new(package_digest);
    let clock = &binding.clock;
    let time_factor = nanos_per_tick(binding, subject.time_unit_scale)?;
    // A caller limit or an unrepresentable timestamp must be reported
    // before deriving any owner identity, even at a later position.
    for position in positions {
        let value = canonical_position_value(position)?;
        if value.len() > owner_limits.max_string_bytes {
            return Err(owner_limit(
                "input-bytes-exceeded",
                owner_limits.max_string_bytes,
                value.len(),
                "filament_observation::authority::Limits.max_string_bytes",
                Some(&position.observation.identity),
            ));
        }
        let nanos = scaled_nanos(position.position, &time_factor, position)?;
        anchor_of(clock, position.position, nanos, &time_factor, position)?;
        if let Some(arrival) = position.arrival {
            scaled_nanos(arrival, &time_factor, position)?;
        }
        let uncertainty = match cover {
            ClockCover::Exact => 0,
            ClockCover::Counter { uncertainty } | ClockCover::Bounded { uncertainty } => {
                uncertainty
            }
        };
        u64::try_from(scaled_nanos(uncertainty, &time_factor, position)?)
            .map_err(|_| unrepresentable(position, "clock_uncertainty_nanos"))?;
    }
    let producer =
        super::producer::admitted().map_err(|cause| owner_fault("producer-bundle", cause))?;
    let clock_identity = Identity::new(binding.clock_name.to_string());
    let clock_revision = Identity::new(binding.profile.identity.to_string());
    let unit = format!("{}/4", clock_unit(clock));
    let declaration = node_wire_text(subject.declaration)?;
    let signal = node_wire_text(subject.signal)?;
    let profile = binding.profile.identity.to_string();
    let binding_identity = canonical_text(&json!({
        "clause": declaration,
        "clock": binding.clock_name.as_ref(),
        "occurrence": subject.occurrence.to_string(),
        "package": subject.package_identity,
        "profile": profile,
    }))?;
    let signal_identity = if signal.is_empty() {
        declaration.clone()
    } else {
        signal
    };
    let trigger_identity = format!("{declaration}/activation");
    let package = PackageSelection {
        format: filament_observation::NATIVE_LINKED_PACKAGE_FORMAT.to_owned(),
        identity: Identity::new(subject.package_identity),
        revision: Identity::new(PACKAGE_REVISION),
        digest: package_digest,
    };
    let expected_subject = QualifiedSubject::new(
        producer,
        SubjectKind::new("package".as_bytes().to_vec())
            .map_err(|_| fault("observation-subject-kind-is-nonempty"))?,
        SubjectIdentity::new(subject.package_identity.as_bytes().to_vec())
            .map_err(|_| fault("observation-subject-identity-is-nonempty"))?,
    );

    let mut records = Vec::new();
    let mut side_map: BTreeMap<Identity, Vec<String>> = BTreeMap::new();
    let mut anchors = Vec::new();
    let mut position_records = Vec::new();
    for (canonical_index, observed) in trace.positions().iter().enumerate() {
        let temporal_position = TemporalPosition::new(
            u64::try_from(canonical_index).map_err(|_| fault("positions-in-u64"))?,
        );
        let source_index = trace
            .supplied_index(temporal_position)
            .ok_or_else(|| fault("canonical-position-has-supplied-index"))?;
        let source_index =
            usize::try_from(source_index).map_err(|_| fault("supplied-position-index-in-usize"))?;
        let position = positions
            .get(source_index)
            .ok_or_else(|| fault("supplied-index-names-a-request-position"))?;
        let event_time_nanos = scaled_nanos(observed.stamp, &time_factor, position)?;
        // The owner now admits an unrecorded ingestion instant. Absence is
        // never promoted to an asserted arrival or an ordering premise.
        let ingestion_time_nanos = position
            .arrival
            .map(|arrival| scaled_nanos(arrival, &time_factor, position))
            .transpose()?;
        let anchor = anchor_of(
            clock,
            observed.stamp,
            event_time_nanos,
            &time_factor,
            position,
        )?;
        let uncertainty = match cover {
            ClockCover::Exact => 0,
            ClockCover::Counter { uncertainty } | ClockCover::Bounded { uncertainty } => {
                u64::try_from(scaled_nanos(uncertainty, &time_factor, position)?)
                    .map_err(|_| unrepresentable(position, "clock_uncertainty_nanos"))?
            }
        };
        let value = canonical_position_value(position)?;
        let mut record = AdmittedRecord {
            identity: Identity::new("pending"),
            binding_identity: Identity::new(binding_identity.clone()),
            source_identity: clock_identity.clone(),
            schema_identity: Identity::new(VALUE_TYPE),
            subject: expected_subject.clone(),
            signal_identity: Identity::new(signal_identity.clone()),
            trigger_identity: Identity::new(trigger_identity.clone()),
            unit: Identity::new(unit.clone()),
            value: ValueState::Present {
                value_type: Identity::new(VALUE_TYPE),
                canonical_value: value,
            },
            visibility: Visibility::Internal,
            anchor,
            event_time_nanos,
            ingestion_time_nanos,
            causal_relationship_identity: None,
            clock_identity: clock_identity.clone(),
            clock_revision: clock_revision.clone(),
            clock_uncertainty_nanos: uncertainty,
        };
        let identity = observation::record_identity(&record, owner_limits)
            .map_err(|error| owner_fault(error.code().as_str(), error.detail()))?;
        record.identity = identity.clone();
        let record_identity = record.identity.clone();
        if let Some(existing) = records
            .iter()
            .find(|existing: &&AdmittedRecord| existing.identity == identity)
        {
            if existing != &record {
                return Err(fault(
                    "equal-observation-record-identities-have-equal-records",
                ));
            }
        } else {
            records.push(record);
        }
        side_map
            .entry(identity)
            .or_default()
            .push(position.observation.identity.clone());
        anchors.push(anchor);
        position_records.push((
            record_identity,
            observed.stamp,
            ingestion_time_nanos,
            position,
        ));
    }

    for identities in side_map.values_mut() {
        identities.sort();
        identities.dedup();
    }
    let mut required: Vec<Identity> = records
        .iter()
        .map(|record| record.identity.clone())
        .collect();
    required.sort();
    required.dedup();
    let membership = population::derive_membership(&required, owner_limits)
        .map_err(|error| owner_fault(error.code().as_str(), error.detail()))?;
    let range = clock_range(clock, &anchors, &time_factor)?;
    let member_by_identity: BTreeMap<Identity, Anchor> = records
        .iter()
        .map(|record| (record.identity.clone(), record.anchor))
        .collect();
    let members = required
        .iter()
        .map(|identity| {
            let anchor = member_by_identity
                .get(identity)
                .copied()
                .ok_or_else(|| fault("required-member-has-record-anchor"))?;
            Ok(Member {
                object_identity: identity.clone(),
                record_identity: identity.clone(),
                anchor,
            })
        })
        .collect::<Result<Vec<_>, CallFailure>>()?;
    let membership_identity = membership.identity().clone();
    let membership_digest = membership.digest();
    let scope = ScopeSelection {
        population_identity: Identity::new("pending"),
        membership_rule_identity: membership_identity.clone(),
        membership_digest,
        membership_document: membership.bytes().to_vec(),
        required_member_identities: required,
        observation_sources: vec![clock_identity.clone()],
        completeness_dependencies: vec![clock_identity.clone()],
        progress_dependencies: vec![clock_identity.clone()],
        clock_identity: clock_identity.clone(),
        clock_revision: clock_revision.clone(),
        membership_complete: true,
        closure_identity: Some(membership_identity.clone()),
        closure_digest: Some(membership_digest),
        kind: ScopeKind::Snapshot {
            snapshot_identity: membership_identity,
        },
        range,
        members,
    };
    let binding_selection = ObservationBinding {
        identity: Identity::new(binding_identity),
        source_identity: clock_identity,
        schema_identity: Identity::new(VALUE_TYPE),
        signal_identity: Identity::new(signal_identity),
        trigger_identity: Identity::new(trigger_identity),
        unit: Identity::new(unit),
        subject_kind: SubjectKind::new("package".as_bytes().to_vec())
            .map_err(|_| fault("observation-subject-kind-is-nonempty"))?,
        required: true,
    };
    let mut admission_request = AdmissionRequest {
        package,
        producer: producer.clone(),
        binding: binding_selection,
        expected_subject,
        relationships: Vec::new(),
        required_relationships: Vec::new(),
        scope,
        records,
        limits: ResourceLimits {
            max_records: side_map.len(),
            max_members: side_map.len(),
            max_relationships: 0,
            max_required_relationships: 0,
        },
    };
    admission_request.scope.population_identity =
        population::population_identity(&admission_request, owner_limits)
            .map_err(|error| owner_fault(error.code().as_str(), error.detail()))?;
    match admit(admission_request) {
        AdmissionOutcome::Available { observation } => {
            let partial_facts = derive_partial_facts(
                binding,
                &observation,
                &package_digest,
                cover,
                &position_records,
            )?;
            let owner = OwnerObservation {
                qualified: observation,
                side_map,
                partial_facts,
            };
            owner.validate()?;
            Ok(owner)
        }
        AdmissionOutcome::Incomplete { reasons } => {
            Err(owner_fault("incomplete", &format!("{reasons:?}")))
        }
        AdmissionOutcome::Refused { cause } => Err(owner_fault("refused", &format!("{cause:?}"))),
    }
}

impl OwnerObservation {
    /// Read only Boolean facts. Undefined values retain the evaluator's
    /// cause and never select or consume an owner possibility set.
    pub(super) fn valuation(
        &self,
        observation: &str,
        atom: u64,
        instance: Option<&SemanticTriggerIdentity>,
        supplied: &Valuation,
    ) -> Result<Valuation, InternalFault> {
        if matches!(supplied, Valuation::Undefined(_)) {
            return Ok(supplied.clone());
        }
        let fact = self
            .partial_facts
            .iter()
            .find(|fact| {
                fact.atom == atom
                    && fact.instance.as_ref() == instance
                    && self
                        .side_map
                        .get(&fact.record_identity)
                        .is_some_and(|labels| labels.iter().any(|label| label == observation))
            })
            .ok_or_else(|| {
                InternalFault::new("temporal.evidence", "supplied-boolean-has-owner-fact")
            })?;
        let view = fact.view.payload();
        match (view.can_be_false(), view.can_be_true()) {
            (false, true) => Ok(Valuation::True),
            (true, false) => Ok(Valuation::False),
            (true, true) => Ok(Valuation::Missing),
            (false, false) => Err(InternalFault::new(
                "temporal.evidence",
                "owner-possibility-set-is-nonempty",
            )),
        }
    }

    fn validate(&self) -> Result<(), CallFailure> {
        if self.qualified.records().len() != self.side_map.len() {
            return Err(fault("owner-side-map-covers-every-unique-record"));
        }
        for fact in &self.partial_facts {
            if !self.side_map.contains_key(&fact.record_identity)
                || !self
                    .qualified
                    .records()
                    .iter()
                    .any(|record| record.identity == fact.record_identity)
            {
                return Err(fault("partial-fact-record-has-source-side-mapping"));
            }
            if fact.view.payload().observation_identity() != fact.record_identity.as_str()
                || fact
                    .instance
                    .as_ref()
                    .is_some_and(|key| key.as_bytes().is_empty())
            {
                return Err(fault("partial-fact-owner-view-is-coherent"));
            }
        }
        Ok(())
    }
}

fn derive_partial_facts(
    binding: &AuthenticatedBinding,
    qualified: &filament_observation::QualifiedObservation,
    package_digest: &Digest,
    cover: ClockCover,
    positions: &[(Identity, u64, Option<i128>, &EvidencePosition)],
) -> Result<Vec<PartialFact>, CallFailure> {
    let ScopeKind::Snapshot { snapshot_identity } = &qualified.scope().kind else {
        return Err(fault("observation-scope-is-a-snapshot"));
    };
    let authority = AuthoritySelection {
        definition_identity: qualified.binding().identity.clone(),
        definition_revision: Identity::new(PACKAGE_REVISION),
        definition_digest: *package_digest,
    };
    let owner_subject = SubjectSelection {
        scope_identity: snapshot_identity.clone(),
        population_identity: qualified.scope().population_identity.clone(),
    };
    let context = Context::new(
        History::batch(qualified),
        &authority,
        &owner_subject,
        1,
        None,
    );
    let mut facts = Vec::new();
    let mut selected_records = BTreeSet::new();
    for (identity, stamp, ingestion_position, position) in positions {
        if !selected_records.insert(identity.clone()) {
            continue;
        }
        let interval = quarter_interval(binding, *stamp, position, cover)?;
        let mut values = Vec::new();
        values.extend(
            position
                .valuations
                .iter()
                .map(|(atom, value)| (*atom, None, value.clone())),
        );
        for (trigger, values_by_atom) in &position.instance_valuations {
            values.extend(
                values_by_atom
                    .iter()
                    .map(|(atom, value)| (*atom, Some(trigger.clone()), value.clone())),
            );
        }
        values.extend(position.aggregate_samples.iter().filter_map(
            |(atom, sample)| match sample {
                AggregateSample::Holds(value) => Some((
                    *atom,
                    None,
                    if *value {
                        Valuation::True
                    } else {
                        Valuation::False
                    },
                )),
                _ => None,
            },
        ));
        for (trigger, samples) in &position.instance_aggregate_samples {
            values.extend(samples.iter().filter_map(|(atom, sample)| match sample {
                AggregateSample::Holds(value) => Some((
                    *atom,
                    Some(trigger.clone()),
                    if *value {
                        Valuation::True
                    } else {
                        Valuation::False
                    },
                )),
                _ => None,
            }));
        }
        for (atom, instance, value) in values {
            let (can_be_false, can_be_true, reason) = match value {
                Valuation::True => (false, true, partial::ValueReason::Known),
                Valuation::False => (true, false, partial::ValueReason::Known),
                Valuation::Missing => (true, true, partial::ValueReason::Missing),
                Valuation::Undefined(_) => continue,
            };
            let possibilities = partial::PossibilitySet::new(can_be_false, can_be_true, reason)
                .map_err(|error| owner_fault(error.code().as_str(), error.detail()))?;
            let selection = partial::Selection::new(
                identity.clone(),
                possibilities,
                interval.clone(),
                *ingestion_position,
            );
            let document = partial::derive(context, &selection, Limits::owner_max())
                .map_err(|error| owner_fault(error.code().as_str(), error.detail()))?;
            let view = partial::read(document.bytes(), context, &selection, Limits::owner_max())
                .map_err(|error| owner_fault(error.code().as_str(), error.detail()))?;
            facts.push(PartialFact {
                record_identity: identity.clone(),
                atom,
                instance,
                view,
            });
        }
    }
    Ok(facts)
}

fn quarter_interval(
    binding: &AuthenticatedBinding,
    stamp: u64,
    position: &EvidencePosition,
    cover: ClockCover,
) -> Result<partial::EventTimeInterval, CallFailure> {
    let (earliest, earliest_open, latest, latest_open) = match position.interval {
        Some(interval) => (
            i128::from(interval.earliest),
            interval.earliest_open,
            i128::from(interval.latest),
            interval.latest_open,
        ),
        None => match cover {
            ClockCover::Exact => (i128::from(stamp), false, i128::from(stamp), false),
            ClockCover::Counter { uncertainty } => (
                i128::from(stamp) - i128::from(uncertainty),
                false,
                i128::from(stamp) + 1 + i128::from(uncertainty),
                true,
            ),
            ClockCover::Bounded { uncertainty } => (
                i128::from(stamp) - i128::from(uncertainty),
                false,
                i128::from(stamp) + i128::from(uncertainty),
                false,
            ),
        },
    };
    let earliest = earliest
        .checked_mul(4)
        .and_then(|value| value.checked_add(i128::from(earliest_open)))
        .ok_or_else(|| fault("quarter-tick-lower-endpoint-fits"))?;
    let latest = latest
        .checked_mul(4)
        .and_then(|value| value.checked_sub(i128::from(latest_open)))
        .ok_or_else(|| fault("quarter-tick-upper-endpoint-fits"))?;
    partial::EventTimeInterval::new(
        Identity::new(binding.clock_name.to_string()),
        Identity::new(binding.profile.identity.to_string()),
        Identity::new(format!("{}/4", clock_unit(&binding.clock))),
        earliest,
        latest,
        Limits::owner_max(),
    )
    .map_err(|error| owner_fault(error.code().as_str(), error.detail()))
}

fn owner_fault(code: &str, cause: &str) -> CallFailure {
    CallFailure::Fault(
        InternalFault::new("temporal.evidence", "observation-admission").with_named(vec![
            ("owner_code", code.to_owned()),
            ("cause", cause.to_owned()),
        ]),
    )
}

fn owner_limit(
    cause: &'static str,
    bound: usize,
    actual: usize,
    setting: &'static str,
    position: Option<&str>,
) -> CallFailure {
    use filament_eval::{CallAdmissionFailure, CallAdmissionLocus, CallAdmissionRecord};
    use filament_semantic_value::admission::AdmissionRecord;
    use filament_semantic_value::catalog::CatalogCode;
    let mut fields = BTreeMap::from([
        ("stage", "admit".to_owned()),
        ("bound", bound.to_string()),
        ("actual", actual.to_string()),
        ("setting", setting.to_owned()),
    ]);
    if let Some(position) = position {
        fields.insert("position", position.to_owned());
    }
    CallFailure::Admission(Box::new(CallAdmissionFailure::Refused(
        CallAdmissionRecord {
            record: AdmissionRecord {
                catalog: CatalogCode::new("stage_limit_exceeded", cause),
                fields,
                path: Vec::new(),
                kernel: None,
            },
            locus: CallAdmissionLocus::Member {
                name: "positions".to_owned(),
                index: None,
            },
        },
    )))
}

fn node_wire_text(node: &CheckedNodeId) -> Result<String, CallFailure> {
    let value =
        serde_json::to_value(node).map_err(|_| fault("checked-node-identity-serializes"))?;
    canonical_text(&value)
}

fn canonical_text(value: &Value) -> Result<String, CallFailure> {
    quire_canonical::to_vec(value, quire_canonical::Limits::new(u64::MAX))
        .map(String::from_utf8)
        .map_err(|_| fault("owner-value-has-canonical-json"))?
        .map_err(|_| fault("owner-value-is-utf8"))
}

fn canonical_position_value(position: &EvidencePosition) -> Result<String, CallFailure> {
    let interval = position.interval.map(|interval| {
        json!({
            "earliest": interval.earliest.to_string(),
            "earliest_open": interval.earliest_open,
            "latest": interval.latest.to_string(),
            "latest_open": interval.latest_open,
        })
    });
    let mut object = Map::new();
    object.insert("interval".to_owned(), interval.unwrap_or(Value::Null));
    object.insert("observation".to_owned(), observation_value(position)?);
    object.insert(
        "order_key".to_owned(),
        position.order_key.as_ref().map_or(
            Value::Null,
            |key| json!({"authority": key.authority, "key": key.key.to_string()}),
        ),
    );
    object.insert("valuations".to_owned(), valuation_map(&position.valuations));
    object.insert(
        "aggregate_samples".to_owned(),
        aggregate_map(&position.aggregate_samples),
    );
    object.insert(
        "instance_valuations".to_owned(),
        instance_valuation_map(&position.instance_valuations),
    );
    object.insert(
        "instance_aggregate_samples".to_owned(),
        instance_aggregate_map(&position.instance_aggregate_samples),
    );
    canonical_text(&Value::Object(object))
}

fn evidence_ref_value(reference: &EvidenceRef) -> Value {
    json!({
        "authority_digest": hex(&reference.authority_digest),
        "contract": reference.contract,
        "digest": hex(&reference.digest),
        "schema_digest": hex(&reference.schema_digest),
    })
}

fn observation_value(position: &EvidencePosition) -> Result<Value, CallFailure> {
    let Some(document) = &position.admitted_document else {
        return Ok(evidence_ref_value(&position.observation));
    };
    let Value::Object(mut document) = document.clone() else {
        return Err(fault("admitted-observation-document-is-an-object"));
    };
    document.remove("identity");
    Ok(Value::Object(document))
}

fn valuation_map(values: &BTreeMap<u64, crate::temporal::Valuation>) -> Value {
    Value::Object(
        values
            .iter()
            .map(|(index, value)| (index.to_string(), valuation_value(value)))
            .collect(),
    )
}

fn instance_valuation_map(
    values: &BTreeMap<SemanticTriggerIdentity, BTreeMap<u64, crate::temporal::Valuation>>,
) -> Value {
    Value::Object(
        values
            .iter()
            .map(|(trigger, values)| (hex(trigger.as_bytes()), valuation_map(values)))
            .collect(),
    )
}

fn valuation_value(value: &crate::temporal::Valuation) -> Value {
    match value {
        crate::temporal::Valuation::True => json!("true"),
        crate::temporal::Valuation::False => json!("false"),
        crate::temporal::Valuation::Missing => json!("missing"),
        crate::temporal::Valuation::Undefined(cause) => {
            json!({"undefined": undefined_value(cause)})
        }
    }
}

fn aggregate_map(values: &BTreeMap<u64, AggregateSample>) -> Value {
    Value::Object(
        values
            .iter()
            .map(|(index, value)| (index.to_string(), aggregate_value(value)))
            .collect(),
    )
}

fn instance_aggregate_map(
    values: &BTreeMap<SemanticTriggerIdentity, BTreeMap<u64, AggregateSample>>,
) -> Value {
    Value::Object(
        values
            .iter()
            .map(|(trigger, values)| (hex(trigger.as_bytes()), aggregate_map(values)))
            .collect(),
    )
}

fn aggregate_value(value: &AggregateSample) -> Value {
    match value {
        AggregateSample::Excluded => json!("excluded"),
        AggregateSample::Value(value) => json!({"value": rational_value(value)}),
        AggregateSample::Holds(value) => json!({"holds": value}),
        AggregateSample::Counted => json!("counted"),
        AggregateSample::Undefined(cause) => {
            json!({"undefined": undefined_value(cause)})
        }
    }
}

fn rational_value(value: &filament_exact::Rational) -> Value {
    json!({
        "kind": "rational",
        "numerator": value.numerator().to_string(),
        "denominator": value.denominator().to_string(),
    })
}

fn undefined_value(cause: &crate::temporal::letters::UndefinedCause) -> Value {
    use crate::temporal::letters::UndefinedCause;
    match cause {
        UndefinedCause::Kernel(reason) => json!({
            "cause": "kernel",
            "reason": kernel_reason(reason),
        }),
        UndefinedCause::Family(record) => {
            let fields: Map<String, Value> = record
                .fields
                .iter()
                .map(|(name, value)| ((*name).to_owned(), json!(value)))
                .collect();
            json!({
                "cause": "family",
                "fields": fields,
                "reason": record.reason.as_str(),
            })
        }
        UndefinedCause::InsufficientData(data) => json!({
            "cause": "insufficient-data",
            "count": data.count.to_string(),
            "min_count": data.min_count.to_string(),
        }),
    }
}

fn kernel_reason(reason: &filament_exact::Undefined) -> &'static str {
    use filament_exact::Undefined;
    match reason {
        Undefined::DivisionByZero => "division-by-zero",
        Undefined::IeeeNotFinite => "ieee-not-finite",
        Undefined::EmptyReduction => "empty-reduction",
        Undefined::NoneValue => "none-value",
        Undefined::NoElement => "no-element",
        Undefined::SumOutOfDomain => "sum-out-of-domain",
        Undefined::MalformedNumber => "malformed-number",
    }
}

fn hex(bytes: &[u8]) -> String {
    let mut text = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write as _;
        let _ = write!(&mut text, "{byte:02x}");
    }
    text
}

fn clock_unit(clock: &ClockConfiguration) -> &str {
    match clock {
        ClockConfiguration::EventPosition { .. } => "position",
        ClockConfiguration::FixedSample { unit, .. } | ClockConfiguration::Timed { unit } => unit,
        ClockConfiguration::TimestampedEvent { timestamp_unit } => timestamp_unit,
    }
}

fn nanos_per_tick(
    binding: &AuthenticatedBinding,
    scale: Option<&filament_exact::Rational>,
) -> Result<filament_exact::Rational, CallFailure> {
    use filament_exact::{Integer, Rational};
    let Some(scale) = scale else {
        return match binding.clock {
            ClockConfiguration::EventPosition { .. } | ClockConfiguration::FixedSample { .. } => {
                Ok(Rational::from_integer(Integer::from(1_i64)))
            }
            _ => Err(fault("checked-clock-has-time-unit-scale")),
        };
    };
    Ok(scale.mul(&Rational::from_integer(Integer::from(1_000_000_000_i64))))
}

fn scaled_nanos(
    ticks: u64,
    time_factor: &filament_exact::Rational,
    position: &EvidencePosition,
) -> Result<i128, CallFailure> {
    use filament_exact::{Integer, Rational};
    let value = time_factor.mul(&Rational::from_integer(Integer::from(ticks)));
    if !value.is_integer() {
        return Err(unrepresentable(position, "event_time_nanos"));
    }
    value
        .numerator()
        .to_string()
        .parse::<i128>()
        .map_err(|_| unrepresentable(position, "event_time_nanos"))
}

fn unrepresentable(position: &EvidencePosition, member: &str) -> CallFailure {
    refusal(
        "invalid-runtime-input",
        member,
        &[("position", position.observation.identity.clone())],
    )
}

fn anchor_of(
    clock: &ClockConfiguration,
    stamp: u64,
    event_time_nanos: i128,
    time_factor: &filament_exact::Rational,
    position: &EvidencePosition,
) -> Result<Anchor, CallFailure> {
    match clock {
        ClockConfiguration::EventPosition { .. } => Ok(Anchor::EventPosition(stamp)),
        ClockConfiguration::TimestampedEvent { .. } | ClockConfiguration::Timed { .. } => {
            Ok(Anchor::TimestampNanos(event_time_nanos))
        }
        ClockConfiguration::FixedSample { epoch, period, .. } => {
            let epoch_nanos = scaled_clock_number(epoch, time_factor, position)?;
            let period_nanos = scaled_clock_number(period, time_factor, position)?;
            let period_nanos = u64::try_from(period_nanos)
                .map_err(|_| unrepresentable(position, "period_nanos"))?;
            if period_nanos == 0 {
                return Err(unrepresentable(position, "period_nanos"));
            }
            Ok(Anchor::FixedSample {
                index: stamp,
                epoch_nanos,
                period_nanos,
            })
        }
    }
}

fn scaled_clock_number(
    number: &ClockExactNumber,
    time_factor: &filament_exact::Rational,
    position: &EvidencePosition,
) -> Result<i128, CallFailure> {
    use filament_exact::{Integer, Rational};
    let number = match number {
        ClockExactNumber::Integer { decimal } => Rational::from_integer(
            decimal
                .parse::<i128>()
                .map(Integer::from)
                .map_err(|_| unrepresentable(position, "clock_number"))?,
        ),
        ClockExactNumber::Rational {
            numerator,
            denominator,
        } => {
            let numerator = numerator
                .parse::<i128>()
                .map(Integer::from)
                .map_err(|_| unrepresentable(position, "clock_number"))?;
            let denominator = denominator
                .parse::<i128>()
                .map(Integer::from)
                .map_err(|_| unrepresentable(position, "clock_number"))?;
            Rational::new(numerator, denominator)
                .map_err(|_| unrepresentable(position, "clock_number"))?
        }
    };
    let value = number.mul(time_factor);
    if !value.is_integer() {
        return Err(unrepresentable(position, "clock_number"));
    }
    value
        .numerator()
        .to_string()
        .parse::<i128>()
        .map_err(|_| unrepresentable(position, "clock_number"))
}

fn clock_range(
    clock: &ClockConfiguration,
    anchors: &[Anchor],
    time_factor: &filament_exact::Rational,
) -> Result<ClockRange, CallFailure> {
    let first = anchors
        .first()
        .copied()
        .ok_or_else(|| fault("owner-trace-is-nonempty"))?;
    match (clock, first) {
        (ClockConfiguration::EventPosition { .. }, Anchor::EventPosition(_)) => {
            let min = anchors
                .iter()
                .filter_map(|anchor| match anchor {
                    Anchor::EventPosition(value) => Some(*value),
                    _ => None,
                })
                .min()
                .ok_or_else(|| fault("event-position-anchors-match-clock"))?;
            let max = anchors
                .iter()
                .filter_map(|anchor| match anchor {
                    Anchor::EventPosition(value) => Some(*value),
                    _ => None,
                })
                .max()
                .ok_or_else(|| fault("event-position-anchors-match-clock"))?;
            Ok(ClockRange::EventPosition {
                start: min,
                end_exclusive: max
                    .checked_add(1)
                    .ok_or_else(|| fault("clock-range-fits"))?,
            })
        }
        (ClockConfiguration::FixedSample { epoch, period, .. }, Anchor::FixedSample { .. }) => {
            let min = anchors
                .iter()
                .filter_map(|anchor| match anchor {
                    Anchor::FixedSample { index, .. } => Some(*index),
                    _ => None,
                })
                .min()
                .ok_or_else(|| fault("fixed-sample-anchors-match-clock"))?;
            let max = anchors
                .iter()
                .filter_map(|anchor| match anchor {
                    Anchor::FixedSample { index, .. } => Some(*index),
                    _ => None,
                })
                .max()
                .ok_or_else(|| fault("fixed-sample-anchors-match-clock"))?;
            let exemplar = anchors
                .iter()
                .find_map(|anchor| match anchor {
                    Anchor::FixedSample {
                        epoch_nanos,
                        period_nanos,
                        ..
                    } => Some((*epoch_nanos, *period_nanos)),
                    _ => None,
                })
                .ok_or_else(|| fault("fixed-sample-clock-has-anchor"))?;
            let _ = (epoch, period, time_factor);
            Ok(ClockRange::FixedSample {
                start: min,
                end_exclusive: max
                    .checked_add(1)
                    .ok_or_else(|| fault("clock-range-fits"))?,
                epoch_nanos: exemplar.0,
                period_nanos: exemplar.1,
            })
        }
        (
            ClockConfiguration::TimestampedEvent { .. } | ClockConfiguration::Timed { .. },
            Anchor::TimestampNanos(_),
        ) => {
            let min = anchors
                .iter()
                .filter_map(|anchor| match anchor {
                    Anchor::TimestampNanos(value) => Some(*value),
                    _ => None,
                })
                .min()
                .ok_or_else(|| fault("timestamp-anchors-match-clock"))?;
            let max = anchors
                .iter()
                .filter_map(|anchor| match anchor {
                    Anchor::TimestampNanos(value) => Some(*value),
                    _ => None,
                })
                .max()
                .ok_or_else(|| fault("timestamp-anchors-match-clock"))?;
            let exclusive = max
                .checked_add(1)
                .ok_or_else(|| fault("clock-range-fits"))?;
            if exclusive <= min {
                return Err(fault("timestamp-clock-range-is-nonempty"));
            }
            let _ = time_factor;
            Ok(ClockRange::Timestamp {
                start_nanos: min,
                end_nanos: exclusive,
            })
        }
        _ => Err(fault("owner-anchor-family-matches-clock")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::temporal::finite_window::Valuation;
    use crate::temporal::window::AggregateSample;
    use std::collections::BTreeMap;

    fn admitted(
        positions: &[EvidencePosition],
        cover: ClockCover,
        event_positions: bool,
    ) -> Result<OwnerObservation, CallFailure> {
        let scale = filament_exact::Rational::new(1_i64.into(), 1000_i64.into()).expect("ms scale");
        admitted_with_scale(positions, cover, event_positions, &scale)
    }

    fn admitted_with_scale(
        positions: &[EvidencePosition],
        cover: ClockCover,
        event_positions: bool,
        scale: &filament_exact::Rational,
    ) -> Result<OwnerObservation, CallFailure> {
        use crate::temporal::finite_window::{
            LowerBoundary, TimestampedObservation, Watermark, WindowClock, WindowProgress,
        };
        use crate::temporal::{ProgressClosure, ProgressCompleteness};
        let declaration: CheckedNodeId = serde_json::from_value(json!({
            "package": "recipe", "path": ["Due"], "ordinal": 0
        }))
        .expect("checked node");
        let profile = if event_positions {
            "filament.temporal.event-position.false-extension/v1"
        } else {
            "filament.temporal.timestamped-event.finite-window/v1"
        };
        let clock = if event_positions {
            ClockConfiguration::EventPosition {
                sequence_authority: "events".into(),
            }
        } else {
            ClockConfiguration::TimestampedEvent {
                timestamp_unit: "ms".into(),
            }
        };
        let binding = AuthenticatedBinding {
            package: "recipe".into(),
            declaration: declaration.clone(),
            profile: serde_json::from_value(json!({"authority": "agent-ix", "identity": profile}))
                .expect("profile"),
            clock_name: "clock".into(),
            clock,
        };
        let definition = TraceDefinition {
            package_identity: "recipe",
            canonical_package_bytes: b"{}",
            time_unit_scale: (!event_positions).then_some(scale),
            declaration: &declaration,
            signal: &declaration,
            occurrence: 0,
        };
        let trace = TimestampedTrace::admit_uncertain(
            "Due",
            WindowClock {
                name: "clock".into(),
                profile: profile.into(),
                unit: if event_positions { "position" } else { "ms" }.into(),
                cover,
            },
            positions
                .iter()
                .map(|position| {
                    (
                        TimestampedObservation {
                            stamp: position.position,
                            observation: position.observation.identity.clone(),
                            order_key: position.order_key.clone(),
                        },
                        position.interval,
                    )
                })
                .collect(),
            WindowProgress {
                watermark: Watermark::Inclusive(100),
                completeness: ProgressCompleteness::Complete { through: 100 },
                scope: ProgressClosure::Closed,
                execution: ProgressClosure::Closed,
                lower_boundary: LowerBoundary::AuthoritativeOrigin,
            },
        )
        .expect("admitted trace");
        admit_defined(&definition, positions, cover, &binding, &trace)
    }

    fn position(label: &str, stamp: u64, valuation: Valuation) -> EvidencePosition {
        EvidencePosition {
            observation: EvidenceRef::named(label),
            position: stamp,
            valuations: BTreeMap::from([(0, valuation)]),
            ..EvidencePosition::default()
        }
    }

    #[test]
    #[ix_trace_rs::trace("TC-3036", "FR-944-AC-8", "FR-944-AC-13", "FR-944-AC-2")]
    fn owner_admission_uses_derived_membership_and_quarter_tick_units() {
        for event_positions in [false, true] {
            let owner = admitted(
                &[position("obs3", 3, Valuation::True)],
                ClockCover::Counter { uncertainty: 1 },
                event_positions,
            )
            .expect("owner admission and strict partial read");
            let qualified = &owner.qualified;
            let unit = if event_positions {
                "position/4"
            } else {
                "ms/4"
            };
            assert_eq!(qualified.binding().unit.as_str(), unit);
            assert_eq!(qualified.records()[0].unit.as_str(), unit);
            assert_eq!(
                qualified.records()[0].event_time_nanos,
                if event_positions { 3 } else { 3_000_000 }
            );
            let ScopeKind::Snapshot { snapshot_identity } = &qualified.scope().kind else {
                panic!("snapshot scope");
            };
            assert_eq!(
                snapshot_identity,
                &qualified.scope().membership_rule_identity
            );
            assert_eq!(
                owner.side_map.values().collect::<Vec<_>>(),
                vec![&vec!["obs3".to_owned()]]
            );
            assert_eq!(owner.partial_facts.len(), 1);
            assert!(!owner.partial_facts[0].view.payload().can_be_false());
            assert!(owner.partial_facts[0].view.payload().can_be_true());
            assert_eq!(
                owner.partial_facts[0].view.payload().reason(),
                partial::ValueReason::Known
            );
            let interval = owner.partial_facts[0].view.payload().interval();
            assert_eq!(
                (interval.earliest, interval.latest, interval.unit),
                ("8", "19", unit)
            );
        }
    }

    #[test]
    #[ix_trace_rs::trace("TC-3036", "FR-944-AC-11", "FR-944-AC-12")]
    fn undefined_atoms_are_record_content_without_boolean_partial_facts() {
        use crate::temporal::letters::UndefinedCause;
        let undefined = UndefinedCause::Kernel(filament_exact::Undefined::DivisionByZero);
        let positions = [
            position("obs0", 0, Valuation::True),
            position("obs1", 1, Valuation::True),
            position("obs2", 2, Valuation::Undefined(undefined)),
        ];
        let owner = admitted(&positions, ClockCover::Counter { uncertainty: 0 }, false)
            .expect("undefined is admissible record content");
        assert_eq!(owner.partial_facts.len(), 2);
        let undefined_record = owner
            .side_map
            .iter()
            .find(|(_, labels)| labels == &&vec!["obs2".to_owned()])
            .expect("undefined record mapping")
            .0;
        assert!(owner
            .partial_facts
            .iter()
            .all(|fact| &fact.record_identity != undefined_record));
        let record = owner
            .qualified
            .records()
            .iter()
            .find(|record| &record.identity == undefined_record)
            .expect("undefined record");
        let ValueState::Present {
            canonical_value, ..
        } = &record.value
        else {
            panic!("present record");
        };
        let value: Value = serde_json::from_str(canonical_value).expect("record value");
        assert_eq!(
            value["valuations"]["0"],
            json!({"undefined": {"cause": "kernel", "reason": "division-by-zero"}})
        );
        assert_eq!(
            owner
                .valuation("obs2", 0, None, &positions[2].valuations[&0])
                .expect("undefined bypasses owner possibilities"),
            positions[2].valuations[&0]
        );
        assert!(
            owner
                .valuation("obs2", 0, None, &Valuation::Missing)
                .is_err(),
            "undefined was never admitted as missing"
        );
    }

    #[test]
    #[ix_trace_rs::trace("TC-3036", "FR-944-AC-8", "FR-944-AC-11")]
    fn relabelled_equal_facts_share_one_owner_record_but_keep_both_labels() {
        let first = position("first", 3, Valuation::True);
        let second = position("second", 3, Valuation::True);
        let owner = admitted(&[first.clone(), second], ClockCover::Exact, false)
            .expect("equal facts coalesce");
        assert_eq!(owner.qualified.records().len(), 1);
        assert_eq!(owner.qualified.scope().members.len(), 1);
        assert_eq!(owner.partial_facts.len(), 1);
        assert_eq!(
            owner.side_map.values().next(),
            Some(&vec!["first".to_owned(), "second".to_owned()])
        );
        let mut changed = first.clone();
        changed.valuations.insert(0, Valuation::False);
        let successor = admitted(&[changed], ClockCover::Exact, false).expect("changed fact");
        assert_ne!(
            owner.qualified.records()[0].identity,
            successor.qualified.records()[0].identity
        );
    }

    #[test]
    #[ix_trace_rs::trace("TC-3036", "FR-944-AC-11")]
    fn every_position_content_change_changes_its_record_but_reference_labels_do_not() {
        let instance = SemanticTriggerIdentity::new(vec![0, 255]).expect("trigger");
        let mut original = position("original", 3, Valuation::True);
        original
            .aggregate_samples
            .insert(1, AggregateSample::Counted);
        original
            .instance_valuations
            .insert(instance.clone(), BTreeMap::from([(2, Valuation::True)]));
        original.instance_aggregate_samples.insert(
            instance.clone(),
            BTreeMap::from([(3, AggregateSample::Excluded)]),
        );
        let identity = |position| {
            admitted(&[position], ClockCover::Exact, false)
                .expect("admitted fact")
                .qualified
                .records()[0]
                .identity
                .clone()
        };
        let baseline = identity(original.clone());
        let mut relabelled = original.clone();
        relabelled.observation.identity = "renamed".to_owned();
        relabelled.observation.authority_identity = "new-authority-label".to_owned();
        relabelled.observation.authority_revision = "new-revision-label".to_owned();
        relabelled.observation.scope = "new-scope-label".to_owned();
        relabelled.observation.population = "new-population-label".to_owned();
        assert_eq!(identity(relabelled), baseline);
        let mut content = original.clone();
        content.observation.digest = vec![0xab];
        let mut valuation = original.clone();
        valuation.valuations.insert(0, Valuation::False);
        let mut aggregate = original.clone();
        aggregate
            .aggregate_samples
            .insert(1, AggregateSample::Excluded);
        let mut bound_value = original.clone();
        bound_value
            .instance_valuations
            .insert(instance.clone(), BTreeMap::from([(2, Valuation::False)]));
        let mut bound_aggregate = original;
        bound_aggregate
            .instance_aggregate_samples
            .insert(instance, BTreeMap::from([(3, AggregateSample::Counted)]));
        for (member, changed) in [
            ("content", content),
            ("valuation", valuation),
            ("aggregate", aggregate),
            ("instance valuation", bound_value),
            ("instance aggregate", bound_aggregate),
        ] {
            assert_ne!(identity(changed), baseline, "changed {member}");
        }
    }

    #[test]
    #[ix_trace_rs::trace("TC-3036", "FR-944-AC-8")]
    fn recorded_arrivals_are_scaled_and_absence_is_not_an_asserted_instant() {
        let mut observed = position("arrival", 3, Valuation::True);
        observed.arrival = Some(8);
        let owner = admitted(&[observed], ClockCover::Exact, false).expect("recorded arrival");
        assert_eq!(
            owner.qualified.records()[0].ingestion_time_nanos,
            Some(8_000_000)
        );
        let owner = admitted(
            &[position("absent", 3, Valuation::True)],
            ClockCover::Exact,
            false,
        )
        .expect("placeholder arrival");
        assert_eq!(owner.qualified.records()[0].ingestion_time_nanos, None);
    }

    #[test]
    #[ix_trace_rs::trace("TC-3036", "FR-944-AC-9")]
    fn oversized_record_values_are_admission_limits_before_owner_derivation() {
        use filament_eval::CallAdmissionFailure;
        let mut observed = position("oversized", 3, Valuation::True);
        let maximum = Limits::owner_max().max_string_bytes;
        observed.observation.contract = "x".repeat(maximum);
        let expected_length = canonical_position_value(&observed)
            .expect("canonical text")
            .len();
        let failure = match admitted(&[observed], ClockCover::Exact, false) {
            Ok(_) => panic!("oversized record value must refuse"),
            Err(failure) => failure,
        };
        let CallFailure::Admission(failure) = failure else {
            panic!("admission limit");
        };
        let CallAdmissionFailure::Refused(record) = *failure else {
            panic!("refused input limit");
        };
        assert_eq!(record.record.catalog.code(), "stage_limit_exceeded");
        assert_eq!(record.record.catalog.cause(), "input-bytes-exceeded");
        assert_eq!(record.record.fields["bound"], maximum.to_string());
        assert_eq!(record.record.fields["actual"], expected_length.to_string());
        assert_eq!(record.record.fields["position"], "oversized");
        assert_eq!(
            record.record.fields["setting"],
            "filament_observation::authority::Limits.max_string_bytes"
        );
    }

    #[test]
    #[ix_trace_rs::trace("TC-2624", "FR-944-AC-2")]
    fn a_nonintegral_nanosecond_stamp_refuses_without_rounding() {
        use filament_eval::CallAdmissionFailure;
        let half_nanosecond = filament_exact::Rational::new(1_i64.into(), 2_000_000_000_i64.into())
            .expect("half-nanosecond unit");
        let failure = match admitted_with_scale(
            &[position("half-ns", 3, Valuation::True)],
            ClockCover::Exact,
            false,
            &half_nanosecond,
        ) {
            Ok(_) => panic!("nonintegral nanoseconds must refuse"),
            Err(failure) => failure,
        };
        let CallFailure::Admission(failure) = failure else {
            panic!("admission refusal");
        };
        let CallAdmissionFailure::Refused(record) = *failure else {
            panic!("invalid value");
        };
        assert_eq!(record.record.catalog.code(), "invalid_runtime_input");
        assert_eq!(record.record.catalog.cause(), "invalid-value");
        assert_eq!(record.record.fields["position"], "half-ns");
    }

    #[test]
    #[ix_trace_rs::trace("TC-3036", "FR-944-AC-11", "FR-944-AC-12")]
    fn record_value_preserves_unbounded_rationals_instances_and_all_undefined_causes() {
        use crate::temporal::letters::UndefinedCause;
        use crate::temporal::window::InsufficientData;
        use filament_semantic_value::catalog::{UndefinedReason, UndefinedRecord};
        let instance = SemanticTriggerIdentity::new(vec![0, 255]).expect("opaque trigger");
        let family = UndefinedCause::Family(UndefinedRecord {
            reason: UndefinedReason::AbsentKey,
            fields: BTreeMap::from([("key", "missing".to_owned())]),
        });
        let insufficient = UndefinedCause::InsufficientData(InsufficientData {
            count: 1,
            min_count: 2,
        });
        let mut observed = position("all-facts", 3, Valuation::Undefined(family));
        observed.aggregate_samples = BTreeMap::from([
            (
                1,
                AggregateSample::Value(
                    filament_exact::Rational::new(
                        "18446744073709551616".parse().expect("unbounded integer"),
                        3_i64.into(),
                    )
                    .expect("unbounded rational"),
                ),
            ),
            (2, AggregateSample::Undefined(insufficient.clone())),
        ]);
        observed
            .instance_valuations
            .insert(instance.clone(), BTreeMap::from([(3, Valuation::False)]));
        observed.instance_aggregate_samples.insert(
            instance.clone(),
            BTreeMap::from([(4, AggregateSample::Undefined(insufficient))]),
        );
        let owner =
            admitted(&[observed.clone()], ClockCover::Exact, false).expect("all content admits");
        assert_eq!(
            owner.partial_facts.len(),
            1,
            "only the defined instance Boolean yields a partial fact"
        );
        assert_eq!(owner.partial_facts[0].atom, 3);
        assert_eq!(owner.partial_facts[0].instance.as_ref(), Some(&instance));
        let value: Value =
            serde_json::from_str(&canonical_position_value(&observed).expect("record value"))
                .expect("JSON");
        assert_eq!(
            value["aggregate_samples"]["1"],
            json!({"value": {"kind": "rational", "numerator": "18446744073709551616", "denominator": "3"}})
        );
        assert_eq!(
            value["valuations"]["0"],
            json!({"undefined": {"cause": "family", "reason": "absent-key", "fields": {"key": "missing"}}})
        );
        assert_eq!(
            value["aggregate_samples"]["2"],
            json!({"undefined": {"cause": "insufficient-data", "count": "1", "min_count": "2"}})
        );
        assert_eq!(value["instance_valuations"]["00ff"]["3"], "false");
        assert_eq!(
            value["instance_aggregate_samples"]["00ff"]["4"],
            value["aggregate_samples"]["2"]
        );
    }

    #[test]
    #[ix_trace_rs::trace("TC-3036", "FR-944-AC-11")]
    fn w70_value_has_exact_seven_member_shape_and_opaque_evidence_content() {
        let position = EvidencePosition {
            observation: EvidenceRef {
                identity: "display label is not content".to_owned(),
                authority_identity: "also omitted".to_owned(),
                authority_revision: "omitted too".to_owned(),
                scope: "not interpreted".to_owned(),
                population: "not interpreted".to_owned(),
                contract: "opaque-contract".to_owned(),
                schema_digest: vec![0, 0xab],
                digest: vec![0xcd],
                authority_digest: vec![0xef],
            },
            position: 12,
            valuations: BTreeMap::from([(3, Valuation::True)]),
            aggregate_samples: BTreeMap::from([(4, AggregateSample::Holds(false))]),
            ..EvidencePosition::default()
        };
        let encoded = canonical_position_value(&position).expect("the W70 record value");
        let parsed: Value = serde_json::from_str(&encoded).expect("canonical JSON parses");
        let object = parsed.as_object().expect("record value is an object");
        assert_eq!(object.len(), 7);
        assert_eq!(
            object.keys().map(String::as_str).collect::<Vec<_>>(),
            [
                "aggregate_samples",
                "instance_aggregate_samples",
                "instance_valuations",
                "interval",
                "observation",
                "order_key",
                "valuations"
            ]
        );
        assert_eq!(
            object["observation"],
            json!({
                "authority_digest": "ef",
                "contract": "opaque-contract",
                "digest": "cd",
                "schema_digest": "00ab"
            })
        );
        assert_eq!(object["valuations"]["3"], json!("true"));
        assert_eq!(object["aggregate_samples"]["4"], json!({"holds": false}));
    }

    #[test]
    #[ix_trace_rs::trace("TC-3036", "FR-944-AC-11")]
    fn retained_spine_document_supplies_d_without_its_identity() {
        let position = EvidencePosition {
            observation: EvidenceRef::named("an-untrusted-display-label"),
            admitted_document: Some(json!({
                "identity": "not part of D",
                "value": {"n": 5},
                "source": "admitted semantic content"
            })),
            ..EvidencePosition::default()
        };
        assert_eq!(
            observation_value(&position).expect("the admitted document value"),
            json!({
                "value": {"n": 5},
                "source": "admitted semantic content"
            })
        );
    }
}
