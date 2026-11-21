// SPDX-License-Identifier: AGPL-3.0-or-later
//! Owned selected model and protocol inputs, independent of display targets.

use std::num::NonZeroU64;

use filament_checked::key::DeclarationKey;
use fsl_eval::model_subject::{CapacityBinding, ModelInstance, RoleBinding};
use fsl_eval::simulation::model::{ExpansionLimits, PopulationUniverse, ProtocolDefinition};
use fsl_eval::simulation::protocol::{MemorySelection, ResolvedMemory};
use fsl_foundation::source::provenance::RawSourceRef;
use fsl_foundation::MemoryModelName;
use fsl_package::TemporalClockSelection;
use fsl_semantics::model::observation::DocumentRef;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use super::{unsigned, ContextError};
use crate::{ModelTraceSubject, ProtocolTraceSubject, TimedReplay};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Declaration {
    package: String,
    node: String,
}

impl Declaration {
    fn from_native(key: &DeclarationKey) -> Self {
        Self {
            package: key.package.clone(),
            node: key.node.clone(),
        }
    }
    fn native(&self) -> DeclarationKey {
        DeclarationKey {
            package: self.package.clone(),
            node: self.node.clone(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Document {
    authority: String,
    identity: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Universe {
    population: Declaration,
    keys: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Expansion {
    max_candidates: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Exploration {
    max_states: String,
    max_transitions: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Timed {
    model: String,
    exploration: Exploration,
    max_clocks: String,
    digital: bool,
}

/// The complete selected model subject in the FR-621 composition.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelSubject {
    initial: Vec<Document>,
    universes: Vec<Universe>,
    expansion: Expansion,
    #[serde(deserialize_with = "super::explicit_option")]
    timed: Option<Timed>,
    temporal_selections: Vec<ClockSelection>,
    #[serde(deserialize_with = "super::explicit_option")]
    over: Option<Instance>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ClockSelection {
    source: Document,
    declaration_span: Span,
    clock: Value,
    definition: Value,
    #[serde(deserialize_with = "super::explicit_option")]
    trace: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Span {
    start: usize,
    end: usize,
}

impl ClockSelection {
    fn from_native(selection: &TemporalClockSelection) -> Result<Self, ContextError> {
        Ok(Self {
            source: Document {
                authority: selection.source.authority().to_owned(),
                identity: selection.source.identity().to_owned(),
            },
            declaration_span: Span {
                start: selection.declaration_span.start,
                end: selection.declaration_span.end,
            },
            clock: serde_json::to_value(&selection.clock)?,
            definition: serde_json::to_value(&selection.definition)?,
            trace: selection.trace.as_deref().map(str::to_owned),
        })
    }

    fn native(&self) -> Result<TemporalClockSelection, ContextError> {
        let source = RawSourceRef::new(self.source.authority.clone(), self.source.identity.clone())
            .map_err(|error| {
                ContextError::invalid("/subject/temporal_selections/source", error.to_string())
            })?;
        if self.declaration_span.start > self.declaration_span.end {
            return Err(ContextError::invalid(
                "/subject/temporal_selections/declaration_span",
                "inverted source span",
            ));
        }
        let mut selected = TemporalClockSelection::new(
            source,
            fsl_foundation::Span {
                start: self.declaration_span.start,
                end: self.declaration_span.end,
            },
            serde_json::from_value(self.clock.clone())?,
        );
        if serde_json::to_value(&selected.definition)? != self.definition {
            return Err(ContextError::invalid(
                "/subject/temporal_selections/definition",
                "clock definition does not match the selected configuration",
            ));
        }
        selected.trace = self.trace.as_deref().map(Into::into);
        Ok(selected)
    }
}

impl ModelSubject {
    /// Capture exactly the model subject used by the producer's family replay.
    pub fn capture(
        subject: &ModelTraceSubject,
        selections: &[TemporalClockSelection],
        over: Option<&ModelInstance>,
    ) -> Result<Self, ContextError> {
        Ok(Self {
            over: over.map(|instance| Instance {
                population: Declaration::from_native(&instance.population),
                key: instance.key.clone(),
            }),
            initial: subject
                .initial
                .iter()
                .map(|reference| Document {
                    authority: reference.authority.clone(),
                    identity: reference.identity.clone(),
                })
                .collect(),
            universes: subject
                .universes
                .iter()
                .map(|universe| Universe {
                    population: Declaration::from_native(&universe.population),
                    keys: universe.keys.clone(),
                })
                .collect(),
            expansion: Expansion {
                max_candidates: subject.expansion.max_candidates.to_string(),
            },
            timed: subject.timed.as_ref().map(|timed| Timed {
                model: timed.model.clone(),
                exploration: Exploration {
                    max_states: timed.exploration.max_states.to_string(),
                    max_transitions: timed.exploration.max_transitions.to_string(),
                },
                max_clocks: timed.max_clocks.to_string(),
                digital: timed.digital,
            }),
            temporal_selections: selections
                .iter()
                .map(ClockSelection::from_native)
                .collect::<Result<_, _>>()?,
        })
    }

    /// The original producer-selected instance, never inferred from a reference.
    /// Its declaration and membership still require native model admission.
    pub fn over(&self) -> Option<ModelInstance> {
        self.over.as_ref().map(|instance| ModelInstance {
            population: instance.population.native(),
            key: instance.key.clone(),
        })
    }

    /// Restore the selected subject and admitted clock selections. Native
    /// model admission still validates declarations against genuine documents.
    pub fn native(&self) -> Result<(ModelTraceSubject, Vec<TemporalClockSelection>), ContextError> {
        let timed = self
            .timed
            .as_ref()
            .map(|timed| {
                let max_clocks =
                    NonZeroU64::new(unsigned(&timed.max_clocks, "/subject/timed/max_clocks")?)
                        .ok_or_else(|| {
                            ContextError::invalid("/subject/timed/max_clocks", "zero clock bound")
                        })?;
                Ok::<_, ContextError>(TimedReplay {
                    model: timed.model.clone(),
                    exploration: fsl_eval::simulation::Limits {
                        max_states: usize::try_from(unsigned(
                            &timed.exploration.max_states,
                            "/subject/timed/exploration/max_states",
                        )?)
                        .map_err(|_| {
                            ContextError::invalid(
                                "/subject/timed/exploration/max_states",
                                "count exceeds native usize",
                            )
                        })?,
                        max_transitions: usize::try_from(unsigned(
                            &timed.exploration.max_transitions,
                            "/subject/timed/exploration/max_transitions",
                        )?)
                        .map_err(|_| {
                            ContextError::invalid(
                                "/subject/timed/exploration/max_transitions",
                                "count exceeds native usize",
                            )
                        })?,
                    },
                    max_clocks,
                    digital: timed.digital,
                })
            })
            .transpose()?;
        for universe in &self.universes {
            let unique = universe
                .keys
                .iter()
                .collect::<std::collections::BTreeSet<_>>();
            if unique.len() != universe.keys.len() {
                return Err(ContextError::invalid(
                    "/subject/universes/keys",
                    "duplicate universe key",
                ));
            }
        }
        Ok((
            ModelTraceSubject {
                initial: self
                    .initial
                    .iter()
                    .map(|reference| DocumentRef {
                        authority: reference.authority.clone(),
                        identity: reference.identity.clone(),
                    })
                    .collect(),
                universes: self
                    .universes
                    .iter()
                    .map(|universe| PopulationUniverse {
                        population: universe.population.native(),
                        keys: universe.keys.clone(),
                    })
                    .collect(),
                expansion: ExpansionLimits {
                    max_candidates: unsigned(
                        &self.expansion.max_candidates,
                        "/subject/expansion/max_candidates",
                    )?,
                },
                timed,
            },
            self.temporal_selections
                .iter()
                .map(ClockSelection::native)
                .collect::<Result<_, _>>()?,
        ))
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Instance {
    population: Declaration,
    key: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Role {
    role: String,
    model: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Capacity {
    name: String,
    value: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Memory {
    parallel: String,
    model: String,
}

/// The complete protocol subject, including selected memory and capacities.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProtocolSubject {
    model: Value,
    protocol: String,
    over: Instance,
    roles: Vec<Role>,
    capacities: Vec<Capacity>,
    memory: Vec<Memory>,
    #[serde(deserialize_with = "super::explicit_option")]
    max_store_buffer: Option<String>,
    #[serde(deserialize_with = "super::explicit_option")]
    max_messages: Option<String>,
    #[serde(deserialize_with = "super::explicit_option")]
    max_live_instances: Option<String>,
    temporal_selections: Vec<ClockSelection>,
}

impl ProtocolSubject {
    /// Restore clock selections needed before the checked protocol is acquired.
    pub fn clock_selections(&self) -> Result<Vec<TemporalClockSelection>, ContextError> {
        self.temporal_selections
            .iter()
            .map(ClockSelection::native)
            .collect()
    }

    /// The selected protocol name, requiring native graph resolution.
    pub fn protocol_name(&self) -> &str {
        &self.protocol
    }
    /// Capture the actual selected protocol subject used for native replay.
    pub fn capture(
        subject: &ProtocolTraceSubject,
        selections: &[TemporalClockSelection],
    ) -> Result<Self, ContextError> {
        let model = ModelSubject::capture(&subject.model, &[], None)?;
        let mut model = serde_json::to_value(model)?;
        if let Some(object) = model.as_object_mut() {
            object.remove("temporal_selections");
            object.remove("over");
        }
        Ok(Self {
            model,
            protocol: subject.protocol.clone(),
            over: Instance {
                population: Declaration::from_native(&subject.over.population),
                key: subject.over.key.clone(),
            },
            roles: subject
                .roles
                .iter()
                .map(|role| Role {
                    role: role.role.clone(),
                    model: role.model.clone(),
                })
                .collect(),
            capacities: subject
                .capacities
                .iter()
                .map(|capacity| Capacity {
                    name: capacity.name.clone(),
                    value: capacity.value.to_string(),
                })
                .collect(),
            memory: subject
                .memory
                .parallels()
                .iter()
                .map(|entry| Memory {
                    parallel: entry.parallel.clone(),
                    model: entry.model.label().to_owned(),
                })
                .collect(),
            max_store_buffer: subject.max_store_buffer.map(|value| value.to_string()),
            max_messages: subject.max_messages.map(|value| value.to_string()),
            max_live_instances: subject.max_live_instances.map(|value| value.to_string()),
            temporal_selections: selections
                .iter()
                .map(ClockSelection::from_native)
                .collect::<Result<_, _>>()?,
        })
    }

    /// Restore a protocol subject against its actual checked definition.
    pub fn native(
        &self,
        protocol: &ProtocolDefinition<'_>,
    ) -> Result<(ProtocolTraceSubject, Vec<TemporalClockSelection>), ContextError> {
        let mut model = self.model.clone();
        let object = model
            .as_object_mut()
            .ok_or_else(|| ContextError::invalid("/subject/model", "expected model subject"))?;
        if object
            .insert("temporal_selections".to_owned(), json!([]))
            .is_some()
        {
            return Err(ContextError::invalid(
                "/subject/model/temporal_selections",
                "clock selections belong to protocol subject",
            ));
        }
        if object.insert("over".to_owned(), Value::Null).is_some() {
            return Err(ContextError::invalid(
                "/subject/model/over",
                "over belongs to protocol subject",
            ));
        }
        let (model, _) = serde_json::from_value::<ModelSubject>(model)?.native()?;
        let memory = self
            .memory
            .iter()
            .map(|entry| {
                let model = match entry.model.as_str() {
                    "sc" => MemoryModelName::Sc,
                    "tso" => MemoryModelName::Tso,
                    "ra" => MemoryModelName::Ra,
                    _ => {
                        return Err(ContextError::invalid(
                            "/subject/memory/model",
                            "unknown memory model",
                        ))
                    }
                };
                Ok(MemorySelection {
                    parallel: entry.parallel.clone(),
                    model,
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        let count = |value: &Option<String>, pointer| {
            value
                .as_ref()
                .map(|value| unsigned(value, pointer))
                .transpose()
        };
        Ok((
            ProtocolTraceSubject {
                model,
                protocol: self.protocol.clone(),
                over: ModelInstance {
                    population: self.over.population.native(),
                    key: self.over.key.clone(),
                },
                roles: self
                    .roles
                    .iter()
                    .map(|role| RoleBinding {
                        role: role.role.clone(),
                        model: role.model.clone(),
                    })
                    .collect(),
                capacities: self
                    .capacities
                    .iter()
                    .map(|capacity| {
                        Ok(CapacityBinding {
                            name: capacity.name.clone(),
                            value: unsigned(&capacity.value, "/subject/capacities/value")?,
                        })
                    })
                    .collect::<Result<_, ContextError>>()?,
                memory: ResolvedMemory::resolve_definition(protocol, &memory)
                    .map_err(|error| ContextError::invalid("/subject/memory", error.to_string()))?,
                max_store_buffer: count(&self.max_store_buffer, "/subject/max_store_buffer")?,
                max_messages: count(&self.max_messages, "/subject/max_messages")?,
                max_live_instances: count(&self.max_live_instances, "/subject/max_live_instances")?,
            },
            self.temporal_selections
                .iter()
                .map(ClockSelection::native)
                .collect::<Result<_, _>>()?,
        ))
    }
}
