// SPDX-License-Identifier: AGPL-3.0-or-later
//! Model-local snapshots for a subject of a multi-model relation.

use filament_checked::key::DeclarationKey;
use fsl_semantics::model::domain_package::DomainPackageRef;
use fsl_semantics::model::observation::DocumentProvision;
use fsl_semantics::model::state::StatePopulation;
use serde_json::Value;

pub(super) fn snapshots(
    provision: &DocumentProvision,
    identity: &str,
    selected: &[StatePopulation],
    admitted: &[DeclarationKey],
    models: &[DomainPackageRef],
) -> DocumentProvision {
    provision
        .iter()
        .map(|(reference, bytes)| {
            let scoped = model_part(bytes, identity, selected, admitted, models);
            (reference.clone(), scoped)
        })
        .collect()
}

#[ix_trace_rs::string_edge]
fn model_part(
    bytes: &[u8],
    identity: &str,
    selected: &[StatePopulation],
    admitted: &[DeclarationKey],
    models: &[DomainPackageRef],
) -> Vec<u8> {
    let Ok(mut document) = serde_json::from_slice::<Value>(bytes) else {
        return bytes.to_vec();
    };
    if let Some(Value::Array(populations)) = document.get_mut("populations") {
        populations.retain(|entry| {
            let Some(population) = entry.get("population").and_then(Value::as_str) else {
                return true;
            };
            // Only another selected model's declared population is excluded.
            // Unknown populations and malformed entries remain for admission
            // to refuse; sharing a snapshot never enlarges the subject.
            !admitted.iter().any(|key| key.node == population)
                || selected.iter().any(|held| held.key.node == population)
        });
    }
    if let Some(model) = document.get_mut("model").and_then(Value::as_object_mut) {
        if model
            .get("identity")
            .and_then(Value::as_str)
            .is_some_and(|header| models.iter().any(|model| model.identity == header))
        {
            model.insert("identity".to_owned(), Value::String(identity.to_owned()));
        }
    }
    serde_json::to_vec(&document).unwrap_or_else(|_| bytes.to_vec())
}
