// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-173 "Event-position form": a trace variable ranges over the executions
//! of its own model, so each alias of a clause over several selected models
//! gets a [`ModelSystem`] over that model alone, started from the part of the
//! request's initial states that model owns.
//!
//! A request's snapshot document spans the selected models: it holds the
//! populations of every one. A system over one model admits the populations of
//! that model only, so [`alias_provisions`] reads each document down to them
//! and [`alias_system`] builds the system over the result.

use fsl_eval::simulation::model::{ModelSimulationRequest, ModelSystem};
use fsl_semantics::check::CheckedHyperClause;
use fsl_semantics::model::observation::{AdmissionFailure, DocumentProvision};
use serde_json::Value;

/// The snapshot documents of each alias of `clause`, reduced to the
/// populations of the alias's model: `None` for an alias whose system is not
/// over one model (a unit that selects one model, or a package-route input).
#[must_use]
pub fn alias_provisions(
    simulation: &ModelSimulationRequest<'_>,
    clause: &CheckedHyperClause,
) -> Vec<Option<DocumentProvision>> {
    clause
        .aliases()
        .iter()
        .map(|alias| {
            let package = simulation.source_package()?;
            if simulation.input.model_selections.len() < 2 {
                return None;
            }
            let identity = package.graph().model_package_identity(alias.as_str())?;
            Some(
                simulation
                    .snapshots
                    .iter()
                    .map(|(reference, bytes)| (reference.clone(), model_part(bytes, identity)))
                    .collect(),
            )
        })
        .collect()
}

/// The document `bytes` with the populations of the package `identity` alone,
/// under that package's identity. A document that is no JSON snapshot is
/// returned as it is, for admission to refuse.
fn model_part(bytes: &[u8], identity: &str) -> Vec<u8> {
    let Ok(mut document) = serde_json::from_slice::<Value>(bytes) else {
        return bytes.to_vec();
    };
    let prefix = format!("ix://{identity}/");
    if let Some(Value::Array(populations)) = document.get_mut("populations") {
        populations.retain(|entry| {
            entry
                .get("population")
                .and_then(Value::as_str)
                .is_some_and(|population| population.starts_with(&prefix))
        });
    }
    if let Some(model) = document.get_mut("model").and_then(Value::as_object_mut) {
        model.insert("identity".to_owned(), Value::String(identity.to_owned()));
    }
    serde_json::to_vec(&document).unwrap_or_else(|_| bytes.to_vec())
}

/// The system of alias `alias`: over its own model, started from `provision`
/// (from [`alias_provisions`]) when the unit selects several.
///
/// # Errors
///
/// As [`ModelSystem::for_alias`].
pub fn alias_system<'p>(
    simulation: &ModelSimulationRequest<'p>,
    provision: Option<&'p DocumentProvision>,
    alias: &str,
) -> Result<ModelSystem<'p>, AdmissionFailure> {
    let mut request = simulation.clone();
    if let Some(provision) = provision {
        request.snapshots = provision;
    }
    ModelSystem::for_alias(request, alias)
}

/// Build a subject using a package clause's exact model-root mapping when present.
pub(super) fn alias_system_for_clause<'p>(
    simulation: &ModelSimulationRequest<'p>,
    provision: Option<&'p DocumentProvision>,
    clause: &CheckedHyperClause,
    alias: &str,
) -> Result<ModelSystem<'p>, AdmissionFailure> {
    let mut request = simulation.clone();
    if let Some(provision) = provision {
        request.snapshots = provision;
    }
    ModelSystem::for_hyper_alias(request, clause, alias)
}
