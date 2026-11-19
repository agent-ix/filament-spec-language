// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-1019 "Class": the first construct of a checked hybrid model outside the
//! class a flowpipe provider decides. Every flow right-hand side, after
//! expansion of non-recursive calls, is built from constants, variables,
//! unary `-`, `+`, `-` and `*`; guards, resets, invariants and initial
//! conditions are Boolean combinations of comparisons of such terms.

use fsl_semantics::check::{
    ArithmeticOp, CheckedGraph, CheckedHybridModel, HybridTerm, ModeIndex, VariableIndex,
};

use super::expand::{Exp, Expander, TermLocus};
use super::{ExpressionSite, HybridReplayMeter, HybridReplayRefusal};
use crate::hybrid::UnsupportedConstruct;

/// Whether a term is read as a number or as a Boolean.
#[derive(Clone, Copy)]
enum Position {
    Numeric,
    Boolean,
}

/// Where a construct outside the class is: the term path within the
/// expression, and its form.
struct Gap {
    form: &'static str,
    locus: TermLocus,
}

struct Walk<'a, 'g> {
    expander: &'a mut Expander<'g>,
    meter: &'a mut HybridReplayMeter,
}

impl Walk<'_, '_> {
    fn term(
        &mut self,
        term: &HybridTerm,
        path: &[usize],
        position: Position,
    ) -> Result<Option<Gap>, HybridReplayRefusal> {
        let here = |form| {
            Some(Gap {
                form,
                locus: TermLocus {
                    term: path.to_vec(),
                    body: Vec::new(),
                },
            })
        };
        let child = |position: usize| {
            let mut next = path.to_vec();
            next.push(position);
            next
        };
        Ok(match (term, position) {
            (HybridTerm::Constant(_) | HybridTerm::Variable(_), Position::Numeric)
            | (HybridTerm::Boolean(_), Position::Boolean) => None,
            (HybridTerm::Negate(a), Position::Numeric) => {
                self.term(a, &child(0), Position::Numeric)?
            }
            (HybridTerm::Arithmetic(ArithmeticOp::Div, _, _), Position::Numeric) => {
                here("division")
            }
            (HybridTerm::Arithmetic(_, a, b), Position::Numeric) => {
                match self.term(a, &child(0), Position::Numeric)? {
                    Some(gap) => Some(gap),
                    None => self.term(b, &child(1), Position::Numeric)?,
                }
            }
            (HybridTerm::Not(a), Position::Boolean) => {
                self.term(a, &child(0), Position::Boolean)?
            }
            (HybridTerm::Logic(_, a, b), Position::Boolean) => {
                match self.term(a, &child(0), Position::Boolean)? {
                    Some(gap) => Some(gap),
                    None => self.term(b, &child(1), Position::Boolean)?,
                }
            }
            (HybridTerm::Compare(_, a, b), Position::Boolean) => {
                match self.term(a, &child(0), Position::Numeric)? {
                    Some(gap) => Some(gap),
                    None => self.term(b, &child(1), Position::Numeric)?,
                }
            }
            (HybridTerm::Call(..), _) => {
                let expanded = self.expander.term(term, self.meter)?;
                // The expansion's loci are relative to the call, so the
                // call's own path leads them.
                polynomial_gap(&expanded, position).map(|(form, inner)| {
                    let mut locus = inner.unwrap_or_default();
                    locus.term.splice(0..0, path.iter().copied());
                    Gap { form, locus }
                })
            }
            // A Boolean-valued `if` is a Boolean combination; a numeric one
            // is piecewise, so not a polynomial.
            (HybridTerm::Conditional(c, a, b), Position::Boolean) => {
                let mut found = self.term(c, &child(0), Position::Boolean)?;
                if found.is_none() {
                    found = self.term(a, &child(1), Position::Boolean)?;
                }
                if found.is_none() {
                    found = self.term(b, &child(2), Position::Boolean)?;
                }
                found
            }
            (HybridTerm::Conditional(..), Position::Numeric) => here("conditional"),
            (HybridTerm::Capture(_) | HybridTerm::ActivationVariable(_), _) => here("activation"),
            _ => here("mixed-kinds"),
        })
    }
}

