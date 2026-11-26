// SPDX-License-Identifier: AGPL-3.0-or-later
//! FSL FR-243: which items the digital-clock route declines because
//! digitisation does not preserve their dense-time verdict.
//!
//! ADR-038 RL-7: which state-graph forms `digital-clocks` admits is read from
//! the declared table (`EN1_DIGITAL`); the claim conditions of FR-243 (b) to
//! (f) are established here. The closed condition (a strict constraint or
//! interval) is checked before it, by [`super::digital`] and the
//! state-graph item.

use filament_exact::{Integer, Rational};
use fsl_eval::simulation::model::{SearchItem, TimedSystem, EN1_DIGITAL};
use fsl_foundation::bound::TemporalUpper;
use fsl_replay::{NotDigitisable, NotDigitisableRule};
use fsl_semantics::check::{
    BinaryTemporal, CheckedInterval, CheckedStateGraphClaim, CheckedTemporalClause, FormulaNode,
    StateGraphForm, UnaryTemporal,
};
use ix_trace_rs::implements;

/// One item the digital-clock route is asked to decide.
pub(crate) enum DigitalItem<'a, 's, 'm> {
    /// A temporal claim over its timed subject.
    Temporal {
        subject: &'a TimedSystem<'s, 'm>,
        clause: &'a CheckedTemporalClause,
    },
    /// A state-graph claim.
    StateGraph(&'a CheckedStateGraphClaim),
}

