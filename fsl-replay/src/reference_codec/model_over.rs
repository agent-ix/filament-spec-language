// SPDX-License-Identifier: AGPL-3.0-or-later
//! Source-model over restoration through the original admitted universe.

use filament_exact::{Cancel, IntegerDecimalBudget};
use fsl_eval::model_subject::ModelInstance;
use serde_json::Value;

use super::{ReferenceCodec, ReferenceCodecError, ReferenceFailure, ReferenceRuntime, Scope};

impl ReferenceCodec<'_, '_, '_> {
    /// Restore a source model's original selected population/key through its
    /// admitted initial/universe context, then cross-check the canonical
    /// reference with this same model-bound codec. Function-call population
    /// binding admission remains a separate route.
    /// `initial_model_index` is the actual base model index retained by the
    /// native family, rather than an index inferred from a state key.
    ///
    /// # Errors
    /// Refuses missing/foreign selections, absent keys, incompatible populations
    /// and noncanonical references, preserving caller work and cancellation.
    pub fn restore_model_over_binding(
        &self,
        value: &Value,
        selected: Option<&ModelInstance>,
        initial_model_index: usize,
        budget: &mut dyn IntegerDecimalBudget,
        cancel: &Cancel,
        max_depth: usize,
    ) -> Result<ModelInstance, ReferenceCodecError> {
        let mut scope = Scope {
            budget,
            cancel,
            normalization_owner: None,
            path: String::new(),
            max_depth,
        };
        let selected = selected.ok_or_else(|| scope.fail(ReferenceFailure::MissingPopulation))?;
        let ReferenceRuntime::Model(system) = &self.runtime else {
            return Err(scope.fail(ReferenceFailure::ForeignCatalog));
        };
        let reference = self.restore_reference(value, &mut *scope.budget, cancel, max_depth)?;
        if !scope.equal_text(reference.object().as_str(), &selected.key)? {
            return Err(scope.fail(ReferenceFailure::WrongPopulation));
        }
        let admitted = system
            .admits_over_instance(
                selected,
                reference.object_type(),
                initial_model_index,
                &mut *scope.budget,
                cancel,
            )
            .map_err(|source| ReferenceCodecError::Key {
                source,
                pointer: scope.path.clone(),
            })?;
        if !admitted {
            return Err(scope.fail(ReferenceFailure::WrongPopulation));
        }
        let encoded = self.encode_reference(&reference, &mut *scope.budget, cancel, max_depth)?;
        if !scope.same_json(value, &encoded)? {
            return Err(scope.fail(ReferenceFailure::MalformedWire));
        }
        Ok(ModelInstance {
            population: filament_checked::key::DeclarationKey {
                package: scope.text(&selected.population.package)?,
                node: scope.text(&selected.population.node)?,
            },
            key: scope.text(&selected.key)?,
        })
    }
}
