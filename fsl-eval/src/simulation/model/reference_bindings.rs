// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-1000: resolve subject-independent reference literals at FR-120 subject
//! admission, without changing the checked graph.

use std::collections::{BTreeMap, BTreeSet};

use filament_checked::node::NodeKind;
use filament_exact::{EffectiveId, ObjectReference};
use filament_semantic_value::declaration::TypeEnvironment;
use fsl_foundation::diagnostic::Code;
use fsl_semantics::check::{
    CheckedGraph, CheckedLifetime, CheckedNode, FormulaNode, FormulaNodeId, TemporalClauseNode,
};
use fsl_semantics::model::state::{StateModel, StatePopulation};

use super::ModelSimState;

/// The identity-bearing part of one checked reference literal.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct ReferenceLiteralKey {
    pub(crate) object_type: EffectiveId,
    pub(crate) identity: Vec<u8>,
}

/// The subject's resolution of one reference literal.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum ReferenceBinding {
    /// The key exists in an initial state; preserve its model reference even
    /// when evaluating another state.
    Fixed(ObjectReference),
    /// The key belongs to the subject's universe, but no initial state holds
    /// it. Read it against each evaluated state.
    CreatedObject {
        object_type: EffectiveId,
        identity: String,
        /// The stable absent-key locus, selected from universe occurrences.
        population: filament_checked::key::DeclarationKey,
        /// Every population the literal's type may inhabit, for lookup and
        /// resolved-footprint reads.
        populations: Vec<filament_checked::key::DeclarationKey>,
    },
}

/// One immutable subject's reference-literal decisions.
#[derive(Clone, Debug, Default)]
pub struct ReferenceBindings {
    base: std::sync::Arc<BTreeMap<ReferenceLiteralKey, ReferenceBinding>>,
    additional: Vec<(ReferenceLiteralKey, ReferenceBinding)>,
}

impl PartialEq for ReferenceBindings {
    fn eq(&self, other: &Self) -> bool {
        self.base
            .keys()
            .chain(self.additional.iter().map(|(key, _)| key))
            .chain(other.base.keys())
            .chain(other.additional.iter().map(|(key, _)| key))
            .all(|key| self.borrowed(key) == other.borrowed(key))
    }
}
impl Eq for ReferenceBindings {}

impl ReferenceBindings {
    fn borrowed(&self, key: &ReferenceLiteralKey) -> Option<&ReferenceBinding> {
        self.additional
            .iter()
            .rev()
            .find(|(candidate, _)| candidate == key)
            .map(|(_, value)| value)
            .or_else(|| self.base.get(key))
    }

    fn of_map(base: BTreeMap<ReferenceLiteralKey, ReferenceBinding>) -> Self {
        Self {
            base: std::sync::Arc::new(base),
            additional: Vec::new(),
        }
    }

    fn merged(&self) -> BTreeMap<ReferenceLiteralKey, ReferenceBinding> {
        let mut merged = (*self.base).clone();
        merged.extend(self.additional.iter().cloned());
        merged
    }

    pub(crate) fn created_for_footprints(
        &self,
    ) -> BTreeMap<(EffectiveId, Vec<u8>), Vec<filament_checked::key::DeclarationKey>> {
        self.base
            .iter()
            .chain(self.additional.iter().map(|(key, binding)| (key, binding)))
            .filter_map(|(literal, binding)| match binding {
                ReferenceBinding::CreatedObject { populations, .. } => Some((
                    (literal.object_type.clone(), literal.identity.clone()),
                    populations.clone(),
                )),
                ReferenceBinding::Fixed(_) => None,
            })
            .collect()
    }

    pub(crate) fn get(
        &self,
        object_type: &EffectiveId,
        identity: &[u8],
    ) -> Option<&ReferenceBinding> {
        self.additional
            .iter()
            .rev()
            .find(|(key, _)| key.object_type == *object_type && key.identity == identity)
            .map(|(_, binding)| binding)
            .or_else(|| {
                self.base.get(&ReferenceLiteralKey {
                    object_type: object_type.clone(),
                    identity: identity.to_vec(),
                })
            })
    }

