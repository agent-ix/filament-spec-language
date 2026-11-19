// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-944-AC-14: activation uses an optional, instance-specific owner history.

use super::completeness::boundary_value;
use super::*;
use crate::temporal::evidence::{TriggerEvidence, TriggerRecord, TriggerScope};
use filament_observation::authority::{activation, capture, closure, completeness, OpenClosed};

pub(in crate::temporal::evidence) struct ActivationCapture<'a> {
    pub(in crate::temporal::evidence) name: &'a str,
    pub(in crate::temporal::evidence) declared_type: &'a str,
    pub(in crate::temporal::evidence) fact: Value,
}

pub(in crate::temporal::evidence) struct ActivationInstance<'a> {
    pub(in crate::temporal::evidence) identity: &'a SemanticTriggerIdentity,
    pub(in crate::temporal::evidence) captures: Vec<ActivationCapture<'a>>,
}

impl OwnerObservation {
    pub(super) fn activation_history(
        &self,
        anchor: &EvidencePosition,
        instance: Option<&ActivationInstance<'_>>,
    ) -> Result<(OwnerObservation, Vec<(Identity, Identity)>), CallFailure> {
        let mut request = self.request.clone();
        let mut identity: Value = serde_json::from_str(request.binding.identity.as_str())
            .map_err(|_| fault("trace-binding-identity-is-json"))?;
        let object = identity
            .as_object_mut()
            .ok_or_else(|| fault("trace-binding-identity-is-an-object"))?;
        object.insert("role".to_owned(), json!("activation"));
        if let Some(instance) = instance {
            object.insert(
                "instance".to_owned(),
                json!(hex(instance.identity.as_bytes())),
            );
        }
        request.binding.identity = Identity::new(canonical_text(&identity)?);
        request.binding.schema_identity = Identity::new("fsl-eval/activation/1");
        request.binding.signal_identity = request.binding.trigger_identity.clone();
        request.binding.required = false;
        let mut captures = Vec::new();
        request.records.clear();
        if let Some(instance) = instance {
            let record_identity = self
                .side_map
                .iter()
                .find(|(_, labels)| labels.contains(&anchor.observation.identity))
                .map(|(identity, _)| identity)
                .ok_or_else(|| fault("activation-anchor-has-trace-record"))?;
            let seed = self
                .request
                .records
                .iter()
                .find(|record| &record.identity == record_identity)
                .ok_or_else(|| fault("activation-anchor-record-is-admitted"))?;
            let key = hex(instance.identity.as_bytes());
            let mut values = vec![(
                Identity::new("fsl-eval/instance"),
                "fsl-eval/activation-trigger/1",
                json!({"instance": key}),
            )];
            for capture in &instance.captures {
                values.push((Identity::new(format!("fsl-eval/capture/{}", capture.name)), "fsl-eval/capture/1",
                    json!({"capture":capture.name, "instance":key, "type":capture.declared_type, "value":capture.fact})));
            }
            values.sort_by(|a, b| a.0.cmp(&b.0));
            for (capture_identity, value_type, value) in values {
                let mut record = seed.clone();
                record.binding_identity = request.binding.identity.clone();
                record.schema_identity = request.binding.schema_identity.clone();
                record.signal_identity = request.binding.signal_identity.clone();
                record.trigger_identity = request.binding.trigger_identity.clone();
                record.value = ValueState::Present {
                    value_type: Identity::new(value_type),
                    canonical_value: canonical_text(&value)?,
                };
                record.identity = observation::record_identity(&record, Limits::owner_max())
                    .map_err(|error| owner_fault(error.code().as_str(), error.detail()))?;
                captures.push((capture_identity, record.identity.clone()));
                request.records.push(record);
            }
        }
        let mut members = request
            .records
            .iter()
            .map(|record| Member {
                object_identity: record.identity.clone(),
                record_identity: record.identity.clone(),
                anchor: record.anchor,
            })
            .collect::<Vec<_>>();
        members.sort_by(|a, b| a.object_identity.cmp(&b.object_identity));
        let required = members
            .iter()
            .map(|member| member.object_identity.clone())
            .collect::<Vec<_>>();
        let membership = population::derive_membership(&required, Limits::owner_max())
            .map_err(|error| owner_fault(error.code().as_str(), error.detail()))?;
        request.scope.members = members;
        request.scope.required_member_identities = required;
        request.scope.membership_rule_identity = membership.identity().clone();
        request.scope.membership_digest = membership.digest();
        request.scope.membership_document = membership.bytes().to_vec();
        request.scope.closure_identity = Some(membership.identity().clone());
        request.scope.closure_digest = Some(membership.digest());
        request.scope.kind = ScopeKind::Snapshot {
            snapshot_identity: membership.identity().clone(),
        };
        request.scope.membership_complete = true;
        request.limits.max_records = request.records.len();
        request.limits.max_members = request.records.len();
        request.scope.population_identity =
            population::population_identity(&request, Limits::owner_max())
                .map_err(|error| owner_fault(error.code().as_str(), error.detail()))?;
        let retained = request.clone();
        let qualified = match admit(request) {
            AdmissionOutcome::Available { observation } => observation,
            AdmissionOutcome::Incomplete { reasons } => {
                return Err(owner_fault("incomplete", &format!("{reasons:?}")))
            }
            AdmissionOutcome::Refused { cause } => {
                return Err(owner_fault("refused", &format!("{cause:?}")))
            }
        };
        let subject = SubjectSelection {
            scope_identity: qualified.scope().membership_rule_identity.clone(),
            population_identity: qualified.scope().population_identity.clone(),
        };
        Ok((
            OwnerObservation {
                request: retained,
                qualified,
                subject,
                authority: self.authority.clone(),
                time_factor: self.time_factor.clone(),
                side_map: BTreeMap::new(),
                partial_facts: Vec::new(),
            },
            captures,
        ))
    }

