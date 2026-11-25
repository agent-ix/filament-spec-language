// SPDX-License-Identifier: AGPL-3.0-or-later
//! ADR-021 SC-1 and SC-2 (FR-158): a state constraint's truth at a model
//! state.
//!
//! The constraint is a checked `at current` state clause of the package,
//! named by the request. It holds at a state when it holds for every object
//! whose most-specific type conforms to its context type, each evaluated
//! through FR-107 in ascending reference-key order under a fresh meter, as
//! an invariant is (FR-120 "Invariants").

use super::contracts::{Verdict, invariant_observations, verdict};
use super::walk::{exhausted, stop, walk_of};
use super::{BROKEN, ClauseOutcome, ModelSimState, ModelSystem};
use crate::simulation::expansion::{ExpansionLimit, ExpansionStop};
use crate::simulation::reduction::ConstraintValue;
use filament_semantic_value::location::ExpressionLocation;
use ix_trace_rs::implements;

impl ModelSystem<'_> {
    /// Where the body of the state clause `name` is, on either route: the
    /// locus of an undefined guard or state constraint (FR-170
    /// "Undefined"). `None` for a name with no clause.
    #[implements("FR-168-AC-11", "FR-168-AC-13")]
    pub fn state_clause_locus(&self, name: &str) -> Option<ExpressionLocation> {
        self.evaluator().state_clause_location(name)
    }

    /// The checked state constraint named `name`, identified by its node
    /// key (FR-158). Invariants, preconditions and postconditions are not
    /// valid constraint selections.
    pub fn constraint_node(&self, name: &str) -> Option<filament_exact::NodeKey> {
        self.input
            .state_clauses
            .iter()
            .find(|clause| clause.name == name && !clause.invariant && clause.operation.is_none())
            .map(|clause| clause.identity.clone())
    }

    /// Whether the package declares a checked `at current` state clause
    /// named `name`, which a request may name as its state constraint.
    pub fn has_constraint(&self, name: &str) -> bool {
        self.input
            .state_clauses
            .iter()
            .any(|clause| clause.name == name && !clause.invariant && clause.operation.is_none())
    }

    /// FR-158 "Truth at a state": `Holds` when every evaluation is true;
    /// `Undefined` for the first object in key order whose evaluation is
    /// undefined; otherwise `Boundary` when some evaluation is false.
    ///
    /// # Errors
    ///
    /// `resource_exhausted`/`insufficient-next-charge` naming the evaluation
    /// meter when an evaluation runs out of it; a refused evaluation stops
    /// with its refusal's code, or `runtime_invariant`/
    /// `established-invariant-broken` for a kernel refusal, a non-Boolean
    /// value, an unknown clause or a call failure.
    pub fn constraint_value(
        &self,
        state: &ModelSimState,
        name: &str,
    ) -> Result<ConstraintValue, ExpansionStop> {
        let clause = self
            .clauses
            .invariants
            .iter()
            .find(|clause| clause.name == name)
            .ok_or_else(|| stop(BROKEN))?;
        let walk = walk_of(self, state, None)?;
        let mut order: Vec<usize> = (0..walk.rows.len()).collect();
        order.sort_by(|left, right| walk.rows[*left].reference.cmp(&walk.rows[*right].reference));
        let mut boundary = false;
        for row in order {
            let reference = &walk.rows[row].reference;
            if !self
                .types
                .conforms(reference.object_type().clone(), clause.context.clone())
            {
                continue;
            }
            let observations = invariant_observations(&walk, clause, row);
            let evaluation = self
                .evaluator()
                .evaluate_clause(
                    &clause.qualified,
                    &observations,
                    &mut self.evaluation_meter(),
                )
                .map_err(|_| stop(BROKEN))?;
            match verdict(&evaluation.evaluation.outcome) {
                Verdict::True => {}
                Verdict::False => boundary = true,
                Verdict::Undecided(outcome) => {
                    if let Some(cause) = outcome.undefined_cause() {
                        return Ok(ConstraintValue::Undefined {
                            object: reference.clone(),
                            cause,
                        });
                    }
                    return Err(match outcome {
                        ClauseOutcome::Incomplete(incomplete) => {
                            exhausted(ExpansionLimit::EvaluationMeter(incomplete.limit_kind))
                        }
                        ClauseOutcome::FamilyRefused(code) | ClauseOutcome::Stopped(code) => {
                            stop(code)
                        }
                        ClauseOutcome::KernelRefused(_)
                        | ClauseOutcome::NotBoolean
                        | ClauseOutcome::KernelUndefined(_)
                        | ClauseOutcome::FamilyUndefined(_) => stop(BROKEN),
                    });
                }
            }
        }
        Ok(if boundary {
            ConstraintValue::Boundary
        } else {
            ConstraintValue::Holds
        })
    }
}
