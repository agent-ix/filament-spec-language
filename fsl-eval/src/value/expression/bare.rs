// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-531, FR-533, FR-945: evaluation of calls from a package read from its
//! bytes alone.
//!
//! The call selects the slot from the package's `protocol` node, projects its
//! expression and the declarations it runs against from the package's nodes
//! and the identity rows, admits the supply, and runs the one S6a machine
//! over the projection. No value of the source side (a `CheckedPackage`, a
//! `CheckedBody`, a source graph) reaches any step.

use std::collections::BTreeMap;

use filament_eval::{
    CallAdmissionFailure, CallAdmissionLocus, CallFailure, Evaluation, FamilyOutcome,
};
use filament_exact::{Charge, ChargePoint, Meter, NodeKey, Outcome, Value};
use filament_semantic_value::call::InputRefusal;
use filament_semantic_value::catalog::InternalFault;
use fsl_package::{
    BareClauseTerm, BarePackage, BareSelectionFailure, BareSlot, BareWorld, ClauseBinder,
    ProtocolSlotDescriptor,
};
use fsl_semantics::model::object_environment::ObjectEnvironment;

use super::evaluate::reader_source::ReaderNode;
use super::evaluate::{Machine, World};
use super::protocol_slot::{resolve, ProtocolSlot, ProtocolSlotEvaluation, ProtocolSupply};
use super::slot_supply::{admit_slot_supply, AdmittedSupply, SlotHost, SourceView};
use super::{
    unsupported_projection, ApplicationObservation, ApplicationObservationOptions,
    CheckedPackageEvaluation, ClauseEvaluation, ClauseSetEvaluation, FrameEvaluation,
    QualifiedName, Separation, SuppliedFailure, SuppliedLimits, SuppliedRun,
    ValueInvariantEvaluation,
};

/// Evaluate a reader projection over the local frame with the shared
/// bytes-only package machine.
pub fn evaluate_bare_expression(
    world: &BareWorld,
    expression: &fsl_package::ReaderProtocolProjection,
    frame: Vec<Option<Value>>,
    meter: &mut Meter,
) -> Result<Evaluation, InternalFault> {
    let objects = ObjectEnvironment::default();
    let reads = BTreeMap::new();
    Machine::with_world(
        World::Bytes(world),
        &objects,
        None,
        Some(&reads),
        meter,
        &[],
    )
    .run_frame(ReaderNode::root(expression), frame)
}

/// Evaluate an admitted refinement row over the existing model world's
/// generated observation and original parameter values.
///
/// This borrows the world's already constructed declarations and uses the
/// caller's meter through the same machine entry as a source refinement row.
///
/// # Errors
/// Returns the evaluator's invariant fault without a fallback value.
pub fn evaluate_bare_refinement_row(
    world: &BareWorld,
    expression: &fsl_package::ReaderProtocolProjection,
    current: &ObjectEnvironment,
    bindings: Vec<Value>,
    meter: &mut Meter,
) -> Result<Evaluation, InternalFault> {
    let reads = BTreeMap::new();
    Machine::with_world(World::Bytes(world), current, None, Some(&reads), meter, &[]).run(
        ReaderNode::root(expression),
        expression.frame_slots(),
        bindings,
    )
}

/// A selection or projection refusal as the call reports it (FR-533
/// "Refusals and loci of the bare slot call"): a witness refusal at the
/// `Package` locus, an unsupported projection at its `Node` locus, a missing
/// node or member at theirs.
fn refusal(package: &BarePackage<'_>, failure: BareSelectionFailure) -> CallFailure {
    match failure.into_view(package.package().name()) {
        Ok(view) => unsupported_projection(&view),
        Err(failure) => resolve::selection_failure(failure),
    }
}

fn clock_declarations(
    package: &BarePackage<'_>,
) -> Result<fsl_semantics::model::clock_reference::normalization::ClockDeclarationTable, CallFailure>
{
    use fsl_semantics::model::observation::AdmissionFailure;
    package
        .clock_declaration_table()
        .map_err(|failure| match failure {
            AdmissionFailure::Fault(fault) => CallFailure::Fault(fault),
            AdmissionFailure::Refused(record) => CallFailure::Admission(Box::new(
                CallAdmissionFailure::Refused(filament_eval::CallAdmissionRecord {
                    record: *record,
                    locus: CallAdmissionLocus::Package,
                }),
            )),
            AdmissionFailure::Incomplete(record) => CallFailure::Admission(Box::new(
                CallAdmissionFailure::Incomplete(filament_eval::CallAdmissionRecord {
                    record: *record,
                    locus: CallAdmissionLocus::Package,
                }),
            )),
        })
}

/// Run the package atom through the same component-aware machine as a source
/// atom, including absent-key bindings.
fn evaluate_hyper_holds(
    package: &BarePackage<'_>,
    clause: &fsl_semantics::check::CheckedHyperClause,
    atom: fsl_semantics::check::FormulaNodeId,
    slots: Vec<crate::hyper::HyperSlot>,
    components: &[&ObjectEnvironment],
    meter: &mut Meter,
) -> Result<Evaluation, CallFailure> {
    let fault = |invariant| CallFailure::Fault(InternalFault::new("S6a", invariant));
    clause
        .package_atom(atom)
        .ok_or_else(|| fault("hyper-atom-resolved-by-clause"))?;
    if slots.len() != clause.atom_slots().len() {
        return Err(fault("hyper-atom-binds-one-value-per-slot"));
    }
    if components.len() != clause.quantifiers().len() {
        return Err(fault("hyper-atom-observes-one-state-per-trace"));
    }
    let first = components
        .first()
        .copied()
        .ok_or_else(|| fault("hyper-atom-observes-one-state-per-trace"))?;
    let claim = clause
        .assembled_occurrence()
        .ok_or_else(|| fault("hyper-clause-has-an-assembled-occurrence"))?;
    let term =
        package
            .project_hyper_holds(claim, atom.index())
            .map_err(|failure| match failure {
                fsl_package::BareClauseRefusal::Selection(failure) => refusal(package, failure),
                fsl_package::BareClauseRefusal::Pending { fact, .. } => unmet(package, fact),
            })?;
    let world = package
        .world()
        .map_err(|failure| refusal(package, failure))?;
    let mut machine = Machine::from_bare_package(&world, first, meter).with_components(
        super::evaluate::ComponentReads {
            environments: components,
            pre_environments: None,
            reads: term.component_reads(),
            observations: None,
        },
    );
    let mut values = Vec::with_capacity(slots.len());
    for (slot, binding) in slots.into_iter().enumerate() {
        match binding {
            crate::hyper::HyperSlot::Bound(value) => values.push(value),
            crate::hyper::HyperSlot::Absent { key, population } => {
                machine = machine.with_absent_key(slot, population, key.clone());
                values.push(Value::Reference(key));
            }
        }
    }
    machine
        .run(
            ReaderNode::root(term.projection()),
            term.projection().frame_slots(),
            values,
        )
        .map_err(CallFailure::Fault)
}

fn function_projection(
    package: &BarePackage<'_>,
    function: &QualifiedName,
) -> Result<fsl_package::ReaderProtocolProjection, CallFailure> {
    let name = function
        .as_unqualified()
        .ok_or_else(|| InputRefusal::UnknownFunction(function.to_string()))?;
    package
        .project_function(name)
        .map_err(|failure| function_refusal(package, failure))?
        .ok_or_else(|| InputRefusal::UnknownFunction(function.to_string()).into())
}

fn function_refusal(package: &BarePackage<'_>, failure: BareSelectionFailure) -> CallFailure {
    match failure {
        BareSelectionFailure::FunctionParameterType { position, view } => {
            unsupported_function_type(&view, "position", position.to_string())
        }
        BareSelectionFailure::FunctionResultType { function, view } => {
            unsupported_function_type(&view, "function", function)
        }
        other => refusal(package, other),
    }
}

fn unsupported_function_type(
    view: &fsl_package::UnsupportedView,
    detail: &'static str,
    value: String,
) -> CallFailure {
    let (code, cause) = view.catalog();
    let mut record = super::occurrence_record(code, cause, Vec::new());
    record.fields.insert(detail, value);
    let locus = view
        .locus()
        .key()
        .map_or(CallAdmissionLocus::Package, |key| {
            CallAdmissionLocus::Node(key.clone())
        });
    CallFailure::Admission(Box::new(CallAdmissionFailure::Refused(
        filament_eval::CallAdmissionRecord { record, locus },
    )))
}

/// What supply admission reads from a bytes-only package: the bare
/// environment, the package's model selections and population catalog, and
/// the owner of an observation (FR-533 "Observation admission").
struct BareHost<'w, 'p> {
    package: &'w BarePackage<'p>,
    world: &'w BareWorld,
    levels: BTreeMap<NodeKey, usize>,
}

impl SlotHost for BareHost<'_, '_> {
    fn types(&self) -> &filament_semantic_value::declaration::TypeEnvironment {
        self.world.types()
    }

    fn clock_declarations(
        &self,
    ) -> Result<
        fsl_semantics::model::clock_reference::normalization::ClockDeclarationTable,
        CallFailure,
    > {
        clock_declarations(self.package)
    }

    fn binder_level(&self, key: &NodeKey) -> usize {
        self.levels.get(key).copied().unwrap_or(usize::MAX)
    }