    /// Extend the model's decisions with the admitted literals of one package clause.
    pub(super) fn with_literals(
        &self,
        literals: Vec<(EffectiveId, Vec<u8>)>,
        model: &StateModel,
        types: &TypeEnvironment,
        populations: &[StatePopulation],
        universes: &BTreeMap<filament_checked::key::DeclarationKey, Vec<String>>,
        initial: &[ModelSimState],
    ) -> Result<Self, fsl_semantics::model::observation::AdmissionFailure> {
        let literals = literals
            .into_iter()
            .map(|(object_type, identity)| ReferenceLiteralKey {
                object_type,
                identity,
            })
            .collect();
        let mut extended = self.merged();
        extended.extend(resolve_literals(
            literals,
            &BTreeSet::new(),
            model,
            types,
            populations,
            universes,
            initial,
        )?);
        Ok(Self::of_map(extended))
    }

    /// Resolve every checked reference literal against the validated subject
    /// universes and initial states. A literal in a barred protocol slot may
    /// name a fixed object, but not one that is created only by later states.
    pub(super) fn bind(
        graph: &CheckedGraph,
        model: &StateModel,
        types: &TypeEnvironment,
        populations: &[StatePopulation],
        universes: &BTreeMap<filament_checked::key::DeclarationKey, Vec<String>>,
        initial: &[ModelSimState],
    ) -> Result<Self, fsl_semantics::model::observation::AdmissionFailure> {
        let mut literals = BTreeSet::new();
        let mut barred = BTreeSet::new();

        for clause in graph.state_clauses() {
            collect_literals(graph, clause.body(), &mut literals);
        }
        for clause in graph.temporal_clauses() {
            for (index, formula_node) in clause.clause().formula().nodes().iter().enumerate() {
                let atom = FormulaNodeId::from_arena_index(index);
                if matches!(formula_node, FormulaNode::Holds { .. }) {
                    if let Some((condition, _)) = clause.condition(atom) {
                        collect_literals(graph, condition, &mut literals);
                    }
                }
                if matches!(formula_node, FormulaNode::Aggregate { .. }) {
                    if let Some(aggregate) = clause.aggregate_conditions(atom) {
                        if let Some((value, _)) = aggregate.value {
                            collect_literals(graph, value, &mut literals);
                        }
                        if let Some((filter, _)) = aggregate.filter {
                            collect_literals(graph, filter, &mut literals);
                        }
                        if let Some((predicate, _)) = aggregate.predicate {
                            collect_literals(graph, predicate, &mut literals);
                        }
                        collect_literals(graph, aggregate.threshold.0, &mut literals);
                    }
                }
            }
            if let Some((guard, _)) = clause.guard() {
                collect_literals(graph, guard, &mut literals);
            }
            for index in 0..clause.clause().captures().len() {
                if let Some((capture, _)) = clause.capture(index) {
                    collect_literals(graph, capture, &mut literals);
                }
            }
        }
        // Hyper-atom literals carry a component only when S3 found an
        // enclosing `@ trace`. W38 allows fixed values anywhere, but a
        // created-object lookup without that component is barred.
        for clause in graph.hyper_clauses() {
            for (index, formula_node) in clause.body().nodes().iter().enumerate() {
                if !matches!(formula_node, FormulaNode::Holds { .. }) {
                    continue;
                }
                let atom = FormulaNodeId::from_arena_index(index);
                let Some((condition, _)) = clause.condition(atom) else {
                    continue;
                };
                collect_literals(graph, condition, &mut literals);
                let Some(components) = clause.component_reads(atom) else {
                    continue;
                };
                let mut visited_functions = BTreeSet::new();
                let mut pending = vec![(condition, false)];
                while let Some((node, indexed)) = pending.pop() {
                    let indexed = indexed || components.contains_key(node.location());
                    pending.extend(node.children().into_iter().map(|child| (child, indexed)));
                    if let NodeKind::Call { function, .. } = node.kind() {
                        if visited_functions.insert((*function, indexed)) {
                            if let Some(function) = graph.function_state(*function) {
                                pending.push((function.body, indexed));
                            }
                        }
                    }
                    if let NodeKind::ReferenceLiteral {
                        object_type,
                        identity,
                    } = node.kind()
                    {
                        if !indexed {
                            barred.insert(ReferenceLiteralKey {
                                object_type: object_type.clone(),
                                identity: identity.clone(),
                            });
                        }
                    }
                }
            }
        }
        for protocol in graph.protocols() {
            let slots = &protocol.slots;
            for body in &slots.bodies {
                collect_literals(graph, body.body.root(), &mut literals);
                if protocol
                    .controls
                    .control(body.node)
                    .is_some_and(|control| control.kind == fsl_forms::ProtocolNodeKind::Check)
                {
                    collect_literals(graph, body.body.root(), &mut barred);
                }
            }
            for choice in &slots.choices {
                for visible in &choice.visible {
                    collect_literals(graph, visible.root(), &mut literals);
                    collect_literals(graph, visible.root(), &mut barred);
                }
                for case in &choice.cases {
                    collect_literals(graph, case.guard.root(), &mut literals);
                    collect_literals(graph, case.guard.root(), &mut barred);
                }
            }
            for repeat in &slots.repeats {
                collect_literals(graph, repeat.guard.root(), &mut literals);
                collect_literals(graph, repeat.guard.root(), &mut barred);
                for visible in &repeat.visible {
                    collect_literals(graph, visible.root(), &mut literals);
                    collect_literals(graph, visible.root(), &mut barred);
                }
                if let Some(proof) = &repeat.loop_proof {
                    collect_literals(graph, proof.invariant.root(), &mut literals);
                    collect_literals(graph, proof.variant.root(), &mut literals);
                }
            }
            for role in &slots.roles {
                if let CheckedLifetime::Until { predicate, .. } = &role.lifetime {
                    collect_literals(graph, predicate.root(), &mut literals);
                }
            }
            for related in &slots.related {
                for end in &related.ends {
                    collect_literals(graph, end.root(), &mut literals);
                }
            }
        }

        // The package graph can contain literals for several selected model
        // aliases. A subject binds only literals whose type its admitted
        // model declares; model-independent clause admission handles the
        // remaining cases.
        literals.retain(|literal| model.type_key(literal.object_type.clone()).is_some());
        barred.retain(|literal| literals.contains(literal));
        Ok(Self::of_map(resolve_literals(
            literals,
            &barred,
            model,
            types,
            populations,
            universes,
            initial,
        )?))
    }