    pub(in crate::temporal::evidence) fn activation_view(
        &self,
        binding: &AuthenticatedBinding,
        cover: ClockCover,
        anchor: &EvidencePosition,
        instance: Option<ActivationInstance<'_>>,
        scope: TriggerScope,
    ) -> Result<activation::View, CallFailure> {
        let (history, capture_records) = self.activation_history(anchor, instance.as_ref())?;
        let context = history.context();
        let limits = Limits::owner_max();
        let boundary = history.closure_boundary()?;
        let closure_selection = closure::Selection::new(
            history.request.binding.source_identity.clone(),
            history.request.scope.clock_revision.clone(),
            history.request.scope.observation_sources.clone(),
            boundary,
            match scope.closure {
                crate::temporal::ProgressClosure::Closed => OpenClosed::Closed,
                crate::temporal::ProgressClosure::Open => OpenClosed::Open,
            },
        );
        let closure_document = closure::derive(context, &closure_selection, limits)
            .map_err(|error| owner_fault(error.code().as_str(), error.detail()))?;
        let closure_view = closure::read(
            closure_document.bytes(),
            context,
            &closure_selection,
            limits,
        )
        .map_err(|error| owner_fault(error.code().as_str(), error.detail()))?;
        let evidence = if scope.evidence == TriggerEvidence::Admitted
            && scope.record == TriggerRecord::Complete
        {
            let selection = completeness::Selection::new(
                Identity::new(canonical_text(&boundary_value(boundary))?),
                boundary,
                history.completeness_facts(true),
            );
            let document = completeness::derive(context, &selection, limits)
                .map_err(|error| owner_fault(error.code().as_str(), error.detail()))?;
            Some(
                completeness::read(document.bytes(), context, &selection, limits)
                    .map_err(|error| owner_fault(error.code().as_str(), error.detail()))?,
            )
        } else {
            None
        };
        let interval = quarter_interval(binding, anchor.position, anchor, cover)?;
        let (activation_selection, proofs) = if instance.is_some() {
            let record = history
                .request
                .records
                .iter()
                .find(|record| {
                    matches!(&record.value, ValueState::Present { value_type, .. }
                    if value_type.as_str() == "fsl-eval/activation-trigger/1")
                })
                .ok_or_else(|| fault("activation-has-an-instance-record"))?;
            let selection = capture::Selection::new(
                history.request.binding.trigger_identity.clone(),
                record.anchor,
                capture_records
                    .iter()
                    .map(|(name, record)| {
                        capture::Binding::new(
                            name.clone(),
                            history.request.binding.identity.clone(),
                            record.clone(),
                        )
                    })
                    .collect(),
            );
            let document = capture::derive(context, &selection, limits)
                .map_err(|error| owner_fault(error.code().as_str(), error.detail()))?;
            let capture_view = capture::read(document.bytes(), context, &selection, limits)
                .map_err(|error| owner_fault(error.code().as_str(), error.detail()))?;
            let captures = capture_view
                .payload()
                .bindings()
                .map(|binding| {
                    activation::Capture::new(
                        Identity::new(binding.capture_identity),
                        Identity::new(binding.observation_identity),
                        Identity::new(binding.value_type),
                        binding.canonical_value.to_owned(),
                        history.request.binding.source_identity.clone(),
                    )
                })
                .collect();
            (
                activation::ActivationSelection::new(
                    self.request.binding.identity.clone(),
                    history.request.binding.identity.clone(),
                    history.request.binding.trigger_identity.clone(),
                    record.identity.clone(),
                    interval.clone(),
                    captures,
                ),
                activation::AuthorityProofs::new(
                    &capture_view,
                    None,
                    Some(&closure_view),
                    evidence.as_ref(),
                )
                .map_err(|error| owner_fault(error.code().as_str(), error.detail()))?,
            )
        } else {
            (
                activation::ActivationSelection::without_trigger(
                    self.request.binding.identity.clone(),
                    history.request.binding.identity.clone(),
                    history.request.binding.trigger_identity.clone(),
                    interval.clone(),
                ),
                activation::AuthorityProofs::without_capture(
                    &history.request.binding.identity,
                    None,
                    Some(&closure_view),
                    evidence.as_ref(),
                )
                .map_err(|error| owner_fault(error.code().as_str(), error.detail()))?,
            )
        };
        // The progress and lateness axes are not consumed by FSL's activation mapping.
        let selection = activation::Selection::new(
            activation_selection,
            activation::ProgressSelection::new(interval.clone(), interval.clone()),
            interval.clone(),
            interval,
            proofs,
        );
        let document = activation::derive(context, &selection, limits)
            .map_err(|error| owner_fault(error.code().as_str(), error.detail()))?;
        activation::read(document.bytes(), context, &selection, limits)
            .map_err(|error| owner_fault(error.code().as_str(), error.detail()))
    }
}
