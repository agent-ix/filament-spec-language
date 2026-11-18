// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-1000: resolve subject-independent reference literals at FR-120 subject
//! admission, without changing the checked graph.

use std::collections::{BTreeMap, BTreeSet};

use filament_checked::node::NodeKind;
use filament_exact::{EffectiveId, ObjectReference};
use filament_semantic_value::declaration::TypeEnvironment;
use fsl_foundation::diagnostic::Code;
use fsl_semantics::check::{
    CheckedGraph, CheckedLifetime, CheckedNode, FormulaNode, FormulaNodeId,
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
        population: filament_checked::key::DeclarationKey,
    },
}

/// One immutable subject's reference-literal decisions.
#[derive(Clone, Debug, Default)]
pub(crate) struct ReferenceBindings(BTreeMap<ReferenceLiteralKey, ReferenceBinding>);

impl ReferenceBindings {
    pub(crate) fn created_for_footprints(
        &self,
    ) -> BTreeMap<(EffectiveId, Vec<u8>), filament_checked::key::DeclarationKey> {
        self.0
            .iter()
            .filter_map(|(literal, binding)| match binding {
                ReferenceBinding::CreatedObject { population, .. } => Some((
                    (literal.object_type.clone(), literal.identity.clone()),
                    population.clone(),
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
            collect_literals(clause.body(), &mut literals);
        }
        for clause in graph.temporal_clauses() {
            for (index, formula_node) in clause.clause().formula().nodes().iter().enumerate() {
                let atom = FormulaNodeId::from_arena_index(index);
                if matches!(formula_node, FormulaNode::Holds { .. }) {
                    if let Some((condition, _)) = clause.condition(atom) {
                        collect_literals(condition, &mut literals);
                    }
                }
                if matches!(formula_node, FormulaNode::Aggregate { .. }) {
                    if let Some(aggregate) = clause.aggregate_conditions(atom) {
                        if let Some((value, _)) = aggregate.value {
                            collect_literals(value, &mut literals);
                        }
                        if let Some((filter, _)) = aggregate.filter {
                            collect_literals(filter, &mut literals);
                        }
                        if let Some((predicate, _)) = aggregate.predicate {
                            collect_literals(predicate, &mut literals);
                        }
                        collect_literals(aggregate.threshold.0, &mut literals);
                    }
                }
            }
            if let Some((guard, _)) = clause.guard() {
                collect_literals(guard, &mut literals);
            }
            for index in 0..clause.clause().captures().len() {
                if let Some((capture, _)) = clause.capture(index) {
                    collect_literals(capture, &mut literals);
                }
            }
        }
        for protocol in graph.protocols() {
            let slots = &protocol.slots;
            for body in &slots.bodies {
                collect_literals(body.body.root(), &mut literals);
                if protocol
                    .controls
                    .control(body.node)
                    .is_some_and(|control| control.kind == fsl_forms::ProtocolNodeKind::Check)
                {
                    collect_literals(body.body.root(), &mut barred);
                }
            }
            for choice in &slots.choices {
                for visible in &choice.visible {
                    collect_literals(visible.root(), &mut literals);
                    collect_literals(visible.root(), &mut barred);
                }
                for case in &choice.cases {
                    collect_literals(case.guard.root(), &mut literals);
                    collect_literals(case.guard.root(), &mut barred);
                }
            }
            for repeat in &slots.repeats {
                collect_literals(repeat.guard.root(), &mut literals);
                collect_literals(repeat.guard.root(), &mut barred);
                for visible in &repeat.visible {
                    collect_literals(visible.root(), &mut literals);
                    collect_literals(visible.root(), &mut barred);
                }
                if let Some(proof) = &repeat.loop_proof {
                    collect_literals(proof.invariant.root(), &mut literals);
                    collect_literals(proof.variant.root(), &mut literals);
                }
            }
            for role in &slots.roles {
                if let CheckedLifetime::Until { predicate, .. } = &role.lifetime {
                    collect_literals(predicate.root(), &mut literals);
                }
            }
            for related in &slots.related {
                for end in &related.ends {
                    collect_literals(end.root(), &mut literals);
                }
            }
        }

        let mut bindings = BTreeMap::new();
        for literal in literals {
            let text = std::str::from_utf8(&literal.identity)
                .map_err(|_| super::fault("reference-literal-identity-is-utf8"))?;
            let type_key = model
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
                    &[("key", identity_hex(&literal.identity))],
                ));
            }

            let mut fixed = None;
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
                    let Some(actual_type) = model.type_identity(object.type_identity.clone())
                    else {
                        return Err(super::fault("reference-literal-member-type-is-selected"));
                    };
                    if types.conforms(actual_type, literal.object_type.clone()) {
                        fixed = Some(
                            model
                                .object_reference(type_key, text)
                                .map_err(|_| super::fault("reference-literal-fixed-reference"))?,
                        );
                        break 'states;
                    }
                }
            }

            let binding = if let Some(reference) = fixed {
                ReferenceBinding::Fixed(reference)
            } else {
                if barred.contains(&literal) {
                    // W36's barred-slot refusal is occurrence-specific: a
                    // fixed key remains a constant and is admitted above.
                    return Err(super::refused(
                        Code::IllTyped,
                        "operator-ineligible",
                        &[("key", identity_hex(&literal.identity))],
                    ));
                }
                // StateModel::populations is declaration-key ordered, so the
                // first overlapping universe gives a stable absent-key locus.
                ReferenceBinding::CreatedObject {
                    object_type: literal.object_type.clone(),
                    identity: text.to_owned(),
                    population: available[0].key.clone(),
                }
            };
            bindings.insert(literal, binding);
        }
        Ok(Self(bindings))
    }
}

fn collect_literals(root: CheckedNode<'_>, literals: &mut BTreeSet<ReferenceLiteralKey>) {
    let mut pending = vec![root];
    while let Some(node) = pending.pop() {
        pending.extend(node.children());
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
        model.type_identity(member.clone()).is_some_and(|member| {
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
