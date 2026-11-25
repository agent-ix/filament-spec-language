// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-1019 "Class": the first construct of a checked hybrid model outside the
//! class a flowpipe provider decides. Every flow right-hand side, after
//! expansion of non-recursive calls, is built from constants, variables,
//! unary `-`, `+`, `-` and `*`; guards, resets, invariants and initial
//! conditions are Boolean combinations of comparisons of such terms.

use fsl_semantics::check::{
    ArithmeticOp, CheckedGraph, CheckedHybridModel, HybridTerm, ModeIndex, VariableIndex,
};
use std::ops::ControlFlow;

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
#[derive(Debug, Eq, PartialEq)]
struct Gap {
    form: &'static str,
    locus: TermLocus,
}

struct Walk<'a, 'g> {
    expander: &'a mut Expander<'g>,
    meter: &'a mut HybridReplayMeter,
    #[cfg(test)]
    stats: WalkStats,
}

#[derive(Default)]
struct WalkStats {
    term_visits: usize,
    term_frames: usize,
    path_steps: usize,
    path_elements_copied: usize,
    exp_visits: usize,
    exp_frames: usize,
}

impl Walk<'_, '_> {
    fn term(
        &mut self,
        term: &HybridTerm,
        path: &[usize],
        position: Position,
    ) -> Result<Option<Gap>, HybridReplayRefusal> {
        let mut walker = TermWalk {
            expander: &mut *self.expander,
            meter: &mut *self.meter,
            path_prefix: path.to_vec(),
            paths: vec![PathLink {
                parent: None,
                edge: 0,
                depth: 0,
            }],
            #[cfg(test)]
            stats: &mut self.stats,
            #[cfg(not(test))]
            stats: std::marker::PhantomData,
            term: std::marker::PhantomData,
        };
        let root = TermNode {
            term,
            position,
            path: 0,
        };
        match filament_walk::walk(&mut walker, root) {
            ControlFlow::Continue(()) => Ok(None),
            ControlFlow::Break(TermStop::Gap(gap)) => Ok(Some(gap)),
            ControlFlow::Break(TermStop::Refusal(refusal)) => Err(refusal),
        }
    }
}

/// One path edge in the indexed arena kept by the class walker. Indices make
/// path destruction iterative too; a linked `Rc` chain would recurse on drop.
struct PathLink {
    parent: Option<usize>,
    edge: usize,
    depth: usize,
}

struct TermNode<'a> {
    term: &'a HybridTerm,
    position: Position,
    path: usize,
}

enum TermStop {
    Gap(Gap),
    Refusal(HybridReplayRefusal),
}

struct TermWalk<'a, 'g, 't, 's> {
    expander: &'a mut Expander<'g>,
    meter: &'a mut HybridReplayMeter,
    path_prefix: Vec<usize>,
    paths: Vec<PathLink>,
    #[cfg(test)]
    stats: &'s mut WalkStats,
    #[cfg(not(test))]
    stats: std::marker::PhantomData<&'s mut WalkStats>,
    term: std::marker::PhantomData<&'t HybridTerm>,
}

impl TermWalk<'_, '_, '_, '_> {
    fn child(&mut self, parent: usize, edge: usize) -> usize {
        let path = self.paths.len();
        let depth = self.paths[parent].depth + 1;
        self.paths.push(PathLink {
            parent: Some(parent),
            edge,
            depth,
        });
        #[cfg(test)]
        {
            self.stats.path_steps += 1;
        }
        path
    }

    fn locus(&mut self, path: usize) -> TermLocus {
        let depth = self.paths[path].depth;
        let mut term = Vec::with_capacity(self.path_prefix.len() + depth);
        term.extend_from_slice(&self.path_prefix);
        let mut edges = Vec::with_capacity(depth);
        let mut current = path;
        while current != 0 {
            let link = &self.paths[current];
            edges.push(link.edge);
            current = link.parent.expect("non-root path has a parent");
        }
        #[cfg(test)]
        {
            self.stats.path_steps += edges.len();
            self.stats.path_elements_copied += term.len() + edges.len();
        }
        term.extend(edges.into_iter().rev());
        TermLocus {
            term,
            body: Vec::new(),
        }
    }
}

impl<'a, 'g, 't, 's> filament_walk::Walk for TermWalk<'a, 'g, 't, 's> {
    type Node = TermNode<'t>;
    type Frame = ();
    type Stop = TermStop;

