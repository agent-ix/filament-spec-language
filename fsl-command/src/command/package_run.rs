// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-100, FR-945, FR-1039: a `native-run/1` request that selects an existing
//! `filament.checked-package/v2` artifact through `package` in place of a
//! `program`.
//!
//! The strict checked-package reader admits the selected bytes against the
//! request's domain packages; the package's `lock` supplies the edition,
//! profiles and laws (the integer-division law included, FR-767) and its name
//! rows the function the `call` selects. The call runs over the
//! [`fsl_package::BarePackage`] of the admitted package. No source is read or
//! compiled, and no refusal falls back to source.

use std::path::Path;

use super::{output, wire, FileCounts, Result, RunCause, RunResult};

/// The FR-517 files a `{directory}` selection reads.
const PACKAGE_FILE: &str = "checked-package.json";
const IDENTITIES_FILE: &str = "identities.json";
const EQUALITY_FILE: &str = "equality.json";

/// The contract version the package route runs (FR-1039).
const CONTRACT_VERSION: &str = "filament.checked-package/v2";

/// A prepared call over a selected checked package: the admitted package,
/// its handoff witness rows when the selection is a directory, and the call.
pub struct PreparedPackageRun {
    package: filament_contract_model::CheckedPackageV2,
    identities: Option<fsl_package::ReadIdentityRows>,
    equality: Option<fsl_package::ReadEqualityRows>,
    budget: filament_contract_model::NativeJoinBudget,
    packages: fsl_semantics::model::intake::PackageInput,
    model_limits: fsl_semantics::model::accounting::ModelNormalizationLimits,
    work: Work,
    /// The request's `package` selection, echoed by the result.
    selection: serde_json::Value,
    /// The domain package sources the request supplied.
    models: Vec<fsl_foundation::Source>,
}

/// What a package run does once the package is admitted.
enum Work {
    /// FR-945: call a function the package names.
    Call(Box<fsl_replay::spine::Call>),
    /// FR-1039: lower a temporal clause the package names (FR-931).
    Lowering(super::tl::TlLowering),
}

/// The package bytes and handoff rows admitted for a selected package. The
/// export route shares this reader so package execution and mapping apply the
/// same strict V2 admission and file/directory handoff rules.
pub(super) struct AdmittedPackage {
    pub(super) package: filament_contract_model::CheckedPackageV2,
    pub(super) identities: Option<fsl_package::ReadIdentityRows>,
    pub(super) equality: Option<fsl_package::ReadEqualityRows>,
    pub(super) budget: filament_contract_model::NativeJoinBudget,
    pub(super) packages: fsl_semantics::model::intake::PackageInput,
    pub(super) model_limits: fsl_semantics::model::accounting::ModelNormalizationLimits,
    pub(super) snapshot: Option<Vec<u8>>,
    pub(super) selection: serde_json::Value,
    pub(super) models: Vec<fsl_foundation::Source>,
}

impl PreparedPackageRun {
    pub(super) fn sources(&self) -> crate::Sources {
        crate::Sources::from_sources(self.models.iter().cloned())
    }
}

fn invalid(member: &str, reason: impl Into<String>) -> RunCause {
    RunCause::ClauseRequest {
        member: member.to_owned(),
        reason: reason.into(),
    }
}

/// The request-stage refusal of a package the strict reader refused, with
/// the reader's pointer under `/request/package` (FR-1039).
fn package_refusal(refusal: fsl_package::PackageAdmissionRefusal) -> RunCause {
    let fsl_package::PackageAdmissionRefusal {
        code,
        cause,
        pointer,
        identity,
    } = refusal;
    let label = match &cause {
        Some(cause) => format!("{}/{cause}", code.as_str()),
        None => code.as_str().to_owned(),
    };
    if let Some(identity) = identity {
        // FR-1039: a recorded model selection with no `models` entry.
        return RunCause::AnalyzePackage {
            code,
            cause,
            pointer: Some("/request/models".to_owned()),
            message: format!("{label} at /request/models: no models entry supplies {identity}"),
        };
    }
    let pointer = format!("/request/package{}", pointer.unwrap_or_default());
    RunCause::AnalyzePackage {
        code,
        cause,
        message: format!("{label} at {pointer}"),
        pointer: Some(pointer),
    }
}

/// A selected path beneath the request's directory.
fn beneath(member: &str, path: &str) -> Result<String> {
    use std::path::Component;
    if path.is_empty()
        || Path::new(path).components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err(invalid(
            member,
            "must name a path beneath the request directory",
        ));
    }
    Ok(path.trim_end_matches('/').to_owned())
}

