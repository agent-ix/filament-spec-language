// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-750: the FSpec FR-331 `Locus` of the term node an [`ExpressionLocus`]
//! names in the checked package, the locus an `unsupported` construct and an
//! `undefined-evaluation` counterexample cite.

use fsl_semantics::check::wire_id::WireGraph;
use fsl_semantics::check::{CheckedGraph, HybridFormula, HybridTerm, HybridTermHome};
use serde_json::Value;

use crate::hybrid_replay::{ExpressionLocus, ExpressionSite, TermLocus};

/// The `{node, occurrence}` locus of the term node `locus` names in the
/// claim at position `claim` of `graph`, with the node spelled by `wire`.
/// `None` when the package has no such claim, expression or term.
pub fn hybrid_locus_wire(
    graph: &CheckedGraph,
    wire: &WireGraph,
    claim: usize,
    locus: &ExpressionLocus,
) -> Option<Value> {
    let model = graph.hybrid_claims().get(claim)?.model.get();
    let home = match locus.site {
        ExpressionSite::Initial { entry } => HybridTermHome::Initial { model, entry },
        ExpressionSite::Invariant { mode } => HybridTermHome::Invariant { model, mode },
        ExpressionSite::Rate { mode, variable } => HybridTermHome::Rate {
            model,
            mode,
            variable,
        },
        ExpressionSite::Guard { mode, transition } => HybridTermHome::Guard {
            model,
            mode,
            transition,
        },
        ExpressionSite::Reset {
            mode,
            transition,
            constraint,
        } => HybridTermHome::Reset {
            model,
            mode,
            transition,
            constraint,
        },
        ExpressionSite::Activation => HybridTermHome::Activation { claim },
        ExpressionSite::Capture { capture } => HybridTermHome::Capture { claim, capture },
        ExpressionSite::ClaimAtom { atom } => HybridTermHome::ClaimAtom { claim, atom },
    };
    let found = graph.hybrid_term_node(home, &locus.term.term)?;
    Some(serde_json::json!({
        "node": wire.wire_value(&found.node).ok()?,
        "occurrence": {
            "role": found.origin.role().as_str(),
            "ordinal": found.origin.ordinal(),
        },
    }))
}

/// The inverse of [`hybrid_locus_wire`]: the expression site and term path of
/// the term node the FSpec FR-331 `Locus` `locus` names, among the
/// expressions of mode `mode` of the model the claim at position `claim`
/// ranges over: the rate of `variable` when it is given, and otherwise the
/// mode's invariant, guards, reset values and the initial conditions that
/// name it. `None` when the locus names no term node of those expressions.
pub fn hybrid_locus_site(
    graph: &CheckedGraph,
    wire: &WireGraph,
    claim: usize,
    mode: usize,
    variable: Option<usize>,
    locus: &Value,
) -> Option<ExpressionLocus> {
    let model_index = graph.hybrid_claims().get(claim)?.model.get();
    let model = graph.hybrid_models().get(model_index)?;
    let target = wire.key_of_value(locus.get("node")?).ok()?;
    let checked = model.modes.get(mode)?;
    let mut homes = Vec::new();
    if let Some(variable) = variable {
        homes.push((
            HybridTermHome::Rate {
                model: model_index,
                mode,
                variable,
            },
            ExpressionSite::Rate { mode, variable },
        ));
    } else {
        homes.push((
            HybridTermHome::Invariant {
                model: model_index,
                mode,
            },
            ExpressionSite::Invariant { mode },
        ));
        for (transition, jump) in checked.transitions.iter().enumerate() {
            homes.push((
                HybridTermHome::Guard {
                    model: model_index,
                    mode,
                    transition,
                },
                ExpressionSite::Guard { mode, transition },
            ));
            for constraint in 0..jump.resets.len() {
                homes.push((
                    HybridTermHome::Reset {
                        model: model_index,
                        mode,
                        transition,
                        constraint,
                    },
                    ExpressionSite::Reset {
                        mode,
                        transition,
                        constraint,
                    },
                ));
            }
        }
        for (entry, initial) in model.initial.iter().enumerate() {
            if initial.mode.get() == mode {
                homes.push((
                    HybridTermHome::Initial {
                        model: model_index,
                        entry,
                    },
                    ExpressionSite::Initial { entry },
                ));
            }
        }
    }
    homes.into_iter().find_map(|(home, site)| {
        graph
            .hybrid_term_path(home, &target)
            .map(|term| ExpressionLocus {
                site,
                term: TermLocus {
                    term,
                    body: Vec::new(),
                },
            })
    })
}

