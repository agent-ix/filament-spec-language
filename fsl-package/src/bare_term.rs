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
        let clause_index =
            declaration_index(package, clause_id, "temporal_clause").ok_or_else(malformed)?;
        let (root, site) = match term {
            ClauseTerm::Holds(position) => {
                let graph_uses = subject.formula_graph();
                let entry = graph_uses.get(position).ok_or_else(malformed)?;
                if entry.kind() != Some(FormulaOperation::Holds) {
                    return Err(malformed());
                }
                // The source arena numbers each distinct formula node by its
                // first use in pre-order (FR-515 "One evaluator formula").
                let mut distinct: Vec<&CheckedNodeId> = Vec::new();
                for used in graph_uses.iter().take(position + 1) {
                    if !distinct.contains(&used.node) {
                        distinct.push(used.node);
                    }
                }
                let node = distinct
                    .iter()
                    .position(|held| *held == entry.node)
                    .ok_or_else(|| fault("bare-holds-use-is-a-formula-node"))?;
                let earlier = graph_uses
                    .iter()
                    .take(position)
                    .filter(|used| used.kind() == Some(FormulaOperation::Holds))
                    .count();
                let leaf = subject
                    .predicate_leaves()
                    .get(earlier)
                    .ok_or_else(|| fault("bare-holds-use-has-a-predicate-leaf"))?;
                (leaf, format!("temporal {clause_index} holds {node}"))
            }
            ClauseTerm::Guard => match guard {
                Some(guard) => (guard, format!("temporal {clause_index} guard")),
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
            .project_term(
                &root_key,
                &keys,
                filament_semantic_value::location::ExpressionOwner::TypeDeclaration { name: site },
            )
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

/// The operation an operation frame node is the frame of: the effective
/// identity of the declaring object type and the operation's name, read from
/// the `state`/`operation_anchor` node that binds the frame (FR-342), its
/// `context` resolved through the identity rows (FR-531).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BareFrameOperation {
    /// The declaring object type's effective identity.
    pub declaring: filament_exact::EffectiveId,
    /// The operation's declared name.
    pub operation: String,
}

impl<'p> BarePackage<'p> {
    /// The operation whose frame node key is `frame` (FR-940 "Package forms
    /// of the source-checked evaluators": the frame body read from the
    /// admitted checked nodes). Charges nothing.
    ///
    /// # Errors
    ///
    /// [`BareSelectionFailure`]: a witness refusal, `MissingNode` for a key
    /// no `state`/`frame` node of the package has, `MalformedNode` for a
    /// frame no anchor binds or an anchor whose context has no object
    /// identity row.
    #[ix_trace_rs::string_edge]
    pub fn frame_operation(
        &self,
        frame: &NodeKey,
    ) -> Result<BareFrameOperation, BareSelectionFailure> {
        let package = self.package();
        let (_supply, identities) = self.witness()?;
        let graph = wire_graph(package).map_err(|_| fault("bare-package-has-structural-keys"))?;
        let nodes = &package.resolved_graph().nodes;
        let mut frame_id = None;
        for node in nodes {
            if node.semantic_form.as_ref() != "frame" {
                continue;
            }
            let key = graph
                .key(&node.node_id)
                .map_err(|_| fault("bare-package-has-structural-keys"))?;
            if key == *frame {
                frame_id = Some(&node.node_id);
                break;
            }
        }
        let frame_id = frame_id.ok_or_else(|| SlotSelectionFailure::MissingNode(frame.clone()))?;
        let malformed =
            || BareSelectionFailure::from(SlotSelectionFailure::MalformedNode(frame.clone()));
        let anchor = nodes
            .iter()
            .filter(|node| node.semantic_form.as_ref() == "operation_anchor")
            .find(|node| {
                anchor_member(&node.body, "frame").and_then(crate::protocol_view::reference)
                    == Some(frame_id)
            })
            .ok_or_else(malformed)?;
        let context = anchor_member(&anchor.body, "context")
            .and_then(crate::protocol_view::reference)
            .ok_or_else(malformed)?;
        let operation = anchor_member(&anchor.body, "operation")
            .and_then(text)
            .ok_or_else(malformed)?;
        let declaring = identities
            .declaration_rows()
            .find(|row| row.structural_node == *context)
            .and_then(|row| match &row.exact_identity {
                filament_contract_model::SelectedExactIdentity::Object(identity) => {
                    Some(identity.clone())
                }
                _ => None,
            })
            .ok_or_else(malformed)?;
        Ok(BareFrameOperation {
            declaring,
            operation: operation.to_owned(),
        })
    }
}

/// The binding `name` of a `state`/`operation_anchor` node's body (FR-342):
/// its `context`, `operation` and `frame` members, in wire order.
fn anchor_member<'t>(term: &'t CheckedTerm, name: &str) -> Option<&'t CheckedTerm> {
    let CheckedTerm::OperationAnchorBody(body) = term else {
        return None;
    };
    body.members.iter().find_map(|member| match member {
        CheckedTerm::Binding { name: bound, value } if &**bound == name => Some(&**value),
        _ => None,
    })
}

