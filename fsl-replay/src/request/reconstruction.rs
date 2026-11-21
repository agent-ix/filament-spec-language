// SPDX-License-Identifier: AGPL-3.0-or-later
//! Source-free admission of genuine reconstruction inputs. Family evidence is
//! bound only after its checked graph exists; no placeholder source is made.

use super::{
    decode_target, BackendId, BoundExceeded, ByteProvision, CheckedRefinementInternal,
    DeclaredDomain, DefinitionSourceRef, DependencyEntry, DependencyEntryWire, InputMeasurement,
    ObjectEnvironmentRef, ObligationIdentity, ProfileSelection, Provision, ReplayLimits,
    ReplayOverBinding, ReplayPackage, ReplayRequest, ReplayRequestRefusal, ReplayRequestWire,
    ReplaySource, ReplayTarget, RequestTargetWire, ScalarLimits, SettingName, StageLimits,
    StateEnvironment, SuppliedLimits, TargetMember,
};
use std::collections::BTreeMap;

/// Actual common replay inputs before a family-owned source is read back.
#[derive(Clone)]
pub struct ReplayReconstructionWire {
    /// Complete original raw package.
    pub package: ReplayPackage,
    /// Admitted profile selections.
    pub profile_selections: Vec<ProfileSelection>,
    /// Genuine dependency selection/source locks.
    pub dependencies: Vec<DependencyEntryWire>,
    /// Actual target members.
    pub replay_target: RequestTargetWire,
    /// Original checked obligation.
    pub obligation_identity: ObligationIdentity,
    /// Original provider identity.
    pub backend: String,
    /// Original scalar environment.
    pub state_environment: StateEnvironment,
    /// Original proving accounting.
    pub accounting_limits: ScalarLimits,
    /// Original proving-stage settings.
    pub stage_limits: BTreeMap<String, u64>,
    /// Original supplied-value ceilings.
    pub supplied_limits: Option<SuppliedLimits>,
    /// Actual selected object environment.
    pub object_environment: Option<ObjectEnvironmentRef>,
    /// Actual original domains.
    pub declared_domains: Vec<DeclaredDomain>,
    /// Actual original source bytes.
    pub byte_provision: Vec<(String, String, Vec<u8>)>,
    /// Actual original document bytes.
    pub documents: Vec<Vec<u8>>,
}

impl ReplayReconstructionWire {
    /// Attach the actual read-back family source and certificate. The result
    /// is still a raw descriptor, requiring native request admission.
    pub fn with_source(
        self,
        source: ReplaySource,
        certificate: Option<serde_json::Value>,
    ) -> ReplayRequestWire {
        ReplayRequestWire {
            package: self.package,
            profile_selections: self.profile_selections,
            dependencies: self.dependencies,
            replay_target: self.replay_target,
            source,
            certificate,
            obligation_identity: self.obligation_identity,
            backend: self.backend,
            state_environment: self.state_environment,
            accounting_limits: self.accounting_limits,
            stage_limits: self.stage_limits,
            supplied_limits: self.supplied_limits,
            object_environment: self.object_environment,
            declared_domains: self.declared_domains,
            byte_provision: self.byte_provision,
            documents: self.documents,
        }
    }

    pub(super) fn effective_accounting_limits(&self) -> ScalarLimits {
        crate::limits::AccountingLimits::with_entries(
            self.accounting_limits,
            self.stage_limits.iter().filter_map(|(name, bound)| {
                match SettingName::from_name(name) {
                    Some(SettingName::Accounting(setting)) => Some((setting, *bound)),
                    _ => None,
                }
            }),
        )
    }
}

impl ReplayRequestWire {
    pub(super) fn into_reconstruction(
        self,
    ) -> (
        ReplayReconstructionWire,
        ReplaySource,
        Option<serde_json::Value>,
    ) {
        (
            ReplayReconstructionWire {
                package: self.package,
                profile_selections: self.profile_selections,
                dependencies: self.dependencies,
                replay_target: self.replay_target,
                obligation_identity: self.obligation_identity,
                backend: self.backend,
                state_environment: self.state_environment,
                accounting_limits: self.accounting_limits,
                stage_limits: self.stage_limits,
                supplied_limits: self.supplied_limits,
                object_environment: self.object_environment,
                declared_domains: self.declared_domains,
                byte_provision: self.byte_provision,
                documents: self.documents,
            },
            self.source,
            self.certificate,
        )
    }
}

