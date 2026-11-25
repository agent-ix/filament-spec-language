// SPDX-License-Identifier: AGPL-3.0-or-later
//! Checked bounded fairness, using the shared model-operation target identity.

use super::*;
use filament_exact::{Identifier, NodeKey, NodePathIter};
use filament_semantic_value::fairness::{FairnessGranularity, FairnessKind};
use fsl_foundation::node_wire::{project_node_key, NodeIdentitySink};
use quire_canonical::{Encode, Error, Sink, Writer};

/// Actual bounded fair-proof checking stages, for default-off observation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BoundedFairProbabilityPhase {
    /// Strict objective intake.
    Intake,
    /// Looking up an operation's actual declaring model identity.
    Declaration,
    /// Canonically encoding a resolved fairness record.
    Metadata,
    /// Independently reconstructing the bounded product and its keys.
    Graph,
    /// Checking offered values and remaining-horizon ranks.
    Equality,
}

pub(super) struct Control<'a> {
    cancel: &'a Cancel,
    #[cfg(feature = "test-support")]
    observe: std::cell::RefCell<Option<&'a mut dyn FnMut(BoundedFairProbabilityPhase)>>,
}
impl<'a> Control<'a> {
    pub(super) fn new(cancel: &'a Cancel) -> Self {
        Self {
            cancel,
            #[cfg(feature = "test-support")]
            observe: std::cell::RefCell::new(None),
        }
    }
    #[cfg(feature = "test-support")]
    pub(super) fn observing(
        cancel: &'a Cancel,
        observe: &'a mut dyn FnMut(BoundedFairProbabilityPhase),
    ) -> Self {
        Self {
            cancel,
            observe: std::cell::RefCell::new(Some(observe)),
        }
    }
    pub(super) fn phase(
        &self,
        phase: BoundedFairProbabilityPhase,
    ) -> Result<(), ProbabilityDocumentError> {
        #[cfg(feature = "test-support")]
        if let Some(observe) = self.observe.borrow_mut().as_mut() {
            observe(phase);
        }
        #[cfg(not(feature = "test-support"))]
        let _ = phase;
        poll(self.cancel)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Record {
    kind: FairnessKind,
    granularity: FairnessGranularity,
    declaration: NodeKey,
    name: Identifier,
}

impl Encode for Record {
    fn encode_into<S: Sink + ?Sized>(&self, writer: &mut Writer<'_, S>) -> Result<(), Error> {
        writer.begin_object()?;
        writer.name("fairness_kind")?;
        writer.string(match self.kind {
            FairnessKind::Weak => "weak",
            FairnessKind::Strong => "strong",
        })?;
        writer.name("granularity")?;
        writer.string(match self.granularity {
            FairnessGranularity::Whole => "whole",
            FairnessGranularity::Each => "each",
        })?;
        writer.name("target")?;
        writer.begin_object()?;
        writer.name("kind")?;
        writer.string("operation")?;
        writer.name("declaration")?;
        fsl_foundation::node_wire::encode_node_key(&self.declaration, writer)?;
        writer.name("name")?;
        writer.string(self.name.as_str())?;
        writer.end_object()?;
        writer.end_object()
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WireRecord<'a> {
    fairness_kind: &'a str,
    granularity: &'a str,
    #[serde(borrow)]
    target: WireTarget<'a>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WireTarget<'a> {
    kind: &'a str,
    #[serde(borrow)]
    declaration: &'a serde_json::value::RawValue,
    name: &'a str,
}

pub(super) fn admit(claim: &CheckedProbabilisticClaim) -> Result<(), ProbabilityDocumentError> {
    if matches!(claim.form, ProbForm::MeanTime { .. }) {
        return if !claim.fairness.is_empty()
            && matches!(claim.scheduler, ProbScheduler::Every)
            && fsl_semantics::check::TemporalProfile::from_identity(claim.profile.identity())
                == Some(fsl_semantics::check::TemporalProfile::InfiniteTrace)
        {
            Ok(())
        } else {
            Err(ProbabilityDocumentError::Unsupported)
        };
    }
    if matches!(
        &claim.form,
        ProbForm::Reach {
            target: ReachTarget::Eventually(_),
            ..
        }
    ) {
        return if !claim.fairness.is_empty()
            && matches!(claim.scheduler, ProbScheduler::Every)
            && fsl_semantics::check::TemporalProfile::from_identity(claim.profile.identity())
                == Some(fsl_semantics::check::TemporalProfile::InfiniteTrace)
        {
            Ok(())
        } else {
            Err(ProbabilityDocumentError::Unsupported)
        };
    }
    if matches!(
        &claim.form,
        ProbForm::Quantile { .. }
            | ProbForm::MeanOfFraction { .. }
            | ProbForm::Probability {
                event: fsl_semantics::check::Event::Compare { .. },
                ..
            }
    ) {
        return if !claim.fairness.is_empty()
            && matches!(claim.scheduler, ProbScheduler::Every)
            && fsl_semantics::check::TemporalProfile::from_identity(claim.profile.identity())
                == Some(fsl_semantics::check::TemporalProfile::EventPositionFalseExtension)
        {
            Ok(())
        } else {
            Err(ProbabilityDocumentError::Unsupported)
        };
    }
    let ProbForm::Probability {
        event:
            fsl_semantics::check::Event::Formula {
                formula,
                timed_horizon: None,
                ..
            },
        ..
    } = &claim.form
    else {
        return Err(ProbabilityDocumentError::Unsupported);
    };
    let profile = fsl_semantics::check::TemporalProfile::from_identity(claim.profile.identity())
        .ok_or(ProbabilityDocumentError::Unsupported)?;
    if claim.fairness.is_empty()
        || !matches!(claim.scheduler, ProbScheduler::Every)
        || !fsl_eval::automaton::QualifiedBoundedMonitor::supports(formula.formula(), profile)
        || !matches!(
            formula
                .formula()
                .nodes()
                .get(formula.formula().root().index()),
            Some(fsl_semantics::check::FormulaNode::Unary {
                operator: fsl_semantics::check::UnaryTemporal::Eventually,
                ..
            })
        )
    {
        return Err(ProbabilityDocumentError::Unsupported);
    }
    Ok(())
}

struct Admission<'a> {
    limits: ProbabilityDocumentLimits,
    cancel: &'a Cancel,
    meter: &'a mut filament_exact::Meter,
    storage: &'a mut u64,
}
impl Admission<'_> {
    fn work(&mut self, amount: u64) -> Result<(), ProbabilityDocumentError> {
        poll(self.cancel)?;
        let mut admission = ProbabilityKeyAdmission::new(
            self.meter,
            self.storage,
            self.limits.storage_bytes,
            self.cancel,
        );
        filament_exact::IntegerDecimalBudget::work(&mut admission, amount)
            .map_err(|_| admission.refusal(fsl_eval::simulation::model::ModelKeyError::Denied))
    }
    fn reserve(&mut self, bytes: usize) -> Result<(), ProbabilityDocumentError> {
        let mut admission = ProbabilityKeyAdmission::new(
            self.meter,
            self.storage,
            self.limits.storage_bytes,
            self.cancel,
        );
        filament_exact::Reservation::reserve(&mut admission, bytes)
            .map_err(|_| admission.refusal(fsl_eval::simulation::model::ModelKeyError::Denied))
    }
    fn bytes(&mut self, record: &Record) -> Result<Vec<u8>, ProbabilityDocumentError> {
        project_node_key(&record.declaration, self)?;
        self.work(u64::try_from(record.name.as_str().len()).unwrap_or(u64::MAX))?;
        self.encoded(record)
    }
    fn declaration(&mut self, text: &str) -> Result<NodeKey, ProbabilityDocumentError> {
        let mut admission = ProbabilityKeyAdmission::new(
            self.meter,
            self.storage,
            self.limits.storage_bytes,
            self.cancel,
        );
        fsl_foundation::node_wire::node_key_from_text_with_budget(text, &mut admission).map_err(
            |error| match error {
                fsl_foundation::node_wire::NodeWireError::Denied => {
                    admission.refusal(fsl_eval::simulation::model::ModelKeyError::Denied)
                }
                _ => ProbabilityDocumentError::MalformedMember("objective"),
            },
        )
    }
    fn grow<T>(&mut self, rows: &mut Vec<T>) -> Result<(), ProbabilityDocumentError> {
        if rows.len() == rows.capacity() {
            let capacity = rows.capacity().saturating_mul(2).max(4);
            self.reserve(
                capacity
                    .checked_mul(std::mem::size_of::<T>())
                    .ok_or(ProbabilityDocumentError::KeySizeOverflow)?,
            )?;
            rows.try_reserve_exact(capacity - rows.len())
                .map_err(|_| ProbabilityDocumentError::KeySizeOverflow)?;
        }
        Ok(())
    }
    fn encoded(&mut self, value: &impl Encode) -> Result<Vec<u8>, ProbabilityDocumentError> {
        let mut admission = ProbabilityKeyAdmission::new(
            self.meter,
            self.storage,
            self.limits.storage_bytes,
            self.cancel,
        );
        struct Reserve<'a, 'b>(&'a mut ProbabilityKeyAdmission<'b>);
        impl quire_canonical::Reservation for Reserve<'_, '_> {
            fn reserve(&mut self, bytes: usize) -> Result<(), quire_canonical::ReservationDenied> {
                filament_exact::IntegerDecimalBudget::work(
                    self.0,
                    u64::try_from(bytes).unwrap_or(u64::MAX),
                )
                .map_err(|_| quire_canonical::ReservationDenied)?;
                filament_exact::Reservation::reserve(self.0, bytes)
                    .map_err(|_| quire_canonical::ReservationDenied)
            }
        }
        let result =
            writer::encode_admitted(value, self.limits.bytes, &mut Reserve(&mut admission));
        result.map_err(|error| match error {
            Error::ReservationDenied { .. } => {
                admission.refusal(fsl_eval::simulation::model::ModelKeyError::Denied)
            }
            Error::Limit(limit) => {
                exceeded(ProbabilityDocumentLimit::Bytes, limit.bound, limit.required)
            }
            _ => ProbabilityDocumentError::MalformedMember("objective"),
        })
    }
}
impl NodeIdentitySink for Admission<'_> {
    type Error = ProbabilityDocumentError;
    fn visit(&mut self) -> Result<(), Self::Error> {
        self.work(1)
    }
    fn begin_object(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
    fn name(&mut self, _: &'static str) -> Result<(), Self::Error> {
        Ok(())
    }
    fn ordinal(&mut self, _: u64) -> Result<(), Self::Error> {
        Ok(())
    }
    fn text(&mut self, text: &str) -> Result<(), Self::Error> {
        self.work(u64::try_from(text.len()).unwrap_or(u64::MAX))
    }
    fn begin_array(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
    fn end_array(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
    fn end_object(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
    fn push_path<'a>(
        &mut self,
        stack: &mut Vec<NodePathIter<'a>>,
        path: NodePathIter<'a>,
    ) -> Result<(), Self::Error> {
        self.grow(stack)?;
        stack.push(path);
        Ok(())
    }
}

pub(super) fn checked(
    system: &ModelSystem<'_>,
    claim: &CheckedProbabilisticClaim,
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
    meter: &mut filament_exact::Meter,
    storage: &mut u64,
) -> Result<Vec<Record>, ProbabilityDocumentError> {
    checked_with_control(
        system,
        claim,
        limits,
        cancel,
        meter,
        storage,
        &Control::new(cancel),
    )
}

#[allow(clippy::too_many_arguments)]
pub(super) fn checked_with_control(
    system: &ModelSystem<'_>,
    claim: &CheckedProbabilisticClaim,
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
    meter: &mut filament_exact::Meter,
    storage: &mut u64,
    control: &Control<'_>,
) -> Result<Vec<Record>, ProbabilityDocumentError> {
    admit(claim)?;
    records(system, claim, limits, cancel, meter, storage, control)
}

/// The claim's fairness set as canonical, sorted and de-duplicated records,
/// for a form that has already been admitted by its own profile.
#[allow(clippy::too_many_arguments)]
pub(super) fn records(
    system: &ModelSystem<'_>,
    claim: &CheckedProbabilisticClaim,
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
    meter: &mut filament_exact::Meter,
    storage: &mut u64,
    control: &Control<'_>,
) -> Result<Vec<Record>, ProbabilityDocumentError> {
    let mut cx = Admission {
        limits,
        cancel,
        meter,
        storage,
    };
    let mut rows: Vec<(Vec<u8>, Record)> = Vec::new();
    for constraint in &claim.fairness {
        cx.work(1)?;
        control.phase(BoundedFairProbabilityPhase::Declaration)?;
        let operation = constraint.operation();
        let key = system
            .declaration_key_with_work(&operation.declaring_type(), &mut |work| cx.work(work))?
            .ok_or(ProbabilityDocumentError::Unsupported)?;
        cx.work(
            u64::try_from(
                key.package
                    .len()
                    .saturating_add(key.node.len())
                    .saturating_add(operation.name().len()),
            )
            .unwrap_or(u64::MAX),
        )?;
        let identity = fsl_semantics::model::key::ModelNodeIdentity::new(
            &key.package,
            "model",
            "object_type",
            &key.node,
        )
        .map_err(|_| ProbabilityDocumentError::MalformedMember("objective"))?;
        identity.project(&mut cx)?;
        let identity = cx.encoded(&identity)?;
        let declaration = cx.declaration(
            std::str::from_utf8(&identity)
                .map_err(|_| ProbabilityDocumentError::MalformedMember("objective"))?,
        )?;
        cx.reserve(operation.name().len())?;
        let record = Record {
            kind: constraint.kind(),
            granularity: constraint.granularity(),
            declaration,
            name: Identifier::new(operation.name())
                .map_err(|_| ProbabilityDocumentError::MalformedMember("objective"))?,
        };
        control.phase(BoundedFairProbabilityPhase::Metadata)?;
        let bytes = cx.bytes(&record)?;
        cx.grow(&mut rows)?;
        let mut at = rows.len();
        while at > 0 {
            cx.work(
                u64::try_from(bytes.len().min(rows[at - 1].0.len()))
                    .unwrap_or(u64::MAX)
                    .saturating_add(1),
            )?;
            if rows[at - 1].0 <= bytes {
                break;
            }
            at -= 1;
        }
        if at > 0 && rows[at - 1].0 == bytes {
            continue;
        }
        rows.insert(at, (bytes, record));
    }
    cx.reserve(rows.len().saturating_mul(std::mem::size_of::<Record>()))?;
    let mut records = Vec::new();
    records
        .try_reserve_exact(rows.len())
        .map_err(|_| ProbabilityDocumentError::KeySizeOverflow)?;
    records.extend(rows.into_iter().map(|(_, record)| record));
    Ok(records)
}

#[derive(Clone)]
struct FairGraphAction {
    identity: Option<fsl_eval::simulation::model::ScheduledIdentity>,
    successors: Vec<usize>,
}

struct FairGraphState {
    actions: Vec<FairGraphAction>,
    enabled: Vec<fsl_eval::simulation::model::ScheduledIdentity>,
}

#[derive(Clone)]
struct FairObligation {
    operation: String,
    kind: FairnessKind,
    whole: bool,
    identity: Option<fsl_eval::simulation::model::ScheduledIdentity>,
}

/// Independently expand a fair eventuality's concrete model graph and return
/// every non-target state in a maximal fair end component.
#[allow(clippy::too_many_arguments)]
pub(super) fn non_target_fair_end_component_states(
    system: &ModelSystem<'_>,
    claim: &CheckedProbabilisticClaim,
    graph: &CheckerGraph,
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
    identity_limits: fsl_foundation::IdentityLimits,
    meter: &mut filament_exact::Meter,
    storage: &mut u64,
) -> Result<std::collections::BTreeSet<ProbabilityProductKey>, ProbabilityDocumentError> {
    let ProbForm::Reach {
        target: ReachTarget::Eventually(predicate),
        ..
    } = &claim.form
    else {
        return Err(ProbabilityDocumentError::Unsupported);
    };
    let binding = claim_binding(system, claim)?;
    let reference = binding.as_ref().map(|bound| &bound.reference);
    let workload = claim_workload(system, claim)?;
    Ok(fair_end_component_keys(
        system,
        graph,
        Some(predicate),
        reference,
        workload,
        &claim.fairness,
        true,
        limits,
        cancel,
        identity_limits,
        meter,
        storage,
    )?
    .into_iter()
    .flatten()
    .collect())
}

/// Independently reconstruct fair maximal end components for an exact
/// checker graph. When `exclude_targets` is true, accepting reachability
/// states are omitted from the recurrent set.
#[allow(clippy::too_many_arguments)]
pub(super) fn fair_end_component_keys(
    system: &ModelSystem<'_>,
    graph: &CheckerGraph,
    target: Option<&fsl_semantics::check::CheckedPredicate>,
    reference: Option<&ObjectReference>,
    workload: Option<&Workload>,
    constraints: &[filament_semantic_value::fairness::FairnessConstraint],
    exclude_targets: bool,
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
    identity_limits: fsl_foundation::IdentityLimits,
    meter: &mut filament_exact::Meter,
    storage: &mut u64,
) -> Result<Vec<Vec<ProbabilityProductKey>>, ProbabilityDocumentError> {
    use filament_semantic_value::fairness::{FairnessGranularity, FairnessKind};
    use fsl_eval::simulation::model::{ModelKeyExpansionError, ScheduledIdentity};
    use std::collections::{BTreeMap, BTreeSet, VecDeque};

    let captured = super::cyclic::CapturedStates::default();
    let expanded =
        super::cyclic::enumerate_graph_capturing::<fsl_semantics::check::CheckedPredicate>(
            system,
            target,
            reference,
            workload,
            limits,
            cancel,
            identity_limits,
            meter,
            storage,
            exclude_targets,
            true,
            Some(&captured),
        )?;
    if expanded.states.keys().ne(graph.states.keys()) {
        return Err(ProbabilityDocumentError::ObjectiveMismatch);
    }
    let keys: Vec<_> = graph.states.keys().cloned().collect();
    let indices: BTreeMap<_, _> = keys
        .iter()
        .cloned()
        .enumerate()
        .map(|(index, key)| (key, index))
        .collect();
    let mut nodes = Vec::new();
    nodes
        .try_reserve_exact(keys.len())
        .map_err(|_| ProbabilityDocumentError::KeySizeOverflow)?;
    for key in &keys {
        poll(cancel)?;
        let state = captured
            .borrow()
            .get(key)
            .cloned()
            .ok_or(ProbabilityDocumentError::ObjectiveMismatch)?;
        let checker_state = graph
            .states
            .get(key)
            .ok_or(ProbabilityDocumentError::ObjectiveMismatch)?;
        if exclude_targets && checker_state.terminal == Some(true) {
            nodes.push(FairGraphState {
                actions: Vec::new(),
                enabled: Vec::new(),
            });
            continue;
        }
        if checker_state.choices.is_empty() {
            let index = *indices
                .get(key)
                .ok_or(ProbabilityDocumentError::ObjectiveMismatch)?;
            nodes.push(FairGraphState {
                actions: vec![FairGraphAction {
                    identity: None,
                    successors: vec![index],
                }],
                enabled: Vec::new(),
            });
            continue;
        }
        let mut admission =
            ProbabilityKeyAdmission::new(meter, storage, limits.storage_bytes, cancel);
        let actions =
            match system.actions_with_budget(&state, identity_limits, &mut admission, cancel) {
                Ok(Ok(actions)) => actions,
                Ok(Err(_)) => return Err(ProbabilityDocumentError::Unsupported),
                Err(error) => {
                    return Err(match error {
                        ModelKeyExpansionError::Key(error) => admission.refusal(error),
                        ModelKeyExpansionError::Expansion(stop) => match cancel.tripped() {
                            Some(cause) => ProbabilityDocumentError::Cancelled(cause),
                            None => ProbabilityDocumentError::Build(Box::new(
                                crate::ExactProbBuildIssue::Expansion(stop),
                            )),
                        },
                    });
                }
            };
        let mut rows = Vec::new();
        let mut enabled = Vec::new();
        for action in actions {
            let mut successors = BTreeSet::new();
            for draw in action.draws {
                if draw.probability == rational(0) {
                    continue;
                }
                for (_, successor) in draw.successors {
                    let mut admission =
                        ProbabilityKeyAdmission::new(meter, storage, limits.storage_bytes, cancel);
                    let bytes = system
                        .state_key_bytes_with_budget(
                            &successor,
                            identity_limits,
                            &mut admission,
                            cancel,
                        )
                        .map_err(|error| admission.refusal(error))?;
                    let successor_key = ProbabilityProductKey::for_model_state_with_budget(
                        reference,
                        &bytes,
                        identity_limits,
                        &mut admission,
                        cancel,
                    )
                    .map_err(|error| admission.refusal(error))?;
                    successors.insert(
                        *indices
                            .get(&successor_key)
                            .ok_or(ProbabilityDocumentError::ObjectiveMismatch)?,
                    );
                }
            }
            if successors.is_empty() {
                continue;
            }
            let identity = action.identity;
            if !enabled.contains(&identity) {
                enabled.push(identity.clone());
            }
            rows.push(FairGraphAction {
                identity: Some(identity),
                successors: successors.into_iter().collect(),
            });
        }
        nodes.push(FairGraphState {
            actions: rows,
            enabled,
        });
    }

    let mut obligations = Vec::new();
    for constraint in constraints {
        let operation = constraint.operation();
        let operation_text = system
            .operation_text(operation.declaring_type(), operation.name())
            .ok_or(ProbabilityDocumentError::Unsupported)?
            .to_owned();
        if constraint.granularity() == FairnessGranularity::Whole {
            obligations.push(FairObligation {
                operation: operation_text,
                kind: constraint.kind(),
                whole: true,
                identity: None,
            });
        } else {
            let mut identities: Vec<ScheduledIdentity> = Vec::new();
            for node in &nodes {
                for action in &node.enabled {
                    if action.operation == operation_text && !identities.contains(action) {
                        identities.push(action.clone());
                    }
                }
            }
            for identity in identities {
                obligations.push(FairObligation {
                    operation: operation_text.clone(),
                    kind: constraint.kind(),
                    whole: false,
                    identity: Some(identity),
                });
            }
        }
    }

    let universe: BTreeSet<_> = keys
        .iter()
        .enumerate()
        .filter_map(|(index, key)| {
            (!exclude_targets
                || graph
                    .states
                    .get(key)
                    .is_some_and(|state| state.terminal != Some(true)))
            .then_some(index)
        })
        .collect();
    let mut pending: VecDeque<_> = maximal_mecs(&nodes, universe).into();
    let mut fair_components = Vec::new();
    while let Some(candidate) = pending.pop_front() {
        let mut rejected = false;
        let mut split = None;
        for obligation in &obligations {
            let enabled = candidate
                .iter()
                .map(|state| {
                    nodes[*state].enabled.iter().any(|identity| {
                        identity.operation == obligation.operation
                            && (obligation.whole || obligation.identity.as_ref() == Some(identity))
                    })
                })
                .collect::<Vec<_>>();
            let taken = candidate.iter().any(|state| {
                nodes[*state].actions.iter().any(|action| {
                    action.identity.as_ref().is_some_and(|identity| {
                        identity.operation == obligation.operation
                            && (obligation.whole || obligation.identity.as_ref() == Some(identity))
                            && action
                                .successors
                                .iter()
                                .all(|successor| candidate.contains(successor))
                    })
                })
            });
            if taken {
                continue;
            }
            match obligation.kind {
                FairnessKind::Weak if enabled.iter().all(|value| *value) => {
                    rejected = true;
                    break;
                }
                FairnessKind::Strong if enabled.iter().any(|value| *value) => {
                    let retained = candidate
                        .iter()
                        .copied()
                        .filter(|state| !enabled[candidate.range(..*state).count()])
                        .collect();
                    split = Some(maximal_mecs(&nodes, retained));
                    break;
                }
                _ => {}
            }
        }
        if rejected {
            continue;
        }
        if let Some(children) = split {
            pending.extend(children);
        } else {
            fair_components.push(candidate);
        }
    }

    Ok(fair_components
        .into_iter()
        .map(|component| {
            component
                .into_iter()
                .map(|index| keys[index].clone())
                .collect()
        })
        .collect())
}

fn maximal_mecs(
    nodes: &[FairGraphState],
    initial: std::collections::BTreeSet<usize>,
) -> Vec<std::collections::BTreeSet<usize>> {
    use std::collections::{BTreeSet, VecDeque};
    let mut pending = VecDeque::from([initial]);
    let mut result = Vec::new();
    while let Some(mut candidate) = pending.pop_front() {
        loop {
            let retained: BTreeSet<_> = candidate
                .iter()
                .copied()
                .filter(|state| {
                    nodes[*state].actions.iter().any(|action| {
                        action
                            .successors
                            .iter()
                            .all(|successor| candidate.contains(successor))
                    })
                })
                .collect();
            if retained == candidate {
                break;
            }
            candidate = retained;
            if candidate.is_empty() {
                break;
            }
        }
        if candidate.is_empty() {
            continue;
        }
        let mut adjacency = vec![Vec::new(); nodes.len()];
        for state in &candidate {
            for action in &nodes[*state].actions {
                if action
                    .successors
                    .iter()
                    .all(|successor| candidate.contains(successor))
                {
                    adjacency[*state].extend(action.successors.iter().copied());
                }
            }
            adjacency[*state].sort_unstable();
            adjacency[*state].dedup();
        }
        let components = fair_sccs(&adjacency, &candidate);
        if components.len() == 1 && components[0] == candidate {
            result.push(candidate);
        } else {
            pending.extend(components);
        }
    }
    result
}

fn fair_sccs(
    adjacency: &[Vec<usize>],
    states: &std::collections::BTreeSet<usize>,
) -> Vec<std::collections::BTreeSet<usize>> {
    use std::collections::BTreeSet;
    let mut reverse = vec![Vec::new(); adjacency.len()];
    for from in states {
        for to in &adjacency[*from] {
            if states.contains(to) {
                reverse[*to].push(*from);
            }
        }
    }
    let mut seen = vec![false; adjacency.len()];
    let mut order = Vec::new();
    for root in states {
        if seen[*root] {
            continue;
        }
        seen[*root] = true;
        let mut stack = vec![(*root, 0_usize)];
        while let Some((state, offset)) = stack.last_mut() {
            if let Some(next) = adjacency[*state]
                .iter()
                .filter(|next| states.contains(next))
                .nth(*offset)
                .copied()
            {
                *offset += 1;
                if !seen[next] {
                    seen[next] = true;
                    stack.push((next, 0));
                }
            } else {
                order.push(stack.pop().expect("nonempty SCC stack").0);
            }
        }
    }
    seen.fill(false);
    let mut components = Vec::new();
    for root in order.into_iter().rev() {
        if seen[root] {
            continue;
        }
        seen[root] = true;
        let mut component = BTreeSet::new();
        let mut stack = vec![root];
        while let Some(state) = stack.pop() {
            component.insert(state);
            for predecessor in &reverse[state] {
                if !seen[*predecessor] {
                    seen[*predecessor] = true;
                    stack.push(*predecessor);
                }
            }
        }
        components.push(component);
    }
    components
}

pub(super) fn same(
    expected: &[Record],
    offered: &[Record],
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
    meter: &mut filament_exact::Meter,
    storage: &mut u64,
) -> Result<bool, ProbabilityDocumentError> {
    let mut cx = Admission {
        limits,
        cancel,
        meter,
        storage,
    };
    cx.work(1)?;
    if expected.len() != offered.len() {
        return Ok(false);
    }
    for (expected, offered) in expected.iter().zip(offered) {
        let expected = cx.bytes(expected)?;
        let offered = cx.bytes(offered)?;
        cx.work(
            u64::try_from(expected.len().min(offered.len()))
                .unwrap_or(u64::MAX)
                .saturating_add(1),
        )?;
        if expected != offered {
            return Ok(false);
        }
    }
    Ok(true)
}

pub(super) fn admit_encoding(
    records: &[Record],
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
) -> Result<(), ProbabilityDocumentError> {
    let mut meter = limits.accounting_meter(cancel);
    let mut storage = 0;
    let mut cx = Admission {
        limits,
        cancel,
        meter: &mut meter,
        storage: &mut storage,
    };
    for record in records {
        cx.bytes(record)?;
    }
    Ok(())
}

#[ix_trace_rs::string_edge]
pub(super) fn read(
    records: &[Box<serde_json::value::RawValue>],
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
    meter: &mut filament_exact::Meter,
    storage: &mut u64,
) -> Result<Vec<Record>, ProbabilityDocumentError> {
    let mut cx = Admission {
        limits,
        cancel,
        meter,
        storage,
    };
    let mut result = Vec::new();
    let mut previous: Option<Vec<u8>> = None;
    for raw in records {
        cx.work(u64::try_from(raw.get().len()).unwrap_or(u64::MAX))?;
        let wire: WireRecord<'_> = serde_json::from_str(raw.get())
            .map_err(|_| ProbabilityDocumentError::MalformedMember("objective"))?;
        cx.reserve(wire.target.name.len())?;
        let record = Record {
            kind: match wire.fairness_kind {
                "weak" => FairnessKind::Weak,
                "strong" => FairnessKind::Strong,
                _ => return Err(ProbabilityDocumentError::MalformedMember("objective")),
            },
            granularity: match wire.granularity {
                "whole" => FairnessGranularity::Whole,
                "each" => FairnessGranularity::Each,
                _ => return Err(ProbabilityDocumentError::MalformedMember("objective")),
            },
            declaration: cx.declaration(wire.target.declaration.get())?,
            name: Identifier::new(wire.target.name)
                .map_err(|_| ProbabilityDocumentError::MalformedMember("objective"))?,
        };
        if wire.target.kind != "operation" {
            return Err(ProbabilityDocumentError::MalformedMember("objective"));
        }
        let bytes = cx.bytes(&record)?;
        if previous.as_ref().is_some_and(|previous| previous >= &bytes) {
            return Err(ProbabilityDocumentError::MalformedMember("objective"));
        }
        previous = Some(bytes);
        cx.grow(&mut result)?;
        result.push(record);
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;

    fn limits() -> ProbabilityDocumentLimits {
        let mut accounting = crate::AccountingLimits::default().0;
        accounting.work_units = u64::MAX;
        ProbabilityDocumentLimits {
            bytes: 1_000_000,
            storage_bytes: 64_000_000,
            states: 128,
            edges: 1024,
            scalar_bits: 128,
            accounting,
            work_scope: None,
        }
    }

    #[trace("TC-1406", "FR-201-AC-50")]
    #[test]
    fn shared_structural_metadata_walk_admits_deep_nodes_and_text_before_writing() {
        std::thread::Builder::new()
            .stack_size(512 * 1024)
            .spawn(|| {
                let mut declaration = NodeKey::new(
                    "pkg",
                    vec![filament_exact::PathSegment::Text("leaf".repeat(2048))],
                    0,
                )
                .unwrap();
                for _ in 0..1024 {
                    declaration = NodeKey::new(
                        "pkg",
                        vec![filament_exact::PathSegment::Node(declaration)],
                        0,
                    )
                    .unwrap();
                }
                let record = Record {
                    kind: FairnessKind::Weak,
                    granularity: FairnessGranularity::Each,
                    declaration,
                    name: Identifier::new("tick").unwrap(),
                };
                let cancel = Cancel::new();
                let mut meter = filament_exact::Meter::new(limits().accounting);
                let mut storage = 0;
                let bytes = Admission {
                    limits: limits(),
                    cancel: &cancel,
                    meter: &mut meter,
                    storage: &mut storage,
                }
                .bytes(&record)
                .unwrap();
                let raw =
                    serde_json::value::RawValue::from_string(String::from_utf8(bytes).unwrap())
                        .unwrap();
                let records = read(&[raw], limits(), &cancel, &mut meter, &mut storage).unwrap();
                assert!(same(
                    std::slice::from_ref(&record),
                    &records,
                    limits(),
                    &cancel,
                    &mut meter,
                    &mut storage
                )
                .unwrap());
                for cap in 0..3 {
                    let mut bound = limits();
                    match cap {
                        0 => bound.bytes = 32,
                        1 => bound.storage_bytes = 0,
                        2 => bound.accounting.work_units = 10,
                        _ => unreachable!(),
                    }
                    let mut meter = filament_exact::Meter::new(bound.accounting);
                    let mut storage = 0;
                    assert!(matches!(
                        Admission {
                            limits: bound,
                            cancel: &cancel,
                            meter: &mut meter,
                            storage: &mut storage
                        }
                        .bytes(&record),
                        Err(ProbabilityDocumentError::Limit { .. }
                            | ProbabilityDocumentError::Accounting(_))
                    ));
                }
                let text =
                    fsl_foundation::node_wire::node_key_to_text(&record.declaration).unwrap();
                let mut bound = limits();
                bound.storage_bytes = 0;
                let mut meter = filament_exact::Meter::new(bound.accounting);
                assert!(matches!(
                    Admission {
                        limits: bound,
                        cancel: &cancel,
                        meter: &mut meter,
                        storage: &mut 0
                    }
                    .declaration(&text),
                    Err(ProbabilityDocumentError::Limit {
                        limit: ProbabilityDocumentLimit::StorageBytes,
                        ..
                    })
                ));
                for cause in [CancelCause::Requested, CancelCause::Deadline] {
                    let cancel = Cancel::new();
                    cancel.cancel(cause);
                    let mut meter = filament_exact::Meter::new(limits().accounting);
                    assert_eq!(
                        Admission {
                            limits: limits(),
                            cancel: &cancel,
                            meter: &mut meter,
                            storage: &mut 0
                        }
                        .declaration(&text),
                        Err(ProbabilityDocumentError::Cancelled(cause))
                    );
                }
            })
            .unwrap()
            .join()
            .unwrap();
    }
}
