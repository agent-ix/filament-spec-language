// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-995 declaration normalization for FR-230's source and bare clock paths.
//! Source handles are resolved against the checked declaration that owns them;
//! a short member spelling is resolved only in its actual declaring row.

use std::collections::BTreeMap;

use filament_checked::key::DeclarationKey;
use filament_exact::{Identifier, NodeKey};
use filament_semantic_value::catalog::InternalFault;

use crate::check::{CheckedGraph, SourceOwner};
use crate::model::clock_reference::identity::DeclarationScope;

/// Original declaration facts read from checking or an admitted package.
/// This value is not an authentication capability for caller-supplied names.
#[derive(Clone, Debug)]
pub struct SourceDeclarationFact {
    /// The declaration's retained SourceOwner.
    pub owner: SourceOwner,
    /// Transient checked node used only to join actual declaration facts.
    pub node: NodeKey,
    /// Original qualified declaration name, not an import alias.
    pub name: Vec<Identifier>,
    /// Model normalization's original declaration key, when it has one.
    pub original: Option<DeclarationKey>,
}

/// Authenticated source declaration correspondences from one checked graph.
#[derive(Clone, Debug, Default)]
pub struct SourceDeclarations {
    packages: BTreeMap<String, String>,
    checked_nodes: BTreeMap<String, String>,
    namespaces: BTreeMap<String, (String, String)>,
    declarations: BTreeMap<DeclarationKey, DeclarationScope>,
    node_scopes: BTreeMap<NodeKey, DeclarationScope>,
    model_scopes: BTreeMap<NodeKey, DeclarationScope>,
}

impl SourceDeclarations {
    /// Read only the SourceOwner and original declaration facts retained by
    /// checking. No short-name search or caller-created numeric id is authority.
    pub fn from_graph(graph: &CheckedGraph) -> Result<Self, InternalFault> {
        let mut facts = Vec::new();
        for node in graph.semantic_graph().nodes() {
            let Some(crate::check::Owner::Source(owner)) = node.owner() else {
                continue;
            };
            let Some(name) = node.declaration() else {
                continue;
            };
            facts.push(SourceDeclarationFact {
                owner: owner.clone(),
                node: node.key(),
                name: name.to_vec(),
                original: graph.resolve_declaration(node.key()).cloned(),
            });
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
            facts.push(SourceDeclarationFact {
                owner: owner.clone(),
                node: key.clone(),
                name: name.to_vec(),
                original: None,
            });
        }
        let mut result = Self::from_facts(facts)?;
        for node in graph.semantic_graph().nodes() {
            if let Some(crate::check::Owner::Model(owner)) = node.owner() {
                result.model_scopes.insert(
                    node.key(),
                    DeclarationScope::Model {
                        identity: owner.identity().to_owned(),
                        declaration: owner.node().to_owned(),
                    },
                );
            }
        }
        for declaration in graph.scope().types().composites() {
            if let Some(node) = graph.semantic_graph().composite_node(&declaration.key()) {
                if result.node_scope(node).is_some() {
                    result.retain_node_alias(node, declaration.key())?;
                }
            }
        }
        Ok(result)
    }

    /// Normalize facts supplied by the checked-source or sealed-package
    /// adapter. Both routes retain source authority before rewriting URIs.
    pub fn from_facts(
        facts: impl IntoIterator<Item = SourceDeclarationFact>,
    ) -> Result<Self, InternalFault> {
        let mut result = Self::default();
        for fact in facts {
            let declaration = declaration_uri(&fact.owner, &fact.name);
            let scope = DeclarationScope::Source {
                authority: fact.owner.authority().to_owned(),
                identity: fact.owner.identity().to_owned(),
                declaration: declaration.clone(),
            };
            if let Some(original) = fact.original {
                let namespace = (
                    fact.owner.authority().to_owned(),
                    fact.owner.identity().to_owned(),
                );
                match result.namespaces.entry(original.package.clone()) {
                    std::collections::btree_map::Entry::Vacant(entry) => {
                        entry.insert(namespace);
                    }
                    std::collections::btree_map::Entry::Occupied(entry)
                        if entry.get() == &namespace => {}
                    std::collections::btree_map::Entry::Occupied(_) => {
                        return Err(fault("source-package-has-one-authenticated-owner"))
                    }
                }
                result
                    .packages
                    .insert(original.package.clone(), fact.owner.identity().to_owned());
                result.declarations.insert(original, scope.clone());
            }
            result
                .checked_nodes
                .insert(fact.node.to_string(), declaration);
            result.node_scopes.insert(fact.node, scope);
        }
        Ok(result)
    }

    /// The original source scope attached to this actual checked node.
    /// Callers may not replace an absent join by a global short-name match.
    pub fn node_scope(&self, node: &NodeKey) -> Option<&DeclarationScope> {
        self.node_scopes.get(node)
    }

    /// Resolve the complete type identity already carried by a checked
    /// resolution row against actual declaration metadata. Distinct scopes
    /// sharing a rendering are refused rather than selected by map order.
    pub fn row_scope(&self, text: &str) -> Result<Option<DeclarationScope>, InternalFault> {
        if let Ok(node) = text.parse::<NodeKey>() {
            if let Some(scope) = self
                .node_scopes
                .get(&node)
                .or_else(|| self.model_scopes.get(&node))
            {
                return Ok(Some(scope.clone()));
            }
        }
        let normalized = self.text(text);
        let mut found = None;
        for scope in self.node_scopes.values().chain(self.model_scopes.values()) {
            let declaration = match scope {
                DeclarationScope::Source { declaration, .. }
                | DeclarationScope::Model { declaration, .. } => declaration,
            };
            if declaration == &normalized {
                if found.as_ref().is_some_and(|previous| previous != scope) {
                    return Err(fault("clock-type-rendering-has-one-original-scope"));
                }
                found = Some(scope.clone());
            }
        }
        Ok(found)
    }

    /// Normalize an actual checked clock field using this graph's retained
    /// declaration context. Inherited fields select their original first
    /// declaring row, not their effective receiver type.
    pub fn clock_template(
        &self,
        clock: &crate::check::ClockField,
    ) -> Result<super::identity::PlaceTemplate, InternalFault> {
        let first = clock
            .resolved_path()
            .first()
            .ok_or_else(|| fault("clock-field-has-a-resolved-path"))?;
        let scope = self
            .row_scope(&first.declaring_type)?
            .ok_or_else(|| fault("clock-field-has-original-declaring-scope"))?;
        super::path::normalize(
            scope,
            clock.path(),
            clock.resolved_path(),
            |text| self.text(text),
            |declaring, member| self.member(declaring, member),
            |text| self.row_scope(text),
        )
    }

    /// Retain a type handle whose actual checked/selected declaration join
    /// already resolved to `node`. This is transient lookup provenance, not
    /// an additional component of the persisted identity.
    pub fn retain_node_alias(
        &mut self,
        node: &NodeKey,
        handle: NodeKey,
    ) -> Result<(), InternalFault> {
        let scope = self
            .node_scopes
            .get(node)
            .ok_or_else(|| fault("type-handle-joins-a-source-declaration"))?
            .clone();
        let declaration = match &scope {
            DeclarationScope::Source { declaration, .. }
            | DeclarationScope::Model { declaration, .. } => declaration.clone(),
        };
        self.checked_nodes.insert(handle.to_string(), declaration);
        self.node_scopes.insert(handle, scope);
        Ok(())
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
