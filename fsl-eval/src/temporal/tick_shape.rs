// SPDX-License-Identifier: AGPL-3.0-or-later
//! The timing shape of a tick-monitor formula (FSL FR-251 "Buffers"): for
//! every temporal operator its direction, the *lookahead* after which its
//! value at a position can no longer change, and, for every interval
//! operator, the *retention* in ticks an entry is kept for.
//!
//! The retention of an interval operator is the span for which a consumer can
//! still read the entry: the operator's own window if it looks backwards,
//! plus the time until every enclosing operator has settled the instances
//! that read it. For a chain of nested interval operators this is the sum of
//! the upper bounds along the path plus one tick cover (`1 + 2u` ticks) per
//! level (FR-251-AC-18, AC-21); the shape also counts the lookahead of the
//! operator's own operands, so an entry survives until its readers settle.

use std::collections::BTreeMap;

use filament_exact::{Integer, Rational};
use fsl_semantics::check::{BinaryTemporal, CheckedFormula, FormulaNode, UnaryTemporal};

use super::tick_monitor::TickBound;

fn int(value: i64) -> Rational {
    Rational::from_integer(Integer::from(value))
}

fn larger(a: Rational, b: Rational) -> Rational {
    if a >= b {
        a
    } else {
        b
    }
}

/// The timing facts of one temporal operator.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NodeShape {
    /// Whether the operator reads later events.
    pub future: bool,
    /// Whether the operator is the dual (`always`, `historically`,
    /// `release`, `triggered`).
    pub dual: bool,
    /// The left operand of a binary operator.
    pub left: Option<usize>,
    /// The (right) operand.
    pub right: usize,
    /// The tick interval, or `None` on an unbounded operator.
    pub bound: Option<TickBound>,
    /// Ticks after an instance's tick at which its value is settled, or
    /// `None` when it never settles by time (an unbounded operator below).
    pub lookahead: Option<Rational>,
    /// For a buffered operator, the ticks an entry is retained for. For an
    /// unbounded past operator this is also the age at which an entry is
    /// folded into the operator's summary cell (FR-251 "Folding").
    pub retention: Rational,
    /// Whether the operator keeps a buffer: an interval operator with an
    /// upper bound, or any past operator (FR-251 "Unbounded past operators").
    pub buffered: bool,
    /// The settling delay `S(n)` of the operator's operands: the largest sum
    /// of the upper bounds of the future operators on a path through them.
    pub settling: Rational,
}

/// The timing shape of a whole formula.
#[derive(Clone, Debug)]
pub struct TickShape {
    nodes: BTreeMap<usize, NodeShape>,
    width: Rational,
}

/// The temporal operators reached from `index` through Boolean nodes only.
pub fn temporal_leaves(formula: &CheckedFormula, index: usize) -> Vec<usize> {
    let nodes = formula.nodes();
    let mut out = Vec::new();
    let mut stack = vec![index];
    while let Some(next) = stack.pop() {
        match &nodes[next] {
            FormulaNode::Unary { .. } | FormulaNode::Binary { .. } => out.push(next),
            other => stack.extend(other.operands().map(|operand| operand.index())),
        }
    }
    out.sort_unstable();
    out.dedup();
    out
}

/// The direction and duality of a temporal operator node, with its operands.
pub fn operator_parts(node: &FormulaNode) -> Option<(bool, bool, Option<usize>, usize)> {
    match node {
        FormulaNode::Unary {
            operator, operand, ..
        } => {
            let (future, dual) = match operator {
                UnaryTemporal::Eventually => (true, false),
                UnaryTemporal::Always => (true, true),
                UnaryTemporal::Once => (false, false),
                UnaryTemporal::Historically => (false, true),
            };
            Some((future, dual, None, operand.index()))
        }
        FormulaNode::Binary {
            operator,
            left,
            right,
            ..
        } => {
            let (future, dual) = match operator {
                BinaryTemporal::Until => (true, false),
                BinaryTemporal::Release => (true, true),
                BinaryTemporal::Since => (false, false),
                BinaryTemporal::Triggered => (false, true),
            };
            Some((future, dual, Some(left.index()), right.index()))
        }
        _ => None,
    }
}