    /// Add the literals from a separately checked hyper clause. Hyper clauses
    /// are checked apart from the package's base graph, so their bindings must
    /// be admitted against each component's subject here.
    #[allow(
        clippy::too_many_arguments,
        reason = "each clause form binds its own distinct set of checked parts"
    )]
    pub(super) fn with_hyper_clause(
        &self,
        graph: &CheckedGraph,
        clause: &fsl_semantics::check::CheckedHyperClause,
        model: &StateModel,
        types: &TypeEnvironment,
        populations: &[StatePopulation],
        universes: &BTreeMap<filament_checked::key::DeclarationKey, Vec<String>>,
        initial: &[ModelSimState],
    ) -> Result<Self, fsl_semantics::model::observation::AdmissionFailure> {
        let mut literals = BTreeSet::new();
        let mut barred = BTreeSet::new();
        for (index, formula) in clause.body().nodes().iter().enumerate() {
            if !matches!(formula, FormulaNode::Holds { .. }) {
                continue;
            }
            let atom = FormulaNodeId::from_arena_index(index);
            let Some((condition, _)) = clause.condition(atom) else {
                continue;
            };
            collect_literals(graph, condition, &mut literals);
            let components = clause.component_reads(atom);
            let mut visited_functions = BTreeSet::new();
            let mut pending = vec![(condition, false)];
            while let Some((node, indexed)) = pending.pop() {
                let indexed =
                    indexed || components.is_some_and(|reads| reads.contains_key(node.location()));
                pending.extend(node.children().into_iter().map(|child| (child, indexed)));
                if let NodeKind::Call { function, .. } = node.kind() {
                    if visited_functions.insert((*function, indexed)) {
                        if let Some(function) = graph.function_state(*function) {
                            pending.push((function.body, indexed));
                        }
                    }
                }
                if let NodeKind::ReferenceLiteral {
                    object_type,
                    identity,
                } = node.kind()
                {
                    if !indexed {
                        barred.insert(ReferenceLiteralKey {
                            object_type: object_type.clone(),
                            identity: identity.clone(),
                        });
                    }
                }
            }
        }
        // A hyper component can select a model that does not declare a
        // literal's qualified type. Such a component has no binding for that
        // literal and does not participate in cross-component agreement.
        literals.retain(|literal| model.type_key(literal.object_type.clone()).is_some());
        barred.retain(|literal| literals.contains(literal));
        let mut extended = self.merged();
        extended.extend(resolve_literals(
            literals,
            &barred,
            model,
            types,
            populations,
            universes,
            initial,
        )?);
        Ok(Self::of_map(extended))
    }

    /// Read the clause's literal occurrences without retaining identity copies.
    pub(super) fn contains_temporal_literals_with_budget(
        &self,
        graph: &CheckedGraph,
        clause: &TemporalClauseNode,
        budget: &crate::policy_budget::Budget<'_>,
    ) -> Result<bool, crate::automaton::PolicyStateError> {
        let mut complete = true;
        temporal_roots(clause, &mut || budget.work(1), &mut |root| {
            budget.work(1)?;
            for (object_type, identity) in literal_refs(graph, root, budget)? {
                let mut found = false;
                for (key, _) in self
                    .base
                    .iter()
                    .chain(self.additional.iter().map(|(key, binding)| (key, binding)))
                {
                    budget.work(1)?;
                    if budget.equal_effectives(&key.object_type, object_type)?
                        && budget.compare_bytes(&key.identity, identity)?
                            == std::cmp::Ordering::Equal
                    {
                        found = true;
                        break;
                    }
                }
                complete &= found;
            }
            Ok(())
        })?;
        Ok(complete)
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "the resolver reads one immutable subject"
    )]
    pub(super) fn with_temporal_clause_with_budget(
        &self,
        graph: &CheckedGraph,
        clause: &TemporalClauseNode,
        model: &StateModel,
        types: &TypeEnvironment,
        populations: &[StatePopulation],
        universes: &BTreeMap<filament_checked::key::DeclarationKey, Vec<String>>,
        initial: &[ModelSimState],
        budget: &crate::policy_budget::Budget<'_>,
    ) -> Result<Self, super::letters::LettersPreparationError> {
        // ModelSystem stores a base admission; per-clause overlays never replace it.
        debug_assert!(self.additional.is_empty());
        let mut literals: Vec<ReferenceLiteralKey> = Vec::new();
        temporal_roots(clause, &mut || budget.work(1), &mut |root| {
            for (object_type, identity) in literal_refs(graph, root, budget)? {
                let mut bytes = Vec::new();
                budget.copy(&mut bytes, identity)?;
                let literal = ReferenceLiteralKey {
                    object_type: budget.effective(object_type)?,
                    identity: bytes,
                };
                let mut position = 0;
                let mut duplicate = false;
                while let Some(existing) = literals.get(position) {
                    budget.work(1)?;
                    let mut order =
                        budget.compare_effectives(&existing.object_type, &literal.object_type)?;
                    if order == std::cmp::Ordering::Equal {
                        order = budget.compare_bytes(&existing.identity, &literal.identity)?;
                    }
                    match order {
                        std::cmp::Ordering::Less => position += 1,
                        std::cmp::Ordering::Equal => {
                            duplicate = true;
                            break;
                        }
                        std::cmp::Ordering::Greater => break,
                    }
                }
                if !duplicate {
                    budget.growth(&mut literals, 1)?;
                    budget.work(literals.len() - position)?;
                    literals.insert(position, literal);
                }
            }
            Ok::<(), crate::automaton::PolicyStateError>(())
        })?;
        let mut additional = Vec::new();
        for literal in literals {
            budget.work(1)?;
            // This is the existing resolver, not a second conformance or
            // reference-admission algorithm. Its owner query/construction
            // boundaries still require their caller-budgeted followthrough.
            let binding = resolve_literal(
                &literal,
                &BTreeSet::new(),
                model,
                types,
                populations,
                universes,
                initial,
                Some(budget),
            )?;
            budget.growth(&mut additional, 1)?;
            additional.push((literal, binding));
        }
        budget.work(1)?;
        Ok(Self {
            base: self.base.clone(),
            additional,
        })
    }

    /// Add the literals from a separately checked temporal clause. Analyze
    /// claims are checked apart from the package's base graph, so resolve the
    /// clause's own checked expressions against this subject at admission.
    #[allow(
        clippy::too_many_arguments,
        reason = "each clause form binds its own distinct set of checked parts"
    )]
    pub(super) fn with_temporal_clause(
        &self,
        graph: &CheckedGraph,
        clause: &TemporalClauseNode,
        model: &StateModel,
        types: &TypeEnvironment,
        populations: &[StatePopulation],
        universes: &BTreeMap<filament_checked::key::DeclarationKey, Vec<String>>,
        initial: &[ModelSimState],
    ) -> Result<Self, fsl_semantics::model::observation::AdmissionFailure> {
        let mut literals = BTreeSet::new();
        let result = temporal_roots(
            clause,
            &mut || Ok::<(), std::convert::Infallible>(()),
            &mut |root| {
                collect_literals(graph, root, &mut literals);
                Ok::<(), std::convert::Infallible>(())
            },
        );
        match result {
            Ok(()) => {}
            Err(never) => match never {},
        }
        let mut extended = self.merged();
        extended.extend(resolve_literals(
            literals,
            &BTreeSet::new(),
            model,
            types,
            populations,
            universes,
            initial,
        )?);
        Ok(Self::of_map(extended))
    }
}

