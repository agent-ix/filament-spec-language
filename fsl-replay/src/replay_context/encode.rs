// SPDX-License-Identifier: AGPL-3.0-or-later
//! Encoders of the existing native request and witness packet members.

use std::collections::BTreeMap;

use filament_exact::ScalarLimits;
use fsl_foundation::bound::{DomainKey, FiniteBound};
use fsl_foundation::source::provenance::OccurrenceKey;
use fsl_package::ObligationIdentity;
use fsl_semantics::check::wire_id::WireGraph;
use serde_json::value::RawValue;
use serde_json::{json, Value};

use super::{
    ContextError, DocumentKind, EnvelopeDescriptor, ProvisionEntry, ReplayContext,
    RequestDescriptor, SourceBytes,
};
use crate::identity::DeclaredDomain;
use crate::request::ReplayRequestWire;
use crate::target::ReplayTargetMembers;
use crate::witness::{EnvelopeSource, FamilyPayload, WitnessEnvelope};

fn identity_error(pointer: &str, error: impl std::fmt::Display) -> ContextError {
    ContextError::invalid(pointer, error.to_string())
}

/// The original accounting object in its existing unsigned-string mapping.
pub fn accounting_wire(limits: ScalarLimits) -> BTreeMap<String, String> {
    crate::limits::AccountingLimits(limits)
        .bounds()
        .into_iter()
        .map(|(setting, bound)| (setting.counter().to_owned(), bound.to_string()))
        .collect()
}

pub(super) fn occurrence_wire(
    occurrence: &OccurrenceKey,
    graph: &WireGraph,
) -> Result<Value, ContextError> {
    Ok(json!({
        "node_id": graph.wire_value(occurrence.node()).map_err(|error| identity_error("/occurrence_key/node_id", error))?,
        "role": occurrence.origin().role().as_str(),
        "ordinal": occurrence.origin().ordinal().to_string(),
    }))
}

fn obligation_wire(
    identity: &ObligationIdentity,
    graph: &WireGraph,
) -> Result<Value, ContextError> {
    Ok(json!({
        "node_id": graph.wire_value(identity.node()).map_err(|error| identity_error("/obligation_identity/node_id", error))?,
        "model": identity.model().map(fsl_package::ModelIdentity::as_str),
        "ordinal": identity.ordinal(),
    }))
}

fn members_wire(members: &ReplayTargetMembers) -> Value {
    // The existing request mapping represents selected target members by
    // presence; an absent member is not serialized as a selected null value.
    #[derive(serde::Serialize)]
    struct Members {
        #[serde(skip_serializing_if = "Option::is_none")]
        function: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        function_owner: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        frame_operation: Option<Value>,
        #[serde(skip_serializing_if = "Option::is_none")]
        state_clause: Option<String>,
    }
    let selected = Members {
        function: members.function.as_ref().map(ToString::to_string),
        function_owner: members.function_owner.as_ref().map(ToString::to_string),
        frame_operation: members.frame_operation.as_ref().map(|operation| {
            json!({
                "object": operation.object.to_string(), "operation": operation.operation.as_str(),
            })
        }),
        state_clause: members.state_clause.as_ref().map(ToString::to_string),
    };
    // Every field is already a JSON value or string; no user-defined serializer
    // can fail here. Keep the conversion fallible at the public boundary below.
    json!(selected)
}

/// Encode an actual declared domain using the existing FR-336 member mapping.
pub fn declared_domain_wire(
    domain: &DeclaredDomain,
    graph: &WireGraph,
) -> Result<Value, ContextError> {
    let key = match domain.domain() {
        DomainKey::Node { node, path } => json!({
            "kind": "node", "node": graph.wire_value(node).map_err(|error| identity_error("/declared_domains/domain/node", error))?,
            "path": path.iter().map(ToString::to_string).collect::<Vec<_>>(),
        }),
        DomainKey::Population {
            member_type,
            ordinal,
        } => json!({
            "kind": "population", "member_type": graph.wire_value(member_type).map_err(|error| identity_error("/declared_domains/domain/member_type", error))?,
            "ordinal": ordinal.to_string(),
        }),
    };
    let bound = match domain.bound() {
        FiniteBound::Cardinality { maximum } => {
            json!({"kind": "cardinality", "maximum": maximum.to_string()})
        }
        FiniteBound::Depth { maximum } => json!({"kind": "depth", "maximum": maximum.to_string()}),
        FiniteBound::IntegerRange(interval) => {
            json!({"kind": "integer_range", "lower": interval.lower().to_string(), "upper": interval.upper().to_string()})
        }
        FiniteBound::Variants { members } => json!({"kind": "variants", "members": members}),
    };
    Ok(
        json!({"domain": key, "kind": domain.proof_bound().kind().map(|kind| json!({"kind": kind.to_wire()})), "bound": bound}),
    )
}

