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

use filament_contract_model::{
    CheckedFairnessGranularity, CheckedFairnessKind, CheckedFairnessTarget, CheckedNodeId,
    CheckedNodeKind, CheckedOperationMember, CheckedSemanticNodeV2, CheckedTerm, NativeJoinBudget,
    NativeJoinLocus, SelectedExactIdentity, TemporalForm,
};
use filament_exact::{EffectiveId, Identifier, NodeKey};
use filament_semantic_value::catalog::InternalFault;
use filament_semantic_value::fairness::{
    FairnessConstraint, FairnessGranularity, FairnessKind, FairnessOperation,
};

use crate::bare::BarePackage;
use crate::bare_slot::BareSelectionFailure;
use crate::checked_v2::wire_graph;
use crate::checked_views::{Activation, FormulaOperation, temporal_subject};
use crate::native_row::{self, NativeRowError};
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
    names: Vec<String>,
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

    /// Each local slot the term reads with the declared name of the clause
    /// binder bound there, in the order of [`Self::binders`].
    pub fn named_binders(&self) -> impl Iterator<Item = (usize, &str)> {
        self.binders
            .iter()
            .zip(&self.names)
            .map(|((slot, _), name)| (*slot, name.as_str()))
    }
}

pub(crate) fn fault(invariant: &'static str) -> BareSelectionFailure {
    SlotSelectionFailure::Fault(InternalFault::new("FR-940", invariant)).into()
}

pub(crate) fn walk_failure(failure: WalkFailure) -> BareSelectionFailure {
    match failure {
        WalkFailure::Unsupported(view) => BareSelectionFailure::Unsupported(Box::new(view)),
        WalkFailure::FunctionParameterType { view, .. }
        | WalkFailure::FunctionResultType { view, .. } => BareSelectionFailure::Unsupported(view),
        WalkFailure::Equality(refusal) => BareSelectionFailure::Equality(refusal),
        WalkFailure::Fault(failure) => SlotSelectionFailure::Fault(failure).into(),
    }
}

/// Which term of the clause to project.
enum ClauseTerm {
    Holds(usize),
    Guard,
}

