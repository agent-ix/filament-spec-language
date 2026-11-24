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
use crate::model::clock_reference::identity::PlaceTemplate;

/// One actual named member's original declaration, before URI normalization.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum OriginalClockMemberOrigin {
    /// An inline member of an authenticated source declaration.
    Source {
        /// The original declaring unit.
        owner: SourceOwner,
        /// The original qualified type and member name.
        qualified_name: Vec<Identifier>,
    },
    /// A member of an admitted original model declaration.
    Model {
        /// The actual original member key, independent of import aliases.
        declaration: DeclarationKey,
    },
}

/// The ordered original named members along a checked clock path.
/// Tuple positions, cases and collection coordinates belong to the template.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ClockDeclarationOrigin(pub Vec<OriginalClockMemberOrigin>);

/// Distinct authenticated declarations claiming one full normalized template.
#[derive(Clone, Debug, Eq, Hash, PartialEq, thiserror::Error)]
#[error("clock declarations {first:?} and {second:?} have one normalized identity")]
pub struct ClockDeclarationCollision {
    /// The first original declaration chain.
    pub first: ClockDeclarationOrigin,
    /// The competing original declaration chain.
    pub second: ClockDeclarationOrigin,
}

/// Competing original declarations for one checked declaring-type context.
#[derive(Clone, Debug, Eq, Hash, PartialEq, thiserror::Error)]
#[error("clock ownership is ambiguous between {first:?} and {second:?}")]
pub struct ClockDeclarationAmbiguity {
    /// The first authenticated candidate.
    pub first: OriginalClockMemberOrigin,
    /// The competing authenticated candidate.
    pub second: OriginalClockMemberOrigin,
}

/// Declaration resolution preserves user ambiguity separately from a broken
/// checked-path invariant.
#[derive(Clone, Debug, thiserror::Error)]
pub enum ClockDeclarationResolutionError {
    /// Complete declaration context has competing authenticated origins.
    #[error(transparent)]
    Ambiguous(Box<ClockDeclarationAmbiguity>),
    /// An admitted path no longer agrees with its declaration metadata.
    #[error("clock declaration invariant: {0:?}")]
    Fault(InternalFault),
}

impl From<ClockDeclarationAmbiguity> for ClockDeclarationResolutionError {
    fn from(ambiguity: ClockDeclarationAmbiguity) -> Self {
        Self::Ambiguous(Box::new(ambiguity))
    }
}

impl From<InternalFault> for ClockDeclarationResolutionError {
    fn from(fault: InternalFault) -> Self {
        Self::Fault(fault)
    }
}

/// One run-local checked place joined to its route-neutral declaration.
/// The run key indexes this context only and is never persisted in an ID.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClockRunPlace {
    /// Actual checked owner type used for the existing conformance check.
    pub owner: filament_exact::EffectiveId,
    /// Shared normalized original declaration and full typed path.
    pub template: std::sync::Arc<PlaceTemplate>,
}

/// Validated static clock declarations, independent of runtime populations.
#[derive(Clone, Debug, Default)]
pub struct ClockDeclarationTable {
    entries: std::sync::Arc<BTreeMap<PlaceTemplate, ClockDeclarationOrigin>>,
    source_declarations: std::sync::Arc<SourceDeclarations>,
    run_places: std::sync::Arc<BTreeMap<NodeKey, ClockRunPlace>>,
}

impl ClockDeclarationTable {
    /// Build atomically, collapsing repeated views of the same original
    /// declaration and refusing distinct origins with one scoped template.
    pub fn from_entries(
        entries: impl IntoIterator<Item = (PlaceTemplate, ClockDeclarationOrigin)>,
    ) -> Result<Self, ClockDeclarationCollision> {
        let mut table = BTreeMap::new();
        for (template, origin) in entries {
            match table.entry(template) {
                std::collections::btree_map::Entry::Vacant(entry) => {
                    entry.insert(origin);
                }
                std::collections::btree_map::Entry::Occupied(entry) if entry.get() == &origin => {}
                std::collections::btree_map::Entry::Occupied(entry) => {
                    return Err(ClockDeclarationCollision {
                        first: entry.get().clone(),
                        second: origin,
                    });
                }
            }
        }
        Ok(Self {
            entries: std::sync::Arc::new(table),
            source_declarations: std::sync::Arc::default(),
            run_places: std::sync::Arc::default(),
        })
    }

