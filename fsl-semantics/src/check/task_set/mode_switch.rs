// SPDX-License-Identifier: AGPL-3.0-or-later
//! FSL FR-249 (FSpec FR-419 "Criticality mode"): S3's check of the
//! `mode S hi on M::o;` declarations of a unit.
//!
//! A declaration names a task set of the unit and an operation of a model,
//! each resolved as a `release` resolves it: a name that does not resolve is
//! refused `missing_declaration`/`missing-name` (an operation of several
//! declaring types is `ambiguous_declaration`/`ambiguous-name`), every one
//! reported. A second `mode S hi` on one operation is refused
//! `conflicting_declaration`/`duplicate-declaration` at the second
//! declaration. A checked declaration is recorded, in source order, as a
//! [`ModeSwitch`] on the unit's [`CheckedTaskSets`] beside its releases. A
//! mode switch does not make a claim release-backed.

use fsl_foundation::Span;

use super::{CheckedTaskSets, TaskSetDeclaration};
use crate::check::probabilistic::{ProbabilisticInput, ProbabilisticOperation, Unresolved};
use crate::check::refusal::TemporalCause;

/// A checked `mode S hi on M::o;`: a step of the operation taken while the
/// task product of `S` is in `LO` mode switches it to `HI` mode (FSL FR-249).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModeSwitch {
    task_set: String,
    operation: ProbabilisticOperation,
    span: Span,
}

impl ModeSwitch {
    /// The task set whose product the operation switches to `HI` mode.
    pub fn task_set(&self) -> &str {
        &self.task_set
    }

    /// The operation whose step switches the mode.
    pub fn operation(&self) -> &ProbabilisticOperation {
        &self.operation
    }

    /// The span of the declaration, `;` included.
    pub fn span(&self) -> Span {
        self.span
    }
}

/// Check the unit's `mode` switch declarations against its task sets and the
/// operations `input` resolves, recording each valid one in `model`.
pub(in crate::check) fn check_mode_switches(
    declarations: &[TaskSetDeclaration],
    input: &ProbabilisticInput<'_>,
    model: &mut CheckedTaskSets,
) -> Vec<TemporalCause> {
    let mut causes = Vec::new();
    for (position, declaration) in declarations.iter().enumerate() {
        let TaskSetDeclaration::ModeSwitch(form) = declaration else {
            continue;
        };
        let known = declarations.iter().any(|candidate| {
            matches!(candidate, TaskSetDeclaration::Taskset(set) if set.name.name == form.task_set.name)
        });
        if !known {
            causes.push(TemporalCause::UnknownModeSwitchTaskSet {
                name: form.task_set.name.clone(),
                operation: form.operation.name.clone(),
                span: form.task_set.span,
            });
        }
        let operation =
            match input.operation(&form.operation, None, input.seen_by_task_set(position)) {
                Ok((operation, _)) => operation,
                Err(fault) => {
                    causes.push(match fault {
                        Unresolved::Missing => TemporalCause::UnresolvedModeSwitchOperation {
                            name: form.operation.name.clone(),
                            span: form.operation.span,
                        },
                        Unresolved::Ambiguous => TemporalCause::AmbiguousModeSwitchOperation {
                            name: form.operation.name.clone(),
                            span: form.operation.span,
                        },
                    });
                    continue;
                }
            };
        if !known {
            continue;
        }
        let first = model.mode_switches.iter().find(|earlier| {
            earlier.operation.key() == operation.key() && earlier.task_set == form.task_set.name
        });
        if let Some(first) = first {
            causes.push(TemporalCause::DuplicateModeSwitch {
                task_set: form.task_set.name.clone(),
                operation: form.operation.name.clone(),
                first: first.span,
                span: form.span,
            });
            continue;
        }
        model.mode_switches.push(ModeSwitch {
            task_set: form.task_set.name.clone(),
            operation,
            span: form.span,
        });
    }
    causes
}
