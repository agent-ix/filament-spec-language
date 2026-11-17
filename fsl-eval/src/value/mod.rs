// SPDX-License-Identifier: AGPL-3.0-or-later
//! The S6a value evaluator (ADR-011 §6.1 layer 5).
//!
//! The layer-3 value semantics this evaluator runs over -- the §6.2
//! `semantic_value` submodules and `value::model_query` -- live in the `fsl-semantics` crate's own
//! `value` module (X-6b). This module re-exports none of them
//! (ADR-011 §7.2): callers name them at `fsl_semantics::value::...`. It holds
//! `value::expression`, which `fsl-eval` takes at X-8, and re-exports only
//! its own items. The layer-4 `CheckedPackage` it evaluates is
//! `fsl_package::CheckedPackage` (X-7), re-exported by none of this
//! crate.

pub mod collections;
pub mod composite;
mod expression;
pub(crate) use expression::evaluate_clause_set_with_cancel;

/// Map entries in FSpec FR-144 canonical key order. Float keys use their
/// membership identity's IEEE total order; integer and decimal keys use their
/// mathematical ordering through `compare_keys`.
pub(crate) fn ordered_map_entries(
    map: &filament_exact::MapValue,
) -> Option<Vec<&(filament_exact::Value, filament_exact::Value)>> {
    use std::cmp::Ordering;

    let mut entries = Vec::new();
    entries.try_reserve(map.entries().len()).ok()?;
    entries.extend(map.entries().iter());
    let mut incomparable = false;
    entries.sort_unstable_by(|(left, _), (right, _)| {
        let order = match (left, right) {
            (filament_exact::Value::Float(left), filament_exact::Value::Float(right)) => {
                Some(left.total_order_key().cmp(&right.total_order_key()))
            }
            _ => filament_exact::compare_keys(left, right),
        };
        order.unwrap_or_else(|| {
            incomparable = true;
            Ordering::Equal
        })
    });
    (!incomparable).then_some(entries)
}

// `CheckedPackageEvaluation` is the trait that carries `call` and `evaluate`
// over the foreign-to-`value` `CheckedPackage` typestate, since an inherent
// impl here is E0116: `CheckedPackage` is `fsl-package`'s own type (X-7).
pub use expression::bare_evaluate_selected_protocol_slot;
pub use expression::collection_write::{Provenance, WrittenCollection};
pub(crate) use expression::evaluate_over_admitted_environment;
pub use expression::object_environment_refusal;
pub use expression::{
    admit_arguments, evaluate_activation_guard, evaluate_activation_guard_at_absent_key,
    evaluate_capture, evaluate_closed_with_frame, evaluate_history_update, evaluate_join_predicate,
    evaluate_operation_contract, evaluate_protocol_slot, evaluate_refinement_row, evaluate_reward,
    evaluate_state_predicate, evaluate_state_predicate_at_absent_key,
    evaluate_state_predicate_with, evaluate_terminal_predicate, input_refusal_code, refusal_record,
    supplied_stop_cause, supplied_stop_code, undefined_contract_code,
    AdmittedEvaluationViewEvaluation, ApplicationObservation, ApplicationObservationOptions,
    BinderCharges, ChargeAmounts, CheckedPackageEvaluation, ClauseEvaluation, ClauseRead,
    ClauseSetEntry, ClauseSetEvaluation, ClauseSetOutcome, FrameEvaluation, InvalidQualifiedName,
    InvocationBinding, ObservationIdentity, OperationCharge, OperationLedger, PackageClosure,
    PackageTask, ProtocolReferenceOccurrence, ProtocolSlot, ProtocolSlotEvaluation,
    ProtocolSuppliedValue, ProtocolSupply, QualifiedName, RuntimeValuePath, Separation,
    SeparationStep, StopReport, SuppliedValue, TruncatedOperationLedger, ValueInvariantEvaluation,
    ValueInvariantOutcome, ValueInvariantResult, ValuePathStep, ValuePathSubject, WitnessClaim,
    BINDER_ACCOUNTING_VERSION,
};