/// A `state`/`state_clause` node read for evaluation (FR-940 "Package forms
/// of the source-checked evaluators": the invariant, precondition and
/// postcondition bodies from the admitted checked nodes).
pub struct BareStateClause {
    observation: fsl_semantics::model::clause_observation::Observation,
    parameters: Vec<(String, usize)>,
    projection: ReaderProtocolProjection,
}

impl BareStateClause {
    /// The observation the clause's reads take (FR-104): `current` for an
    /// invariant or constraint, `pre` for a precondition, `post` for a
    /// postcondition.
    pub fn observation(&self) -> fsl_semantics::model::clause_observation::Observation {
        self.observation
    }

    /// The clause's parameters in declaration order, `self` first, each
    /// with the local slot the body reads it through.
    pub fn parameters(&self) -> &[(String, usize)] {
        &self.parameters
    }

    /// The clause body's evaluator nodes.
    pub fn projection(&self) -> &ReaderProtocolProjection {
        &self.projection
    }
}

impl<'p> BarePackage<'p> {
    /// The state clause whose node key is `clause`, its body projected over
    /// its parameters and located under the state clause site `name`
    /// declares (`ExpressionOwner::StateClause`), its index the clause's
    /// rank among the unit's state clause declarations in source order.
    /// Charges nothing.
    ///
    /// The package names no state clause until FSpec FR-322 `claim_names`
    /// is admitted (QSL-2410), so `name` is the caller's.
    ///
    /// # Errors
    ///
    /// [`BareSelectionFailure`]: a witness refusal, `MissingNode` for a key
    /// the package holds no node for, `MalformedNode` for a node that is no
    /// state clause, or the first unsupported projection.
    #[ix_trace_rs::string_edge]
    pub fn project_state_clause(
        &self,
        clause: &NodeKey,
        name: &str,
    ) -> Result<BareStateClause, BareSelectionFailure> {
        use filament_contract_model::{CheckedOperationMember, StateClauseKind};
        use fsl_semantics::model::clause_observation::Observation;
        let package = self.package();
        let (_supply, identities) = self.witness()?;
        let graph = wire_graph(package).map_err(|_| fault("bare-package-has-structural-keys"))?;
        let mut found = None;
        for node in &package.resolved_graph().nodes {
            let key = graph
                .key(&node.node_id)
                .map_err(|_| fault("bare-package-has-structural-keys"))?;
            if key == *clause {
                found = Some(node);
                break;
            }
        }
        let node = found.ok_or_else(|| SlotSelectionFailure::MissingNode(clause.clone()))?;
        let malformed =
            || BareSelectionFailure::from(SlotSelectionFailure::MalformedNode(clause.clone()));
        let CheckedTerm::Application {
            operation,
            arguments,
            ..
        } = &node.body
        else {
            return Err(malformed());
        };
        let read = operation.read().map_err(|_| malformed())?;
        let Some(CheckedOperationMember::StateClause { clause: kind }) = read.member else {
            return Err(malformed());
        };
        let observation = match kind {
            StateClauseKind::Invariant | StateClauseKind::Constraint => Observation::Current,
            StateClauseKind::Precondition => Observation::Pre,
            StateClauseKind::Postcondition => Observation::Post,
        };
        let [CheckedTerm::Aggregate { members }, _anchor, CheckedTerm::Reference { target: body }] =
            arguments.as_slice()
        else {
            return Err(malformed());
        };
        let nodes: BTreeMap<&CheckedNodeId, &CheckedSemanticNodeV2> = package
            .resolved_graph()
            .nodes
            .iter()
            .map(|node| (&node.node_id, node))
            .collect();
        let mut keys = Vec::with_capacity(members.len());
        let mut names = Vec::with_capacity(members.len());
        for member in members {
            let CheckedTerm::Reference { target } = member else {
                return Err(malformed());
            };
            let parameter = nodes.get(target).ok_or_else(malformed)?;
            let parameter_name = group_member(&parameter.body, "name")
                .and_then(text)
                .ok_or_else(|| fault("bare-binder-has-a-name"))?;
            names.push(parameter_name.to_owned());
            keys.push(
                graph
                    .key(target)
                    .map_err(|_| fault("bare-package-has-structural-keys"))?,
            );
        }
        let index =
            declaration_index(package, &node.node_id, "state_clause").ok_or_else(malformed)?;
        let walker = BareWalker::new(
            package,
            identities,
            self.equality_decisions(),
            self.exact_type_projections(),
        )
        .map_err(SlotSelectionFailure::Fault)?;
        let root_key = graph
            .key(body)
            .map_err(|_| fault("bare-package-has-structural-keys"))?;
        let projection = walker
            .project_term(
                &root_key,
                &keys,
                filament_semantic_value::location::ExpressionOwner::StateClause {
                    clause: name.to_owned(),
                    index,
                },
            )
            .map_err(walk_failure)?;
        let world = self.world()?;
        if projection.has_unrepresented_types(world.types()) {
            return Err(BareSelectionFailure::Unsupported(Box::new(
                UnsupportedView::missing_type(package.name(), root_key),
            )));
        }
        let parameters = names
            .into_iter()
            .zip(&keys)
            .map(|(name, key)| {
                projection
                    .local_slot(key)
                    .map(|slot| (name, slot))
                    .ok_or_else(|| fault("bare-clause-binder-has-a-local-slot"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(BareStateClause {
            observation,
            parameters,
            projection,
        })
    }
}

/// The rank of `node`'s first `claim` occurrence among every `claim`
/// occurrence of the nodes of `form` in the same source, by the start of
/// their source-map regions: the node's declaration index in source order.
fn declaration_index(
    package: &filament_contract_model::CheckedPackageV2,
    node: &CheckedNodeId,
    form: &str,
) -> Option<usize> {
    use filament_contract_model::CheckedOccurrenceRole;
    let of_form: BTreeSet<&CheckedNodeId> = package
        .resolved_graph()
        .nodes
        .iter()
        .filter(|candidate| candidate.semantic_form.as_ref() == form)
        .map(|candidate| &candidate.node_id)
        .collect();
    let claims: Vec<&filament_contract_model::CheckedSourceMapEntry> = package
        .source_map()
        .iter()
        .filter(|entry| {
            entry.role == CheckedOccurrenceRole::Claim && of_form.contains(&entry.node_id)
        })
        .collect();
    let own = claims
        .iter()
        .filter(|entry| entry.node_id == *node)
        .filter_map(|entry| entry.regions.first())
        .min_by_key(|region| region.start)?;
    Some(
        claims
            .iter()
            .filter_map(|entry| entry.regions.first())
            .filter(|region| region.source == own.source && region.start < own.start)
            .count(),
    )
}

/// Why the package form of a temporal clause was not projected (FR-940).
#[derive(Debug)]
pub enum BareClauseRefusal {
    /// The clause is not selected: a witness refusal, a key no temporal
    /// clause has, a node that is no readable clause.
    Selection(BareSelectionFailure),
    /// The clause carries a fact its package form does not read yet; no
    /// form is built without it.
    Pending {
        /// The fact.
        fact: &'static str,
        /// The node that carries it.
        node: CheckedNodeId,
    },
}

impl From<BareSelectionFailure> for BareClauseRefusal {
    fn from(failure: BareSelectionFailure) -> Self {
        Self::Selection(failure)
    }
}

impl<'p> BarePackage<'p> {
    /// FR-940: the package form of the temporal clause whose node key is
    /// `clause`, read from the package's admitted nodes, source map, name
    /// rows and type projections: its name, span, profile, `over`, trigger
    /// and capture binders, clock role, activation and formula arena. Its
    /// population domains are the caller's: they are read from the
    /// supplied domain packages, which a package does not hold. Charges
    /// nothing.
    ///
    /// # Errors
    ///
    /// [`BareClauseRefusal::Selection`] for a clause the package does not
    /// select; [`BareClauseRefusal::Pending`] for a fact the form does not
    /// read yet: a fairness set, an interval, an aggregate atom or a
    /// formula node used twice.
    #[ix_trace_rs::string_edge]
    pub fn temporal_clause_form(
        &self,
        clause: &NodeKey,
    ) -> Result<fsl_semantics::check::PackageClauseForm, BareClauseRefusal> {
        use fsl_semantics::check::{BinaryTemporal, PackageFormulaNode, UnaryTemporal};
        let package = self.package();
        self.witness()?;
        let graph = wire_graph(package).map_err(|_| fault("bare-package-has-structural-keys"))?;
        let nodes: BTreeMap<&CheckedNodeId, &CheckedSemanticNodeV2> = package
            .resolved_graph()
            .nodes
            .iter()
            .map(|node| (&node.node_id, node))
            .collect();
        let missing =
            || BareSelectionFailure::from(SlotSelectionFailure::MissingNode(clause.clone()));
        let malformed =
            || BareSelectionFailure::from(SlotSelectionFailure::MalformedNode(clause.clone()));
        let mut found = None;
        for node in nodes.values() {
            if node.semantic_form.as_ref() != "temporal_clause" {
                continue;
            }
            let key = graph
                .key(&node.node_id)
                .map_err(|_| fault("bare-package-has-structural-keys"))?;
            if key == *clause {
                found = Some(*node);
                break;
            }
        }
        let node = found.ok_or_else(missing)?;
        let view = temporal_subject(package, &node.node_id).map_err(|_| malformed())?;
        let name = view.name().ok_or_else(malformed)?.to_owned();
        let CheckedTerm::Application { arguments, .. } = &node.body else {
            return Err(malformed().into());
        };
        // FR-337 "Clause emission": `over`, clock, activation, captures,
        // fairness, formula.
        let [_, clock, _, _, CheckedTerm::Aggregate { members: fairness }, _] =
            arguments.as_slice()
        else {
            return Err(malformed().into());
        };
        if !fairness.is_empty() {
            return Err(BareClauseRefusal::Pending {
                fact: "fairness",
                node: node.node_id.clone(),
            });
        }
        let clock = text(clock).ok_or_else(malformed)?.to_owned();
        let span = view
            .source_span()
            .regions
            .first()
            .map(region_span)
            .ok_or_else(malformed)?;
        let binder =
            |id: &CheckedNodeId| -> Result<(String, filament_exact::ValueType), BareClauseRefusal> {
                let parameter = nodes.get(id).ok_or_else(malformed)?;
                let name = group_member(&parameter.body, "name")
                    .and_then(text)
                    .ok_or_else(malformed)?;
                let value_type = self
                    .exact_type_projections()
                    .get(id)
                    .and_then(|projected| projected.as_ref().ok())
                    .map(|projected| projected.value_type().clone())
                    .ok_or_else(malformed)?;
                Ok((name.to_owned(), value_type))
            };
        let over = binder(view.over())?;
        let (activation, trigger) = match view.activation() {
            Activation::Origin => (fsl_semantics::check::Activation::Origin, None),
            Activation::Each { trigger, guard } => (
                fsl_semantics::check::Activation::Each {
                    guarded: guard.is_some(),
                },
                Some(binder(trigger)?),
            ),
        };
        let mut captures = Vec::with_capacity(view.captures().len());
        for (initializer, capture) in view.captures().iter().zip(view.capture_names()) {
            let value_type = self
                .exact_type_projections()
                .get(initializer)
                .and_then(|projected| projected.as_ref().ok())
                .map(|projected| projected.value_type().clone())
                .ok_or_else(malformed)?;
            captures.push((capture.clone(), value_type));
        }
        let uses = view.formula_graph();
        let mut formula = Vec::with_capacity(uses.len());
        let mut seen = BTreeSet::new();
        for entry in uses {
            let pending = |fact| BareClauseRefusal::Pending {
                fact,
                node: entry.node.clone(),
            };
            if !seen.insert(entry.node) {
                return Err(pending("shared-formula-node"));
            }
            if entry.interval.is_some() {
                return Err(pending("interval"));
            }
            let operand =
                |position: usize| entry.operands.get(position).copied().ok_or_else(malformed);
            let kind = entry.kind().ok_or_else(malformed)?;
            formula.push(match kind {
                FormulaOperation::True => PackageFormulaNode::Constant(true),
                FormulaOperation::False => PackageFormulaNode::Constant(false),
                FormulaOperation::Holds => PackageFormulaNode::Holds {
                    span: package
                        .source_map()
                        .iter()
                        .find(|map| map.node_id == *entry.node)
                        .and_then(|map| map.regions.first())
                        .map(region_span)
                        .ok_or_else(malformed)?,
                },
                FormulaOperation::Aggregate => return Err(pending("aggregate")),
                FormulaOperation::Not => PackageFormulaNode::Not(operand(0)?),
                FormulaOperation::And => PackageFormulaNode::And(operand(0)?, operand(1)?),
                FormulaOperation::Or => PackageFormulaNode::Or(operand(0)?, operand(1)?),
                FormulaOperation::Implies => PackageFormulaNode::Implies(operand(0)?, operand(1)?),
                FormulaOperation::Eventually
                | FormulaOperation::Always
                | FormulaOperation::Once
                | FormulaOperation::Historically => PackageFormulaNode::Unary {
                    operator: match kind {
                        FormulaOperation::Eventually => UnaryTemporal::Eventually,
                        FormulaOperation::Always => UnaryTemporal::Always,
                        FormulaOperation::Once => UnaryTemporal::Once,
                        _ => UnaryTemporal::Historically,
                    },
                    operand: operand(0)?,
                },
                FormulaOperation::Until
                | FormulaOperation::Release
                | FormulaOperation::Since
                | FormulaOperation::Triggered => PackageFormulaNode::Binary {
                    operator: match kind {
                        FormulaOperation::Until => BinaryTemporal::Until,
                        FormulaOperation::Release => BinaryTemporal::Release,
                        FormulaOperation::Since => BinaryTemporal::Since,
                        _ => BinaryTemporal::Triggered,
                    },
                    left: operand(0)?,
                    right: operand(1)?,
                },
            });
        }
        let profile = fsl_semantics::check::TemporalProfile::from_identity(
            &view.profile().definition.identity,
        )
        .ok_or_else(malformed)?;
        Ok(fsl_semantics::check::PackageClauseForm {
            name,
            span,
            profile,
            package_name: package.name().to_owned(),
            over,
            clock,
            activation,
            trigger,
            captures,
            formula,
            populations: Vec::new(),
        })
    }
}

fn region_span(region: &filament_contract_model::CheckedSourceRegion) -> fsl_foundation::Span {
    fsl_foundation::Span {
        start: usize::try_from(region.start).unwrap_or(usize::MAX),
        end: usize::try_from(region.end).unwrap_or(usize::MAX),
    }
}