    fn population_types(
        &self,
        node: &NodeKey,
    ) -> Option<(
        &filament_checked::key::DeclarationKey,
        &[filament_exact::EffectiveId],
    )> {
        self.package.population_types(node)
    }

    fn holds_model(&self, model: &fsl_semantics::model::domain_package::DomainPackageRef) -> bool {
        self.package.model_selections().contains(model)
    }

    fn observation_is_for_view(
        &self,
        observation: &fsl_package::AdmittedProtocolObservation<'_>,
    ) -> bool {
        observation.is_for_bare_package(self.package)
    }
}

/// FR-533 "The call": evaluate `slot`, already selected from `package`'s
/// nodes, over `supply`.
///
/// # Errors
///
/// A [`CallFailure`]: a view-creation refusal as `Admission` with the
/// catalog pair `unknown_required_feature`/`unsupported-feature` before any
/// charge, a supply refusal as FR-390 states it, or a fault.
pub fn evaluate_selected_protocol_slot(
    package: &BarePackage<'_>,
    slot: &BareSlot,
    supply: &ProtocolSupply<'_>,
    meter: &mut Meter,
) -> Result<ProtocolSlotEvaluation, CallFailure> {
    evaluate_selected_protocol_slot_with(package, slot, supply, meter, admit_slot_supply)
}

/// Internal model-engine entry for already generated binders. This is not a
/// public FR-390 supply port and never manufactures missing origin metadata.
pub(crate) fn evaluate_generated_package_protocol_slot(
    package: &BarePackage<'_>,
    descriptor: &ProtocolSlot,
    supply: &ProtocolSupply<'_>,
    meter: &mut Meter,
) -> Result<ProtocolSlotEvaluation, CallFailure> {
    let descriptor = ProtocolSlotDescriptor::new(
        descriptor.node.clone(),
        descriptor.member.clone(),
        descriptor.index,
    );
    let selected = package
        .select_protocol_slot(&descriptor)
        .map_err(|failure| refusal(package, failure))?;
    evaluate_selected_protocol_slot_with(
        package,
        &selected,
        supply,
        meter,
        super::slot_supply::admit_generated_slot_supply,
    )
}

