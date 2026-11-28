// SPDX-License-Identifier: AGPL-3.0-or-later
//! The strict `filament.checked-package/v2` admission a request that selects a
//! package by its bytes uses (FR-027, FR-850): the bytes are read by IR's
//! reader with the request's supplied domain package documents as evidence,
//! with no fallback to source compilation.

use std::collections::BTreeMap;
use std::sync::Arc;

use filament_contract_model::{
    read_checked_package, read_checked_package_with_catalog, CheckedArtifactRef,
    CheckedPackageDispatchResult, CheckedPackageEvidence, CheckedPackageRefusal,
    CheckedPackageRefusalCause, CheckedPackageRefusalCode, CheckedPackageV2,
};
use fsl_foundation::diagnostic::{Code, StageFailure};
use fsl_semantics::value::DefinitionRole;
use ix_trace_rs::implements;

use crate::checked_v2::{ImportViewRefusal, V2ReadLimits, V2ReadRefusal};

/// A package the strict reader refused.
#[derive(Clone, Debug, Eq, PartialEq)]
#[implements("FR-110-AC-16")]
pub struct PackageAdmissionRefusal {
    /// Original cumulative read-budget denial, when admission exhausted it.
    pub limit: Option<Box<fsl_foundation::diagnostic::LimitExceeded>>,
    /// The catalogued code: `missing_import` where no document is supplied
    /// under a selected domain package identity, `resource_exhausted` where
    /// a read ceiling was reached, `stage_limit_exceeded` for a shared-budget
    /// denial, and `invalid_package` for every other refusal.
    pub code: Code,
    /// The reader's typed cause tag, such as `missing-selection`.
    pub cause: Option<CheckedPackageRefusalCause>,
    /// The RFC 6901 pointer the reader names, absent for a byte-stream defect.
    pub pointer: Option<String>,
    /// The domain or library identity a `missing-selection` refusal names.
    pub identity: Option<String>,
    /// A boxed host allocation failure the strict reader admitted before the
    /// host refused. This is a tool failure and carries no package refusal
    /// data; boxing keeps the common package-refusal representation small.
    pub host_allocation_failure:
        Option<Box<filament_contract_model::CheckedPackageHostAllocationFailed>>,
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
    admit_checked_package_with_dependencies(bytes, documents, &BTreeMap::new(), limits)
}

/// Admit a checked package against the caller's extension catalog as well as
/// the supplied domain-package evidence.
#[implements("FR-110-AC-16")]
pub fn admit_checked_package_with_catalog(
    bytes: &[u8],
    documents: &BTreeMap<String, Vec<u8>>,
    extensions: &fsl_semantics::library::bundle::DefinitionCatalog,
    limits: V2ReadLimits,
) -> Result<CheckedPackageV2, PackageAdmissionRefusal> {
    let evidence = checked_package_evidence(documents, &BTreeMap::new())?;
    let catalog = checked_package_definition_catalog(extensions);
    match read_checked_package_with_catalog(bytes, limits.for_ir(), &evidence, &catalog) {
        CheckedPackageDispatchResult::AdmittedV2(package) => {
            if let Some(refusal) = crate::bare_synthesis::plan_candidate_refusal(&package) {
                return Err(of_reader_refusal(&refusal));
            }
            Ok(*package)
        }
        CheckedPackageDispatchResult::Refused(mut refusal) => {
            crate::bare_synthesis::locate_plan_candidate_refusal(bytes, &mut refusal);
            Err(of_reader_refusal(&refusal).naming_identity_in(Some(bytes)))
        }
        CheckedPackageDispatchResult::Incomplete(incomplete) => {
            Err(reader_ceiling_refusal(incomplete))
        }
        CheckedPackageDispatchResult::CatalogUnavailable(_) => Err(unavailable_catalog_refusal()),
        CheckedPackageDispatchResult::HostAllocationFailed(failure) => {
            Err(host_allocation_failed(failure))
        }
    }
}

