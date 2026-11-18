// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-940 "Package forms of the source-checked evaluators": the `holds`
//! conditions and the activation guard of a temporal clause, read from the
//! package's own nodes.
//!
//! A clause term is located by the clause node's key and, for a `holds`
//! condition, by the atom's use position in the clause's `formula_graph()`
//! pre-order (FR-515), which FR-515 "One evaluator formula" makes the source
//! formula's pre-order. The term's free binders are the clause's `over`
//! parameter, the trigger binder of an `on each` activation and the binders
//! of its captures, each classified by the clause the view reads. Nothing
//! here names a `CheckedPackage`, a `CheckedBody` or a source graph.

use std::collections::{BTreeMap, BTreeSet};

use filament_contract_model::{CheckedNodeId, CheckedSemanticNodeV2, CheckedTerm};
use filament_exact::NodeKey;
use filament_semantic_value::catalog::InternalFault;

use crate::bare::BarePackage;
use crate::bare_slot::BareSelectionFailure;
use crate::checked_v2::wire_graph;
use crate::checked_views::{temporal_subject, Activation, FormulaOperation};
use crate::protocol_selection::SlotSelectionFailure;
use crate::protocol_view::{group_member, text};
use crate::reader_protocol_projection::{
    BareWalker, ReaderProtocolProjection, UnsupportedView, WalkFailure,
};

/// Which binder of its temporal clause a clause term reads (FR-337 "Captures
/// at S3": `over` first, then the trigger binder, then the captures).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClauseBinder {
    /// The clause's `over` parameter.
    Over,
    /// The trigger binder of an `on each` activation.
    Trigger,
    /// The binder of the clause's capture at this source-order index.
    Capture(usize),
}

/// A projected clause term and the clause binder each of its local slots
/// reads.
pub struct BareClauseTerm {
    projection: ReaderProtocolProjection,
    binders: Vec<(usize, ClauseBinder)>,
}

impl BareClauseTerm {
    /// The term's evaluator nodes.
    pub fn projection(&self) -> &ReaderProtocolProjection {
        &self.projection
    }

    /// Each local slot the term reads with the clause binder bound there.
    pub fn binders(&self) -> &[(usize, ClauseBinder)] {
        &self.binders
    }
}

fn fault(invariant: &'static str) -> BareSelectionFailure {
    SlotSelectionFailure::Fault(InternalFault::new("FR-940", invariant)).into()
}

fn walk_failure(failure: WalkFailure) -> BareSelectionFailure {
    match failure {
        WalkFailure::Unsupported(view)
        | WalkFailure::FunctionParameterType { view, .. }
        | WalkFailure::FunctionResultType { view, .. } => {
            BareSelectionFailure::Unsupported(Box::new(view))
        }
        WalkFailure::Equality(refusal) => BareSelectionFailure::Equality(refusal),
        WalkFailure::Fault(failure) => SlotSelectionFailure::Fault(failure).into(),
    }
}

/// Which term of the clause to project.
enum ClauseTerm {
    Holds(usize),
    Guard,
}

impl<'p> BarePackage<'p> {
    /// The `holds` condition at use `position` of the `formula_graph()` of
    /// the temporal clause whose node key is `clause`, projected over the
    /// clause binders it reads. Charges nothing.
    ///
    /// # Errors
    ///
    /// [`BareSelectionFailure`]: a witness refusal, `MissingNode` for a key
    /// the package holds no node for, `MalformedNode` for a node that is no
    /// readable temporal clause or a position that is no `holds` use, or the
    /// first unsupported projection.
    pub fn project_holds(
        &self,
        clause: &NodeKey,
        position: usize,
    ) -> Result<BareClauseTerm, BareSelectionFailure> {
        self.project_clause_term(clause, ClauseTerm::Holds(position))?
            .ok_or_else(|| SlotSelectionFailure::MalformedNode(clause.clone()).into())
    }

    /// The `when` guard of the `on each` activation of the temporal clause
    /// whose node key is `clause`, projected over `over` and the trigger
    /// binder; `None` for a clause whose activation writes no guard.
    ///
    /// # Errors
    ///
    /// As [`Self::project_holds`].
    pub fn project_guard(
        &self,
        clause: &NodeKey,
    ) -> Result<Option<BareClauseTerm>, BareSelectionFailure> {
        self.project_clause_term(clause, ClauseTerm::Guard)
    }