fn evaluate_selected_protocol_slot_with(
    package: &BarePackage<'_>,
    slot: &BareSlot,
    supply: &ProtocolSupply<'_>,
    meter: &mut Meter,
    admit: impl FnOnce(
        &dyn SlotHost,
        &[SourceView<'_>],
        usize,
        &ProtocolSupply<'_>,
    ) -> Result<AdmittedSupply, CallFailure>,
) -> Result<ProtocolSlotEvaluation, CallFailure> {
    let world = package
        .world()
        .map_err(|failure| refusal(package, failure))?;
    let projection = package
        .project_protocol_slot(slot)
        .map_err(|failure| refusal(package, failure))?;
    if slot
        .sources()
        .iter()
        .any(|source| projection.local_slot(source.binder()) != Some(source.index()))
    {
        return Err(CallFailure::Fault(InternalFault::new(
            "FR-533",
            "bare-slot-binder-is-the-selected-frame-slot",
        )));
    }
    let sources = slot
        .sources()
        .iter()
        .map(|source| SourceView {
            index: source.index(),
            value_type: source.value_type(),
            binder: source.binder(),
        })
        .collect::<Vec<_>>();
    let host = BareHost {
        package,
        world: &world,
        levels: slot
            .sources()
            .iter()
            .map(|source| (source.binder().clone(), source.level()))
            .collect(),
    };
    let AdmittedSupply {
        mut frame,
        reference_origins,
        unavailable,
        binder_slots,
    } = admit(&host, &sources, slot.frame_len(), supply)?;
    let empty = ObjectEnvironment::default();
    let objects = supply
        .observation
        .and_then(|admission| admission.authority().ok())
        .map(|observation| observation.environment())
        .unwrap_or(&empty);
    let no_reads = BTreeMap::new();
    // The call owns the meter's interval capture while it runs (FR-520-AC-11).
    let caller_capture = meter.take_charge_accounting().is_some();
    meter.enable_charge_accounting();
    frame.resize(projection.frame_slots().max(frame.len()), None);
    let run = Machine::with_world(
        World::Bytes(&world),
        objects,
        None,
        Some(&no_reads),
        meter,
        &[],
    )
    .with_clock_declarations(clock_declarations(package)?)
    .with_reference_origins(reference_origins)
    .run_protocol_frame(
        ReaderNode::root(&projection),
        frame,
        unavailable,
        binder_slots,
    )
    .map_err(CallFailure::Fault);
    if !caller_capture {
        meter.disable_charge_accounting();
    }
    let (result, charges) = run?;
    Ok(match result {
        Ok(evaluation) => ProtocolSlotEvaluation::Evaluated(evaluation, charges),
        Err((binder, cause, location)) => ProtocolSlotEvaluation::Unavailable {
            binder,
            cause,
            location,
            charges,
        },
    })
}

/// A capability of the source route the bytes do not carry (FR-531
/// "Capability inventory"): refused at the `Package` locus, with no charge.
fn unmet(package: &BarePackage<'_>, reason: &'static str) -> CallFailure {
    let mut record = super::occurrence_record(
        "unknown_required_feature",
        "unsupported-feature",
        Vec::new(),
    );
    record
        .fields
        .insert("package", package.package().name().to_owned());
    record.fields.insert("reason", reason.to_owned());
    CallFailure::Admission(Box::new(CallAdmissionFailure::Refused(
        filament_eval::CallAdmissionRecord {
            record,
            locus: CallAdmissionLocus::Package,
        },
    )))
}

/// The values of a clause term's local slots, in slot order (FR-337
/// "Captures at S3"): `over` and the trigger binder, which the source route
/// binds to the same object, and each capture the term reads. The slots are
/// the term's own binder positions, so every one is given.
fn clause_arguments(
    term: &BareClauseTerm,
    over: &Value,
    captures: &[Value],
) -> Result<Vec<Value>, CallFailure> {
    let mut arguments = vec![None; term.binders().len()];
    for (slot, binder) in term.binders() {
        let value = match binder {
            ClauseBinder::Over | ClauseBinder::Trigger => over.clone(),
            ClauseBinder::Capture(index) => captures.get(*index).cloned().ok_or_else(|| {
                CallFailure::Fault(InternalFault::new(
                    "FR-860",
                    "clause-term-capture-is-supplied",
                ))
            })?,
        };
        let entry = arguments.get_mut(*slot).ok_or_else(|| {
            CallFailure::Fault(InternalFault::new("FR-940", "clause-binder-slot-in-frame"))
        })?;
        *entry = Some(value);
    }
    arguments
        .into_iter()
        .map(|value| {
            value.ok_or_else(|| {
                CallFailure::Fault(InternalFault::new("FR-940", "clause-binder-slot-is-bound"))
            })
        })
        .collect()
}

/// The operation table a package-route model-check input carries (FR-940;
/// the seam QSL-2404's input struct implements): the declared frame effect of
/// an operation by its declaring object type's effective identity and its
/// name. The source route reads the effect from its checked frame; the
/// package's frame node carries its grants in canonical order but not the
/// declared order a frame witness lists, so the package form reads the
/// declared effect from the input's operation table.
pub trait PackageOperations {
    /// The declared effect of `operation` declared by `declaring`; `None`
    /// when the type declares no such operation.
    fn declared_effect(
        &self,
        declaring: &filament_exact::EffectiveId,
        operation: &str,
    ) -> Option<&fsl_semantics::model::domain_package::OperationEffect>;
}

impl PackageOperations for fsl_semantics::model::operation::OperationTable {
    fn declared_effect(
        &self,
        declaring: &filament_exact::EffectiveId,
        operation: &str,
    ) -> Option<&fsl_semantics::model::domain_package::OperationEffect> {
        self.declared_by(declaring.clone())
            .iter()
            .find(|declared| declared.name() == operation)
            .map(fsl_semantics::model::operation::OperationDeclaration::effect)
    }
}

/// FR-940 package form of [`CheckedPackageEvaluation::evaluate_frame`]: check
/// `invocation` against the operation frame whose frame node key is `frame`,
/// the operation read from the package's anchor node and its declared effect
/// from `operations`. Returns the source form's value: the same verdict,
/// witness and (empty) location and losses, charging nothing.
///
/// # Errors
///
/// `InputRefusal::ObservationsMismatch` when `invocation` was admitted for
/// another frame, a selection refusal at its locus for a frame the package
/// does not bind, and a fault for an operation `operations` does not declare.
pub fn evaluate_frame(
    package: &BarePackage<'_>,
    operations: &impl PackageOperations,
    frame: &NodeKey,
    invocation: &fsl_semantics::model::observation::AdmittedInvocation<'_>,
) -> Result<FrameEvaluation, CallFailure> {
    if invocation.frame != *frame {
        return Err(InputRefusal::ObservationsMismatch.into());
    }
    let bound = package
        .frame_operation(frame)
        .map_err(|failure| refusal(package, failure))?;
    let effect = operations
        .declared_effect(&bound.declaring, &bound.operation)
        .ok_or_else(|| {
            CallFailure::Fault(InternalFault::new(
                "S6a",
                "checked-identity-not-resolved-by-package",
            ))
        })?;
    let (outcome, witness) = super::s6a::protocol_clause::frame_outcome(invocation, effect)
        .map_err(CallFailure::Fault)?;
    Ok(FrameEvaluation {
        evaluation: Evaluation {
            outcome: outcome.into(),
            location: None,
            losses: Vec::new(),
            frame_depth: 0,
            peak_live_occurrences: 0,
        },
        witness,
    })
}

/// The state clause `name` names over `observations`, read from the node the
/// observations were admitted for. The package carries no state clause name
/// until FSpec FR-322 `claim_names` is admitted (QSL-2410): the node is the
/// one `observations` names, `name` locates its body, and a node that is no
/// state clause is `ObservationsMismatch`.
fn state_clause(
    package: &BarePackage<'_>,
    name: &QualifiedName,
    observations: &fsl_semantics::model::observation::AdmittedObservations,
) -> Result<fsl_package::BareStateClause, CallFailure> {
    let declared = name
        .as_unqualified()
        .ok_or_else(|| InputRefusal::UnknownClause(name.to_string()))?;
    package
        .project_state_clause(&observations.clause, declared)
        .map_err(|failure| match failure {
            BareSelectionFailure::Slot(
                fsl_package::SlotSelectionFailure::MissingNode(_)
                | fsl_package::SlotSelectionFailure::MalformedNode(_),
            ) => InputRefusal::ObservationsMismatch.into(),
            other => refusal(package, other),
        })
}

/// FR-940 package form of [`CheckedPackageEvaluation::evaluate_clause`]: the
/// clause body read from its admitted node, bound over `observations` as the
/// source form binds it, and run through the shared evaluator with its
/// trail. A `pre(..)` read observes the pre snapshot. `result` is bound
/// exactly when the clause is a postcondition, takes it in slot 1 and the
/// admitted observations carry the invocation's result, which they carry
/// only for an operation that declares one (FR-104).
fn evaluate_state_clause(
    package: &BarePackage<'_>,
    name: &QualifiedName,
    observations: &fsl_semantics::model::observation::AdmittedObservations,
    meter: &mut Meter,
) -> Result<ClauseEvaluation, CallFailure> {
    use fsl_semantics::model::clause_observation::Observation;
    let clause = state_clause(package, name, observations)?;
    let world = package
        .world()
        .map_err(|failure| refusal(package, failure))?;
    let names: Vec<String> = clause
        .parameters()
        .iter()
        .map(|(parameter, _)| parameter.clone())
        .collect();
    if clause
        .parameters()
        .iter()
        .enumerate()
        .any(|(position, (_, slot))| position != *slot)
    {
        return Err(CallFailure::Fault(InternalFault::new(
            "FR-940",
            "state-clause-parameters-take-their-slots-in-order",
        )));
    }
    let binds_result = clause.observation() == Observation::Post
        && names.len() > 1
        && observations.result.is_some();
    let super::ClauseBinding {
        current,
        pre,
        bindings,
        reference_origins,
        trail,
        receiver_path,
    } = super::bind_clause(clause.observation(), &names, binds_result, observations)?;
    let reads: BTreeMap<filament_semantic_value::location::ExpressionLocation, Observation> =
        clause
            .projection()
            .pre_read_locations()
            .into_iter()
            .map(|location| (location, Observation::Pre))
            .collect();
    let (evaluation, trail) =
        Machine::with_world(World::Bytes(&world), current, pre, Some(&reads), meter, &[])
            .with_clock_declarations(clock_declarations(package)?)
            .with_reference_origins(reference_origins)
            .with_trail(trail)
            .run_traced(
                ReaderNode::root(clause.projection()),
                clause.projection().frame_slots(),
                bindings,
            )
            .map_err(CallFailure::Fault)?;
    Ok(ClauseEvaluation::new(evaluation, trail, receiver_path))
}

/// FR-940: the state-graph predicate of `role` of the claim at `claim` on
/// the package route, read from its admitted node and run through the shared
/// evaluator at the state whose objects are `current`, with the `over`
/// parameter at slot 0 bound to `over` and `further` after it. A claim
/// instance whose key holds no object (`absent`) reads the parameter as
/// `absent-key` undefined, `key` standing for the reference where it is a
/// `lookup`'s operand.
///
/// # Errors
///
/// An internal fault when the claim or its predicate does not read, which
/// the package form of the claim has already established.
pub(crate) fn evaluate_state_graph_predicate(
    package: &BarePackage<'_>,
    claim: &fsl_foundation::source::provenance::OccurrenceKey,
    role: fsl_semantics::check::PredicateRole,
    objects: &ObjectEnvironment,
    over: StateGraphOver,
    further: Vec<Value>,
    meter: &mut Meter,
) -> Result<Evaluation, InternalFault> {
    let fault = |cause: &'static str| InternalFault::new("FR-940", cause);
    let predicate = package
        .project_state_graph_predicate(claim, role)
        .map_err(|_| fault("state-graph-predicate-projects"))?;
    evaluate_projected_state_predicate(package, &predicate, objects, over, further, meter)
}

pub(crate) fn evaluate_probability_predicate(
    package: &BarePackage<'_>,
    claim: &fsl_foundation::source::provenance::OccurrenceKey,
    objects: &ObjectEnvironment,
    over: Option<filament_exact::ObjectReference>,
    meter: &mut Meter,
) -> Result<Evaluation, InternalFault> {
    let predicate = package
        .project_probability_predicate(claim)
        .map_err(|_| InternalFault::new("FR-940", "probability-predicate-projects"))?;
    let over = over.map_or(StateGraphOver::None, StateGraphOver::Bound);
    evaluate_projected_state_predicate(package, &predicate, objects, over, Vec::new(), meter)
}

fn evaluate_projected_state_predicate(
    package: &BarePackage<'_>,
    predicate: &fsl_package::BareStatePredicate,
    objects: &ObjectEnvironment,
    over: StateGraphOver,
    mut further: Vec<Value>,
    meter: &mut Meter,
) -> Result<Evaluation, InternalFault> {
    let fault = |cause: &'static str| InternalFault::new("FR-940", cause);
    let world = package
        .world()
        .map_err(|_| fault("state-graph-predicate-world"))?;
    if predicate
        .parameters()
        .iter()
        .enumerate()
        .any(|(position, (_, slot))| position != *slot)
    {
        return Err(fault("state-graph-parameters-take-their-slots-in-order"));
    }
    // The source route evaluates a state-graph predicate as a state-clause
    // body (`Machine::with_pre`, no observations of its own), so its field
    // reads carry FR-107's model-read charge; the same family here.
    let reads = std::collections::BTreeMap::new();
    let mut machine = Machine::with_world(
        World::Bytes(&world),
        objects,
        None,
        Some(&reads),
        meter,
        &[],
    );
    let mut bindings = Vec::new();
    match over {
        StateGraphOver::None => {}
        StateGraphOver::Bound(reference) => bindings.push(Value::Reference(reference)),
        StateGraphOver::Absent { key, binding } => {
            machine = machine.with_absent_parameter(0, binding);
            bindings.push(Value::Reference(key));
        }
    }
    bindings.append(&mut further);
    machine.run(
        ReaderNode::root(predicate.projection()),
        predicate.projection().frame_slots(),
        bindings,
    )
}

/// How a state-graph predicate's `over` parameter is bound.
pub(crate) enum StateGraphOver {
    /// The claim has no `over` parameter.
    None,
    /// The instance's object.
    Bound(filament_exact::ObjectReference),
    /// The instance's key holds no object: the reference of the parameter's
    /// type under the key, and the population's name for the undefined value.
    Absent {
        key: filament_exact::ObjectReference,
        binding: String,
    },
}

/// FR-940 package form of [`super::source_post_targets`]: the targets of
/// the postcondition `name` read from its admitted node, bound over
/// `observations` as [`evaluate_state_clause`] binds it, each `e` evaluated
/// with a meter from `meter`.
///
/// # Errors
///
/// As [`evaluate_state_clause`].
pub(crate) fn post_targets(
    package: &BarePackage<'_>,
    name: &QualifiedName,
    observations: &fsl_semantics::model::observation::AdmittedObservations,
    meter: &mut dyn FnMut() -> Meter,
) -> Result<super::post_targets::PostTargets, CallFailure> {
    use fsl_semantics::model::clause_observation::Observation;
    let clause = state_clause(package, name, observations)?;
    let world = package
        .world()
        .map_err(|failure| refusal(package, failure))?;
    let names: Vec<String> = clause
        .parameters()
        .iter()
        .map(|(parameter, _)| parameter.clone())
        .collect();
    let binds_result = clause.observation() == Observation::Post
        && names.len() > 1
        && observations.result.is_some();
    let binding = super::bind_clause(clause.observation(), &names, binds_result, observations)?;
    let reads: BTreeMap<filament_semantic_value::location::ExpressionLocation, Observation> =
        clause
            .projection()
            .pre_read_locations()
            .into_iter()
            .map(|location| (location, Observation::Pre))
            .collect();
    let mut frame: Vec<Option<Value>> = binding.bindings.into_iter().map(Some).collect();
    frame.resize(clause.projection().frame_slots().max(frame.len()), None);
    super::post_targets::targets(
        ReaderNode::root(clause.projection()),
        binds_result,
        |node| {
            let mut meter = meter();
            Machine::with_world(
                World::Bytes(&world),
                binding.current,
                binding.pre,
                Some(&reads),
                &mut meter,
                &[],
            )
            .run_frame(node, frame.clone())
            .map(|evaluation| evaluation.outcome)
            .map_err(CallFailure::Fault)
        },
    )
}

/// A reward a package's `reward` node carries, projected once: the clause
/// function its expression names, and the type of the value it computes
/// (FR-187, ADR-024 PM-5).
pub struct ProjectedReward {
    projection: fsl_package::ReaderProtocolProjection,
    slots: usize,
    value_type: filament_exact::ValueType,
}

impl ProjectedReward {
    /// The projection of `reward`'s clause function over `package`.
    ///
    /// # Errors
    ///
    /// The [`BareSelectionFailure`] of the first function, type or equality
    /// the projection cannot read, or of a node that is no function.
    pub fn of(
        package: &BarePackage<'_>,
        reward: &fsl_semantics::check::ReadReward,
    ) -> Result<Self, BareSelectionFailure> {
        let projection = package.project_clause_function(&reward.function)?;
        let selected = projection.selected_function().ok_or_else(|| {
            fsl_package::SlotSelectionFailure::MalformedNode(reward.function.clone())
        })?;
        let (slots, value_type) = (selected.slots, selected.result_type.clone());
        Ok(Self {
            projection,
            slots,
            value_type,
        })
    }

    /// The type of the value the reward computes.
    pub fn value_type(&self) -> &filament_exact::ValueType {
        &self.value_type
    }

    /// The reward evaluated on `arguments`, one value per parameter of its
    /// operation in declared order, over the package's `world`. A reward
    /// reads its operation's parameters alone, so the object environment is
    /// empty.
    ///
    /// # Errors
    ///
    /// An internal fault, as the evaluator reports it.
    pub fn evaluate(
        &self,
        world: &BareWorld,
        arguments: Vec<Value>,
        meter: &mut Meter,
    ) -> Result<Evaluation, InternalFault> {
        let objects = ObjectEnvironment::default();
        Machine::from_bare_package(world, &objects, meter).run(
            ReaderNode::root(&self.projection),
            self.slots,
            arguments,
        )
    }
}

impl super::clause_set::ClauseHost for BarePackage<'_> {
    fn clause_observation(
        &self,
        name: &QualifiedName,
        observations: &fsl_semantics::model::observation::AdmittedObservations,
    ) -> Result<fsl_semantics::model::clause_observation::Observation, CallFailure> {
        Ok(state_clause(self, name, observations)?.observation())
    }

    fn clause(
        &self,
        name: &QualifiedName,
        observations: &fsl_semantics::model::observation::AdmittedObservations,
        meter: &mut Meter,
    ) -> Result<ClauseEvaluation, CallFailure> {
        evaluate_state_clause(self, name, observations, meter)
    }
}
const SOURCE_DECLARATION: &str = "needs-source-declaration";
const SOURCE_CHECKED_FORM: &str = "needs-source-checked-form";
const OPERATION_TABLE: &str = "needs-operation-table";

