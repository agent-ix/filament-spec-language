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
        struct Pending<'a> {
            term: &'a HybridTerm,
            position: Position,
            /// The path length before this term's incoming edge.
            base_path_len: usize,
            incoming_edge: Option<usize>,
        }

        let mut pending = vec![Pending {
            term,
            position,
            base_path_len: path.len(),
            incoming_edge: None,
        }];
        let mut current_path = path.to_vec();

        while let Some(next) = pending.pop() {
            #[cfg(test)]
            {
                self.stats.term_visits += 1;
                self.stats.path_steps += 1;
            }
            current_path.truncate(next.base_path_len);
            if let Some(edge) = next.incoming_edge {
                current_path.push(edge);
                #[cfg(test)]
                {
                    self.stats.path_steps += 1;
                }
            }

            let here = |form| {
                Some(Gap {
                    form,
                    locus: TermLocus {
                        term: current_path.clone(),
                        body: Vec::new(),
                    },
                })
            };
            match (next.term, next.position) {
                (HybridTerm::Constant(_) | HybridTerm::Variable(_), Position::Numeric)
                | (HybridTerm::Boolean(_), Position::Boolean) => {}
                (HybridTerm::Negate(a), Position::Numeric) => {
                    pending.push(Pending {
                        term: a,
                        position: Position::Numeric,
                        base_path_len: current_path.len(),
                        incoming_edge: Some(0),
                    });
                    #[cfg(test)]
                    {
                        self.stats.term_frames += 1;
                    }
                }
                (HybridTerm::Arithmetic(ArithmeticOp::Div, _, _), Position::Numeric) => {
                    #[cfg(test)]
                    {
                        self.stats.path_elements_copied += current_path.len();
                    }
                    return Ok(here("division"));
                }
                (HybridTerm::Arithmetic(_, a, b), Position::Numeric)
                | (HybridTerm::Logic(_, a, b), Position::Boolean)
                | (HybridTerm::Compare(_, a, b), Position::Boolean) => {
                    let child_position = match next.position {
                        Position::Numeric => Position::Numeric,
                        Position::Boolean if matches!(next.term, HybridTerm::Logic(..)) => {
                            Position::Boolean
                        }
                        Position::Boolean => Position::Numeric,
                    };
                    pending.push(Pending {
                        term: b,
                        position: child_position,
                        base_path_len: current_path.len(),
                        incoming_edge: Some(1),
                    });
                    pending.push(Pending {
                        term: a,
                        position: child_position,
                        base_path_len: current_path.len(),
                        incoming_edge: Some(0),
                    });
                    #[cfg(test)]
                    {
                        self.stats.term_frames += 2;
                    }
                }
                (HybridTerm::Not(a), Position::Boolean) => {
                    pending.push(Pending {
                        term: a,
                        position: Position::Boolean,
                        base_path_len: current_path.len(),
                        incoming_edge: Some(0),
                    });
                    #[cfg(test)]
                    {
                        self.stats.term_frames += 1;
                    }
                }
                (HybridTerm::Call(..), _) => {
                    let expanded = self.expander.term(next.term, self.meter)?;
                    #[cfg(test)]
                    let gap =
                        polynomial_gap_with_stats(&expanded, next.position, Some(&mut self.stats));
                    #[cfg(not(test))]
                    let gap = polynomial_gap(&expanded, next.position);
                    if let Some((form, inner)) = gap {
                        // The expansion's loci are relative to the call, so
                        // the call's own path leads them.
                        let mut locus = inner.unwrap_or_default();
                        #[cfg(test)]
                        {
                            self.stats.path_elements_copied += current_path.len();
                        }
                        locus.term.splice(0..0, current_path.iter().copied());
                        return Ok(Some(Gap { form, locus }));
                    }
                }
                // A Boolean-valued `if` is a Boolean combination; a numeric
                // one is piecewise, so not a polynomial.
                (HybridTerm::Conditional(c, a, b), Position::Boolean) => {
                    pending.push(Pending {
                        term: b,
                        position: Position::Boolean,
                        base_path_len: current_path.len(),
                        incoming_edge: Some(2),
                    });
                    pending.push(Pending {
                        term: a,
                        position: Position::Boolean,
                        base_path_len: current_path.len(),
                        incoming_edge: Some(1),
                    });
                    pending.push(Pending {
                        term: c,
                        position: Position::Boolean,
                        base_path_len: current_path.len(),
                        incoming_edge: Some(0),
                    });
                    #[cfg(test)]
                    {
                        self.stats.term_frames += 3;
                    }
                }
                (HybridTerm::Conditional(..), Position::Numeric) => {
                    #[cfg(test)]
                    {
                        self.stats.path_elements_copied += current_path.len();
                    }
                    return Ok(here("conditional"));
                }
                (HybridTerm::Capture(_) | HybridTerm::ActivationVariable(_), _) => {
                    #[cfg(test)]
                    {
                        self.stats.path_elements_copied += current_path.len();
                    }
                    return Ok(here("activation"));
                }
                _ => {
                    #[cfg(test)]
                    {
                        self.stats.path_elements_copied += current_path.len();
                    }
                    return Ok(here("mixed-kinds"));
                }
            }
        }
        Ok(None)
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

