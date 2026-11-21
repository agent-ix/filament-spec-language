// SPDX-License-Identifier: AGPL-3.0-or-later
//! The source-backed handoff borrows the authoritative reference converter.

use fsl_eval::simulation::model::ModelSystem;
use fsl_package::{Emission, EmitLimits, ReferenceCatalog, V2ReadLimits};
use fsl_semantics::model::intake::PackageInput;

use crate::reference_codec::{ReferenceCodec, ReferenceCodecError};
use fsl_eval::simulation::model::ModelRestoreBudget;

/// Admission failures remain native causes, never identity guesses.
#[derive(Debug, thiserror::Error)]
pub enum ReferenceContextError {
    /// The actual native state could not be canonically identified.
    #[error(transparent)]
    StateKey(#[from] fsl_eval::simulation::EncodingRefusal),
    /// This entry accepts an actual source-backed runtime only.
    #[error("reference context has no checked source owner")]
    SourceOwner,
    /// Strict package admission refused the original emission.
    #[error("reference context package admission refused: {0:?}")]
    Admission(Box<fsl_package::ImportViewRefusal>),
    /// Catalog construction refused its actual native owners.
    #[error(transparent)]
    Catalog(#[from] fsl_package::ReferenceCatalogError),
    /// The shared converter refused a reference, value or transition.
    #[error(transparent)]
    Codec(#[from] ReferenceCodecError),
}

/// Admit the actual retained source emission and scope one converter to the
/// actual runtime model. The callback cannot retain a catalog past its owners.
pub fn with_source_reference_codec<T>(
    system: &ModelSystem<'_>,
    emission: &Emission,
    packages: &PackageInput,
    read_limits: V2ReadLimits,
    emit_limits: EmitLimits,
    budget: &mut dyn ModelRestoreBudget,
    run: impl FnOnce(
        &ReferenceCodec<'_, '_, '_>,
        &mut dyn ModelRestoreBudget,
    ) -> Result<T, ReferenceContextError>,
) -> Result<T, ReferenceContextError> {
    let source = system
        .input()
        .source_package()
        .ok_or(ReferenceContextError::SourceOwner)?;
    let identity = crate::LibraryName::new(source.graph().package_name().to_owned())
        .map_err(|_| ReferenceContextError::SourceOwner)?;
    let _read = fsl_package::read_evaluation_emission(
        source,
        emission,
        identity,
        packages,
        read_limits,
        emit_limits,
        fsl_package::SourceWitnessSupplier::new(emission),
    )
    .map_err(|cause| ReferenceContextError::Admission(Box::new(cause)))?;
    let catalog = ReferenceCatalog::from_checked_model(source, system.reference_model(), budget)?;
    let codec = ReferenceCodec::new(&catalog, system)?;
    run(&codec, budget)
}
