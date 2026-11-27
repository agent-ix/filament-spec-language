// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-155, FR-940: the footprint derivations over any tree of nodes a read
//! derivation can walk, so the package route reads the locations the source
//! route reads from the one derivation.

use std::collections::BTreeSet;

use filament_exact::{EffectiveId, NodeKey, ValueType};
use filament_semantic_value::declaration::TypeEnvironment;
use filament_semantic_value::location::ExpressionLocation as Location;

use super::{
    constraint_reads_of, descendants, BodyScope, Derivation, DerivationClause, FootprintObject,
    FootprintRefusal, Observed, Reader, ReferenceLiterals, SymbolicFootprint, SymbolicLocation,
};
use crate::check::{CheckedGraph, CheckedNode};
use crate::model::clause_observation::Observation;
use crate::model::state::StateModel;
use filament_checked::node::NodeKind;

/// A node of an expression tree a body read derivation walks: the checked
/// body of a source clause, or the reader projection of a package node
/// (FR-940 "Package forms of the source-checked evaluators"). One derivation
/// serves both, so a package clause reads the locations its source clause
/// reads.
pub trait ReadNode<'a>: Copy + 'a {
    /// What the node names an operand by.
    type Child: Copy + 'a;
    /// How a literal node holds its value.
    type Literal: 'a;

    /// What the node computes.
    fn kind(self) -> &'a NodeKind<Self::Child, Self::Literal>;

    /// The operand `child` names.
    fn at(self, child: Self::Child) -> Self;

    /// The node's checked static type.
    fn value_type(self) -> &'a ValueType;

    /// The node's location, where its source holds one.
    fn location(self) -> Option<&'a Location>;

    /// The node's direct operands in evaluation order.
    fn children(self) -> Vec<Self>;

    /// The body of function `function` that a call at this node calls: out
    /// of `graph` for a checked body, out of the projection for a package
    /// node, which ignores `graph`.
    fn callee(self, graph: Option<&'a CheckedGraph>, function: usize) -> Option<Self>;
}

impl<'a> ReadNode<'a> for CheckedNode<'a> {
    type Child = super::super::ir::NodeId;
    type Literal = filament_checked::node::CheckedLiteral;

    fn kind(self) -> &'a NodeKind<Self::Child, Self::Literal> {
        CheckedNode::kind(self)
    }

    fn at(self, child: Self::Child) -> Self {
        CheckedNode::at(self, child)
    }

    fn value_type(self) -> &'a ValueType {
        CheckedNode::value_type(self)
    }

    fn location(self) -> Option<&'a Location> {
        Some(CheckedNode::location(self))
    }

    fn children(self) -> Vec<Self> {
        CheckedNode::children(self)
    }

    fn callee(self, graph: Option<&'a CheckedGraph>, function: usize) -> Option<Self> {
        graph?.function_state(function).map(|state| state.body)
    }
}

/// One state clause body a footprint derivation reads, as the package route
/// projects it.
pub struct ClauseBody<'a, N> {
    /// The clause's declared name.
    pub name: &'a str,
    /// The clause node's key.
    pub identity: NodeKey,
    /// The object type the clause constrains.
    pub context: EffectiveId,
    /// The declaring type and name of the operation a precondition or
    /// postcondition is anchored on.
    pub operation: Option<(EffectiveId, String)>,
    /// Whether slot 1 is `result`.
    pub binds_result: bool,
    /// `self`, `result` when bound, then the operation's parameters.
    pub parameters: Vec<(String, ValueType)>,
    /// The observation the clause's own reads take.
    pub observation: Observation,
    /// The locations the body reads under `pre(..)`.
    pub pre_reads: BTreeSet<Location>,
    /// The body's root.
    pub root: N,
}

impl<N> ClauseBody<'_, N> {
    fn scope(&self) -> BodyScope<'_> {
        BodyScope {
            name: self.name,
            receivers: &[0],
            binds_result: self.binds_result,
            parameters: &self.parameters,
            observed: Observed::Pre(&self.pre_reads),
            observation: self.observation,
        }
    }
}

/// The operation a package derivation reads: its receiver's type, its
/// declaring type, its name and parameters, and the object type of each bound
/// reference argument.
pub struct PackageOperation<'a> {
    /// The receiver's object type.
    pub receiver_type: EffectiveId,
    /// The declaring type.
    pub declaring: EffectiveId,
    /// The operation's declaration.
    pub operation: &'a crate::model::operation::OperationDeclaration,
    /// The object type of each bound reference argument.
    pub arguments: &'a [(String, EffectiveId)],
}

/// The footprint of `operation` over clause bodies of any [`ReadNode`]
/// source with the subject-bound created reference literals: the package
/// route's derivation from its own projected clauses (FR-940), equal to the
/// source route's over the same clauses.
///
/// # Errors
///
/// [`FootprintRefusal::UnsupportedForm`] when a clause reads through a form
/// FSpec FR-383 gives no rule for, and [`FootprintRefusal::Unresolved`] when
/// the model does not declare a type the derivation needs.
pub fn derive_selected_footprint_over<'a, N: ReadNode<'a>>(
    types: &'a TypeEnvironment,
    model: &'a StateModel,
    clauses: &'a [ClauseBody<'a, N>],
    reference_literals: &'a ReferenceLiterals,
    operation: PackageOperation<'a>,
    select: &'a dyn Fn(NodeKey) -> bool,
) -> Result<SymbolicFootprint, FootprintRefusal> {
    let reader = Reader::over(types, model, None, Some(reference_literals))?;
    let clauses = clauses
        .iter()
        .map(|clause| DerivationClause {
            identity: clause.identity.clone(),
            owner: clause
                .operation
                .as_ref()
                .map(|(declaring, name)| (declaring.clone(), name.clone())),
            scope: clause.scope(),
            root: clause.root,
        })
        .collect();
    let PackageOperation {
        receiver_type,
        declaring,
        operation,
        arguments,
    } = operation;
    Derivation {
        reader,
        clauses,
        receiver_type,
        declaring,
        operation,
        arguments,
        select,
    }
    .run()
}