/// Admit a package with authenticated direct library artifacts. The strict
/// reader verifies the supplied identities and retains their shared closure.
///
/// # Errors
/// Returns the original strict reader refusal or a conflicting evidence refusal.
pub fn admit_checked_package_with_dependencies(
    bytes: &[u8],
    documents: &BTreeMap<String, Vec<u8>>,
    dependencies: &BTreeMap<String, Arc<CheckedPackageV2>>,
    limits: V2ReadLimits,
) -> Result<CheckedPackageV2, PackageAdmissionRefusal> {
    let evidence = checked_package_evidence(documents, dependencies)?;
    match read_checked_package(bytes, limits.for_ir(), &evidence) {
        CheckedPackageDispatchResult::AdmittedV2(package) => {
            if let Some(refusal) = crate::bare_synthesis::plan_candidate_refusal(&package) {
                return Err(of_reader_refusal(&refusal));
            }
            Ok(*package)
        }
        CheckedPackageDispatchResult::Refused(mut refusal) => {
            crate::bare_synthesis::locate_plan_candidate_refusal(bytes, &mut refusal);
            Err(of_reader_refusal(&refusal).naming_identity_in(Some(bytes)))
        }
        CheckedPackageDispatchResult::Incomplete(incomplete) => {
            Err(reader_ceiling_refusal(incomplete))
        }
        CheckedPackageDispatchResult::CatalogUnavailable(_) => Err(PackageAdmissionRefusal {
            limit: None,
            code: Code::InvalidPackage,
            cause: None,
            pointer: None,
            identity: None,
            host_allocation_failure: None,
        }),
        CheckedPackageDispatchResult::HostAllocationFailed(failure) => {
            Err(host_allocation_failed(failure))
        }
    }
}

/// A read ceiling the strict reader reached: `resource_exhausted` at the
/// node it stopped on, naming the limit, its bound, the count reached and the
/// `i2.*` setting that raises it (FR-255 Behavior 1, QSL-3543).
fn reader_ceiling_refusal(
    incomplete: filament_contract_model::CheckedPackageIncomplete,
) -> PackageAdmissionRefusal {
    PackageAdmissionRefusal {
        limit: Some(Box::new(fsl_foundation::diagnostic::LimitExceeded::new(
            crate::checked_v2::limit_setting(incomplete.limit_kind),
            incomplete.limit,
            u128::from(incomplete.consumed),
        ))),
        code: Code::ResourceExhausted,
        cause: None,
        pointer: incomplete.path.map(|path| path.as_str().to_owned()),
        identity: None,
        host_allocation_failure: None,
    }
}

/// The host refused an allocation the strict read had admitted: no judgement
/// of the package.
fn host_allocation_failed(
    failure: filament_contract_model::CheckedPackageHostAllocationFailed,
) -> PackageAdmissionRefusal {
    PackageAdmissionRefusal {
        limit: None,
        code: Code::RuntimeInvariant,
        cause: None,
        pointer: None,
        identity: None,
        host_allocation_failure: Some(Box::new(failure)),
    }
}

/// Strict admission consuming the invocation's shared work allowance.
///
/// # Errors
/// Preserves strict-reader refusals and cumulative typed budget denials.
pub fn admit_checked_package_with_budget(
    bytes: &[u8],
    documents: &BTreeMap<String, Vec<u8>>,
    dependencies: &BTreeMap<String, Arc<CheckedPackageV2>>,
    limits: V2ReadLimits,
    budget: &mut filament_contract_model::NativeJoinBudget,
) -> Result<CheckedPackageV2, PackageAdmissionRefusal> {
    let evidence = checked_package_evidence(documents, dependencies)?;
    admit_checked_package_with_evidence(bytes, &evidence, limits, budget)
}