/// Read a `package` request (FR-1039 "Behavior"): every member that would
/// restate the package's records refuses before any selected file is opened;
/// then the package and the request's domain packages are read and the
/// strict reader admits the package.
pub(super) fn prepare(
    directory: &Path,
    request: &wire::Request,
    package: &wire::RunPackageSelection,
    dependent_bytes: usize,
) -> Result<PreparedPackageRun> {
    validate_request(request)?;
    if package.source_package.is_some() {
        return Err(invalid(
            "package.source_package",
            "source_package is only selected with package export",
        ));
    }
    for (present, member) in [
        (request.clause.is_some(), "clause"),
        (request.items.is_some(), "items"),
    ] {
        if present {
            return Err(invalid(
                member,
                "running this member over a selected checked package is not implemented yet",
            ));
        }
    }
    if let Some(member) = &request.tl_lowering {
        return prepare_lowering(directory, request, package, member, dependent_bytes);
    }
    let Some(call) = &request.call else {
        return Err(invalid("call", "a package run names its call"));
    };
    if call.requirements.is_some() {
        return Err(invalid(
            "call.requirements",
            "checked-package requirement items need the V2 requirement record from QSL-2543",
        ));
    }
    let mut spine_call = super::read_call(call, request)?;
    let admitted = admit(
        directory,
        request,
        package,
        dependent_bytes,
        call.snapshot.as_deref(),
    )?;
    if let Some(bytes) = &admitted.snapshot {
        let limits = fsl_replay::spine::ObservationLimitsWire::resolve(None)
            .map_err(|error| invalid(&error.member, error.reason))?;
        let observation =
            fsl_replay::spine::SuppliedObservation::from_document("snapshot", bytes.clone())
                .map_err(|error| invalid(&error.member, error.reason))?;
        spine_call.snapshot = Some(fsl_replay::spine::CallSnapshot {
            observation,
            limits,
        });
    }
    Ok(PreparedPackageRun {
        package: admitted.package,
        identities: admitted.identities,
        equality: admitted.equality,
        budget: admitted.budget,
        packages: admitted.packages,
        model_limits: admitted.model_limits,
        work: Work::Call(Box::new(spine_call)),
        selection: admitted.selection,
        models: admitted.models,
    })
}

/// FR-1039: prepare a `tl_lowering` request over a selected package. Its
/// member decodes as the source route decodes it; a `cross_evaluate` builds
/// states over the package's model world, so it needs the handoff's rows
/// and a `{file}` selection refuses it before any file is opened.
fn prepare_lowering(
    directory: &Path,
    request: &wire::Request,
    package: &wire::RunPackageSelection,
    member: &serde_json::Value,
    dependent_bytes: usize,
) -> Result<PreparedPackageRun> {
    if request.call.is_some() {
        return Err(invalid(
            "tl_lowering",
            "tl_lowering, call and clause are mutually exclusive",
        ));
    }
    let lowering = super::tl::decode(member)?;
    if lowering.cross.is_some() {
        if package.file.is_some() {
            return Err(RunCause::RequiresDirectory {
                member: "tl_lowering.cross_evaluate".to_owned(),
            });
        }
        return Err(invalid(
            "tl_lowering.cross_evaluate",
            "cross-evaluating over a handoff directory is not implemented yet",
        ));
    }
    let admitted = admit(directory, request, package, dependent_bytes, None)?;
    Ok(PreparedPackageRun {
        package: admitted.package,
        identities: admitted.identities,
        equality: admitted.equality,
        budget: admitted.budget,
        packages: admitted.packages,
        model_limits: admitted.model_limits,
        work: Work::Lowering(lowering),
        selection: admitted.selection,
        models: admitted.models,
    })
}

/// Package routes cannot restate choices already held by the package.
pub(super) fn validate_request(request: &wire::Request) -> Result<()> {
    // FR-767: a package carries its recorded law, so a request-selected law
    // beside it refuses first, naming `integer_division`.
    if request.integer_division.is_some() {
        return Err(invalid(
            "integer_division",
            "a package already records its integer-division law",
        ));
    }
    if request.program.is_some() {
        return Err(invalid(
            "program",
            "a program cannot be selected beside a package",
        ));
    }
    if request.synthesis.is_some() {
        return Err(invalid(
            "synthesis",
            "package synthesis requires the S1-S3 lowering from QSL-2235",
        ));
    }
    if !request.libraries.is_empty() {
        return Err(invalid(
            "libraries",
            "a package already holds its dependency selections",
        ));
    }
    // FR-640: a package was compiled under its clock selections.
    if !request.clocks.is_empty() {
        return Err(invalid("clocks", "a package already records its clocks"));
    }
    Ok(())
}