fn temporal_roots<'a, E>(
    clause: &'a TemporalClauseNode,
    work: &mut impl FnMut() -> Result<(), E>,
    root: &mut impl FnMut(CheckedNode<'a>) -> Result<(), E>,
) -> Result<(), E> {
    for (index, formula_node) in clause.clause().formula().nodes().iter().enumerate() {
        work()?;
        let atom = FormulaNodeId::from_arena_index(index);
        if matches!(formula_node, FormulaNode::Holds { .. }) {
            if let Some((condition, _)) = clause.condition(atom) {
                root(condition)?;
            }
        }
        if matches!(formula_node, FormulaNode::Aggregate { .. }) {
            if let Some(aggregate) = clause.aggregate_conditions(atom) {
                if let Some((value, _)) = aggregate.value {
                    root(value)?;
                }
                if let Some((filter, _)) = aggregate.filter {
                    root(filter)?;
                }
                if let Some((predicate, _)) = aggregate.predicate {
                    root(predicate)?;
                }
                root(aggregate.threshold.0)?;
            }
        }
    }
    work()?;
    if let Some((guard, _)) = clause.guard() {
        root(guard)?;
    }
    for index in 0..clause.clause().captures().len() {
        work()?;
        if let Some((capture, _)) = clause.capture(index) {
            root(capture)?;
        }
    }
    Ok(())
}