/// Read against invocation-owned evidence without copying domain documents or
/// dependency graphs at each package boundary. The strict reader retains only
/// the dependencies its lock selects.
///
/// # Errors
/// Preserves strict-reader refusals and cumulative typed budget denials.
#[implements("FR-110-AC-16")]
pub fn admit_checked_package_with_evidence(
    bytes: &[u8],
    evidence: &CheckedPackageEvidence,
    limits: V2ReadLimits,
    budget: &mut filament_contract_model::NativeJoinBudget,
) -> Result<CheckedPackageV2, PackageAdmissionRefusal> {
    use filament_contract_model::CheckedPackageBudgetedDispatchResult as Read;
    let denial = |limit, actual, pointer| PackageAdmissionRefusal {
        limit: Some(Box::new(fsl_foundation::diagnostic::LimitExceeded::new(
            fsl_foundation::Setting::I2WorkUnits,
            limit,
            actual,
        ))),
        code: Code::StageLimitExceeded,
        cause: Some(CheckedPackageRefusalCause::InsufficientNextCharge),
        pointer,
        identity: None,
        host_allocation_failure: None,
    };
    match filament_contract_model::read_checked_package_with_budget(
        bytes,
        limits.for_ir(),
        evidence,
        budget,
    ) {
        Read::AdmittedV2(package) => {
            if let Some(refusal) = crate::bare_synthesis::plan_candidate_refusal(&package) {
                return Err(of_reader_refusal(&refusal));
            }
            Ok(*package)
        }
        Read::Refused(mut refusal) => {
            crate::bare_synthesis::locate_plan_candidate_refusal(bytes, &mut refusal);
            Err(of_reader_refusal(&refusal).naming_identity_in(Some(bytes)))
        }
        Read::BudgetIncomplete(incomplete) => Err(denial(
            incomplete.limit,
            u128::from(incomplete.consumed),
            incomplete.path.map(|path| path.as_str().to_owned()),
        )),
        Read::Exhausted(exhausted) => Err(denial(
            exhausted.configured_bound,
            u128::from(budget.consumed().work).saturating_add(exhausted.denied.amount),
            None,
        )),
        Read::Incomplete(incomplete) => Err(reader_ceiling_refusal(incomplete)),
        Read::CatalogUnavailable(_) => Err(PackageAdmissionRefusal {
            limit: None,
            code: Code::InvalidPackage,
            cause: None,
            pointer: None,
            identity: None,
            host_allocation_failure: None,
        }),
        Read::HostAllocationFailed(failure) => Err(host_allocation_failed(failure)),
    }
}

/// Strict admission against invocation-owned evidence and an extension
/// catalog, charging the catalog fallback to the same caller-owned budget.
#[implements("FR-110-AC-16")]
pub fn admit_checked_package_with_evidence_and_catalog(
    bytes: &[u8],
    evidence: &CheckedPackageEvidence,
    extensions: &fsl_semantics::library::bundle::DefinitionCatalog,
    limits: V2ReadLimits,
    budget: &mut filament_contract_model::NativeJoinBudget,
) -> Result<CheckedPackageV2, PackageAdmissionRefusal> {
    use filament_contract_model::CheckedPackageBudgetedDispatchResult as Read;
    let denial = |limit, actual, pointer| PackageAdmissionRefusal {
        limit: Some(Box::new(fsl_foundation::diagnostic::LimitExceeded::new(
            fsl_foundation::Setting::I2WorkUnits,
            limit,
            actual,
        ))),
        code: Code::StageLimitExceeded,
        cause: Some("insufficient-next-charge".to_owned()),
        pointer,
        identity: None,
        host_allocation_failure: None,
    };
    let catalog = checked_package_definition_catalog(extensions);
    match filament_contract_model::read_checked_package_with_catalog_and_budget(
        bytes,
        limits.for_ir(),
        evidence,
        &catalog,
        budget,
    ) {
        Read::AdmittedV2(package) => {
            if let Some(refusal) = crate::bare_synthesis::plan_candidate_refusal(&package) {
                return Err(of_reader_refusal(&refusal));
            }
            Ok(*package)
        }
        Read::Refused(mut refusal) => {
            crate::bare_synthesis::locate_plan_candidate_refusal(bytes, &mut refusal);
            Err(of_reader_refusal(&refusal).naming_identity_in(Some(bytes)))
        }
        Read::BudgetIncomplete(incomplete) => Err(denial(
            incomplete.limit,
            u128::from(incomplete.consumed),
            incomplete.path.map(|path| path.as_str().to_owned()),
        )),
        Read::Exhausted(exhausted) => Err(denial(
            exhausted.configured_bound,
            u128::from(budget.consumed().work).saturating_add(exhausted.denied.amount),
            None,
        )),
        Read::Incomplete(incomplete) => Err(reader_ceiling_refusal(incomplete)),
        Read::CatalogUnavailable(_) => Err(unavailable_catalog_refusal()),
        Read::HostAllocationFailed(failure) => Err(host_allocation_failed(failure)),
    }
}