impl TickShape {
    /// The shape of `formula` under the interval `bounds` (by node index) and
    /// the clock `uncertainty` in ticks.
    pub fn of(
        formula: &CheckedFormula,
        bounds: &BTreeMap<usize, TickBound>,
        uncertainty: &Rational,
    ) -> Self {
        let width = int(1).add(&int(2).mul(uncertainty));
        let arena = formula.nodes();
        let operands = |index: usize| -> Vec<usize> {
            let (_, _, left, right) =
                operator_parts(&arena[index]).unwrap_or((false, false, None, 0));
            left.into_iter()
                .chain([right])
                .flat_map(|operand| temporal_leaves(formula, operand))
                .collect()
        };
        let temporal: Vec<usize> = (0..arena.len())
            .filter(|index| operator_parts(&arena[*index]).is_some())
            .collect();
        // Lookahead, children first: an operator precedes its operands in the
        // arena, so descending index order visits operands before operators.
        let mut lookahead: BTreeMap<usize, Option<Rational>> = BTreeMap::new();
        for &index in temporal.iter().rev() {
            let (future, _, _, _) =
                operator_parts(&arena[index]).unwrap_or((false, false, None, 0));
            let mut below = Some(int(0));
            for child in operands(index) {
                below = match (below, lookahead.get(&child).cloned().flatten()) {
                    (Some(a), Some(b)) => Some(larger(a, b)),
                    _ => None,
                };
            }
            let value = match bounds.get(&index) {
                Some(bound) if future => match (&bound.upper, below) {
                    (Some(upper), Some(b)) => Some(upper.add(&width).add(&b)),
                    _ => None,
                },
                Some(_) => below,
                None if future => None,
                // An unbounded past operator settles with its operands.
                None => below,
            };
            lookahead.insert(index, value);
        }
        let finite = |index: usize| -> Rational {
            lookahead
                .get(&index)
                .cloned()
                .flatten()
                .unwrap_or_else(|| int(0))
        };
        // The delay `S` of every operator: the sum of the upper bounds of the
        // future operators on the longest path from it down (an unbounded
        // future operator adds none, its instance settles by activation).
        let mut delays: BTreeMap<usize, Rational> = BTreeMap::new();
        for &index in temporal.iter().rev() {
            let (future, ..) = operator_parts(&arena[index]).unwrap_or((false, false, None, 0));
            let own = match bounds.get(&index).and_then(|bound| bound.upper.clone()) {
                Some(upper) if future => upper,
                _ => int(0),
            };
            let below = operands(index)
                .into_iter()
                .fold(int(0), |acc, leaf| larger(acc, delays.get(&leaf).cloned().unwrap_or_else(|| int(0))));
            delays.insert(index, own.add(&below));
        }
        let delay = |index: usize| delays.get(&index).cloned().unwrap_or_else(|| int(0));
        // The evaluation reach `E` of every operator, parents first.
        let root = formula.root().index();
        let mut reach: BTreeMap<usize, Rational> = BTreeMap::new();
        let mut rooted: BTreeMap<usize, bool> = BTreeMap::new();
        let join = |leaves: &[usize]| -> Rational {
            leaves
                .iter()
                .fold(int(0), |acc, leaf| larger(acc, finite(*leaf)))
        };
        let root_leaves = temporal_leaves(formula, root);
        for &index in &temporal {
            let mut candidates = vec![finite(index)];
            let mut is_rooted = false;
            if index == root {
                is_rooted = true;
            }
            if root_leaves.contains(&index) && !temporal.contains(&root) {
                is_rooted = true;
                candidates.push(join(&root_leaves));
            }
            for &parent in temporal.iter().filter(|p| **p < index) {
                if !operands(parent).contains(&index) {
                    continue;
                }
                let (future, ..) =
                    operator_parts(&arena[parent]).unwrap_or((false, false, None, 0));
                let upper = bounds.get(&parent).and_then(|bound| bound.upper.clone());
                if future && upper.is_none() {
                    // An unbounded consumer: its instances fold an entry when
                    // it is dropped, so the operands of one context are kept
                    // for the context's longest lookahead.
                    is_rooted = true;
                    candidates.push(join(&operands(parent)));
                    continue;
                }
                // An enclosing unbounded past operator adds 0 to `L` for its
                // operand and one tick cover to `d` (FR-251).
                let offset = if future {
                    int(0)
                } else {
                    upper.unwrap_or_else(|| int(0)).add(&width)
                };
                let parent_reach = reach.get(&parent).cloned().unwrap_or_else(|| int(0));
                candidates.push(offset.add(&parent_reach));
            }
            rooted.insert(index, is_rooted);
            reach.insert(index, candidates.into_iter().fold(int(0), larger));
        }
        let mut nodes = BTreeMap::new();
        for &index in &temporal {
            let Some((future, dual, left, right)) = operator_parts(&arena[index]) else {
                continue;
            };
            let bound = bounds.get(&index).cloned();
            let settling = operands(index)
                .into_iter()
                .fold(int(0), |acc, leaf| larger(acc, delay(leaf)));
            // The span an entry of the operator is kept for on its own:
            // `b + w` for a bounded past operator, `T(n) + w` with
            // `T(n) = a + S(n)` for an unbounded past one (FR-251).
            let own = if future {
                int(0)
            } else {
                match bound.as_ref().and_then(|bound| bound.upper.clone()) {
                    Some(upper) => upper.add(&width),
                    None => bound
                        .as_ref()
                        .map_or_else(|| int(0), |bound| bound.lower.clone())
                        .add(&settling)
                        .add(&width),
                }
            };
            let buffered = !future || bound.as_ref().is_some_and(|bound| bound.upper.is_some());
            let e = reach.get(&index).cloned().unwrap_or_else(|| int(0));
            let retention = if rooted.get(&index).copied().unwrap_or(false) {
                larger(own, e)
            } else {
                own.add(&e)
            };
            nodes.insert(
                index,
                NodeShape {
                    future,
                    dual,
                    left,
                    right,
                    bound,
                    lookahead: lookahead.get(&index).cloned().flatten(),
                    retention,
                    buffered,
                    settling,
                },
            );
        }
        Self { nodes, width }
    }

    /// The shape of the temporal operator at `index`.
    pub fn node(&self, index: usize) -> Option<&NodeShape> {
        self.nodes.get(&index)
    }

    /// All temporal operators, by ascending index.
    pub fn nodes(&self) -> &BTreeMap<usize, NodeShape> {
        &self.nodes
    }

    /// The widened tick-cover width `1 + 2u`.
    pub fn width(&self) -> &Rational {
        &self.width
    }
}