struct RequestMembers<'a> {
    package: &'a crate::ReplayPackage,
    profile_selections: &'a [crate::ProfileSelection],
    dependencies: &'a [crate::DependencyEntryWire],
    replay_target: &'a crate::RequestTargetWire,
    obligation_identity: &'a ObligationIdentity,
    backend: &'a str,
    state_environment: &'a crate::StateEnvironment,
    accounting_limits: ScalarLimits,
    stage_limits: &'a BTreeMap<String, u64>,
    supplied_limits: Option<filament_semantic_value::supplied_stop::SuppliedLimits>,
    object_environment: Option<&'a crate::ObjectEnvironmentRef>,
    declared_domains: &'a [DeclaredDomain],
    byte_provision: &'a [(String, String, Vec<u8>)],
}

macro_rules! borrowed_request {
    ($wire:ident) => {
        RequestMembers {
            package: &$wire.package,
            profile_selections: &$wire.profile_selections,
            dependencies: &$wire.dependencies,
            replay_target: &$wire.replay_target,
            obligation_identity: &$wire.obligation_identity,
            backend: &$wire.backend,
            state_environment: &$wire.state_environment,
            accounting_limits: $wire.accounting_limits,
            stage_limits: &$wire.stage_limits,
            supplied_limits: $wire.supplied_limits,
            object_environment: $wire.object_environment.as_ref(),
            declared_domains: &$wire.declared_domains,
            byte_provision: &$wire.byte_provision,
        }
    };
}
/// Retain the common request descriptor and every actual source byte.
pub fn request_wire(
    wire: &ReplayRequestWire,
) -> Result<(RequestDescriptor, Vec<SourceBytes>), ContextError> {
    encode_request(borrowed_request!(wire))
}

pub(super) fn authenticate_common(
    context: &ReplayContext,
    wire: &crate::request::ReplayReconstructionWire,
) -> Result<(), ContextError> {
    let (mut encoded, sources) = encode_request(borrowed_request!(wire))?;
    if encoded.byte_provision.len() != context.request.byte_provision.len() {
        return Err(ContextError::invalid(
            "/request/byte_provision",
            "common source provision differs from original context",
        ));
    }
    let mut recorded = BTreeMap::new();
    for source in &context.source_bytes {
        if recorded
            .insert(source.bytes_path.as_str(), source.bytes.as_slice())
            .is_some()
        {
            return Err(ContextError::invalid(
                "/source_bytes",
                "duplicate original source token",
            ));
        }
    }
    for ((actual, expected), bytes) in encoded
        .byte_provision
        .iter_mut()
        .zip(&context.request.byte_provision)
        .zip(&sources)
    {
        if actual.authority != expected.authority
            || actual.identity != expected.identity
            || recorded.get(expected.bytes_path.as_str()).copied() != Some(bytes.bytes.as_slice())
        {
            return Err(ContextError::invalid(
                "/request/byte_provision",
                "common source bytes or labels differ from original context",
            ));
        }
        actual.bytes_path.clone_from(&expected.bytes_path);
    }
    if serde_json::to_string(&encoded)? != serde_json::to_string(&context.request)? {
        return Err(ContextError::invalid(
            "/request",
            "common native input differs from immutable original descriptor",
        ));
    }
    let original_documents = context
        .documents
        .iter()
        .filter(|document| document.kind != DocumentKind::DependencyPackage)
        .map(|document| document.bytes.as_slice());
    if !original_documents.eq(wire.documents.iter().map(Vec::as_slice)) {
        return Err(ContextError::invalid(
            "/documents",
            "common native documents differ from original context",
        ));
    }
    Ok(())
}