fn literal_refs<'a>(
    graph: &'a CheckedGraph,
    root: CheckedNode<'a>,
    budget: &crate::policy_budget::Budget<'_>,
) -> Result<Vec<(&'a EffectiveId, &'a [u8])>, crate::automaton::PolicyStateError> {
    let mut visited = Vec::new();
    let mut pending = Vec::new();
    budget.growth(&mut pending, 1)?;
    pending.push(root);
    let mut literals = Vec::new();
    while let Some(node) = pending.pop() {
        budget.work(1)?;
        node.try_for_each_child(&mut |child| {
            budget.work(1)?;
            budget.growth(&mut pending, 1)?;
            pending.push(child);
            Ok::<(), crate::automaton::PolicyStateError>(())
        })?;
        if let NodeKind::Call { function, .. } = node.kind() {
            let mut seen = false;
            for previous in &visited {
                budget.work(1)?;
                if previous == function {
                    seen = true;
                    break;
                }
            }
            if !seen {
                budget.growth(&mut visited, 1)?;
                visited.push(*function);
                budget.work(1)?;
                if let Some(function) = graph.function_state(*function) {
                    budget.growth(&mut pending, 1)?;
                    pending.push(function.body);
                }
            }
        }
        if let NodeKind::ReferenceLiteral {
            object_type,
            identity,
        } = node.kind()
        {
            budget.growth(&mut literals, 1)?;
            literals.push((object_type, identity.as_slice()));
        }
    }
    Ok(literals)
}