    /// Attach only actual checked run nodes already represented in this
    /// atomic declaration table; no allocation key becomes declaration data.
    pub fn with_run_places(
        mut self,
        places: impl IntoIterator<Item = (NodeKey, filament_exact::EffectiveId, PlaceTemplate)>,
    ) -> Result<Self, InternalFault> {
        let mut run_places = BTreeMap::new();
        for (node, owner, template) in places {
            if !self.entries.contains_key(&template) {
                return Err(fault("run-place-has-an-admitted-original-declaration"));
            }
            let place = ClockRunPlace {
                owner,
                template: std::sync::Arc::new(template),
            };
            if let Some(previous) = run_places.insert(node, place.clone()) {
                if previous != place {
                    return Err(fault("checked-run-place-has-one-declaration"));
                }
            }
        }
        self.run_places = std::sync::Arc::new(run_places);
        Ok(self)
    }

    /// The original declared place selected by this run's actual checked key.
    pub fn run_place(&self, node: &NodeKey) -> Option<&ClockRunPlace> {
        self.run_places.get(node)
    }

    /// Actual checked places and their existing owner-conformance authority.
    pub fn run_places(&self) -> impl Iterator<Item = (&NodeKey, &ClockRunPlace)> {
        self.run_places.iter()
    }

    /// The authenticated origin of a full scoped declaration template.
    pub fn origin(&self, template: &PlaceTemplate) -> Option<&ClockDeclarationOrigin> {
        self.entries.get(template)
    }

    /// Each unique declaration in canonical template order.
    pub fn entries(&self) -> impl Iterator<Item = (&PlaceTemplate, &ClockDeclarationOrigin)> {
        self.entries.iter()
    }

    /// Attach the original declaration context used by this route's checked
    /// factory, including original population-key correspondences.
    pub fn with_source_declarations(mut self, declarations: SourceDeclarations) -> Self {
        self.source_declarations = std::sync::Arc::new(declarations);
        self
    }

    /// Original source provenance used to authenticate population scopes.
    pub fn source_declarations(&self) -> &SourceDeclarations {
        &self.source_declarations
    }
}

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
    original_nodes: BTreeMap<NodeKey, OriginalClockMemberOrigin>,
    original_declarations: BTreeMap<DeclarationKey, Vec<OriginalClockMemberOrigin>>,
}

impl SourceDeclarations {
    /// Read only the SourceOwner and original declaration facts retained by
    /// checking. No short-name search or caller-created numeric id is authority.
    pub fn from_graph(graph: &CheckedGraph) -> Result<Self, InternalFault> {
        Self::from_graph_with_composites(graph, graph.scope().types().composites())
    }

