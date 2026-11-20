// SPDX-License-Identifier: AGPL-3.0-or-later
//! The rules of `inductive-enclosure` (FSpec FR-652 "Rules of
//! `inductive-enclosure`"): `box-shape`, `a`, `b`, `c`, `d`.

use fsl_semantics::check::{CheckedHybridClaim, CheckedHybridModel, LogicOp};

use super::super::exact::Formula;
use super::super::interval::{Interval, Tri};
use super::claim::{CapStatus, ClaimEval};
use super::machine::{within, Bx, Def, Machine, ModelX};
use super::wire::{WireBox, ROOT};
use super::{names, reject, shaped, At, Fail, Rule};

fn box_pointer(model: &CheckedHybridModel, mode: usize) -> String {
    format!("{ROOT}/boxes/{}", model.modes[mode].name.as_str())
}

/// An obligation's state formula over `readings`, by Kleene's rules; `None`
/// when the formula holds a temporal operator.
fn kleene(formula: &Formula, atoms: &[Tri]) -> Option<Tri> {
    Some(match formula {
        Formula::Atom(i) => atoms[*i],
        Formula::Not(f) => kleene(f, atoms)?.not(),
        Formula::Logic(op, a, b) => {
            let (a, b) = (kleene(a, atoms)?, kleene(b, atoms)?);
            match op {
                LogicOp::And => a.and(b),
                LogicOp::Or => a.or(b),
                LogicOp::Implies => a.not().or(b),
            }
        }
        Formula::Unary { .. } | Formula::Binary { .. } => return None,
    })
}

fn is_state(formula: &Formula) -> bool {
    match formula {
        Formula::Atom(_) => true,
        Formula::Not(f) => is_state(f),
        Formula::Logic(_, a, b) => is_state(a) && is_state(b),
        Formula::Unary { .. } | Formula::Binary { .. } => false,
    }
}

/// The state formulas of the obligations of `formula` (`always φ`,
/// `always I φ`, or an `and` of them) when rule `d` admits the claim form.
fn obligations<'f>(formula: &'f Formula, out: &mut Vec<&'f Formula>) -> bool {
    match formula {
        Formula::Logic(LogicOp::And, a, b) => obligations(a, out) && obligations(b, out),
        Formula::Unary {
            past: false,
            universal: true,
            operand,
            ..
        } => {
            if !is_state(operand) {
                return false;
            }
            out.push(operand);
            true
        }
        _ => false,
    }
}

pub(super) fn check(
    machine: &mut Machine<'_, '_>,
    mx: &ModelX,
    model: &CheckedHybridModel,
    claim: &CheckedHybridClaim,
    boxes: &[WireBox],
) -> Result<(), Fail> {
    let variable_names = names(model);
    let n = mx.n;
    // 1: box-shape
    let mut enclosures: Vec<Bx> = Vec::with_capacity(boxes.len());
    for (mode, wire) in boxes.iter().enumerate() {
        match shaped(wire, &variable_names) {
            Some(bx) => enclosures.push(bx),
            None => return reject(Rule::BoxShape, box_pointer(model, mode)),
        }
    }
    // 2: a
    for (mode, condition) in &mx.initial {
        let pointer = box_pointer(model, *mode);
        let invariant = &mx.modes[*mode].invariant;
        let Some(hull) = machine.hull(&[condition, invariant], n).at(&pointer)? else {
            continue;
        };
        let defined = machine.defined(condition, &hull).at(&pointer)?.is_yes()
            && machine.defined(invariant, &hull).at(&pointer)?.is_yes();
        if !defined || !within(&hull, &enclosures[*mode]) {
            return reject(Rule::A, pointer);
        }
    }
    // 3: b
    for (mode, x) in mx.modes.iter().enumerate() {
        let pointer = box_pointer(model, mode);
        let e = &enclosures[mode];
        for exp in x.rates.iter().chain(std::iter::once(&x.invariant)) {
            if !machine.defined(exp, e).at(&pointer)?.is_yes() {
                return reject(Rule::B, pointer);
            }
        }
        for rate in &x.rates {
            if !machine.strictly_decided(rate, e, false).at(&pointer)? {
                return reject(Rule::B, pointer);
            }
        }
        for i in 0..n {
            for upper in [false, true] {
                if machine.boundary_face(x, i, upper, e).at(&pointer)? {
                    continue;
                }
                let bound = if upper {
                    e[i].upper.clone()
                } else {
                    e[i].lower.clone()
                };
                let mut face = e.clone();
                face[i] = bound.map_or_else(Interval::whole, Interval::point);
                let rate = machine.num(&x.rates[i], &face).at(&pointer)?;
                let inward = if upper {
                    rate.upper
                        .as_ref()
                        .is_some_and(|r| r.is_zero() || r.numerator().is_negative())
                } else {
                    rate.lower
                        .as_ref()
                        .is_some_and(|r| !r.numerator().is_negative())
                };
                if !inward {
                    return reject(Rule::B, pointer);
                }
            }
        }
    }
    // 4: c
    for (mode, x) in mx.modes.iter().enumerate() {
        let source = box_pointer(model, mode);
        for transition in &x.transitions {
            if !machine
                .defined(&transition.guard, &enclosures[mode])
                .at(&source)?
                .is_yes()
            {
                return reject(Rule::C, source);
            }
            let target = &mx.modes[transition.target];
            let Some((g, post)) = machine
                .fire(transition, &enclosures[mode], &target.invariant)
                .at(&source)?
            else {
                continue;
            };
            for (_, _, reset) in &transition.resets {
                if !machine.defined(reset, &g).at(&source)?.is_yes() {
                    return reject(Rule::C, source);
                }
            }
            if !within(&post, &enclosures[transition.target]) {
                return reject(Rule::C, box_pointer(model, transition.target));
            }
        }
    }
    // 5: d
    let eval = ClaimEval::build(machine, claim, n).at(ROOT)?;
    let mut states = Vec::new();
    if !obligations(&eval.formula, &mut states) {
        return reject(Rule::D, ROOT);
    }
    if eval.atoms.iter().any(|atom| {
        atom.any_node(|node| matches!(node, super::super::expand::Exp::Variable(i) if *i >= 2 * n))
    }) {
        return reject(Rule::D, ROOT);
    }
    for (mode, e) in enclosures.iter().enumerate() {
        let pointer = box_pointer(model, mode);
        let (ctx, status) = eval.context(machine, e, e).at(&pointer)?;
        if !matches!(status, CapStatus::Ok) {
            return reject(Rule::D, pointer);
        }
        let ext = ClaimEval::ext(e, &ctx);
        if let Some(guard) = &eval.guard {
            if !machine.defined(guard, &ext).at(&pointer)?.is_yes() {
                return reject(Rule::D, pointer);
            }
        }
        let readings = eval.atoms_over(machine, e, &ctx).at(&pointer)?;
        if readings.iter().any(|(_, def)| !matches!(def, Def::Yes)) {
            return reject(Rule::D, pointer);
        }
        let values: Vec<Tri> = readings.iter().map(|(tri, _)| *tri).collect();
        for state in &states {
            if kleene(state, &values) != Some(Tri::True) {
                return reject(Rule::D, pointer);
            }
        }
    }
    Ok(())
}