/// A fairness projection preserves both the original clause refusal and a
/// denial on the invocation's cumulative native work/storage ledger.
#[derive(Debug, thiserror::Error)]
pub enum BareFairnessRefusal {
    /// The clause or fairness target has no admitted package form.
    #[error("fairness clause refused: {0:?}")]
    Clause(BareClauseRefusal),
    /// The existing native allowance was exhausted before the read or copy.
    #[error(transparent)]
    Native(#[from] NativeRowError),
    /// The admitted structural identity could not be materialized.
    #[error(transparent)]
    Structural(#[from] fsl_semantics::check::wire_id::WireIdError),
    /// A prepaid structural-key reservation failed.
    #[error(transparent)]
    Allocation(#[from] std::collections::TryReserveError),
}

impl From<BareClauseRefusal> for BareFairnessRefusal {
    fn from(refusal: BareClauseRefusal) -> Self {
        Self::Clause(refusal)
    }
}

/// Complete clause fairness and the exact joined identity used at construction.
/// Borrowed identities price later copies without substituting receiver types.
pub struct BareClauseConstraints<'a> {
    constraints: Vec<fsl_semantics::check::ClauseConstraint>,
    declaring: Vec<&'a EffectiveId>,
}
impl BareClauseConstraints<'_> {
    /// Actual constraints in admitted fairness-member order.
    pub fn constraints(&self) -> &[fsl_semantics::check::ClauseConstraint] {
        &self.constraints
    }
    /// Exact identity that constructed the constraint at this position.
    pub fn declaring_at(&self, position: usize) -> Option<&EffectiveId> {
        self.declaring.get(position).copied()
    }
}

/// A batch clause-form read keeps admitted-view, existing form and native
/// refusals distinct; no denial is converted into a semantic pending fact.
#[derive(Debug, thiserror::Error)]
pub enum BareClauseFormRefusal {
    /// An existing package clause-form refusal.
    #[error("package clause form refused: {0:?}")]
    Form(BareClauseRefusal),
    /// The requested naming row or clause shape is not owned by this package.
    #[error(transparent)]
    View(#[from] crate::checked_views::ViewRefusal),
    /// The invocation's original native allowance was exhausted.
    #[error(transparent)]
    Native(#[from] NativeRowError),
    /// A paid body reservation failed.
    #[error("package clause body allocation failed: {0}")]
    Allocation(#[from] std::collections::TryReserveError),
}

impl From<BareClauseRefusal> for BareClauseFormRefusal {
    fn from(refusal: BareClauseRefusal) -> Self {
        Self::Form(refusal)
    }
}
impl From<crate::checked_views::ClauseBodyRefusal> for BareClauseFormRefusal {
    fn from(refusal: crate::checked_views::ClauseBodyRefusal) -> Self {
        match refusal {
            crate::checked_views::ClauseBodyRefusal::View(refusal) => Self::View(refusal),
            crate::checked_views::ClauseBodyRefusal::Native(refusal) => Self::Native(refusal),
            crate::checked_views::ClauseBodyRefusal::Allocation(refusal) => {
                Self::Allocation(refusal)
            }
        }
    }
}

struct FormParts<'a, 'p> {
    over: &'p CheckedNodeId,
    activation: &'a Activation,
    captures: &'a [CheckedNodeId],
    capture_names: &'a [String],
    formula: &'a [crate::checked_views::FormulaNodeUse<'p>],
    profile: &'p filament_contract_model::CheckedSelection,
}

struct FormResolution<'n, 'a, W: FormWork> {
    lookup: &'a dyn Fn(&CheckedNodeId) -> Option<&'n CheckedSemanticNodeV2>,
    intervals: bool,
    malformed: &'a dyn Fn() -> BareClauseRefusal,
    atom_span: &'a mut (
        dyn for<'p, 'entry> FnMut(
                usize,
                &'entry crate::checked_views::FormulaNodeUse<'p>,
                &mut W,
            ) -> Result<fsl_foundation::Span, W::Error>
            + 'a,
    ),
    work: &'a mut W,
}

trait FairnessWork {
    type Error: From<BareClauseRefusal>;
    fn visit(&mut self) -> Result<(), Self::Error>;
    fn items(&mut self, count: usize) -> Result<(), Self::Error>;
    fn text(&mut self, value: &str) -> Result<(), Self::Error>;
    fn node(&mut self, value: &CheckedNodeId) -> Result<(), Self::Error>;
    fn effective(&mut self, value: &EffectiveId) -> Result<(), Self::Error> {
        self.items(value.owner_chain().len())?;
        for original in value.owner_chain() {
            self.text(original.package())?;
            self.text(original.node())?;
        }
        Ok(())
    }
}

struct UnmeteredFairness;

impl FairnessWork for UnmeteredFairness {
    type Error = BareClauseRefusal;
    fn visit(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
    fn items(&mut self, _: usize) -> Result<(), Self::Error> {
        Ok(())
    }
    fn text(&mut self, _: &str) -> Result<(), Self::Error> {
        Ok(())
    }
    fn node(&mut self, _: &CheckedNodeId) -> Result<(), Self::Error> {
        Ok(())
    }
}

struct MeteredFairness<'a> {
    budget: &'a mut NativeJoinBudget,
    locus: NativeJoinLocus,
}

impl FairnessWork for MeteredFairness<'_> {
    type Error = BareFairnessRefusal;
    fn visit(&mut self) -> Result<(), Self::Error> {
        Ok(native_row::work(self.budget, self.locus, 1)?)
    }
    fn items(&mut self, count: usize) -> Result<(), Self::Error> {
        Ok(native_row::items(self.budget, self.locus, count)?)
    }
    fn text(&mut self, value: &str) -> Result<(), Self::Error> {
        Ok(native_row::text(self.budget, self.locus, value)?)
    }
    fn node(&mut self, value: &CheckedNodeId) -> Result<(), Self::Error> {
        Ok(native_row::node(self.budget, self.locus, value)?)
    }
}

trait FormWork: FairnessWork {
    fn value_type(&mut self, value: &filament_exact::ValueType) -> Result<(), Self::Error>;
    fn interval(&mut self, value: &crate::CheckedOperationInterval) -> Result<(), Self::Error>;
}
impl FormWork for UnmeteredFairness {
    fn value_type(&mut self, _: &filament_exact::ValueType) -> Result<(), Self::Error> {
        Ok(())
    }
    fn interval(&mut self, _: &crate::CheckedOperationInterval) -> Result<(), Self::Error> {
        Ok(())
    }
}
struct MeteredForm<'a> {
    budget: &'a mut NativeJoinBudget,
    locus: NativeJoinLocus,
}
impl FairnessWork for MeteredForm<'_> {
    type Error = BareClauseFormRefusal;
    fn visit(&mut self) -> Result<(), Self::Error> {
        Ok(native_row::work(self.budget, self.locus, 1)?)
    }
    fn items(&mut self, count: usize) -> Result<(), Self::Error> {
        Ok(native_row::items(self.budget, self.locus, count)?)
    }
    fn text(&mut self, text: &str) -> Result<(), Self::Error> {
        Ok(native_row::text(self.budget, self.locus, text)?)
    }
    fn node(&mut self, node: &CheckedNodeId) -> Result<(), Self::Error> {
        Ok(native_row::node(self.budget, self.locus, node)?)
    }
}
impl FormWork for MeteredForm<'_> {
    fn value_type(&mut self, value: &filament_exact::ValueType) -> Result<(), Self::Error> {
        Ok(native_row::value_type(self.budget, self.locus, value)?)
    }
    fn interval(&mut self, value: &crate::CheckedOperationInterval) -> Result<(), Self::Error> {
        self.visit()?;
        if let crate::CheckedOperationInterval::Timed { lower, upper, .. } = value {
            for bound in std::iter::once(lower).chain(upper) {
                native_row::integer(self.budget, self.locus, &bound.numerator)?;
                native_row::integer(self.budget, self.locus, &bound.denominator)?;
            }
        }
        Ok(())
    }
}

impl<'p> BarePackage<'p> {
    /// Read a document-held temporal clause's model-operation fairness from
    /// admitted operation members and this owner's joined identity rows.
    /// Every visited member/identity row and retained copy is charged to the
    /// invocation's existing native allowance before it is read or copied.
    pub fn temporal_clause_fairness_with_budget(
        &self,
        declaration: &CheckedNodeId,
        budget: &mut NativeJoinBudget,
    ) -> Result<Vec<FairnessConstraint>, BareFairnessRefusal> {
        let locus = native_row::start(budget, "temporal_fairness")?;
        let mut work = MeteredFairness { budget, locus };
        work.visit()?;
        if self.package().node_kind(declaration)
            != Some(CheckedNodeKind::Temporal(TemporalForm::TemporalClause))
        {
            return Err(BareClauseRefusal::from(fault("fairness-clause-is-admitted")).into());
        }
        let term = self
            .package()
            .term(declaration)
            .ok_or_else(|| BareClauseRefusal::from(fault("fairness-clause-is-admitted")))?;
        self.read_clause_fairness(declaration, term, &mut work)
    }