fn resolve_literals(
    literals: BTreeSet<ReferenceLiteralKey>,
    barred: &BTreeSet<ReferenceLiteralKey>,
    model: &StateModel,
    types: &TypeEnvironment,
    populations: &[StatePopulation],
    universes: &BTreeMap<filament_checked::key::DeclarationKey, Vec<String>>,
    initial: &[ModelSimState],
) -> Result<
    BTreeMap<ReferenceLiteralKey, ReferenceBinding>,
    fsl_semantics::model::observation::AdmissionFailure,
> {
    let mut bindings = BTreeMap::new();
    for literal in literals {
        let binding = resolve_literal(
            &literal,
            barred,
            model,
            types,
            populations,
            universes,
            initial,
            None,
        )
        .map_err(|error| match error {
            super::letters::LettersPreparationError::Admission(
                super::letters::LettersRefusal::ReferenceBindingsAdmission(error),
            ) => error,
            _ => unreachable!("unmetered reference resolution only reports reference admission"),
        })?;
        bindings.insert(literal, binding);
    }
    Ok(bindings)
}

struct Preparation<'b, 'm>(Option<&'b crate::policy_budget::Budget<'m>>);
impl Preparation<'_, '_> {
    fn work_units(&self, units: u64) -> Result<(), crate::automaton::PolicyStateError> {
        self.work(usize::try_from(units).map_err(|_| crate::automaton::PolicyStateError::Invalid)?)
    }
    fn work(&self, units: usize) -> Result<(), crate::automaton::PolicyStateError> {
        self.0.map_or(Ok(()), |budget| budget.work(units))
    }
    fn push<T>(
        &self,
        values: &mut Vec<T>,
        value: T,
    ) -> Result<(), crate::automaton::PolicyStateError> {
        if let Some(budget) = self.0 {
            budget.growth(values, 1)?;
        }
        values.push(value);
        Ok(())
    }
    fn text_equal(
        &self,
        left: &str,
        right: &str,
    ) -> Result<bool, crate::automaton::PolicyStateError> {
        self.0.map_or_else(
            || Ok(left == right),
            |budget| {
                Ok(budget.compare_bytes(left.as_bytes(), right.as_bytes())?
                    == std::cmp::Ordering::Equal)
            },
        )
    }
    fn declaration_equal(
        &self,
        left: &filament_checked::key::DeclarationKey,
        right: &filament_checked::key::DeclarationKey,
    ) -> Result<bool, crate::automaton::PolicyStateError> {
        self.0.map_or_else(
            || Ok(left == right),
            |budget| Ok(budget.compare_declarations(left, right)? == std::cmp::Ordering::Equal),
        )
    }
    fn effective(
        &self,
        value: &EffectiveId,
    ) -> Result<EffectiveId, crate::automaton::PolicyStateError> {
        self.0
            .map_or_else(|| Ok(value.clone()), |budget| budget.effective(value))
    }
    fn text(&self, value: &str) -> Result<String, crate::automaton::PolicyStateError> {
        self.0
            .map_or_else(|| Ok(value.to_owned()), |budget| budget.text(value))
    }
    fn declaration(
        &self,
        value: &filament_checked::key::DeclarationKey,
    ) -> Result<filament_checked::key::DeclarationKey, crate::automaton::PolicyStateError> {
        self.work(1)?;
        Ok(filament_checked::key::DeclarationKey {
            package: self.text(&value.package)?,
            node: self.text(&value.node)?,
        })
    }
    fn conforms(
        &self,
        types: &TypeEnvironment,
        sub: &EffectiveId,
        sup: &EffectiveId,
    ) -> Result<bool, crate::automaton::PolicyStateError> {
        // Internal query visits await the owner's borrowed work hook.
        Ok(types.conforms(self.effective(sub)?, self.effective(sup)?))
    }
    fn overlaps(
        &self,
        model: &StateModel,
        types: &TypeEnvironment,
        population: &StatePopulation,
        parameter: &EffectiveId,
    ) -> Result<bool, crate::automaton::PolicyStateError> {
        for member in &population.member_types {
            self.work(1)?;
            if let Some(member) =
                model.type_identity_with_work(member, &mut |units| self.work_units(units))?
            {
                if self.conforms(types, member, parameter)?
                    || self.conforms(types, parameter, member)?
                {
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }
}

#[allow(
    clippy::too_many_arguments,
    reason = "the resolver reads one immutable subject"
)]
fn resolve_literal(
    literal: &ReferenceLiteralKey,
    barred: &BTreeSet<ReferenceLiteralKey>,
    model: &StateModel,
    types: &TypeEnvironment,
    populations: &[StatePopulation],
    universes: &BTreeMap<filament_checked::key::DeclarationKey, Vec<String>>,
    initial: &[ModelSimState],
    budget: Option<&crate::policy_budget::Budget<'_>>,
) -> Result<ReferenceBinding, super::letters::LettersPreparationError> {
    let prep = Preparation(budget);
    prep.work(literal.identity.len())?;
    let text = std::str::from_utf8(&literal.identity).map_err(|_| {
        super::letters::LettersRefusal::ReferenceBindingsAdmission(super::fault(
            "reference-literal-identity-is-utf8",
        ))
    })?;
    model
        .declaration_key_with_work(&literal.object_type, &mut |units| prep.work_units(units))?
        .ok_or_else(|| {
            super::letters::LettersRefusal::ReferenceBindingsAdmission(super::fault(
                "reference-literal-type-is-selected",
            ))
        })?;
    let mut overlapping = Vec::new();
    for population in populations {
        prep.work(1)?;
        if prep.overlaps(model, types, population, &literal.object_type)? {
            prep.push(&mut overlapping, population)?;
        }
    }
    let mut available = Vec::new();
    for population in &overlapping {
        prep.work(1)?;
        let mut contains = false;
        for (key, keys) in universes {
            prep.work(1)?;
            if prep.declaration_equal(key, &population.key)? {
                for key in keys {
                    prep.work(1)?;
                    if prep.text_equal(key, text)? {
                        contains = true;
                        break;
                    }
                }
                break;
            }
        }
        if contains {
            prep.push(&mut available, *population)?;
        }
    }
    if available.is_empty() {
        return Err(
            super::letters::LettersRefusal::ReferenceBindingsAdmission(super::refused(
                Code::MissingDeclaration,
                "missing-name",
                &[
                    ("type", literal.object_type.original().node().to_owned()),
                    ("key", identity_hex(&literal.identity)),
                ],
            ))
            .into(),
        );
    }

    let mut fixed = None;
    let mut initial_nonconforming = false;
    'states: for state in initial {
        prep.work(1)?;
        for population in &overlapping {
            prep.work(1)?;
            let mut object = None;
            for (key, objects) in &state.state().populations {
                prep.work(1)?;
                if prep.declaration_equal(key, &population.key)? {
                    for (key, candidate) in objects {
                        prep.work(1)?;
                        if prep.text_equal(key, text)? {
                            object = Some(candidate);
                            break;
                        }
                    }
                    break;
                }
            }
            let Some(object) = object else {
                continue;
            };
            let Some(actual_type) = model
                .type_identity_with_work(&object.type_identity, &mut |units| {
                    prep.work_units(units)
                })?
            else {
                return Err(super::letters::LettersRefusal::ReferenceBindingsAdmission(
                    super::fault("reference-literal-member-type-is-selected"),
                )
                .into());
            };
            if prep.conforms(types, actual_type, &literal.object_type)? {
                // The owner's reference construction still needs its actual
                // reservation/copy hooks for a separately checked fixed literal.
                fixed = Some(
                    model
                        .object_reference(&object.type_identity, text)
                        .map_err(|_| {
                            super::letters::LettersRefusal::ReferenceBindingsAdmission(
                                super::fault("reference-literal-fixed-reference"),
                            )
                        })?,
                );
                break 'states;
            } else {
                initial_nonconforming = true;
            }
        }
    }

