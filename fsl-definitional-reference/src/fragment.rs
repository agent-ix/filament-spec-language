// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-1051's accept-only fragment check over a checked graph.
//!
//! Every construct is refused unless a rule here admits it, so a construct
//! the language gains later is refused until the reference defines it.

use filament_checked::node::{Arithmetic, Connective, NodeKind};
use filament_contract_model::CheckedFrameModification;
use filament_exact::{Presence, Value};
use fsl_semantics::check::{
    BinaryTemporal, BoundedMeasure, CheckedFormula, CheckedGraph, CheckedNode,
    CheckedStatePredicate, ClaimActivation, DelayDistribution, Event, FormulaNode, HybridTerm,
    ProbForm, ReachTarget, StateClauseKind, StateGraphForm, TerminalDeclaration, TimedModel,
    UnaryTemporal,
};
use fsl_semantics::model::domain_package::{
    DomainPackage, DomainPackageRecord, NativeValueType, ScalarValueDomain, ValueTypeRef,
};
use ix_trace_rs::implements;

use crate::{ReferenceRefusal, scheduler_quantifier, validate_quantile_scheduler};

/// The reference's state bound.
const MAX_VALUATIONS: u128 = 1 << 16;

/// Expression nodes read before a body is refused as too large.
const MAX_NODES: usize = 100_000;

fn refuse(construct: impl Into<String>) -> ReferenceRefusal {
    ReferenceRefusal {
        construct: construct.into(),
    }
}

/// What position an expression is read in.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Position {
    /// A guard, effect or frame clause: a conjunction of comparisons.
    Conjunction,
    /// A claim predicate: any Boolean combination of comparisons.
    Combination,
    /// A comparison operand.
    Term,
}

/// Refuses every checked construct outside the fragment the reference
/// defines, before any truth value can be returned.
#[implements("FR-1051-AC-19")]
pub fn validate_tiny_fragment(graph: &CheckedGraph) -> Result<(), ReferenceRefusal> {
    crate::validate_conjunctive_guards(graph)?;
    for package in graph
        .selected_model_views()
        .values()
        .map(|view| view.domain_package())
        .chain(graph.source_packages())
    {
        fields(package)?;
    }
    for model in graph.timed_models() {
        timed(model)?;
    }
    hybrid(graph)?;
    probabilistic(graph)?;
    for clause in graph.state_clauses() {
        let position = match clause.kind() {
            StateClauseKind::Precondition | StateClauseKind::Postcondition => Position::Conjunction,
            _ => Position::Combination,
        };
        expression(clause.body(), position)?;
    }
    for claim in graph.state_graph_claims() {
        let predicates = match claim.form() {
            StateGraphForm::Possible { target } => vec![target],
            StateGraphForm::AlwaysPossible { from, target } => vec![from, target],
            StateGraphForm::UniquePath { from, to } => vec![from, to],
        };
        for predicate in predicates {
            match predicate {
                CheckedStatePredicate::ConstantTrue => {}
                CheckedStatePredicate::Checked(body) => {
                    expression(body.body(), Position::Combination)?;
                }
                CheckedStatePredicate::Package(_) => {
                    return Err(refuse("state-graph predicate without a source body"));
                }
            }
        }
    }
    for clause in graph.temporal_clauses() {
        formula(clause.clause().formula())?;
    }
    for clause in graph.hyper_clauses() {
        if clause.quantifiers().len() > 2 {
            return Err(refuse("hyper formula over more than two copies"));
        }
        if clause.match_expression().is_some() || !clause.align_skip().is_empty() {
            return Err(refuse("hyper step matching"));
        }
        formula(clause.body())?;
        for index in 0..clause.body().nodes().len() {
            let atom = clause.body().node_id(index);
            if let Some((root, _)) = atom.and_then(|atom| clause.condition(atom)) {
                expression(root, Position::Combination)?;
            }
        }
    }
    uninterpreted(graph)?;
    effects(graph)
}

