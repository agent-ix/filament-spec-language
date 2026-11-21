// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-282 source-backed probability certificate checking without an analyzer.

use super::document::{
    check_probability_certificate_document, ProbabilityDocumentError, ProbabilityDocumentLimits,
};
use super::{ExactProbabilityCertificateCheck, ObligationIdentity};
use crate::spine::{
    self, DependencyInput, FrontEndFailure, ParseRequest, SourceComponent, SpineLimits,
};
use crate::{ByteProvision, DefinitionSourceRef};
use filament_exact::{Cancel, ScalarLimits};
use fsl_eval::simulation::model::{
    ExpansionLimits, ModelSimulationRequest, ModelSystem, PopulationUniverse,
};
use fsl_foundation::source::{Source, SourceReadCause, SourceReadError};
use fsl_foundation::SourceIdentity;
use fsl_semantics::model::intake::package_input;
use fsl_semantics::model::observation::{AdmissionFailure, DocumentRef, ObservationLimits};

/// All source and subject data needed to recheck one native-source certificate.
/// Imported library configurations are outside this initial source facade.
pub struct ProbabilitySourceRequest<'a> {
    /// Authored source identity, looked up only through the byte provision.
    pub source: &'a DefinitionSourceRef,
    /// Exact expected source component, including admitted UTF-8 content.
    pub expected_source: &'a SourceComponent,
    /// Native package name supplied to the checking stage.
    pub package_name: &'a str,
    /// Complete source, domain-package and snapshot provision.
    pub provision: &'a ByteProvision,
    /// Requested structural claim identity, independent of the certificate.
    pub item: &'a ObligationIdentity,
    /// Initial document identities; exactly one is admitted by the profile.
    pub initial: Vec<DocumentRef>,
    /// Finite universes keyed by original declaration identities.
    pub universes: Vec<PopulationUniverse>,
    /// Exact canonical certificate bytes.
    pub certificate: &'a [u8],
}

/// Caller-owned stage, admission and certificate ceilings.
#[derive(Clone, Copy)]
pub struct ProbabilitySourceLimits {
    /// Compiler and package-emission ceilings.
    pub spine: SpineLimits,
    /// Snapshot admission ceilings.
    pub observation: ObservationLimits,
    /// Per-state expansion ceiling.
    pub expansion: ExpansionLimits,
    /// Per-evaluation scalar ceilings.
    pub scalar: ScalarLimits,
    /// Document and independent graph ceilings.
    pub document: ProbabilityDocumentLimits,
}

