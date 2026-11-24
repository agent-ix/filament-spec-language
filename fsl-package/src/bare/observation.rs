// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-533 "Observation admission": the bare route's admission of a current
//! snapshot, over the type environment the package's own reads give.

use std::collections::BTreeMap;

use filament_semantic_value::admission::{AdmissionRecord, ObservationAdmission};
use filament_semantic_value::catalog::CatalogCode;
use fsl_semantics::model::accounting::ModelNormalizationLimits;
use fsl_semantics::model::domain_package::DomainPackageRef;
use fsl_semantics::model::intake::PackageInput;
use fsl_semantics::model::observation::{
    admit_current_snapshot, AdmissionFailure, DocumentRef, Observation, ObservationLimits,
};

use super::BarePackage;
use crate::AdmittedProtocolObservation;

fn refused(catalog: CatalogCode, fields: BTreeMap<&'static str, String>) -> AdmissionRecord {
    AdmissionRecord {
        catalog,
        fields,
        path: Vec::new(),
        kernel: None,
    }
}

impl BarePackage<'_> {
    /// The atomic original Clock declaration context read from this package.
    ///
    /// Selection failures retain the existing unsupported representation
    /// boundary; competing authenticated origins retain both declarations.
    pub fn clock_declaration_table(
        &self,
    ) -> Result<
        fsl_semantics::model::clock_reference::normalization::ClockDeclarationTable,
        AdmissionFailure,
    > {
        use fsl_semantics::model::clock_reference::normalization::ClockDeclarationResolutionError;
        let unsupported = || {
            AdmissionFailure::Refused(Box::new(refused(
                CatalogCode::new("unknown_required_feature", "unsupported-feature"),
                BTreeMap::from([("reason", "missing-type-representation".to_owned())]),
            )))
        };
        let declarations = self.clock_declarations().map_err(|_| unsupported())?;
        let models =
            crate::CheckedTimedModel::from_admitted(self.package()).map_err(|_| unsupported())?;
        declarations.table(&models).map_err(|failure| {
            let (catalog, first, second) = match failure {
                crate::ClockDeclarationTableError::Resolution(
                    ClockDeclarationResolutionError::Fault(fault),
                ) => {
                    return AdmissionFailure::Fault(fault);
                }
                crate::ClockDeclarationTableError::Resolution(
                    ClockDeclarationResolutionError::Ambiguous(origins),
                ) => (
                    CatalogCode::new("ambiguous_declaration", "ambiguous-name"),
                    format!("{:?}", origins.first),
                    format!("{:?}", origins.second),
                ),
                crate::ClockDeclarationTableError::Collision(origins) => (
                    CatalogCode::new("conflicting_declaration", "duplicate-declaration"),
                    format!("{:?}", origins.first),
                    format!("{:?}", origins.second),
                ),
            };
            AdmissionFailure::Refused(Box::new(refused(
                catalog,
                BTreeMap::from([("first", first), ("second", second)]),
            )))
        })
    }
    /// Admit a simulated current state against this package's selected models
    /// and type environment. The resulting authority belongs to this exact
    /// bare package, as an admitted snapshot's authority does.
    pub fn admit_protocol_state(
        &self,
        model: &DomainPackageRef,
        packages: &PackageInput,
        model_limits: ModelNormalizationLimits,
        state: &fsl_semantics::model::state::ModelState,
        identity: DocumentRef,
    ) -> ObservationAdmission<AdmittedProtocolObservation<'static>> {
        if !self.model_selections().contains(model) {
            return ObservationAdmission::Refused(refused(
                CatalogCode::new("invalid_model_binding", "wrong-model-selection"),
                BTreeMap::new(),
            ));
        }
        let unsupported = |reason: &str| {
            ObservationAdmission::Refused(refused(
                CatalogCode::new("unknown_required_feature", "unsupported-feature"),
                BTreeMap::from([
                    ("package", self.package().name().to_owned()),
                    ("reason", reason.to_owned()),
                ]),
            ))
        };
        if let Err(failure) = self.witness() {
            return match failure {
                crate::BareSelectionFailure::Identity(refusal) => unsupported(
                    &crate::reader_protocol_projection::UnsupportedReason::from(refusal).name(),
                ),
                _ => unsupported("missing-type-representation"),
            };
        }
        if self.object_declarations().is_err() {
            return unsupported("missing-type-representation");
        }
        let Ok(world) = self.world() else {
            return unsupported("missing-type-representation");
        };
        let table = match self.clock_declaration_table() {
            Ok(table) => table,
            Err(AdmissionFailure::Refused(record)) => {
                return ObservationAdmission::Refused(*record);
            }
            Err(AdmissionFailure::Incomplete(record)) => {
                return ObservationAdmission::Incomplete(*record);
            }
            Err(AdmissionFailure::Fault(fault)) => return ObservationAdmission::Fault(fault),
        };
        let selections = self.model_selections();
        let observed = fsl_semantics::model::state::StateModel::new(
            fsl_semantics::model::observation::ModelSelections::new(selections.as_slice(), &[])
                .with_clock_declaration_table(&table),
            world.types(),
            packages,
            model_limits,
        )
        .and_then(|model| model.protocol_observation(state, identity.clone()));
        match observed {
            Ok(observation) => {
                ObservationAdmission::Admitted(AdmittedProtocolObservation::admitted_by_bare(
                    self.owner(),
                    model.clone(),
                    identity,
                    observation,
                    fsl_semantics::model::observation::AdmissionUsage::default(),
                ))
            }
            Err(AdmissionFailure::Refused(record)) => ObservationAdmission::Refused(*record),
            Err(AdmissionFailure::Incomplete(record)) => ObservationAdmission::Incomplete(*record),
            Err(AdmissionFailure::Fault(fault)) => ObservationAdmission::Fault(fault),
        }
    }

    /// Admit the current snapshot a package function call resolves its
    /// Reference and Population arguments against (FR-650). The model
    /// selections, checked types and witness rows come from this admitted
    /// V2 package and its supplied package rows.
    pub fn admit_call_snapshot(
        &self,
        packages: &PackageInput,
        model_limits: ModelNormalizationLimits,
        snapshots: &BTreeMap<DocumentRef, Vec<u8>>,
        selected: &DocumentRef,
        limits: ObservationLimits,
    ) -> Result<
        (
            fsl_semantics::model::object_environment::ObjectEnvironment,
            fsl_semantics::model::observation::AdmissionUsage,
        ),
        AdmissionFailure,
    > {
        let world = self.call_snapshot_world()?;
        let model_selections = self.model_selections();
        let table = self.clock_declaration_table()?;
        fsl_semantics::model::observation::admit_current_snapshot(
            fsl_semantics::model::observation::ModelSelections::new(
                model_selections.as_slice(),
                &[],
            )
            .with_clock_declaration_table(&table),
            world.types(),
            packages,
            model_limits,
            snapshots,
            selected,
            limits,
        )
    }

    /// [`Self::admit_call_snapshot`] retaining the effective model views
    /// from the same snapshot admission for subsequent population binding.
    /// The returned views are moved from admission without normalization.
    pub fn admit_call_snapshot_with_views(
        &self,
        packages: &PackageInput,
        model_limits: ModelNormalizationLimits,
        snapshots: &BTreeMap<DocumentRef, Vec<u8>>,
        selected: &DocumentRef,
        limits: ObservationLimits,
    ) -> Result<
        (
            fsl_semantics::model::object_environment::ObjectEnvironment,
            fsl_semantics::model::observation::AdmissionUsage,
            Vec<fsl_semantics::model::normalize::EffectiveView>,
        ),
        AdmissionFailure,
    > {
        let world = self.call_snapshot_world()?;
        let model_selections = self.model_selections();
        let table = self.clock_declaration_table()?;
        fsl_semantics::model::observation::admit_current_snapshot_with_views(
            fsl_semantics::model::observation::ModelSelections::new(
                model_selections.as_slice(),
                &[],
            )
            .with_clock_declaration_table(&table),
            world.types(),
            packages,
            model_limits,
            snapshots,
            selected,
            limits,
        )
    }

    /// Admit the snapshot and retain the genuine per-model normalization
    /// owners for population binding and reference identity work. Each owner
    /// keeps the exact identities and original meter used by this admission.
    pub fn admit_call_snapshot_with_provenance(
        &self,
        packages: &PackageInput,
        model_limits: ModelNormalizationLimits,
        snapshots: &BTreeMap<DocumentRef, Vec<u8>>,
        selected: &DocumentRef,
        limits: ObservationLimits,
    ) -> Result<
        (
            fsl_semantics::model::object_environment::ObjectEnvironment,
            fsl_semantics::model::observation::AdmissionUsage,
            Vec<fsl_semantics::model::intake::NormalizedModel>,
        ),
        AdmissionFailure,
    > {
        let world = self.call_snapshot_world()?;
        let model_selections = self.model_selections();
        fsl_semantics::model::observation::admit_current_snapshot_with_provenance(
            model_selections.as_slice(),
            world.types(),
            packages,
            model_limits,
            snapshots,
            selected,
            limits,
        )
    }

    fn call_snapshot_world(&self) -> Result<crate::BareWorld, AdmissionFailure> {
        let unsupported = |reason: &str| {
            AdmissionFailure::Refused(Box::new(refused(
                CatalogCode::new("unknown_required_feature", "unsupported-feature"),
                BTreeMap::from([
                    ("package", self.package().name().to_owned()),
                    ("reason", reason.to_owned()),
                ]),
            )))
        };
        if let Err(failure) = self.witness() {
            let reason = match failure {
                crate::BareSelectionFailure::Identity(refusal) => {
                    crate::reader_protocol_projection::UnsupportedReason::from(refusal).name()
                }
                _ => "missing-type-representation".to_owned(),
            };
            return Err(unsupported(&reason));
        }
        let world = self
            .world()
            .map_err(|_| unsupported("missing-type-representation"))?;
        self.object_declarations()
            .map_err(|_| unsupported("missing-type-representation"))?;
        Ok(world)
    }

    /// Admit one current snapshot for a model this package selects, over the
    /// package's own reads, its supply and the caller's domain package
    /// documents (FR-533 "Observation admission"). The returned authority
    /// belongs to this `BarePackage` value alone.
    pub fn admit_protocol_observation(
        &self,
        model: &DomainPackageRef,
        packages: &PackageInput,
        model_limits: ModelNormalizationLimits,
        snapshots: &BTreeMap<DocumentRef, Vec<u8>>,
        selected: &DocumentRef,
        limits: ObservationLimits,
    ) -> ObservationAdmission<AdmittedProtocolObservation<'static>> {
        if !self.model_selections().contains(model) {
            return ObservationAdmission::Refused(refused(
                CatalogCode::new("invalid_model_binding", "wrong-model-selection"),
                BTreeMap::new(),
            ));
        }
        let unsupported = |reason: &str| {
            ObservationAdmission::Refused(refused(
                CatalogCode::new("unknown_required_feature", "unsupported-feature"),
                BTreeMap::from([
                    ("package", self.package().name().to_owned()),
                    ("reason", reason.to_owned()),
                ]),
            ))
        };
        if let Err(failure) = self.witness() {
            return match failure {
                crate::BareSelectionFailure::Identity(refusal) => unsupported(
                    &crate::reader_protocol_projection::UnsupportedReason::from(refusal).name(),
                ),
                _ => unsupported("missing-type-representation"),
            };
        }
        let Ok(world) = self.world() else {
            return unsupported("missing-type-representation");
        };
        if self.object_declarations().is_err() {
            return unsupported("missing-type-representation");
        }
        let table = match self.clock_declaration_table() {
            Ok(table) => table,
            Err(AdmissionFailure::Refused(record)) => {
                return ObservationAdmission::Refused(*record);
            }
            Err(AdmissionFailure::Incomplete(record)) => {
                return ObservationAdmission::Incomplete(*record);
            }
            Err(AdmissionFailure::Fault(fault)) => return ObservationAdmission::Fault(fault),
        };
        match admit_current_snapshot(
            fsl_semantics::model::observation::ModelSelections::new(
                core::slice::from_ref(model),
                &[],
            )
            .with_clock_declaration_table(&table),
            world.types(),
            packages,
            model_limits,
            snapshots,
            selected,
            limits,
        ) {
            Ok((environment, usage)) => {
                let populations = environment.population_completeness().clone();
                ObservationAdmission::Admitted(AdmittedProtocolObservation::admitted_by_bare(
                    self.owner(),
                    model.clone(),
                    selected.clone(),
                    Observation {
                        identity: selected.clone(),
                        populations,
                        environment,
                        out_of_range: Vec::new(),
                    },
                    usage,
                ))
            }
            Err(AdmissionFailure::Refused(record)) => ObservationAdmission::Refused(*record),
            Err(AdmissionFailure::Incomplete(record)) => ObservationAdmission::Incomplete(*record),
            Err(AdmissionFailure::Fault(fault)) => ObservationAdmission::Fault(fault),
        }
    }
}