#[implements("FR-110-AC-16")]
fn checked_package_definition_catalog(
    extensions: &fsl_semantics::library::bundle::DefinitionCatalog,
) -> filament_contract_model::DefinitionCatalog {
    let mut catalog = filament_contract_model::DefinitionCatalog::new();
    // These built-in selections are already classified by FSL's header and
    // qualification rules. Keep them out of IR's unknown/profile law check;
    // operation-specific value laws are admitted from the checked graph.
    for layer in fsl_semantics::value::layer::HeaderLayer::ALL {
        catalog.insert_selection(CheckedArtifactRef {
            authority: "agent-ix".into(),
            identity: layer.identity().into(),
        });
    }
    for role in fsl_semantics::value::always_roles()
        .into_iter()
        .chain([DefinitionRole::TextProfile, DefinitionRole::IeeeProfile])
    {
        catalog.insert_selection(CheckedArtifactRef {
            authority: "agent-ix".into(),
            identity: role.identity().into(),
        });
    }
    for definition in extensions.iter() {
        if !definition.is_extension() {
            continue;
        }
        let reference = CheckedArtifactRef {
            authority: definition.exact().authority().into(),
            identity: definition.exact().identity().into(),
        };
        catalog.insert_extension(reference);
    }
    catalog
}

fn unavailable_catalog_refusal() -> PackageAdmissionRefusal {
    PackageAdmissionRefusal {
        limit: None,
        code: Code::InvalidPackage,
        cause: None,
        pointer: None,
        identity: None,
        host_allocation_failure: None,
    }
}

/// Own the invocation's domain documents once and supply authenticated library
/// values by shared ownership. Additional admitted libraries can be inserted
/// through IR's existing evidence API before subsequent reads.
///
/// # Errors
/// Refuses conflicting documents or library identities through IR's insertion API.
#[implements("FR-110-AC-16")]
pub fn checked_package_evidence(
    documents: &BTreeMap<String, Vec<u8>>,
    dependencies: &BTreeMap<String, Arc<CheckedPackageV2>>,
) -> Result<CheckedPackageEvidence, PackageAdmissionRefusal> {
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
                limit: None,
                code: Code::InvalidPackage,
                cause: Some(CheckedPackageRefusalCause::ConflictingDefinition),
                pointer: None,
                identity: Some(identity.clone()),
                host_allocation_failure: None,
            });
        }
    }
    for (identity, dependency) in dependencies {
        evidence
            .insert_dependency_package(identity.as_str(), Arc::clone(dependency))
            .map_err(|_| PackageAdmissionRefusal {
                limit: None,
                code: Code::InvalidPackage,
                cause: Some(CheckedPackageRefusalCause::ConflictingDefinition),
                pointer: None,
                identity: Some(identity.clone()),
                host_allocation_failure: None,
            })?;
    }
    Ok(evidence)
}

/// The refusal IR's reader raised, with its pointer and typed cause. The
/// identity at fault is the caller's to read from the bytes.
#[ix_trace_rs::string_edge]
#[implements("FR-110-AC-16")]
fn of_reader_refusal(refusal: &CheckedPackageRefusal) -> PackageAdmissionRefusal {
    let pointer = refusal
        .loci
        .first()
        .and_then(|locus| locus.path.as_ref())
        .map(|path| path.as_str().to_owned());
    let cause = refusal.cause;
    // FR-940 "Claim names": a name shared by two `claim_names` rows is the one
    // ambiguous declaration, whichever reader found it. QSL-2545: the
    // contract-ir reader still reports `invalid_package` here; delete this
    // remap once the reader returns `ambiguous_declaration` itself.
    let ambiguous_claim = cause == Some(CheckedPackageRefusalCause::AmbiguousName)
        && pointer
            .as_deref()
            .is_some_and(|pointer| pointer.starts_with("/claim_names/"));
    let code = if refusal.code == CheckedPackageRefusalCode::MissingImport {
        Code::MissingImport
    } else if refusal.code == CheckedPackageRefusalCode::UnknownProfile {
        Code::UnknownProfile
    } else if ambiguous_claim {
        Code::AmbiguousDeclaration
    } else {
        Code::InvalidPackage
    };
    PackageAdmissionRefusal {
        limit: None,
        code,
        cause,
        pointer,
        identity: None,
        host_allocation_failure: None,
    }
}

