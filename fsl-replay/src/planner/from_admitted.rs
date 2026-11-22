// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-1044: composition premises from authenticated borrowed package facts.

use filament_contract_model::{
    CheckedComponentClause, CheckedEnvironmentSupplies, CheckedFairnessGranularity,
    CheckedFairnessKind, NativeJoinBudget, NativeJoinChargeKind, NativeJoinDeniedCharge,
    NativeJoinLocus, TemporalClauseName,
};
use filament_exact::{EffectiveId, NodeKey, Origin, OriginalDeclarationKey, Role};
use filament_semantic_value::fairness::{
    FairnessConstraint, FairnessGranularity, FairnessKind, FairnessOperation, PropertyForm,
};
use fsl_foundation::source::provenance::{OccurrenceKey, SourceRegion};
use fsl_package::{
    BareClauseFormRefusal, BarePackage, CompositionMemberValue, CompositionModelContextRefusal,
    CompositionView, CompositionViewRefusal, OwnedNodeRef,
};
use fsl_semantics::check::wire_id::{
    StructuralKeyMaterializationError, StructuralKeys, WireIdError,
};
use fsl_semantics::check::{
    CompositionGraphRefusal, CompositionRule, PackageClauseForm, PackagePropertyRefusal,
};

use super::composition_graph::{composition_cycle_rows, CompositionCycleRefusal, CompositionEdge};
use super::declaration::{
    DerivedSlot, EnvSupplies, EnvironmentMember, SchedulingPremise, SlotBacking, SlotRole,
};
use super::from_package::ItemKeyInput;
use super::key::{ClaimKind, ExtentClass, LocalKey, OperationRef, SlotKey};
use super::key_build::{self, KeyBuildRefusal};

