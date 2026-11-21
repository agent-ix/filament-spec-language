// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-230/FR-995: original declaration provenance from the admitted bare
//! graph. No source-side package or new witness participates in this view.

use std::collections::BTreeMap;

use filament_contract_model::CheckedNodeOwner;
use filament_exact::{Identifier, NodeKey};
use filament_semantic_value::catalog::InternalFault;
use fsl_semantics::check::SourceOwner;
use fsl_semantics::model::clock_reference::identity::DeclarationScope;
use fsl_semantics::model::clock_reference::normalization::{
    SourceDeclarationFact, SourceDeclarations,
};

use crate::{BarePackage, BareSelectionFailure};

/// Original source and model declaration scopes retained by admitted nodes.
#[derive(Clone, Debug)]
pub struct ClockDeclarations {
    source: SourceDeclarations,
    models: BTreeMap<NodeKey, DeclarationScope>,
}

impl ClockDeclarations {
    /// The same declaration normalizer the source route and emitter use.
    pub fn source(&self) -> &SourceDeclarations {
        &self.source
    }

    /// Read the actual admitted node's original scope. An absent declaration
    /// is not replaced by a short-name or effective-receiver lookup.
    pub fn scope(&self, node: &NodeKey) -> Option<&DeclarationScope> {
        self.source
            .node_scope(node)
            .or_else(|| self.models.get(node))
    }
}

impl BarePackage<'_> {
    /// Read original declaration facts from this package's already admitted
    /// node metadata. A retained SourceOwner wins on the bare route too.
    ///
    /// # Errors
    /// An internal fault if admitted owner or declaration metadata no longer
    /// agrees with the package's checked structural node correspondence.
    pub fn clock_declarations(&self) -> Result<ClockDeclarations, BareSelectionFailure> {
        let mut source = Vec::new();
        let mut models = BTreeMap::new();
        for node in &self.package().resolved_graph().nodes {
            let Some(owner) = &node.owner else {
                continue;
            };
            let key = self
                .node_key(&node.node_id)
                .ok_or_else(|| fault("clock-declaration-has-admitted-node-key"))?;
            match owner {
                CheckedNodeOwner::Source {
                    authority,
                    identity,
                } => {
                    let Some(declaration) = &node.declaration else {
                        continue;
                    };
                    let name = declaration
                        .qualified_name
                        .iter()
                        .map(|part| part.as_ref().parse::<Identifier>())
                        .collect::<Result<Vec<_>, _>>()
                        .map_err(|_| fault("admitted-source-name-is-qualified-identifiers"))?;
                    let owner = SourceOwner::new(authority.as_ref(), identity.as_ref())
                        .map_err(|_| fault("admitted-source-owner-is-valid"))?;
                    source.push(SourceDeclarationFact {
                        owner,
                        node: key,
                        name,
                        original: None,
                    });
                }
                CheckedNodeOwner::Model { identity, node } => {
                    models.insert(
                        key,
                        DeclarationScope::Model {
                            identity: identity.to_string(),
                            declaration: node.to_string(),
                        },
                    );
                }
                CheckedNodeOwner::ModelRoot { .. } => {}
            }
        }
        let mut source = SourceDeclarations::from_facts(source)
            .map_err(|_| fault("admitted-source-declaration-provenance-is-consistent"))?;
        let mut aliases = self
            .source_records()
            .iter()
            .map(|row| (&row.structural_node, row.type_handle.clone()))
            .chain(
                self.source_unions()
                    .iter()
                    .map(|row| (&row.structural_node, row.type_handle.clone())),
            )
            .collect::<Vec<_>>();
        if let Ok((_, identities)) = self.witness() {
            aliases.extend(
                identities
                    .source_tuple_rows()
                    .iter()
                    .map(|row| (&row.structural_node, row.type_handle.clone())),
            );
        }
        for (node, handle) in aliases {
            let key = self
                .node_key(node)
                .ok_or_else(|| fault("source-type-row-has-admitted-node"))?;
            source
                .retain_node_alias(&key, handle)
                .map_err(|_| fault("source-type-handle-joins-its-original-declaration"))?;
        }
        Ok(ClockDeclarations { source, models })
    }
}

fn fault(invariant: &'static str) -> BareSelectionFailure {
    BareSelectionFailure::from(crate::protocol_selection::SlotSelectionFailure::Fault(
        InternalFault::new("clock-declaration-projection", invariant),
    ))
}
