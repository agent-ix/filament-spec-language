// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-940: an admitted hyper atom projected over its ordered atom slots.

use filament_checked::node::NodeKind;
use filament_semantic_value::location::{ExpressionLocation, ExpressionOwner};

use super::{group_member, integer, malformed, reference, text, BTreeMap, BTreeSet};
use crate::bare_term::walk_failure;
use crate::reader_protocol_projection::{BareWalker, ReaderProtocolProjection};
use crate::{BareClauseRefusal, BarePackage};
use fsl_foundation::source::provenance::OccurrenceKey;

/// A package atom's evaluator nodes and the trace component of every model
/// read, derived from the admitted slot nodes at their declared levels.
pub struct BareHyperTerm {
    projection: ReaderProtocolProjection,
    components: BTreeMap<ExpressionLocation, usize>,
}

impl BareHyperTerm {
    /// The atom's projection through the shared reader expression walker.
    pub fn projection(&self) -> &ReaderProtocolProjection {
        &self.projection
    }

    /// The owning trace component of each projected model read.
    pub fn component_reads(&self) -> &BTreeMap<ExpressionLocation, usize> {
        &self.components
    }
}

impl BarePackage<'_> {
    /// Project `atom` of the selected claim's package formula, binding all
    /// admitted atom slots in level order. Charges no evaluation work.
    ///
    /// # Errors
    /// Preserves the hyper form's typed refusal. A foreign atom, unknown
    /// slot, or model read rooted in zero or multiple components refuses
    /// `MalformedNode` at the selected hyperproperty node.
    pub fn project_hyper_holds(
        &self,
        claim: &OccurrenceKey,
        atom: usize,
    ) -> Result<BareHyperTerm, BareClauseRefusal> {
        let form = self.hyper_clause_form(claim)?;
        let (_, root, _) = form
            .atoms
            .iter()
            .find(|(index, _, _)| *index == atom)
            .ok_or_else(|| malformed(claim))?;
        let node = self
            .node_by_key(claim.node())?
            .ok_or_else(|| malformed(claim))?;
        let hyper =
            crate::resolve_hyper_root(self.package(), claim).map_err(|_| malformed(claim))?;
        let wrapper = self
            .node_by_key(hyper.body().ok_or_else(|| malformed(claim))?)?
            .ok_or_else(|| malformed(claim))?;
        let filament_contract_model::CheckedTerm::Application { arguments, .. } = &wrapper.body
        else {
            return Err(malformed(claim));
        };
        let graph = crate::checked_v2::wire_graph(self.package()).map_err(|_| malformed(claim))?;
        let nodes: BTreeMap<_, _> = self
            .package()
            .resolved_graph()
            .nodes
            .iter()
            .map(|node| (&node.node_id, node))
            .collect();
        let mut pending: Vec<_> = form
            .atoms
            .iter()
            .map(|(_, key, _)| {
                self.node_by_key(key)
                    .map_err(BareClauseRefusal::from)?
                    .map(|node| &node.node_id)
                    .ok_or_else(|| malformed(claim))
            })
            .collect::<Result<_, _>>()?;
        pending.push(
            arguments
                .first()
                .and_then(reference)
                .ok_or_else(|| malformed(claim))?,
        );
        let mut seen = BTreeSet::new();
        let mut keys = BTreeMap::new();
        while let Some(id) = pending.pop() {
            if !seen.insert(id) {
                continue;
            }
            if u64::try_from(seen.len()).map_or(true, |n| n > super::FORMULA_VISITS) {
                return Err(super::pending(&node.node_id, "hyper-formula-work"));
            }
            let found = nodes.get(id).ok_or_else(|| malformed(claim))?;
            if found.semantic_form.as_ref() == "parameter"
                && group_member(&found.body, "name")
                    .and_then(text)
                    .is_some_and(|name| name.starts_with("slot_"))
            {
                let level = group_member(&found.body, "level")
                    .and_then(integer)
                    .ok_or_else(|| malformed(claim))?;
                keys.insert(level, graph.key(id).map_err(|_| malformed(claim))?);
            } else {
                pending.extend(found.body.reference_targets());
            }
        }
        if keys.len() != form.atom_slots.len() || keys.keys().copied().ne(0..form.atom_slots.len())
        {
            return Err(malformed(claim));
        }
        let binders: Vec<_> = keys.into_values().collect();
        let (_, identities) = self.witness()?;
        let walker = BareWalker::new(
            self.package(),
            identities,
            self.equality_decisions(),
            self.exact_type_projections(),
            self.dependency_views(),
        )
        .map_err(crate::protocol_selection::SlotSelectionFailure::Fault)
        .map_err(crate::BareSelectionFailure::from)?
        .with_populations(self);
        let projection = walker
            .project_term(
                root,
                &binders,
                ExpressionOwner::TypeDeclaration {
                    name: format!("hyper package holds {atom}"),
                },
            )
            .map_err(walk_failure)?;
        for (level, binder) in binders.iter().enumerate() {
            if projection.local_slot(binder) != Some(level) {
                return Err(malformed(claim));
            }
        }
        let components =
            components(&projection, &form.atom_slots).ok_or_else(|| malformed(claim))?;
        Ok(BareHyperTerm {
            projection,
            components,
        })
    }
}

