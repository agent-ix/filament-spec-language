// SPDX-License-Identifier: AGPL-3.0-or-later
//! The strict `filament.checked-package/v2` admission a request that selects a
//! package by its bytes uses (FR-027, FR-850): the bytes are read by IR's
//! reader with the request's supplied domain package documents as evidence,
//! with no fallback to source compilation.

use std::collections::BTreeMap;

use filament_contract_model::{
    read_checked_package, CheckedPackageDispatchResult, CheckedPackageEvidence,
    CheckedPackageRefusalCode, CheckedPackageV2,
};
use fsl_foundation::diagnostic::Code;
use fsl_semantics::value::DefinitionRole;

use crate::checked_v2::V2ReadLimits;

/// A package the strict reader refused.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PackageAdmissionRefusal {
    /// The catalogued code: `missing_import` where no document is supplied
    /// under a selected domain package identity, `resource_exhausted` where
    /// a read ceiling was reached, `invalid_package` for every other refusal.
    pub code: Code,
    /// The reader's typed cause tag, such as `missing-selection`.
    pub cause: Option<String>,
    /// The RFC 6901 pointer the reader names, absent for a byte-stream defect.
    pub pointer: Option<String>,
    /// The domain package identity a `missing-selection` refusal is for.
    pub identity: Option<String>,
}

/// Read `bytes` as a `filament.checked-package/v2` artifact, with `documents`
/// (domain package documents by their declared identity) as the evidence its
/// model selections are read against.
///
/// # Errors
///
/// [`PackageAdmissionRefusal`] for bytes the reader refuses, an unsupplied
/// selected domain package, a supplied document its selection does not admit,
/// or a read ceiling reached.
pub fn admit_checked_package(
    bytes: &[u8],
    documents: &BTreeMap<String, Vec<u8>>,
    limits: V2ReadLimits,
) -> Result<CheckedPackageV2, PackageAdmissionRefusal> {
    let mut evidence = CheckedPackageEvidence::new();
    // The one feature FSL emits and so the one it reads: a package that
    // requires another is refused by the reader.
    evidence.support_feature(DefinitionRole::Root.identity());
    for (identity, document) in documents {
        if evidence
            .insert_domain_package_document(identity.as_str(), document.as_slice())
            .is_err()
        {
            return Err(PackageAdmissionRefusal {
                code: Code::InvalidPackage,
                cause: Some("conflicting-definition".to_owned()),
                pointer: None,
                identity: Some(identity.clone()),
            });
        }
    }
    match read_checked_package(bytes, limits.for_ir(), &evidence) {
        CheckedPackageDispatchResult::AdmittedV2(package) => Ok(*package),
        CheckedPackageDispatchResult::Refused(refusal) => {
            let pointer = refusal
                .loci
                .first()
                .and_then(|locus| locus.path.as_ref())
                .map(|path| path.as_str().to_owned());
            let cause = refusal.cause.and_then(|cause| {
                serde_json::to_value(cause)
                    .ok()
                    .and_then(|value| value.as_str().map(str::to_owned))
            });
            let missing = refusal.code == CheckedPackageRefusalCode::MissingImport;
            let identity = if missing {
                pointer
                    .as_deref()
                    .and_then(|pointer| identity_at(bytes, pointer))
            } else {
                None
            };
            Err(PackageAdmissionRefusal {
                code: if missing {
                    Code::MissingImport
                } else {
                    Code::InvalidPackage
                },
                cause,
                pointer,
                identity,
            })
        }
        CheckedPackageDispatchResult::Incomplete(incomplete) => Err(PackageAdmissionRefusal {
            code: Code::ResourceExhausted,
            cause: None,
            pointer: incomplete.path.map(|path| path.as_str().to_owned()),
            identity: None,
        }),
        CheckedPackageDispatchResult::CatalogUnavailable(_) => Err(PackageAdmissionRefusal {
            code: Code::InvalidPackage,
            cause: None,
            pointer: None,
            identity: None,
        }),
    }
}

/// The domain package identity the value at `pointer` of the package JSON
/// names: the value itself when it is a string, else its `identity` member.
fn identity_at(bytes: &[u8], pointer: &str) -> Option<String> {
    let document: serde_json::Value = serde_json::from_slice(bytes).ok()?;
    let value = document.pointer(pointer)?;
    value
        .as_str()
        .or_else(|| value.get("identity").and_then(serde_json::Value::as_str))
        .map(str::to_owned)
}
