// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-995 declaration normalization for FR-230's source and bare clock paths.
//! Source handles are resolved against the checked declaration that owns them;
//! a short member spelling is resolved only in its actual declaring row.

use std::collections::BTreeMap;

use filament_checked::key::DeclarationKey;
use filament_semantic_value::catalog::InternalFault;

use crate::check::{CheckedGraph, SourceOwner};
use crate::model::clock_reference::identity::DeclarationScope;

/// Authenticated source declaration correspondences from one checked graph.
#[derive(Clone, Debug, Default)]
pub struct SourceDeclarations {
    packages: BTreeMap<String, String>,
    checked_nodes: BTreeMap<String, String>,
    namespaces: BTreeMap<String, (String, String)>,
    declarations: BTreeMap<DeclarationKey, DeclarationScope>,
}

impl SourceDeclarations {
    /// Read only the SourceOwner and original declaration facts retained by
    /// checking. No short-name search or caller-created numeric id is authority.
    pub fn from_graph(graph: &CheckedGraph) -> Result<Self, InternalFault> {
        let mut result = Self::default();
        for node in graph.semantic_graph().nodes() {
            let Some(crate::check::Owner::Source(owner)) = node.owner() else {
                continue;
            };
            let Some(name) = node.declaration() else {
                continue;
            };
            let declaration = declaration_uri(owner, name);
            if let Some(original) = graph.resolve_declaration(node.key()) {
                let namespace = (owner.authority().to_owned(), owner.identity().to_owned());
                match result.namespaces.entry(original.package.clone()) {
                    std::collections::btree_map::Entry::Vacant(entry) => {
                        entry.insert(namespace);
                    }
                    std::collections::btree_map::Entry::Occupied(entry)
                        if entry.get() == &namespace => {}
                    std::collections::btree_map::Entry::Occupied(_) => {
                        return Err(fault("source-package-has-one-authenticated-owner"));
                    }
                }
                result
                    .packages
                    .insert(original.package.clone(), owner.identity().to_owned());
                result.declarations.insert(
                    original.clone(),
                    DeclarationScope::Source {
                        authority: owner.authority().to_owned(),
                        identity: owner.identity().to_owned(),
                        declaration: declaration.clone(),
                    },
                );
            }
            result
                .checked_nodes
                .insert(node.key().to_string(), declaration);
        }
        for declaration in graph.scope().types().composites() {
            let Some(key) = graph.semantic_graph().composite_node(&declaration.key()) else {
                continue;
            };
            let Some(node) = graph.semantic_graph().node(key.clone()) else {
                return Err(fault("checked-composite-has-a-declaration-node"));
            };
            let (Some(crate::check::Owner::Source(owner)), Some(name)) =
                (node.owner(), node.declaration())
            else {
                continue;
            };
            result
                .checked_nodes
                .insert(key.to_string(), declaration_uri(owner, name));
        }
        Ok(result)
    }

    /// Choose the original scope from actual declaration provenance, never
    /// from the source or bare route through which the declaration is read.
    pub fn scope(&self, declaration: &DeclarationKey) -> Result<DeclarationScope, InternalFault> {
        if let Some(scope) = self.declarations.get(declaration) {
            return Ok(scope.clone());
        }
        if let Some((authority, identity)) = self.namespaces.get(&declaration.package) {
            return Ok(DeclarationScope::Source {
                authority: authority.clone(),
                identity: identity.clone(),
                declaration: self.original_key(declaration)?.node,
            });
        }
        Ok(DeclarationScope::Model {
            identity: declaration.package.clone(),
            declaration: declaration.node.clone(),
        })
    }

    /// Spell an original source key by its retained SourceOwner identity.
    pub fn original_key(&self, key: &DeclarationKey) -> Result<DeclarationKey, InternalFault> {
        let Some(identity) = self.packages.get(&key.package) else {
            return Ok(key.clone());
        };
        let prefix = format!("ix://{}/", key.package);
        let declaration = key
            .node
            .strip_prefix(&prefix)
            .ok_or_else(|| fault("source-key-belongs-to-its-declaring-package"))?;
        Ok(DeclarationKey {
            package: identity.clone(),
            node: format!("ix://{identity}/{declaration}"),
        })
    }

    /// Normalize declaration handles and selected private source URIs in a
    /// checked type spelling. This never resolves an unqualified member.
    pub fn text(&self, text: &str) -> String {
        if let Some(uri) = self.checked_nodes.get(text) {
            return uri.clone();
        }
        let mut normalized = text.to_owned();
        for (checked, uri) in &self.checked_nodes {
            if normalized.contains(checked) {
                normalized = normalized.replace(checked, uri);
            }
        }
        for (package, identity) in &self.packages {
            let prefix = format!("ix://{package}/");
            if let Some(declaration) = normalized.strip_prefix(&prefix) {
                return format!("ix://{identity}/{declaration}");
            }
        }
        normalized
    }

    /// Resolve a retained member spelling through its authenticated declaring
    /// type. In particular, W/c is never matched against an unrelated W.
    pub fn member(&self, declaring: &str, member: &str) -> Result<String, InternalFault> {
        let normalized = self.text(member);
        if normalized.starts_with("ix://") || normalized.contains("[slot]") {
            return Ok(normalized);
        }
        let declaring = self.text(declaring);
        let Some((_, name)) = declaring.rsplit_once('/') else {
            return Err(fault("clock-member-has-a-normalized-declaring-type"));
        };
        let prefix = format!("{name}/");
        let suffix = normalized
            .strip_prefix(&prefix)
            .ok_or_else(|| fault("clock-member-belongs-to-its-declaring-type"))?;
        Ok(format!("{declaring}/{suffix}"))
    }
}

fn declaration_uri(owner: &SourceOwner, name: &[filament_exact::Identifier]) -> String {
    format!(
        "ix://{}/{}",
        owner.identity(),
        name.iter()
            .map(|part| part.as_str())
            .collect::<Vec<_>>()
            .join("/")
    )
}

fn fault(invariant: &'static str) -> InternalFault {
    InternalFault::new("clock-declaration-normalization", invariant)
}