/// Source checking never fabricates a truth value on refusal or stop.
#[derive(Debug, thiserror::Error)]
pub enum ProbabilitySourceError {
    /// A provision document exceeded its caller-owned admission ceiling.
    #[error("probability certificate provision limit: {setting} ({actual} > {bound})")]
    ProvisionLimit {
        /// Shared limit setting name.
        setting: &'static str,
        /// Caller ceiling.
        bound: u64,
        /// Actual offered bytes.
        actual: u64,
    },
    /// A referenced source is absent or differs from the expected component.
    #[error("conflicting_identity: probability certificate source")]
    SourceMismatch,
    /// Source bytes failed the shared source admission rules.
    #[error("probability certificate source refused: {cause:?}")]
    Source {
        /// Shared source admission condition.
        cause: SourceReadCause,
        /// Exact source provenance and diagnostic.
        error: Box<SourceReadError>,
        /// Caller source byte ceiling.
        byte_bound: usize,
        /// Offered source byte length.
        byte_length: usize,
    },
    /// A compiler stage refused, stopped or was cancelled.
    #[error("probability certificate compilation stopped: {0:?}")]
    Compile(Box<FrontEndFailure>),
    /// Domain package intake failed.
    #[error("probability certificate domain intake failed: {0:?}")]
    Packages(Box<fsl_semantics::model::normalize::ModelRefusal>),
    /// State/model admission failed with its original typed cause.
    #[error("probability certificate subject refused: {0:?}")]
    Subject(AdmissionFailure),
    /// The emitted package bytes are not a readable document.
    #[error("probability certificate package unreadable: {0}")]
    PackageRead(quire_canonical::ReadError),
    /// The emitted package does not describe its name and sources.
    #[error("probability certificate package refused: {0}")]
    Package(Box<crate::ReplayRequestRefusal>),
    /// The emitted package does not name exactly one source.
    #[error("probability certificate package names {0} sources, not one")]
    PackageSources(usize),
    /// The request names a package name other than the emitted package's.
    #[error("conflicting_identity: probability certificate package name {requested:?} is not {actual:?}")]
    PackageName {
        /// The name the request stated.
        requested: String,
        /// The name the emitted package carries.
        actual: String,
    },
    /// The package the byte provision recompiles to differs from the
    /// emitted package the request names.
    #[error("conflicting_identity: probability certificate package")]
    PackageMismatch,
    /// Certificate admission or independent checking stopped.
    #[error(transparent)]
    Document(#[from] ProbabilityDocumentError),
}

/// A certificate checked over the package S4 emitted for its source. The
/// package names the source and the package name; the byte provision
/// supplies the source and the documents.
pub struct ProbabilityPackageRequest<'a> {
    /// The emitted `filament.checked-package/v2` bytes.
    pub package: &'a [u8],
    /// The package name the request states, which must be the package's own.
    pub package_name: Option<&'a str>,
    /// Complete source, domain-package and snapshot provision.
    pub provision: &'a ByteProvision,
    /// Requested structural claim identity, independent of the certificate.
    pub item: &'a ObligationIdentity,
    /// Initial document identities; exactly one is admitted by the profile.
    pub initial: Vec<DocumentRef>,
    /// Finite universes keyed by original declaration identities.
    pub universes: Vec<PopulationUniverse>,
    /// Exact canonical certificate bytes.
    pub certificate: &'a [u8],
}

/// Recompile exact admitted source through S1–S3, construct the production
/// model system and check the document. No emitted package, solver product,
/// ambient I/O or analyzer implementation enters this path.
pub fn check_probability_certificate_source(
    request: ProbabilitySourceRequest<'_>,
    limits: ProbabilitySourceLimits,
    cancel: &Cancel,
) -> Result<ExactProbabilityCertificateCheck, ProbabilitySourceError> {
    check_recompiled(request, None, limits, cancel)
}

/// Check a certificate over an emitted package: the package names its
/// source and package name, the byte provision recompiles through S1–S4, and
/// the recompiled package must be byte-identical to the one named before the
/// document is checked.
pub fn check_probability_certificate_package(
    request: ProbabilityPackageRequest<'_>,
    limits: ProbabilitySourceLimits,
    cancel: &Cancel,
) -> Result<ExactProbabilityCertificateCheck, ProbabilitySourceError> {
    let ceiling = limits
        .spine
        .model
        .intake
        .input_bytes
        .max(limits.observation.document_bytes);
    let package = crate::ReplayPackage::read(request.package, ceiling)
        .map_err(ProbabilitySourceError::PackageRead)?;
    let (name, sources) = package
        .name_and_sources()
        .map_err(|error| ProbabilitySourceError::Package(Box::new(error)))?;
    if let Some(requested) = request.package_name.filter(|requested| *requested != name) {
        return Err(ProbabilitySourceError::PackageName {
            requested: requested.to_owned(),
            actual: name,
        });
    }
    let [source] = sources.as_slice() else {
        return Err(ProbabilitySourceError::PackageSources(sources.len()));
    };
    let bytes = request
        .provision
        .get(source)
        .ok_or(ProbabilitySourceError::SourceMismatch)?;
    let identity = SourceIdentity::new(source.authority(), source.identity());
    let text = Source::read_typed(
        identity.clone(),
        source.identity(),
        bytes,
        limits.spine.source.source_bytes,
    )
    .map_err(|refusal| ProbabilitySourceError::Source {
        cause: refusal.cause,
        error: Box::new(refusal.error),
        byte_bound: limits.spine.source.source_bytes,
        byte_length: bytes.len(),
    })?;
    let expected = SourceComponent {
        authority: identity.authority,
        identity: identity.identity,
        content: text.text().to_owned(),
    };
    check_recompiled(
        ProbabilitySourceRequest {
            source,
            expected_source: &expected,
            package_name: &name,
            provision: request.provision,
            item: request.item,
            initial: request.initial,
            universes: request.universes,
            certificate: request.certificate,
        },
        Some(request.package),
        limits,
        cancel,
    )
}