    fn enter(
        &mut self,
        node: Self::Node,
        children: &mut filament_walk::Children<'_, Self::Node>,
    ) -> ControlFlow<Self::Stop, Self::Frame> {
        #[cfg(test)]
        {
            self.stats.term_visits += 1;
        }
        let gap = |walker: &mut Self, form| {
            ControlFlow::Break(TermStop::Gap(Gap {
                form,
                locus: walker.locus(node.path),
            }))
        };
        match (node.term, node.position) {
            (HybridTerm::Constant(_) | HybridTerm::Variable(_), Position::Numeric)
            | (HybridTerm::Boolean(_), Position::Boolean) => {}
            (HybridTerm::Negate(a), Position::Numeric) => {
                let path = self.child(node.path, 0);
                children.push(TermNode {
                    term: a,
                    position: Position::Numeric,
                    path,
                });
                #[cfg(test)]
                {
                    self.stats.term_frames += 1;
                }
            }
            (HybridTerm::Arithmetic(ArithmeticOp::Div, _, _), Position::Numeric) => {
                return gap(self, "division");
            }
            (HybridTerm::Arithmetic(_, a, b), Position::Numeric)
            | (HybridTerm::Logic(_, a, b), Position::Boolean)
            | (HybridTerm::Compare(_, a, b), Position::Boolean) => {
                let child_position = match node.position {
                    Position::Numeric => Position::Numeric,
                    Position::Boolean if matches!(node.term, HybridTerm::Logic(..)) => {
                        Position::Boolean
                    }
                    Position::Boolean => Position::Numeric,
                };
                let left = self.child(node.path, 0);
                let right = self.child(node.path, 1);
                children.push(TermNode {
                    term: a,
                    position: child_position,
                    path: left,
                });
                children.push(TermNode {
                    term: b,
                    position: child_position,
                    path: right,
                });
                #[cfg(test)]
                {
                    self.stats.term_frames += 2;
                }
            }
            (HybridTerm::Not(a), Position::Boolean) => {
                let path = self.child(node.path, 0);
                children.push(TermNode {
                    term: a,
                    position: Position::Boolean,
                    path,
                });
                #[cfg(test)]
                {
                    self.stats.term_frames += 1;
                }
            }
            (HybridTerm::Call(..), _) => {
                let expanded = match self.expander.term(node.term, self.meter) {
                    Ok(expanded) => expanded,
                    Err(refusal) => return ControlFlow::Break(TermStop::Refusal(refusal)),
                };
                #[cfg(test)]
                let inner =
                    polynomial_gap_with_stats(&expanded, node.position, Some(&mut *self.stats));
                #[cfg(not(test))]
                let inner = polynomial_gap(&expanded, node.position);
                if let Some((form, inner)) = inner {
                    let mut locus = inner.unwrap_or_default();
                    let prefix = self.locus(node.path).term;
                    #[cfg(test)]
                    {
                        self.stats.path_elements_copied += prefix.len() + locus.term.len();
                    }
                    let mut term = prefix;
                    term.append(&mut locus.term);
                    locus.term = term;
                    return ControlFlow::Break(TermStop::Gap(Gap { form, locus }));
                }
            }
            (HybridTerm::Conditional(c, a, b), Position::Boolean) => {
                for (edge, term) in [(0, c), (1, a), (2, b)] {
                    let path = self.child(node.path, edge);
                    children.push(TermNode {
                        term,
                        position: Position::Boolean,
                        path,
                    });
                    #[cfg(test)]
                    {
                        self.stats.term_frames += 1;
                    }
                }
            }
            (HybridTerm::Conditional(..), Position::Numeric) => {
                return gap(self, "conditional");
            }
            (HybridTerm::Capture(_) | HybridTerm::ActivationVariable(_), _) => {
                return gap(self, "activation");
            }
            _ => return gap(self, "mixed-kinds"),
        }
        ControlFlow::Continue(())
    }

    fn exit(&mut self, (): Self::Frame) -> ControlFlow<Self::Stop> {
        ControlFlow::Continue(())
    }
}

