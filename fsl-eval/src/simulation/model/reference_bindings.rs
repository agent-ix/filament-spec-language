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
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ReferenceBindings(BTreeMap<ReferenceLiteralKey, ReferenceBinding>);

impl ReferenceBindings {
    pub(crate) fn created_for_footprints(
        &self,
    ) -> BTreeMap<(EffectiveId, Vec<u8>), Vec<filament_checked::key::DeclarationKey>> {
        self.0
            .iter()
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
        self.0.get(&ReferenceLiteralKey {
            object_type: object_type.clone(),
            identity: identity.to_vec(),
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
        let mut extended = self.0.clone();
        extended.extend(resolve_literals(
            literals,
            &BTreeSet::new(),
            model,
            types,
            populations,
            universes,
            initial,
        )?);
        Ok(Self(extended))
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
        Ok(Self(resolve_literals(
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
        let mut extended = self.0.clone();
        extended.extend(resolve_literals(
            literals,
            &barred,
            model,
            types,
            populations,
            universes,
            initial,
        )?);
        Ok(Self(extended))
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
        let mut extended = self.0.clone();
        extended.extend(resolve_literals(
            literals,
            &BTreeSet::new(),
            model,
            types,
            populations,
            universes,
            initial,
        )?);
        Ok(Self(extended))
    }
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
        let text = std::str::from_utf8(&literal.identity)
            .map_err(|_| super::fault("reference-literal-identity-is-utf8"))?;
        model
            .type_key(literal.object_type.clone())
            .ok_or_else(|| super::fault("reference-literal-type-is-selected"))?;
        let overlapping: Vec<&StatePopulation> = populations
            .iter()
            .filter(|population| overlaps(model, types, population, &literal.object_type))
            .collect();
        let available: Vec<&StatePopulation> = overlapping
            .iter()
            .copied()
            .filter(|population| {
                universes
                    .get(&population.key)
                    .is_some_and(|keys| keys.iter().any(|key| key == text))
            })
            .collect();
        if available.is_empty() {
            return Err(super::refused(
                Code::MissingDeclaration,
                "missing-name",
                &[
                    ("type", literal.object_type.original().node().to_owned()),
                    ("key", identity_hex(&literal.identity)),
                ],
            ));
        }

        let mut fixed = None;
        let mut initial_nonconforming = false;
        'states: for state in initial {
            for population in &overlapping {
                let Some(object) = state
                    .state()
                    .populations
                    .get(&population.key)
                    .and_then(|objects| objects.get(text))
                else {
                    continue;
                };
                let Some(actual_type) = model.type_identity(&object.type_identity) else {
                    return Err(super::fault("reference-literal-member-type-is-selected"));
                };
                if types.conforms(actual_type, literal.object_type.clone()) {
                    fixed = Some(
                        model
                            .object_reference(&object.type_identity, text)
                            .map_err(|_| super::fault("reference-literal-fixed-reference"))?,
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
                return Err(super::refused(
                    Code::MissingDeclaration,
                    "missing-name",
                    &[
                        ("type", literal.object_type.original().node().to_owned()),
                        ("key", identity_hex(&literal.identity)),
                    ],
                ));
            }
            if barred.contains(&literal) {
                // W36's barred-slot refusal is occurrence-specific: a
                // fixed key remains a constant and is admitted above.
                return Err(super::refused(
                    Code::IllTyped,
                    "operator-ineligible",
                    &[
                        ("type", literal.object_type.original().node().to_owned()),
                        ("key", identity_hex(&literal.identity)),
                    ],
                ));
            }
            // StateModel::populations is declaration-key ordered, so the
            // first overlapping universe gives a stable absent-key locus.
            ReferenceBinding::CreatedObject {
                object_type: literal.object_type.clone(),
                identity: text.to_owned(),
                population: available[0].key.clone(),
                populations: overlapping
                    .iter()
                    .map(|population| population.key.clone())
                    .collect(),
            }
        };
        bindings.insert(literal, binding);
    }
    Ok(bindings)
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

fn overlaps(
    model: &StateModel,
    types: &TypeEnvironment,
    population: &StatePopulation,
    object_type: &EffectiveId,
) -> bool {
    population.member_types.iter().any(|member| {
        model.type_identity(member).is_some_and(|member| {
            types.conforms(member.clone(), object_type.clone())
                || types.conforms(object_type.clone(), member)
        })
    })
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
