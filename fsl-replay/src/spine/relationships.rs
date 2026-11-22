// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-1045: decode an explicit native-run relationship selection and hand it
//! to filament-observation without adding correlation semantics here.

use filament_observation::{
    admit, AdmissionOutcome, AdmissionRequest, AdmittedBundleKey, AdmittedRecord,
    AdmittedStaticBundle, Anchor, ClockRange, Digest, Identity, Member, ObservationBinding,
    PackageSelection, ProducerDigest, QualifiedSubject, Relationship, RelationshipExtent,
    RelationshipExtentState, RequiredRelationship, ResourceLimits, Revision, ScopeKind,
    ScopeSelection, StaticProducerBundle, SubjectIdentity, SubjectKind, ValueState, Visibility,
    NATIVE_LINKED_PACKAGE_FORMAT,
};
use serde_json::Value;
use std::collections::BTreeSet;

/// A malformed FR-1045 request member and its full JSON Pointer.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReadError {
    /// Pointer to the malformed or absent member.
    pub path: String,
    /// Stable structural explanation for the command adapter.
    pub reason: String,
}

type ReadResult<T> = Result<T, ReadError>;

/// Fully decoded caller selection, ready to combine with admitted snapshot
/// relationship extents and the selected checked-package context.
#[derive(Clone, Debug)]
pub struct RelationshipSelection {
    producer: AdmittedStaticBundle,
    binding: BindingParts,
    expected_subject: SubjectParts,
    required: Vec<RequiredParts>,
    relationship_identities: Vec<RelationshipIdentityMapping>,
    scope: ScopeSelection,
    records: Vec<RecordParts>,
    limits: ResourceLimits,
}

#[derive(Clone, Debug)]
struct RelationshipIdentityMapping {
    relationship: String,
    declaration_identity: String,
}

#[derive(Clone, Debug)]
struct BindingParts {
    identity: Identity,
    source_identity: Identity,
    schema_identity: Identity,
    signal_identity: Identity,
    trigger_identity: Identity,
    unit: Identity,
    subject_kind: String,
    required: bool,
}

#[derive(Clone, Debug)]
struct SubjectParts {
    authority: AdmittedBundleKey,
    subject_kind: String,
    identity: Vec<u8>,
}

#[derive(Clone, Debug)]
struct RecordParts {
    identity: Identity,
    binding_identity: Identity,
    source_identity: Identity,
    schema_identity: Identity,
    subject: SubjectParts,
    signal_identity: Identity,
    trigger_identity: Identity,
    unit: Identity,
    value: ValueState,
    visibility: Visibility,
    anchor: Anchor,
    event_time_nanos: i128,
    ingestion_time_nanos: Option<i128>,
    causal_relationship_identity: Option<filament_observation::RelationshipIdentity>,
    clock_identity: Identity,
    clock_revision: Identity,
    clock_uncertainty_nanos: u64,
}

#[derive(Clone, Debug)]
struct RequiredParts {
    declaration_identity: String,
    source: SubjectParts,
    target: SubjectParts,
}

impl RelationshipSelection {
    /// Reads the closed request object. This method performs shape decoding
    /// only; relationship constructors and resolver outcomes are deferred
    /// until the snapshot has been admitted.
    pub fn read(value: &Value) -> ReadResult<Self> {
        let root = object(value, "/call/relationships")?;
        closed(
            root,
            &[
                "producer",
                "binding",
                "expected_subject",
                "required_relationships",
                "relationship_identities",
                "scope",
                "records",
                "limits",
            ],
            "/call/relationships",
        )?;

        let producer_value = member(root, "producer", "/call/relationships")?;
        let producer_bytes = serde_json::to_vec(producer_value).map_err(|_| ReadError {
            path: "/call/relationships/producer".to_owned(),
            reason: "expected a Producer interface 1.2 bundle".to_owned(),
        })?;
        let producer =
            StaticProducerBundle::admit_json(&producer_bytes).map_err(|error| ReadError {
                path: "/call/relationships/producer".to_owned(),
                reason: error.to_string(),
            })?;

        let binding_path = "/call/relationships/binding";
        let binding_value = object(
            member(root, "binding", "/call/relationships")?,
            binding_path,
        )?;
        closed(
            binding_value,
            &[
                "identity",
                "source_identity",
                "schema_identity",
                "signal_identity",
                "trigger_identity",
                "unit",
                "subject_kind",
                "required",
            ],
            binding_path,
        )?;
        let binding = BindingParts {
            identity: identity(
                member(binding_value, "identity", binding_path)?,
                &format!("{binding_path}/identity"),
            )?,
            source_identity: identity(
                member(binding_value, "source_identity", binding_path)?,
                &format!("{binding_path}/source_identity"),
            )?,
            schema_identity: identity(
                member(binding_value, "schema_identity", binding_path)?,
                &format!("{binding_path}/schema_identity"),
            )?,
            signal_identity: identity(
                member(binding_value, "signal_identity", binding_path)?,
                &format!("{binding_path}/signal_identity"),
            )?,
            trigger_identity: identity(
                member(binding_value, "trigger_identity", binding_path)?,
                &format!("{binding_path}/trigger_identity"),
            )?,
            unit: identity(
                member(binding_value, "unit", binding_path)?,
                &format!("{binding_path}/unit"),
            )?,
            subject_kind: text(
                member(binding_value, "subject_kind", binding_path)?,
                &format!("{binding_path}/subject_kind"),
            )?,
            required: boolean(
                member(binding_value, "required", binding_path)?,
                &format!("{binding_path}/required"),
            )?,
        };
        let expected_subject = read_subject(
            member(root, "expected_subject", "/call/relationships")?,
            "/call/relationships/expected_subject",
        )?;

        let required_path = "/call/relationships/required_relationships";
        let required_values = array(
            member(root, "required_relationships", "/call/relationships")?,
            required_path,
        )?;
        let mut required = Vec::with_capacity(required_values.len());
        for (index, slot_value) in required_values.iter().enumerate() {
            let path = format!("{required_path}/{index}");
            let slot = object(slot_value, &path)?;
            closed(slot, &["declaration_identity", "source", "target"], &path)?;
            required.push(RequiredParts {
                declaration_identity: text(
                    member(slot, "declaration_identity", &path)?,
                    &format!("{path}/declaration_identity"),
                )?,
                source: read_subject(member(slot, "source", &path)?, &format!("{path}/source"))?,
                target: read_subject(member(slot, "target", &path)?, &format!("{path}/target"))?,
            });
        }

        let mappings_path = "/call/relationships/relationship_identities";
        let mapping_values = array(
            member(root, "relationship_identities", "/call/relationships")?,
            mappings_path,
        )?;
        let mut relationship_names = BTreeSet::new();
        let mut declaration_names = BTreeSet::new();
        let mut relationship_identities = Vec::with_capacity(mapping_values.len());
        for (index, mapping_value) in mapping_values.iter().enumerate() {
            let path = format!("{mappings_path}/{index}");
            let mapping = object(mapping_value, &path)?;
            closed(mapping, &["relationship", "declaration_identity"], &path)?;
            let relationship_path = format!("{path}/relationship");
            let relationship = text(member(mapping, "relationship", &path)?, &relationship_path)?;
            let declaration_path = format!("{path}/declaration_identity");
            let declaration_identity = text(
                member(mapping, "declaration_identity", &path)?,
                &declaration_path,
            )?;
            if !relationship_names.insert(relationship.clone()) {
                return Err(error(
                    &relationship_path,
                    "relationship is mapped more than once",
                ));
            }
            if !declaration_names.insert(declaration_identity.clone()) {
                return Err(error(
                    &declaration_path,
                    "declaration_identity is mapped more than once",
                ));
            }
            relationship_identities.push(RelationshipIdentityMapping {
                relationship,
                declaration_identity,
            });
        }

        let scope_path = "/call/relationships/scope";
        let scope_value = object(member(root, "scope", "/call/relationships")?, scope_path)?;
        closed(
            scope_value,
            &[
                "population_identity",
                "membership_rule_identity",
                "membership_digest",
                "membership_document",
                "required_member_identities",
                "observation_sources",
                "completeness_dependencies",
                "progress_dependencies",
                "clock_identity",
                "clock_revision",
                "membership_complete",
                "closure_identity",
                "closure_digest",
                "kind",
                "range",
                "members",
            ],
            scope_path,
        )?;
        let membership_document_path = format!("{scope_path}/membership_document");
        let membership_document = text(
            member(scope_value, "membership_document", scope_path)?,
            &membership_document_path,
        )?
        .into_bytes();
        let members_path = format!("{scope_path}/members");
        let member_values = array(member(scope_value, "members", scope_path)?, &members_path)?;
        let mut members = Vec::with_capacity(member_values.len());
        for (index, member_value) in member_values.iter().enumerate() {
            let path = format!("{members_path}/{index}");
            let member_object = object(member_value, &path)?;
            closed(
                member_object,
                &["object_identity", "record_identity", "anchor"],
                &path,
            )?;
            members.push(Member {
                object_identity: identity(
                    member(member_object, "object_identity", &path)?,
                    &format!("{path}/object_identity"),
                )?,
                record_identity: identity(
                    member(member_object, "record_identity", &path)?,
                    &format!("{path}/record_identity"),
                )?,
                anchor: read_anchor(
                    member(member_object, "anchor", &path)?,
                    &format!("{path}/anchor"),
                )?,
            });
        }
        let scope = ScopeSelection {
            population_identity: identity(
                member(scope_value, "population_identity", scope_path)?,
                &format!("{scope_path}/population_identity"),
            )?,
            membership_rule_identity: identity(
                member(scope_value, "membership_rule_identity", scope_path)?,
                &format!("{scope_path}/membership_rule_identity"),
            )?,
            membership_digest: digest(
                member(scope_value, "membership_digest", scope_path)?,
                &format!("{scope_path}/membership_digest"),
                false,
            )?,
            membership_document,
            required_member_identities: identity_array(
                member(scope_value, "required_member_identities", scope_path)?,
                &format!("{scope_path}/required_member_identities"),
            )?,
            observation_sources: identity_array(
                member(scope_value, "observation_sources", scope_path)?,
                &format!("{scope_path}/observation_sources"),
            )?,
            completeness_dependencies: identity_array(
                member(scope_value, "completeness_dependencies", scope_path)?,
                &format!("{scope_path}/completeness_dependencies"),
            )?,
            progress_dependencies: identity_array(
                member(scope_value, "progress_dependencies", scope_path)?,
                &format!("{scope_path}/progress_dependencies"),
            )?,
            clock_identity: identity(
                member(scope_value, "clock_identity", scope_path)?,
                &format!("{scope_path}/clock_identity"),
            )?,
            clock_revision: identity(
                member(scope_value, "clock_revision", scope_path)?,
                &format!("{scope_path}/clock_revision"),
            )?,
            membership_complete: boolean(
                member(scope_value, "membership_complete", scope_path)?,
                &format!("{scope_path}/membership_complete"),
            )?,
            closure_identity: optional_identity(
                member(scope_value, "closure_identity", scope_path)?,
                &format!("{scope_path}/closure_identity"),
            )?,
            closure_digest: optional_digest(
                member(scope_value, "closure_digest", scope_path)?,
                &format!("{scope_path}/closure_digest"),
            )?,
            kind: read_scope_kind(
                member(scope_value, "kind", scope_path)?,
                &format!("{scope_path}/kind"),
            )?,
            range: read_range(
                member(scope_value, "range", scope_path)?,
                &format!("{scope_path}/range"),
            )?,
            members,
        };

        let records_path = "/call/relationships/records";
        let record_values = array(
            member(root, "records", "/call/relationships")?,
            records_path,
        )?;
        let mut records = Vec::with_capacity(record_values.len());
        for (index, value) in record_values.iter().enumerate() {
            records.push(read_record(value, &format!("{records_path}/{index}"))?);
        }
        let limits_path = "/call/relationships/limits";
        let limits_value = object(member(root, "limits", "/call/relationships")?, limits_path)?;
        closed(
            limits_value,
            &[
                "max_records",
                "max_members",
                "max_relationships",
                "max_required_relationships",
            ],
            limits_path,
        )?;
        let limits = ResourceLimits {
            max_records: limit(
                member(limits_value, "max_records", limits_path)?,
                &format!("{limits_path}/max_records"),
            )?,
            max_members: limit(
                member(limits_value, "max_members", limits_path)?,
                &format!("{limits_path}/max_members"),
            )?,
            max_relationships: limit(
                member(limits_value, "max_relationships", limits_path)?,
                &format!("{limits_path}/max_relationships"),
            )?,
            max_required_relationships: limit(
                member(limits_value, "max_required_relationships", limits_path)?,
                &format!("{limits_path}/max_required_relationships"),
            )?,
        };
        Ok(Self {
            producer,
            binding,
            expected_subject,
            required,
            relationship_identities,
            scope,
            records,
            limits,
        })
    }

