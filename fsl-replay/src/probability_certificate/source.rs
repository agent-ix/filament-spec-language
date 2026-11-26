// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-282 source-backed probability certificate checking without an analyzer.

use super::document::{
    check_probability_certificate_document_as, ProbabilityDocumentError, ProbabilityDocumentLimits,
};
use super::{CertificateRole, ExactProbabilityCertificateCheck, ObligationIdentity};
use ix_trace_rs::implements;
use crate::spine::{
    self, DependencyInput, FrontEndFailure, ParseRequest, SourceComponent, SpineLimits,
};
use crate::{ByteProvision, DefinitionSourceRef};
use filament_exact::{Cancel, ScalarLimits};
use fsl_eval::simulation::model::{
    ExpansionLimits, ModelSimulationRequest, ModelSystem, PopulationUniverse,
};
use fsl_foundation::node_wire::{node_key_from_text, node_key_to_json};
use fsl_foundation::source::{Source, SourceReadCause, SourceReadError};
use fsl_foundation::SourceIdentity;
use fsl_semantics::model::intake::package_input;
use fsl_semantics::model::observation::{AdmissionFailure, DocumentRef, ObservationLimits};
use serde_json::value::RawValue;
use std::collections::BTreeMap;

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
    /// Whether the certificate is the claim side or a value-bounds
    /// counterpart (FR-1042).
    pub role: CertificateRole,
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
    /// Whether the certificate is the claim side or a value-bounds
    /// counterpart (FR-1042).
    pub role: CertificateRole,
}