    #[ix_trace_rs::string_edge]
    fn project_clause_term(
        &self,
        clause: &NodeKey,
        term: ClauseTerm,
    ) -> Result<Option<BareClauseTerm>, BareSelectionFailure> {
        let package = self.package();
        let (_supply, identities) = self.witness()?;
        let graph = wire_graph(package).map_err(|_| fault("bare-package-has-structural-keys"))?;
        let nodes: BTreeMap<&CheckedNodeId, &CheckedSemanticNodeV2> = package
            .resolved_graph()
            .nodes
            .iter()
            .map(|node| (&node.node_id, node))
            .collect();
        let mut clause_id = None;
        for id in nodes.keys() {
            let key = graph
                .key(id)
                .map_err(|_| fault("bare-package-has-structural-keys"))?;
            if key == *clause {
                clause_id = Some(*id);
                break;
            }
        }
        let clause_id =
            clause_id.ok_or_else(|| SlotSelectionFailure::MissingNode(clause.clone()))?;
        let malformed =
            || BareSelectionFailure::from(SlotSelectionFailure::MalformedNode(clause.clone()));
        let subject = temporal_subject(package, clause_id).map_err(|_| malformed())?;
        let (trigger, guard) = match subject.activation() {
            Activation::Origin => (None, None),
            Activation::Each { trigger, guard } => (Some(trigger), guard.as_ref()),
        };
        let root = match term {
            ClauseTerm::Holds(position) => {
                let graph_uses = subject.formula_graph();
                let entry = graph_uses.get(position).ok_or_else(malformed)?;
                if entry.kind() != Some(FormulaOperation::Holds) {
                    return Err(malformed());
                }
                let earlier = graph_uses
                    .iter()
                    .take(position)
                    .filter(|used| used.kind() == Some(FormulaOperation::Holds))
                    .count();
                subject
                    .predicate_leaves()
                    .get(earlier)
                    .ok_or_else(|| fault("bare-holds-use-has-a-predicate-leaf"))?
            }
            ClauseTerm::Guard => match guard {
                Some(guard) => guard,
                None => return Ok(None),
            },
        };
        // The term's free binders: the `value`/`parameter` nodes its
        // dependency closure reaches, less those it introduces itself
        // (FR-390 rule 3).
        let mut found: BTreeSet<&CheckedNodeId> = BTreeSet::new();
        let lookup = |id: &CheckedNodeId| nodes.get(id).copied();
        crate::introduced_binders::reach_term(
            &lookup,
            &CheckedTerm::Reference {
                target: root.clone(),
            },
            None,
            &mut |parameter| {
                found.insert(&parameter.node_id);
            },
        );
        let mut keys = Vec::with_capacity(found.len());
        let mut classes = Vec::with_capacity(found.len());
        for id in found {
            let class = if id == subject.over() {
                ClauseBinder::Over
            } else if Some(id) == trigger {
                ClauseBinder::Trigger
            } else {
                let binder = nodes
                    .get(id)
                    .copied()
                    .ok_or_else(|| fault("bare-binder-is-held"))?;
                let name = group_member(&binder.body, "name")
                    .and_then(text)
                    .ok_or_else(|| fault("bare-binder-has-a-name"))?;
                let index = subject
                    .capture_names()
                    .iter()
                    .position(|capture| capture == name)
                    .ok_or_else(|| fault("bare-clause-term-reads-only-clause-binders"))?;
                ClauseBinder::Capture(index)
            };
            keys.push(
                graph
                    .key(id)
                    .map_err(|_| fault("bare-package-has-structural-keys"))?,
            );
            classes.push(class);
        }
        let walker = BareWalker::new(
            package,
            identities,
            self.equality_decisions(),
            self.exact_type_projections(),
        )
        .map_err(SlotSelectionFailure::Fault)?;
        let root_key = graph
            .key(root)
            .map_err(|_| fault("bare-package-has-structural-keys"))?;
        let projection = walker
            .project_term(&root_key, &keys)
            .map_err(walk_failure)?;
        let world = self.world()?;
        if projection.has_unrepresented_types(world.types()) {
            return Err(BareSelectionFailure::Unsupported(Box::new(
                UnsupportedView::missing_type(package.name(), root_key),
            )));
        }
        let binders = keys
            .iter()
            .zip(classes)
            .map(|(key, class)| {
                projection
                    .local_slot(key)
                    .map(|slot| (slot, class))
                    .ok_or_else(|| fault("bare-clause-binder-has-a-local-slot"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Some(BareClauseTerm {
            projection,
            binders,
        }))
    }
}