impl CheckedPackageEvaluation for BarePackage<'_> {
    fn call(
        &self,
        function: &QualifiedName,
        arguments: Vec<Value>,
        objects: &ObjectEnvironment,
        meter: &mut Meter,
    ) -> Result<Evaluation, CallFailure> {
        let cancel = meter.cancel().cloned().unwrap_or_default();
        let mut run = SuppliedRun::new(SuppliedLimits::default(), cancel)
            .map_err(|stop| CallFailure::from(SuppliedFailure::Stop(stop)))?;
        self.call_supplied(function, arguments, objects, meter, &mut run)
    }

    fn call_observing_applications(
        &self,
        function: &QualifiedName,
        arguments: Vec<Value>,
        objects: &ObjectEnvironment,
        meter: &mut Meter,
        options: ApplicationObservationOptions,
    ) -> ApplicationObservation {
        let displaced_capture = meter.disable_charge_capture();
        let mut capture = super::OperationCapture::new(options);
        capture.start(meter);
        let mut observed = Vec::new();
        let mut unavailable_imported_call = false;
        let mut function_evaluations = 0;
        let mut function_case_limit_reached = false;
        let evaluation = (|| {
            let projection = function_projection(self, function)?;
            let selected = projection.selected_function().ok_or_else(|| {
                CallFailure::Fault(InternalFault::new(
                    "FR-945",
                    "bare-call-has-selected-function",
                ))
            })?;
            let parameters = selected
                .parameters
                .iter()
                .map(|parameter| (parameter.name.clone(), parameter.value_type.clone()))
                .collect::<Vec<_>>();
            let body_location = selected
                .body_location
                .or_else(|| projection.location(selected.body));
            let world = self.world().map_err(|failure| refusal(self, failure))?;
            let cancel = meter.cancel().cloned().unwrap_or_default();
            let mut run = SuppliedRun::new(SuppliedLimits::default(), cancel)
                .map_err(|stop| CallFailure::from(SuppliedFailure::Stop(stop)))?;
            super::admit_arguments(&parameters, &arguments, objects, world.types(), &mut run)?;
            if let Err(incomplete) = meter.charge(Charge::new(ChargePoint::FunctionCall)) {
                return Ok(Evaluation {
                    outcome: FamilyOutcome::Evaluated(Outcome::Incomplete(incomplete)),
                    location: body_location.cloned(),
                    losses: Vec::new(),
                    frame_depth: 0,
                    peak_live_occurrences: 0,
                });
            }
            capture.drain(meter, body_location);
            if options.function_case_limit == 0 {
                function_case_limit_reached = true;
                return Ok(Evaluation {
                    outcome: FamilyOutcome::Evaluated(Outcome::Incomplete(
                        filament_exact::Incomplete {
                            limit_kind: filament_exact::LimitKind::WorkUnits,
                            limit: 0,
                            consumed: 0,
                            next_charge: filament_exact::Integer::from(1_u64),
                            charge_point: ChargePoint::FunctionCall,
                        },
                    )),
                    location: body_location.cloned(),
                    losses: Vec::new(),
                    frame_depth: 0,
                    peak_live_occurrences: 0,
                });
            }
            function_evaluations = 1;
            Machine::observing_bare_package(
                &world,
                objects,
                meter,
                super::ApplicationObserver {
                    observed: &mut observed,
                    unavailable_imported_call: &mut unavailable_imported_call,
                    function_case_limit: options.function_case_limit,
                    function_evaluations: &mut function_evaluations,
                    function_case_limit_reached: &mut function_case_limit_reached,
                    capture: &mut capture,
                },
            )
            .run(ReaderNode::root(&projection), selected.slots, arguments)
            .map_err(CallFailure::Fault)
        })();
        let ledger = capture.finish(meter);
        ApplicationObservation {
            evaluation,
            observed,
            unavailable_imported_call,
            function_evaluations,
            function_case_limit_reached,
            meter_totals: super::ChargeAmounts::from_meter(meter),
            ledger,
            displaced_capture,
        }
    }

    fn call_supplied(
        &self,
        function: &QualifiedName,
        arguments: Vec<Value>,
        objects: &ObjectEnvironment,
        meter: &mut Meter,
        run: &mut SuppliedRun,
    ) -> Result<Evaluation, CallFailure> {
        let projection = function_projection(self, function)?;
        let selected = projection.selected_function().ok_or_else(|| {
            CallFailure::Fault(InternalFault::new(
                "FR-945",
                "bare-call-has-selected-function",
            ))
        })?;
        let parameters = selected
            .parameters
            .iter()
            .map(|parameter| (parameter.name.clone(), parameter.value_type.clone()))
            .collect::<Vec<_>>();
        let body_location = selected
            .body_location
            .or_else(|| projection.location(selected.body));
        let world = self.world().map_err(|failure| refusal(self, failure))?;
        super::admit_arguments(&parameters, &arguments, objects, world.types(), run)?;
        if let Err(incomplete) = meter.charge(Charge::new(ChargePoint::FunctionCall)) {
            return Ok(Evaluation {
                outcome: FamilyOutcome::Evaluated(Outcome::Incomplete(incomplete)),
                location: body_location.cloned(),
                losses: Vec::new(),
                frame_depth: 0,
                peak_live_occurrences: 0,
            });
        }
        Machine::from_bare_package(&world, objects, meter)
            .run(ReaderNode::root(&projection), selected.slots, arguments)
            .map_err(CallFailure::Fault)
    }

    fn evaluate(
        &self,
        _expression: &fsl_semantics::check::CheckedExpression,
        _arguments: Vec<Value>,
        _objects: &ObjectEnvironment,
        _meter: &mut Meter,
    ) -> Result<Evaluation, CallFailure> {
        Err(unmet(self, SOURCE_CHECKED_FORM))
    }

    fn evaluate_protocol_slot(
        &self,
        slot: &ProtocolSlot,
        supply: &ProtocolSupply<'_>,
        meter: &mut Meter,
    ) -> Result<ProtocolSlotEvaluation, CallFailure> {
        let descriptor =
            ProtocolSlotDescriptor::new(slot.node.clone(), slot.member.clone(), slot.index);
        let selected = self
            .select_protocol_slot(&descriptor)
            .map_err(|failure| refusal(self, failure))?;
        evaluate_selected_protocol_slot(self, &selected, supply, meter)
    }

    fn evaluate_selected_protocol_slot(
        &self,
        _view: &fsl_package::AdmittedEvaluationView<'_>,
        selected: &fsl_package::SelectedProtocolTerm<'_, '_>,
        _supply: &ProtocolSupply<'_>,
        _meter: &mut Meter,
    ) -> Result<ProtocolSlotEvaluation, CallFailure> {
        // A view and a selection of the source side belong to a source
        // package, never to this one (use `evaluate_selected_protocol_slot`
        // of this module for a slot selected from the bytes).
        Err(resolve::refused(
            "invalid_model_binding",
            "wrong-model-selection",
            CallAdmissionLocus::Node(selected.root().node().clone()),
        ))
    }

    fn evaluate_holds(
        &self,
        atom: &super::HoldsAtom,
        over: Value,
        captures: &[Value],
        objects: &ObjectEnvironment,
        meter: &mut Meter,
    ) -> Result<Evaluation, CallFailure> {
        let world = self.world().map_err(|failure| refusal(self, failure))?;
        let term = self
            .project_holds(&atom.clause, atom.position)
            .map_err(|failure| refusal(self, failure))?;
        let arguments = clause_arguments(&term, &over, captures)?;
        Machine::from_bare_package(&world, objects, meter)
            .run(
                ReaderNode::root(term.projection()),
                term.projection().frame_slots(),
                arguments,
            )
            .map_err(CallFailure::Fault)
    }

    fn evaluate_package_capture(
        &self,
        clause: &NodeKey,
        index: usize,
        over: Value,
        captures: &[Value],
        objects: &ObjectEnvironment,
        meter: &mut Meter,
    ) -> Result<Evaluation, CallFailure> {
        let world = self.world().map_err(|failure| refusal(self, failure))?;
        let term = self
            .project_capture(clause, index)
            .map_err(|failure| refusal(self, failure))?;
        let arguments = clause_arguments(&term, &over, captures)?;
        Machine::from_bare_package(&world, objects, meter)
            .run(
                ReaderNode::root(term.projection()),
                term.projection().frame_slots(),
                arguments,
            )
            .map_err(CallFailure::Fault)
    }

    #[allow(clippy::too_many_arguments)] // each value is an explicit evaluation input
    fn evaluate_model_holds(
        &self,
        atom: &super::HoldsAtom,
        over: Value,
        absent_population: Option<&str>,
        captures: &[Value],
        objects: &ObjectEnvironment,
        reference_bindings: &crate::simulation::model::ReferenceBindings,
        meter: &mut Meter,
    ) -> Result<Evaluation, CallFailure> {
        let world = self.world().map_err(|failure| refusal(self, failure))?;
        let term = self
            .project_holds(&atom.clause, atom.position)
            .map_err(|failure| refusal(self, failure))?;
        let arguments = clause_arguments(&term, &over, captures)?;
        let reads = BTreeMap::new();
        let mut machine = Machine::with_world(
            World::Bytes(&world),
            objects,
            None,
            Some(&reads),
            meter,
            &[],
        )
        .with_reference_bindings(reference_bindings);
        if let Some(population) = absent_population {
            let Value::Reference(key) = &over else {
                return Err(CallFailure::Fault(InternalFault::new(
                    "FR-125",
                    "absent-model-over-is-reference",
                )));
            };
            for (slot, binder) in term.binders() {
                if matches!(binder, ClauseBinder::Over | ClauseBinder::Trigger) {
                    machine = machine.with_absent_key(*slot, population.to_owned(), key.clone());
                }
            }
        }
        machine
            .run(
                ReaderNode::root(term.projection()),
                term.projection().frame_slots(),
                arguments,
            )
            .map_err(CallFailure::Fault)
    }

    fn evaluate_holds_traced(
        &self,
        atom: &super::HoldsAtom,
        binders: super::HoldsBinders<'_>,
        objects: &ObjectEnvironment,
        observation: super::ObservationIdentity,
        meter: &mut Meter,
    ) -> Result<ClauseEvaluation, CallFailure> {
        let super::HoldsBinders { over, captures } = binders;
        let world = self.world().map_err(|failure| refusal(self, failure))?;
        let term = self
            .project_holds(&atom.clause, atom.position)
            .map_err(|failure| refusal(self, failure))?;
        let arguments = clause_arguments(&term, &Value::Reference(over.clone()), captures)?;
        // `over` is the condition's receiver: a value read from it is rooted
        // at its declared object identity (FR-107 "The receiver path"), at
        // whichever local slot the projection gave it.
        let subject = super::s6a::separation::ValuePathSubject::Object(over.clone());
        let mut trail = super::s6a::separation::Trail::new(observation.clone(), None);
        trail.receiver = Some(over.clone());
        trail.bindings = term.binders().iter().fold(
            vec![None; arguments.len()],
            |mut bindings, (slot, binder)| {
                if *binder == ClauseBinder::Over {
                    if let Some(entry) = bindings.get_mut(*slot) {
                        *entry = Some(subject.clone());
                    }
                }
                bindings
            },
        );
        let (evaluation, trail) = Machine::from_bare_package(&world, objects, meter)
            .with_trail(trail)
            .run_traced(
                ReaderNode::root(term.projection()),
                term.projection().frame_slots(),
                arguments,
            )
            .map_err(CallFailure::Fault)?;
        Ok(ClauseEvaluation::new(
            evaluation,
            trail,
            Some(super::RuntimeValuePath {
                observation,
                subject,
                steps: Vec::new(),
            }),
        ))
    }

    fn evaluate_guard(
        &self,
        clause: &NodeKey,
        over: Value,
        objects: &ObjectEnvironment,
        meter: &mut Meter,
    ) -> Result<Evaluation, CallFailure> {
        let world = self.world().map_err(|failure| refusal(self, failure))?;
        let term = self
            .project_guard(clause)
            .map_err(|failure| refusal(self, failure))?
            .ok_or_else(|| {
                CallFailure::Fault(InternalFault::new("S6a", "guard-resolved-by-clause"))
            })?;
        let arguments = clause_arguments(&term, &over, &[])?;
        Machine::from_bare_package(&world, objects, meter)
            .run(
                ReaderNode::root(term.projection()),
                term.projection().frame_slots(),
                arguments,
            )
            .map_err(CallFailure::Fault)
    }

    fn evaluate_model_guard(
        &self,
        clause: &NodeKey,
        over: Value,
        absent_population: Option<&str>,
        objects: &ObjectEnvironment,
        reference_bindings: &crate::simulation::model::ReferenceBindings,
        meter: &mut Meter,
    ) -> Result<Evaluation, CallFailure> {
        let world = self.world().map_err(|failure| refusal(self, failure))?;
        let term = self
            .project_guard(clause)
            .map_err(|failure| refusal(self, failure))?
            .ok_or_else(|| {
                CallFailure::Fault(InternalFault::new("S6a", "guard-resolved-by-clause"))
            })?;
        let arguments = clause_arguments(&term, &over, &[])?;
        let reads = BTreeMap::new();
        let mut machine = Machine::with_world(
            World::Bytes(&world),
            objects,
            None,
            Some(&reads),
            meter,
            &[],
        )
        .with_reference_bindings(reference_bindings);
        if let Some(population) = absent_population {
            let Value::Reference(key) = &over else {
                return Err(CallFailure::Fault(InternalFault::new(
                    "FR-125",
                    "absent-model-over-is-reference",
                )));
            };
            for (slot, binder) in term.binders() {
                if matches!(binder, ClauseBinder::Over | ClauseBinder::Trigger) {
                    machine = machine.with_absent_key(*slot, population.to_owned(), key.clone());
                }
            }
        }
        machine
            .run(
                ReaderNode::root(term.projection()),
                term.projection().frame_slots(),
                arguments,
            )
            .map_err(CallFailure::Fault)
    }

    fn evaluate_hyper_holds(
        &self,
        clause: &fsl_semantics::check::CheckedHyperClause,
        atom: fsl_semantics::check::FormulaNodeId,
        slots: Vec<crate::hyper::HyperSlot>,
        components: &[&ObjectEnvironment],
        meter: &mut Meter,
    ) -> Result<Evaluation, CallFailure> {
        evaluate_hyper_holds(self, clause, atom, slots, components, meter)
    }

    fn evaluate_step_predicate(
        &self,
        _clause: &fsl_semantics::check::CheckedHyperClause,
        _part: crate::hyper::MatchPart,
        _conjunct: usize,
        _labels: &[crate::hyper::StepLabel<'_>],
        _meter: &mut Meter,
    ) -> Result<Evaluation, CallFailure> {
        Err(unmet(self, SOURCE_CHECKED_FORM))
    }

    fn evaluate_clause(
        &self,
        clause: &QualifiedName,
        observations: &fsl_semantics::model::observation::AdmittedObservations,
        meter: &mut Meter,
    ) -> Result<ClauseEvaluation, CallFailure> {
        evaluate_state_clause(self, clause, observations, meter)
    }

    fn evaluate_clause_set(
        &self,
        clauses: &[(
            QualifiedName,
            &fsl_semantics::model::observation::AdmittedObservations,
        )],
        limits: filament_exact::ScalarLimits,
    ) -> Result<ClauseSetEvaluation, CallFailure> {
        super::clause_set::evaluate(self, clauses, limits)
    }

    fn evaluate_value_invariants(
        &self,
        _observations: &fsl_semantics::model::observation::AdmittedObservations,
        _meter: &mut Meter,
    ) -> Result<ValueInvariantEvaluation, CallFailure> {
        Err(unmet(self, SOURCE_DECLARATION))
    }

    fn evaluate_current_value_invariants(
        &self,
        _current: &fsl_semantics::model::observation::Observation,
        _meter: &mut Meter,
    ) -> Result<ValueInvariantEvaluation, CallFailure> {
        Err(unmet(self, SOURCE_DECLARATION))
    }

    fn evaluate_value_invariant_selection(
        &self,
        _observations: &fsl_semantics::model::observation::ValueInvariantObservations,
        _meter: &mut Meter,
    ) -> Result<ValueInvariantEvaluation, CallFailure> {
        Err(unmet(self, SOURCE_DECLARATION))
    }

    fn check_separation(
        &self,
        _clause: &QualifiedName,
        _observations: &fsl_semantics::model::observation::AdmittedObservations,
        _claim: super::WitnessClaim<'_>,
        _meter: &mut Meter,
    ) -> Result<Separation, CallFailure> {
        Err(unmet(self, SOURCE_DECLARATION))
    }

    fn evaluate_frame(
        &self,
        _frame: NodeKey,
        _invocation: &fsl_semantics::model::observation::AdmittedInvocation<'_>,
        _meter: &mut Meter,
    ) -> Result<FrameEvaluation, CallFailure> {
        // The frame's declared effect is the model-check input's operation
        // table, which this call does not take: the package form is
        // [`evaluate_frame`] over that table.
        Err(unmet(self, OPERATION_TABLE))
    }

    fn evaluate_collection_write(
        &self,
        _clause: &QualifiedName,
        _observations: &fsl_semantics::model::observation::AdmittedObservations,
        _field: &str,
        _written: &fsl_semantics::check::Written,
        _meter: &mut Meter,
    ) -> Result<Option<super::collection_write::WrittenCollection>, CallFailure> {
        Err(unmet(self, SOURCE_DECLARATION))
    }
}