impl PackageAdmissionRefusal {
    /// The strict reader's cause, when the reader supplied one.
    #[must_use]
    #[implements("FR-110-AC-16")]
    pub fn reader_cause(&self) -> Option<&'static str> {
        self.cause.and_then(|cause| cause_spelling(cause).ok())
    }

    /// The refusal spelled `code/cause for identity at pointer`, each part
    /// when present.
    #[must_use]
    #[implements("FR-110-AC-16")]
    pub fn label(&self) -> String {
        if let Some(failure) = self.host_allocation_failure.as_deref() {
            return format!(
                "host allocation failed at {:?}, requested {} bytes (consumed {})",
                failure.stage, failure.requested, failure.consumed
            );
        }
        let mut label = self.code.as_str().to_owned();
        if let Some(cause) = self.cause {
            if let Ok(spelling) = cause_spelling(cause) {
                label.push('/');
                label.push_str(spelling);
            }
        }
        if let Some(identity) = &self.identity {
            label.push_str(" for ");
            label.push_str(identity);
        }
        if let Some(pointer) = &self.pointer {
            label.push_str(" at ");
            label.push_str(pointer);
        }
        label
    }

    /// The cause as a catalog spelling: the reader's own cause, else the
    /// catalog default of the code (`insufficient-next-charge` for a reached
    /// ceiling, `missing-selection` for a missing import, `invalid-value`
    #[must_use]
    #[implements("FR-110-AC-16")]
    pub fn catalog_cause(&self) -> &'static str {
        let default = match self.code {
            Code::ResourceExhausted => "insufficient-next-charge",
            Code::MissingImport => "missing-selection",
            _ => "invalid-value",
        };
        self.reader_cause().unwrap_or(default)
    }
    /// The strict reader's refusal inside a failed evaluation read of a
    /// package the run compiled, with the reader's code, cause and pointer;
    /// `bytes` are the package bytes the read refused, from which a missing
    /// import names its identity. `None` for a failure that is no admission
    /// refusal: an emission that did not complete, a broken invariant, a
    /// cancellation.
    #[must_use]
    pub fn of_import_view(refusal: &ImportViewRefusal, bytes: Option<&[u8]>) -> Option<Self> {
        let ImportViewRefusal::Read { refusal, .. } = refusal else {
            return None;
        };
        let reader = |refusal: &V2ReadRefusal| match refusal {
            V2ReadRefusal::Envelope { refusal, .. } => Some(of_reader_refusal(refusal)),
            V2ReadRefusal::HostAllocationFailed(failure) => Some(host_allocation_failed(*failure)),
            V2ReadRefusal::Fault(_) => None,
            other => Some(Self {
                limit: None,
                code: other.code(),
                cause: None,
                pointer: None,
                identity: None,
                host_allocation_failure: None,
            }),
        };
        let exhausted = || Self {
            limit: None,
            code: Code::ResourceExhausted,
            cause: None,
            pointer: None,
            identity: None,
            host_allocation_failure: None,
        };
        let found = match refusal.as_ref() {
            StageFailure::Refused(refusal) => reader(refusal),
            StageFailure::RefusedWithPayload(refusal, _) => reader(refusal),
            StageFailure::Limit(_) | StageFailure::LimitWithPayload(_, _) => Some(exhausted()),
            StageFailure::Cancelled(_) | StageFailure::Fault(_) => None,
        };
        found.map(|found| found.naming_identity_in(bytes))
    }

    /// This refusal, with the identity a missing import names read from the
    /// package `bytes` at its pointer.
    fn naming_identity_in(mut self, bytes: Option<&[u8]>) -> Self {
        if self.code == Code::MissingImport {
            self.identity = self
                .pointer
                .as_deref()
                .zip(bytes)
                .and_then(|(pointer, bytes)| identity_at(bytes, pointer));
        }
        self
    }
}

#[implements("FR-110-AC-16")]
pub(crate) struct CauseSpellingSerializer;

macro_rules! reject_non_unit_variant_methods {
    ($(fn $name:ident($($argument:ident: $kind:ty),*);)+) => {
        $(
            fn $name(self, $($argument: $kind),*) -> Result<Self::Ok, Self::Error> {
                Err(serde::ser::Error::custom("expected a unit variant"))
            }
        )+
    };
}