/// Resolve any emitted term locus in one hybrid counterexample's claim or
/// model expression back to its checked expression path.
pub fn hybrid_counterexample_locus(
    graph: &CheckedGraph,
    wire: &WireGraph,
    claim: usize,
    locus: &Value,
) -> Option<ExpressionLocus> {
    let checked_claim = graph.hybrid_claims().get(claim)?;
    let model = checked_claim.model.get();
    let target = wire.key_of_value(locus.get("node")?).ok()?;
    let mut homes = Vec::new();
    let checked_model = graph.hybrid_models().get(model)?;
    for (entry, _) in checked_model.initial.iter().enumerate() {
        homes.push((
            HybridTermHome::Initial { model, entry },
            ExpressionSite::Initial { entry },
        ));
    }
    for (mode, checked_mode) in checked_model.modes.iter().enumerate() {
        homes.push((
            HybridTermHome::Invariant { model, mode },
            ExpressionSite::Invariant { mode },
        ));
        for (variable, _) in checked_mode.rates.iter().enumerate() {
            homes.push((
                HybridTermHome::Rate {
                    model,
                    mode,
                    variable,
                },
                ExpressionSite::Rate { mode, variable },
            ));
        }
        for (transition, checked_transition) in checked_mode.transitions.iter().enumerate() {
            homes.push((
                HybridTermHome::Guard {
                    model,
                    mode,
                    transition,
                },
                ExpressionSite::Guard { mode, transition },
            ));
            for (constraint, _) in checked_transition.resets.iter().enumerate() {
                homes.push((
                    HybridTermHome::Reset {
                        model,
                        mode,
                        transition,
                        constraint,
                    },
                    ExpressionSite::Reset {
                        mode,
                        transition,
                        constraint,
                    },
                ));
            }
        }
    }
    if matches!(
        &checked_claim.activation,
        fsl_semantics::check::HybridActivation::Each { guard: Some(_), .. }
    ) {
        homes.push((
            HybridTermHome::Activation { claim },
            ExpressionSite::Activation,
        ));
    }
    for (capture, _) in checked_claim.captures.iter().enumerate() {
        homes.push((
            HybridTermHome::Capture { claim, capture },
            ExpressionSite::Capture { capture },
        ));
    }
    let mut atoms = Vec::new();
    fn collect_atoms<'a>(
        formula: &'a HybridFormula,
        atoms: &mut Vec<&'a fsl_semantics::check::HybridTerm>,
    ) {
        match formula {
            HybridFormula::Holds(term) => atoms.push(term),
            HybridFormula::Not(inner)
            | HybridFormula::Always(_, inner)
            | HybridFormula::Eventually(_, inner)
            | HybridFormula::Historically(_, inner)
            | HybridFormula::Once(_, inner) => collect_atoms(inner, atoms),
            HybridFormula::Logic(_, left, right)
            | HybridFormula::Until(_, left, right)
            | HybridFormula::Release(_, left, right)
            | HybridFormula::Since(_, left, right)
            | HybridFormula::Triggered(_, left, right) => {
                collect_atoms(left, atoms);
                collect_atoms(right, atoms);
            }
        }
    }
    collect_atoms(&checked_claim.formula, &mut atoms);
    for (atom, _) in atoms.iter().enumerate() {
        homes.push((
            HybridTermHome::ClaimAtom { claim, atom },
            ExpressionSite::ClaimAtom { atom },
        ));
    }
    homes.into_iter().find_map(|(home, site)| {
        let term = graph.hybrid_term_path(home, &target)?;
        Some(ExpressionLocus {
            site,
            term: TermLocus {
                term,
                body: Vec::new(),
            },
        })
    })
}

/// The checked expression term at a resolved locus.
pub fn hybrid_term_at_locus<'a>(
    graph: &'a CheckedGraph,
    claim_index: usize,
    locus: &ExpressionLocus,
) -> Option<&'a HybridTerm> {
    let claim = graph.hybrid_claims().get(claim_index)?;
    let model = graph.hybrid_models().get(claim.model.get())?;
    let root = match locus.site {
        ExpressionSite::Initial { entry } => &model.initial.get(entry)?.condition,
        ExpressionSite::Invariant { mode } => &model.modes.get(mode)?.invariant,
        ExpressionSite::Rate { mode, variable } => model.modes.get(mode)?.rates.get(variable)?,
        ExpressionSite::Guard { mode, transition } => {
            &model.modes.get(mode)?.transitions.get(transition)?.guard
        }
        ExpressionSite::Reset {
            mode,
            transition,
            constraint,
        } => {
            &model
                .modes
                .get(mode)?
                .transitions
                .get(transition)?
                .resets
                .get(constraint)?
                .value
        }
        ExpressionSite::Activation => match &claim.activation {
            fsl_semantics::check::HybridActivation::Each {
                guard: Some(guard), ..
            } => guard,
            _ => return None,
        },
        ExpressionSite::Capture { capture } => &claim.captures.get(capture)?.value,
        ExpressionSite::ClaimAtom { atom } => {
            fn atom_at<'a>(
                formula: &'a HybridFormula,
                target: usize,
                seen: &mut usize,
            ) -> Option<&'a HybridTerm> {
                match formula {
                    HybridFormula::Holds(term) => {
                        let position = *seen;
                        *seen += 1;
                        (position == target).then_some(term)
                    }
                    HybridFormula::Not(inner)
                    | HybridFormula::Always(_, inner)
                    | HybridFormula::Eventually(_, inner)
                    | HybridFormula::Historically(_, inner)
                    | HybridFormula::Once(_, inner) => atom_at(inner, target, seen),
                    HybridFormula::Logic(_, left, right)
                    | HybridFormula::Until(_, left, right)
                    | HybridFormula::Release(_, left, right)
                    | HybridFormula::Since(_, left, right)
                    | HybridFormula::Triggered(_, left, right) => {
                        atom_at(left, target, seen).or_else(|| atom_at(right, target, seen))
                    }
                }
            }
            atom_at(&claim.formula, atom, &mut 0)?
        }
    };
    let mut current = root;
    for position in &locus.term.term {
        current = match current {
            HybridTerm::Negate(child) | HybridTerm::Not(child) if *position == 0 => child,
            HybridTerm::Arithmetic(_, left, right)
            | HybridTerm::Compare(_, left, right)
            | HybridTerm::Logic(_, left, right) => match position {
                0 => left,
                1 => right,
                _ => return None,
            },
            HybridTerm::Conditional(condition, yes, no) => match position {
                0 => condition,
                1 => yes,
                2 => no,
                _ => return None,
            },
            HybridTerm::Call(_, arguments) => arguments.get(*position)?,
            _ => return None,
        };
    }
    Some(current)
}