/// Declaration families the reference has no definition for.
fn uninterpreted(graph: &CheckedGraph) -> Result<(), ReferenceRefusal> {
    let present = [
        (!graph.protocols().is_empty(), "protocol declaration"),
        (!graph.refinements().is_empty(), "refinement declaration"),
        (!graph.step_relations().is_empty(), "model relation"),
        (
            !graph.finite_hyper_clauses().is_empty(),
            "finite-trace hyper clause",
        ),
        (!graph.synthesis_claims().is_empty(), "synthesis claim"),
        (!graph.syntheses().is_empty(), "synthesis declaration"),
        (!graph.hybrid_claims().is_empty(), "hybrid claim"),
        (!graph.type_invariants().is_empty(), "type invariant"),
        (
            graph
                .terminals()
                .iter()
                .any(|row| !matches!(row.declaration(), TerminalDeclaration::None)),
            "terminal declaration",
        ),
        (
            !graph.task_sets().task_sets().is_empty() || !graph.task_sets().claims().is_empty(),
            "task set",
        ),
    ];
    match present.iter().find(|(found, _)| *found) {
        Some((_, construct)) => Err(refuse(*construct)),
        None => Ok(()),
    }
}

/// State is finitely many Boolean, enumeration or bounded integer fields
/// (and clocks), at most 2^16 valuations.
fn fields(package: &DomainPackage) -> Result<(), ReferenceRefusal> {
    let mut valuations = 1_u128;
    for record in &package.records {
        let DomainPackageRecord::FieldMember(field) = record else {
            continue;
        };
        if field.value_type == ValueTypeRef::Native(NativeValueType::Integer) {
            return Err(refuse("unbounded integer field"));
        }
        if field.value_type == ValueTypeRef::Clock {
            continue;
        }
        if field.multiplicity.lower != 1 || field.multiplicity.upper != Some(1) {
            return Err(refuse("collection-valued field"));
        }
        if field.presence != Presence::Required {
            return Err(refuse("optional field"));
        }
        let size = match &field.value_type {
            ValueTypeRef::Native(NativeValueType::Boolean) => 2,
            ValueTypeRef::Package(key) => package
                .records
                .iter()
                .find_map(|record| match record {
                    DomainPackageRecord::Enum(found) if found.key == *key => {
                        u128::try_from(found.members.len()).ok()
                    }
                    DomainPackageRecord::ScalarType(found) if found.key == *key => {
                        match &found.domain {
                            ScalarValueDomain::Integer { lower, upper } => {
                                upper.checked_sub(*lower)?.checked_add(1)?.try_into().ok()
                            }
                            _ => None,
                        }
                    }
                    _ => None,
                })
                .ok_or_else(|| refuse("field type that is not finite"))?,
            _ => return Err(refuse("field type that is not finite")),
        };
        valuations = valuations
            .checked_mul(size)
            .filter(|count| *count <= MAX_VALUATIONS)
            .ok_or_else(|| refuse("finite state space exceeds 2^16 valuations"))?;
    }
    Ok(())
}

/// Clock constraints are conjunctions with integer constants; resets write
/// zero.
fn timed(model: &TimedModel) -> Result<(), ReferenceRefusal> {
    for guard in model.guards() {
        for part in guard.parts() {
            if !part.presences().is_empty()
                || !part.element_presences().is_empty()
                || !part.elements().is_empty()
                || !part.families().is_empty()
            {
                return Err(refuse("timed guard over presences or elements"));
            }
        }
    }
    let constraints = model
        .guards()
        .iter()
        .flat_map(|guard| guard.parts())
        .flat_map(|part| part.clocks())
        .chain(model.invariants().iter().flat_map(|item| item.bounds()));
    for constraint in constraints {
        if !constraint.bound().is_integer() {
            return Err(refuse("clock compared with a non-integer constant"));
        }
        if !constraint.via().is_empty()
            || !constraint.slots().is_empty()
            || constraint.parameter().is_some()
        {
            return Err(refuse("indirect or parameterized clock constraint"));
        }
    }
    for reset in model.resets().iter().flat_map(|item| item.resets()) {
        if !reset.value().is_zero() {
            return Err(refuse("clock reset to a nonzero constant"));
        }
        if reset.parameter().is_some() || reset.selection().is_some() {
            return Err(refuse("selected clock reset"));
        }
    }
    if !model.urgency().is_empty() {
        return Err(refuse("urgency declaration"));
    }
    Ok(())
}