    /// Returns whether the selected Producer bundle declares this exact
    /// relationship identity.
    pub fn declaration_identity_for(&self, relationship: &str) -> Option<&str> {
        self.relationship_identities
            .iter()
            .find(|mapping| mapping.relationship == relationship)
            .map(|mapping| mapping.declaration_identity.as_str())
    }

    /// The explicit caller correspondence, in its wire order.
    pub fn relationship_mappings(&self) -> impl Iterator<Item = (&str, &str)> {
        self.relationship_identities.iter().map(|mapping| {
            (
                mapping.relationship.as_str(),
                mapping.declaration_identity.as_str(),
            )
        })
    }

    /// The admitted Producer bundle used to qualify every resolver relationship.
    pub fn producer(&self) -> &AdmittedStaticBundle {
        &self.producer
    }

    /// Refuses a mapping whose local relationship is not a navigation
    /// relationship in the selected, admitted model views.

    /// Builds the exact owner request after snapshot admission and returns the
    /// owner's outcome unchanged.
    pub fn admit(
        self,
        package: PackageSelection,
        snapshot_extents: Option<&Value>,
        objects: &fsl_semantics::model::object_environment::ObjectEnvironment,
    ) -> AdmissionOutcome {
        match self.into_admission_request(package, snapshot_extents, objects) {
            Ok(request) => admit(request),
            Err(cause) => AdmissionOutcome::Refused { cause },
        }
    }

    fn into_admission_request(
        self,
        package: PackageSelection,
        snapshot_extents: Option<&Value>,
        objects: &fsl_semantics::model::object_environment::ObjectEnvironment,
    ) -> Result<AdmissionRequest, filament_observation::RefusalCause> {
        let binding_subject_kind = match SubjectKind::new(self.binding.subject_kind.into_bytes()) {
            Ok(kind) => kind,
            Err(error) => return Err(filament_observation::RefusalCause::InvalidReference(error)),
        };
        let binding = ObservationBinding {
            identity: self.binding.identity,
            source_identity: self.binding.source_identity,
            schema_identity: self.binding.schema_identity,
            signal_identity: self.binding.signal_identity,
            trigger_identity: self.binding.trigger_identity,
            unit: self.binding.unit,
            subject_kind: binding_subject_kind,
            required: self.binding.required,
        };
        let expected_subject = match self.expected_subject.into_subject() {
            Ok(subject) => subject,
            Err(error) => return Err(filament_observation::RefusalCause::InvalidReference(error)),
        };
        let required_relationships = (|| {
            let mut relationships = Vec::with_capacity(self.required.len());
            for slot in self.required {
                let source = slot.source.into_subject()?;
                let target = slot.target.into_subject()?;
                relationships.push(RequiredRelationship::new(
                    &self.producer,
                    slot.declaration_identity,
                    source,
                    target,
                )?);
            }
            Ok::<_, filament_observation::ReferenceRefusal>(relationships)
        })();
        let required_relationships = match required_relationships {
            Ok(value) => value,
            Err(error) => return Err(filament_observation::RefusalCause::InvalidReference(error)),
        };
        let records = match self
            .records
            .into_iter()
            .map(RecordParts::into_record)
            .collect::<Result<Vec<_>, _>>()
        {
            Ok(records) => records,
            Err(error) => return Err(filament_observation::RefusalCause::InvalidReference(error)),
        };
        let (relationships, relationship_extents) = match read_extents(
            &self.producer,
            snapshot_extents,
            objects,
            &self.relationship_identities,
        ) {
            Ok(value) => value,
            Err(error) => return Err(filament_observation::RefusalCause::InvalidReference(error)),
        };
        Ok(AdmissionRequest {
            package,
            producer: self.producer,
            binding,
            expected_subject,
            relationships,
            relationship_extents,
            required_relationships,
            scope: self.scope,
            records,
            limits: self.limits,
        })
    }
}

/// Returns the first snapshot relationship path that does not name a
/// navigation declaration in the selected model package views. The caller
/// invokes this only after FR-106 has admitted the same snapshot document.
pub fn unknown_snapshot_relationship(
    extents: Option<&Value>,
    navigation_relationships: &std::collections::BTreeSet<String>,
) -> Option<String> {
    let extents = extents?.as_array()?;
    extents.iter().enumerate().find_map(|(index, extent)| {
        let relationship = extent.get("relationship")?.as_str()?;
        (!navigation_relationships.contains(relationship))
            .then(|| format!("/call/snapshot/relationships/{index}/relationship"))
    })
}

/// Returns the first malformed identity on a mapped candidate, after FR-106
/// has admitted the sanitized snapshot.
pub fn invalid_snapshot_relationship_identity(
    extents: Option<&Value>,
    selection: &RelationshipSelection,
) -> Option<String> {
    let extents = extents?.as_array()?;
    extents.iter().enumerate().find_map(|(extent_index, extent)| {
        let relationship = extent.get("relationship")?.as_str()?;
        if selection.declaration_identity_for(relationship).is_none() {
            return None;
        }
        let instances = extent.get("instances")?.as_array()?;
        instances
            .iter()
            .enumerate()
            .find_map(|(instance_index, instance)| {
                let identity = instance.get("identity").and_then(Value::as_str);
                identity
                    .is_none_or(|identity| !is_nonempty_lower_hex(identity))
                    .then(|| {
                        format!(
                            "/call/snapshot/relationships/{extent_index}/instances/{instance_index}/identity"
                        )
                    })
            })
    })
}