/// An actual uniquely named clause and its retained authenticated library view.
pub struct PackageCompositionClause<'a, 'p> {
    /// The component label from the composition body.
    pub component: &'p str,
    /// The exact admitted owning evaluation view.
    pub owner: &'a BarePackage<'p>,
    /// The authentic owning temporal-clause naming row.
    pub name: &'p TemporalClauseName,
    /// The document-independent structural clause identity.
    pub node: NodeKey,
    /// The source-compatible subject identity from actual selected model facts.
    pub subject: String,
    /// The existing package clause form, absent only when a cycle stops preparation.
    pub form: Option<PackageClauseForm>,
    /// Source property classification, absent only before cyclic refusal.
    pub property: Option<PropertyForm>,
    /// Actual owning fairness targets, with structural context and operation identity.
    pub fairness: Option<fsl_package::BareClauseConstraints<'a>>,
}
/// An admitted premise and the actual clauses it links, without source slot objects.
pub struct PackageCompositionSlot {
    /// The shared planner's neutral premise descriptor.
    pub derived: DerivedSlot,
    /// Index in the composition's uniquely resolved clause table.
    pub guarantee: Option<usize>,
    /// A reliance or dependency's peer clause index.
    pub on: Option<usize>,
}
/// One dependency cycle with its authentic first authored edge.
pub struct PackageCompositionCycle {
    /// Indices in the composition's clause table, in source edge-tail order.
    pub guarantees: Vec<usize>,
    /// The original authored member region.
    pub source: SourceRegion,
}
/// The admitted composition facts shared by routing and planner settlement.
pub struct PackageComposition<'a, 'p> {
    /// Actual composition declaration identity.
    pub(crate) node: NodeKey,
    /// Actual root package identity retained from its sealed owning view.
    pub(crate) package: String,
    /// The admitted rule.
    pub(crate) rule: CompositionRule,
    /// Actual owner-bound clause forms; no source checked graph is reconstructed.
    pub(crate) clauses: Vec<PackageCompositionClause<'a, 'p>>,
    /// Source-compatible ordered premise descriptors.
    pub(crate) slots: Vec<PackageCompositionSlot>,
    /// Declared guarantee positions assumed by each component-guarantee slot.
    pub(crate) assumptions: Vec<Vec<usize>>,
    /// Cycles are derived before clause execution forms or premise slots.
    pub(crate) cycles: Vec<PackageCompositionCycle>,
    /// Actual authored environment members.
    pub(crate) environment: Vec<EnvironmentMember>,
    /// Scheduling constraints supplied by the composition.
    pub(crate) supplies: Vec<FairnessConstraint>,
    raw_supplies: Vec<RawSupply>,
}
struct RawSupply {
    context: NodeKey,
    operation: filament_exact::Identifier,
    kind: FairnessKind,
    granularity: FairnessGranularity,
}
/// A composition read preserves original semantic and native refusals.
#[derive(Debug, thiserror::Error)]
pub enum PackageCompositionRefusal {
    /// The supplied root view belongs to a different admission.
    #[error("composition and evaluation view have different admissions")]
    ForeignView,
    /// An actual owning evaluation view is missing or duplicated.
    #[error("composition owner is unavailable or duplicated")]
    Owner,
    /// An admitted member cannot be resolved in the declaration's own tables.
    #[error("composition member has no corresponding admitted row")]
    Member,
    /// An admitted rule spelling is not a known composition rule.
    #[error("composition rule is unavailable")]
    Rule,
    /// Cyclic compositions have no planner declaration or interface decision.
    #[error("the composition dependency graph holds a cycle")]
    Cyclic,
    /// A clause has no model property form the source composition admits.
    #[error("component {component} clause {clause} has no model property form")]
    Property { component: String, clause: String },
    /// The original borrowed composition-view refusal.
    #[error(transparent)]
    View(#[from] CompositionViewRefusal),
    /// The original owning clause-form refusal.
    #[error(transparent)]
    Form(#[from] BareClauseFormRefusal),
    /// The original world-free property refusal.
    #[error(transparent)]
    PropertyForm(#[from] PackagePropertyRefusal),
    /// Authentic candidate contexts were absent or conflicting.
    #[error(transparent)]
    Context(#[from] CompositionModelContextRefusal),
    /// The existing structural identity policy refused.
    #[error(transparent)]
    Structural(#[from] WireIdError),
    /// The bounded reason-major mismatch core refused.
    #[error(transparent)]
    Mismatch(#[from] super::mismatch::MismatchRefusal),
    /// The shared topology traversal refused.
    #[error(transparent)]
    Graph(#[from] CompositionGraphRefusal),
    /// The actual canonical writer refused.
    #[error(transparent)]
    Canonical(#[from] quire_canonical::Error),
    /// The existing key wire projection refused.
    #[error(transparent)]
    Key(#[from] super::key::KeyWireError),
    /// The original first unpaid native charge, unchanged.
    #[error("composition projection exceeded {denied:?} at {locus:?}")]
    Native {
        denied: NativeJoinDeniedCharge,
        locus: Option<NativeJoinLocus>,
        configured_bound: u64,
    },
    /// A prepaid reservation failed.
    #[error(transparent)]
    Allocation(#[from] std::collections::TryReserveError),
    /// A requested retained size cannot be represented.
    #[error("composition projection size overflow")]
    Size,
}
struct Work<'b> {
    budget: &'b mut NativeJoinBudget,
    locus: NativeJoinLocus,
}
impl Work<'_> {
    fn charge(
        &mut self,
        kind: NativeJoinChargeKind,
        amount: u128,
    ) -> Result<(), PackageCompositionRefusal> {
        self.budget
            .accounted_charge(kind, amount, self.locus)
            .map_err(|denied| PackageCompositionRefusal::Native {
                configured_bound: match denied.kind {
                    NativeJoinChargeKind::Work => self.budget.work_limit(),
                    NativeJoinChargeKind::Storage => self.budget.storage_limit(),
                },
                locus: self.budget.first_unpaid_locus(),
                denied,
            })
    }
    fn visit(&mut self) -> Result<(), PackageCompositionRefusal> {
        self.charge(NativeJoinChargeKind::Work, 1)
    }
    fn room<T>(&mut self, list: &mut Vec<T>) -> Result<(), PackageCompositionRefusal> {
        if list.len() == list.capacity() {
            let extra = list.len().max(1);
            self.charge(
                NativeJoinChargeKind::Storage,
                1 + list.len() as u128 + extra as u128,
            )?;
            list.try_reserve_exact(extra)?;
        }
        Ok(())
    }
    fn text_cost(&mut self, text: &str) -> Result<(), PackageCompositionRefusal> {
        self.charge(NativeJoinChargeKind::Work, 1 + text.len() as u128)?;
        if !text.is_empty() {
            self.charge(NativeJoinChargeKind::Storage, 1 + text.len() as u128)?;
        }
        Ok(())
    }
    fn text(&mut self, text: &str) -> Result<String, PackageCompositionRefusal> {
        self.text_cost(text)?;
        Ok(text.to_owned())
    }
    fn join(&mut self, parts: &[&str]) -> Result<String, PackageCompositionRefusal> {
        let len = parts
            .iter()
            .try_fold(0usize, |n, part| n.checked_add(part.len()))
            .ok_or(PackageCompositionRefusal::Size)?;
        self.charge(NativeJoinChargeKind::Storage, 1 + len as u128)?;
        let mut text = String::new();
        text.try_reserve_exact(len)?;
        for part in parts {
            self.charge(NativeJoinChargeKind::Work, 1 + part.len() as u128)?;
            text.push_str(part);
        }
        Ok(text)
    }
    fn region(&mut self, source: &SourceRegion) -> Result<SourceRegion, PackageCompositionRefusal> {
        self.visit()?;
        self.text_cost(source.source().authority())?;
        self.text_cost(source.source().identity())?;
        Ok(source.clone())
    }
    fn effective_cost(&mut self, value: &EffectiveId) -> Result<(), PackageCompositionRefusal> {
        if !value.owner_chain().is_empty() {
            self.charge(
                NativeJoinChargeKind::Storage,
                1 + value.owner_chain().len() as u128,
            )?;
        }
        for original in value.owner_chain() {
            self.visit()?;
            self.text_cost(original.package())?;
            self.text_cost(original.node())?;
        }
        Ok(())
    }
    fn text_equal(&mut self, left: &str, right: &str) -> Result<bool, PackageCompositionRefusal> {
        let (mut left, mut right) = (left.bytes(), right.bytes());
        loop {
            self.visit()?;
            match (left.next(), right.next()) {
                (Some(a), Some(b)) if a == b => {}
                (None, None) => return Ok(true),
                _ => return Ok(false),
            }
        }
    }
    fn key(
        &mut self,
        owner: &filament_contract_model::CheckedPackageV2,
        node: &filament_contract_model::CheckedNodeId,
        keys: &mut StructuralKeys,
    ) -> Result<NodeKey, PackageCompositionRefusal> {
        self.visit()?;
        let structural = owner
            .structural_key(node)
            .ok_or(PackageCompositionRefusal::Member)?;
        match keys.try_key(structural, self.budget, self.locus) {
            Ok(key) => Ok(key),
            Err(StructuralKeyMaterializationError::Wire(e)) => Err(e.into()),
            Err(StructuralKeyMaterializationError::Allocation(e)) => Err(e.into()),
            Err(StructuralKeyMaterializationError::Budget {
                denied,
                configured_bound,
                locus,
            }) => Err(PackageCompositionRefusal::Native {
                denied,
                configured_bound,
                locus,
            }),
        }
    }
}
fn fairness_error(error: fsl_package::BareFairnessRefusal) -> PackageCompositionRefusal {
    match error {
        fsl_package::BareFairnessRefusal::Clause(error) => {
            PackageCompositionRefusal::Form(BareClauseFormRefusal::Form(error))
        }
        fsl_package::BareFairnessRefusal::Native(error) => {
            PackageCompositionRefusal::Form(BareClauseFormRefusal::Native(error))
        }
        fsl_package::BareFairnessRefusal::Structural(error) => {
            PackageCompositionRefusal::Structural(error)
        }
        fsl_package::BareFairnessRefusal::Allocation(error) => {
            PackageCompositionRefusal::Allocation(error)
        }
    }
}
fn key_error(error: KeyBuildRefusal) -> PackageCompositionRefusal {
    match error {
        KeyBuildRefusal::Native {
            denied,
            locus,
            configured_bound,
        } => PackageCompositionRefusal::Native {
            denied,
            locus,
            configured_bound,
        },
        KeyBuildRefusal::Canonical(e) => e.into(),
        KeyBuildRefusal::Wire(e) => e.into(),
    }
}
struct Member<'p> {
    value: CompositionMemberValue<'p>,
    source: SourceRegion,
}
struct Premise {
    guarantee: usize,
    on: usize,
    source: SourceRegion,
    role: SlotRole,
}
struct Component<'a, 'p> {
    name: &'p str,
    owner: &'a BarePackage<'p>,
    subject: String,
}
fn owner<'a, 'p>(
    root: &'a BarePackage<'p>,
    reference: &OwnedNodeRef<'p>,
    work: &mut Work<'_>,
) -> Result<&'a BarePackage<'p>, PackageCompositionRefusal> {
    let mut found = None;
    for candidate in root.dependency_views() {
        work.visit()?;
        if candidate.package().admission_binding() == reference.owner().admission_binding() {
            if found.is_some() {
                return Err(PackageCompositionRefusal::Owner);
            }
            found = Some(candidate);
        }
    }
    found.ok_or(PackageCompositionRefusal::Owner)
}
fn local(role: SlotRole, members: Vec<String>) -> SlotKey {
    SlotKey::Local(LocalKey { role, members })
}
fn base(role: SlotRole, source: SourceRegion, key: SlotKey) -> DerivedSlot {
    DerivedSlot {
        role,
        key,
        backing: SlotBacking::Pool,
        constraint: None,
        operation: None,
        premise_subject: None,
        peer_clause: None,
        source: Some(source),
    }
}
fn display(
    clause: &PackageCompositionClause<'_, '_>,
    work: &mut Work<'_>,
) -> Result<String, PackageCompositionRefusal> {
    work.join(&[clause.component, "::", clause.name.name.as_ref()])
}
fn clause<'a, 'p>(
    root: &'a BarePackage<'p>,
    view: &CompositionView<'p>,
    target: &'p CheckedComponentClause,
    components: &[Component<'a, 'p>],
    clauses: &mut Vec<PackageCompositionClause<'a, 'p>>,
    work: &mut Work<'_>,
    keys: &mut StructuralKeys,
) -> Result<usize, PackageCompositionRefusal> {
    let reference = view.clause_with_budget(target, work.budget)?;
    let owner = owner(root, &reference, work)?;
    let name = reference
        .clause_name()
        .ok_or(PackageCompositionRefusal::Member)?;
    let node = work.key(owner.package(), reference.node(), keys)?;
    for (at, known) in clauses.iter().enumerate() {
        work.visit()?;
        if known.component == target.component.as_ref() && known.node == node {
            return Ok(at);
        }
    }
    let mut component = None;
    for candidate in components {
        work.visit()?;
        if candidate.name == target.component.as_ref() {
            component = Some(candidate);
            break;
        }
    }
    let component = component.ok_or(PackageCompositionRefusal::Member)?;
    if component.owner.package().admission_binding() != owner.package().admission_binding() {
        return Err(PackageCompositionRefusal::Owner);
    }
    let subject = work.text(&component.subject)?;
    work.room(clauses)?;
    let at = clauses.len();
    clauses.push(PackageCompositionClause {
        component: target.component.as_ref(),
        owner,
        name,
        node,
        subject,
        form: None,
        property: None,
        fairness: None,
    });
    Ok(at)
}

impl<'a, 'p> PackageComposition<'a, 'p> {
    /// The actual declaration's structural identity.
    pub fn node(&self) -> &NodeKey {
        &self.node
    }
    /// The admitted composition rule.
    pub fn rule(&self) -> CompositionRule {
        self.rule
    }
    /// Actual owning clause facts, in first referenced order.
    pub fn clauses(&self) -> &[PackageCompositionClause<'a, 'p>] {
        &self.clauses
    }
    /// Source-compatible ordered premise slots.
    pub fn slots(&self) -> &[PackageCompositionSlot] {
        &self.slots
    }
    /// The declared guarantee positions assumed by each guarantee slot.
    pub fn assumptions(&self) -> &[Vec<usize>] {
        &self.assumptions
    }
    /// Actual cycles, available before any execution form is prepared.
    pub fn cycles(&self) -> &[PackageCompositionCycle] {
        &self.cycles
    }
    /// The actual authored environment members.
    pub fn environment(&self) -> &[EnvironmentMember] {
        &self.environment
    }
    /// The supplied scheduling constraints.
    pub fn supplies(&self) -> &[FairnessConstraint] {
        &self.supplies
    }
    /// Recompute interface mismatch from complete authenticated clause and
    /// raw supply facts. No provider claim or pooled result is an authority.
    pub fn interface_mismatch_with_budget(
        &self,
        budget: &mut NativeJoinBudget,
    ) -> Result<Option<super::mismatch::InterfaceMismatch>, PackageCompositionRefusal> {
        use super::mismatch::{
            MismatchClauseFairness, MismatchFacts, MismatchNeed, MismatchSupply,
        };
        let locus = NativeJoinLocus {
            source: filament_contract_model::NativeJoinLocusSource::Package,
            form: "composition_mismatch_facts",
        };
        let mut work = Work { budget, locus };
        work.visit()?;
        if !self.cycles.is_empty() {
            return Err(PackageCompositionRefusal::Cyclic);
        }
        let mut references = Vec::new();
        for clause in &self.clauses {
            work.visit()?;
            work.room(&mut references)?;
            references.push(fsl_semantics::check::ClauseRef {
                component: work.text(clause.component)?,
                name: work.text(clause.name.name.as_ref())?,
                node: clause.node.clone(),
            });
        }
        let mut declared = Vec::new();
        let mut needs = Vec::new();
        for (position, slot) in self.slots.iter().enumerate() {
            work.visit()?;
            match slot.derived.role {
                SlotRole::ComponentGuarantee => {
                    let peer = references
                        .get(slot.guarantee.ok_or(PackageCompositionRefusal::Member)?)
                        .ok_or(PackageCompositionRefusal::Member)?;
                    work.room(&mut declared)?;
                    declared.push(peer);
                }
                SlotRole::Rely | SlotRole::LivenessDependency => {
                    let peer = references
                        .get(slot.on.ok_or(PackageCompositionRefusal::Member)?)
                        .ok_or(PackageCompositionRefusal::Member)?;
                    work.room(&mut needs)?;
                    needs.push(MismatchNeed {
                        premise: position,
                        peer,
                    });
                }
                SlotRole::Fairness | SlotRole::Environment => {}
                _ => return Err(PackageCompositionRefusal::Member),
            }
        }
        let mut clauses = Vec::new();
        for (position, clause) in self.clauses.iter().enumerate() {
            work.visit()?;
            let fairness = clause
                .fairness
                .as_ref()
                .ok_or(PackageCompositionRefusal::Member)?;
            let peer = references
                .get(position)
                .ok_or(PackageCompositionRefusal::Member)?;
            work.room(&mut clauses)?;
            clauses.push(MismatchClauseFairness {
                peer,
                needed: fairness.constraints(),
            });
        }
        let mut supplies = Vec::new();
        for supply in &self.raw_supplies {
            work.visit()?;
            work.room(&mut supplies)?;
            supplies.push(MismatchSupply {
                context: &supply.context,
                operation: &supply.operation,
                kind: supply.kind,
                granularity: supply.granularity,
            });
        }
        Ok(super::mismatch::interface_mismatch_over(
            MismatchFacts {
                needs: &needs,
                declared: &declared,
                clauses: &clauses,
                supplies: &supplies,
            },
            work.budget,
        )?)
    }

    /// Move one actual clause form into its owning execution preparation.
    pub fn take_form(&mut self, at: usize) -> Option<PackageClauseForm> {
        self.clauses.get_mut(at)?.form.take()
    }

    /// Read actual ordered premises from one admitted composition and the
    /// invocation's retained owning views, under its cumulative native ledger.
    ///
    /// # Errors
    /// Original view, witness, form, property and native refusals; no missing
    /// package fact is replaced by a source declaration or an empty fact set.
    pub fn from_view(
        root: &'a BarePackage<'p>,
        view: &CompositionView<'p>,
        budget: &mut NativeJoinBudget,
    ) -> Result<Self, PackageCompositionRefusal> {
        let locus = NativeJoinLocus {
            source: filament_contract_model::NativeJoinLocusSource::Package,
            form: "composition_slots",
        };
        if let Some(denied) = budget.first_unpaid() {
            return Err(PackageCompositionRefusal::Native {
                denied,
                locus: budget.first_unpaid_locus(),
                configured_bound: match denied.kind {
                    NativeJoinChargeKind::Work => budget.work_limit(),
                    NativeJoinChargeKind::Storage => budget.storage_limit(),
                },
            });
        }
        budget.resume();
        let mut work = Work { budget, locus };
        work.visit()?;
        if root.package().admission_binding() != view.package().admission_binding() {
            return Err(PackageCompositionRefusal::ForeignView);
        }
        let rule =
            CompositionRule::from_spelling(view.rule()).ok_or(PackageCompositionRefusal::Rule)?;
        let count = view.member_count()?;
        if count != 0 {
            work.charge(NativeJoinChargeKind::Storage, 1 + count as u128)?;
        }
        let mut authored = Vec::new();
        authored.try_reserve_exact(count)?;
        for member in view.members_with_budget(work.budget) {
            let (value, source) = member?.into_parts();
            authored.push(Member {
                value,
                source: source.region,
            });
        }
        for at in 1..authored.len() {
            let mut next = at;
            while next > 0 {
                work.visit()?;
                if authored[next - 1].source.source() != authored[next].source.source() {
                    return Err(PackageCompositionRefusal::Member);
                }
                let order = (
                    authored[next - 1].source.start(),
                    authored[next - 1].source.end(),
                )
                    .cmp(&(authored[next].source.start(), authored[next].source.end()));
                if !order.is_gt() {
                    break;
                }
                work.visit()?;
                authored.swap(next - 1, next);
                next -= 1;
            }
        }
        let mut keys = StructuralKeys::default();
        let mut components = Vec::new();
        for member in &authored {
            work.visit()?;
            if let CompositionMemberValue::Component(component) = member.value {
                let target = view.component_subject_with_budget(&component.name, work.budget)?;
                let actual_owner = owner(root, &target, &mut work)?;
                let context = root.model_context_with_budget(&[target], work.budget)?;
                let subject = work.join(&[
                    component.package.as_ref(),
                    " using ",
                    context.selected_document(),
                ])?;
                work.room(&mut components)?;
                components.push(Component {
                    name: component.name.as_ref(),
                    owner: actual_owner,
                    subject,
                });
            }
        }
        let mut clauses = Vec::new();
        let mut guarantees = Vec::new();
        let mut guarantee_sources = Vec::new();
        for member in &authored {
            work.visit()?;
            if let CompositionMemberValue::Guarantee(target) = member.value {
                let at = clause(
                    root,
                    view,
                    target,
                    &components,
                    &mut clauses,
                    &mut work,
                    &mut keys,
                )?;
                work.room(&mut guarantees)?;
                guarantees.push(at);
                let source = work.region(&member.source)?;
                work.room(&mut guarantee_sources)?;
                guarantee_sources.push(source);
            }
        }
        let mut premises = Vec::new();
        for role in [SlotRole::Rely, SlotRole::LivenessDependency] {
            for member in &authored {
                work.visit()?;
                let target = match (role, member.value) {
                    (SlotRole::Rely, CompositionMemberValue::Rely(target))
                    | (
                        SlotRole::LivenessDependency,
                        CompositionMemberValue::LivenessDependency(target),
                    ) => target,
                    _ => continue,
                };
                let guarantee = clause(
                    root,
                    view,
                    &target.guarantee,
                    &components,
                    &mut clauses,
                    &mut work,
                    &mut keys,
                )?;
                let on = clause(
                    root,
                    view,
                    &target.on,
                    &components,
                    &mut clauses,
                    &mut work,
                    &mut keys,
                )?;
                let source = work.region(&member.source)?;
                work.room(&mut premises)?;
                premises.push(Premise {
                    guarantee,
                    on,
                    source,
                    role,
                });
            }
        }
        let mut edges = Vec::new();
        for premise in &premises {
            work.visit()?;
            let mut from = None;
            let mut to = None;
            for (at, &known) in guarantees.iter().enumerate() {
                work.visit()?;
                if known == premise.guarantee {
                    from = Some(at);
                }
                if known == premise.on {
                    to = Some(at);
                }
            }
            if let (Some(from), Some(to)) = (from, to) {
                work.room(&mut edges)?;
                edges.push(CompositionEdge {
                    from,
                    to,
                    source: &premise.source,
                });
            }
        }
        let rows =
            composition_cycle_rows(guarantees.len(), &edges, work.budget).map_err(|error| {
                match error {
                    CompositionCycleRefusal::Graph(error) => {
                        PackageCompositionRefusal::Graph(error)
                    }
                    CompositionCycleRefusal::InvalidEdge { .. }
                    | CompositionCycleRefusal::SourceEdgeIndex { .. } => {
                        PackageCompositionRefusal::Member
                    }
                }
            })?;
        let mut cycles = Vec::new();
        for row in rows {
            let mut members = Vec::new();
            for at in row.guarantees {
                work.visit()?;
                let clause = *guarantees
                    .get(at)
                    .ok_or(PackageCompositionRefusal::Member)?;
                work.room(&mut members)?;
                members.push(clause);
            }
            let source = work.region(
                edges
                    .get(row.source_edge)
                    .ok_or(PackageCompositionRefusal::Member)?
                    .source,
            )?;
            work.room(&mut cycles)?;
            cycles.push(PackageCompositionCycle {
                guarantees: members,
                source,
            });
        }
        let node = view.source().occurrence.node().clone();
        let mut result = Self {
            package: work.text(root.package().name())?,
            node,
            rule,
            clauses,
            slots: Vec::new(),
            assumptions: Vec::new(),
            cycles,
            environment: Vec::new(),
            supplies: Vec::new(),
            raw_supplies: Vec::new(),
        };
        if !result.cycles.is_empty() {
            return Ok(result);
        }
        // One actual batch per authenticated owner. No assumed/disposition-only
        // premise constructs an execution world or repeats normalization.
        for component in &components {
            let mut indices = Vec::new();
            let mut names = Vec::new();
            for (at, clause) in result.clauses.iter().enumerate() {
                work.visit()?;
                if clause.owner.package().admission_binding()
                    == component.owner.package().admission_binding()
                    && clause.form.is_none()
                {
                    work.room(&mut indices)?;
                    indices.push(at);
                    work.room(&mut names)?;
                    names.push(clause.name);
                }
            }
            if indices.is_empty() {
                continue;
            }
            let forms = component
                .owner
                .temporal_clause_forms_with_budget(&names, work.budget)?;
            if forms.len() != indices.len() {
                return Err(PackageCompositionRefusal::Member);
            }
            for (at, form) in indices.into_iter().zip(forms) {
                let property = form.property_form_with_budget(work.budget)?;
                if property.is_none() {
                    return Err(PackageCompositionRefusal::Property {
                        component: work.text(result.clauses[at].component)?,
                        clause: work.text(result.clauses[at].name.name.as_ref())?,
                    });
                }
                let clause = &result.clauses[at];
                let fairness = clause
                    .owner
                    .temporal_clause_constraints_with_budget(&clause.name.declaration, work.budget)
                    .map_err(fairness_error)?;
                result.clauses[at].fairness = Some(fairness);
                result.clauses[at].property = property;
                result.clauses[at].form = Some(form);
            }
        }
        for (position, &at) in guarantees.iter().enumerate() {
            let mut assumption_nodes = Vec::new();
            let mut peers = Vec::new();
            if rule == CompositionRule::AssumeGuarantee {
                for premise in &premises {
                    work.visit()?;
                    if premise.guarantee != at {
                        continue;
                    }
                    for (peer_position, &known) in guarantees.iter().enumerate() {
                        work.visit()?;
                        if known != premise.on {
                            continue;
                        }
                        let mut already_assumed = false;
                        for &peer in &peers {
                            work.visit()?;
                            if peer == peer_position {
                                already_assumed = true;
                                break;
                            }
                        }
                        if !already_assumed {
                            work.room(&mut peers)?;
                            peers.push(peer_position);
                        }
                        insert_node(
                            &mut assumption_nodes,
                            &result.clauses[known].node,
                            &mut work,
                        )?;
                    }
                }
            }
            for end in 1..peers.len() {
                let mut next = end;
                while next > 0 {
                    work.visit()?;
                    if peers[next - 1] <= peers[next] {
                        break;
                    }
                    work.visit()?;
                    peers.swap(next - 1, next);
                    next -= 1;
                }
            }
            let clause = &result.clauses[at];
            let form = clause
                .form
                .as_ref()
                .ok_or(PackageCompositionRefusal::Member)?;
            let fairness = key_build::fairness_keys_with_budget(&form.fairness, work.budget)
                .map_err(key_error)?;
            let role = work.text("claim")?;
            let key = key_build::item_key(
                ItemKeyInput {
                    claim: OccurrenceKey::new(clause.node.clone(), Origin::new(Role::new(role), 0)),
                    kind: ClaimKind::TemporalSatisfaction,
                    subject_identity: &clause.subject,
                    assumptions: &assumption_nodes,
                    fairness,
                    stutter_closure: clause.property == Some(PropertyForm::Liveness),
                    correspondence: None,
                    extent: ExtentClass::Unbounded,
                },
                Some(work.budget),
            )
            .map_err(key_error)?;
            let source = work.region(
                guarantee_sources
                    .get(position)
                    .ok_or(PackageCompositionRefusal::Member)?,
            )?;
            let mut derived = base(SlotRole::ComponentGuarantee, source, SlotKey::Item(key));
            derived.backing = SlotBacking::Item;
            work.room(&mut result.slots)?;
            result.slots.push(PackageCompositionSlot {
                derived,
                guarantee: Some(at),
                on: None,
            });
            work.room(&mut result.assumptions)?;
            result.assumptions.push(peers);
        }
        for premise in &premises {
            let guarantee = display(&result.clauses[premise.guarantee], &mut work)?;
            let on = display(&result.clauses[premise.on], &mut work)?;
            let mut members = Vec::new();
            work.room(&mut members)?;
            members.push(guarantee);
            work.room(&mut members)?;
            members.push(on);
            let source = work.region(&premise.source)?;
            let mut derived = base(premise.role, source, local(premise.role, members));
            derived.premise_subject = Some(
                key_build::subject_text_with_budget(
                    &result.clauses[premise.on].subject,
                    &[],
                    work.budget,
                )
                .map_err(key_error)?,
            );
            derived.peer_clause = Some(result.clauses[premise.on].node.clone());
            work.room(&mut result.slots)?;
            result.slots.push(PackageCompositionSlot {
                derived,
                guarantee: Some(premise.guarantee),
                on: Some(premise.on),
            });
        }
        // Each retained constraint is copied by the same admitted fairness
        // reader, which can inspect the borrowed actual declaring identity
        // before allocation; the public semantic getter otherwise owns a copy.
        let mut fairness_owners = Vec::new();
        for (position, &at) in guarantees.iter().enumerate() {
            work.room(&mut fairness_owners)?;
            fairness_owners.push((at, &guarantee_sources[position]));
        }
        for premise in &premises {
            work.room(&mut fairness_owners)?;
            fairness_owners.push((premise.on, &premise.source));
        }
        let mut seen: Vec<(usize, super::key::FairnessKey)> = Vec::new();
        for (at, source) in fairness_owners {
            let clause = &result.clauses[at];
            let constraints = clause
                .owner
                .temporal_clause_fairness_with_budget(&clause.name.declaration, work.budget)
                .map_err(fairness_error)?;
            let keys = key_build::fairness_keys_with_budget(&constraints, work.budget)
                .map_err(key_error)?;
            for (constraint, key) in constraints.into_iter().zip(keys) {
                let mut duplicate = false;
                for (known, value) in &seen {
                    work.visit()?;
                    if *known == at && work.text_equal(value.canonical(), key.canonical())? {
                        duplicate = true;
                        break;
                    }
                }
                if duplicate {
                    continue;
                }
                let guarantee = display(clause, &mut work)?;
                let text = work.text(key.canonical())?;
                let mut members = Vec::new();
                work.room(&mut members)?;
                members.push(guarantee);
                work.room(&mut members)?;
                members.push(text);
                let mut derived = base(
                    SlotRole::Fairness,
                    work.region(source)?,
                    local(SlotRole::Fairness, members),
                );
                derived.constraint = Some(constraint);
                work.room(&mut result.slots)?;
                result.slots.push(PackageCompositionSlot {
                    derived,
                    guarantee: Some(at),
                    on: None,
                });
                work.room(&mut seen)?;
                seen.push((at, key));
            }
        }
        for member in &authored {
            work.visit()?;
            match member.value {
                CompositionMemberValue::Supply(supply) => {
                    let targets = view.supply_contexts_with_budget(supply, work.budget)?;
                    let context = root.model_context_with_budget(&targets, work.budget)?;
                    let constraint = supplied(&context, supply, &result.clauses, &mut work)?;
                    let operation = filament_exact::Identifier::new(work.text(&supply.operation)?)
                        .map_err(|_| PackageCompositionRefusal::Member)?;
                    work.room(&mut result.raw_supplies)?;
                    result.raw_supplies.push(RawSupply {
                        context: context.node().clone(),
                        operation,
                        kind: constraint.kind(),
                        granularity: constraint.granularity(),
                    });
                    work.room(&mut result.supplies)?;
                    result.supplies.push(constraint);
                }
                CompositionMemberValue::Environment(environment) => {
                    let supplies = match &environment.supplies {
                        CheckedEnvironmentSupplies::Fairness(supply) => {
                            let targets = view.supply_contexts_with_budget(supply, work.budget)?;
                            let context = root.model_context_with_budget(&targets, work.budget)?;
                            let constraint =
                                supplied(&context, supply, &result.clauses, &mut work)?;
                            let operation = OperationRef {
                                context: context.node().clone(),
                                operation: work.text(&supply.operation)?,
                            };
                            EnvSupplies::Fairness(SchedulingPremise {
                                constraint,
                                operation,
                            })
                        }
                        CheckedEnvironmentSupplies::Assumption(_) => {
                            let target =
                                view.environment_assumption_with_budget(environment, work.budget)?;
                            EnvSupplies::Assumption(work.key(
                                target.owner(),
                                target.node(),
                                &mut keys,
                            )?)
                        }
                    };
                    let name = work.text(&environment.name)?;
                    let supplier = work.text(&environment.supplier)?;
                    let local_name = work.text(&environment.name)?;
                    let mut members = Vec::new();
                    work.room(&mut members)?;
                    members.push(local_name);
                    let mut derived = base(
                        SlotRole::Environment,
                        work.region(&member.source)?,
                        local(SlotRole::Environment, members),
                    );
                    derived.backing = SlotBacking::Environment {
                        member: Some(work.text(&environment.name)?),
                    };
                    work.room(&mut result.environment)?;
                    result.environment.push(EnvironmentMember {
                        name,
                        supplier,
                        supplies,
                    });
                    work.room(&mut result.slots)?;
                    result.slots.push(PackageCompositionSlot {
                        derived,
                        guarantee: None,
                        on: None,
                    });
                }
                _ => {}
            }
        }
        Ok(result)
    }
}
fn insert_node(
    nodes: &mut Vec<NodeKey>,
    node: &NodeKey,
    work: &mut Work<'_>,
) -> Result<(), PackageCompositionRefusal> {
    let mut at = 0;
    while let Some(known) = nodes.get(at) {
        work.visit()?;
        match known.cmp(node) {
            std::cmp::Ordering::Equal => return Ok(()),
            std::cmp::Ordering::Greater => break,
            std::cmp::Ordering::Less => at += 1,
        }
    }
    work.room(nodes)?;
    nodes.push(node.clone());
    let mut next = nodes.len() - 1;
    while next > at {
        work.visit()?;
        nodes.swap(next - 1, next);
        next -= 1;
    }
    Ok(())
}
fn supplied(
    context: &fsl_package::CompositionModelContext<'_>,
    supply: &filament_contract_model::CheckedFairnessSupply,
    clauses: &[PackageCompositionClause<'_, '_>],
    work: &mut Work<'_>,
) -> Result<FairnessConstraint, PackageCompositionRefusal> {
    let mut matching = None;
    for clause in clauses {
        work.visit()?;
        let facts = clause
            .fairness
            .as_ref()
            .ok_or(PackageCompositionRefusal::Member)?;
        for (position, entry) in facts.constraints().iter().enumerate() {
            work.visit()?;
            if entry.context == *context.node()
                && work.text_equal(entry.operation.as_str(), &supply.operation)?
            {
                let identity = facts
                    .declaring_at(position)
                    .ok_or(PackageCompositionRefusal::Member)?;
                matching = Some((&entry.constraint, identity));
                break;
            }
        }
        if matching.is_some() {
            break;
        }
    }
    let declaring = if let Some((constraint, identity)) = matching {
        // This retained row constructed this exact constraint; inherited or
        // redeclared operation identities are never inferred from receivers.
        work.effective_cost(identity)?;
        constraint.operation().declaring_type()
    } else {
        let display =
            key_build::node_display_text(context.node(), Some(work.budget)).map_err(key_error)?;
        let package = work.text(context.node().package())?;
        work.charge(NativeJoinChargeKind::Storage, 2)?;
        EffectiveId::new(None, OriginalDeclarationKey::new(package, display))
    };
    let operation = work.text(&supply.operation)?;
    Ok(FairnessConstraint::new(
        match supply.kind {
            CheckedFairnessKind::Weak => FairnessKind::Weak,
            CheckedFairnessKind::Strong => FairnessKind::Strong,
        },
        FairnessOperation::new(declaring, operation),
        match supply.granularity {
            CheckedFairnessGranularity::Whole => FairnessGranularity::Whole,
            CheckedFairnessGranularity::Each => FairnessGranularity::Each,
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::super::admitted_fixture::{AdmittedCompositionFixture, CompositionCase};
    use super::*;
    use fsl_package::AnalysisView;

    /// Trace: FR-1044-AC-10, FSpec-FR-491-AC-6
    #[test]
    fn admitted_slots_match_actual_source_keys_order_assumptions_and_regions() {
        let fixture = AdmittedCompositionFixture::new(CompositionCase::Acyclic);
        let mut budget = NativeJoinBudget::new(10_000_000, 10_000_000);
        let bare = fixture
            .bare(&mut budget)
            .expect("genuine supplied owner views");
        let named = bare.claim_name("Sys").expect("actual composition name");
        let view = bare
            .composition_with_budget(&named.declaration, &mut budget)
            .expect("bounded admitted view");
        let projected = PackageComposition::from_view(&bare, &view, &mut budget)
            .expect("actual admitted premise projection");
        assert!(projected.cycles().is_empty());
        let source = fixture
            .source
            .graph()
            .compositions()
            .iter()
            .find(|c| c.name() == "Sys")
            .expect("source oracle");
        assert_eq!(projected.node(), source.node());
        assert_eq!(projected.rule(), source.rule());
        assert_eq!(projected.assumptions(), source.item().assumptions);
        for actual in projected.clauses() {
            let expected = source
                .clauses()
                .iter()
                .find(|fact| {
                    fact.reference.component == actual.component
                        && fact.reference.node == actual.node
                })
                .expect("same authenticated source clause");
            assert_eq!(
                actual
                    .fairness
                    .as_ref()
                    .expect("prepared fairness facts")
                    .constraints(),
                expected.fairness,
                "actual context/operation/declaring identity and source order"
            );
            let facts = actual
                .fairness
                .as_ref()
                .expect("prepared exact declarations");
            for (position, constraint) in facts.constraints().iter().enumerate() {
                assert_eq!(
                    facts.declaring_at(position),
                    Some(&constraint.constraint.operation().declaring_type())
                );
            }
            assert!(facts.declaring_at(facts.constraints().len()).is_none());
        }
        let expected = super::super::composition_premises(&fixture.source, source.node())
            .expect("actual source slots");
        assert_eq!(projected.slots().len(), expected.len());
        for (actual, expected) in projected.slots().iter().zip(expected) {
            assert_eq!(actual.derived.role, expected.derived.role);
            assert_eq!(actual.derived.key, expected.derived.key);
            assert_eq!(actual.derived.backing, expected.derived.backing);
            assert_eq!(actual.derived.constraint, expected.derived.constraint);
            assert_eq!(actual.derived.operation, expected.derived.operation);
            assert_eq!(
                actual.derived.premise_subject,
                expected.derived.premise_subject
            );
            assert_eq!(actual.derived.peer_clause, expected.derived.peer_clause);
            assert_eq!(actual.derived.source, expected.derived.source);
        }
        let shared = projected
            .clauses()
            .iter()
            .filter(|c| c.name.name.as_ref() == "Shared")
            .collect::<Vec<_>>();
        assert_eq!(shared.len(), 2);
        assert_ne!(
            shared[0].owner.package().name(),
            shared[1].owner.package().name()
        );
        assert_ne!(
            shared[0].node, shared[1].node,
            "same names never replace authentic owning identities"
        );
    }

    /// Trace: FR-1044-AC-10, FSpec-FR-491-AC-6
    #[test]
    fn admitted_unused_supply_keeps_source_original_declaration_bytes() {
        let fixture = AdmittedCompositionFixture::new(CompositionCase::UnusedSupply);
        let mut budget = NativeJoinBudget::new(10_000_000, 10_000_000);
        let bare = fixture.bare(&mut budget).expect("genuine owners");
        let named = bare.claim_name("Sys").expect("actual composition");
        let view = bare
            .composition_with_budget(&named.declaration, &mut budget)
            .expect("actual view");
        let projected = PackageComposition::from_view(&bare, &view, &mut budget)
            .expect("actual supply projection");
        let source = fixture
            .source
            .graph()
            .compositions()
            .iter()
            .find(|c| c.name() == "Sys")
            .expect("source oracle");
        let declaration =
            super::super::PlannerDeclaration::from_package(&fixture.source, source.node())
                .expect("unchanged source declaration");
        assert_eq!(projected.supplies(), declaration.supplies);
        let expected_bytes = declaration
            .supplies
            .iter()
            .map(|c| super::super::key::FairnessKey::of(c).expect("source canonical bytes"))
            .collect::<Vec<_>>();
        let actual_bytes = key_build::fairness_keys_with_budget(projected.supplies(), &mut budget)
            .expect("native canonical bytes");
        assert_eq!(actual_bytes, expected_bytes);
        let supply = source
            .supplies()
            .last()
            .expect("actual unused receive supply");
        assert_eq!(supply.operation.as_str(), "receive");
        assert!(
            source
                .clauses()
                .iter()
                .flat_map(|c| &c.fairness)
                .all(|c| c.context != supply.context || c.operation != supply.operation),
            "fallback is exercised by a genuine operation absent from every F_P"
        );
        let expected = EffectiveId::new(
            None,
            OriginalDeclarationKey::new(supply.context.package(), supply.context.to_string()),
        );
        assert_eq!(projected.supplies().last().expect("unused supplied constraint").operation().declaring_type(), expected, "canonical original-declaration spelling is independent of authenticated execution ownership");
    }

    /// Trace: FR-1044-AC-10, FSpec-FR-492-AC-1
    #[test]
    fn admitted_mismatch_recomputes_full_source_witness_from_actual_facts() {
        for (case, reason) in [
            (CompositionCase::Acyclic, None),
            (
                CompositionCase::MissingInterface,
                Some(super::super::MismatchReason::Absent),
            ),
            (
                CompositionCase::MissingSupply,
                Some(super::super::MismatchReason::Weaker),
            ),
        ] {
            let fixture = AdmittedCompositionFixture::new(case);
            let mut budget = NativeJoinBudget::new(10_000_000, 10_000_000);
            let bare = fixture.bare(&mut budget).expect("genuine owners");
            let named = bare.claim_name("Sys").expect("actual composition");
            let view = bare
                .composition_with_budget(&named.declaration, &mut budget)
                .expect("actual view");
            let projected = PackageComposition::from_view(&bare, &view, &mut budget)
                .expect("complete authenticated facts");
            let source = fixture
                .source
                .graph()
                .compositions()
                .iter()
                .find(|c| c.name() == "Sys")
                .expect("source oracle");
            let mut measured = NativeJoinBudget::new(1_000_000, 1_000_000);
            let actual = projected
                .interface_mismatch_with_budget(&mut measured)
                .expect("bounded authenticated mismatch");
            assert_eq!(actual, super::super::interface_mismatch(source));
            assert_eq!(actual.as_ref().map(|witness| witness.reason), reason);
            let paid = measured.consumed();
            let mut exact = NativeJoinBudget::new(paid.work, paid.storage);
            assert_eq!(
                projected
                    .interface_mismatch_with_budget(&mut exact)
                    .expect("exact cumulative allowance"),
                actual
            );
            for (work, storage) in [(paid.work - 1, paid.storage), (paid.work, paid.storage - 1)] {
                let mut short = NativeJoinBudget::new(work, storage);
                let error = projected
                    .interface_mismatch_with_budget(&mut short)
                    .expect_err("first unpaid fact or decision operation");
                let (denied, locus, configured_bound) = match error {
                    PackageCompositionRefusal::Native {
                        denied,
                        locus,
                        configured_bound,
                    }
                    | PackageCompositionRefusal::Mismatch(
                        super::super::MismatchRefusal::Native {
                            denied,
                            locus,
                            configured_bound,
                        },
                    ) => (denied, locus, configured_bound),
                    other => panic!("native refusal metadata was lost: {other:?}"),
                };
                assert_eq!(Some(denied), short.first_unpaid());
                assert_eq!(locus, short.first_unpaid_locus());
                assert_eq!(
                    configured_bound,
                    match denied.kind {
                        NativeJoinChargeKind::Work => work,
                        NativeJoinChargeKind::Storage => storage,
                    }
                );
            }
        }
    }

    /// Trace: FR-1044-AC-10, FR-515-AC-1
    #[test]
    fn admitted_step_intervals_use_the_run_form_and_source_slot_keys() {
        let fixture = AdmittedCompositionFixture::new(CompositionCase::StepBounded);
        let mut budget = NativeJoinBudget::new(10_000_000, 10_000_000);
        let bare = fixture.bare(&mut budget).expect("genuine owners");
        let named = bare.claim_name("Sys").expect("actual composition");
        let view = bare
            .composition_with_budget(&named.declaration, &mut budget)
            .expect("actual view");
        let projected = PackageComposition::from_view(&bare, &view, &mut budget)
            .expect("bounded run form remains supported");
        let source = fixture
            .source
            .graph()
            .compositions()
            .iter()
            .find(|c| c.name() == "Sys")
            .expect("source oracle");
        let expected = super::super::composition_premises(&fixture.source, source.node())
            .expect("source slots");
        assert_eq!(projected.slots().len(), expected.len());
        for (actual, expected) in projected.slots().iter().zip(expected) {
            assert_eq!(actual.derived.key, expected.derived.key);
        }
        let clause = projected
            .clauses()
            .iter()
            .find(|c| c.component == "Producer" && c.name.name.as_ref() == "Shared")
            .expect("bounded owner clause");
        assert!(clause
            .form
            .as_ref()
            .expect("run form")
            .formula
            .iter()
            .any(|node| matches!(
                node,
                fsl_semantics::check::PackageFormulaNode::Unary {
                    interval: Some(_),
                    ..
                }
            )));
        assert_eq!(clause.property, Some(PropertyForm::Safety));
    }

    /// Trace: FR-1044-AC-6, FR-1044-AC-10
    #[test]
    fn admitted_cycles_match_actual_source_tail_and_first_authored_edge() {
        let fixture = AdmittedCompositionFixture::new(CompositionCase::Cyclic);
        let mut budget = NativeJoinBudget::new(10_000_000, 10_000_000);
        let bare = fixture.bare(&mut budget).expect("genuine owner views");
        let named = bare.claim_name("Sys").expect("actual name");
        let view = bare
            .composition_with_budget(&named.declaration, &mut budget)
            .expect("actual view");
        let projected = PackageComposition::from_view(&bare, &view, &mut budget)
            .expect("actual cycle projection");
        let source = fixture
            .source
            .graph()
            .compositions()
            .iter()
            .find(|c| c.name() == "Sys")
            .expect("source oracle");
        assert_eq!(projected.cycles().len(), source.cycles().len());
        assert!(
            projected.slots().is_empty(),
            "cycles stop before premise slots or execution forms"
        );
        assert!(projected
            .clauses()
            .iter()
            .all(|clause| clause.form.is_none()));
        for (actual, expected) in projected.cycles().iter().zip(source.cycles()) {
            let actual_nodes = actual
                .guarantees
                .iter()
                .map(|&at| projected.clauses()[at].node.clone())
                .collect::<Vec<_>>();
            assert_eq!(
                actual_nodes,
                expected
                    .guarantees
                    .iter()
                    .map(|r| r.node.clone())
                    .collect::<Vec<_>>()
            );
            assert_eq!(actual.source.source(), fixture.source.graph().source());
            assert_eq!(
                actual.source.start(),
                u64::try_from(expected.source.start).expect("source start")
            );
            assert_eq!(
                actual.source.end(),
                u64::try_from(expected.source.end).expect("source end")
            );
            assert_eq!(
                &fixture.source_text[expected.source.start..expected.source.end],
                "depends Consumer::Done on Producer::Sends;"
            );
        }
    }

    /// Trace: FR-251-AC-49, FR-1044-AC-10
    #[test]
    fn admitted_slot_projection_preserves_the_same_remaining_native_allowance() {
        let fixture = AdmittedCompositionFixture::new(CompositionCase::Acyclic);
        let mut setup = NativeJoinBudget::new(10_000_000, 10_000_000);
        let bare = fixture.bare(&mut setup).expect("owning views");
        let named = bare.claim_name("Sys").expect("name");
        let view = bare
            .composition_with_budget(&named.declaration, &mut setup)
            .expect("view");
        let mut measured = NativeJoinBudget::new(10_000_000, 10_000_000);
        let actual =
            PackageComposition::from_view(&bare, &view, &mut measured).expect("paid metadata");
        let paid = measured.consumed();
        let mut exact = NativeJoinBudget::new(paid.work, paid.storage);
        assert_eq!(
            PackageComposition::from_view(&bare, &view, &mut exact)
                .expect("exact allowance")
                .slots()
                .len(),
            actual.slots().len()
        );
        assert_eq!(exact.consumed(), paid);
        for (work, storage) in [(paid.work - 1, 10_000_000), (10_000_000, paid.storage - 1)] {
            let mut short = NativeJoinBudget::new(work, storage);
            assert!(PackageComposition::from_view(&bare, &view, &mut short).is_err());
            let denied = short.first_unpaid().expect("original unpaid native charge");
            assert_eq!(
                denied.kind,
                if work == paid.work - 1 {
                    NativeJoinChargeKind::Work
                } else {
                    NativeJoinChargeKind::Storage
                }
            );
            assert!(short.first_unpaid_locus().is_some());
        }
    }
}
