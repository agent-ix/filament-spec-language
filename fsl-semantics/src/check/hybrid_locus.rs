// SPDX-License-Identifier: AGPL-3.0-or-later
//! FSpec FR-481, FR-750: the emitted term node a checked hybrid expression's
//! term path names, as the FSpec FR-331 `Locus` (node and occurrence) a
//! provider's result and an undefined-evaluation counterexample cite.
//!
//! A [`HybridTerm`](super::HybridTerm) carries no node identity, and the
//! lowering keeps no table from a term to its node, so the node is found the
//! way a process provider finds it, by reading the emitted nodes: the
//! construct's `hybrid_model`, `hybrid_mode`, `hybrid_transition` or
//! `temporal_clause` node, the root term its body names, then each child of
//! the term path through its application's operands.

use filament_exact::{NodeKey, Origin};

use super::family::OccurrenceRole;
use super::lowering::{hybrid_location, HybridSite};
use super::node_key::{
    BindingValue, BodyTerm, GroupMember, LeafTerm, MemberTerm, NodeRef, TupleMember,
};
use super::CheckedGraph;

/// The expression of a checked hybrid model or claim a term path starts at.
/// Each position is the one the checked model or claim holds it at, and
/// `model` and `claim` index [`CheckedGraph::hybrid_models`] and
/// [`CheckedGraph::hybrid_claims`].
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum HybridTermHome {
    /// The condition of `initial[entry]` of a model.
    Initial {
        /// The model's position.
        model: usize,
        /// The entry's position.
        entry: usize,
    },
    /// A mode's invariant.
    Invariant {
        /// The model's position.
        model: usize,
        /// The mode's position.
        mode: usize,
    },
    /// A mode's rate of a variable.
    Rate {
        /// The model's position.
        model: usize,
        /// The mode's position.
        mode: usize,
        /// The variable's position.
        variable: usize,
    },
    /// A transition's guard.
    Guard {
        /// The model's position.
        model: usize,
        /// The source mode's position.
        mode: usize,
        /// The transition's position in the mode.
        transition: usize,
    },
    /// A transition's reset constraint value.
    Reset {
        /// The model's position.
        model: usize,
        /// The source mode's position.
        mode: usize,
        /// The transition's position in the mode.
        transition: usize,
        /// The constraint's position in the reset.
        constraint: usize,
    },
    /// The claim's `on each ... when` guard.
    Activation {
        /// The claim's position.
        claim: usize,
    },
    /// A capture's initializer.
    Capture {
        /// The claim's position.
        claim: usize,
        /// The capture's position.
        capture: usize,
    },
    /// A `holds(...)` atom of the claim's formula, by its position among the
    /// formula's `holds` leaves in pre-order.
    ClaimAtom {
        /// The claim's position.
        claim: usize,
        /// The atom's position.
        atom: usize,
    },
}

/// A term node of the emitted package: the FSpec FR-331 `Locus` of a term.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HybridTermNode {
    /// The term's node.
    pub node: NodeKey,
    /// The term's `expression` occurrence at its construct.
    pub origin: Origin,
}

const CALL: &str = "filament.op.function.call";

impl CheckedGraph {
    /// The emitted node `path` names under `home`, with its `expression`
    /// occurrence at the construct, or `None` when the model or claim has no
    /// such construct or term. `path` is a term's child positions from the
    /// expression's root: 0 for `Negate` and `Not`; 0 and 1 for `Arithmetic`,
    /// `Compare` and `Logic`; 0 to 2 for `Conditional`; the argument's
    /// position for `Call`. A position inside a called function's body names
    /// the `Call` node.
    pub fn hybrid_term_node(&self, home: HybridTermHome, path: &[usize]) -> Option<HybridTermNode> {
        let (root, site) = self.hybrid_term_root(home)?;
        let mut node = root;
        for &position in path {
            node = self.operand(&node, position)?;
        }
        let location = hybrid_location(site);
        let expression = OccurrenceRole::Expression.kernel_role();
        let origin = self
            .occurrences_at(&location)
            .iter()
            .find(|(key, origin)| *key == node && *origin.role() == expression)
            .map(|(_, origin)| origin.clone())?;
        Some(HybridTermNode { node, origin })
    }

