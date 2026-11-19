// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-142 "Subjects and pre-check" and FR-145 "Systems": the two
//! `ModelSystem`s of a refinement's subjects, built the one way the engine
//! and replay both build them.
//!
//! The concrete system is over the refinement's concrete model, with the
//! concrete subject's request. The abstract system is over its abstract model
//! with the abstract subject's initial snapshots and the derived universes:
//! each abstract population's universe is the concrete universe of its
//! source population (ADR-020 RM-2).

use fsl_eval::model_subject::{
    CapacityBinding, ModelInstance, ModelSubject, ProtocolSubject, RoleBinding,
};
use fsl_eval::simulation::model::{ModelSimulationRequest, ModelSystem, PopulationUniverse};
use fsl_eval::simulation::protocol::{
    Capacities, CapacityRefusal, ResolvedMemory, UnboundCapacity,
};
use fsl_eval::simulation::ExpansionStop;
use fsl_package::CheckedPackage;
use fsl_semantics::check::{AbstractSide, CheckedRefinement};

use fsl_semantics::model::observation::{AdmissionFailure, DocumentRef, SelectedAnchor};
use fsl_semantics::model::state::ModelState;

use crate::refinement::history::initial_history;
use crate::refinement::map::{map_state, subject_models};

/// The abstract subject's initial states (FSL FR-630 "Request item").
#[derive(Clone, Debug)]
pub enum AbstractInitial {
    /// The abstract subject names these FR-106 initial snapshots.
    Snapshots(Vec<DocumentRef>),
    /// The abstract subject is absent: its initial states are the abstract
    /// states the mapping gives the concrete initial states (FR-144).
    Mapped,
}

impl From<Vec<DocumentRef>> for AbstractInitial {
    fn from(snapshots: Vec<DocumentRef>) -> Self {
        Self::Snapshots(snapshots)
    }
}

impl From<Option<Vec<DocumentRef>>> for AbstractInitial {
    fn from(snapshots: Option<Vec<DocumentRef>>) -> Self {
        snapshots.map_or(Self::Mapped, Self::Snapshots)
    }
}

/// Why a refinement's subjects have no systems.
#[derive(Debug)]
pub enum SubjectRefusal {
    /// A subject's model system did not admit (FR-106's record).
    Admission(Box<AdmissionFailure>),
    /// The refinement maps no population or object, so it names no model.
    NoModels,
    /// An abstract population's source population has no universe in the
    /// concrete request.
    NoUniverse,
    /// The refinement's step rows select a concrete protocol (FR-136). The
    /// systems here explore a concrete model, and a protocol subject's
    /// refinement check (FR-142, FR-145) is not built yet.
    ProtocolSubject,
    /// The abstract subject is a protocol clause, whose steps FR-148 decides
    /// and no model system does.
    ProtocolAbstractSide,
    /// Building the abstract initial states from the mapping stopped on a
    /// budget (FSL FR-630 "Request item"): the run is incomplete, naming it.
    Stopped(ExpansionStop),
}

/// Why [`open_protocol_systems`] built no systems: a subject refusal, or a
/// refusal of the request's capacity binding of the abstract protocol
/// (FSpec FR-785 "Admission").
#[derive(Debug)]
pub enum ProtocolSystemsRefusal {
    /// The subjects do not open, as for [`open_systems`].
    Subject(SubjectRefusal),
    /// The capacity binding is refused.
    Capacity(CapacityRefusal),
    /// The capacity binding leaves these capacity names unbound:
    /// requires-bound.
    UnboundCapacity(Vec<UnboundCapacity>),
}

impl From<SubjectRefusal> for ProtocolSystemsRefusal {
    fn from(refusal: SubjectRefusal) -> Self {
        Self::Subject(refusal)
    }
}

/// The concrete and abstract systems of `refinement`.
pub struct RefinementSystems<'a> {
    /// The system whose states a refinement explores and maps.
    pub concrete: ModelSystem<'a>,
    /// The system the mapped states are built in and judged by.
    pub abstract_system: ModelSystem<'a>,
    /// The abstract model request from which protocol systems are built.
    pub abstract_request: ModelSimulationRequest<'a>,
    /// The selected abstract model's package identity.
    pub abstract_model: String,
    /// The admitted abstract protocol system when the abstract side is a
    /// protocol subject. Replay uses the same system that produced the run.
    pub abstract_protocol: Option<super::protocol_system::AbstractProtocolSystem<'a>>,
}

