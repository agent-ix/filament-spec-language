// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-571 "The protocol subject": the realizability subject built from a
//! checked package, its domain package documents and a snapshot provision,
//! for a caller that holds the package in process (for example one compiled
//! back from a checked handoff directory) and the request's bindings.

use filament_exact::ScalarLimits;
use fsl_eval::model_subject::{ModelInstance, ModelSubject, ProtocolSubject, RoleBinding};
use fsl_eval::simulation::model::{ExpansionLimits, ModelSimulationRequest, PopulationUniverse};
use fsl_eval::simulation::protocol::{MemorySelection, MemorySelectionRefusal, ResolvedMemory};
use fsl_package::{CheckedPackage, EmitLimits};
use fsl_semantics::model::accounting::ModelNormalizationLimits;
use fsl_semantics::model::intake::PackageInput;
use fsl_semantics::model::observation::{DocumentProvision, DocumentRef, ObservationLimits};

/// The package a protocol subject is over: the checked package, FR-056's
/// package input and FR-106's snapshot provision.
#[derive(Clone, Copy)]
pub struct SubjectPackage<'a> {
    /// The in-process checked package.
    pub package: &'a CheckedPackage,
    /// The domain package documents by the identity each declares.
    pub packages: &'a PackageInput,
    /// The snapshot documents the initial states name.
    pub snapshots: &'a DocumentProvision,
}

/// What a request binds in a protocol subject: the protocol clause, its
/// `over` instance and role bindings, the request's memory selection, the
/// initial states, the universes and the limits.
#[derive(Clone, Debug)]
pub struct SubjectBindings {
    /// The declared name of a protocol clause of the package.
    pub protocol: String,
    /// The object bound to the clause's `over` parameter.
    pub over: ModelInstance,
    /// A binding for each static role.
    pub roles: Vec<RoleBinding>,
    /// The request's `memory` member; an outermost `parallel` it does not
    /// name keeps its source default (FR-219).
    pub memory: Vec<MemorySelection>,
    /// The initial states: documents of the snapshot provision.
    pub initial: Vec<DocumentRef>,
    /// For each simulated population, its finite key set.
    pub universes: Vec<PopulationUniverse>,
    /// The normalization limits FR-106 admission takes.
    pub normalization: ModelNormalizationLimits,
    /// FR-106's document limits.
    pub observation: ObservationLimits,
    /// The per-expansion candidate bound.
    pub expansion: ExpansionLimits,
    /// The evaluation meter budget.
    pub meter: ScalarLimits,
    /// The ceiling on each package emission the analysis makes.
    pub emit: EmitLimits,
}

/// Why no protocol subject was built.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum SubjectRefusal {
    /// The package declares no protocol clause of the bound name.
    #[error("invalid_runtime_input/invalid-value: the package declares no protocol `{0}`")]
    UnknownProtocol(String),
    /// The memory selection does not resolve over the protocol.
    #[error(transparent)]
    Memory(#[from] MemorySelectionRefusal),
}

/// The protocol subject of `bindings` over `package`, with the memory model
/// of each outermost `parallel` resolved from the request's selection and
/// the source defaults (FR-219). The subject carries no proof bounds.
///
/// # Errors
///
/// [`SubjectRefusal::UnknownProtocol`] for a protocol the package does not
/// declare, and [`SubjectRefusal::Memory`] for a memory selection that does
/// not resolve.
pub fn protocol_subject(
    package: SubjectPackage<'_>,
    bindings: SubjectBindings,
) -> Result<ProtocolSubject<'_>, SubjectRefusal> {
    let SubjectBindings {
        protocol,
        over,
        roles,
        memory,
        initial,
        universes,
        normalization,
        observation,
        expansion,
        meter,
        emit,
    } = bindings;
    let checked = package
        .package
        .graph()
        .protocol(&protocol)
        .ok_or_else(|| SubjectRefusal::UnknownProtocol(protocol.clone()))?;
    let memory = ResolvedMemory::resolve(checked, &memory)?;
    Ok(ProtocolSubject {
        model: ModelSubject {
            simulation: ModelSimulationRequest {
                package: package.package,
                packages: package.packages,
                normalization,
                snapshots: package.snapshots,
                initial,
                universes,
                observation,
                expansion,
                meter,
                emit,
            },
            proof_bounds: Vec::new(),
        },
        protocol,
        over,
        roles,
        memory,
    })
}