fn encode_request(
    wire: RequestMembers<'_>,
) -> Result<(RequestDescriptor, Vec<SourceBytes>), ContextError> {
    let graph = WireGraph::of_package_bytes(wire.package.bytes())
        .map_err(|error| identity_error("/request/package", error))?;
    let package_text = std::str::from_utf8(wire.package.bytes())
        .map_err(|error| identity_error("/request/package", error))?;
    let package = RawValue::from_string(package_text.to_owned())?;
    let mut provisions = Vec::with_capacity(wire.byte_provision.len());
    let mut source_bytes = Vec::with_capacity(wire.byte_provision.len());
    for (index, (authority, identity, bytes)) in wire.byte_provision.iter().enumerate() {
        let token = format!("source-{index}");
        provisions.push(ProvisionEntry {
            authority: authority.clone(),
            identity: identity.clone(),
            bytes_path: token.clone(),
        });
        source_bytes.push(SourceBytes {
            bytes_path: token,
            bytes: bytes.clone(),
        });
    }
    #[derive(serde::Serialize)]
    struct Dependency<'a> {
        identity: &'a str,
        sources: Vec<Value>,
        #[serde(skip_serializing_if = "Option::is_none")]
        refinement_internals: &'a Option<Vec<filament_contract_model::CheckedRefinementInternal>>,
    }
    let dependencies = wire
        .dependencies
        .iter()
        .map(|dependency| Dependency {
            identity: &dependency.identity,
            sources: dependency
                .sources
                .iter()
                .map(|(authority, identity)| json!({"authority": authority, "identity": identity}))
                .collect(),
            refinement_internals: &dependency.refinement_internals,
        })
        .collect::<Vec<_>>();
    let target = &wire.replay_target;
    let target_value = json!({
        "members": members_wire(&target.members),
        "arguments": target.arguments,
        "occurrence_key": target.occurrence_key.as_ref().map(|key| occurrence_wire(key, &graph)).transpose()?,
        "over": target.over.as_ref().map(|over| json!({"population": over.population, "key": over.key})),
    });
    let supplied_limits = wire
        .supplied_limits
        .map(|limits| {
            json!({
                "admission_work_units": limits.admission_work_units().to_string(),
                "conversion_work_units": limits.conversion_work_units().to_string(),
                "numeric_magnitude_bits": limits.numeric_magnitude_bits().to_string(),
            })
        })
        .unwrap_or(Value::Null);
    Ok((
        RequestDescriptor {
            package,
            profile_selections: json!(wire
                .profile_selections
                .iter()
                .map(|profile| json!({"profile": profile.profile(), "value": profile.value()}))
                .collect::<Vec<_>>()),
            dependencies: json!(dependencies),
            replay_target: target_value,
            obligation_identity: obligation_wire(&wire.obligation_identity, &graph)?,
            backend: wire.backend.to_owned(),
            state_environment: json!(wire.state_environment.entries()),
            accounting_limits: accounting_wire(wire.accounting_limits),
            stage_limits: wire
                .stage_limits
                .iter()
                .map(|(name, bound)| (name.clone(), bound.to_string()))
                .collect(),
            supplied_limits,
            object_environment: wire
                .object_environment
                .map(crate::ObjectEnvironmentRef::to_value)
                .unwrap_or(Value::Null),
            declared_domains: wire
                .declared_domains
                .iter()
                .map(|domain| declared_domain_wire(domain, &graph))
                .collect::<Result<_, _>>()?,
            byte_provision: provisions,
        },
        source_bytes,
    ))
}

/// Serialize the actual packet members with the family's existing wire payload.
pub fn envelope_wire<P: FamilyPayload>(
    envelope: &WitnessEnvelope<P>,
    payload: Value,
    graph: &WireGraph,
) -> Result<EnvelopeDescriptor, ContextError> {
    let source = match envelope.source() {
        EnvelopeSource::Witness(witness) => {
            json!({"kind": "witness", "transcript": witness.transcript()})
        }
        EnvelopeSource::Input(_) => {
            return Err(ContextError::invalid(
                "/envelope/source",
                "temporal producer packet must retain its actual witness source",
            ))
        }
    };
    let target: ReplayTargetMembers = envelope.replay_target().clone().into();
    Ok(EnvelopeDescriptor {
        obligation_identity: obligation_wire(&envelope.obligation_identity(), graph)?,
        occurrence_key: occurrence_wire(envelope.occurrence_key(), graph)?,
        clause_node: graph
            .wire_value(envelope.clause_node())
            .map_err(|error| identity_error("/envelope/clause_node", error))?,
        replay_target: members_wire(&target),
        profile_selections: json!(envelope
            .profile_selections()
            .iter()
            .map(|profile| json!({"profile": profile.profile(), "value": profile.value()}))
            .collect::<Vec<_>>()),
        run_limits: accounting_wire(envelope.run_limits()),
        declared_domains: envelope
            .declared_domains()
            .iter()
            .map(|domain| declared_domain_wire(domain, graph))
            .collect::<Result<_, _>>()?,
        backend: envelope.backend().as_str().to_owned(),
        trace_position: envelope
            .trace_position()
            .map(|position| json!(position.as_str()))
            .unwrap_or(Value::Null),
        source,
        family_payload: payload,
    })
}