/// The first node of an expanded expression outside the class, with its
/// locus when the expansion kept one. A call's argument and result ranges
/// (`Exp::Ranged`) are definedness conditions on a value that is the wrapped
/// term's, as in exact replay's polynomial flow (FR-450 "Exact replay"), so
/// they do not leave the class.
fn polynomial_gap(exp: &Exp, position: Position) -> Option<(&'static str, Option<TermLocus>)> {
    polynomial_gap_with_stats(exp, position, None)
}

struct PolynomialNode<'a> {
    exp: &'a Exp,
    position: Position,
}

struct PolynomialWalk<'a> {
    stats: Option<&'a mut WalkStats>,
}

impl<'a> PolynomialWalk<'a> {
    fn push(
        &mut self,
        children: &mut filament_walk::Children<'_, PolynomialNode<'a>>,
        exp: &'a Exp,
        position: Position,
    ) {
        children.push(PolynomialNode { exp, position });
        #[cfg(test)]
        if let Some(stats) = self.stats.as_deref_mut() {
            stats.exp_frames += 1;
        }
    }
}

impl<'a> filament_walk::Walk for PolynomialWalk<'a> {
    type Node = PolynomialNode<'a>;
    type Frame = ();
    type Stop = (&'static str, Option<TermLocus>);

    fn enter(
        &mut self,
        node: Self::Node,
        children: &mut filament_walk::Children<'_, Self::Node>,
    ) -> ControlFlow<Self::Stop, Self::Frame> {
        #[cfg(test)]
        if let Some(stats) = self.stats.as_deref_mut() {
            stats.exp_visits += 1;
        }
        match (node.exp, node.position) {
            (Exp::Number(_) | Exp::Variable(_) | Exp::Boolean(_), _) => {}
            (Exp::Negate(a) | Exp::Not(a), _) => self.push(children, a, node.position),
            (Exp::Ranged { value, .. }, _) => self.push(children, value, node.position),
            (Exp::Arithmetic(ArithmeticOp::Div, _, _, locus), _) => {
                return ControlFlow::Break(("division", Some(locus.clone())));
            }
            (Exp::Arithmetic(_, a, b, _), _) => {
                self.push(children, a, Position::Numeric);
                self.push(children, b, Position::Numeric);
            }
            (Exp::Logic(_, a, b), _) => {
                self.push(children, a, Position::Boolean);
                self.push(children, b, Position::Boolean);
            }
            (Exp::Compare(_, a, b), _) => {
                self.push(children, a, Position::Numeric);
                self.push(children, b, Position::Numeric);
            }
            (Exp::Conditional(c, a, b, _), Position::Boolean) => {
                self.push(children, c, Position::Boolean);
                self.push(children, a, Position::Boolean);
                self.push(children, b, Position::Boolean);
            }
            (Exp::Conditional(.., locus), Position::Numeric) => {
                return ControlFlow::Break(("conditional", Some(locus.clone())));
            }
            (Exp::Recursive(call), _) => {
                return ControlFlow::Break(("recursive-call", Some(call.locus.clone())));
            }
            (Exp::Activation(_) | Exp::Capture(_) | Exp::Parameter(_), _) => {
                return ControlFlow::Break(("activation", None));
            }
        }
        ControlFlow::Continue(())
    }

    fn exit(&mut self, (): Self::Frame) -> ControlFlow<Self::Stop> {
        ControlFlow::Continue(())
    }
}

fn polynomial_gap_with_stats(
    exp: &Exp,
    position: Position,
    stats: Option<&mut WalkStats>,
) -> Option<(&'static str, Option<TermLocus>)> {
    let mut walker = PolynomialWalk { stats };
    match filament_walk::walk(&mut walker, PolynomialNode { exp, position }) {
        ControlFlow::Continue(()) => None,
        ControlFlow::Break(gap) => Some(gap),
    }
}

/// The first construct of `model` outside the class of FR-1019 "Class", in
/// source order: the initial entries, then each mode in declared order (its
/// invariant, its flow equations, then each transition's guard and reset
/// values), each expression by a pre-order walk. A checked model stores a
/// mode's rates by variable, in the order the variables are first declared
/// across the modes, so a mode's flow equations are taken in the declaration
/// order of the variables they define, not in the order the mode writes
/// them (FR-1019 "Class"). `Ok(None)` when every construct is inside it.
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
        #[cfg(test)]
        stats: WalkStats::default(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use filament_exact::Rational;
    use fsl_semantics::check::CompareOp;
    use ix_trace_rs::trace;

    fn q(numerator: i64) -> Rational {
        Rational::new(numerator.into(), 1.into()).expect("one is a nonzero denominator")
    }

    fn deep_term(depth: usize) -> &'static HybridTerm {
        let mut term = Box::new(HybridTerm::ActivationVariable(VariableIndex::new(0)));
        for _ in 0..depth {
            term = Box::new(HybridTerm::Negate(term));
        }
        Box::leak(term)
    }