fn check_recompiled(
    request: ProbabilitySourceRequest<'_>,
    emitted: Option<&[u8]>,
    limits: ProbabilitySourceLimits,
    cancel: &Cancel,
) -> Result<ExactProbabilityCertificateCheck, ProbabilitySourceError> {
    // A certificate an analyze result carries holds its node ids as index
    // segments of the emitted package (FSpec FR-322); it resolves against the
    // graph of the package the request names, or, for a source request, of the
    // package the byte provision recompiles to.
    let held = HeldCertificate::read(request.certificate);
    let mut resolved = None;
    if let (Some(held), Some(emitted)) = (&held, emitted) {
        resolved = Some(held.resolve(emitted)?);
    }
    let certificate = resolved.as_deref().unwrap_or(request.certificate);
    // Refuse a foreign claim before compiler/model work as well as enumeration.
    if held.is_none() || resolved.is_some() {
        refuse_foreign_claim(certificate, request.item, limits, cancel)?;
    }
    if request.initial.len() != 1 {
        return Err(ProbabilityDocumentError::Unsupported.into());
    }
    let bytes = request
        .provision
        .get(request.source)
        .ok_or(ProbabilitySourceError::SourceMismatch)?;
    let identity = SourceIdentity::new(request.source.authority(), request.source.identity());
    let source = Source::read_typed(
        identity.clone(),
        request.source.identity(),
        bytes,
        limits.spine.source.source_bytes,
    )
    .map_err(|refusal| ProbabilitySourceError::Source {
        cause: refusal.cause,
        error: Box::new(refusal.error),
        byte_bound: limits.spine.source.source_bytes,
        byte_length: bytes.len(),
    })?;
    let actual = SourceComponent {
        authority: identity.authority.clone(),
        identity: identity.identity.clone(),
        content: source.text().to_owned(),
    };
    if actual != *request.expected_source {
        return Err(ProbabilitySourceError::SourceMismatch);
    }
    let document_ceiling = limits
        .spine
        .model
        .intake
        .input_bytes
        .max(limits.observation.document_bytes);
    for bytes in request.provision.documents() {
        let actual = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
        if actual > document_ceiling {
            return Err(ProbabilitySourceError::ProvisionLimit {
                setting: "provision.input_bytes",
                bound: document_ceiling,
                actual,
            });
        }
    }
    for bytes in request.provision.package_documents() {
        let actual = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
        if actual > limits.spine.model.intake.input_bytes {
            return Err(ProbabilitySourceError::ProvisionLimit {
                setting: "intake.input_bytes",
                bound: limits.spine.model.intake.input_bytes,
                actual,
            });
        }
    }
    let packages = package_input(request.provision.package_documents())
        .map_err(|refusal| ProbabilitySourceError::Packages(Box::new(refusal)))?;
    let parsed = spine::parse(
        &ParseRequest {
            source: &identity,
            path: request.source.identity(),
            bytes,
        },
        limits.spine.source,
        cancel,
    )
    .map_err(|failure| ProbabilitySourceError::Compile(Box::new(failure)))?
    .into_value();
    if !parsed.unit().selections().imports.is_empty() {
        return Err(ProbabilityDocumentError::Unsupported.into());
    }
    let models = spine::select(&parsed, &packages, limits.spine.model, cancel)
        .map_err(|failure| ProbabilitySourceError::Compile(Box::new(failure)))?
        .into_value();
    let checked = spine::check(
        &parsed,
        request.package_name.to_owned(),
        &models,
        &DependencyInput::default(),
        limits.spine,
        cancel,
    )
    .map_err(|failure| ProbabilitySourceError::Compile(Box::new(failure)))?
    .into_value();
    if let Some(emitted) = emitted {
        let recompiled = spine::package(&checked, limits.spine.package, cancel)
            .map_err(|failure| ProbabilitySourceError::Compile(Box::new(failure)))?
            .into_value();
        if recompiled.package().bytes() != emitted {
            return Err(ProbabilitySourceError::PackageMismatch);
        }
    }
    if let (Some(held), None, None) = (&held, emitted, &resolved) {
        let recompiled = spine::package(&checked, limits.spine.package, cancel)
            .map_err(|failure| ProbabilitySourceError::Compile(Box::new(failure)))?
            .into_value();
        resolved = Some(held.resolve(recompiled.package().bytes())?);
        refuse_foreign_claim(
            resolved.as_deref().unwrap_or_default(),
            request.item,
            limits,
            cancel,
        )?;
    }
    let certificate = resolved.as_deref().unwrap_or(request.certificate);
    let claims = checked.package().graph().probabilistic_model().claims();
    super::bind_claim(claims, request.item, cancel).map_err(ProbabilityDocumentError::from)?;
    let snapshots = request.provision.snapshots();
    let simulation = ModelSimulationRequest {
        input: fsl_eval::simulation::model::ModelCheckInput::source(checked.package()),
        packages: &packages,
        normalization: limits.spine.model,
        snapshots: &snapshots,
        initial: request.initial,
        universes: request.universes,
        observation: limits.observation,
        expansion: limits.expansion,
        meter: limits.scalar,
        emit: limits.spine.package.emit(),
    };
    let admitted = ModelSystem::new_with_cancel(simulation, cancel);
    super::poll_cancel(cancel).map_err(ProbabilityDocumentError::from)?;
    let system = admitted.map_err(ProbabilitySourceError::Subject)?;
    check_probability_certificate_document(
        &system,
        claims,
        request.item,
        certificate,
        limits.document,
        cancel,
    )
    .map_err(Into::into)
}