    /// A term path from `home`'s root to the node `target`, the inverse of
    /// [`Self::hybrid_term_node`]: the first path in operand order, or `None`
    /// when no term of that expression is `target`.
    pub fn hybrid_term_path(&self, home: HybridTermHome, target: &NodeKey) -> Option<Vec<usize>> {
        let (root, _) = self.hybrid_term_root(home)?;
        let mut path = Vec::new();
        self.find_term(&root, target, &mut path).then_some(path)
    }

    fn find_term(&self, node: &NodeKey, target: &NodeKey, path: &mut Vec<usize>) -> bool {
        if node == target {
            return true;
        }
        let mut position = 0;
        while let Some(operand) = self.operand(node, position) {
            path.push(position);
            if self.find_term(&operand, target, path) {
                return true;
            }
            path.pop();
            position += 1;
        }
        false
    }

    fn body_of(&self, key: &NodeKey) -> Option<&BodyTerm> {
        self.semantic_graph()
            .node(key.clone())
            .map(|node| node.body())
    }

    /// The node at `site` whose `semantic_form` is `form`.
    fn construct_node(&self, site: HybridSite, form: &str) -> Option<NodeKey> {
        self.occurrences_at(&hybrid_location(site))
            .iter()
            .find(|(key, _)| {
                self.semantic_graph()
                    .node(key.clone())
                    .is_some_and(|node| node.semantic_form() == form)
            })
            .map(|(key, _)| key.clone())
    }

    fn operand(&self, key: &NodeKey, position: usize) -> Option<NodeKey> {
        let BodyTerm::Application(application) = self.body_of(key)? else {
            return None;
        };
        let offset = usize::from(application.operation.identity == CALL);
        match application.arguments.get(position + offset)? {
            MemberTerm::Leaf(leaf) => leaf_key(leaf),
            _ => None,
        }
    }

    fn hybrid_term_root(&self, home: HybridTermHome) -> Option<(NodeKey, HybridSite)> {
        match home {
            HybridTermHome::Initial { model, entry } => {
                let site = HybridSite::Model(model);
                let node = self.construct_node(site, "hybrid_model")?;
                let initial = binding(self.body_of(&node)?, "initial")?;
                let BindingValue::Tuple(tuple) = initial else {
                    return None;
                };
                let TupleMember::Group(group) = tuple.members.get(entry)? else {
                    return None;
                };
                Some((group_binding(&group.members, "condition")?, site))
            }
            HybridTermHome::Invariant { model, mode } => {
                let site = HybridSite::Mode(model, mode);
                let node = self.construct_node(site, "hybrid_mode")?;
                Some((leaf_binding(self.body_of(&node)?, "invariant")?, site))
            }
            HybridTermHome::Rate {
                model,
                mode,
                variable,
            } => {
                let site = HybridSite::Mode(model, mode);
                let node = self.construct_node(site, "hybrid_mode")?;
                let BindingValue::Tuple(rates) = binding(self.body_of(&node)?, "rates")? else {
                    return None;
                };
                let TupleMember::Leaf(rate) = rates.members.get(variable)? else {
                    return None;
                };
                Some((leaf_key(rate)?, site))
            }
            HybridTermHome::Guard {
                model,
                mode,
                transition,
            } => {
                let site = HybridSite::Transition(model, mode, transition);
                let node = self.construct_node(site, "hybrid_transition")?;
                Some((leaf_binding(self.body_of(&node)?, "guard")?, site))
            }
            HybridTermHome::Reset {
                model,
                mode,
                transition,
                constraint,
            } => {
                let site = HybridSite::Transition(model, mode, transition);
                let node = self.construct_node(site, "hybrid_transition")?;
                let BindingValue::Tuple(resets) = binding(self.body_of(&node)?, "resets")? else {
                    return None;
                };
                let TupleMember::Group(reset) = resets.members.get(constraint)? else {
                    return None;
                };
                Some((group_binding(&reset.members, "value")?, site))
            }
            HybridTermHome::Activation { claim } => {
                let (node, site) = self.claim_node(claim)?;
                let MemberTerm::Group(group) = self.claim_argument(&node, 2)? else {
                    return None;
                };
                Some((group_binding(&group.members, "guard")?, site))
            }
            HybridTermHome::Capture { claim, capture } => {
                let (node, site) = self.claim_node(claim)?;
                let MemberTerm::Group(group) = self.claim_argument(&node, 3)? else {
                    return None;
                };
                let GroupMember::Binding(bound) = group.members.get(capture)? else {
                    return None;
                };
                Some((leaf_key(&bound.value)?, site))
            }
            HybridTermHome::ClaimAtom { claim, atom } => {
                let (node, site) = self.claim_node(claim)?;
                let MemberTerm::Leaf(formula) = self.claim_argument(&node, 5)? else {
                    return None;
                };
                Some((self.nth_atom_term(leaf_key(formula)?, atom)?, site))
            }
        }
    }