/// Admit the selected V2 package and the source documents for its selected
/// domain packages, preserving file/directory handoff behavior.
pub(super) fn admit(
    directory: &Path,
    request: &wire::Request,
    package: &wire::RunPackageSelection,
    dependent_bytes: usize,
    snapshot: Option<&str>,
) -> Result<AdmittedPackage> {
    let (selection, handoff) = match (&package.file, &package.directory) {
        (Some(file), None) => (
            serde_json::json!({ "file": file }),
            Selected::File(beneath("package.file", file)?),
        ),
        (None, Some(handoff)) => (
            serde_json::json!({ "directory": handoff }),
            Selected::Directory(beneath("package.directory", handoff)?),
        ),
        _ => {
            return Err(invalid(
                "package",
                "expected exactly one of file and directory",
            ));
        }
    };
    if request
        .models
        .iter()
        .any(|model| !super::is_domain_package(model))
    {
        return Err(RunCause::CompleteRunSelection(
            super::CompleteRunSelection::NativeModel,
        ));
    }
    let mut intake = super::intake_with_bytes(
        directory,
        FileCounts {
            programs: match handoff {
                Selected::File(_) => 1,
                Selected::Directory(_) => 3,
            },
            models: request.models.len(),
            observations: usize::from(snapshot.is_some()),
            extensions: 0,
        },
        dependent_bytes,
    )?;
    let package_path = match &handoff {
        Selected::File(file) => file.clone(),
        Selected::Directory(handoff) => format!("{handoff}/{PACKAGE_FILE}"),
    };
    if !directory.join(&package_path).is_file() {
        return Err(invalid(
            "package",
            format!("no checked package is at {package_path}"),
        ));
    }
    let bytes = intake.file(&package_path, intake.remaining)?;
    require_contract_version(&bytes)?;
    let documents = request
        .models
        .iter()
        .enumerate()
        .map(|(index, model)| intake.source(&model.source, &format!("models[{index}].source")))
        .collect::<Result<Vec<_>>>()?;
    let supplied = fsl_semantics::model::intake::package_input(
        documents
            .iter()
            .map(|document| document.source().text().as_bytes()),
    )
    .map_err(|refusal| invalid("models", refusal.detail))?;
    let limits =
        fsl_package::V2ReadLimits::bounded(fsl_replay::spine::I2_NATIVE_JOIN_STORAGE_DEFAULT);
    let admitted =
        fsl_package::admit_checked_package(&bytes, &supplied, limits).map_err(package_refusal)?;
    // FR-1039: a `models` entry the package records no selection of.
    let identities = supplied.keys().collect::<Vec<_>>();
    if let Some((index, identity)) = identities.iter().enumerate().find(|(_, identity)| {
        !admitted
            .selected_model_packages()
            .any(|recorded| recorded == identity.as_str())
    }) {
        return Err(invalid(
            &format!("/request/models/{index}"),
            format!("the package records no model selection {identity}"),
        ));
    }
    let mut budget = limits.native_join_limits().budget();
    let (identities, equality) = match &handoff {
        Selected::File(_) => (None, None),
        Selected::Directory(handoff) => {
            read_witness(directory, &mut intake, handoff, &admitted, &mut budget)?
        }
    };
    let snapshot = snapshot
        .map(|file| intake.file(file, intake.remaining))
        .transpose()?;
    Ok(AdmittedPackage {
        package: admitted,
        identities,
        equality,
        budget,
        packages: supplied,
        model_limits: fsl_semantics::model::accounting::ModelNormalizationLimits::default(),
        snapshot,
        selection,
        models: documents
            .iter()
            .map(|document| document.source().clone())
            .collect(),
    })
}

/// The form of a `package` selection.
enum Selected {
    File(String),
    Directory(String),
}

/// FR-1039 "A package that cannot run": a document whose `contract_version`
/// is not the v2 contract is refused before the strict reader runs.
fn require_contract_version(bytes: &[u8]) -> Result<()> {
    #[derive(serde::Deserialize)]
    struct Contract {
        contract_version: Option<String>,
    }
    // A document that is not a JSON object is the strict reader's refusal.
    let Ok(Contract {
        contract_version: Some(version),
    }) = serde_json::from_slice::<Contract>(bytes)
    else {
        return Ok(());
    };
    if version == CONTRACT_VERSION {
        return Ok(());
    }
    Err(RunCause::AnalyzePackage {
        code: fsl_foundation::Code::InvalidPackage,
        cause: Some("unknown-contract-version".to_owned()),
        pointer: Some("/request/package/contract_version".to_owned()),
        message: format!(
            "invalid_package/unknown-contract-version at /request/package/contract_version: \
             {version} is not {CONTRACT_VERSION}"
        ),
    })
}