/// Refuse a certificate that names a claim other than the requested one.
fn refuse_foreign_claim(
    certificate: &[u8],
    item: &ObligationIdentity,
    limits: ProbabilitySourceLimits,
    cancel: &Cancel,
) -> Result<(), ProbabilitySourceError> {
    let identity = match super::document::certificate_identity(certificate, limits.document, cancel)
    {
        Ok(identity) => identity,
        Err(error) => {
            return super::document::document_admission_error(error)
                .map(|_| ())
                .map_err(Into::into)
        }
    };
    if &identity != item {
        return Err(ProbabilityDocumentError::IdentityMismatch.into());
    }
    Ok(())
}

/// A certificate document whose identity names its node by index segments of
/// an emitted package (FSpec FR-322 "Operand index segments"), the form an
/// analyze result writes.
struct HeldCertificate(serde_json::Value);

impl HeldCertificate {
    /// The document, when its identity path holds an index segment.
    fn read(bytes: &[u8]) -> Option<Self> {
        let value: serde_json::Value = serde_json::from_slice(bytes).ok()?;
        let held = value["identity"]["node_id"]["path"]
            .as_array()?
            .iter()
            .any(|segment| segment.get("node").is_some());
        held.then_some(Self(value))
    }

    /// The document with every node id resolved against `package`'s graph,
    /// written as canonical structural bytes.
    fn resolve(&self, package: &[u8]) -> Result<Vec<u8>, ProbabilityDocumentError> {
        let graph = fsl_semantics::check::wire_id::WireGraph::of_package_bytes(package)
            .map_err(|_| ProbabilityDocumentError::Malformed)?;
        let structural = graph
            .structural_value(&self.0)
            .map_err(|_| ProbabilityDocumentError::Malformed)?;
        fsl_foundation::canonical_json::encode(&structural)
            .map_err(|_| ProbabilityDocumentError::Malformed)
    }
}