fn is_nonempty_lower_hex(value: &str) -> bool {
    !value.is_empty()
        && value.len().is_multiple_of(2)
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

/// Finds the pointer for a declaration-specific FR-106 role-mapping refusal.
pub fn snapshot_relationship_member(extents: Option<&Value>, relationship: &str) -> Option<String> {
    extents?
        .as_array()?
        .iter()
        .enumerate()
        .find_map(|(index, extent)| {
            (extent.get("relationship")?.as_str()? == relationship)
                .then(|| format!("/call/snapshot/relationships/{index}/relationship"))
        })
}

impl SubjectParts {
    fn into_subject(self) -> Result<QualifiedSubject, filament_observation::ReferenceRefusal> {
        let kind = SubjectKind::new(self.subject_kind.into_bytes())?;
        let identity = SubjectIdentity::new(self.identity)?;
        Ok(QualifiedSubject::from_authority(
            self.authority,
            kind,
            identity,
        ))
    }
}

impl RecordParts {
    fn into_record(self) -> Result<AdmittedRecord, filament_observation::ReferenceRefusal> {
        Ok(AdmittedRecord {
            identity: self.identity,
            binding_identity: self.binding_identity,
            source_identity: self.source_identity,
            schema_identity: self.schema_identity,
            subject: self.subject.into_subject()?,
            signal_identity: self.signal_identity,
            trigger_identity: self.trigger_identity,
            unit: self.unit,
            value: self.value,
            visibility: self.visibility,
            anchor: self.anchor,
            event_time_nanos: self.event_time_nanos,
            ingestion_time_nanos: self.ingestion_time_nanos,
            causal_relationship_identity: self.causal_relationship_identity,
            clock_identity: self.clock_identity,
            clock_revision: self.clock_revision,
            clock_uncertainty_nanos: self.clock_uncertainty_nanos,
        })
    }
}

fn read_extents(
    producer: &AdmittedStaticBundle,
    value: Option<&Value>,
    objects: &fsl_semantics::model::object_environment::ObjectEnvironment,
    mappings: &[RelationshipIdentityMapping],
) -> Result<(Vec<Relationship>, Vec<RelationshipExtent>), filament_observation::ReferenceRefusal> {
    let Some(value) = value else {
        return Ok((Vec::new(), Vec::new()));
    };
    let Some(extents) = value.as_array() else {
        return Ok((Vec::new(), Vec::new()));
    };
    let mut relationships = Vec::new();
    let mut relationship_extents = Vec::with_capacity(extents.len());
    for extent in extents {
        let Some(relationship) = extent.get("relationship").and_then(Value::as_str) else {
            continue;
        };
        let Some(declaration) = mappings
            .iter()
            .find(|mapping| mapping.relationship == relationship)
            .map(|mapping| mapping.declaration_identity.as_str())
        else {
            continue;
        };
        let complete = extent
            .get("complete")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        relationship_extents.push(RelationshipExtent::new(
            producer,
            declaration,
            if complete {
                RelationshipExtentState::Complete
            } else {
                RelationshipExtentState::Incomplete
            },
        )?);
        let Some(instances) = extent.get("instances").and_then(Value::as_array) else {
            continue;
        };
        for instance in instances {
            let Some(identity_text) = instance.get("identity").and_then(Value::as_str) else {
                continue;
            };
            let identity_bytes =
                decode_hex(identity_text).ok_or(filament_observation::ReferenceRefusal::Empty(
                    filament_observation::ReferenceComponent::RelationshipIdentity,
                ))?;
            let identity = filament_observation::RelationshipIdentity::new(identity_bytes)?;
            let source = read_snapshot_subject(instance.get("source"), producer, objects)?;
            let target = read_snapshot_subject(instance.get("target"), producer, objects)?;
            relationships.push(Relationship::new(
                producer,
                declaration,
                identity,
                source,
                target,
            )?);
        }
    }
    Ok((relationships, relationship_extents))
}

fn read_snapshot_subject(
    value: Option<&Value>,
    producer: &AdmittedStaticBundle,
    objects: &fsl_semantics::model::object_environment::ObjectEnvironment,
) -> Result<QualifiedSubject, filament_observation::ReferenceRefusal> {
    let population = value
        .and_then(|value| value.get("population"))
        .and_then(Value::as_str)
        .unwrap_or_default();
    let key = value
        .and_then(|value| value.get("key"))
        .and_then(Value::as_str)
        .unwrap_or_default();
    let reference = objects
        .population_members(population)
        .and_then(|members| {
            members
                .iter()
                .find(|reference| reference.object().as_str() == key)
        })
        .ok_or_else(
            || filament_observation::ReferenceRefusal::UnknownRelationshipDeclaration {
                identity: population.to_owned(),
            },
        )?;
    let type_identity = reference
        .object_type()
        .owner_chain()
        .last()
        .map(|declaration| declaration.node())
        .ok_or_else(
            || filament_observation::ReferenceRefusal::UnknownRelationshipDeclaration {
                identity: key.to_owned(),
            },
        )?;
    let kind = SubjectKind::new(type_identity.as_bytes().to_vec())?;
    let identity = SubjectIdentity::new(key.as_bytes().to_vec())?;
    Ok(QualifiedSubject::from_authority(
        producer.key().clone(),
        kind,
        identity,
    ))
}

fn read_record(value: &Value, path: &str) -> ReadResult<RecordParts> {
    let record = object(value, path)?;
    const FIELDS: &[&str] = &[
        "identity",
        "binding_identity",
        "source_identity",
        "schema_identity",
        "subject",
        "signal_identity",
        "trigger_identity",
        "unit",
        "value",
        "visibility",
        "anchor",
        "event_time_nanos",
        "ingestion_time_nanos",
        "causal_relationship_identity",
        "clock_identity",
        "clock_revision",
        "clock_uncertainty_nanos",
    ];
    closed(record, FIELDS, path)?;
    let value_path = format!("{path}/value");
    let state = object(member(record, "value", path)?, &value_path)?;
    match text(
        member(state, "state", &value_path)?,
        &format!("{value_path}/state"),
    )?
    .as_str()
    {
        "present" => {
            closed(
                state,
                &["state", "value_type", "canonical_value"],
                &value_path,
            )?;
        }
        "missing" => {
            closed(state, &["state"], &value_path)?;
        }
        _ => return Err(error(&format!("{value_path}/state"), "unknown value state")),
    }
    let value_state = if state["state"] == "missing" {
        ValueState::Missing
    } else {
        ValueState::Present {
            value_type: identity(
                member(state, "value_type", &value_path)?,
                &format!("{value_path}/value_type"),
            )?,
            canonical_value: text(
                member(state, "canonical_value", &value_path)?,
                &format!("{value_path}/canonical_value"),
            )?,
        }
    };
    let visibility = match text(
        member(record, "visibility", path)?,
        &format!("{path}/visibility"),
    )?
    .as_str()
    {
        "internal" => Visibility::Internal,
        "external" => Visibility::External,
        _ => return Err(error(&format!("{path}/visibility"), "unknown visibility")),
    };
    let causal = member(record, "causal_relationship_identity", path)?;
    let causal_relationship_identity = if causal.is_null() {
        None
    } else {
        let causal_text = text(causal, &format!("{path}/causal_relationship_identity"))?;
        let causal_bytes = decode_hex(&causal_text).ok_or_else(|| {
            error(
                &format!("{path}/causal_relationship_identity"),
                "expected lowercase hexadecimal bytes",
            )
        })?;
        Some(
            filament_observation::RelationshipIdentity::new(causal_bytes).map_err(|_| {
                error(
                    &format!("{path}/causal_relationship_identity"),
                    "expected non-empty identity bytes",
                )
            })?,
        )
    };
    Ok(RecordParts {
        identity: identity(
            member(record, "identity", path)?,
            &format!("{path}/identity"),
        )?,
        binding_identity: identity(
            member(record, "binding_identity", path)?,
            &format!("{path}/binding_identity"),
        )?,
        source_identity: identity(
            member(record, "source_identity", path)?,
            &format!("{path}/source_identity"),
        )?,
        schema_identity: identity(
            member(record, "schema_identity", path)?,
            &format!("{path}/schema_identity"),
        )?,
        subject: read_subject(member(record, "subject", path)?, &format!("{path}/subject"))?,
        signal_identity: identity(
            member(record, "signal_identity", path)?,
            &format!("{path}/signal_identity"),
        )?,
        trigger_identity: identity(
            member(record, "trigger_identity", path)?,
            &format!("{path}/trigger_identity"),
        )?,
        unit: identity(member(record, "unit", path)?, &format!("{path}/unit"))?,
        value: value_state,
        visibility,
        anchor: read_anchor(member(record, "anchor", path)?, &format!("{path}/anchor"))?,
        event_time_nanos: decimal(
            member(record, "event_time_nanos", path)?,
            &format!("{path}/event_time_nanos"),
        )?,
        ingestion_time_nanos: optional_decimal(
            member(record, "ingestion_time_nanos", path)?,
            &format!("{path}/ingestion_time_nanos"),
        )?,
        causal_relationship_identity,
        clock_identity: identity(
            member(record, "clock_identity", path)?,
            &format!("{path}/clock_identity"),
        )?,
        clock_revision: identity(
            member(record, "clock_revision", path)?,
            &format!("{path}/clock_revision"),
        )?,
        clock_uncertainty_nanos: unsigned_decimal(
            member(record, "clock_uncertainty_nanos", path)?,
            &format!("{path}/clock_uncertainty_nanos"),
        )?,
    })
}

fn read_subject(value: &Value, path: &str) -> ReadResult<SubjectParts> {
    let subject = object(value, path)?;
    closed(
        subject,
        &[
            "authority",
            "subject_kind",
            "authority_local_object_identity",
        ],
        path,
    )?;
    let authority_path = format!("{path}/authority");
    let authority_value = object(member(subject, "authority", path)?, &authority_path)?;
    closed(
        authority_value,
        &["bundle_identity", "bundle_revision", "digest"],
        &authority_path,
    )?;
    let revision_path = format!("{authority_path}/bundle_revision");
    let revision_value = object(
        member(authority_value, "bundle_revision", &authority_path)?,
        &revision_path,
    )?;
    closed(revision_value, &["namespace", "value"], &revision_path)?;
    let digest_path = format!("{authority_path}/digest");
    let digest_value = object(
        member(authority_value, "digest", &authority_path)?,
        &digest_path,
    )?;
    closed(digest_value, &["label", "value"], &digest_path)?;
    let digest_text = text(
        member(digest_value, "value", &digest_path)?,
        &format!("{digest_path}/value"),
    )?;
    let authority = AdmittedBundleKey {
        bundle_identity: text(
            member(authority_value, "bundle_identity", &authority_path)?,
            &format!("{authority_path}/bundle_identity"),
        )?,
        bundle_revision: Revision {
            namespace: text(
                member(revision_value, "namespace", &revision_path)?,
                &format!("{revision_path}/namespace"),
            )?,
            value: text(
                member(revision_value, "value", &revision_path)?,
                &format!("{revision_path}/value"),
            )?,
        },
        digest: ProducerDigest {
            label: text(
                member(digest_value, "label", &digest_path)?,
                &format!("{digest_path}/label"),
            )?,
            value: digest_text,
        },
    };
    let subject_kind = text(
        member(subject, "subject_kind", path)?,
        &format!("{path}/subject_kind"),
    )?;
    let identity_text = text(
        member(subject, "authority_local_object_identity", path)?,
        &format!("{path}/authority_local_object_identity"),
    )?;
    let identity = decode_hex(&identity_text).ok_or_else(|| {
        error(
            &format!("{path}/authority_local_object_identity"),
            "expected lowercase hexadecimal bytes",
        )
    })?;
    Ok(SubjectParts {
        authority,
        subject_kind,
        identity,
    })
}

fn read_anchor(value: &Value, path: &str) -> ReadResult<Anchor> {
    let object = object(value, path)?;
    let kind = text(member(object, "kind", path)?, &format!("{path}/kind"))?;
    match kind.as_str() {
        "event-position" => {
            closed(object, &["kind", "position"], path)?;
            Ok(Anchor::EventPosition(unsigned_decimal(
                member(object, "position", path)?,
                &format!("{path}/position"),
            )?))
        }
        "fixed-sample" => {
            closed(
                object,
                &["kind", "index", "epoch_nanos", "period_nanos"],
                path,
            )?;
            Ok(Anchor::FixedSample {
                index: unsigned_decimal(member(object, "index", path)?, &format!("{path}/index"))?,
                epoch_nanos: decimal(
                    member(object, "epoch_nanos", path)?,
                    &format!("{path}/epoch_nanos"),
                )?,
                period_nanos: unsigned_decimal(
                    member(object, "period_nanos", path)?,
                    &format!("{path}/period_nanos"),
                )?,
            })
        }
        "timestamp" => {
            closed(object, &["kind", "instant_nanos"], path)?;
            Ok(Anchor::TimestampNanos(decimal(
                member(object, "instant_nanos", path)?,
                &format!("{path}/instant_nanos"),
            )?))
        }
        _ => Err(error(&format!("{path}/kind"), "unknown anchor kind")),
    }
}

fn read_scope_kind(value: &Value, path: &str) -> ReadResult<ScopeKind> {
    let object = object(value, path)?;
    let kind = text(member(object, "kind", path)?, &format!("{path}/kind"))?;
    match kind.as_str() {
        "snapshot" => {
            closed(object, &["kind", "snapshot_identity"], path)?;
            Ok(ScopeKind::Snapshot {
                snapshot_identity: identity(
                    member(object, "snapshot_identity", path)?,
                    &format!("{path}/snapshot_identity"),
                )?,
            })
        }
        "window" => {
            closed(object, &["kind", "window_identity"], path)?;
            Ok(ScopeKind::Window {
                window_identity: identity(
                    member(object, "window_identity", path)?,
                    &format!("{path}/window_identity"),
                )?,
            })
        }
        _ => Err(error(&format!("{path}/kind"), "unknown scope kind")),
    }
}

fn read_range(value: &Value, path: &str) -> ReadResult<ClockRange> {
    let object = object(value, path)?;
    let family = text(
        member(object, "clock_family", path)?,
        &format!("{path}/clock_family"),
    )?;
    match family.as_str() {
        "event-position" => {
            closed(object, &["clock_family", "start", "end_exclusive"], path)?;
            Ok(ClockRange::EventPosition {
                start: unsigned_decimal(member(object, "start", path)?, &format!("{path}/start"))?,
                end_exclusive: unsigned_decimal(
                    member(object, "end_exclusive", path)?,
                    &format!("{path}/end_exclusive"),
                )?,
            })
        }
        "fixed-sample" => {
            closed(
                object,
                &[
                    "clock_family",
                    "start",
                    "end_exclusive",
                    "epoch_nanos",
                    "period_nanos",
                ],
                path,
            )?;
            Ok(ClockRange::FixedSample {
                start: unsigned_decimal(member(object, "start", path)?, &format!("{path}/start"))?,
                end_exclusive: unsigned_decimal(
                    member(object, "end_exclusive", path)?,
                    &format!("{path}/end_exclusive"),
                )?,
                epoch_nanos: decimal(
                    member(object, "epoch_nanos", path)?,
                    &format!("{path}/epoch_nanos"),
                )?,
                period_nanos: unsigned_decimal(
                    member(object, "period_nanos", path)?,
                    &format!("{path}/period_nanos"),
                )?,
            })
        }
        "timestamped-event" => {
            closed(object, &["clock_family", "start_nanos", "end_nanos"], path)?;
            Ok(ClockRange::Timestamp {
                start_nanos: decimal(
                    member(object, "start_nanos", path)?,
                    &format!("{path}/start_nanos"),
                )?,
                end_nanos: decimal(
                    member(object, "end_nanos", path)?,
                    &format!("{path}/end_nanos"),
                )?,
            })
        }
        _ => Err(error(
            &format!("{path}/clock_family"),
            "unknown clock family",
        )),
    }
}

fn object<'a>(value: &'a Value, path: &str) -> ReadResult<&'a serde_json::Map<String, Value>> {
    value
        .as_object()
        .ok_or_else(|| error(path, "expected object"))
}
fn array<'a>(value: &'a Value, path: &str) -> ReadResult<&'a [Value]> {
    value
        .as_array()
        .map(Vec::as_slice)
        .ok_or_else(|| error(path, "expected array"))
}
fn member<'a>(
    object: &'a serde_json::Map<String, Value>,
    name: &str,
    path: &str,
) -> ReadResult<&'a Value> {
    object
        .get(name)
        .ok_or_else(|| error(&format!("{path}/{name}"), "missing member"))
}
fn closed(object: &serde_json::Map<String, Value>, names: &[&str], path: &str) -> ReadResult<()> {
    if let Some(name) = names.iter().find(|name| !object.contains_key(**name)) {
        return Err(error(&format!("{path}/{name}"), "missing member"));
    }
    if let Some((name, _)) = object
        .iter()
        .find(|(name, _)| !names.contains(&name.as_str()))
    {
        return Err(error(&format!("{path}/{name}"), "unknown member"));
    }
    Ok(())
}
fn text(value: &Value, path: &str) -> ReadResult<String> {
    value
        .as_str()
        .map(ToOwned::to_owned)
        .ok_or_else(|| error(path, "expected string"))
}
fn identity(value: &Value, path: &str) -> ReadResult<Identity> {
    Ok(Identity::new(text(value, path)?))
}
fn boolean(value: &Value, path: &str) -> ReadResult<bool> {
    value
        .as_bool()
        .ok_or_else(|| error(path, "expected Boolean"))
}
fn identity_array(value: &Value, path: &str) -> ReadResult<Vec<Identity>> {
    array(value, path)?
        .iter()
        .enumerate()
        .map(|(index, value)| identity(value, &format!("{path}/{index}")))
        .collect()
}
fn optional_identity(value: &Value, path: &str) -> ReadResult<Option<Identity>> {
    if value.is_null() {
        Ok(None)
    } else {
        identity(value, path).map(Some)
    }
}
fn optional_digest(value: &Value, path: &str) -> ReadResult<Option<Digest>> {
    if value.is_null() {
        Ok(None)
    } else {
        digest(value, path, false).map(Some)
    }
}
fn digest(value: &Value, path: &str, prefix: bool) -> ReadResult<Digest> {
    let value = text(value, path)?;
    parse_digest(&value, prefix)
        .map(Digest::new)
        .ok_or_else(|| error(path, "expected lowercase SHA-256 digest"))
}
fn parse_digest(value: &str, prefix: bool) -> Option<[u8; 32]> {
    let value = if prefix {
        value.strip_prefix("sha256:")?
    } else {
        value
    };
    if value.len() != 64
        || !value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return None;
    }
    let decoded = decode_hex(value)?;
    decoded.try_into().ok()
}
fn decode_hex(value: &str) -> Option<Vec<u8>> {
    if value.is_empty()
        || value.len() % 2 != 0
        || !value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return None;
    }
    value
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| Some((nibble(pair[0])? << 4) | nibble(pair[1])?))
        .collect()
}
fn nibble(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        _ => None,
    }
}
fn decimal(value: &Value, path: &str) -> ReadResult<i128> {
    let value = text(value, path)?;
    if !valid_decimal(&value, true) {
        return Err(error(path, "expected canonical signed decimal string"));
    }
    value
        .parse()
        .map_err(|_| error(path, "decimal is outside i128 range"))
}
fn unsigned_decimal(value: &Value, path: &str) -> ReadResult<u64> {
    let value = text(value, path)?;
    if !valid_decimal(&value, false) {
        return Err(error(path, "expected canonical unsigned decimal string"));
    }
    value
        .parse()
        .map_err(|_| error(path, "decimal is outside u64 range"))
}
fn optional_decimal(value: &Value, path: &str) -> ReadResult<Option<i128>> {
    if value.is_null() {
        Ok(None)
    } else {
        decimal(value, path).map(Some)
    }
}
fn limit(value: &Value, path: &str) -> ReadResult<usize> {
    let value = unsigned_decimal(value, path)?;
    Ok(usize::try_from(value).unwrap_or(usize::MAX))
}
fn valid_decimal(value: &str, signed: bool) -> bool {
    let digits = if signed {
        value.strip_prefix('-').unwrap_or(value)
    } else {
        value
    };
    !digits.is_empty()
        && (digits == "0"
            || (digits.as_bytes()[0] != b'0' && digits.bytes().all(|byte| byte.is_ascii_digit())))
        && (!signed || value != "-0")
}
fn error(path: &str, reason: impl Into<String>) -> ReadError {
    ReadError {
        path: path.to_owned(),
        reason: reason.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        invalid_snapshot_relationship_identity, read_extents, unknown_snapshot_relationship,
        RelationshipIdentityMapping, RelationshipSelection,
    };
    use filament_exact::{
        EffectiveId, NodeKey, ObjectId, ObjectReference, OriginalDeclarationKey, PathSegment,
        UniverseId,
    };
    use filament_observation::{
        ProducerConfiguration, ProducerDigest, RelationshipDeclaration,
        RelationshipEndpointDeclaration, Revision, StaticProducerBundle,
        PRODUCER_INTERFACE_VERSION,
    };
    use fsl_semantics::model::object_environment::ObjectEnvironment;
    use ix_trace_rs::trace;
    use serde_json::json;
    use serde_json::Value;
    use std::collections::{BTreeMap, BTreeSet};

    const ORDER: &str = "ix://test/workflow/Order";
    const SHIPMENT: &str = "ix://test/workflow/Shipment";

    fn producer(target: &str) -> filament_observation::AdmittedStaticBundle {
        let mut candidate = StaticProducerBundle {
            bundle_identity: Some("ix://test/workflow/producer".to_owned()),
            bundle_revision: Some(Revision {
                namespace: "test-revision".to_owned(),
                value: "1".to_owned(),
            }),
            interface_version: Some(PRODUCER_INTERFACE_VERSION.to_owned()),
            configuration: Some(ProducerConfiguration {
                configuration_identity: "ix://test/workflow/configuration".to_owned(),
                digest: ProducerDigest {
                    label: "filament-canonical-json-1/1".to_owned(),
                    value: format!("sha256:{}", "00".repeat(32)),
                },
            }),
            relationships: Some(vec![RelationshipDeclaration {
                relationship_identity: "producer:Order-ships-Shipment".to_owned(),
                source: RelationshipEndpointDeclaration {
                    type_identity: ORDER.to_owned(),
                },
                target: RelationshipEndpointDeclaration {
                    type_identity: target.to_owned(),
                },
            }]),
            ..StaticProducerBundle::default()
        };
        candidate.digest = Some(
            candidate
                .canonical_digest_selection()
                .expect("producer candidate has a canonical digest"),
        );
        candidate.admit().expect("producer bundle admits")
    }

    fn producer_wire() -> Value {
        let mut bundle = StaticProducerBundle {
            bundle_identity: Some("ix://test/workflow/producer".to_owned()),
            bundle_revision: Some(Revision {
                namespace: "test-revision".to_owned(),
                value: "1".to_owned(),
            }),
            interface_version: Some(PRODUCER_INTERFACE_VERSION.to_owned()),
            configuration: Some(ProducerConfiguration {
                configuration_identity: "ix://test/workflow/configuration".to_owned(),
                digest: ProducerDigest {
                    label: "filament-canonical-json-1/1".to_owned(),
                    value: format!("sha256:{}", "00".repeat(32)),
                },
            }),
            relationships: Some(vec![RelationshipDeclaration {
                relationship_identity: "producer:Order-ships-Shipment".to_owned(),
                source: RelationshipEndpointDeclaration {
                    type_identity: ORDER.to_owned(),
                },
                target: RelationshipEndpointDeclaration {
                    type_identity: SHIPMENT.to_owned(),
                },
            }]),
            ..StaticProducerBundle::default()
        };
        let digest = bundle
            .canonical_digest_selection()
            .expect("producer digest");
        bundle.digest = Some(digest.clone());
        let mut value = serde_json::to_value(bundle).expect("producer JSON");
        let wire_digest = |text: &str| json!({"algorithm":"sha256", "domain":"filament-canonical-json-1", "value":text, "version":"1"});
        value["digest"] = wire_digest(&digest.value);
        value["configuration"]["digest"] = wire_digest(&format!("sha256:{}", "00".repeat(32)));
        value
    }

    fn decoding_selection() -> Value {
        let producer = producer_wire();
        let authority = json!({
            "bundle_identity":"ix://test/workflow/producer",
            "bundle_revision":{"namespace":"test-revision", "value":"1"},
            "digest":{"label":"filament-canonical-json-1/1", "value":producer["digest"]["value"]}
        });
        let mut selection = json!({
            "producer":producer,
            "binding":{"identity":"binding", "source_identity":"source", "schema_identity":"schema", "signal_identity":"signal", "trigger_identity":"trigger", "unit":"unit", "subject_kind":"kind-π", "required":false},
            "expected_subject":{"authority":authority, "subject_kind":"expected", "authority_local_object_identity":"00ff"},
            "required_relationships":[{
                "declaration_identity":"producer:Order-ships-Shipment",
                "source":{"authority":authority, "subject_kind":ORDER, "authority_local_object_identity":"6f31"},
                "target":{"authority":authority, "subject_kind":SHIPMENT, "authority_local_object_identity":"7331"}
            }],
            "relationship_identities":[],
            "scope":{
                "population_identity":"population", "membership_rule_identity":"rule", "membership_digest":"11".repeat(32), "membership_document":"exact bytes π",
                "required_member_identities":["m2", "m1"], "observation_sources":["source-b", "source-a"], "completeness_dependencies":["complete-b", "complete-a"], "progress_dependencies":["progress-b", "progress-a"],
                "clock_identity":"clock", "clock_revision":"revision", "membership_complete":false, "closure_identity":null, "closure_digest":null,
                "kind":{"kind":"window", "window_identity":"window"},
                "range":{"clock_family":"timestamped-event", "start_nanos":"-20", "end_nanos":"20"},
                "members":[
                    {"object_identity":"o1", "record_identity":"r1", "anchor":{"kind":"event-position", "position":"2"}},
                    {"object_identity":"o2", "record_identity":"r2", "anchor":{"kind":"fixed-sample", "index":"3", "epoch_nanos":"-7", "period_nanos":"5"}},
                    {"object_identity":"o3", "record_identity":"r3", "anchor":{"kind":"timestamp", "instant_nanos":"-11"}}
                ]
            },
            "records":[{
                "identity":"record", "binding_identity":"binding", "source_identity":"source", "schema_identity":"schema", "subject":{"authority":authority, "subject_kind":"record-kind", "authority_local_object_identity":"a0b1"},
                "signal_identity":"signal", "trigger_identity":"trigger", "unit":"unit", "value":{"state":"missing"}, "visibility":"external", "anchor":{"kind":"timestamp", "instant_nanos":"-13"},
                "event_time_nanos":"-17", "ingestion_time_nanos":"-5", "causal_relationship_identity":"00ff", "clock_identity":"clock", "clock_revision":"revision", "clock_uncertainty_nanos":"18446744073709551615"
            }],
            "limits":{"max_records":"18446744073709551615", "max_members":"2", "max_relationships":"3", "max_required_relationships":"4"}
        });
        let missing = selection["records"][0].clone();
        let mut event = missing.clone();
        event["identity"] = json!("record-event");
        event["value"] =
            json!({"state":"present", "value_type":"temperature", "canonical_value":"21.5"});
        event["visibility"] = json!("internal");
        event["anchor"] = json!({"kind":"event-position", "position":"4"});
        event["ingestion_time_nanos"] = Value::Null;
        event["causal_relationship_identity"] = Value::Null;
        event["clock_uncertainty_nanos"] = json!("18446744073709551615");

        let mut fixed = missing.clone();
        fixed["identity"] = json!("record-fixed");
        fixed["anchor"] =
            json!({"kind":"fixed-sample", "index":"8", "epoch_nanos":"-3", "period_nanos":"2"});
        fixed["causal_relationship_identity"] = json!("00ff");

        let mut timestamp = missing;
        timestamp["identity"] = json!("record-timestamp");
        timestamp["value"] =
            json!({"state":"present", "value_type":"temperature", "canonical_value":"22"});
        timestamp["anchor"] = json!({"kind":"timestamp", "instant_nanos":"13"});
        timestamp["ingestion_time_nanos"] = json!("-5");
        timestamp["causal_relationship_identity"] = json!("a0b1");
        selection["records"] = json!([event, fixed, timestamp]);
        selection
    }

    fn object_reference(type_identity: &str, key: &str) -> ObjectReference {
        let node = type_identity
            .rsplit('/')
            .next()
            .expect("type identity has a leaf");
        let universe = NodeKey::new("test/workflow", vec![PathSegment::Text(node.to_owned())], 0)
            .expect("model node key");
        ObjectReference::new(
            UniverseId::new(universe),
            EffectiveId::new(
                None,
                OriginalDeclarationKey::new("test/workflow", type_identity),
            ),
            ObjectId::new(key).expect("object key"),
        )
    }

    fn objects() -> ObjectEnvironment {
        ObjectEnvironment::default().with_population_members(BTreeMap::from([
            ("orders".to_owned(), vec![object_reference(ORDER, "o1")]),
            (
                "shipments".to_owned(),
                vec![object_reference(SHIPMENT, "s1")],
            ),
        ]))
    }

    fn objects_with_second_shipment() -> ObjectEnvironment {
        ObjectEnvironment::default().with_population_members(BTreeMap::from([
            ("orders".to_owned(), vec![object_reference(ORDER, "o1")]),
            (
                "shipments".to_owned(),
                vec![
                    object_reference(SHIPMENT, "s1"),
                    object_reference(SHIPMENT, "s2"),
                ],
            ),
        ]))
    }

    fn valid_relationship_request(
        extents: Option<&Value>,
        objects: &ObjectEnvironment,
    ) -> filament_observation::AdmissionRequest {
        let mut selection = decoding_selection();
        selection["records"] = json!([]);
        selection["scope"]["required_member_identities"] = json!([]);
        selection["scope"]["observation_sources"] = json!([]);
        selection["scope"]["completeness_dependencies"] = json!([]);
        selection["scope"]["progress_dependencies"] = json!([]);
        selection["scope"]["members"] = json!([]);
        selection["scope"]["membership_complete"] = json!(true);
        selection["scope"]["closure_identity"] = json!("closure");
        selection["scope"]["closure_digest"] = json!("22".repeat(32));
        selection["relationship_identities"] = json!([{
            "relationship":"local:ships",
            "declaration_identity":"producer:Order-ships-Shipment"
        }]);
        let selection =
            super::RelationshipSelection::read(&selection).expect("relationship request decodes");
        let mut request = selection
            .into_admission_request(
                super::package_selection("package-x".to_owned(), [7; 32]),
                extents,
                objects,
            )
            .expect("typed request constructors accept selected relationship");
        filament_observation::authority::population::assign_request_identities(
            &mut request,
            filament_observation::authority::Limits::owner_max(),
        )
        .expect("membership and population owner identities derive");
        request
    }

    fn decoded_admission_request() -> filament_observation::AdmissionRequest {
        super::RelationshipSelection::read(&decoding_selection())
            .expect("closed forms decode")
            .into_admission_request(
                super::package_selection("package-x".to_owned(), [7; 32]),
                None,
                &ObjectEnvironment::default(),
            )
            .expect("typed request constructors accept non-empty references")
    }

    fn candidate(identity: &str, target: &str) -> Value {
        json!({
            "identity":identity,
            "source":{"population":"orders", "key":"o1"},
            "target":{"population":"shipments", "key":target}
        })
    }

    fn extent(complete: bool, instances: Vec<Value>) -> Value {
        json!([{
            "relationship":"local:ships",
            "complete":complete,
            "instances":instances
        }])
    }

    #[trace("FR-1045-AC-8", "FR-1045-AC-9", "FR-1045-AC-10")]
    #[test]
    fn wire_members_decode_into_the_exact_typed_admission_request() {
        let request = decoded_admission_request();

        assert_eq!(request.binding.identity.as_str(), "binding");
        assert_eq!(request.binding.source_identity.as_str(), "source");
        assert_eq!(request.binding.schema_identity.as_str(), "schema");
        assert_eq!(request.binding.signal_identity.as_str(), "signal");
        assert_eq!(request.binding.trigger_identity.as_str(), "trigger");
        assert_eq!(request.binding.unit.as_str(), "unit");
        assert_eq!(request.binding.subject_kind.as_bytes(), "kind-π".as_bytes());
        assert!(!request.binding.required);
        assert_eq!(request.expected_subject.identity().as_bytes(), [0, 255]);
        assert_eq!(request.expected_subject.kind().as_bytes(), b"expected");
        assert_eq!(request.expected_subject.authority(), request.producer.key());
        assert_eq!(request.package.identity.as_str(), "package-x");
        assert_eq!(
            request.package.revision.as_str(),
            "filament.checked-package/v2"
        );
        assert_eq!(request.package.digest.as_bytes(), &[7; 32]);
        assert_eq!(request.required_relationships.len(), 1);
        assert_eq!(
            request.required_relationships[0].declaration_identity(),
            "producer:Order-ships-Shipment"
        );
        assert_eq!(
            request.required_relationships[0].source().kind().as_bytes(),
            ORDER.as_bytes()
        );
        assert_eq!(
            request.required_relationships[0]
                .source()
                .identity()
                .as_bytes(),
            b"o1"
        );
        assert_eq!(
            request.required_relationships[0]
                .target()
                .identity()
                .as_bytes(),
            b"s1"
        );

        assert_eq!(
            request.scope.membership_document.as_slice(),
            "exact bytes π".as_bytes()
        );
        assert_eq!(request.scope.membership_digest.as_bytes(), &[0x11; 32]);
        assert_eq!(request.scope.population_identity.as_str(), "population");
        assert_eq!(request.scope.membership_rule_identity.as_str(), "rule");
        assert_eq!(request.scope.required_member_identities[0].as_str(), "m2");
        assert_eq!(request.scope.required_member_identities[1].as_str(), "m1");
        assert_eq!(request.scope.observation_sources[0].as_str(), "source-b");
        assert_eq!(request.scope.observation_sources[1].as_str(), "source-a");
        assert_eq!(
            request.scope.completeness_dependencies[0].as_str(),
            "complete-b"
        );
        assert_eq!(
            request.scope.completeness_dependencies[1].as_str(),
            "complete-a"
        );
        assert_eq!(
            request.scope.progress_dependencies[0].as_str(),
            "progress-b"
        );
        assert_eq!(
            request.scope.progress_dependencies[1].as_str(),
            "progress-a"
        );
        assert_eq!(request.scope.clock_identity.as_str(), "clock");
        assert_eq!(request.scope.clock_revision.as_str(), "revision");
        assert!(!request.scope.membership_complete);
        assert_eq!(request.scope.closure_identity, None);
        assert_eq!(request.scope.closure_digest, None);
        assert_eq!(
            request.scope.kind,
            filament_observation::ScopeKind::Window {
                window_identity: filament_observation::Identity::new("window")
            }
        );
        assert_eq!(
            request.scope.range,
            filament_observation::ClockRange::Timestamp {
                start_nanos: -20,
                end_nanos: 20,
            }
        );
        assert_eq!(request.scope.members.len(), 3);
        assert_eq!(request.scope.members[0].object_identity.as_str(), "o1");
        assert_eq!(request.scope.members[0].record_identity.as_str(), "r1");
        assert_eq!(
            request.scope.members[0].anchor,
            filament_observation::Anchor::EventPosition(2)
        );
        assert_eq!(
            request.scope.members[1].anchor,
            filament_observation::Anchor::FixedSample {
                index: 3,
                epoch_nanos: -7,
                period_nanos: 5
            }
        );
        assert_eq!(
            request.scope.members[2].anchor,
            filament_observation::Anchor::TimestampNanos(-11)
        );

        assert_eq!(request.records.len(), 3);
        let event = &request.records[0];
        assert_eq!(event.identity.as_str(), "record-event");
        assert_eq!(event.binding_identity.as_str(), "binding");
        assert_eq!(event.source_identity.as_str(), "source");
        assert_eq!(event.schema_identity.as_str(), "schema");
        assert_eq!(event.subject.kind().as_bytes(), b"record-kind");
        assert_eq!(event.subject.identity().as_bytes(), [0xa0, 0xb1]);
        assert_eq!(event.subject.authority(), request.producer.key());
        assert_eq!(event.signal_identity.as_str(), "signal");
        assert_eq!(event.trigger_identity.as_str(), "trigger");
        assert_eq!(event.unit.as_str(), "unit");
        assert_eq!(
            event.value,
            filament_observation::ValueState::Present {
                value_type: filament_observation::Identity::new("temperature"),
                canonical_value: "21.5".to_owned()
            }
        );
        assert_eq!(event.visibility, filament_observation::Visibility::Internal);
        assert_eq!(event.anchor, filament_observation::Anchor::EventPosition(4));
        assert_eq!(event.event_time_nanos, -17);
        assert_eq!(event.ingestion_time_nanos, None);
        assert_eq!(event.causal_relationship_identity, None);
        assert_eq!(event.clock_uncertainty_nanos, u64::MAX);

        let fixed = &request.records[1];
        assert_eq!(fixed.identity.as_str(), "record-fixed");
        assert_eq!(fixed.value, filament_observation::ValueState::Missing);
        assert_eq!(fixed.visibility, filament_observation::Visibility::External);
        assert_eq!(
            fixed.anchor,
            filament_observation::Anchor::FixedSample {
                index: 8,
                epoch_nanos: -3,
                period_nanos: 2
            }
        );
        assert_eq!(fixed.event_time_nanos, -17);
        assert_eq!(fixed.ingestion_time_nanos, Some(-5));
        assert_eq!(
            fixed
                .causal_relationship_identity
                .as_ref()
                .unwrap()
                .as_bytes(),
            [0, 255]
        );
        let timestamp = &request.records[2];
        assert_eq!(
            timestamp.anchor,
            filament_observation::Anchor::TimestampNanos(13)
        );
        assert_eq!(
            timestamp
                .causal_relationship_identity
                .as_ref()
                .unwrap()
                .as_bytes(),
            [160, 177]
        );
        assert_eq!(request.limits.max_records, usize::MAX);
        assert_eq!(request.limits.max_members, 2);
        assert_eq!(request.limits.max_relationships, 3);
        assert_eq!(request.limits.max_required_relationships, 4);
    }

    #[trace("FR-1045-AC-2", "FR-1045-AC-3", "FR-1045-AC-4")]
    #[test]
    fn relationship_extent_states_and_conflict_precedence_reach_resolver() {
        let no_extents = valid_relationship_request(None, &objects());
        assert!(matches!(
            filament_observation::admit(no_extents),
            filament_observation::AdmissionOutcome::Incomplete { reasons }
                if matches!(reasons.as_slice(), [filament_observation::IncompleteReason::MissingRelationship { .. }])
        ));

        let complete_empty = extent(true, Vec::new());
        let request = valid_relationship_request(Some(&complete_empty), &objects());
        assert!(matches!(
            filament_observation::admit(request),
            filament_observation::AdmissionOutcome::Incomplete { reasons }
                if matches!(reasons.as_slice(), [filament_observation::IncompleteReason::MissingRelationship { .. }])
        ));

        let incomplete_with_match = extent(false, vec![candidate("01", "s1")]);
        let request = valid_relationship_request(Some(&incomplete_with_match), &objects());
        assert!(matches!(
            filament_observation::admit(request),
            filament_observation::AdmissionOutcome::Incomplete { reasons }
                if matches!(reasons.as_slice(), [filament_observation::IncompleteReason::IncompleteRelationshipExtent { .. }])
        ));

        let exact_replay = extent(true, vec![candidate("01", "s1"), candidate("01", "s1")]);
        let request = valid_relationship_request(Some(&exact_replay), &objects());
        assert!(matches!(
            filament_observation::admit(request),
            filament_observation::AdmissionOutcome::Available { .. }
        ));

        let ambiguous = extent(true, vec![candidate("01", "s1"), candidate("02", "s1")]);
        let request = valid_relationship_request(Some(&ambiguous), &objects());
        assert!(matches!(
            filament_observation::admit(request),
            filament_observation::AdmissionOutcome::Refused {
                cause: filament_observation::RefusalCause::AmbiguousRelationship { .. }
            }
        ));

        let conflict = extent(true, vec![candidate("01", "s1"), candidate("02", "s2")]);
        let request = valid_relationship_request(Some(&conflict), &objects_with_second_shipment());
        assert!(matches!(
            filament_observation::admit(request),
            filament_observation::AdmissionOutcome::Refused {
                cause: filament_observation::RefusalCause::RelationshipConflict { .. }
            }
        ));

        let contradicted = extent(true, vec![candidate("01", "s1"), candidate("01", "s2")]);
        let request =
            valid_relationship_request(Some(&contradicted), &objects_with_second_shipment());
        assert!(matches!(
            filament_observation::admit(request),
            filament_observation::AdmissionOutcome::Refused {
                cause: filament_observation::RefusalCause::RelationshipIdentityContradiction { .. }
            }
        ));
    }

    #[trace("FR-1045-AC-2")]
    #[test]
    fn relationship_identity_defines_exact_replay() {
        let replayed = extent(true, vec![candidate("00ff", "s1"), candidate("00ff", "s1")]);
        let request = valid_relationship_request(Some(&replayed), &objects());
        assert!(matches!(
            filament_observation::admit(request),
            filament_observation::AdmissionOutcome::Available { .. }
        ));
    }

    #[trace("FR-1045-AC-4")]
    #[test]
    fn required_relationship_limit_counts_raw_entries() {
        let matching = extent(true, vec![candidate("01", "s1")]);
        let mut duplicated = valid_relationship_request(Some(&matching), &objects());
        let repeated = duplicated.required_relationships[0].clone();
        duplicated.required_relationships.push(repeated.clone());
        duplicated.limits.max_required_relationships = 1;
        assert!(matches!(
            filament_observation::admit(duplicated),
            filament_observation::AdmissionOutcome::Refused {
                cause: filament_observation::RefusalCause::ResourceLimit {
                    limit: filament_observation::ResourceKind::RequiredRelationships
                }
            }
        ));

        let mut deduplicated = valid_relationship_request(Some(&matching), &objects());
        let repeated = deduplicated.required_relationships[0].clone();
        deduplicated.required_relationships.push(repeated);
        deduplicated.limits.max_required_relationships = 2;
        assert!(matches!(
            filament_observation::admit(deduplicated),
            filament_observation::AdmissionOutcome::Available { .. }
        ));

        let mut no_slots = valid_relationship_request(Some(&matching), &objects());
        no_slots.required_relationships.clear();
        assert!(matches!(
            filament_observation::admit(no_slots),
            filament_observation::AdmissionOutcome::Available { .. }
        ));
    }

    #[trace("FR-1045-AC-4")]
    #[test]
    fn required_slots_are_a_set_and_missing_reasons_use_canonical_order() {
        let mut duplicate = valid_relationship_request(None, &objects());
        let repeated = duplicate.required_relationships[0].clone();
        duplicate.required_relationships.push(repeated);
        let outcome = filament_observation::admit(duplicate);
        assert!(matches!(
            outcome,
            filament_observation::AdmissionOutcome::Incomplete { reasons }
                if matches!(reasons.as_slice(), [filament_observation::IncompleteReason::MissingRelationship { .. }])
        ));
    }

    #[trace("FR-1045-AC-4")]
    #[test]
    fn candidate_relationships_do_not_infer_required_slots() {
        let matching = extent(true, vec![candidate("01", "s1")]);
        let mut request = valid_relationship_request(Some(&matching), &objects());
        request.required_relationships.clear();
        assert!(matches!(
            filament_observation::admit(request),
            filament_observation::AdmissionOutcome::Available { .. }
        ));
    }

    #[trace("FR-1045-AC-12")]
    #[test]
    fn required_slot_without_mapping_is_missing_relationship() {
        let mut selection = decoding_selection();
        selection["records"] = json!([]);
        selection["scope"]["required_member_identities"] = json!([]);
        selection["scope"]["observation_sources"] = json!([]);
        selection["scope"]["completeness_dependencies"] = json!([]);
        selection["scope"]["progress_dependencies"] = json!([]);
        selection["scope"]["members"] = json!([]);
        selection["scope"]["membership_complete"] = json!(true);
        selection["scope"]["closure_identity"] = json!("closure");
        selection["scope"]["closure_digest"] = json!("22".repeat(32));
        selection["relationship_identities"] = json!([]);
        let mut request = super::RelationshipSelection::read(&selection)
            .expect("explicit required slot without a mapping parses")
            .into_admission_request(
                super::package_selection("package-x".to_owned(), [7; 32]),
                None,
                &ObjectEnvironment::default(),
            )
            .expect("required slot constructor is independent of candidate mapping");
        filament_observation::authority::population::assign_request_identities(
            &mut request,
            filament_observation::authority::Limits::owner_max(),
        )
        .expect("membership/population identities derive");
        assert!(matches!(
            filament_observation::admit(request),
            filament_observation::AdmissionOutcome::Incomplete { reasons }
                if matches!(reasons.as_slice(), [filament_observation::IncompleteReason::MissingRelationship { .. }])
        ));
    }

    #[trace("FR-1045-AC-4", "FR-1045-AC-5")]
    #[test]
    fn slot_refusal_discards_collected_incomplete_reasons() {
        let conflict = extent(true, vec![candidate("02", "s2")]);
        let mut request =
            valid_relationship_request(Some(&conflict), &objects_with_second_shipment());
        request.binding.subject_kind = request.expected_subject.kind().clone();
        request.binding.required = true;
        request.scope.range = filament_observation::ClockRange::Timestamp {
            start_nanos: 0,
            end_nanos: 1,
        };
        let anchor = filament_observation::Anchor::EventPosition(0);
        let mut record = filament_observation::AdmittedRecord {
            identity: filament_observation::Identity::new("pending-record-identity"),
            binding_identity: request.binding.identity.clone(),
            source_identity: request.binding.source_identity.clone(),
            schema_identity: request.binding.schema_identity.clone(),
            subject: request.expected_subject.clone(),
            signal_identity: request.binding.signal_identity.clone(),
            trigger_identity: request.binding.trigger_identity.clone(),
            unit: request.binding.unit.clone(),
            value: filament_observation::ValueState::Missing,
            visibility: filament_observation::Visibility::External,
            anchor,
            event_time_nanos: 0,
            ingestion_time_nanos: None,
            causal_relationship_identity: None,
            clock_identity: request.scope.clock_identity.clone(),
            clock_revision: request.scope.clock_revision.clone(),
            clock_uncertainty_nanos: 0,
        };
        let limits = filament_observation::authority::Limits::owner_max();
        record.identity =
            filament_observation::authority::observation::record_identity(&record, limits)
                .expect("record identity derives");
        request.scope.required_member_identities =
            vec![filament_observation::Identity::new("member")];
        request.scope.members = vec![filament_observation::Member {
            object_identity: filament_observation::Identity::new("member"),
            record_identity: record.identity.clone(),
            anchor,
        }];
        request.records = vec![record];
        filament_observation::authority::population::assign_request_identities(
            &mut request,
            limits,
        )
        .expect("population identities derive");
        assert!(matches!(
            filament_observation::admit(request),
            filament_observation::AdmissionOutcome::Refused {
                cause: filament_observation::RefusalCause::RelationshipConflict { .. }
            }
        ));
    }

    #[trace("FR-1045-AC-3", "FR-1045-AC-4")]
    #[test]
    fn missing_relationship_reasons_use_canonical_order() {
        let mut request = valid_relationship_request(None, &objects());
        let first = request.required_relationships[0].clone();
        let source = first.source().clone();
        let target = filament_observation::QualifiedSubject::from_authority(
            request.producer.key().clone(),
            filament_observation::SubjectKind::new(SHIPMENT.as_bytes().to_vec()).unwrap(),
            filament_observation::SubjectIdentity::new(b"s2".to_vec()).unwrap(),
        );
        let second = filament_observation::RequiredRelationship::new(
            &request.producer,
            first.declaration_identity(),
            source,
            target,
        )
        .expect("second relationship slot has the declared endpoint kinds");
        request.required_relationships.push(second);
        let outcome = filament_observation::admit(request);
        let filament_observation::AdmissionOutcome::Incomplete { reasons } = outcome else {
            panic!("both slots are missing: {outcome:?}");
        };
        assert_eq!(reasons.len(), 2);
        let target_keys = reasons
            .iter()
            .map(|reason| match reason {
                filament_observation::IncompleteReason::MissingRelationship { required } => {
                    required.target().identity().as_bytes().to_vec()
                }
                other => panic!("unexpected incomplete reason: {other:?}"),
            })
            .collect::<Vec<_>>();
        assert_eq!(target_keys, [b"s1".to_vec(), b"s2".to_vec()]);
    }

    #[trace("FR-1045-AC-9")]
    #[test]
    fn scope_members_decode_into_scope_selection() {
        let cases = [
            (
                json!({"clock_family":"event-position", "start":"1", "end_exclusive":"2"}),
                filament_observation::ClockRange::EventPosition {
                    start: 1,
                    end_exclusive: 2,
                },
            ),
            (
                json!({"clock_family":"timestamped-event", "start_nanos":"-4", "end_nanos":"9"}),
                filament_observation::ClockRange::Timestamp {
                    start_nanos: -4,
                    end_nanos: 9,
                },
            ),
        ];
        for (range, expected) in cases {
            let mut selection = decoding_selection();
            selection["scope"]["range"] = range;
            let parsed =
                super::RelationshipSelection::read(&selection).expect("scope range form decodes");
            assert_eq!(parsed.scope.range, expected);
        }

        let mut selection = decoding_selection();
        selection["scope"]["kind"] = json!({"kind":"snapshot", "snapshot_identity":"snap"});
        let parsed =
            super::RelationshipSelection::read(&selection).expect("snapshot scope kind decodes");
        assert_eq!(
            parsed.scope.kind,
            filament_observation::ScopeKind::Snapshot {
                snapshot_identity: filament_observation::Identity::new("snap")
            }
        );
    }

    #[trace("FR-1045-AC-8")]
    #[test]
    fn record_members_decode_into_admitted_records() {
        let request = decoded_admission_request();
        assert_eq!(request.records.len(), 3);
        assert_eq!(request.records[0].identity.as_str(), "record-event");
        assert_eq!(request.records[0].subject.kind().as_bytes(), b"record-kind");
        assert_eq!(
            request.records[0].subject.identity().as_bytes(),
            [0xa0, 0xb1]
        );
        assert_eq!(
            request.records[0].value,
            filament_observation::ValueState::Present {
                value_type: filament_observation::Identity::new("temperature"),
                canonical_value: "21.5".to_owned()
            }
        );
        assert_eq!(
            request.records[0].anchor,
            filament_observation::Anchor::EventPosition(4)
        );
        assert_eq!(request.records[0].ingestion_time_nanos, None);
        assert_eq!(
            request.records[1].value,
            filament_observation::ValueState::Missing
        );
        assert_eq!(
            request.records[1].anchor,
            filament_observation::Anchor::FixedSample {
                index: 8,
                epoch_nanos: -3,
                period_nanos: 2
            }
        );
        assert_eq!(request.records[1].ingestion_time_nanos, Some(-5));
        assert_eq!(
            request.records[1]
                .causal_relationship_identity
                .as_ref()
                .unwrap()
                .as_bytes(),
            [0, 255]
        );
        assert_eq!(
            request.records[2].anchor,
            filament_observation::Anchor::TimestampNanos(13)
        );
        assert_eq!(
            request.records[2]
                .causal_relationship_identity
                .as_ref()
                .unwrap()
                .as_bytes(),
            [160, 177]
        );
        assert_eq!(request.records[0].clock_uncertainty_nanos, u64::MAX);
    }

    #[trace("FR-1045-AC-10")]
    #[test]
    fn subject_binding_and_limit_members_decode_exactly() {
        let request = decoded_admission_request();
        assert_eq!(request.binding.identity.as_str(), "binding");
        assert_eq!(request.binding.source_identity.as_str(), "source");
        assert_eq!(request.binding.schema_identity.as_str(), "schema");
        assert_eq!(request.binding.signal_identity.as_str(), "signal");
        assert_eq!(request.binding.trigger_identity.as_str(), "trigger");
        assert_eq!(request.binding.unit.as_str(), "unit");
        assert_eq!(request.binding.subject_kind.as_bytes(), "kind-π".as_bytes());
        assert!(!request.binding.required);
        assert_eq!(request.expected_subject.identity().as_bytes(), [0, 255]);
        assert_eq!(request.expected_subject.kind().as_bytes(), b"expected");
        assert_eq!(request.expected_subject.authority(), request.producer.key());
        assert_eq!(request.required_relationships.len(), 1);
        assert_eq!(
            request.required_relationships[0].declaration_identity(),
            "producer:Order-ships-Shipment"
        );
        assert_eq!(
            request.required_relationships[0]
                .source()
                .identity()
                .as_bytes(),
            b"o1"
        );
        assert_eq!(
            request.required_relationships[0]
                .target()
                .identity()
                .as_bytes(),
            b"s1"
        );
        assert_eq!(request.limits.max_records, usize::MAX);
        assert_eq!(request.limits.max_members, 2);
        assert_eq!(request.limits.max_relationships, 3);
        assert_eq!(request.limits.max_required_relationships, 4);
    }

    #[trace("FR-1045-AC-6", "FR-100-AC-6")]
    #[test]
    fn unknown_snapshot_relationship_names_its_exact_pointer_after_admission() {
        let extents = json!([
            {"relationship": "ix://model/relationship/known"},
            {"relationship": "ix://model/relationship/unknown"}
        ]);
        let navigation = BTreeSet::from(["ix://model/relationship/known".to_owned()]);
        assert_eq!(
            unknown_snapshot_relationship(Some(&extents), &navigation).as_deref(),
            Some("/call/snapshot/relationships/1/relationship")
        );
        assert_eq!(
            unknown_snapshot_relationship(
                Some(&json!([{"relationship": "ix://model/relationship/known"}])),
                &navigation,
            ),
            None
        );
    }

    #[trace("FR-1045-AC-2", "FR-106-AC-74")]
    #[test]
    fn mapped_candidate_identity_validation_is_strict_and_post_admission() {
        let mut selection_value = decoding_selection();
        selection_value["relationship_identities"] = json!([{
            "relationship": "local:ships",
            "declaration_identity": "producer:Order-ships-Shipment"
        }]);
        let selection = RelationshipSelection::read(&selection_value)
            .expect("relationship mapping has the declared producer relation");

        for identity in [
            Value::Null,
            json!(false),
            json!(""),
            json!("00FF"),
            json!("0"),
            json!("0g"),
        ] {
            let extents = json!([{
                "relationship": "local:ships",
                "complete": true,
                "instances": [{"identity": identity}]
            }]);
            assert_eq!(
                invalid_snapshot_relationship_identity(Some(&extents), &selection).as_deref(),
                Some("/call/snapshot/relationships/0/instances/0/identity")
            );
        }

        let extents = json!([
            {
                "relationship": "local:ships",
                "complete": true,
                "instances": [{"identity": "00ff"}]
            },
            {
                "relationship": "local:unmapped",
                "complete": false,
                "instances": [{"identity": "NOT-HEX"}]
            }
        ]);
        assert_eq!(
            invalid_snapshot_relationship_identity(Some(&extents), &selection),
            None
        );
    }

    #[trace("FR-1045-AC-2", "FR-1045-AC-12")]
    #[test]
    fn snapshot_relationship_identity_round_trips_exact_bytes() {
        let producer = producer(SHIPMENT);
        let extents = json!([
            {
                "relationship": "ix://test/workflow/relationship/Order-ships-Shipment",
                "complete": false,
                "instances": [{
                    "identity": "00ff",
                    "source": {"population": "orders", "key": "o1"},
                    "target": {"population": "shipments", "key": "s1"}
                }]
            },
            {
                "relationship": "ix://test/workflow/relationship/unmapped",
                "complete": false,
                "instances": [{"identity": "NOT-HEX"}]
            }
        ]);
        let mappings = vec![RelationshipIdentityMapping {
            relationship: "ix://test/workflow/relationship/Order-ships-Shipment".to_owned(),
            declaration_identity: "producer:Order-ships-Shipment".to_owned(),
        }];
        let (relationships, relationship_extents) =
            read_extents(&producer, Some(&extents), &objects(), &mappings)
                .expect("mapped candidates admit and unmapped entries are ignored");
        assert_eq!(relationships.len(), 1);
        assert_eq!(relationships[0].identity().as_bytes(), [0, 255]);
        assert_eq!(
            relationships[0].declaration_identity(),
            "producer:Order-ships-Shipment"
        );
        assert_eq!(relationship_extents.len(), 1);
    }

    #[trace("FR-1045-AC-14")]
    #[test]
    fn mapped_unknown_producer_declaration_is_left_to_the_resolver() {
        let producer = producer(SHIPMENT);
        let extents = json!([{
            "relationship": "local:ships",
            "complete": true,
            "instances": []
        }]);
        let mappings = vec![RelationshipIdentityMapping {
            relationship: "local:ships".to_owned(),
            declaration_identity: "producer:missing".to_owned(),
        }];
        let refusal = read_extents(&producer, Some(&extents), &objects(), &mappings)
            .expect_err("an undeclared Producer relation is a resolver refusal");
        assert!(matches!(
            refusal,
            filament_observation::ReferenceRefusal::UnknownRelationshipDeclaration { identity }
                if identity == "producer:missing"
        ));
    }

    #[trace("FR-1045-AC-14")]
    #[test]
    fn wrong_snapshot_endpoint_kind_is_left_to_the_resolver() {
        let producer = producer("ix://test/workflow/PaymentAttempt");
        let extents = json!([{
            "relationship": "local:ships",
            "complete": true,
            "instances": [{
                "identity": "01",
                "source": {"population": "orders", "key": "o1"},
                "target": {"population": "shipments", "key": "s1"}
            }]
        }]);
        let mappings = vec![RelationshipIdentityMapping {
            relationship: "local:ships".to_owned(),
            declaration_identity: "producer:Order-ships-Shipment".to_owned(),
        }];
        let refusal = read_extents(&producer, Some(&extents), &objects(), &mappings)
            .expect_err("the resolver compares snapshot type identities");
        assert!(matches!(
            refusal,
            filament_observation::ReferenceRefusal::WrongEndpointKind {
                endpoint: filament_observation::EndpointSide::Target
            }
        ));
    }
}

// Kept in this module for the public package-selection contract and used by
// the command side when it has the canonical emitted package bytes.
/// Build the package identity and digest passed to relationship admission.
///
/// `name` is the selected package identity; `digest` is the SHA-256 digest of
/// its canonical checked-package document.
pub fn package_selection(name: String, digest: [u8; 32]) -> PackageSelection {
    PackageSelection {
        format: NATIVE_LINKED_PACKAGE_FORMAT.to_owned(),
        identity: Identity::new(name),
        revision: Identity::new("filament.checked-package/v2"),
        digest: Digest::new(digest),
    }
}