/// FR-411, FR-940: the `terminal when` condition `predicate` on the package
/// route, read from its admitted node and run through the shared evaluator at
/// the state whose objects are `objects`, with `self` bound to `self_object`
/// when the condition reads it. Like an invariant's, the condition reads no
/// `pre(..)`, so every model read observes `objects`.
///
/// # Errors
///
/// An internal fault when the package's world does not read.
pub(crate) fn evaluate_terminal_predicate(
    package: &BarePackage<'_>,
    predicate: &fsl_package::BareTerminalPredicate,
    objects: &ObjectEnvironment,
    self_object: Option<filament_exact::ObjectReference>,
    meter: &mut Meter,
) -> Result<Evaluation, InternalFault> {
    let world = package
        .world()
        .map_err(|_| InternalFault::new("FR-940", "terminal-predicate-world"))?;
    let bindings = if predicate.reads_self() {
        vec![Value::Reference(self_object.ok_or_else(|| {
            InternalFault::new("FR-124", "terminal-self-object")
        })?)]
    } else {
        Vec::new()
    };
    Machine::from_bare_package(&world, objects, meter).run(
        ReaderNode::root(predicate.projection()),
        predicate.projection().frame_slots(),
        bindings,
    )
}

/// FR-940: run an HP-1 body using only its admitted reader projection.
pub(crate) fn evaluate_model_relation_tuple(
    package: &BarePackage<'_>,
    relation: &crate::simulation::model::ModelRelationDescriptor,
    tuple: crate::simulation::model::ModelRelationTuple<'_>,
    meter: &mut Meter,
) -> Result<super::ModelRelationEvaluation, InternalFault> {
    use fsl_package::AnalysisView;
    use fsl_semantics::model::clause_observation::Observation;
    let fault = || InternalFault::new("S6a", "relation-package-form-projects");
    let declaration = package
        .package()
        .resolved_graph()
        .nodes
        .iter()
        .find(|node| package.node_key(&node.node_id).as_ref() == Some(relation.occurrence().node()))
        .ok_or_else(fault)?
        .node_id
        .clone();
    let claim = fsl_package::ClaimOccurrence {
        declaration,
        occurrence: fsl_package::CheckedOccurrence {
            role: fsl_package::CheckedOccurrenceRole::Claim,
            ordinal: relation.occurrence().origin().ordinal(),
        },
    };
    let form = package.model_relation_form(&claim).map_err(|_| fault())?;
    let body = package.project_model_relation(&form).map_err(|_| fault())?;
    if body.executions() != relation.executions()
        || body.parameters() != relation.parameters()
        || tuple.pre.len() != relation.executions().len()
        || tuple.post.len() != tuple.pre.len()
    {
        return Err(fault());
    }
    let arguments = super::bind_model_relation_transitions(
        relation.executions(),
        body.parameters(),
        tuple.transitions,
        tuple.results,
    )?;
    let current = tuple.post.first().copied().ok_or_else(fault)?;
    let world = package.world().map_err(|_| fault())?;
    let observations = body
        .projection()
        .pre_read_locations()
        .into_iter()
        .map(|location| (location, Observation::Pre))
        .collect::<BTreeMap<_, _>>();
    let evaluation = Machine::from_bare_package(&world, current, meter)
        .with_components(super::evaluate::ComponentReads {
            environments: tuple.post,
            pre_environments: Some(tuple.pre),
            reads: body.components(),
            observations: Some(&observations),
        })
        .run(
            ReaderNode::root(body.projection()),
            body.projection().frame_slots(),
            arguments,
        )?;
    super::settle_model_relation_evaluation(evaluation)
}