/// A non-constant rate is named; the reference defines no hybrid semantics,
/// so any other hybrid model is refused too.
fn hybrid(graph: &CheckedGraph) -> Result<(), ReferenceRefusal> {
    let models = graph.hybrid_models();
    if models.is_empty() {
        return Ok(());
    }
    let varying = models
        .iter()
        .flat_map(|model| &model.modes)
        .flat_map(|mode| &mode.rates)
        .any(|rate| !matches!(rate, HybridTerm::Constant(_)));
    Err(refuse(if varying {
        "hybrid mode with a non-constant rate"
    } else {
        "hybrid model"
    }))
}

/// Finite literal supports, literal rewards and the claim forms over state
/// predicates.
fn probabilistic(graph: &CheckedGraph) -> Result<(), ReferenceRefusal> {
    let model = graph.probabilistic_model();
    for reward in model.rewards() {
        if !matches!(
            reward.expression().kind(),
            NodeKind::Literal(literal) if matches!(
                literal.0,
                Value::Integer(_) | Value::Rational(_) | Value::Quantity(_)
            )
        ) {
            return Err(refuse("non-literal reward"));
        }
    }
    for delay in model.delays() {
        if !matches!(delay.distribution(), DelayDistribution::Discrete(_)) {
            return Err(refuse("continuous delay distribution"));
        }
    }
    for claim in model.claims() {
        if matches!(claim.form, ProbForm::Quantile { .. }) {
            validate_quantile_scheduler(&scheduler_quantifier(&claim.scheduler))?;
        }
        if !claim.fairness.is_empty() {
            return Err(refuse("fairness constraint on a probabilistic claim"));
        }
        if !matches!(claim.activation, ClaimActivation::Origin) {
            return Err(refuse("on-each activation"));
        }
        let mut predicates = Vec::new();
        match &claim.form {
            ProbForm::Probability { event, .. } => match event {
                Event::Formula {
                    formula: checked, ..
                } => {
                    formula(checked.formula())?;
                    for node in 0..checked.formula().nodes().len() {
                        predicates.extend(checked.condition(node));
                    }
                }
                Event::Compare { measure, .. } => measured(measure, &mut predicates),
            },
            ProbForm::Quantile { measure, .. } => measured(measure, &mut predicates),
            ProbForm::MeanOfFraction { fraction, .. } => predicates.push(&fraction.predicate),
            ProbForm::LongRunFraction { predicate, .. } => predicates.push(predicate),
            ProbForm::Reach { target, .. } => match target {
                ReachTarget::Eventually(one) | ReachTarget::Always(one) => predicates.push(one),
                ReachTarget::Until(left, right) => predicates.extend([left, right]),
            },
            ProbForm::ExpectedReward { until, .. } | ProbForm::MeanTime { until, .. } => {
                predicates.push(until);
            }
        }
        for predicate in predicates {
            let body = predicate
                .body()
                .ok_or_else(|| refuse("claim predicate without a source body"))?;
            expression(body, Position::Combination)?;
        }
    }
    Ok(())
}

fn measured<'a>(
    measure: &'a BoundedMeasure,
    predicates: &mut Vec<&'a fsl_semantics::check::CheckedPredicate>,
) {
    match measure {
        BoundedMeasure::Accumulate { from, until, .. }
        | BoundedMeasure::Steps { from, until, .. }
        | BoundedMeasure::Elapsed { from, until, .. } => {
            predicates.extend([from.as_ref(), until.as_ref()]);
        }
        BoundedMeasure::Fraction(fraction) => predicates.push(&fraction.predicate),
    }
}

/// `until`, `always`, `eventually`, their windows and Boolean connectives.
fn formula(formula: &CheckedFormula) -> Result<(), ReferenceRefusal> {
    for node in formula.nodes() {
        match node {
            FormulaNode::Constant(_)
            | FormulaNode::Holds { .. }
            | FormulaNode::Not(_)
            | FormulaNode::And(..)
            | FormulaNode::Or(..)
            | FormulaNode::Implies(..)
            | FormulaNode::Unary {
                operator: UnaryTemporal::Eventually | UnaryTemporal::Always,
                ..
            }
            | FormulaNode::Binary {
                operator: BinaryTemporal::Until,
                ..
            } => {}
            FormulaNode::Aggregate { .. } => return Err(refuse("window aggregate")),
            FormulaNode::Unary { .. } => return Err(refuse("past temporal operator")),
            FormulaNode::Binary { .. } => {
                return Err(refuse("temporal operator outside the fragment"));
            }
        }
    }
    Ok(())
}

