// SPDX-License-Identifier: AGPL-3.0-or-later
//! Native acquisition and replay with immutable original proving inputs.

use std::collections::{BTreeMap, BTreeSet};

use fsl_semantics::check::wire_id::WireGraph;
use serde::Deserialize;
use serde_json::{json, Value};

use super::{
    ConsumerLimits, ContextError, DocumentKind, ModelSubject, ProtocolSubject, ReplayContext,
    ReplayFamily, ReplayReconstructionInput, ReplayReconstructionWire, SourceDescriptor,
};
use crate::execute::{recompile_with_temporal_selections, Recompiled};
use crate::limits::CallerLimits;
use crate::request::{RecompilationInput, StageLimits};
use crate::{ReplayRefusal, ReplayRequestRefusal, ReplaySource, Witness};
mod protocol;

/// A context error or a preserved native request/replay refusal.
#[derive(Debug, thiserror::Error)]
pub enum NativeContextError {
    /// The owning package/catalog/reference converter refused admission.
    #[error(transparent)]
    Reference(#[from] super::ReferenceContextError),
    /// The consumer's actual restoration budget stopped before reconstruction.
    #[error(transparent)]
    Resource(#[from] super::RestoreStop),
    /// Genuine protocol transition or family metadata reconstruction refused.
    #[error(transparent)]
    TemporalReadback(Box<super::TemporalReadbackError>),
    /// Malformed or inconsistent original context.
    #[error(transparent)]
    Context(#[from] ContextError),
    /// Native request admission refusal.
    #[error(transparent)]
    Request(#[from] ReplayRequestRefusal),
    /// Native source/package or replay refusal.
    #[error(transparent)]
    Replay(Box<ReplayRefusal>),
    /// A fresh native result could not be serialized; no partial row exists.
    #[error(transparent)]
    Output(#[from] super::ContextOutputError),
}

impl From<super::TemporalReadbackError> for NativeContextError {
    fn from(cause: super::TemporalReadbackError) -> Self {
        Self::TemporalReadback(Box::new(cause))
    }
}

impl From<ReplayRefusal> for NativeContextError {
    fn from(refusal: ReplayRefusal) -> Self {
        Self::Replay(Box::new(refusal))
    }
}
impl From<serde_json::Error> for NativeContextError {
    fn from(error: serde_json::Error) -> Self {
        ContextError::Json(error).into()
    }
}

/// Genuine source/package reconstruction, before family readback and replay.
pub struct ReconstructedContext {
    context: ReplayContext,
    common: ReplayReconstructionInput,
    compiled: Recompiled,
    caller: CallerLimits,
    consumer: ConsumerLimits,
}

fn caller_limits(
    original: &StageLimits,
    consumer: &ConsumerLimits,
) -> Result<CallerLimits, NativeContextError> {
    let mut execution_stages = StageLimits::default();
    for (setting, bound) in original.iter() {
        execution_stages = execution_stages
            .with_named(setting.name(), bound)
            .map_err(|error| ContextError::invalid("/request/stage_limits", error.to_string()))?;
    }
    for (setting, bound) in crate::AccountingLimits(consumer.accounting()).bounds() {
        execution_stages = execution_stages
            .with_named(setting.name(), bound)
            .map_err(|error| {
                ContextError::invalid("/effective_limits/stage_limits", error.to_string())
            })?;
    }
    Ok(crate::execute::request_limits(
        &execution_stages,
        consumer.accounting(),
        consumer.replay(),
    )?)
}

impl ReconstructedContext {
    /// Admit original common inputs and recompile their actual source bytes.
    /// Consumer bounds constrain acquisition resources without replacing any
    /// proving member or changing an original witness envelope.
    pub fn prepare(
        context: ReplayContext,
        wire: ReplayReconstructionWire,
        consumer: ConsumerLimits,
    ) -> Result<Self, NativeContextError> {
        super::encode::authenticate_common(&context, &wire)?;
        context.authenticate_packet()?;
        let selections = match context.family {
            ReplayFamily::ModelTemporal => {
                serde_json::from_value::<ModelSubject>(context.subject.clone())?
                    .native()?
                    .1
            }
            ReplayFamily::ProtocolTemporal => {
                serde_json::from_value::<ProtocolSubject>(context.subject.clone())?
                    .clock_selections()?
            }
            ReplayFamily::Hybrid => Vec::new(),
        };
        let common = ReplayReconstructionInput::decode(wire, consumer.replay())?;
        let caller = caller_limits(common.stage_limits(), &consumer)?;
        let compiled = recompile_with_temporal_selections(&common, &caller, &selections)?;
        if compiled.emitted.package().bytes() != common.package().bytes() {
            return Err(ContextError::invalid(
                "/source_bytes",
                "actual source recompilation differs from the original emitted package",
            )
            .into());
        }
        let retained = compiled.checked.shared_library_packages();
        let mut supplied = BTreeSet::new();
        for (index, document) in context.documents.iter().enumerate() {
            if document.kind != DocumentKind::DependencyPackage {
                continue;
            }
            let selection = document.selection.as_deref().ok_or_else(|| {
                ContextError::invalid(
                    format!("/documents/{index}/selection"),
                    "dependency selection is missing",
                )
            })?;
            let identity = crate::LibraryName::new(selection).map_err(|_| {
                ContextError::invalid(
                    format!("/documents/{index}/selection"),
                    "invalid library identity",
                )
            })?;
            if !supplied.insert(identity.clone()) {
                return Err(ContextError::invalid(
                    format!("/documents/{index}"),
                    "duplicate dependency package",
                )
                .into());
            }
            if retained
                .get(&identity)
                .is_none_or(|package| package.bytes() != document.bytes)
            {
                return Err(ContextError::invalid(
                    format!("/documents/{index}/bytes"),
                    "dependency bytes differ from genuine recompiled selection",
                )
                .into());
            }
        }
        if retained.keys().any(|identity| !supplied.contains(identity)) {
            return Err(ContextError::invalid(
                "/documents",
                "missing selected dependency package bytes",
            )
            .into());
        }
        Ok(Self {
            context,
            common,
            compiled,
            caller,
            consumer,
        })
    }

    /// Replay a model packet independently on the freshly checked graph.
    pub fn replay_model(self) -> Result<crate::ModelTraceReplayResult, NativeContextError> {
        self.with_model_result(|result, _, _| Ok(result))
    }

    /// Replay a model packet and encode its fresh native outcome for the driver.
    pub fn replay_model_output(self) -> Result<super::NativeReplayOutput, NativeContextError> {
        self.with_model_result(|result, kind, wire| {
            Ok(super::NativeReplayOutput::temporal(
                ReplayFamily::ModelTemporal,
                &result,
                kind,
                wire,
            )?)
        })
    }

    fn with_model_result<T>(
        self,
        finish: impl FnOnce(
            crate::ModelTraceReplayResult,
            &crate::CounterexampleKind,
            &Value,
        ) -> Result<T, NativeContextError>,
    ) -> Result<T, NativeContextError> {
        let Self {
            context,
            common,
            compiled,
            caller,
            consumer,
        } = self;
        if context.family != ReplayFamily::ModelTemporal {
            return Err(ContextError::invalid("/family", "expected model-temporal").into());
        }
        let packet = context
            .envelope
            .as_ref()
            .ok_or_else(|| ContextError::invalid("/envelope", "model witness packet is missing"))?;
        let source: SourceDescriptor = serde_json::from_value(packet.source.clone())?;
        if source.kind != "witness" {
            return Err(ContextError::invalid(
                "/envelope/source/kind",
                "expected original witness",
            )
            .into());
        }
        let witness = Witness::parse(source.transcript).map_err(|error| {
            ContextError::invalid("/envelope/source/transcript", error.to_string())
        })?;
        let original = common.bind_source(ReplaySource::Witness(witness), context.certificate)?;
        let selected: ModelSubject = serde_json::from_value(context.subject)?;
        let over = selected.over();
        let (subject, _) = selected.native()?;
        let snapshots = original.byte_provision().snapshots();
        let simulation = fsl_eval::simulation::model::ModelSimulationRequest {
            input: fsl_eval::simulation::model::ModelCheckInput::source(compiled.checked.package()),
            packages: &compiled.packages,
            normalization: caller.spine.model,
            snapshots: &snapshots,
            initial: subject.initial.clone(),
            universes: subject.universes.clone(),
            observation: caller.observation,
            expansion: subject.expansion,
            meter: consumer.accounting(),
            emit: caller.spine.package.emit(),
        };
        let claim = packet.occurrence(&original.to_wire())?;
        let no_clause = !compiled
            .checked
            .package()
            .graph()
            .temporal_clauses()
            .iter()
            .any(|node| node.identity() == *claim.node() && node.claim() == claim.origin());
        let derived = no_clause
            && (context.payload["kind"] == "deadlock"
                || (context.payload["kind"]["type"] == "undefined-evaluation" && over.is_none()));
        let system = if derived {
            fsl_eval::simulation::model::ModelSystem::new_over_read_identities(simulation)
        } else {
            fsl_eval::simulation::model::ModelSystem::new(simulation)
        }
        .map_err(ReplayRefusal::Admission)?;
        let readback = crate::model_check::replay::model_readback::ModelReadback {
            compiled: &compiled,
            system: &system,
            subject: &subject,
            over: over.as_ref(),
            claim: &claim,
            supplied: &context.payload,
        };
        let mut meter = filament_exact::Meter::new(consumer.accounting());
        let mut budget = super::RestoreBudget::new(&mut meter);
        let cancel = filament_exact::Cancel::new();
        let restored = super::with_source_reference_codec(
            &system,
            compiled.emitted.emission(),
            &compiled.packages,
            caller.spine.imports,
            caller.spine.package.emit(),
            &mut budget,
            |codec, budget| Ok(readback.admit(codec, budget, &cancel)),
        );
        if let Some(stop) = budget.into_stop() {
            return Err(stop.into());
        }
        let payload = restored??;
        let envelope = packet.reconstruct(&original.to_wire(), payload, consumer.replay())?;
        let result = crate::model_check::replay::reconstruction::model(
            &compiled,
            &original,
            &system,
            &subject,
            &envelope,
            consumer.accounting(),
        )?;
        let wire_kind = context.payload.get("kind").ok_or_else(|| {
            ContextError::invalid("/payload/kind", "original counterexample kind is absent")
        })?;
        finish(result, &envelope.family_payload().kind, wire_kind)
    }

    /// Replay a hybrid trace and encode its actual native result and category.
    pub fn replay_hybrid_output(self) -> Result<super::NativeReplayOutput, NativeContextError> {
        Ok(super::NativeReplayOutput::hybrid(self.replay_hybrid()?)?)
    }

    /// Admit and replay a hybrid trace and its actual required certificate.
    pub fn replay_hybrid(self) -> Result<crate::ReplayResult, NativeContextError> {
        let Self {
            context,
            common,
            compiled,
            caller: _,
            consumer,
        } = self;
        if context.family != ReplayFamily::Hybrid {
            return Err(ContextError::invalid("/family", "expected hybrid").into());
        }
        let subject: HybridSubject = serde_json::from_value(context.subject)?;
        subject.admit_settings()?;
        if subject.proof_bounds != context.request.declared_domains {
            return Err(ContextError::invalid(
                "/subject/proof_bounds",
                "original substituted domains differ",
            )
            .into());
        }
        let graph = compiled.checked.package().graph();
        let wire = WireGraph::of_package_bytes(common.package().bytes())
            .map_err(|error| ContextError::invalid("/request/package", error.to_string()))?;
        let key = wire
            .key_of_value(&subject.claim.node)
            .map_err(|error| ContextError::invalid("/subject/claim/node", error))?;
        if wire
            .wire_value(&key)
            .map_err(|error| ContextError::invalid("/subject/claim/node", error.to_string()))?
            != subject.claim.node
            || subject.claim.occurrence.role != "claim"
        {
            return Err(ContextError::invalid(
                "/subject/claim",
                "claim is not a document-held claim occurrence",
            )
            .into());
        }
        let origin = filament_exact::Origin::new(
            filament_exact::Role::new(subject.claim.occurrence.role),
            subject.claim.occurrence.ordinal,
        );
        let mut sites = graph
            .hybrid_claim_sites()
            .iter()
            .enumerate()
            .filter(|(_, site)| site.identity == key && site.claim == origin);
        let (position, _) = sites.next().ok_or_else(|| {
            ContextError::invalid("/subject/claim", "claim is absent from admitted graph")
        })?;
        if sites.next().is_some() {
            return Err(ContextError::invalid("/subject/claim", "ambiguous checked claim").into());
        }
        let claim = graph.hybrid_claims().get(position).ok_or_else(|| {
            ContextError::invalid("/subject/claim", "checked claim position is absent")
        })?;
        let model = graph
            .hybrid_models()
            .get(claim.model.get())
            .ok_or_else(|| {
                ContextError::invalid("/subject/model", "checked model position is absent")
            })?;
        if model.name.as_str() != subject.model
            || context.request.obligation_identity.get("node_id") != Some(&subject.claim.node)
        {
            return Err(ContextError::invalid(
                "/subject/model",
                "model or original obligation differs from checked claim",
            )
            .into());
        }
        let payload = crate::proof_result::read_plugin_hybrid_counterexample_over(
            &json!({"counterexample": context.payload}),
            graph,
            claim,
            &wire,
        )
        .map_err(|error| ContextError::invalid("/payload", error.to_string()))?;
        crate::hybrid_replay::admit(model, &payload.trace).map_err(ReplayRefusal::HybridTrace)?;
        if let Some(certificate) = &context.certificate {
            crate::hybrid_replay::read_certificate(
                model,
                crate::hybrid_replay::Expect::Witness(&payload.trace),
                certificate,
            )
            .map_err(|error| ContextError::invalid("/certificate", error.to_string()))?;
        }
        let original = common.bind_source(
            ReplaySource::Hybrid(Box::new(payload.clone())),
            context.certificate,
        )?;
        Ok(crate::execute::replay_hybrid_with_meter(
            original,
            &compiled,
            &payload,
            Some(consumer.accounting()),
        )?)
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct HybridSubject {
    claim: Claim,
    model: String,
    proof_bounds: Vec<Value>,
    budget: Budget,
    options: BTreeMap<String, String>,
    #[serde(deserialize_with = "super::explicit_option")]
    deadline_ms: Option<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Claim {
    node: Value,
    occurrence: Occurrence,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Occurrence {
    role: String,
    ordinal: u64,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Budget {
    #[serde(deserialize_with = "super::explicit_option")]
    max_solver_time: Option<String>,
    #[serde(deserialize_with = "super::explicit_option")]
    max_solver_steps: Option<String>,
}

impl HybridSubject {
    fn admit_settings(&self) -> Result<(), ContextError> {
        let count = |value: &Option<String>, at| {
            value
                .as_ref()
                .map(|value| super::unsigned(value, at))
                .transpose()
        };
        let time = count(
            &self.budget.max_solver_time,
            "/subject/budget/max_solver_time",
        )?
        .map(|seconds| {
            filament_exact::Rational::from_integer(filament_exact::Integer::from(seconds))
        });
        crate::SolverBudget::from_request(
            time.as_ref(),
            count(
                &self.budget.max_solver_steps,
                "/subject/budget/max_solver_steps",
            )?,
        )
        .map_err(|error| ContextError::invalid("/subject/budget", format!("{error:?}")))?;
        count(&self.deadline_ms, "/subject/deadline_ms")?;
        let _options = &self.options;
        Ok(())
    }
}