/// The handoff directory's identity rows, and its equality rows when the
/// producer wrote them (FR-850 "Admission", FR-531).
fn read_witness(
    directory: &Path,
    intake: &mut super::Intake<'_>,
    handoff: &str,
    package: &filament_contract_model::CheckedPackageV2,
    budget: &mut filament_contract_model::NativeJoinBudget,
) -> Result<(
    Option<fsl_package::ReadIdentityRows>,
    Option<fsl_package::ReadEqualityRows>,
)> {
    let witness = |file: &str, detail: String| RunCause::AnalyzePackage {
        code: fsl_foundation::Code::InvalidPackage,
        cause: Some("native-witness".to_owned()),
        pointer: Some("/request/package/directory".to_owned()),
        message: format!("invalid_package/native-witness: {file}: {detail}"),
    };
    let identities_path = format!("{handoff}/{IDENTITIES_FILE}");
    if !directory.join(&identities_path).is_file() {
        return Err(invalid(
            "package",
            format!("the handoff directory {handoff} holds no {IDENTITIES_FILE}"),
        ));
    }
    let identity_bytes = intake.file(&identities_path, intake.remaining)?;
    let identities = fsl_package::ReadIdentityRows::read(&identity_bytes, package, budget)
        .map_err(|refusal| witness(IDENTITIES_FILE, refusal.to_string()))?;
    let equality_path = format!("{handoff}/{EQUALITY_FILE}");
    let equality = if directory.join(&equality_path).is_file() {
        let equality_bytes = intake.file(&equality_path, intake.remaining)?;
        Some(
            fsl_package::ReadEqualityRows::read(&equality_bytes, package)
                .map_err(|refusal| witness(EQUALITY_FILE, refusal.to_string()))?,
        )
    } else {
        None
    };
    Ok((Some(identities), equality))
}

/// Run a prepared package call (FR-945): build the bare package over the
/// admitted bytes and its witness rows, call the function and render the
/// result as the source route renders it.
pub(super) fn execute(prepared: PreparedPackageRun) -> Result<RunResult> {
    let PreparedPackageRun {
        package,
        identities,
        equality,
        mut budget,
        packages,
        model_limits,
        work,
        selection,
        models,
    } = prepared;
    // A `{file}` selection carries no handoff rows: its call reads only the
    // declarations the package's own nodes give (FR-1039 "a `call` runs from
    // either form").
    let identities = identities.unwrap_or_else(|| {
        fsl_package::ReadIdentityRows::of_rows(
            &package,
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
        )
    });
    let bare = fsl_package::BarePackage::new(&package)
        .with_supply(
            fsl_package::BareSupply {
                identities: &identities,
                equality: equality.as_ref(),
            },
            &mut budget,
        )
        .map_err(|failure| {
            let setting = failure
                .setting()
                .map_or_else(|| "i2.work_units".to_owned(), |setting| setting.to_string());
            RunCause::AnalyzePackage {
                code: fsl_foundation::Code::StageLimitExceeded,
                cause: None,
                pointer: None,
                message: format!("stage_limit_exceeded: {setting}: {failure:?}"),
            }
        })?;
    let call = match work {
        Work::Call(call) => *call,
        Work::Lowering(lowering) => {
            let record = super::tl::lower_over_package(&bare, &lowering)?;
            return Ok(RunResult {
                exit_code: record.exit_code,
                value: super::NativeResult::lowering_document(super::tl::document(record.record)),
                outcome: None,
                requirements: None,
                sources: crate::Sources::from_sources(models),
            });
        }
    };
    let objects = if let Some(snapshot) = &call.snapshot {
        let provision = std::collections::BTreeMap::from([(
            snapshot.observation.reference.clone(),
            snapshot.observation.bytes.clone(),
        )]);
        bare.admit_call_snapshot(
            &packages,
            model_limits,
            &provision,
            &snapshot.observation.reference,
            snapshot.limits,
        )
        .map(|(objects, _)| objects)
        .map_err(|failure| {
            use fsl_semantics::model::observation::AdmissionFailure;
            match failure {
                AdmissionFailure::Refused(record) | AdmissionFailure::Incomplete(record) => {
                    RunCause::SpineRun(Box::new(fsl_replay::spine::RunRefusal::ObjectEnvironment(
                        record,
                    )))
                }
                AdmissionFailure::Fault(fault) => {
                    RunCause::SpineRun(Box::new(fsl_replay::spine::RunRefusal::Fault(fault)))
                }
            }
        })?
    } else {
        fsl_semantics::model::object_environment::ObjectEnvironment::default()
    };
    let outcome =
        fsl_replay::spine::run_package(&bare, &call, &objects, &filament_exact::Cancel::new())
            .map_err(|refusal| RunCause::SpineRun(Box::new(*refusal)))?;
    let mut result = output::spine_run_result_for_package(&selection, &call.function, outcome)?;
    result.sources = crate::Sources::from_sources(models);
    Ok(result)
}
