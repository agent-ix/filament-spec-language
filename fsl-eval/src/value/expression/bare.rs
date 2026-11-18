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
        unavailable,
        binder_slots,
    } = admit_slot_supply(&host, &sources, slot.frame_len(), supply)?;
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
        },
        witness,
    })
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
            let world = self.world().map_err(|failure| refusal(self, failure))?;
            let cancel = meter.cancel().cloned().unwrap_or_default();
            let mut run = SuppliedRun::new(SuppliedLimits::default(), cancel)
                .map_err(|stop| CallFailure::from(SuppliedFailure::Stop(stop)))?;
            super::admit_arguments(&parameters, &arguments, objects, world.types(), &mut run)?;
            if let Err(incomplete) = meter.charge(Charge::new(ChargePoint::FunctionCall)) {
                return Ok(Evaluation {
                    outcome: FamilyOutcome::Evaluated(Outcome::Incomplete(incomplete)),
                    location: None,
                    losses: Vec::new(),
                });
            }
            capture.drain(meter, projection.location(selected.body));
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
                    location: projection.location(selected.body).cloned(),
                    losses: Vec::new(),
                });
            }
            function_evaluations = 1;
            Ok(Machine::observing_bare_package(
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
            .map_err(CallFailure::Fault)?)
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
        let world = self.world().map_err(|failure| refusal(self, failure))?;
        super::admit_arguments(&parameters, &arguments, objects, world.types(), run)?;
        if let Err(incomplete) = meter.charge(Charge::new(ChargePoint::FunctionCall)) {
            return Ok(Evaluation {
                outcome: FamilyOutcome::Evaluated(Outcome::Incomplete(incomplete)),
                location: None,
                losses: Vec::new(),
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

    fn evaluate_hyper_holds(
        &self,
        _clause: &fsl_semantics::check::CheckedHyperClause,
        _atom: fsl_semantics::check::FormulaNodeId,
        _slots: Vec<crate::hyper::HyperSlot>,
        _components: &[&ObjectEnvironment],
        _meter: &mut Meter,
    ) -> Result<Evaluation, CallFailure> {
        Err(unmet(self, SOURCE_CHECKED_FORM))
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
        _clause: &QualifiedName,
        _observations: &fsl_semantics::model::observation::AdmittedObservations,
        _meter: &mut Meter,
    ) -> Result<ClauseEvaluation, CallFailure> {
        Err(unmet(self, SOURCE_DECLARATION))
    }

    fn evaluate_clause_set(
        &self,
        _clauses: &[(
            QualifiedName,
            &fsl_semantics::model::observation::AdmittedObservations,
        )],
        _limits: filament_exact::ScalarLimits,
    ) -> Result<ClauseSetEvaluation, CallFailure> {
        Err(unmet(self, SOURCE_DECLARATION))
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
                .check(CheckMode::Linked, CheckingLimits::default())
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
        let source_text =
            "language \"ix:native\" edition \"1-draft\";\n\
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
                .check(CheckMode::Linked, CheckingLimits::default())
                .expect("the source-supported package checks"),
        );
        (source, models)
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
                .check(CheckMode::Linked, CheckingLimits::default())
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