#[cfg(test)]
mod tests {
    use super::*;
    use filament_exact::{Integer, LimitKind, ScalarLimits};
    use filament_semantic_value::checking::{CheckMode, CheckingLimits};
    use fsl_forms::{DeclarationSpans, Expression, ExpressionSpans, FunctionDeclaration, TypeForm};
    use fsl_semantics::check::PackageDeclarations;
    use ix_trace_rs::trace;

    const SPAN: fsl_foundation::Span = fsl_foundation::Span { start: 0, end: 0 };
    const UNLIMITED: ScalarLimits = ScalarLimits {
        integer_bits: u64::MAX,
        decimal_digits: u64::MAX,
        scale_expansion: u64::MAX,
        text_input_bytes: u64::MAX,
        text_scalars: u64::MAX,
        normalized_scalars: u64::MAX,
        unit_edges: u64::MAX,
        value_occurrences: u64::MAX,
        work_units: u64::MAX,
        result_units: u64::MAX,
    };

    fn package() -> fsl_package::CheckedPackage {
        let integer = TypeForm::builtin(fsl_forms::BuiltinType::Integer, SPAN);
        let mut declarations = PackageDeclarations::new(
            fsl_semantics::check::fixture_source(),
            "bare-call-test".to_owned(),
            fsl_foundation::IdentityLimits::default(),
        );
        declarations.functions.push(
            FunctionDeclaration::new(
                "one",
                vec![("a".to_owned(), integer.clone())],
                integer,
                None,
                Expression::name("a".to_owned()),
            )
            .with_spans(DeclarationSpans {
                declaration: fsl_foundation::Span { start: 0, end: 20 },
                body: ExpressionSpans::new(fsl_foundation::Span { start: 10, end: 11 })
                    .expect("the body span is ordered"),
                measure: None,
            })
            .expect("the declaration owns its body span"),
        );
        fsl_package::CheckedPackage::link(
            declarations
                .check(
                    CheckMode::Linked,
                    CheckingLimits::default(),
                    &crate::value::S6aRewardEvaluator,
                )
                .expect("the identity function checks"),
        )
    }

    fn population_collection_package() -> (
        fsl_package::CheckedPackage,
        fsl_semantics::model::intake::PackageInput,
    ) {
        let model_bytes =
            include_bytes!("../../../../examples/config-version/model.semantic-ir.json");
        let models = fsl_semantics::model::intake::package_input([model_bytes.as_slice()])
            .expect("model document reads");
        let source_text = "language \"ix:native\" edition \"1-draft\";\n\
             profile v = \"filament.value.complete/v1\";\n\
             model M = \"example/config-version\";\n\
             function takes using v(a: Sequence<Integer>[0, 3], b: Sequence<Integer>[0, 3]): Integer pure { 1 }\n\
             function gives using v(): Sequence<Integer>[0, 3] pure { sequence[1] }\n\
             function population_source using v(p: Population<M::ConfigVersion>[3]): Integer pure { 1 }\n";
        let parsed = fsl_cst::parse(
            fsl_foundation::SourceIdentity::new("test", "bare-call-types"),
            "bare-call-types.native",
            source_text.as_bytes(),
            fsl_cst::Limits::default(),
        )
        .expect("the source unit parses");
        assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
        let unit = fsl_forms::build_unit(&parsed).expect("the source unit builds");
        let selected_models = fsl_semantics::model::intake::admit_unit(
            &unit.selections().models,
            &models,
            fsl_semantics::model::accounting::ModelNormalizationLimits::default(),
        )
        .expect("the model selection admits");
        let declarations = PackageDeclarations::assemble(
            parsed.source().reference().clone(),
            "bare-call-types-test".to_owned(),
            unit,
            selected_models,
            Vec::new(),
        )
        .expect("the declarations assemble");
        let source = fsl_package::CheckedPackage::link(
            declarations
                .check(
                    CheckMode::Linked,
                    CheckingLimits::default(),
                    &crate::value::S6aRewardEvaluator,
                )
                .expect("the source-supported package checks"),
        );
        (source, models)
    }