impl serde::Serializer for CauseSpellingSerializer {
    type Ok = &'static str;
    type Error = serde_json::Error;
    type SerializeSeq = serde::ser::Impossible<Self::Ok, Self::Error>;
    type SerializeTuple = serde::ser::Impossible<Self::Ok, Self::Error>;
    type SerializeTupleStruct = serde::ser::Impossible<Self::Ok, Self::Error>;
    type SerializeTupleVariant = serde::ser::Impossible<Self::Ok, Self::Error>;
    type SerializeMap = serde::ser::Impossible<Self::Ok, Self::Error>;
    type SerializeStruct = serde::ser::Impossible<Self::Ok, Self::Error>;
    type SerializeStructVariant = serde::ser::Impossible<Self::Ok, Self::Error>;

    reject_non_unit_variant_methods! {
        fn serialize_bool(_value: bool);
        fn serialize_i8(_value: i8);
        fn serialize_i16(_value: i16);
        fn serialize_i32(_value: i32);
        fn serialize_i64(_value: i64);
        fn serialize_u8(_value: u8);
        fn serialize_u16(_value: u16);
        fn serialize_u32(_value: u32);
        fn serialize_u64(_value: u64);
        fn serialize_f32(_value: f32);
        fn serialize_f64(_value: f64);
        fn serialize_char(_value: char);
        fn serialize_str(_value: &str);
        fn serialize_bytes(_value: &[u8]);
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        Err(serde::ser::Error::custom("expected a unit variant"))
    }

    fn serialize_some<T: ?Sized + serde::Serialize>(
        self,
        _value: &T,
    ) -> Result<Self::Ok, Self::Error> {
        Err(serde::ser::Error::custom("expected a unit variant"))
    }

    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        Err(serde::ser::Error::custom("expected a unit variant"))
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok, Self::Error> {
        Err(serde::ser::Error::custom("expected a unit variant"))
    }

    #[implements("FR-110-AC-16")]
    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _index: u32,
        variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        Ok(variant)
    }

    fn serialize_newtype_struct<T: ?Sized + serde::Serialize>(
        self,
        _name: &'static str,
        _value: &T,
    ) -> Result<Self::Ok, Self::Error> {
        Err(serde::ser::Error::custom("expected a unit variant"))
    }

    fn serialize_newtype_variant<T: ?Sized + serde::Serialize>(
        self,
        _name: &'static str,
        _index: u32,
        _variant: &'static str,
        _value: &T,
    ) -> Result<Self::Ok, Self::Error> {
        Err(serde::ser::Error::custom("expected a unit variant"))
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        Err(serde::ser::Error::custom("expected a unit variant"))
    }

    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        Err(serde::ser::Error::custom("expected a unit variant"))
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        Err(serde::ser::Error::custom("expected a unit variant"))
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        Err(serde::ser::Error::custom("expected a unit variant"))
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        Err(serde::ser::Error::custom("expected a unit variant"))
    }

    fn serialize_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        Err(serde::ser::Error::custom("expected a unit variant"))
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        Err(serde::ser::Error::custom("expected a unit variant"))
    }
}

#[implements("FR-110-AC-16")]
pub(crate) fn cause_spelling(
    cause: CheckedPackageRefusalCause,
) -> Result<&'static str, serde_json::Error> {
    serde::Serialize::serialize(&cause, CauseSpellingSerializer)
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

#[cfg(test)]
mod host_allocation_tests {
    use super::*;
    use ix_trace_rs::trace;

    /// A host allocation failure during the strict read says nothing about the
    /// package: it is the catalogued `resource_exhausted`/`allocation-failed`
    /// host failure, with its stage and charged size and no package locus.
    #[trace("FR-096-AC-24")]
    #[test]
    fn a_host_allocation_failure_is_an_internal_failure_not_a_package_refusal() {
        let failure = filament_contract_model::CheckedPackageHostAllocationFailed {
            stage: filament_contract_model::PackageReadStage::Envelope,
            locus: filament_contract_model::NativeJoinLocus {
                source: filament_contract_model::NativeJoinLocusSource::PackageRead(
                    filament_contract_model::PackageReadStage::Envelope,
                    0,
                ),
                form: "package_bytes",
            },
            requested: 32,
            consumed: 64,
        };
        let refusal = host_allocation_failed(failure);
        assert_eq!(refusal.code, Code::RuntimeInvariant,);
        assert_eq!(refusal.cause, None);
        assert_eq!(refusal.pointer, None);
        assert_eq!(refusal.identity, None);
        assert_eq!(refusal.host_allocation_failure, Some(Box::new(failure)));
        assert!(refusal.label().contains("Envelope"));
        assert!(refusal.label().contains("32 bytes"));
    }
}
