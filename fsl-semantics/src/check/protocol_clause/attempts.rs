//! FR-114 attempt binding: each `attempt`'s `contracts` list resolved
//! against the unit's state clauses and its already-resolved operation.

use std::collections::BTreeMap;

use filament_semantic_value::declaration::{ObjectTypeDeclaration, TypeEnvironment};
use fsl_forms::{BinderKind, ProtocolDeclarationForm};
use fsl_foundation::Span;

use super::{refusal, BoundAttempt};
use crate::check::refusal::{CheckRefusal, ProtocolAnchorCause};
use crate::check::state_clause::{AttemptDeclaration, StateClauseDeclaration};

/// FR-114 "Behavior": each `attempt`'s already-resolved operation
/// (`attempts`, the assembler's own `AttemptDeclaration`s, index-aligned
/// with `protocol.attempts`) bound against its `contracts` list: an entry
/// naming no state clause of the unit refuses `missing_declaration`/
/// `missing-name` at the entry; one naming an invariant or a `pre`/`post`
/// clause anchored at a different operation refuses `wrong_snapshot`/
/// `wrong-anchor` at the entry, naming both anchors. An entry naming two
/// or more state clauses of one name refuses `ambiguous_declaration`/
/// `ambiguous-name` at the entry (SR-770 FND-005): no source order or first
/// match picks one, the same rule FR-113 applies to a protocol node.
pub(super) fn bind_attempts(
    protocol: &ProtocolDeclarationForm,
    attempts: &[AttemptDeclaration],
    state_clauses: &[StateClauseDeclaration],
    types: &TypeEnvironment,
    type_names: &impl crate::check::type_form::TypeNames,
) -> (Vec<BoundAttempt>, Vec<(Span, CheckRefusal)>) {
    let mut by_name: BTreeMap<&str, Vec<usize>> = BTreeMap::new();
    for (index, clause) in state_clauses.iter().enumerate() {
        by_name.entry(clause.name.as_str()).or_default().push(index);
    }
    let anchor_label = |declaring, operation: &str| -> String {
        let context = types
            .object_type(declaring)
            .map(ObjectTypeDeclaration::name)
            .unwrap_or("?");
        format!("{context}::{operation}")
    };

    let mut refusals = Vec::new();
    let mut bound = Vec::with_capacity(attempts.len());
    for (index, attempt_decl) in attempts.iter().enumerate() {
        let form = &protocol.attempts[index];
        if let Some(binder) = super::record_binder(protocol, attempt_decl.declaration) {
            if binder.kind == BinderKind::RecordBinder {
                if let Ok(actual_result) =
                    crate::check::type_form::resolve_form(type_names, &binder.value_type)
                {
                    let expected_result = attempt_decl.operation.declaration.result().cloned();
                    if expected_result.as_ref() != Some(&actual_result) {
                        refusals.push((
                            binder.name.span,
                            refusal(ProtocolAnchorCause::AttemptBinderType {
                                binder: binder.name.name.clone(),
                                expected_result: expected_result.map(Box::new),
                                actual_result: Box::new(actual_result),
                                span: binder.name.span,
                            }),
                        ));
                    }
                }
            }
        }
        let attempt_anchor = anchor_label(
            attempt_decl.operation.declaring.clone(),
            attempt_decl.operation.declaration.name(),
        );
        let mut contracts = Vec::with_capacity(form.contracts.len());
        for entry in &form.contracts {
            let Some(indices) = by_name.get(entry.name.as_str()) else {
                refusals.push((
                    entry.span,
                    refusal(ProtocolAnchorCause::MissingContract {
                        entry: entry.name.clone(),
                        span: entry.span,
                    }),
                ));
                continue;
            };
            // SR-770 FND-005: two state clauses sharing the entry's name
            // leave no single clause for it to bind -- refused at the
            // entry, naming every clause of that name, the same
            // no-first-match rule FR-113 applies to a protocol node name.
            let [clause_index] = indices[..] else {
                refusals.push((
                    entry.span,
                    refusal(ProtocolAnchorCause::Ambiguous {
                        name: entry.name.clone(),
                        loci: indices
                            .iter()
                            .filter_map(|&index| state_clauses.get(index))
                            .map(|clause| clause.spans.declaration)
                            .collect(),
                        span: entry.span,
                    }),
                ));
                continue;
            };
            let Some(clause) = state_clauses.get(clause_index) else {
                continue;
            };
            let matches = clause
                .operation
                .as_ref()
                .is_some_and(|clause_operation| *clause_operation == attempt_decl.operation);
            if matches {
                contracts.push(clause_index);
                continue;
            }
            let clause_anchor = match &clause.operation {
                Some(clause_operation) => anchor_label(
                    clause_operation.declaring.clone(),
                    clause_operation.declaration.name(),
                ),
                None => types
                    .object_type(clause.context.clone())
                    .map(ObjectTypeDeclaration::name)
                    .unwrap_or("?")
                    .to_owned(),
            };
            refusals.push((
                entry.span,
                refusal(ProtocolAnchorCause::WrongContractAnchor {
                    entry: entry.name.clone(),
                    attempt_anchor: attempt_anchor.clone(),
                    clause_anchor,
                    span: entry.span,
                }),
            ));
        }
        bound.push(BoundAttempt {
            declaration: attempt_decl.declaration,
            operation: attempt_decl.operation.clone(),
            receiver: attempt_decl.receiver.clone(),
            frame_population: attempt_decl.frame_population.clone(),
            contracts,
        });
    }
    (bound, refusals)
}
