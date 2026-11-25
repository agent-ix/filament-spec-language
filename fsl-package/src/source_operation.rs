// SPDX-License-Identifier: AGPL-3.0-or-later
//! The operations a unit's own `object` declarations declare, and the
//! selection of such an operation's clause function (FR-390, FR-531). The
//! emitter's source-operation identity rows and the source route's selector
//! read the operations here, and the source and bytes routes share the rule
//! that names a clause function.

use crate::protocol_selection::SlotSelectionFailure;
use crate::CheckedPackage;
use alloc::{format, string::String};
use filament_checked::key::DeclarationKey;
use filament_exact::NodeKey;
use filament_semantic_value::catalog::InternalFault;
use fsl_semantics::check::{
    CheckedGraph, FunctionState, NodeTag, Owner, SemanticNode, SourceOwner,
};
use fsl_semantics::model::domain_package::DomainPackageRecord;
use ix_trace_rs::implements;

/// An authored source operation contract clause selected by its checked
/// operation anchor, never by a synthesized function name.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SourceOperationClause {
    /// The operation's precondition clause function.
    Precondition {
        /// Authored clause identity in this operation's checked declaration.
        name: String,
    },
    /// The operation's postcondition clause function.
    Postcondition {
        /// Authored clause identity in this operation's checked declaration.
        name: String,
    },
}

impl SourceOperationClause {
    const fn declaration_kind(&self) -> &'static str {
        match self {
            Self::Precondition { .. } => "precondition",
            Self::Postcondition { .. } => "postcondition",
        }
    }

    fn authored_name(&self) -> &str {
        match self {
            Self::Precondition { name } | Self::Postcondition { name } => name,
        }
    }

    /// Whether `declaration`, a clause function's FR-569 declaration name
    /// `[<type>, <operation>, <kind>, <label>]`, declares this clause of the
    /// operation `operation_name` of the source type `type_name`.
    #[ix_trace_rs::string_edge]
    #[implements("FR-531-AC-40")]
    pub(crate) fn is_declared_by<'a>(
        &self,
        type_name: &str,
        operation_name: &str,
        declaration: impl IntoIterator<Item = &'a str>,
    ) -> bool {
        declaration.into_iter().eq([
            type_name,
            operation_name,
            self.declaration_kind(),
            self.authored_name(),
        ])
    }
}

/// A checked operation anchor whose operation a unit's own `object`
/// declaration declares: the operations that have a source-operation identity
/// row (FR-390).
pub(crate) struct DeclaredSourceOperation<'g> {
    /// The `state`/`operation_anchor` node.
    pub(crate) anchor: NodeKey,
    /// The declaring source type's name.
    pub(crate) type_name: &'g str,
    /// The operation's name.
    pub(crate) operation_name: &'g str,
    object: &'g DeclarationKey,
}

impl<'g> DeclaredSourceOperation<'g> {
    /// The owner of the unit that declares the operation's type, or `None`
    /// when the graph holds no source-owned node of that type.
    pub(crate) fn owner(&self, graph: &'g CheckedGraph) -> Option<&'g SourceOwner> {
        match source_node_for_declaration(graph, self.object)?.owner() {
            Some(Owner::Source(owner)) => Some(owner),
            _ => None,
        }
    }
}

/// Every [`DeclaredSourceOperation`] of `graph`, in source package record
/// order.
#[ix_trace_rs::string_edge]
#[implements("FR-390-AC-70")]
pub(crate) fn declared_source_operations(
    graph: &CheckedGraph,
) -> impl Iterator<Item = DeclaredSourceOperation<'_>> {
    graph
        .source_packages()
        .iter()
        .flat_map(|package| &package.records)
        .filter_map(move |record| {
            let DomainPackageRecord::OperationMember(operation) = record else {
                return None;
            };
            let type_name =
                source_identity_segment(&operation.owner.package, &operation.owner.node)?;
            let operation_name = operation
                .key
                .node
                .strip_prefix(&format!("{}/", operation.owner.node))
                .filter(|name| !name.is_empty() && !name.contains('/'))?;
            let anchor = graph.source_operation_anchor(&operation.owner, operation_name)?;
            Some(DeclaredSourceOperation {
                anchor,
                type_name,
                operation_name,
                object: &operation.owner,
            })
        })
}

/// The source-owned `model`/`object_type` node of `declaration`.
#[ix_trace_rs::string_edge]
pub(crate) fn source_node_for_declaration<'a>(
    graph: &'a CheckedGraph,
    declaration: &DeclarationKey,
) -> Option<&'a SemanticNode> {
    graph.semantic_graph().nodes().find(|node| {
        graph.resolve_declaration(node.key()) == Some(declaration)
            && matches!(node.owner(), Some(Owner::Source(_)))
            && node.node_tag() == NodeTag::Model
            && node.semantic_form() == "object_type"
    })
}

/// The final authored segment in a declaration key's exact `ix://` identity.
/// This parses the admitted key shape; it does not use a formatter or display.
#[ix_trace_rs::string_edge]
pub(crate) fn source_identity_segment<'a>(package: &str, identity: &'a str) -> Option<&'a str> {
    identity
        .strip_prefix(&format!("ix://{package}/"))
        .filter(|segment| !segment.is_empty() && !segment.contains('/'))
}

impl CheckedPackage {
    /// The clause function of `clause` of the source operation whose checked
    /// `state`/`operation_anchor` node is `anchor`: the source route's form of
    /// `BarePackage::project_source_operation_clause` (FR-531). As on the
    /// bytes route, the anchor's operation is source-declared and the function
    /// is owned by the unit that declares the operation's type.
    ///
    /// # Errors
    ///
    /// [`SlotSelectionFailure::MalformedNode`] of `anchor` when it anchors no
    /// source-declared operation of this package or more than one function
    /// declares the clause, [`SlotSelectionFailure::MissingNode`] of `anchor`
    /// when none does.
    #[implements("FR-531-AC-40")]
    pub fn select_source_operation_clause(
        &self,
        anchor: &NodeKey,
        clause: &SourceOperationClause,
    ) -> Result<FunctionState<'_>, SlotSelectionFailure> {
        let graph = self.graph();
        let malformed = || SlotSelectionFailure::MalformedNode(anchor.clone());
        let operation = declared_source_operations(graph)
            .find(|operation| operation.anchor == *anchor)
            .ok_or_else(malformed)?;
        let owner = operation.owner(graph).ok_or_else(|| {
            SlotSelectionFailure::Fault(InternalFault::new(
                "FR-531",
                "source-operation-type-has-a-source-owner",
            ))
        })?;
        let mut matches = (0..)
            .map_while(|index| Some((index, graph.function_identity_at(index)?)))
            .filter(|(_, identity)| {
                graph
                    .semantic_graph()
                    .node(identity.clone())
                    .is_some_and(|node| {
                        matches!(
                            node.owner(),
                            Some(Owner::Source(subject))
                                if subject.authority() == owner.authority()
                                    && subject.identity() == owner.identity()
                        ) && node.declaration().is_some_and(|declaration| {
                            clause.is_declared_by(
                                operation.type_name,
                                operation.operation_name,
                                declaration.iter().map(|segment| segment.as_str()),
                            )
                        })
                    })
            });
        let Some((index, _)) = matches.next() else {
            return Err(SlotSelectionFailure::MissingNode(anchor.clone()));
        };
        if matches.next().is_some() {
            return Err(malformed());
        }
        graph.function_state(index).ok_or_else(|| {
            SlotSelectionFailure::Fault(InternalFault::new(
                "FR-531",
                "source-operation-clause-function-is-checked",
            ))
        })
    }
}