    let binding = if let Some(reference) = fixed {
        ReferenceBinding::Fixed(reference)
    } else {
        if initial_nonconforming {
            return Err(super::letters::LettersRefusal::ReferenceBindingsAdmission(
                super::refused(
                    Code::MissingDeclaration,
                    "missing-name",
                    &[
                        ("type", literal.object_type.original().node().to_owned()),
                        ("key", identity_hex(&literal.identity)),
                    ],
                ),
            )
            .into());
        }
        prep.work(1)?;
        if barred.contains(literal) {
            // W36's barred-slot refusal is occurrence-specific: a
            // fixed key remains a constant and is admitted above.
            return Err(super::letters::LettersRefusal::ReferenceBindingsAdmission(
                super::refused(
                    Code::IllTyped,
                    "operator-ineligible",
                    &[
                        ("type", literal.object_type.original().node().to_owned()),
                        ("key", identity_hex(&literal.identity)),
                    ],
                ),
            )
            .into());
        }
        // StateModel::populations is declaration-key ordered, so the
        // first overlapping universe gives a stable absent-key locus.
        ReferenceBinding::CreatedObject {
            object_type: prep.effective(&literal.object_type)?,
            identity: prep.text(text)?,
            population: prep.declaration(&available[0].key)?,
            populations: {
                let mut keys = Vec::new();
                for population in &overlapping {
                    prep.work(1)?;
                    let key = prep.declaration(&population.key)?;
                    prep.push(&mut keys, key)?;
                }
                keys
            },
        }
    };
    Ok(binding)
}

pub(super) fn collect_literals(
    graph: &CheckedGraph,
    root: CheckedNode<'_>,
    literals: &mut BTreeSet<ReferenceLiteralKey>,
) {
    let mut visited_functions = BTreeSet::new();
    let mut pending = vec![root];
    while let Some(node) = pending.pop() {
        pending.extend(node.children());
        if let NodeKind::Call { function, .. } = node.kind() {
            if visited_functions.insert(*function) {
                if let Some(function) = graph.function_state(*function) {
                    pending.push(function.body);
                }
            }
        }
        if let NodeKind::ReferenceLiteral {
            object_type,
            identity,
        } = node.kind()
        {
            literals.insert(ReferenceLiteralKey {
                object_type: object_type.clone(),
                identity: identity.clone(),
            });
        }
    }
}

fn identity_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut text = String::with_capacity(bytes.len().saturating_mul(2));
    for byte in bytes {
        text.push(char::from(HEX[usize::from(byte >> 4)]));
        text.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    text
}