/// Build both systems of `refinement`: the concrete one from `concrete`, the
/// abstract one over `abstract_package` from `abstract_initial` and the
/// derived universes.
///
/// # Errors
///
/// [`SubjectRefusal`] when a system does not admit, a universe cannot be
/// derived or the concrete subject is a protocol.
pub fn open_systems<'a>(
    refinement: &CheckedRefinement,
    concrete: &ModelSimulationRequest<'a>,
    abstract_package: &'a CheckedPackage,
    abstract_initial: impl Into<AbstractInitial>,
) -> Result<RefinementSystems<'a>, SubjectRefusal> {
    open_systems_inner(
        refinement,
        concrete,
        abstract_package,
        abstract_initial.into(),
        false,
    )
}

/// Build model mapping systems and the abstract protocol system selected by
/// the admitted refinement. The request supplies its `over` binding and the
/// capacity binding of the clause's symbolic capacities (FSpec FR-785); the
/// clause, roles and resolved default memory come from the checked package.
///
/// # Errors
///
/// [`ProtocolSystemsRefusal::Subject`] as [`open_systems`];
/// [`ProtocolSystemsRefusal::Capacity`] for a refused capacity binding, and
/// [`ProtocolSystemsRefusal::UnboundCapacity`] for a capacity name it leaves
/// unbound, before the protocol system is built.
pub fn open_protocol_systems<'a>(
    refinement: &CheckedRefinement,
    concrete: &ModelSimulationRequest<'a>,
    abstract_package: &'a CheckedPackage,
    abstract_initial: impl Into<AbstractInitial>,
    over: ModelInstance,
    capacities: &[CapacityBinding],
) -> Result<RefinementSystems<'a>, ProtocolSystemsRefusal> {
    let abstract_initial = abstract_initial.into();
    let mapped = matches!(abstract_initial, AbstractInitial::Mapped);
    let mut systems = open_systems_inner(
        refinement,
        concrete,
        abstract_package,
        abstract_initial,
        true,
    )?;
    let AbstractSide::Protocol { clause, .. } = refinement.abstract_side() else {
        return Err(SubjectRefusal::ProtocolAbstractSide.into());
    };
    let protocol = abstract_package
        .graph()
        .protocol(clause)
        .ok_or(SubjectRefusal::ProtocolAbstractSide)?;
    let roles = protocol
        .roles
        .iter()
        .filter(|role| !role.replicated)
        .map(|role| {
            role.model_selection
                .clone()
                .or_else(|| {
                    role.context
                        .as_ref()
                        .map(|context| context.original().package().to_owned())
                })
                .map(|model| RoleBinding {
                    role: role.name.clone(),
                    model,
                })
                .ok_or(SubjectRefusal::ProtocolAbstractSide)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let admitted =
        Capacities::admit(protocol, capacities).map_err(ProtocolSystemsRefusal::Capacity)?;
    if !admitted.unbound().is_empty() {
        return Err(ProtocolSystemsRefusal::UnboundCapacity(
            admitted.unbound().to_vec(),
        ));
    }
    let memory =
        ResolvedMemory::resolve(protocol, &[]).map_err(|_| SubjectRefusal::ProtocolAbstractSide)?;
    let subject = ProtocolSubject {
        model: ModelSubject {
            simulation: systems.abstract_request.clone(),
            proof_bounds: Vec::new(),
        },
        protocol: clause.clone(),
        over,
        roles,
        capacities: capacities.to_vec(),
        memory,
    };
    let model = ModelSystem::over_model(systems.abstract_request.clone(), &systems.abstract_model)
        .map_err(|failure| SubjectRefusal::Admission(Box::new(failure)))?;
    let model = if mapped {
        model.with_initial_states(
            systems
                .abstract_system
                .initial_states()
                .iter()
                .map(|state| (state.state().clone(), state.anchor().clone())),
        )
    } else {
        model
    };
    let protocol_system = super::protocol_system::AbstractProtocolSystem::new(model, &subject)
        .map_err(|_| SubjectRefusal::ProtocolAbstractSide)?;
    systems.abstract_protocol = Some(protocol_system);
    Ok(systems)
}

fn open_systems_inner<'a>(
    refinement: &CheckedRefinement,
    concrete: &ModelSimulationRequest<'a>,
    abstract_package: &'a CheckedPackage,
    abstract_initial: AbstractInitial,
    allow_protocol: bool,
) -> Result<RefinementSystems<'a>, SubjectRefusal> {
    if refinement.concrete_protocol().is_some() {
        return Err(SubjectRefusal::ProtocolSubject);
    }
    if !allow_protocol && !matches!(refinement.abstract_side(), AbstractSide::Model) {
        return Err(SubjectRefusal::ProtocolAbstractSide);
    }
    let (concrete_model, abstract_model) =
        subject_models(refinement).ok_or(SubjectRefusal::NoModels)?;
    let universes = refinement
        .populations()
        .iter()
        .map(|row| {
            concrete
                .universes
                .iter()
                .find(|universe| universe.population == row.concrete_population)
                .map(|universe| PopulationUniverse {
                    population: row.abstract_population.clone(),
                    keys: universe.keys.clone(),
                })
                .ok_or(SubjectRefusal::NoUniverse)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let admit = |failure| SubjectRefusal::Admission(Box::new(failure));
    let (snapshots, mapped) = match abstract_initial {
        AbstractInitial::Snapshots(snapshots) => (snapshots, false),
        AbstractInitial::Mapped => (Vec::new(), true),
    };
    let abstract_request = ModelSimulationRequest {
        input: fsl_eval::simulation::model::ModelCheckInput::source(abstract_package),
        packages: concrete.packages,
        normalization: concrete.normalization,
        snapshots: concrete.snapshots,
        initial: snapshots,
        universes,
        observation: concrete.observation,
        expansion: concrete.expansion,
        meter: concrete.meter,
        emit: concrete.emit,
    };
    let concrete_system =
        ModelSystem::over_model(concrete.clone(), concrete_model).map_err(admit)?;
    let mut abstract_system =
        ModelSystem::over_model(abstract_request.clone(), abstract_model).map_err(admit)?;
    if mapped {
        let images = mapped_initial(refinement, &concrete_system, &abstract_system)?;
        abstract_system = abstract_system.with_initial_states(images);
    }
    Ok(RefinementSystems {
        concrete: concrete_system,
        abstract_system,
        abstract_request,
        abstract_model: abstract_model.to_owned(),
        abstract_protocol: None,
    })
}

/// FSL FR-630 "Request item": the abstract states the mapping gives each
/// concrete initial state, anchored as that state is. A state the mapping
/// does not determine gives none here: the initial check names its failure
/// from the same mapping. The abstract fields the mapping leaves open range
/// over the values the abstract model's invariants admit, so one concrete
/// state can give several abstract states, and none when the image violates
/// the abstract constraint (the initial check then refutes at position 0).
fn mapped_initial(
    refinement: &CheckedRefinement,
    concrete: &ModelSystem<'_>,
    abstract_system: &ModelSystem<'_>,
) -> Result<Vec<(ModelState, SelectedAnchor)>, SubjectRefusal> {
    let mut images = Vec::new();
    // One budget for every concrete initial state: the total of assignments.
    let mut budget = u128::from(abstract_system.max_candidates());
    for initial in concrete.initial_states() {
        let history = initial_history(refinement, concrete, initial.state());
        let Ok(mapped) = map_state(
            refinement,
            concrete,
            abstract_system,
            initial.state(),
            &history,
        ) else {
            continue;
        };
        let slots: Vec<_> = mapped
            .hidden
            .iter()
            .map(|slot| {
                (
                    slot.object.population.clone(),
                    slot.object.key.clone(),
                    slot.field.clone(),
                )
            })
            .collect();
        let states = abstract_system
            .complete_open_fields(&mapped.visible, initial.anchor(), &slots, &mut budget)
            .map_err(SubjectRefusal::Stopped)?;
        images.extend(
            states
                .into_iter()
                .map(|state| (state, initial.anchor().clone())),
        );
    }
    Ok(images)
}