/// The first node of an expanded expression outside the class, with its
/// locus when the expansion kept one. A call's argument and result ranges
/// (`Exp::Ranged`) are definedness conditions on a value that is the wrapped
/// term's, as in exact replay's polynomial flow (FR-450 "Exact replay"), so
/// they do not leave the class.
fn polynomial_gap(exp: &Exp, position: Position) -> Option<(&'static str, Option<TermLocus>)> {
    match (exp, position) {
        (Exp::Number(_) | Exp::Variable(_) | Exp::Boolean(_), _) => None,
        (Exp::Negate(a) | Exp::Not(a), _) => polynomial_gap(a, position),
        (Exp::Ranged { value, .. }, _) => polynomial_gap(value, position),
        (Exp::Arithmetic(ArithmeticOp::Div, _, _, locus), _) => {
            Some(("division", Some(locus.clone())))
        }
        (Exp::Arithmetic(_, a, b, _), _) => {
            polynomial_gap(a, Position::Numeric).or_else(|| polynomial_gap(b, Position::Numeric))
        }
        (Exp::Logic(_, a, b), _) => {
            polynomial_gap(a, Position::Boolean).or_else(|| polynomial_gap(b, Position::Boolean))
        }
        (Exp::Compare(_, a, b), _) => {
            polynomial_gap(a, Position::Numeric).or_else(|| polynomial_gap(b, Position::Numeric))
        }
        (Exp::Conditional(c, a, b, _), Position::Boolean) => polynomial_gap(c, Position::Boolean)
            .or_else(|| polynomial_gap(a, Position::Boolean))
            .or_else(|| polynomial_gap(b, Position::Boolean)),
        (Exp::Conditional(.., locus), Position::Numeric) => {
            Some(("conditional", Some(locus.clone())))
        }
        (Exp::Recursive(call), _) => Some(("recursive-call", Some(call.locus.clone()))),
        (Exp::Activation(_) | Exp::Capture(_) | Exp::Parameter(_), _) => Some(("activation", None)),
    }
}

/// The first construct of `model` outside the class of FR-1019 "Class", in
/// source order: the initial entries, then each mode in declared order (its
/// invariant, its flow equations, then each transition's guard and reset
/// values), each expression by a pre-order walk. A checked model stores a
/// mode's rates by variable, so its equations are taken in variable order,
/// which is the written order whenever the modes write them in the order the
/// variables first appear. `Ok(None)` when every construct is inside it.
///
/// # Errors
///
/// [`HybridReplayRefusal`] when expanding a call of the model exhausts
/// `meter` or the package holds no such function.
pub fn polynomial_class_gap(
    graph: &CheckedGraph,
    model: &CheckedHybridModel,
    meter: &mut HybridReplayMeter,
) -> Result<Option<UnsupportedConstruct>, HybridReplayRefusal> {
    let mut expander = Expander::new(graph);
    let mut walk = Walk {
        expander: &mut expander,
        meter,
    };
    let found = |gap: Option<Gap>, site: ExpressionSite, mode: usize, variable: Option<usize>| {
        gap.map(|gap| UnsupportedConstruct {
            form: gap.form.to_owned(),
            site,
            term: gap.locus,
            mode: ModeIndex::new(mode),
            variable: variable.map(VariableIndex::new),
        })
    };
    for (entry, initial) in model.initial.iter().enumerate() {
        let gap = walk.term(&initial.condition, &[], Position::Boolean)?;
        if let Some(found) = found(
            gap,
            ExpressionSite::Initial { entry },
            initial.mode.get(),
            None,
        ) {
            return Ok(Some(found));
        }
    }
    for (mode_index, mode) in model.modes.iter().enumerate() {
        let gap = walk.term(&mode.invariant, &[], Position::Boolean)?;
        if let Some(found) = found(
            gap,
            ExpressionSite::Invariant { mode: mode_index },
            mode_index,
            None,
        ) {
            return Ok(Some(found));
        }
        for (variable, rate) in mode.rates.iter().enumerate() {
            let gap = walk.term(rate, &[], Position::Numeric)?;
            if let Some(found) = found(
                gap,
                ExpressionSite::Rate {
                    mode: mode_index,
                    variable,
                },
                mode_index,
                Some(variable),
            ) {
                return Ok(Some(found));
            }
        }
        for (transition, jump) in mode.transitions.iter().enumerate() {
            let gap = walk.term(&jump.guard, &[], Position::Boolean)?;
            if let Some(found) = found(
                gap,
                ExpressionSite::Guard {
                    mode: mode_index,
                    transition,
                },
                mode_index,
                None,
            ) {
                return Ok(Some(found));
            }
            for (constraint, reset) in jump.resets.iter().enumerate() {
                let gap = walk.term(&reset.value, &[], Position::Numeric)?;
                if let Some(found) = found(
                    gap,
                    ExpressionSite::Reset {
                        mode: mode_index,
                        transition,
                        constraint,
                    },
                    mode_index,
                    Some(reset.variable.get()),
                ) {
                    return Ok(Some(found));
                }
            }
        }
    }
    Ok(None)
}