fn polynomial_gap_with_stats(
    exp: &Exp,
    position: Position,
    mut stats: Option<&mut WalkStats>,
) -> Option<(&'static str, Option<TermLocus>)> {
    let mut pending = vec![(exp, position)];
    if let Some(stats) = stats.as_deref_mut() {
        stats.exp_frames += 1;
    }
    while let Some((next, position)) = pending.pop() {
        if let Some(stats) = stats.as_deref_mut() {
            stats.exp_visits += 1;
        }
        match (next, position) {
            (Exp::Number(_) | Exp::Variable(_) | Exp::Boolean(_), _) => {}
            (Exp::Negate(a) | Exp::Not(a), _) => {
                pending.push((a, position));
                if let Some(stats) = stats.as_deref_mut() {
                    stats.exp_frames += 1;
                }
            }
            (Exp::Ranged { value, .. }, _) => {
                pending.push((value, position));
                if let Some(stats) = stats.as_deref_mut() {
                    stats.exp_frames += 1;
                }
            }
            (Exp::Arithmetic(ArithmeticOp::Div, _, _, locus), _) => {
                return Some(("division", Some(locus.clone())));
            }
            (Exp::Arithmetic(_, a, b, _), _) => {
                pending.push((b, Position::Numeric));
                pending.push((a, Position::Numeric));
                if let Some(stats) = stats.as_deref_mut() {
                    stats.exp_frames += 2;
                }
            }
            (Exp::Logic(_, a, b), _) => {
                pending.push((b, Position::Boolean));
                pending.push((a, Position::Boolean));
                if let Some(stats) = stats.as_deref_mut() {
                    stats.exp_frames += 2;
                }
            }
            (Exp::Compare(_, a, b), _) => {
                pending.push((b, Position::Numeric));
                pending.push((a, Position::Numeric));
                if let Some(stats) = stats.as_deref_mut() {
                    stats.exp_frames += 2;
                }
            }
            (Exp::Conditional(c, a, b, _), Position::Boolean) => {
                pending.push((b, Position::Boolean));
                pending.push((a, Position::Boolean));
                pending.push((c, Position::Boolean));
                if let Some(stats) = stats.as_deref_mut() {
                    stats.exp_frames += 3;
                }
            }
            (Exp::Conditional(.., locus), Position::Numeric) => {
                return Some(("conditional", Some(locus.clone())));
            }
            (Exp::Recursive(call), _) => {
                return Some(("recursive-call", Some(call.locus.clone())));
            }
            (Exp::Activation(_) | Exp::Capture(_) | Exp::Parameter(_), _) => {
                return Some(("activation", None));
            }
        }
    }
    None
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
    use ix_trace_rs::trace;

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
        assert_eq!(stats.exp_frames, depth + 1);

        // The pre-change term walk clones every path prefix. Its deterministic
        // element-copy count exceeds the linear candidate bound at each size.
        let prior_prefix_copies = depth * (depth + 1) / 2;
        assert!(prior_prefix_copies > 2 * (depth + 1));
    }

    #[test]
    #[trace("NFR-141-AC-22", "FR-1019")]
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
                assert_eq!(stats.exp_frames, 100_001);
            })
            .expect("small-stack thread starts")
            .join()
            .expect("both explicit-stack walkers support depth 100,000");
    }

    #[test]
    #[trace("NFR-141-AC-22", "FR-1019")]
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