/// Admitted common inputs for one source/package reconstruction, before the
/// actual family evidence is bound. This grants no source or verdict authority.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReplayReconstructionInput {
    pub(super) encoded_input_bytes: usize,
    pub(super) package: ReplayPackage,
    pub(super) package_name: String,
    pub(super) sources: Vec<DefinitionSourceRef>,
    pub(super) refinement_internals: Option<Vec<CheckedRefinementInternal>>,
    pub(super) dependencies: Vec<DependencyEntry>,
    pub(super) replay_target: Option<ReplayTarget>,
    pub(super) over: Option<ReplayOverBinding>,
    pub(super) profile_selections: Vec<ProfileSelection>,
    pub(super) obligation_identity: ObligationIdentity,
    pub(super) backend: BackendId,
    pub(super) state_environment: StateEnvironment,
    pub(super) accounting_limits: ScalarLimits,
    pub(super) stage_limits: StageLimits,
    pub(super) supplied_limits: SuppliedLimits,
    pub(super) object_environment: Option<ObjectEnvironmentRef>,
    pub(super) declared_domains: Vec<DeclaredDomain>,
    pub(super) byte_provision: ByteProvision,
    pub(super) replay_limits: ReplayLimits,
}

impl ReplayReconstructionInput {
    /// Bind actual evidence after graph reconstruction and run the existing
    /// native request's source/target/certificate checks without re-running a
    /// compiler or changing any proving member.
    pub fn bind_source(
        self,
        source: ReplaySource,
        certificate: Option<serde_json::Value>,
    ) -> Result<ReplayRequest, ReplayRequestRefusal> {
        let limits = self.replay_limits;
        ReplayRequest::decode(self.bind(source, certificate).to_wire(), limits)
    }

    pub(crate) fn stage_limits(&self) -> &StageLimits {
        &self.stage_limits
    }
    /// Admit the original common request members and genuine byte supply.
    pub fn decode(
        mut wire: ReplayReconstructionWire,
        limits: ReplayLimits,
    ) -> Result<Self, ReplayRequestRefusal> {
        let encoded_input_bytes = InputMeasurement::of_common(&wire).bytes();
        BoundExceeded::check(encoded_input_bytes, limits)?;
        let over = wire.replay_target.over.clone();
        let target = decode_target(std::mem::take(&mut wire.replay_target))?;
        if over.is_some() && target.member() != TargetMember::StateClause {
            return Err(ReplayRequestRefusal::UnexpectedOverBinding);
        }
        Self::decode_after_target(
            wire,
            limits,
            Provision::Sources,
            encoded_input_bytes,
            Some(target),
            over,
        )
    }

    pub(super) fn bind(
        self,
        source: ReplaySource,
        certificate: Option<serde_json::Value>,
    ) -> ReplayRequest {
        ReplayRequest {
            encoded_input_bytes: self.encoded_input_bytes,
            package: self.package,
            package_name: self.package_name,
            sources: self.sources,
            refinement_internals: self.refinement_internals,
            dependencies: self.dependencies,
            replay_target: self.replay_target,
            over: self.over,
            source,
            hybrid_certificate: certificate,
            profile_selections: self.profile_selections,
            obligation_identity: self.obligation_identity,
            backend: self.backend,
            state_environment: self.state_environment,
            accounting_limits: self.accounting_limits,
            stage_limits: self.stage_limits,
            supplied_limits: self.supplied_limits,
            object_environment: self.object_environment,
            declared_domains: self.declared_domains,
            byte_provision: self.byte_provision,
            replay_limits: self.replay_limits,
        }
    }
}

/// Common data read by recompilation, independent of family evidence.
pub(crate) trait RecompilationInput {
    fn package(&self) -> &ReplayPackage;
    fn package_name(&self) -> &str;
    fn sources(&self) -> &[DefinitionSourceRef];
    fn dependencies(&self) -> &[DependencyEntry];
    fn refinement_internals(&self) -> Option<&[CheckedRefinementInternal]>;
    fn byte_provision(&self) -> &ByteProvision;
    fn check_external_package_bytes(
        &self,
        lengths: impl IntoIterator<Item = usize>,
    ) -> Result<(), ReplayRequestRefusal>;
}

macro_rules! common_recompilation_input {
    ($type:ty) => {
        impl RecompilationInput for $type {
            fn package(&self) -> &ReplayPackage {
                &self.package
            }
            fn package_name(&self) -> &str {
                &self.package_name
            }
            fn sources(&self) -> &[DefinitionSourceRef] {
                &self.sources
            }
            fn dependencies(&self) -> &[DependencyEntry] {
                &self.dependencies
            }
            fn refinement_internals(&self) -> Option<&[CheckedRefinementInternal]> {
                self.refinement_internals.as_deref()
            }
            fn byte_provision(&self) -> &ByteProvision {
                &self.byte_provision
            }
            fn check_external_package_bytes(
                &self,
                lengths: impl IntoIterator<Item = usize>,
            ) -> Result<(), ReplayRequestRefusal> {
                let actual = lengths
                    .into_iter()
                    .fold(self.encoded_input_bytes as u128, |total, length| {
                        total.saturating_add(length as u128)
                    });
                if actual > u128::from(self.replay_limits.input_bytes) {
                    return Err(BoundExceeded {
                        bound: self.replay_limits.input_bytes,
                        actual,
                    }
                    .into());
                }
                Ok(())
            }
        }
    };
}
common_recompilation_input!(ReplayRequest);
common_recompilation_input!(ReplayReconstructionInput);