    /// Include composite declarations retained in isolated checked scopes,
    /// using their actual semantic declaration nodes for the same provenance join.
    pub fn from_graph_with_composites<'a>(
        graph: &CheckedGraph,
        composites: impl IntoIterator<
            Item = &'a filament_semantic_value::declaration::CompositeDeclaration,
        >,
    ) -> Result<Self, InternalFault> {
        let composites: Vec<_> = composites.into_iter().collect();
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
        for declaration in &composites {
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
        result =
            result.with_model_declarations(graph.semantic_graph().nodes().filter_map(|node| {
                let Some(crate::check::Owner::Model(owner)) = node.owner() else {
                    return None;
                };
                Some((
                    node.key(),
                    DeclarationKey {
                        package: owner.identity().to_owned(),
                        node: owner.node().to_owned(),
                    },
                ))
            }));
        for declaration in &composites {
            if let Some(node) = graph.semantic_graph().composite_node(&declaration.key()) {
                if result.node_scope(node).is_some() {
                    result.retain_node_alias(node, declaration.key())?;
                }
            }
        }
        Ok(result)
    }

    /// Retain original model declarations from their actual checked nodes.
    /// A retained SourceOwner remains authoritative for a source-owned node.
    pub fn with_model_declarations(
        mut self,
        declarations: impl IntoIterator<Item = (NodeKey, DeclarationKey)>,
    ) -> Self {
        for (node, declaration) in declarations {
            if !self.node_scopes.contains_key(&node) {
                self.model_scopes.insert(
                    node,
                    DeclarationScope::Model {
                        identity: declaration.package,
                        declaration: declaration.node,
                    },
                );
            }
        }
        self
    }

    /// Normalize facts supplied by the checked-source or sealed-package
    /// adapter. Both routes retain source authority before rewriting URIs.
    pub fn from_facts(
        facts: impl IntoIterator<Item = SourceDeclarationFact>,
    ) -> Result<Self, InternalFault> {
        let mut result = Self::default();
        for fact in facts {
            let origin = OriginalClockMemberOrigin::Source {
                owner: fact.owner.clone(),
                qualified_name: fact.name.clone(),
            };
            result
                .original_nodes
                .insert(fact.node.clone(), origin.clone());
            let declaration = declaration_uri(&fact.owner, &fact.name);
            let scope = DeclarationScope::Source {
                authority: fact.owner.authority().to_owned(),
                identity: fact.owner.identity().to_owned(),
                declaration: declaration.clone(),
            };
            if let Some(original) = fact.original {
                result
                    .original_declarations
                    .entry(original.clone())
                    .or_default()
                    .push(origin);
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
                        return Err(fault("source-package-has-one-authenticated-owner"));
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

    /// Original provenance of an actual checked declaration node, retaining
    /// SourceOwner priority over selected-model provenance.
    pub fn declaration_scope_for_node(&self, node: &NodeKey) -> Option<&DeclarationScope> {
        self.node_scopes
            .get(node)
            .or_else(|| self.model_scopes.get(node))
    }

    /// The original source scope attached to this actual checked node.
    /// Callers may not replace an absent join by a global short-name match.
    pub fn node_scope(&self, node: &NodeKey) -> Option<&DeclarationScope> {
        self.node_scopes.get(node)
    }

    /// Resolve the complete type identity already carried by a checked
    /// resolution row against actual declaration metadata. Distinct scopes
    /// sharing a rendering are refused rather than selected by map order.
    pub fn row_scope(
        &self,
        text: &str,
    ) -> Result<Option<DeclarationScope>, ClockDeclarationResolutionError> {
        let mut joined = None;
        for (node, scope) in self.node_scopes.iter().chain(&self.model_scopes) {
            if node.to_string() == text {
                retain_scope(&mut joined, scope, self.node_origin(node, scope)?)?;
            }
        }
        if joined.is_some() {
            return Ok(joined.map(|(scope, _)| scope));
        }
        let mut original = None;
        for (key, scope) in &self.declarations {
            if key.node == text {
                let origins = self
                    .original_declarations
                    .get(key)
                    .ok_or_else(|| fault("original-declaration-retains-raw-provenance"))?;
                for origin in origins {
                    retain_scope(&mut original, scope, origin.clone())?;
                }
            }
        }
        if original.is_some() {
            return Ok(original.map(|(scope, _)| scope));
        }
        let normalized = self.text(text);
        let mut found = None;
        for (node, scope) in self.node_scopes.iter().chain(&self.model_scopes) {
            let declaration = match scope {
                DeclarationScope::Source { declaration, .. }
                | DeclarationScope::Model { declaration, .. } => declaration,
            };
            if declaration == &normalized {
                retain_scope(&mut found, scope, self.node_origin(node, scope)?)?;
            }
        }
        Ok(found.map(|(scope, _)| scope))
    }

    fn node_origin(
        &self,
        node: &NodeKey,
        scope: &DeclarationScope,
    ) -> Result<OriginalClockMemberOrigin, InternalFault> {
        match scope {
            DeclarationScope::Source { .. } => self
                .original_nodes
                .get(node)
                .cloned()
                .ok_or_else(|| fault("source-declaration-retains-raw-provenance")),
            DeclarationScope::Model {
                identity,
                declaration,
            } => Ok(OriginalClockMemberOrigin::Model {
                declaration: DeclarationKey {
                    package: identity.clone(),
                    node: declaration.clone(),
                },
            }),
        }
    }

    /// Normalize an actual checked clock field using this graph's retained
    /// declaration context. Inherited fields select their original first
    /// declaring row, not their effective receiver type.
    pub fn clock_template(
        &self,
        clock: &crate::check::ClockField,
    ) -> Result<super::identity::PlaceTemplate, ClockDeclarationResolutionError> {
        self.clock_template_rows(clock.path(), clock.resolved_path())
    }

    /// Normalize the actual checked rows supplied by either route through
    /// the same original-declaration resolver and path normalizer.
    pub fn clock_template_rows(
        &self,
        steps: &[crate::check::PlaceStep],
        rows: &[crate::check::PlaceRow],
    ) -> Result<PlaceTemplate, ClockDeclarationResolutionError> {
        let first = rows
            .first()
            .ok_or_else(|| fault("clock-field-has-a-resolved-path"))?;
        let scope = self
            .row_scope(&first.declaring_type)?
            .ok_or_else(|| fault("clock-field-has-original-declaring-scope"))?;
        super::path::normalize(
            scope,
            steps,
            rows,
            |text| self.text(text),
            |declaring, member| self.member(declaring, member).map_err(Into::into),
            |text| self.row_scope(text),
        )
    }

    /// Recover the original named-member chain through the complete checked
    /// declaring-type handles in the resolved path. Inline source fields
    /// belong to their declaring type's actual source owner and name.
    pub fn clock_origin(
        &self,
        clock: &crate::check::ClockField,
    ) -> Result<ClockDeclarationOrigin, ClockDeclarationResolutionError> {
        self.clock_origin_rows(clock.path(), clock.resolved_path())
    }

    /// Recover the same original inline member chain from either route's
    /// checked steps and resolution rows, without a second provenance join.
    pub fn clock_origin_rows(
        &self,
        steps: &[crate::check::PlaceStep],
        rows: &[crate::check::PlaceRow],
    ) -> Result<ClockDeclarationOrigin, ClockDeclarationResolutionError> {
        if steps.len() != rows.len() {
            return Err(fault("clock-path-and-resolution-rows-correspond").into());
        }
        let mut members = Vec::new();
        for (step, row) in steps.iter().zip(rows) {
            let crate::check::PlaceStep::Field(name) = step else {
                continue;
            };
            if super::identity::Decimal::new(name).is_ok() {
                continue;
            }
            let mut declaring = row.declaring_type.as_str();
            while let Some(payload) = declaring
                .strip_prefix("Option<")
                .and_then(|value| value.strip_suffix('>'))
            {
                declaring = payload;
            }
            let mut original = None;
            for (node, origin) in &self.original_nodes {
                if node.to_string() == declaring {
                    retain_original(&mut original, origin)?;
                }
            }
            for (key, origins) in &self.original_declarations {
                if key.node == declaring {
                    for origin in origins {
                        retain_original(&mut original, origin)?;
                    }
                }
            }
            if original.is_none() {
                // Emitted rows carry the full original declaration URI rather
                // than its transient structural handle. Resolve that URI
                // against authenticated metadata, then retain every matching
                // original owner/name fact; distinct origins still refuse.
                if let Some(scope) = self.row_scope(declaring)? {
                    for (node, origin) in &self.original_nodes {
                        if self.node_scopes.get(node) == Some(&scope) {
                            retain_original(&mut original, origin)?;
                        }
                    }
                }
            }
            match original {
                Some(OriginalClockMemberOrigin::Source {
                    owner,
                    mut qualified_name,
                }) => {
                    qualified_name.push(
                        Identifier::new(name.clone())
                            .map_err(|_| fault("checked-clock-field-name-is-an-identifier"))?,
                    );
                    members.push(OriginalClockMemberOrigin::Source {
                        owner,
                        qualified_name,
                    });
                }
                Some(OriginalClockMemberOrigin::Model { .. }) => {
                    return Err(fault("source-origin-map-retains-source-declarations").into());
                }
                None => {
                    let Some(DeclarationScope::Model { identity, .. }) =
                        self.row_scope(declaring)?
                    else {
                        return Err(fault("clock-member-joins-an-original-declaration").into());
                    };
                    members.push(OriginalClockMemberOrigin::Model {
                        declaration: DeclarationKey {
                            package: identity,
                            node: row.member_identity.clone(),
                        },
                    });
                }
            }
        }
        Ok(ClockDeclarationOrigin(members))
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
        if let Some(origin) = self.original_nodes.get(node).cloned() {
            self.original_nodes.insert(handle.clone(), origin);
        }
        self.node_scopes.insert(handle, scope);
        Ok(())
    }

    /// Resolve an actual catalog key only through retained declaration
    /// provenance. An unknown key is not promoted to a Model descriptor.
    pub fn catalog_scope(&self, declaration: &DeclarationKey) -> Option<&DeclarationScope> {
        self.declarations.get(declaration).or_else(|| {
            self.model_scopes.values().find(|scope| {
                matches!(scope,
                    DeclarationScope::Model { identity, declaration: original }
                        if identity == &declaration.package && original == &declaration.node
                )
            })
        })
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

fn retain_original(
    selected: &mut Option<OriginalClockMemberOrigin>,
    candidate: &OriginalClockMemberOrigin,
) -> Result<(), ClockDeclarationResolutionError> {
    if let Some(previous) = selected.as_ref().filter(|previous| *previous != candidate) {
        return Err(ClockDeclarationAmbiguity {
            first: previous.clone(),
            second: candidate.clone(),
        }
        .into());
    }
    *selected = Some(candidate.clone());
    Ok(())
}

fn retain_scope(
    selected: &mut Option<(DeclarationScope, OriginalClockMemberOrigin)>,
    scope: &DeclarationScope,
    origin: OriginalClockMemberOrigin,
) -> Result<(), ClockDeclarationResolutionError> {
    if let Some((previous_scope, previous)) = selected {
        if previous_scope != scope || previous != &origin {
            return Err(ClockDeclarationAmbiguity {
                first: previous.clone(),
                second: origin,
            }
            .into());
        }
    }
    *selected = Some((scope.clone(), origin));
    Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::check::{PackageDeclarations, test_support::ZeroRewardEvaluator};
    use crate::model::intake::SelectedModels;
    use filament_semantic_value::checking::{CheckMode, CheckingLimits};
    use ix_trace_rs::trace;

    fn source_graph() -> CheckedGraph {
        let text = "language \"ix:native\" edition \"1-draft\";
            profile t = \"filament.temporal.complete/v1\";
            dimension Time; unit ms : Time = rational(1, 1);
            record W { c: Clock; }
            object O { w: W; d: Clock; }
            time O dense unit ms;";
        let parsed = fsl_cst::parse(
            fsl_foundation::SourceIdentity::new("test", "clock-table"),
            "clocks.native",
            text.as_bytes(),
            fsl_cst::Limits::default(),
        )
        .expect("source clock fixture parses");
        assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
        let forms = fsl_forms::build_unit(&parsed).expect("source clock forms");
        let mut declarations = PackageDeclarations::assemble(
            parsed.source().reference().clone(),
            "clock-table-test".to_owned(),
            forms,
            SelectedModels::fixture_entries(Vec::new()),
            Vec::new(),
        )
        .expect("source clock declarations");
        let pending = declarations
            .link_dispatch(crate::model::accounting::ModelNormalizationLimits::default())
            .expect("source dispatch links");
        declarations
            .check(
                CheckMode::Linked,
                CheckingLimits::default(),
                &ZeroRewardEvaluator,
            )
            .map_err(|refusals| pending.report(refusals))
            .expect("source clocks check and publish their table")
    }

    #[trace("FR-1040-AC-9", "TC-3084")]
    #[test]
    fn checked_source_publishes_full_original_member_chains() {
        let graph = source_graph();
        let table = graph.clock_declaration_table();
        assert_eq!(table.entries().count(), 2);
        let nested = table
            .entries()
            .find(|(_, origin)| origin.0.len() == 2)
            .expect("nested record clock retains two original fields");
        let names: Vec<_> = nested
            .1
            .0
            .iter()
            .map(|member| match member {
                OriginalClockMemberOrigin::Source { qualified_name, .. } => qualified_name
                    .iter()
                    .map(Identifier::as_str)
                    .collect::<Vec<_>>(),
                other => panic!("source provenance was lost: {other:?}"),
            })
            .collect();
        assert_eq!(names, [vec!["O", "w"], vec!["W", "c"]]);
        let alias_view = ClockDeclarationTable::from_entries(
            table
                .entries()
                .chain(table.entries())
                .map(|(template, origin)| (template.clone(), origin.clone())),
        )
        .expect("repeated authenticated aliases are one view");
        assert_eq!(alias_view.entries().count(), 2);
        let shared = table.clone();
        assert!(std::sync::Arc::ptr_eq(&table.entries, &shared.entries));
        assert!(std::sync::Arc::ptr_eq(
            &table.source_declarations,
            &shared.source_declarations
        ));
    }

    #[trace("FR-1040-AC-9", "TC-3084")]
    #[test]
    fn collision_names_both_checked_origins_and_returns_no_table() {
        let graph = source_graph();
        let entries: Vec<_> = graph.clock_declaration_table().entries().collect();
        let (template, first) = entries[0];
        let (_, second) = entries[1];
        assert_ne!(first, second, "fixture has distinct original declarations");
        let error = ClockDeclarationTable::from_entries([
            (template.clone(), first.clone()),
            (template.clone(), second.clone()),
        ])
        .expect_err("distinct original chains cannot share a normalized identity");
        assert_eq!(&error.first, first);
        assert_eq!(&error.second, second);
        let cause = crate::check::CheckCause::DuplicateClockDeclaration(Box::new(error));
        assert_eq!(
            cause.code(),
            fsl_foundation::diagnostic::Code::ConflictingDeclaration
        );
        assert_eq!(cause.cause(), Some("duplicate-declaration"));
        let mut other = template.clone();
        let DeclarationScope::Source { identity, .. } = &mut other.scope else {
            panic!("source fixture");
        };
        *identity = "independent-unit".to_owned();
        let distinct = ClockDeclarationTable::from_entries([
            (template.clone(), first.clone()),
            (other, second.clone()),
        ])
        .expect("identical paths in different authenticated scopes coexist");
        assert_eq!(distinct.entries().count(), 2);
    }

    #[trace("FR-1040-AC-9", "TC-3084")]
    #[test]
    fn complete_ownership_ambiguity_retains_both_original_source_units() {
        let facts: Vec<_> = ["first", "second"]
            .into_iter()
            .map(|authority| SourceDeclarationFact {
                owner: SourceOwner::new(authority, "shared-identity").expect("source owner"),
                node: NodeKey::new(
                    "checked",
                    vec![filament_exact::PathSegment::Text(authority.to_owned())],
                    0,
                )
                .expect("transient typed join key"),
                name: vec![Identifier::new("W").expect("qualified declaration")],
                original: None,
            })
            .collect();
        let declarations = SourceDeclarations::from_facts(facts).expect("two authenticated units");
        assert_eq!(
            declarations
                .row_scope("W")
                .expect("no global short-name lookup"),
            None
        );
        let error = declarations
            .row_scope("ix://shared-identity/W")
            .expect_err("one full rendering cannot select either authenticated source unit");
        let ClockDeclarationResolutionError::Ambiguous(ambiguity) = error else {
            panic!("ownership ambiguity must retain its typed refusal");
        };
        let authorities: Vec<_> = [&ambiguity.first, &ambiguity.second]
            .into_iter()
            .map(|origin| match origin {
                OriginalClockMemberOrigin::Source { owner, .. } => owner.authority(),
                other => panic!("source provenance was lost: {other:?}"),
            })
            .collect();
        assert_eq!(authorities, ["first", "second"]);
        let cause = crate::check::CheckCause::AmbiguousClockDeclaration(ambiguity);
        assert_eq!(
            cause.code(),
            fsl_foundation::diagnostic::Code::AmbiguousDeclaration
        );
        assert_eq!(cause.cause(), Some("ambiguous-name"));
    }
}