    /// Public FR-390 supply admission remains strict. The internal engine
    /// can forward its typed owner value, but absent P never authorizes ref.
    #[trace("FR-390-AC-23", "FR-230-AC-71", "TC-3084")]
    #[test]
    fn generated_owner_without_population_keeps_public_admission_strict_and_clock_closed() {
        use super::super::protocol_slot::{ProtocolSuppliedValue, SuppliedValue};
        use fsl_semantics::model::accounting::ModelNormalizationLimits;
        let mut document: serde_json::Value = serde_json::from_slice(include_bytes!(
            "../../../../examples/config-version/model.semantic-ir.json"
        ))
        .expect("real Model fixture");
        let fields = document["types"][1]["fields"]
            .as_array_mut()
            .expect("ConfigVersion fields");
        let mut clock = fields[0].clone();
        clock["identity"] = serde_json::json!("ix://example/config-version/ConfigVersion/x");
        clock["name"] = serde_json::json!("x");
        clock["typeRef"] = serde_json::json!("ix://filament/native/Clock");
        clock["multiplicity"] =
            serde_json::json!({"lower":1,"upper":1,"ordered":false,"unique":false});
        clock["origin"]["generated"]["generatorIdentity"] = clock["identity"].clone();
        fields.push(clock);
        let document = serde_json::to_vec(&document).expect("JSON");
        let models =
            fsl_semantics::model::intake::package_input([document.as_slice()]).expect("Model");
        let source = "language \"ix:native\" edition \"1-draft\";\n\
            profile v = \"filament.value.complete/v1\";\n\
            profile m = \"filament.model.complete/v1\";\n\
            profile t = \"filament.temporal.complete/v1\";\n\
            profile p = \"filament.protocol.finite-global/v1\";\n\
            model M = \"example/config-version\";\n\
            dimension Time; unit ms : Time = rational(1, 1); time M dense unit ms;\n\
            protocol Owners using p over (c: M::ConfigVersion) on each (opened: Boolean) when (opened) {\n\
              capture plain: Reference<M::ConfigVersion> = c;\n\
              capture place: Reference<Clock> = ref(c.x);\n\
              role Observer on M::ConfigVersion;\n\
              run sequence Main { check Ready using v { true }; }\n\
              finish End as (outcome: Boolean) { true };\n\
            }";
        let parsed = fsl_cst::parse(
            fsl_foundation::SourceIdentity::new("test", "generated-origins"),
            "generated-origins.native",
            source.as_bytes(),
            fsl_cst::Limits::default(),
        )
        .expect("parse");
        assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
        let unit = fsl_forms::build_unit(&parsed).expect("forms");
        let admitted = fsl_semantics::model::intake::admit_unit(
            &unit.selections().models,
            &models,
            ModelNormalizationLimits::default(),
        )
        .expect("selection");
        let source = fsl_package::CheckedPackage::link(
            PackageDeclarations::assemble(
                parsed.source().reference().clone(),
                "generated-origins".to_owned(),
                unit,
                admitted,
                Vec::new(),
            )
            .expect("assembly")
            .with_unwritten_timed_members()
            .check(
                CheckMode::Linked,
                CheckingLimits::default(),
                &crate::value::S6aRewardEvaluator,
            )
            .expect("timed check"),
        );
        let state = fsl_semantics::model::state::StateModel::new(
            source.graph().model_inputs(),
            source.graph().scope().types(),
            &models,
            ModelNormalizationLimits::default(),
        )
        .expect("model");
        let owner = state
            .object_reference(
                &filament_checked::key::DeclarationKey {
                    package: "example/config-version".to_owned(),
                    node: "ix://example/config-version/ConfigVersion".to_owned(),
                },
                "c1",
            )
            .expect("actual declared owner identity");
        let keys = source.graph().protocol_node_keys("Owners").expect("keys");
        let binder = keys
            .binders
            .iter()
            .find(|binder| binder.name == "c")
            .expect("owner");
        let supply = ProtocolSupply {
            binders: vec![(
                binder.node.clone(),
                ProtocolSuppliedValue {
                    value: SuppliedValue::Value(Value::Reference(owner.clone())),
                    references: Vec::new(),
                },
            )],
            observation: None,
        };
        let read = fsl_package::read_evaluation_package(
            &source,
            fsl_semantics::library::LibraryName::new("generated-origins").expect("name"),
            &models,
            fsl_package::V2ReadLimits::fixture(),
            fsl_package::EmitLimits::fixture(),
        )
        .expect("read");
        let rows = read
            .emission()
            .identity_rows(
                read.admitted(),
                &mut fsl_package::V2ReadLimits::fixture().native_join_budget(),
            )
            .expect("rows");
        let bare = BarePackage::new(read.admitted())
            .with_supply(
                fsl_package::BareSupply {
                    identities: &rows,
                    equality: None,
                },
                &mut fsl_package::V2ReadLimits::fixture().native_join_budget(),
            )
            .expect("Bare");
        let slot = |index| ProtocolSlot {
            node: keys.clause.clone().expect("root"),
            member: "captures".to_owned(),
            index: Some(index),
        };
        let CallFailure::Admission(failure) = bare
            .evaluate_protocol_slot(&slot(0), &supply, &mut Meter::new(UNLIMITED))
            .expect_err("public supply requires P")
        else {
            panic!("admission");
        };
        let CallAdmissionFailure::Refused(record) = *failure else {
            panic!("refused");
        };
        assert_eq!(
            record.record.catalog,
            filament_semantic_value::catalog::CatalogCode::new(
                "invalid_runtime_input",
                "missing-member"
            )
        );
        let ProtocolSlotEvaluation::Evaluated(evaluation, _) =
            evaluate_generated_package_protocol_slot(
                &bare,
                &slot(0),
                &supply,
                &mut Meter::new(UNLIMITED),
            )
            .expect("generated ordinary value")
        else {
            panic!("available");
        };
        let FamilyOutcome::Evaluated(Outcome::Completed(Value::Reference(actual))) =
            evaluation.outcome
        else {
            panic!("ordinary reference returns unchanged");
        };
        assert_eq!(actual, owner);
        let ProtocolSlotEvaluation::Evaluated(evaluation, _) =
            evaluate_generated_package_protocol_slot(
                &bare,
                &slot(1),
                &supply,
                &mut Meter::new(UNLIMITED),
            )
            .expect("typed Clock boundary")
        else {
            panic!("available");
        };
        let FamilyOutcome::FamilyEvaluated(filament_eval::FamilyResult::Refused(cause)) =
            evaluation.outcome
        else {
            panic!("missing P must refuse Clock consumption");
        };
        assert_eq!(
            cause.catalog_code(),
            filament_semantic_value::catalog::CatalogCode::new(
                "unknown_required_feature",
                "unsupported-feature"
            )
        );
        assert_eq!(
            cause
                .catalog_fields()
                .expect("fields")
                .get("reason")
                .map(String::as_str),
            Some("clock-owner-population-authority")
        );
    }

    fn completed_integer(evaluation: Evaluation) -> Integer {
        match evaluation.outcome {
            FamilyOutcome::Evaluated(Outcome::Completed(Value::Integer(value))) => value,
            other => panic!("expected a completed integer call, got {other:?}"),
        }
    }

    /// FR-531/FR-945: a call through a bytes-only package returns the same
    /// value and work usage as the checked package, including observation.
    #[trace("FR-531-AC-1", "FR-945-AC-1")]
    #[test]
    fn call_matches_the_checked_package_from_admitted_bytes() {
        let source = package();
        let models = fsl_semantics::model::intake::package_input(std::iter::empty::<&[u8]>())
            .expect("the package selects no models");
        let read = fsl_package::read_evaluation_package(
            &source,
            fsl_semantics::library::LibraryName::new("bare-call-test").expect("library identity"),
            &models,
            fsl_package::V2ReadLimits::fixture(),
            fsl_package::EmitLimits::fixture(),
        )
        .expect("the package emits and its bytes are admitted");
        let admitted = read.admitted();
        let mut rows_budget = fsl_package::V2ReadLimits::fixture().native_join_budget();
        let rows = read
            .emission()
            .identity_rows(admitted, &mut rows_budget)
            .expect("identity rows join the package");
        let mut join_budget = fsl_package::V2ReadLimits::fixture().native_join_budget();
        let bare = BarePackage::new(admitted)
            .with_supply(
                fsl_package::BareSupply {
                    identities: &rows,
                    equality: None,
                },
                &mut join_budget,
            )
            .expect("the bytes view admits its supply");
        let function = QualifiedName::unqualified("one").expect("function name");
        let objects = ObjectEnvironment::default();
        let argument = Value::Integer(Integer::from(17_i64));
        let mut source_meter = Meter::new(UNLIMITED);
        let source_result = source
            .call(
                &function,
                vec![argument.clone()],
                &objects,
                &mut source_meter,
            )
            .expect("source call evaluates");
        let mut bare_meter = Meter::new(UNLIMITED);
        let bare_result = bare
            .call(&function, vec![argument], &objects, &mut bare_meter)
            .expect("bytes call evaluates");
        let source_value = completed_integer(source_result);
        let bare_value = completed_integer(bare_result);
        assert_eq!(source_value, bare_value);
        assert_eq!(
            source_meter.consumed(LimitKind::WorkUnits),
            bare_meter.consumed(LimitKind::WorkUnits)
        );

        let mut observed_meter = Meter::new(UNLIMITED);
        let observed = bare.call_observing_applications(
            &function,
            vec![Value::Integer(Integer::from(17_i64))],
            &objects,
            &mut observed_meter,
            ApplicationObservationOptions {
                function_case_limit: 10,
                max_events: 32,
                max_size_requests: 32,
            },
        );
        let observed_value =
            completed_integer(observed.evaluation.expect("observed bytes call evaluates"));
        assert_eq!(observed_value, Integer::from(17_i64));
        assert_eq!(observed.function_evaluations, 1);
        assert_eq!(
            observed_meter.consumed(LimitKind::WorkUnits),
            bare_meter.consumed(LimitKind::WorkUnits)
        );
    }