fn expression(root: CheckedNode<'_>, position: Position) -> Result<(), ReferenceRefusal> {
    let mut budget = MAX_NODES;
    let mut pending = vec![(root, position)];
    while let Some((node, position)) = pending.pop() {
        budget = budget
            .checked_sub(1)
            .ok_or_else(|| refuse("checked expression exceeds the reference node bound"))?;
        match (node.kind(), position) {
            (
                NodeKind::Connective(Connective::And, _, left, right),
                Position::Conjunction | Position::Combination,
            ) => {
                pending.push((node.at(*left), position));
                pending.push((node.at(*right), position));
            }
            (
                NodeKind::Connective(Connective::Or | Connective::Implies, _, left, right),
                Position::Combination,
            ) => {
                pending.push((node.at(*left), position));
                pending.push((node.at(*right), position));
            }
            (NodeKind::Connective(Connective::Or, ..), _) => {
                return Err(refuse("disjunctive operation guard"));
            }
            (NodeKind::Connective(Connective::Implies, ..), _) => {
                return Err(refuse("implication in a guard or effect"));
            }
            (NodeKind::Not(operand), Position::Combination) => {
                pending.push((node.at(*operand), position));
            }
            // A negated conjunction is a disjunction, so only one comparison
            // or Boolean field may be negated outside a claim.
            (NodeKind::Not(operand), _) => pending.push((node.at(*operand), Position::Term)),
            (NodeKind::Order(_, _, left, right) | NodeKind::Equality(_, _, left, right), _) => {
                pending.push((node.at(*left), Position::Term));
                pending.push((node.at(*right), Position::Term));
            }
            (NodeKind::Literal(literal), _)
                if matches!(
                    literal.0,
                    Value::Boolean(_) | Value::Integer(_) | Value::Rational(_) | Value::Enum(_)
                ) => {}
            (NodeKind::Local(_), _) => {}
            (NodeKind::Attribute { reference, .. }, _) => {
                if !matches!(node.at(*reference).kind(), NodeKind::Local(_)) {
                    return Err(refuse("field read through another field"));
                }
            }
            (NodeKind::Pre(operand) | NodeKind::Coerce(operand, _), _) => {
                pending.push((node.at(*operand), position));
            }
            (
                NodeKind::Arithmetic(Arithmetic::Add | Arithmetic::Subtract, left, right),
                Position::Term,
            ) => {
                let literal = |mut operand: CheckedNode<'_>| {
                    while let NodeKind::Coerce(inner, _) = operand.kind() {
                        operand = operand.at(*inner);
                    }
                    matches!(
                        operand.kind(),
                        NodeKind::Literal(found) if matches!(found.0, Value::Integer(_))
                    )
                };
                if literal(node.at(*left)) == literal(node.at(*right)) {
                    return Err(refuse("non-linear integer term"));
                }
                pending.push((node.at(*left), position));
                pending.push((node.at(*right), position));
            }
            (kind, _) => return Err(refuse(crate::interpreter::node_name(kind))),
        }
    }
    Ok(())
}

/// Operations write fields only: no creation, deletion or relationship.
fn effects(graph: &CheckedGraph) -> Result<(), ReferenceRefusal> {
    for row in graph.operation_effects().into_iter().flatten() {
        let effect = &row.effect;
        if !effect.creates.is_empty()
            || !effect.deletes.is_empty()
            || !effect.deletes_scoped.is_empty()
        {
            return Err(refuse("object creation or deletion effect"));
        }
        if effect
            .modifies
            .iter()
            .any(|item| matches!(item, CheckedFrameModification::Relationship { .. }))
        {
            return Err(refuse("relationship effect"));
        }
    }
    Ok(())
}