/// The decline FR-243 gives `item`, or `None` when digitisation preserves
/// its verdict.
///
/// A temporal claim is read in its normal form (of the claim itself, not of
/// its negation): `implies` expanded and every negation pushed to the atoms.
/// It is declined unless every timed interval is closed (a, checked before
/// this); no atom reads a clock (b: S3 admits no such atom, so nothing is
/// tested here); at most one `eventually`, `until`, `once` or `since`
/// carries an interval other than `[0, *]` (c); that interval has lower
/// bound 0 or no upper bound (d); the operator is inside no operand of an
/// `eventually`, `once`, `until` or `since` and no left operand of a
/// `release` or `triggered` (e); and a claim declaring fairness has a subject
/// with no clock constraint (f). The named part is the first, by source
/// start and then longer span, that breaks (c) to (e).
///
/// # Errors
///
/// The part to name keeps no source span, so no decline can be written and
/// the item must not be decided.
#[implements("FR-243-AC-5", "FR-243-AC-6", "FR-243-AC-7")]
pub(crate) fn digitisation_decline(
    item: DigitalItem<'_, '_, '_>,
) -> Result<Option<NotDigitisable>, &'static str> {
    const NO_SPAN: &str = "digital-source-span";
    let (subject, clause) = match item {
        DigitalItem::Temporal { subject, clause } => (subject, clause),
        DigitalItem::StateGraph(claim) => {
            let item = match claim.form() {
                StateGraphForm::Possible { .. } => SearchItem::Possible,
                StateGraphForm::AlwaysPossible { .. } => SearchItem::AlwaysPossible,
                StateGraphForm::UniquePath { .. } => SearchItem::UniquePath,
            };
            // ADR-038's EN-1 rows are the declared table; this module names
            // the FR-243 rule of a form no row admits.
            if EN1_DIGITAL.contains(&item) {
                return Ok(None);
            }
            // Each of these two forms quantifies over every reachable timed
            // state, which an integer-clock reading does not visit.
            let rule = match item {
                SearchItem::AlwaysPossible => NotDigitisableRule::AlwaysPossible,
                SearchItem::UniquePath => NotDigitisableRule::UniquePath,
                SearchItem::DigitisableTemporalClaim
                | SearchItem::Possible
                | SearchItem::DeadlockFreedom => return Err("digital-admission-table"),
            };
            return Ok(Some(NotDigitisable {
                rule,
                span: claim.span(),
            }));
        }
    };
    let formula = clause.formula();
    let nodes = formula.nodes();
    let zero = Rational::from_integer(Integer::zero());
    // The occurrences in the normal form of the operators that count for (c):
    // (node, whether (d) is broken, whether (e) is broken).
    let mut counted = Vec::new();
    // (node, negated, nested): an explicit stack, however deep the formula.
    let mut pending = vec![(formula.root(), false, false)];
    while let Some((id, negated, nested)) = pending.pop() {
        let node = nodes.get(id.index()).ok_or("digital-formula-node")?;
        // Whether the interval is other than `[0, *]`, and whether it is
        // two-sided.
        let bounds = |interval: Option<&CheckedInterval>| match interval {
            None => (false, false),
            Some(CheckedInterval::Timed(interval)) => {
                let late = *interval.lower() != zero;
                (late || interval.is_bounded(), late && interval.is_bounded())
            }
            Some(CheckedInterval::Steps(interval)) => {
                let late = interval.lower() != 0;
                let bounded = matches!(interval.upper(), TemporalUpper::Finite(_));
                (late || bounded, late && bounded)
            }
        };
        match node {
            FormulaNode::Constant(_)
            | FormulaNode::Holds { .. }
            | FormulaNode::Aggregate { .. } => {}
            FormulaNode::Not(operand) => pending.push((*operand, !negated, nested)),
            FormulaNode::And(left, right) | FormulaNode::Or(left, right) => {
                pending.push((*left, negated, nested));
                pending.push((*right, negated, nested));
            }
            FormulaNode::Implies(left, right) => {
                pending.push((*left, !negated, nested));
                pending.push((*right, negated, nested));
            }
            FormulaNode::Unary {
                operator,
                interval,
                operand,
            } => {
                // A negation turns the operator into its dual.
                let eventual = match operator {
                    UnaryTemporal::Eventually | UnaryTemporal::Once => !negated,
                    UnaryTemporal::Always | UnaryTemporal::Historically => negated,
                };
                let (bounded, two_sided) = bounds(interval.as_ref());
                if eventual && bounded {
                    counted.push((id, two_sided, nested));
                }
                pending.push((*operand, negated, nested || eventual));
            }
            FormulaNode::Binary {
                operator,
                interval,
                left,
                right,
            } => {
                let until = match operator {
                    BinaryTemporal::Until | BinaryTemporal::Since => !negated,
                    BinaryTemporal::Release | BinaryTemporal::Triggered => negated,
                };
                let (bounded, two_sided) = bounds(interval.as_ref());
                if until && bounded {
                    counted.push((id, two_sided, nested));
                }
                // The left operand of all four is read at many positions or
                // at positions the claim does not fix; the right one only
                // under `until` and `since`.
                pending.push((*left, negated, true));
                pending.push((*right, negated, nested || until));
            }
        }
    }
    let mut spanned = Vec::with_capacity(counted.len());
    for (id, two_sided, nested) in counted {
        let span = clause.formula_span(id).ok_or(NO_SPAN)?;
        spanned.push((span, two_sided, nested));
    }
    spanned.sort_by_key(|(span, ..)| (span.start, std::cmp::Reverse(span.end)));
    // Occurrences with one span are read together.
    let mut before = 0_usize;
    for group in spanned.chunk_by(|left, right| left.0 == right.0) {
        let rule = if before + group.len() > 1 {
            Some(NotDigitisableRule::SecondBoundedOperator)
        } else if group.iter().any(|(_, two_sided, _)| *two_sided) {
            Some(NotDigitisableRule::TwoSidedInterval)
        } else if group.iter().any(|(.., nested)| *nested) {
            Some(NotDigitisableRule::NestedBoundedOperator)
        } else {
            None
        };
        if let (Some(rule), Some((span, ..))) = (rule, group.first()) {
            return Ok(Some(NotDigitisable { rule, span: *span }));
        }
        before += group.len();
    }
    // (f): a disabled constrained operation can be a strict clock region the
    // integer grid misses.
    let fair = !clause.fairness().is_empty() || clause.protocol_fairness().next().is_some();
    if fair && !crate::exact_probabilistic::model_constraints(subject).is_empty() {
        return Ok(Some(NotDigitisable {
            rule: NotDigitisableRule::FairnessOverClockConstraint,
            span: clause.first_fairness_span().ok_or(NO_SPAN)?,
        }));
    }
    Ok(None)
}