    /// FR-945-AC-2 / FR-650: nested population types are refused from the
    /// admitted package before argument admission or a function-call charge.
    #[trace("FR-945-AC-2", "FR-650-AC-6")]
    #[test]
    fn nested_population_parameter_and_result_have_typed_details() {
        let (source, models) = population_collection_package();
        let limits = fsl_package::V2ReadLimits::fixture();
        let emit_limits = fsl_package::EmitLimits::fixture();
        let original = fsl_package::read_evaluation_package(
            &source,
            fsl_semantics::library::LibraryName::new("bare-call-types-test")
                .expect("library identity"),
            &models,
            limits,
            emit_limits,
        )
        .expect("source package emits and is admitted");
        let mut wire = fsl_semantics::check::wire_id::resolved_package_value(
            original.emission().package().bytes(),
        )
        .expect("emitted package resolves to its node ids");
        let nodes = wire["semantic_graph"]["nodes"]
            .as_array_mut()
            .expect("semantic graph has nodes");
        let population_type = nodes
            .iter()
            .find(|node| {
                node["node_tag"] == "bounded_domain" && node["semantic_form"] == "model_population"
            })
            .expect("the admitted package carries its population type")["node_id"]
            .clone();
        let sequence = nodes
            .iter_mut()
            .find(|node| {
                node["node_tag"] == "composite_type" && node["semantic_form"] == "sequence"
            })
            .expect("the admitted package carries its sequence type");
        sequence["body"]["members"][0]["target"] = population_type.clone();
        sequence["dependencies"] = serde_json::json!([population_type]);
        wire["semantic_graph"]
            .as_object_mut()
            .expect("semantic graph is an object")
            .remove("operand_rows");
        fsl_semantics::check::wire_id::write_operand_rows(&mut wire);
        let nested_population_bytes = serde_json::to_vec(&wire).expect("mutated package encodes");
        let emission = original
            .emission()
            .clone()
            .with_package_bytes(nested_population_bytes);
        let admitted = fsl_package::read_evaluation_emission(
            &source,
            &emission,
            fsl_semantics::library::LibraryName::new("bare-call-types-test")
                .expect("library identity"),
            &models,
            limits,
            emit_limits,
            fsl_package::SourceWitnessSupplier::new(&emission),
        )
        .expect("the checked package with nested-population type nodes admits");
        let mut rows_budget = limits.native_join_budget();
        let rows = admitted
            .emission()
            .identity_rows(admitted.admitted(), &mut rows_budget)
            .expect("identity rows join the admitted bytes");
        let mut join_budget = limits.native_join_budget();
        let bare = BarePackage::new(admitted.admitted())
            .with_supply(
                fsl_package::BareSupply {
                    identities: &rows,
                    equality: None,
                },
                &mut join_budget,
            )
            .expect("the bare view admits its rows");
        let objects = ObjectEnvironment::default();

        let mut parameter_meter = Meter::new(UNLIMITED);
        let parameter_failure = bare
            .call(
                &QualifiedName::unqualified("takes").expect("function name"),
                vec![Value::Boolean(true)],
                &objects,
                &mut parameter_meter,
            )
            .expect_err("the unsupported parameter is refused before argument admission");
        assert_type_refusal(parameter_failure, "position", "0");
        assert_eq!(parameter_meter.consumed(LimitKind::WorkUnits), 0);

        let mut result_meter = Meter::new(UNLIMITED);
        let result_failure = bare
            .call(
                &QualifiedName::unqualified("gives").expect("function name"),
                vec![Value::Boolean(true)],
                &objects,
                &mut result_meter,
            )
            .expect_err("the unsupported result is refused before argument admission");
        assert_type_refusal(result_failure, "function", "gives");
        assert_eq!(result_meter.consumed(LimitKind::WorkUnits), 0);
    }

    /// FR-945-AC-2 / FR-650: a supported checked result type does not turn an
    /// unsupported operation at the body root into a result-type refusal.
    #[trace("FR-945-AC-2", "FR-650-AC-6")]
    #[test]
    fn unsupported_body_root_operation_keeps_generic_refusal_details() {
        let models = fsl_semantics::model::intake::package_input(std::iter::empty::<&[u8]>())
            .expect("the package selects no models");
        let source_text = "language \"ix:native\" edition \"1-draft\";\n\
             profile v = \"filament.value.complete/v1\";\n\
             function gives using v(): Set<Integer>[0, 3] pure { set[1] }\n";
        let parsed = fsl_cst::parse(
            fsl_foundation::SourceIdentity::new("test", "bare-call-operation"),
            "bare-call-operation.native",
            source_text.as_bytes(),
            fsl_cst::Limits::default(),
        )
        .expect("the source unit parses");
        assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
        let unit = fsl_forms::build_unit(&parsed).expect("the source unit builds");
        let selected_models = fsl_semantics::model::intake::admit_unit(
            &unit.selections().models,
            &models,
            fsl_semantics::model::accounting::ModelNormalizationLimits::default(),
        )
        .expect("the package selects no models");
        let declarations = PackageDeclarations::assemble(
            parsed.source().reference().clone(),
            "bare-call-operation-test".to_owned(),
            unit,
            selected_models,
            Vec::new(),
        )
        .expect("the declarations assemble");
        let source = fsl_package::CheckedPackage::link(
            declarations
                .check(
                    CheckMode::Linked,
                    CheckingLimits::default(),
                    &crate::value::S6aRewardEvaluator,
                )
                .expect("the set-returning function checks"),
        );
        let limits = fsl_package::V2ReadLimits::fixture();
        let emit_limits = fsl_package::EmitLimits::fixture();
        let original = fsl_package::read_evaluation_package(
            &source,
            fsl_semantics::library::LibraryName::new("bare-call-operation-test")
                .expect("library identity"),
            &models,
            limits,
            emit_limits,
        )
        .expect("source package emits and is admitted");
        let mut rows_budget = limits.native_join_budget();
        let rows = original
            .emission()
            .identity_rows(original.admitted(), &mut rows_budget)
            .expect("identity rows join the package");
        let mut join_budget = limits.native_join_budget();
        let bare = BarePackage::new(original.admitted())
            .with_supply(
                fsl_package::BareSupply {
                    identities: &rows,
                    equality: None,
                },
                &mut join_budget,
            )
            .expect("the bare view admits its rows");
        let mut meter = Meter::new(UNLIMITED);
        let failure = bare
            .call(
                &QualifiedName::unqualified("gives").expect("function name"),
                vec![Value::Boolean(true)],
                &ObjectEnvironment::default(),
                &mut meter,
            )
            .expect_err("the unsupported operation is refused before argument admission");
        assert_operation_refusal_without_function_detail(failure);
        assert_eq!(meter.consumed(LimitKind::WorkUnits), 0);
    }

    fn assert_operation_refusal_without_function_detail(failure: CallFailure) {
        let CallFailure::Admission(failure) = failure else {
            panic!("expected typed admission refusal, got {failure:?}");
        };
        let CallAdmissionFailure::Refused(failure) = *failure else {
            panic!("expected typed refusal, got {failure:?}");
        };
        assert_eq!(
            (
                failure.record.catalog.code(),
                failure.record.catalog.cause()
            ),
            ("unknown_required_feature", "unsupported-feature")
        );
        assert!(
            failure
                .record
                .fields
                .get("reason")
                .is_some_and(|reason| { reason.starts_with("missing-operation-projection:") }),
            "expected the operation projection reason, fields: {:?}",
            failure.record.fields
        );
        assert!(
            !failure.record.fields.contains_key("function"),
            "a supported checked result type must not add function detail: {:?}",
            failure.record.fields
        );
    }

    fn assert_type_refusal(failure: CallFailure, detail: &str, value: &str) {
        let CallFailure::Admission(failure) = failure else {
            panic!("expected typed admission refusal, got {failure:?}");
        };
        let CallAdmissionFailure::Refused(failure) = *failure else {
            panic!("expected typed refusal, got {failure:?}");
        };
        assert_eq!(
            (
                failure.record.catalog.code(),
                failure.record.catalog.cause()
            ),
            ("unknown_required_feature", "unsupported-feature")
        );
        assert_eq!(
            failure.record.fields.len(),
            1,
            "fields: {:?}",
            failure.record.fields
        );
        assert_eq!(
            failure.record.fields.get(detail).map(String::as_str),
            Some(value)
        );
    }
}