/// Propagate slot ownership through the expression DAG, keeping independent
/// reads in a Boolean composition independent. A model read needs exactly
/// one owning trace; an unbound local never receives a default component.
#[derive(Clone, Copy, Eq, PartialEq)]
enum Ownership {
    None,
    Trace(usize),
    Mixed,
}

impl Ownership {
    fn join(self, other: Self) -> Self {
        match (self, other) {
            (Self::None, other) | (other, Self::None) => other,
            (Self::Trace(first), Self::Trace(second)) if first == second => self,
            (Self::Trace(_), Self::Trace(_)) | (Self::Mixed, _) | (_, Self::Mixed) => Self::Mixed,
        }
    }
}

fn components(
    projection: &ReaderProtocolProjection,
    slots: &[fsl_semantics::check::AtomSlot],
) -> Option<BTreeMap<ExpressionLocation, usize>> {
    let mut owners: BTreeMap<usize, Ownership> = BTreeMap::new();
    let mut reads = BTreeMap::new();
    let mut pending = vec![(projection.root(), false)];
    while let Some((index, exit)) = pending.pop() {
        if owners.contains_key(&index) {
            continue;
        }
        let kind = projection.kind(index)?;
        let children = projection.children(index)?;
        if !exit {
            pending.push((index, true));
            pending.extend(children.iter().rev().map(|child| (*child, false)));
            continue;
        }
        let mut ownership = Ownership::None;
        for child in children {
            ownership = ownership.join(*owners.get(child)?);
        }
        if let NodeKind::Local(level) = kind {
            ownership = ownership.join(Ownership::Trace(slots.get(*level)?.trace));
        }
        if model_read(kind) {
            let Ownership::Trace(component) = ownership else {
                return None;
            };
            reads.insert(projection.location(index)?.clone(), component);
        }
        owners.insert(index, ownership);
    }
    // Function bodies are separate projection contexts. A read in one has
    // no atom-slot owner, so it must never select the first component by
    // falling through the root context's component map.
    let mut index = 0;
    while let Some(kind) = projection.kind(index) {
        if model_read(kind) && !owners.contains_key(&index) {
            return None;
        }
        index += 1;
    }
    Some(reads)
}

fn model_read(kind: &NodeKind<usize, filament_checked::node::CheckedLiteral>) -> bool {
    matches!(
        kind,
        NodeKind::Attribute { .. }
            | NodeKind::History { .. }
            | NodeKind::Navigate { .. }
            | NodeKind::Reaches { .. }
            | NodeKind::ReachesEnd { .. }
            | NodeKind::AllInstances { .. }
            | NodeKind::Lookup { .. }
    )
}