    /// Read complete operation fairness facts from the actual owning clause.
    /// Context identities and operation names come from admitted fairness
    /// targets, alongside the exact joined object identity. Every conversion
    /// and retained copy consumes the caller's existing native allowance.
    pub fn temporal_clause_constraints_with_budget<'a>(
        &'a self,
        declaration: &CheckedNodeId,
        budget: &mut NativeJoinBudget,
    ) -> Result<BareClauseConstraints<'a>, BareFairnessRefusal> {
        use fsl_semantics::check::wire_id::{StructuralKeyMaterializationError, StructuralKeys};
        let locus = native_row::start(budget, "temporal_clause_constraints")?;
        let mut work = MeteredFairness { budget, locus };
        work.visit()?;
        if self.package().node_kind(declaration)
            != Some(CheckedNodeKind::Temporal(TemporalForm::TemporalClause))
        {
            return Err(BareClauseRefusal::from(fault("fairness-clause-is-admitted")).into());
        }
        let term = self
            .package()
            .term(declaration)
            .ok_or_else(|| BareClauseRefusal::from(fault("fairness-clause-is-admitted")))?;
        let mut keys = StructuralKeys::default();
        let CheckedTerm::Application { arguments, .. } = term else {
            return Err(BareClauseRefusal::from(fault("fairness-clause-is-admitted")).into());
        };
        let [_, _, _, _, CheckedTerm::Aggregate { members, .. }, _] = arguments.as_slice() else {
            return Err(BareClauseRefusal::from(fault("fairness-clause-is-admitted")).into());
        };
        work.visit()?;
        work.items(members.len())?;
        let mut declaring = Vec::new();
        declaring.try_reserve_exact(members.len())?;
        let constraints = self.read_clause_fairness_with(
            declaration,
            term,
            &mut work,
            |context, name, identity, constraint, work| {
                let node = match keys.try_key(context, work.budget, work.locus) {
                    Ok(node) => node,
                    Err(StructuralKeyMaterializationError::Wire(error)) => return Err(error.into()),
                    Err(StructuralKeyMaterializationError::Allocation(error)) => {
                        return Err(error.into());
                    }
                    Err(StructuralKeyMaterializationError::Budget { denied, .. }) => {
                        return Err(NativeRowError::exhausted(denied, work.budget).into());
                    }
                };
                // Identifier owns a separate copy from FairnessOperation.
                work.text(name)?;
                let operation = Identifier::new(name.to_owned()).map_err(|_| {
                    BareClauseRefusal::from(fault("fairness-operation-name-is-admitted"))
                })?;
                work.visit()?;
                declaring.push(identity);
                Ok(fsl_semantics::check::ClauseConstraint {
                    constraint,
                    context: node,
                    operation,
                })
            },
        )?;
        Ok(BareClauseConstraints {
            constraints,
            declaring,
        })
    }

    fn read_clause_fairness<W: FairnessWork>(
        &self,
        declaration: &CheckedNodeId,
        term: &CheckedTerm,
        work: &mut W,
    ) -> Result<Vec<FairnessConstraint>, W::Error> {
        self.read_clause_fairness_with(declaration, term, work, |_, _, _, constraint, _| {
            Ok(constraint)
        })
    }

    fn read_clause_fairness_with<'a, W: FairnessWork, T>(
        &'a self,
        declaration: &CheckedNodeId,
        term: &CheckedTerm,
        work: &mut W,
        mut retain: impl FnMut(
            &CheckedNodeId,
            &str,
            &'a EffectiveId,
            FairnessConstraint,
            &mut W,
        ) -> Result<T, W::Error>,
    ) -> Result<Vec<T>, W::Error> {
        let malformed = || BareClauseRefusal::from(fault("fairness-member-is-admitted"));
        let (_, identities) = self.witness().map_err(BareClauseRefusal::from)?;
        let CheckedTerm::Application { arguments, .. } = term else {
            return Err(malformed().into());
        };
        let [_, _, _, _, CheckedTerm::Aggregate { members, .. }, _] = arguments.as_slice() else {
            return Err(malformed().into());
        };
        work.items(members.len())?;
        let mut fairness = Vec::with_capacity(members.len());
        for member in members {
            work.visit()?;
            let CheckedTerm::Reference { target } = member else {
                return Err(malformed().into());
            };
            let Some(CheckedTerm::Application { operation, .. }) = self.package().term(target)
            else {
                return Err(malformed().into());
            };
            let read = operation.read().map_err(|_| malformed())?;
            let Some(CheckedOperationMember::Fairness {
                fairness_kind,
                granularity,
                target,
            }) = read.member
            else {
                return Err(malformed().into());
            };
            let CheckedFairnessTarget::Operation {
                declaration: context,
                name,
            } = target
            else {
                work.node(declaration)?;
                return Err(BareClauseRefusal::Pending {
                    fact: "fairness-target",
                    node: declaration.clone(),
                }
                .into());
            };
            let structural = self
                .package()
                .structural_key(context)
                .ok_or_else(malformed)?;
            let mut declaring = None;
            for row in identities.declaration_rows() {
                work.visit()?;
                if row.structural_node == *structural {
                    let SelectedExactIdentity::Object(owner) = &row.exact_identity else {
                        return Err(malformed().into());
                    };
                    declaring = Some(owner);
                    break;
                }
            }
            let declaring = declaring.ok_or_else(malformed)?;
            work.effective(declaring)?;
            work.text(name)?;
            let constraint = FairnessConstraint::new(
                match fairness_kind {
                    CheckedFairnessKind::Weak => FairnessKind::Weak,
                    CheckedFairnessKind::Strong => FairnessKind::Strong,
                },
                FairnessOperation::new(declaring.clone(), name.to_string()),
                match granularity {
                    CheckedFairnessGranularity::Whole => FairnessGranularity::Whole,
                    CheckedFairnessGranularity::Each => FairnessGranularity::Each,
                },
            );
            fairness.push(retain(structural, name, declaring, constraint, work)?);
        }
        Ok(fairness)
    }

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
        self.project_clause_term(clause, ClauseTerm::Holds(position), false)?
            .ok_or_else(|| SlotSelectionFailure::MalformedNode(clause.clone()).into())
    }

    /// [`Self::project_holds`] read to print the condition (FR-931 "The
    /// lowering input"): printing is not evaluation, so the walk reads names
    /// and term structure from the package alone and a node whose exact type
    /// the identity rows do not select is typed by its own declaration node.
    /// The package world is neither built nor consulted.
    ///
    /// # Errors
    ///
    /// As [`Self::project_holds`], less every refusal that is a missing
    /// identity row.
    pub fn project_holds_for_print(
        &self,
        clause: &NodeKey,
        position: usize,
    ) -> Result<BareClauseTerm, BareSelectionFailure> {
        self.project_clause_term(clause, ClauseTerm::Holds(position), true)?
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
        self.project_clause_term(clause, ClauseTerm::Guard, false)
    }

    #[ix_trace_rs::string_edge]
    fn project_clause_term(
        &self,
        clause: &NodeKey,
        term: ClauseTerm,
        print: bool,
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
        let guard_term = matches!(term, ClauseTerm::Guard);
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
        let mut names = Vec::with_capacity(found.len());
        for id in found {
            let binder = nodes
                .get(id)
                .copied()
                .ok_or_else(|| fault("bare-binder-is-held"))?;
            let name = group_member(&binder.body, "name")
                .and_then(text)
                .ok_or_else(|| fault("bare-binder-has-a-name"))?;
            names.push(name.to_owned());
            let class = if id == subject.over() {
                ClauseBinder::Over
            } else if Some(id) == trigger {
                ClauseBinder::Trigger
            } else {
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
            self.dependency_views(),
        )
        .map_err(SlotSelectionFailure::Fault)?
        .with_populations(self);
        let walker = if print { walker.printing() } else { walker };
        let root_key = graph
            .key(root)
            .map_err(|_| fault("bare-package-has-structural-keys"))?;
        // The checker numbers a binder an expression introduces after every
        // binder the term is typed over, read or not: `over` and the trigger
        // for the guard, and also each capture for a `holds` condition. The
        // slots given are those the term reads.
        let clause_binders = if guard_term {
            1 + usize::from(trigger.is_some())
        } else {
            1 + usize::from(trigger.is_some()) + subject.capture_names().len()
        };
        let projection = walker
            .project_term_at(
                &root_key,
                &keys,
                clause_binders.saturating_sub(keys.len()),
                filament_semantic_value::location::ExpressionOwner::TypeDeclaration { name: site },
            )
            .map_err(walk_failure)?;
        if !print {
            let world = self.world()?;
            if projection.has_unrepresented_types(world.types()) {
                return Err(BareSelectionFailure::Unsupported(Box::new(
                    UnsupportedView::missing_type(package.name(), root_key),
                )));
            }
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
            names,
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
    /// The declaring type and operation selected by a protocol attempt's
    /// admitted `operation_anchor` node (FR-531/FR-940).
    pub fn protocol_operation(
        &self,
        anchor_id: &CheckedNodeId,
    ) -> Result<BareFrameOperation, BareSelectionFailure> {
        let package = self.package();
        let (_supply, identities) = self.witness()?;
        let nodes = &package.resolved_graph().nodes;
        let malformed = || fault("protocol-operation-anchor-is-admitted");
        let graph = wire_graph(package).map_err(|_| malformed())?;
        let anchor_key = graph.key(anchor_id).map_err(|_| malformed())?;
        let anchor = nodes
            .iter()
            .find(|node| {
                node.semantic_form.as_ref() == "operation_anchor"
                    && graph
                        .key(&node.node_id)
                        .map(|key| key == anchor_key)
                        .unwrap_or(false)
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

    /// The exact object type selected by one protocol role's model node
    /// (FR-531 static-role selection).
    pub fn protocol_role_model(
        &self,
        model: &CheckedNodeId,
    ) -> Result<EffectiveId, BareSelectionFailure> {
        let (_supply, identities) = self.witness()?;
        identities
            .declaration_rows()
            .find(|row| row.structural_node == *model)
            .and_then(|row| match &row.exact_identity {
                filament_contract_model::SelectedExactIdentity::Object(identity) => {
                    Some(identity.clone())
                }
                _ => None,
            })
            .ok_or_else(|| fault("protocol-role-model-is-selected"))
    }

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

/// One `state`/`state_clause` declaration as a model check reads it (FR-107,
/// FR-940): its name from its `claim_names` row (FSpec FR-322), its node, the
/// object type its `self` parameter is over, the operation its anchor names
/// and the observation its kind reads.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BareStateClauseDeclaration {
    /// The declared name, from the declaration's `claim_names` row.
    pub name: String,
    /// The clause node's key.
    pub identity: NodeKey,
    /// The declaration's `claim` occurrence of the node, from its row.
    pub claim: filament_exact::Origin,
    /// The object type the clause constrains: its `self` parameter's type.
    pub context: filament_exact::EffectiveId,
    /// The operation a precondition or postcondition is anchored on; `None`
    /// for an invariant or constraint.
    pub operation: Option<BareFrameOperation>,
    /// The observation the clause's kind reads (FR-104).
    pub observation: fsl_semantics::model::clause_observation::Observation,
    /// Whether the clause is an `invariant` (and no `constraint`).
    pub invariant: bool,
}

impl BarePackage<'_> {
    /// FR-940: every state clause declaration the package names, one per
    /// `claim_names` row of a `state`/`state_clause` node, in source order
    /// (the order of the rows' `declaration_region`s). Charges nothing.
    ///
    /// # Errors
    ///
    /// [`BareSelectionFailure`]: a witness refusal, or `MalformedNode` for a
    /// clause node whose body, `self` parameter or anchor does not read.
    pub fn state_clause_declarations(
        &self,
    ) -> Result<Vec<BareStateClauseDeclaration>, BareSelectionFailure> {
        use filament_contract_model::{CheckedOperationMember, StateClauseKind};
        use fsl_semantics::model::clause_observation::Observation;
        let package = self.package();
        let (_supply, identities) = self.witness()?;
        let graph = wire_graph(package).map_err(|_| fault("bare-package-has-structural-keys"))?;
        let nodes: BTreeMap<&CheckedNodeId, &CheckedSemanticNodeV2> = package
            .resolved_graph()
            .nodes
            .iter()
            .map(|node| (&node.node_id, node))
            .collect();
        let object = |node: &CheckedNodeId| {
            identities
                .declaration_rows()
                .find(|row| row.structural_node == *node)
                .and_then(|row| match &row.exact_identity {
                    filament_contract_model::SelectedExactIdentity::Object(identity) => {
                        Some(identity.clone())
                    }
                    _ => None,
                })
        };
        let all = package.claim_names().unwrap_or_default();
        // Each declaration once, at its claim row on its own node: a derived
        // conjunction's rows name declarations whose own node is elsewhere
        // (FSpec FR-322 "Rows of one declaration").
        let mut rows: Vec<&filament_contract_model::ClaimNameRow> = all
            .iter()
            .filter(|row| {
                nodes
                    .get(&row.declaration)
                    .is_some_and(|node| node.semantic_form.as_ref() == "state_clause")
            })
            .filter(|row| {
                let path = row
                    .qualified_name
                    .iter()
                    .map(AsRef::as_ref)
                    .collect::<Vec<&str>>()
                    .join("::");
                crate::analysis_view::own_claim_row(all, &path).is_some_and(|own| {
                    own.declaration == row.declaration && own.occurrence == row.occurrence
                })
            })
            .collect();
        rows.sort_by(|left, right| {
            let at = |row: &filament_contract_model::ClaimNameRow| {
                (
                    row.declaration_region.source.authority.clone(),
                    row.declaration_region.source.identity.clone(),
                    row.declaration_region.start,
                )
            };
            at(left).cmp(&at(right))
        });
        let mut declarations = Vec::with_capacity(rows.len());
        for row in rows {
            let node = nodes
                .get(&row.declaration)
                .copied()
                .ok_or_else(|| fault("bare-claim-row-names-a-node"))?;
            let key = graph
                .key(&node.node_id)
                .map_err(|_| fault("bare-package-has-structural-keys"))?;
            let malformed =
                || BareSelectionFailure::from(SlotSelectionFailure::MalformedNode(key.clone()));
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
            let [
                CheckedTerm::Aggregate { members, .. },
                CheckedTerm::Reference { target: anchor },
                _,
            ] = arguments.as_slice()
            else {
                return Err(malformed());
            };
            let Some(CheckedTerm::Reference { target: receiver }) = members.first() else {
                return Err(malformed());
            };
            // `self` is typed `Reference<T>`; its type projection names `T`.
            let context = self
                .exact_type_projections()
                .get(receiver)
                .and_then(|projected| projected.as_ref().ok())
                .and_then(|projected| match projected.value_type() {
                    filament_exact::ValueType::Reference(object) => Some(object.clone()),
                    _ => None,
                })
                .ok_or_else(malformed)?;
            let anchor = nodes.get(anchor).copied().ok_or_else(malformed)?;
            let operation = if anchor.semantic_form.as_ref() == "operation_anchor" {
                let declaring = anchor_member(&anchor.body, "context")
                    .and_then(crate::protocol_view::reference)
                    .and_then(object)
                    .ok_or_else(malformed)?;
                let operation = anchor_member(&anchor.body, "operation")
                    .and_then(text)
                    .ok_or_else(malformed)?;
                Some(BareFrameOperation {
                    declaring,
                    operation: operation.to_owned(),
                })
            } else {
                None
            };
            declarations.push(BareStateClauseDeclaration {
                name: row.name.as_ref().to_owned(),
                identity: key,
                claim: filament_exact::Origin::new(
                    filament_exact::Role::new("claim"),
                    row.occurrence.ordinal,
                ),
                context,
                operation,
                observation,
                invariant: matches!(kind, StateClauseKind::Invariant),
            });
        }
        Ok(declarations)
    }
}

/// The binding `name` of a `state`/`operation_anchor` node's body (FR-342):
/// its `context`, `operation` and `frame` members, in wire order.
fn anchor_member<'t>(term: &'t CheckedTerm, name: &str) -> Option<&'t CheckedTerm> {
    let CheckedTerm::OperationAnchorBody(body) = term else {
        return None;
    };
    body.members.iter().find_map(|member| match member {
        CheckedTerm::Binding {
            name: bound, value, ..
        } if &**bound == name => Some(&**value),
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
        let [
            CheckedTerm::Aggregate { members, .. },
            _anchor,
            CheckedTerm::Reference { target: body },
        ] = arguments.as_slice()
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
            self.dependency_views(),
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
pub(crate) fn declaration_index(
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
    /// FR-940: the node of the temporal clause whose node key is `clause`,
    /// which a refusal of its package form names.
    ///
    /// # Errors
    ///
    /// [`BareClauseRefusal::Selection`] when the package holds no temporal
    /// clause at the key.
    pub fn temporal_clause_node(
        &self,
        clause: &NodeKey,
    ) -> Result<&'p CheckedNodeId, BareClauseRefusal> {
        self.temporal_clause_semantic_node(clause)
            .map(|node| &node.node_id)
    }

    fn temporal_clause_semantic_node(
        &self,
        clause: &NodeKey,
    ) -> Result<&'p CheckedSemanticNodeV2, BareClauseRefusal> {
        let package = self.package();
        let graph = wire_graph(package).map_err(|_| fault("bare-package-has-structural-keys"))?;
        for node in &package.resolved_graph().nodes {
            if node.semantic_form.as_ref() != "temporal_clause" {
                continue;
            }
            let key = graph
                .key(&node.node_id)
                .map_err(|_| fault("bare-package-has-structural-keys"))?;
            if key == *clause {
                return Ok(node);
            }
        }
        Err(BareSelectionFailure::from(SlotSelectionFailure::MissingNode(clause.clone())).into())
    }

    /// FR-940: the package form of the temporal clause declared at the
    /// `claim` occurrence `claim` (its node key and occurrence), read from
    /// the package's admitted nodes, source map, name rows and type
    /// projections: its name, span, profile, `over`, trigger
    /// and capture binders, clock role, activation and formula arena. Its
    /// population domains are the caller's: they are read from the
    /// supplied domain packages, which a package does not hold. Charges
    /// nothing.
    ///
    /// # Errors
    ///
    /// [`BareClauseRefusal::Selection`] for a clause the package does not
    /// select; [`BareClauseRefusal::Pending`] for a fact the form does not
    /// read yet: a non-operation fairness target, an interval or an aggregate atom.
    pub fn temporal_clause_form(
        &self,
        claim: &fsl_foundation::source::provenance::OccurrenceKey,
    ) -> Result<fsl_semantics::check::PackageClauseForm, BareClauseRefusal> {
        self.read_temporal_clause_form(claim, false)
    }

    /// Project several authentic naming rows of this owner through one paid
    /// graph index and the same converter used by the legacy clause reader.
    /// Step/timed interval payloads are read as the package run form does;
    /// populations remain the caller's supplied domain facts.
    ///
    /// # Errors
    /// Rejects foreign/copied naming rows, preserves unsupported clause facts,
    /// and retains the original first unpaid native charge without a new meter.
    pub fn temporal_clause_forms_with_budget(
        &self,
        rows: &[&filament_contract_model::TemporalClauseName],
        budget: &mut NativeJoinBudget,
    ) -> Result<Vec<fsl_semantics::check::PackageClauseForm>, BareClauseFormRefusal> {
        let locus = native_row::start(budget, "temporal_clause_forms")?;
        native_row::work(budget, locus, 1)?;
        self.witness().map_err(BareClauseRefusal::from)?;
        let bodies = crate::checked_views::ClauseBodies::new(self.package(), budget)?;
        let mut work = MeteredForm { budget, locus };
        work.items(rows.len())?;
        let mut forms = Vec::with_capacity(rows.len());
        for row in rows {
            let mut authentic = false;
            for candidate in self.package().temporal_clause_names().into_iter().flatten() {
                work.visit()?;
                if std::ptr::eq(candidate, *row) {
                    authentic = true;
                    break;
                }
            }
            if !authentic {
                return Err(crate::checked_views::ViewRefusal::UnresolvedClause.into());
            }
            let body = bodies.body(&row.declaration, work.budget)?;
            let node = bodies
                .node(&row.declaration)
                .ok_or(crate::checked_views::ViewRefusal::UnresolvedClause)?;
            forms.push(self.finish_temporal_clause_form(
                row,
                node,
                FormParts {
                    over: body.over,
                    activation: &body.activation,
                    captures: &body.captures,
                    capture_names: &body.capture_names,
                    formula: &body.formula,
                    profile: body.profile,
                },
                FormResolution {
                    lookup: &|id: &CheckedNodeId| bodies.node(id),
                    intervals: true,
                    malformed: &|| BareClauseRefusal::from(fault("clause-form-shape-is-admitted")),
                    atom_span:
                        &mut |_position: usize,
                              atom: &crate::checked_views::FormulaNodeUse<'_>,
                              work: &mut MeteredForm<'_>| {
                            let mut found = None;
                            let mut multiple = false;
                            for entry in self.package().source_map() {
                                work.visit()?;
                                if entry.node_id != *atom.node
                                || entry.role
                                    != filament_contract_model::CheckedOccurrenceRole::Expression
                            {
                                continue;
                            }
                                for region in &entry.regions {
                                    work.visit()?;
                                    if region.source == row.declaration_region.source
                                        && region.start >= row.declaration_region.start
                                        && region.end <= row.declaration_region.end
                                    {
                                        if found.is_some() {
                                            multiple = true;
                                        } else {
                                            found = Some(region_span(region));
                                        }
                                    }
                                }
                            }
                            match (found, multiple) {
                                (Some(span), false) => Ok(span),
                                (Some(_), true) => Ok(region_span(&row.declaration_region)),
                                (None, _) => {
                                    Err(crate::checked_views::ViewRefusal::LeafHasNoSource.into())
                                }
                            }
                        },
                    work: &mut work,
                },
            )?);
        }
        Ok(forms)
    }

    /// [`Self::temporal_clause_form`] with each operator's step interval read
    /// into the form, for a run that evaluates the clause over a trace. A
    /// timed interval is still a pending fact.
    ///
    /// # Errors
    ///
    /// As [`Self::temporal_clause_form`], less the step interval.
    pub fn temporal_clause_run_form(
        &self,
        claim: &fsl_foundation::source::provenance::OccurrenceKey,
    ) -> Result<fsl_semantics::check::PackageClauseForm, BareClauseRefusal> {
        self.read_temporal_clause_form(claim, true)
    }

    /// Candidate atom occurrences inside the explicitly selected claim's
    /// declaration and native source. Structural sharing does not identify
    /// a particular use; callers may compare supplied text, never pair ranks.
    pub fn temporal_atom_sources(
        &self,
        claim: &fsl_foundation::source::provenance::OccurrenceKey,
        position: usize,
    ) -> Result<
        Vec<crate::occurrence_source::OccurrenceSource>,
        crate::occurrence_source::OccurrenceSourceRefusal,
    > {
        use crate::occurrence_source::{OccurrenceSourceRefusal, occurrence_source};
        use filament_contract_model::{CheckedOccurrence, CheckedOccurrenceRole};
        let package = self.package();
        let node = self
            .temporal_clause_semantic_node(claim.node())
            .map_err(|_| OccurrenceSourceRefusal::UnknownNode)?;
        let view = temporal_subject(package, &node.node_id)
            .map_err(|_| OccurrenceSourceRefusal::UnknownNode)?;
        let row = view
            .name_row_of(&crate::checked_views::ClauseOccurrence {
                node: node.node_id.clone(),
                role: CheckedOccurrenceRole::Claim,
                ordinal: claim.origin().ordinal(),
            })
            .ok_or(OccurrenceSourceRefusal::UnknownOccurrence)?;
        let atom = view
            .formula_graph()
            .get(position)
            .filter(|entry| entry.kind() == Some(FormulaOperation::Holds))
            .ok_or(OccurrenceSourceRefusal::UnknownOccurrence)?;
        let declaration = crate::checked_v2::source_region(&row.declaration_region)
            .map_err(|_| OccurrenceSourceRefusal::MissingRegion)?;
        let mut candidates = Vec::new();
        for entry in package.source_map().iter().filter(|entry| {
            entry.node_id == *atom.node && entry.role == CheckedOccurrenceRole::Expression
        }) {
            for region in &entry.regions {
                let region = crate::checked_v2::source_region(region)
                    .map_err(|_| OccurrenceSourceRefusal::MissingRegion)?;
                if region.source() == declaration.source()
                    && region.start() >= declaration.start()
                    && region.end() <= declaration.end()
                {
                    candidates.push(occurrence_source(
                        package,
                        atom.node,
                        &CheckedOccurrence {
                            role: entry.role.clone(),
                            ordinal: entry.ordinal,
                        },
                        Some(&region),
                    )?);
                }
            }
        }
        if candidates.is_empty() {
            return Err(OccurrenceSourceRefusal::MissingRegion);
        }
        Ok(candidates)
    }

    #[ix_trace_rs::string_edge]
    fn read_temporal_clause_form(
        &self,
        claim: &fsl_foundation::source::provenance::OccurrenceKey,
        intervals: bool,
    ) -> Result<fsl_semantics::check::PackageClauseForm, BareClauseRefusal> {
        let package = self.package();
        self.witness()?;
        let nodes: BTreeMap<&CheckedNodeId, &CheckedSemanticNodeV2> = package
            .resolved_graph()
            .nodes
            .iter()
            .map(|node| (&node.node_id, node))
            .collect();
        let clause = claim.node();
        let malformed =
            || BareSelectionFailure::from(SlotSelectionFailure::MalformedNode(clause.clone()));
        let node = self.temporal_clause_semantic_node(clause)?;
        let view = temporal_subject(package, &node.node_id).map_err(|_| malformed())?;
        // W58: structurally equal clauses share one node; each declaration is
        // its own `claim` occurrence with its own name row.
        let row = view
            .name_row_of(&crate::checked_views::ClauseOccurrence {
                node: node.node_id.clone(),
                role: filament_contract_model::CheckedOccurrenceRole::Claim,
                ordinal: claim.origin().ordinal(),
            })
            .ok_or_else(malformed)?;
        self.finish_temporal_clause_form(
            row,
            node,
            FormParts {
                over: view.over(),
                activation: view.activation(),
                captures: view.captures(),
                capture_names: view.capture_names(),
                formula: view.formula_graph(),
                profile: view.profile(),
            },
            FormResolution {
                lookup: &|id: &CheckedNodeId| nodes.get(id).copied(),
                intervals,
                malformed: &|| BareClauseRefusal::from(malformed()),
                atom_span: &mut |position: usize,
                                 _entry: &crate::checked_views::FormulaNodeUse<'_>,
                                 _work: &mut UnmeteredFairness| {
                    let sources = self
                        .temporal_atom_sources(claim, position)
                        .map_err(|_| malformed())?;
                    match sources.as_slice() {
                        [only] => Ok(fsl_foundation::Span {
                            start: usize::try_from(only.region.start()).map_err(|_| malformed())?,
                            end: usize::try_from(only.region.end()).map_err(|_| malformed())?,
                        }),
                        _ => Ok(region_span(&row.declaration_region)),
                    }
                },
                work: &mut UnmeteredFairness,
            },
        )
    }

    #[ix_trace_rs::string_edge]
    fn finish_temporal_clause_form<'n, W: FormWork>(
        &self,
        row: &filament_contract_model::TemporalClauseName,
        node: &'n CheckedSemanticNodeV2,
        parts: FormParts<'_, '_>,
        resolution: FormResolution<'n, '_, W>,
    ) -> Result<fsl_semantics::check::PackageClauseForm, W::Error> {
        let FormResolution {
            lookup,
            intervals,
            malformed,
            mut atom_span,
            work,
            ..
        } = resolution;
        use fsl_semantics::check::{BinaryTemporal, PackageFormulaNode, UnaryTemporal};
        let package = self.package();
        work.text(row.name.as_ref())?;
        let name = row.name.as_ref().to_owned();
        let CheckedTerm::Application { arguments, .. } = &node.body else {
            return Err(malformed().into());
        };
        // FR-337 "Clause emission": `over`, clock, activation, captures,
        // fairness, formula.
        let [_, clock, _, _, CheckedTerm::Aggregate { .. }, _] = arguments.as_slice() else {
            return Err(malformed().into());
        };
        let fairness = self.read_clause_fairness(&node.node_id, &node.body, work)?;
        let clock = text(clock).ok_or_else(malformed)?;
        work.text(clock)?;
        let clock = clock.to_owned();
        // FSpec FR-322 "Clause declaration regions": the whole declaration.
        let span = region_span(&row.declaration_region);
        let mut binder =
            |id: &CheckedNodeId| -> Result<(String, filament_exact::ValueType), W::Error> {
                work.visit()?;
                let parameter = lookup(id).ok_or_else(malformed)?;
                let CheckedTerm::Aggregate { members, .. } = &parameter.body else {
                    return Err(malformed().into());
                };
                let mut name = None;
                for field in members {
                    work.visit()?;
                    if let CheckedTerm::Binding {
                        name: bound, value, ..
                    } = field
                    {
                        if &**bound == "name" {
                            name = text(value);
                            break;
                        }
                    }
                }
                let name = name.ok_or_else(malformed)?;
                work.visit()?;
                let value_type = self
                    .exact_type_projections()
                    .get(id)
                    .and_then(|projected| projected.as_ref().ok())
                    .map(|projected| projected.value_type())
                    .ok_or_else(malformed)?;
                work.value_type(value_type)?;
                work.text(name)?;
                Ok((name.to_owned(), value_type.clone()))
            };
        let over = binder(parts.over)?;
        let (activation, trigger) = match parts.activation {
            Activation::Origin => (fsl_semantics::check::Activation::Origin, None),
            Activation::Each { trigger, guard } => (
                fsl_semantics::check::Activation::Each {
                    guarded: guard.is_some(),
                },
                Some(binder(trigger)?),
            ),
        };
        work.items(parts.captures.len())?;
        let mut captures = Vec::with_capacity(parts.captures.len());
        for (initializer, capture) in parts.captures.iter().zip(parts.capture_names) {
            work.visit()?;
            let value_type = self
                .exact_type_projections()
                .get(initializer)
                .and_then(|projected| projected.as_ref().ok())
                .map(|projected| projected.value_type())
                .ok_or_else(malformed)?;
            work.value_type(value_type)?;
            work.text(capture)?;
            captures.push((capture.clone(), value_type.clone()));
        }
        let uses = parts.formula;
        work.items(uses.len())?;
        let mut formula = Vec::with_capacity(uses.len());
        for (position, entry) in uses.iter().enumerate() {
            work.visit()?;
            let pending = |fact| BareClauseRefusal::Pending {
                fact,
                node: entry.node.clone(),
            };
            let interval = match (&entry.interval, intervals) {
                (None, _) => None,
                (Some(interval), true) => {
                    work.interval(interval)?;
                    match checked_interval(interval) {
                        Some(interval) => Some(interval),
                        None => {
                            work.node(entry.node)?;
                            return Err(pending("interval").into());
                        }
                    }
                }
                (Some(_), false) => {
                    work.node(entry.node)?;
                    return Err(pending("interval").into());
                }
            };
            let operand =
                |position: usize| entry.operands.get(position).copied().ok_or_else(malformed);
            let kind = entry.kind().ok_or_else(malformed)?;
            formula.push(match kind {
                FormulaOperation::True => PackageFormulaNode::Constant(true),
                FormulaOperation::False => PackageFormulaNode::Constant(false),
                FormulaOperation::Holds => {
                    let atom_span = atom_span(position, entry, work)?;
                    PackageFormulaNode::Holds { span: atom_span }
                }
                FormulaOperation::Aggregate => {
                    work.node(entry.node)?;
                    return Err(pending("aggregate").into());
                }
                FormulaOperation::Not => PackageFormulaNode::Not(operand(0)?),
                FormulaOperation::And => PackageFormulaNode::And(operand(0)?, operand(1)?),
                FormulaOperation::Or => PackageFormulaNode::Or(operand(0)?, operand(1)?),
                FormulaOperation::Implies => PackageFormulaNode::Implies(operand(0)?, operand(1)?),
                FormulaOperation::Eventually
                | FormulaOperation::Always
                | FormulaOperation::Once
                | FormulaOperation::Historically => PackageFormulaNode::Unary {
                    interval,
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
                    interval,
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
            &parts.profile.definition.identity,
        )
        .ok_or_else(malformed)?;
        work.text(package.name())?;
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
            fairness,
            populations: Vec::new(),
        })
    }
}

/// The step interval a package operator carries; a timed interval has none.
fn checked_interval(
    interval: &crate::CheckedOperationInterval,
) -> Option<fsl_semantics::check::CheckedInterval> {
    use fsl_foundation::bound::{TemporalInterval, TemporalUpper};

    match interval {
        crate::CheckedOperationInterval::Integer { lower, upper } => {
            let upper = match upper {
                None => TemporalUpper::Open,
                Some(upper) => TemporalUpper::Finite(upper.to_u64()?),
            };
            Some(fsl_semantics::check::CheckedInterval::Steps(
                TemporalInterval::new(lower.to_u64()?, upper).ok()?,
            ))
        }
        crate::CheckedOperationInterval::Timed {
            lower,
            upper,
            lower_end,
            upper_end,
        } => {
            let rational = |value: &crate::CheckedOperationRational| {
                filament_exact::Rational::new(value.numerator.clone(), value.denominator.clone())
                    .ok()
            };
            let lower = rational(lower)?;
            let lower_closed = *lower_end == crate::CheckedIntervalEnd::Closed;
            let interval = match upper {
                Some(upper) => fsl_foundation::bound::TimedInterval::new(
                    lower,
                    lower_closed,
                    rational(upper)?,
                    *upper_end == crate::CheckedIntervalEnd::Closed,
                ),
                None => fsl_foundation::bound::TimedInterval::unbounded(lower, lower_closed),
            }
            .ok()?;
            Some(fsl_semantics::check::CheckedInterval::Timed(interval))
        }
    }
}

fn region_span(region: &filament_contract_model::CheckedSourceRegion) -> fsl_foundation::Span {
    fsl_foundation::Span {
        start: usize::try_from(region.start).unwrap_or(usize::MAX),
        end: usize::try_from(region.end).unwrap_or(usize::MAX),
    }
}

/// The names of a record, tuple or union declaration of a package, in
/// declaration order, read from its declaration node alone.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DeclaredShape {
    /// A record's field names.
    Record(Vec<String>),
    /// A union's member names.
    Union(Vec<String>),
    /// A tuple, which names no member.
    Tuple,
}

/// A record, tuple or union declaration node of a package.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeclaredComposite {
    /// The declaration node's structural key.
    pub key: NodeKey,
    /// Its member names.
    pub shape: DeclaredShape,
}