    fn deep_exp(depth: usize) -> &'static Exp {
        let mut exp = Box::new(Exp::Activation(0));
        for _ in 0..depth {
            exp = Box::new(Exp::Negate(exp));
        }
        Box::leak(exp)
    }

    fn assert_linear_hybrid_walk(graph: &CheckedGraph, depth: usize) {
        let term = deep_term(depth);
        let mut expander = Expander::new(graph);
        let mut meter = HybridReplayMeter::new(u64::MAX);
        let mut walk = Walk {
            expander: &mut expander,
            meter: &mut meter,
            stats: WalkStats::default(),
        };
        let gap = walk
            .term(term, &[], Position::Numeric)
            .expect("walk does not expand calls")
            .expect("the terminal activation is outside FR-1019 Class");
        assert_eq!(gap.form, "activation");
        assert_eq!(gap.locus.term, vec![0; depth]);
        assert_eq!(walk.stats.term_visits, depth + 1);
        assert_eq!(walk.stats.term_frames, depth);
        assert!(walk.stats.path_steps <= 2 * (depth + 1));
        assert!(walk.stats.path_elements_copied <= 2 * (depth + 1));

        let exp = deep_exp(depth);
        let mut stats = WalkStats::default();
        let gap = polynomial_gap_with_stats(exp, Position::Numeric, Some(&mut stats))
            .expect("the terminal activation is outside FR-1019 Class");
        assert_eq!(gap.0, "activation");
        assert_eq!(stats.exp_visits, depth + 1);
        assert_eq!(stats.exp_frames, depth);

        // The pre-change term walk clones every path prefix. Its deterministic
        // element-copy count exceeds the linear candidate bound at each size.
        let prior_prefix_copies = depth * (depth + 1) / 2;
        assert!(prior_prefix_copies > 2 * (depth + 1));
    }

    fn prior_polynomial_gap(
        exp: &Exp,
        position: Position,
    ) -> Option<(&'static str, Option<TermLocus>)> {
        match (exp, position) {
            (Exp::Number(_) | Exp::Variable(_) | Exp::Boolean(_), _) => None,
            (Exp::Negate(a) | Exp::Not(a), _) => prior_polynomial_gap(a, position),
            (Exp::Ranged { value, .. }, _) => prior_polynomial_gap(value, position),
            (Exp::Arithmetic(ArithmeticOp::Div, _, _, locus), _) => {
                Some(("division", Some(locus.clone())))
            }
            (Exp::Arithmetic(_, a, b, _), _) => prior_polynomial_gap(a, Position::Numeric)
                .or_else(|| prior_polynomial_gap(b, Position::Numeric)),
            (Exp::Logic(_, a, b), _) => prior_polynomial_gap(a, Position::Boolean)
                .or_else(|| prior_polynomial_gap(b, Position::Boolean)),
            (Exp::Compare(_, a, b), _) => prior_polynomial_gap(a, Position::Numeric)
                .or_else(|| prior_polynomial_gap(b, Position::Numeric)),
            (Exp::Conditional(c, a, b, _), Position::Boolean) => {
                prior_polynomial_gap(c, Position::Boolean)
                    .or_else(|| prior_polynomial_gap(a, Position::Boolean))
                    .or_else(|| prior_polynomial_gap(b, Position::Boolean))
            }
            (Exp::Conditional(.., locus), Position::Numeric) => {
                Some(("conditional", Some(locus.clone())))
            }
            (Exp::Recursive(call), _) => Some(("recursive-call", Some(call.locus.clone()))),
            (Exp::Activation(_) | Exp::Capture(_) | Exp::Parameter(_), _) => {
                Some(("activation", None))
            }
        }
    }

    fn prior_term(
        term: &HybridTerm,
        path: &[usize],
        position: Position,
        expander: &mut Expander<'_>,
        meter: &mut HybridReplayMeter,
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
        let child = |edge: usize| {
            let mut next = path.to_vec();
            next.push(edge);
            next
        };
        Ok(match (term, position) {
            (HybridTerm::Constant(_) | HybridTerm::Variable(_), Position::Numeric)
            | (HybridTerm::Boolean(_), Position::Boolean) => None,
            (HybridTerm::Negate(a), Position::Numeric) => {
                prior_term(a, &child(0), Position::Numeric, expander, meter)?
            }
            (HybridTerm::Arithmetic(ArithmeticOp::Div, _, _), Position::Numeric) => {
                here("division")
            }
            (HybridTerm::Arithmetic(_, a, b), Position::Numeric) => {
                match prior_term(a, &child(0), Position::Numeric, expander, meter)? {
                    Some(gap) => Some(gap),
                    None => prior_term(b, &child(1), Position::Numeric, expander, meter)?,
                }
            }
            (HybridTerm::Not(a), Position::Boolean) => {
                prior_term(a, &child(0), Position::Boolean, expander, meter)?
            }
            (HybridTerm::Logic(_, a, b), Position::Boolean) => {
                match prior_term(a, &child(0), Position::Boolean, expander, meter)? {
                    Some(gap) => Some(gap),
                    None => prior_term(b, &child(1), Position::Boolean, expander, meter)?,
                }
            }
            (HybridTerm::Compare(_, a, b), Position::Boolean) => {
                match prior_term(a, &child(0), Position::Numeric, expander, meter)? {
                    Some(gap) => Some(gap),
                    None => prior_term(b, &child(1), Position::Numeric, expander, meter)?,
                }
            }
            (HybridTerm::Call(..), _) => {
                let expanded = expander.term(term, meter)?;
                prior_polynomial_gap(&expanded, position).map(|(form, inner)| {
                    let mut locus = inner.unwrap_or_default();
                    locus.term.splice(0..0, path.iter().copied());
                    Gap { form, locus }
                })
            }
            (HybridTerm::Conditional(c, a, b), Position::Boolean) => {
                let mut found = prior_term(c, &child(0), Position::Boolean, expander, meter)?;
                if found.is_none() {
                    found = prior_term(a, &child(1), Position::Boolean, expander, meter)?;
                }
                if found.is_none() {
                    found = prior_term(b, &child(2), Position::Boolean, expander, meter)?;
                }
                found
            }
            (HybridTerm::Conditional(..), Position::Numeric) => here("conditional"),
            (HybridTerm::Capture(_) | HybridTerm::ActivationVariable(_), _) => here("activation"),
            _ => here("mixed-kinds"),
        })
    }

    #[test]
    #[trace("NFR-141-AC-27", "FR-1019")]
    fn bounded_branch_conditional_and_call_walks_match_the_prior_oracle() {
        let graph = super::super::tests::graph_for_class_walk();
        let div = || {
            Box::new(HybridTerm::Arithmetic(
                ArithmeticOp::Div,
                Box::new(HybridTerm::Constant(q(1))),
                Box::new(HybridTerm::Constant(q(0))),
            ))
        };
        let fixtures = [
            (
                HybridTerm::Arithmetic(ArithmeticOp::Add, div(), div()),
                Position::Numeric,
            ),
            (
                HybridTerm::Conditional(
                    Box::new(HybridTerm::Boolean(true)),
                    Box::new(HybridTerm::Boolean(false)),
                    Box::new(HybridTerm::Compare(
                        CompareOp::Eq,
                        div(),
                        Box::new(HybridTerm::Constant(q(1))),
                    )),
                ),
                Position::Boolean,
            ),
            (
                HybridTerm::Arithmetic(
                    ArithmeticOp::Add,
                    Box::new(HybridTerm::Constant(q(1))),
                    Box::new(HybridTerm::Call(
                        fsl_semantics::check::CheckedFunctionRef::new(1),
                        vec![HybridTerm::Constant(q(1))],
                    )),
                ),
                Position::Numeric,
            ),
        ];
        for (term, position) in fixtures {
            let mut expander = Expander::new(&graph);
            let mut meter = HybridReplayMeter::new(u64::MAX);
            let expected = prior_term(&term, &[], position, &mut expander, &mut meter)
                .expect("bounded oracle walk completes");
            let mut expander = Expander::new(&graph);
            let mut meter = HybridReplayMeter::new(u64::MAX);
            let actual = Walk {
                expander: &mut expander,
                meter: &mut meter,
                stats: WalkStats::default(),
            }
            .term(&term, &[], position)
            .expect("bounded toolkit walk completes");
            assert_eq!(actual, expected);
        }

        let locus = |term| TermLocus {
            term,
            body: Vec::new(),
        };
        let number = || Exp::Number(q(1));
        let fixtures = [
            (
                Exp::Conditional(
                    Box::new(Exp::Boolean(true)),
                    Box::new(Exp::Boolean(false)),
                    Box::new(Exp::Compare(
                        CompareOp::Eq,
                        Box::new(Exp::Arithmetic(
                            ArithmeticOp::Div,
                            Box::new(number()),
                            Box::new(number()),
                            locus(vec![8, 2]),
                        )),
                        Box::new(number()),
                    )),
                    locus(vec![9]),
                ),
                Position::Boolean,
            ),
            (
                Exp::Compare(
                    CompareOp::Eq,
                    Box::new(Exp::Activation(0)),
                    Box::new(Exp::Arithmetic(
                        ArithmeticOp::Div,
                        Box::new(number()),
                        Box::new(number()),
                        locus(vec![4]),
                    )),
                ),
                Position::Boolean,
            ),
        ];
        for (exp, position) in fixtures {
            assert_eq!(
                polynomial_gap(&exp, position),
                prior_polynomial_gap(&exp, position)
            );
        }
    }

    #[test]
    #[trace("NFR-141-AC-27", "FR-1019")]
    fn hybrid_class_walks_scale_and_support_one_hundred_thousand_depth() {
        let graph = super::super::tests::graph_for_class_walk();
        for depth in [5_000, 10_000, 20_000] {
            assert_linear_hybrid_walk(&graph, depth);
        }

        let term = deep_term(100_000);
        let exp = deep_exp(100_000);
        std::thread::Builder::new()
            .name("hybrid-class-depth-100000".to_owned())
            .stack_size(512 * 1024)
            .spawn(move || {
                let mut expander = Expander::new(&graph);
                let mut meter = HybridReplayMeter::new(u64::MAX);
                let mut walk = Walk {
                    expander: &mut expander,
                    meter: &mut meter,
                    stats: WalkStats::default(),
                };
                let gap = walk
                    .term(term, &[], Position::Numeric)
                    .expect("deep walk does not expand calls")
                    .expect("terminal activation leaves the class");
                assert_eq!(gap.locus.term.len(), 100_000);
                assert_eq!(walk.stats.term_visits, 100_001);
                assert_eq!(walk.stats.term_frames, 100_000);

                let mut stats = WalkStats::default();
                let gap = polynomial_gap_with_stats(exp, Position::Numeric, Some(&mut stats))
                    .expect("terminal activation leaves the class");
                assert_eq!(gap.0, "activation");
                assert_eq!(stats.exp_visits, 100_001);
                assert_eq!(stats.exp_frames, 100_000);
            })
            .expect("small-stack thread starts")
            .join()
            .expect("both explicit-stack walkers support depth 100,000");
    }

    #[test]
    #[trace("NFR-141-AC-27", "FR-1019")]
    fn prior_recursive_class_walk_fails_only_in_a_child_process() {
        const CHILD: &str = "FSL_RECURSIVE_CLASS_WALK_CHILD";
        if std::env::var_os(CHILD).is_some() {
            fn prior_recursive_walk(term: &HybridTerm) -> usize {
                match term {
                    HybridTerm::Negate(inner) => 1 + prior_recursive_walk(inner),
                    _ => 0,
                }
            }

            let term = deep_term(100_000);
            std::thread::Builder::new()
                .stack_size(512 * 1024)
                .spawn(move || std::hint::black_box(prior_recursive_walk(term)))
                .expect("small-stack child thread starts")
                .join()
                .expect("the prior recursive walk overflows this isolated stack");
            return;
        }

        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "hybrid_replay::class::tests::prior_recursive_class_walk_fails_only_in_a_child_process",
                "--nocapture",
            ])
            .env(CHILD, "1")
            .status()
            .expect("test binary starts as an isolated child");
        assert!(
            !status.success(),
            "the old recursive walk unexpectedly survived"
        );
    }
}