    fn claim_node(&self, claim: usize) -> Option<(NodeKey, HybridSite)> {
        let identity = self.hybrid_claim_sites().get(claim)?.identity.clone();
        Some((identity, HybridSite::Claim(claim)))
    }

    fn claim_argument(&self, node: &NodeKey, position: usize) -> Option<&MemberTerm> {
        match self.body_of(node)? {
            BodyTerm::Application(application) => application.arguments.get(position),
            _ => None,
        }
    }

    /// The term of the `atom`-th `holds` leaf under `formula`, in pre-order.
    /// A `true` or `false` leaf counts as an atom and holds no term.
    fn nth_atom_term(&self, formula: NodeKey, atom: usize) -> Option<NodeKey> {
        let mut pending = vec![formula];
        let mut seen = 0_usize;
        while let Some(key) = pending.pop() {
            let BodyTerm::Application(application) = self.body_of(&key)? else {
                return None;
            };
            let identity = application.operation.identity.as_str();
            if matches!(
                identity,
                "filament.op.temporal.holds"
                    | "filament.op.temporal.true"
                    | "filament.op.temporal.false"
            ) {
                if seen == atom {
                    return match application.arguments.first() {
                        Some(MemberTerm::Leaf(leaf)) => leaf_key(leaf),
                        _ => None,
                    };
                }
                seen += 1;
                continue;
            }
            for operand in application.arguments.iter().rev() {
                if let MemberTerm::Leaf(leaf) = operand {
                    pending.push(leaf_key(leaf)?);
                }
            }
        }
        None
    }
}

fn leaf_key(leaf: &LeafTerm) -> Option<NodeKey> {
    match leaf {
        LeafTerm::Reference {
            target: NodeRef::Admitted(key),
        } => Some(key.clone()),
        _ => None,
    }
}

fn binding<'a>(body: &'a BodyTerm, name: &str) -> Option<&'a BindingValue> {
    let BodyTerm::Aggregate(aggregate) = body else {
        return None;
    };
    aggregate.members.iter().find_map(|member| match member {
        MemberTerm::Binding(bound) if bound.name == name => Some(&bound.value),
        _ => None,
    })
}

fn leaf_binding(body: &BodyTerm, name: &str) -> Option<NodeKey> {
    match binding(body, name)? {
        BindingValue::Leaf(leaf) => leaf_key(leaf),
        _ => None,
    }
}

fn group_binding(members: &[GroupMember], name: &str) -> Option<NodeKey> {
    members.iter().find_map(|member| match member {
        GroupMember::Binding(bound) if bound.name == name => leaf_key(&bound.value),
        _ => None,
    })
}