/// Recompile exact admitted source through S1–S3, construct the production
/// model system and check the document. No emitted package, solver product,
/// ambient I/O or analyzer implementation enters this path.
#[implements("FR-282-AC-11")]
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
#[implements("FR-282-AC-1")]
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
            role: request.role,
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
        resolved = Some(held.resolve(emitted, request.item)?);
    }
    let certificate = resolved.as_deref().unwrap_or(request.certificate);
    // Refuse a foreign claim before compiler/model work as well as enumeration.
    if held.is_none() || resolved.is_some() {
        refuse_foreign_claim(
            certificate,
            &claimed(request.item, held.is_some()),
            limits,
            cancel,
        )?;
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
    // A held certificate and its request may agree on a model that does not
    // own the claim: the recompiled package decides the owner.
    if let (Some(_), Some(model)) = (&held, request.item.model()) {
        let graph = checked.package().graph();
        if graph.owning_model(request.item.node()) != Some(model.as_str()) {
            return Err(ProbabilityDocumentError::IdentityMismatch.into());
        }
    }
    if let (Some(held), None, None) = (&held, emitted, &resolved) {
        let recompiled = spine::package(&checked, limits.spine.package, cancel)
            .map_err(|failure| ProbabilitySourceError::Compile(Box::new(failure)))?
            .into_value();
        resolved = Some(held.resolve(recompiled.package().bytes(), request.item)?);
        refuse_foreign_claim(
            resolved.as_deref().unwrap_or_default(),
            &claimed(request.item, true),
            limits,
            cancel,
        )?;
    }
    let certificate = resolved.as_deref().unwrap_or(request.certificate);
    let claims = checked.package().graph().probabilistic_model().claims();
    super::bind_claim(claims, &claimed(request.item, held.is_some()), cancel)
        .map_err(ProbabilityDocumentError::from)?;
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
    check_probability_certificate_document_as(
        &system,
        claims,
        &claimed(request.item, held.is_some()),
        certificate,
        request.role,
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

/// A certificate document in the form an analyze result writes: its identity
/// names the owning model, and its node ids are index segments of an emitted
/// package (FSpec FR-322 "Operand index segments"), except each identity's
/// `node_id`, which is the structural id (FSpec FR-412).
struct HeldCertificate {
    /// The document without its identities' `node_id` members.
    document: serde_json::Value,
    /// Each identity's JSON pointer and written `node_id`.
    node_ids: Vec<(String, Box<RawValue>)>,
}

/// `written`, a certificate object, as JSON with the `node_id` of its
/// identity moved to `node_ids`. That id nests as deeply as the claim's term,
/// past serde_json's recursion limit, so it stays text until the iterative
/// node reader reads it. `parts` is how many levels of conditional members
/// below `written` hold identities of their own.
fn detach_node_ids(
    written: &RawValue,
    at: &str,
    parts: u8,
    node_ids: &mut Vec<(String, Box<RawValue>)>,
) -> Option<serde_json::Value> {
    type Members = BTreeMap<String, Box<RawValue>>;
    let members: Members = serde_json::from_str(written.get()).ok()?;
    let mut object = serde_json::Map::new();
    for (name, member) in members {
        let value = match name.as_str() {
            "identity" => {
                let mut identity: Members = serde_json::from_str(member.get()).ok()?;
                if let Some(node_id) = identity.remove("node_id") {
                    node_ids.push((format!("{at}/identity"), node_id));
                }
                serde_json::to_value(identity).ok()?
            }
            "conditional" | "numerator" | "denominator" | "post" | "reset"
                if parts > 0 && member.get().starts_with('{') =>
            {
                detach_node_ids(&member, &format!("{at}/{name}"), parts - 1, node_ids)?
            }
            _ => serde_json::from_str(member.get()).ok()?,
        };
        object.insert(name, value);
    }
    Some(serde_json::Value::Object(object))
}

impl HeldCertificate {
    /// The document, when an index segment `{"node": i}` appears anywhere in it
    /// or its identity names a model: the form an analyze result writes.
    fn read(bytes: &[u8]) -> Option<Self> {
        let mut node_ids = Vec::new();
        // Two levels: `conditional`, then the part it names.
        let value = detach_node_ids(serde_json::from_slice(bytes).ok()?, "", 2, &mut node_ids)?;
        let mut held = !value["identity"]["model"].is_null();
        let mut pending = vec![&value];
        while let Some(current) = pending.pop() {
            match current {
                serde_json::Value::Object(object) => {
                    held |=
                        object.len() == 1 && object.get("node").is_some_and(|node| node.is_u64());
                    pending.extend(object.values());
                }
                serde_json::Value::Array(items) => pending.extend(items.iter()),
                _ => {}
            }
        }
        held.then_some(Self {
            document: value,
            node_ids,
        })
    }

    /// The document with every held node id resolved against `package`'s
    /// graph, written as canonical structural bytes. An identity's `node_id`
    /// is read as the structural id it is, so a held one is refused.
    /// Every identity, a conditional part's included, must name the model
    /// the request names; the checker derives the claim itself and its
    /// document names no model, so the resolved bytes carry none.
    fn resolve(
        &self,
        package: &[u8],
        item: &ObligationIdentity,
    ) -> Result<Vec<u8>, ProbabilityDocumentError> {
        let graph = fsl_semantics::check::wire_id::WireGraph::of_package_bytes(package)
            .map_err(|_| ProbabilityDocumentError::Malformed)?;
        let mut structural = graph
            .structural_value(&self.document)
            .map_err(|_| ProbabilityDocumentError::Malformed)?;
        // An identity nests as deeply as its text says, so the value that
        // holds it is encoded and released without recursion.
        let written = self
            .restore_identities(&mut structural, item)
            .and_then(|()| {
                quire_canonical::to_vec(&structural, quire_canonical::Limits::new(u64::MAX))
                    .map_err(|_| ProbabilityDocumentError::Malformed)
            });
        quire_canonical::drop_value(structural);
        written
    }

    /// Put each identity's `node_id` back into `structural`, and replace the
    /// model it names, which must be `item`'s, with null.
    fn restore_identities(
        &self,
        structural: &mut serde_json::Value,
        item: &ObligationIdentity,
    ) -> Result<(), ProbabilityDocumentError> {
        let model = item.model().map(|model| model.as_str());
        let model = model.map_or(serde_json::Value::Null, serde_json::Value::from);
        for (at, written) in &self.node_ids {
            let identity = structural
                .pointer_mut(at)
                .and_then(serde_json::Value::as_object_mut)
                .ok_or(ProbabilityDocumentError::Malformed)?;
            if identity.get("model") != Some(&model) {
                return Err(ProbabilityDocumentError::IdentityMismatch);
            }
            identity.insert("model".to_owned(), serde_json::Value::Null);
            let node_id = node_key_from_text(written.get())
                .and_then(|key| node_key_to_json(&key))
                .map_err(|_| ProbabilityDocumentError::Malformed)?;
            identity.insert("node_id".to_owned(), node_id);
        }
        Ok(())
    }
}

/// The identity the checker derives for the claim: the requested node and
/// ordinal, and no model when the certificate is a resolved held document.
fn claimed(item: &ObligationIdentity, held: bool) -> ObligationIdentity {
    if held {
        ObligationIdentity::new(item.node().clone(), None, item.ordinal())
    } else {
        item.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;

    const PACKAGE: &[u8] = br#"{"semantic_graph":{"nodes":[
        {"node_id":{"package":"own","path":["leaf"],"ordinal":0}},
        {"node_id":{"package":"own","path":["wrap"],"ordinal":0}}]}}"#;

    /// Operands nested far past what a small stack can recurse over.
    const HOSTILE_DEPTH: usize = 50_000;

    /// A structural id, `depth` operands nested, in canonical member order.
    fn nested(depth: usize) -> String {
        let open = r#"{"ordinal":0,"package":"own","path":["application","#.repeat(depth);
        let leaf = r#"{"ordinal":0,"package":"own","path":["leaf"]}"#;
        format!("{open}{leaf}{}", "]}".repeat(depth))
    }

    /// The written form of a certificate under model `m` and one conditional
    /// part under `part_model`, both naming the claim `node_id`, with one held
    /// operand elsewhere.
    fn written(node_id: &str, part_model: &str) -> String {
        let identity = |model| format!(r#"{{"model":"{model}","node_id":{node_id},"ordinal":0}}"#);
        let (top, part) = (identity("m"), identity(part_model));
        format!(
            r#"{{"conditional":{{"mode":"ratio","numerator":{{"identity":{part}}}}},"identity":{top},"reward":{{"ordinal":0,"package":"own","path":["wrap",{{"node":0}}]}}}}"#
        )
    }

    fn item(node_id: &str) -> ObligationIdentity {
        ObligationIdentity::new(
            node_key_from_text(node_id).expect("a structural id"),
            Some(crate::ModelIdentity::new("m".to_owned())),
            0,
        )
    }

    fn resolve(
        document: &str,
        item: &ObligationIdentity,
    ) -> Result<String, ProbabilityDocumentError> {
        let held = HeldCertificate::read(document.as_bytes()).expect("the held form");
        held.resolve(PACKAGE, item)
            .map(|bytes| String::from_utf8(bytes).expect("UTF-8"))
    }

    /// FR-621-AC-16: a written certificate whose structural identity nests
    /// past serde_json's recursion limit is read as the held form and
    /// resolved without recursing once per level: its held ids resolve, each
    /// identity's node id is kept as written and each model is null.
    #[trace("TC-3154", "FR-621-AC-16")]
    #[test]
    fn a_deeply_nested_certificate_identity_resolves() {
        let check = || {
            let node_id = nested(HOSTILE_DEPTH);
            let document = written(&node_id, "m");
            assert!(serde_json::from_str::<serde_json::Value>(&document).is_err());
            let resolved = resolve(&document, &item(&node_id)).expect("the document resolves");
            let identity = format!(r#"{{"model":null,"node_id":{node_id},"ordinal":0}}"#);
            assert!(resolved.contains(&format!(r#","identity":{identity},"#)));
            assert!(resolved.contains(&format!(r#""numerator":{{"identity":{identity}}}"#)));
            assert!(resolved.contains(&format!(
                r#""reward":{{"ordinal":0,"package":"own","path":["wrap",{}]}}"#,
                nested(0)
            )));
        };
        std::thread::Builder::new()
            .stack_size(512 * 1024)
            .spawn(check)
            .expect("a thread starts")
            .join()
            .expect("the check passes");
    }

    /// FR-621-AC-16: a conditional part names the request's model too.
    #[trace("TC-3154", "FR-621-AC-16")]
    #[test]
    fn a_conditional_part_naming_another_model_is_refused() {
        let node_id = nested(0);
        assert_eq!(
            resolve(&written(&node_id, "other"), &item(&node_id)),
            Err(ProbabilityDocumentError::IdentityMismatch)
        );
    }

    /// FR-621-AC-16: a certificate identity is the structural id, so one
    /// written with an index segment is refused.
    #[trace("TC-3154", "FR-621-AC-16")]
    #[test]
    fn a_held_certificate_identity_is_refused() {
        let held = r#"{"ordinal":0,"package":"own","path":["wrap",{"node":0}]}"#;
        assert_eq!(
            resolve(&written(held, "m"), &item(&nested(0))),
            Err(ProbabilityDocumentError::Malformed)
        );
    }
}