/// [`super::derive_constraint_reads`] over a clause body of any [`ReadNode`]
/// source.
///
/// # Errors
///
/// As [`super::derive_atom_reads`].
pub fn derive_constraint_reads_over<'a, N: ReadNode<'a>>(
    types: &'a TypeEnvironment,
    model: &'a StateModel,
    clause: &ClauseBody<'a, N>,
) -> Result<BTreeSet<SymbolicLocation>, FootprintRefusal> {
    let reader = Reader::new(types, model)?;
    constraint_reads_of(&reader, &clause.scope(), clause.root, &clause.context)
}

fn unobserved<'c>(name: &'c str, parameters: &'c [(String, ValueType)]) -> BodyScope<'c> {
    BodyScope {
        name,
        receivers: &[0],
        binds_result: false,
        parameters,
        observed: Observed::None,
        observation: Observation::Current,
    }
}

/// [`super::derive_slot_reads`] over a body of any [`ReadNode`] source.
///
/// # Errors
///
/// As [`super::derive_atom_reads`].
pub fn derive_slot_reads_over<'a, N: ReadNode<'a>>(
    types: &'a TypeEnvironment,
    model: &'a StateModel,
    name: &str,
    parameters: &[(String, ValueType)],
    body: N,
) -> Result<BTreeSet<SymbolicLocation>, FootprintRefusal> {
    let reader = Reader::new(types, model)?;
    Ok(reader
        .body_reads(&unobserved(name, parameters), body)?
        .into_iter()
        .map(|(location, _)| match location {
            SymbolicLocation::Field { field, .. } => SymbolicLocation::AnyField { field },
            other => other,
        })
        .collect())
}

/// [`super::derive_node_reads`] over a body of any [`ReadNode`] source,
/// with the actual receiver slots of that source's binder frame. A compact
/// package frame may place its `over` binder at any slot, or omit it.
///
/// # Errors
///
/// As [`super::derive_atom_reads`].
pub fn derive_node_reads_over<'a, N: ReadNode<'a>>(
    types: &'a TypeEnvironment,
    model: &'a StateModel,
    name: &str,
    parameters: &[(String, ValueType)],
    receivers: &[usize],
    body: N,
) -> Result<BTreeSet<SymbolicLocation>, FootprintRefusal> {
    let reader = Reader::new(types, model)?;
    let scope = BodyScope {
        receivers,
        ..unobserved(name, parameters)
    };
    Ok(reader
        .body_reads(&scope, body)?
        .into_iter()
        .map(|(location, _)| match location {
            SymbolicLocation::Field {
                object: FootprintObject::Param(_),
                field,
            } => SymbolicLocation::AnyField { field },
            other => other,
        })
        .collect())
}

/// [`super::derive_guard_reads`] over a body of any [`ReadNode`] source.
///
/// # Errors
///
/// As [`super::derive_atom_reads`].
pub fn derive_guard_reads_over<'a, N: ReadNode<'a>>(
    types: &'a TypeEnvironment,
    model: &'a StateModel,
    parameters: &[(String, ValueType)],
    receivers: &[usize],
    condition: N,
) -> Result<BTreeSet<SymbolicLocation>, FootprintRefusal> {
    let reader = Reader::new(types, model)?;
    let alias = |object: FootprintObject| match object {
        FootprintObject::Param(_) => FootprintObject::Receiver,
        receiver @ FootprintObject::Receiver => receiver,
    };
    Ok(reader
        .body_reads(
            &BodyScope {
                receivers,
                ..unobserved("guard", parameters)
            },
            condition,
        )?
        .into_iter()
        .map(|(location, _)| match location {
            SymbolicLocation::Field { object, field } => SymbolicLocation::Field {
                object: alias(object),
                field,
            },
            SymbolicLocation::Membership { population, object } => SymbolicLocation::Membership {
                population,
                object: alias(object),
            },
            other => other,
        })
        .collect())
}

/// The locations `root` reads under `pre(..)`, or through a reference obtained
/// there (FR-104), in a clause whose own observation is `clause` and whose
/// slots below `roots` are its `self`, `result` and operation parameters.
pub fn pre_observed_locations<'a, N: ReadNode<'a>>(
    root: N,
    clause: Observation,
    roots: usize,
) -> BTreeSet<Location> {
    crate::check::observation::observed_reads(root, clause, roots)
        .into_iter()
        .filter(|(_, observation)| *observation == Observation::Pre)
        .filter_map(|(node, _)| node.location().cloned())
        .collect()
}

/// The reference literals a body of any [`ReadNode`] source names, each as
/// the (object type, identity) it is bound by.
pub fn reference_literals_of<'a, N: ReadNode<'a>>(root: N) -> Vec<(EffectiveId, Vec<u8>)> {
    std::iter::once(root)
        .chain(descendants(root))
        .filter_map(|node| match node.kind() {
            NodeKind::ReferenceLiteral {
                object_type,
                identity,
            } => Some((object_type.clone(), identity.clone())),
            _ => None,
        })
        .collect()
}