impl BarePackage<'_> {
    /// Every record, tuple and union declaration node of the package with
    /// the member names its body lists, in node order. The printer of an
    /// atom reads these names when no identity row selects the declaration
    /// (FR-931 "The lowering input"). Charges nothing.
    pub fn declared_composites(&self) -> Vec<DeclaredComposite> {
        use crate::wire_form::{WireForm, WireTag, has_form};
        use filament_contract_model::CheckedBindingNameKind;

        let names = |node: &CheckedSemanticNodeV2| -> Vec<String> {
            let CheckedTerm::Aggregate { members, .. } = &node.body else {
                return Vec::new();
            };
            members
                .iter()
                .filter_map(|member| match member {
                    CheckedTerm::Binding { name, .. } => match name.kind() {
                        CheckedBindingNameKind::Other(name) => Some(name.to_owned()),
                        _ => None,
                    },
                    _ => None,
                })
                .collect()
        };
        self.package()
            .resolved_graph()
            .nodes
            .iter()
            .filter_map(|node| {
                let shape = if has_form(node, WireTag::CompositeType, WireForm::Record) {
                    DeclaredShape::Record(names(node))
                } else if has_form(node, WireTag::CompositeType, WireForm::Union) {
                    DeclaredShape::Union(names(node))
                } else if has_form(node, WireTag::CompositeType, WireForm::Tuple) {
                    DeclaredShape::Tuple
                } else {
                    return None;
                };
                Some(DeclaredComposite {
                    key: self.node_key(&node.node_id)?,
                    shape,
                })
            })
            .collect()
    }
}
