// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-940, FSpec FR-491: borrowed admitted composition facts and their owners.

use filament_contract_model::{
    CheckedComponentClause, CheckedCompositionComponent, CheckedCompositionPremise,
    CheckedEnvironmentEntry, CheckedFairnessSupply, CheckedNodeId, CheckedNodePathSegment,
    CheckedOccurrence, CheckedOccurrenceRole, CheckedOperandEntry, CheckedPackageV2, CheckedTerm,
    ClaimNameRow, NativeJoinBudget, NativeJoinLocus, TemporalClauseName,
};

use crate::native_row::{self, NativeRowError};
use crate::occurrence_source::{self, OccurrenceSource, OccurrenceSourceRefusal};

/// Why an admitted node cannot supply the requested composition fact.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum CompositionViewRefusal {
    /// The selected document-held node is absent or has another form.
    #[error("the selected node is not a composition declaration")]
    NotComposition,
    /// An admitted composition has no matching declaration naming row.
    #[error("the composition declaration has no admitted name")]
    MissingName,
    /// A component label does not belong to the composition.
    #[error("the composition has no such component")]
    UnknownComponent,
    /// The authenticated component package is not retained as a dependency.
    #[error("the composition component owner is unavailable")]
    MissingOwner,
    /// A member target does not resolve in its authenticated owning package.
    #[error("the composition target does not belong to its owner")]
    UnknownTarget,
    /// The authentic clause node has several named claim occurrences and
    /// the composition's NodeRef does not select one of them.
    #[error("component {component} clause {clause:?} has multiple admitted naming occurrences")]
    AmbiguousClause {
        /// The admitted composition component label.
        component: String,
        /// The authentic owner-held clause reference.
        clause: CheckedNodeId,
    },
    /// A canonical member position cannot be represented as an occurrence.
    #[error("the composition member position is out of range")]
    MemberOrdinal,
    /// The selected occurrence lacks its exact admitted source attribution.
    #[error(transparent)]
    Source(#[from] OccurrenceSourceRefusal),
    /// The invocation's existing native work or logical storage was exhausted.
    #[error(transparent)]
    Native(#[from] NativeRowError),
}

impl From<occurrence_source::MeteredOccurrenceSourceRefusal> for CompositionViewRefusal {
    fn from(value: occurrence_source::MeteredOccurrenceSourceRefusal) -> Self {
        match value {
            occurrence_source::MeteredOccurrenceSourceRefusal::Source(source) => {
                Self::Source(source)
            }
            occurrence_source::MeteredOccurrenceSourceRefusal::Native(native) => {
                Self::Native(native)
            }
        }
    }
}

struct ProjectionWork<'a> {
    budget: Option<&'a mut NativeJoinBudget>,
    locus: Option<NativeJoinLocus>,
}

impl<'a> ProjectionWork<'a> {
    fn unmetered() -> Self {
        Self {
            budget: None,
            locus: None,
        }
    }
    fn metered(budget: &'a mut NativeJoinBudget) -> Result<Self, CompositionViewRefusal> {
        let locus = native_row::start(budget, "component_composition")?;
        Ok(Self {
            budget: Some(budget),
            locus: Some(locus),
        })
    }
    fn visit(&mut self) -> Result<(), CompositionViewRefusal> {
        if let (Some(budget), Some(locus)) = (&mut self.budget, self.locus) {
            native_row::work(budget, locus, 1)?;
        }
        Ok(())
    }
    fn node(&mut self, node: &CheckedNodeId) -> Result<(), CompositionViewRefusal> {
        if let (Some(budget), Some(locus)) = (&mut self.budget, self.locus) {
            native_row::node(budget, locus, node)?;
        }
        Ok(())
    }
    fn text(&mut self, text: &str) -> Result<(), CompositionViewRefusal> {
        if let (Some(budget), Some(locus)) = (&mut self.budget, self.locus) {
            native_row::text(budget, locus, text)?;
        }
        Ok(())
    }
}

/// A resolved reference together with its exact admitted owning package.
/// Construction is private so an importing graph cannot become its owner.
#[derive(Clone)]
pub struct OwnedNodeRef<'p> {
    owner: &'p CheckedPackageV2,
    node: CheckedNodeId,
    name: Option<&'p TemporalClauseName>,
}

impl<'p> OwnedNodeRef<'p> {
    /// The exact retained owning package.
    pub fn owner(&self) -> &'p CheckedPackageV2 {
        self.owner
    }

    /// The resolved dependency-held ID, accepted by the owner's `term`,
    /// `node_kind` and `structural_key` lookups. It is not an expanded key.
    pub fn node(&self) -> &CheckedNodeId {
        &self.node
    }

    /// The exact unique owning claim-name row of a clause reference.
    /// Component subject references have no clause naming row.
    pub fn clause_name(&self) -> Option<&'p TemporalClauseName> {
        self.name
    }
}

/// One borrowed entry from the six separate canonical composition arrays.
#[derive(Clone, Copy, Debug)]
pub enum CompositionMemberValue<'p> {
    /// A component and its selected subject.
    Component(&'p CheckedCompositionComponent),
    /// A declared guarantee.
    Guarantee(&'p CheckedComponentClause),
    /// A safety premise.
    Rely(&'p CheckedCompositionPremise),
    /// A liveness premise.
    LivenessDependency(&'p CheckedCompositionPremise),
    /// A supplied scheduling constraint.
    Supply(&'p CheckedFairnessSupply),
    /// An environment premise.
    Environment(&'p CheckedEnvironmentEntry),
}

/// A canonical member with its exact Generated occurrence and authored region.
#[derive(Clone, Debug)]
pub struct CompositionMember<'p> {
    value: CompositionMemberValue<'p>,
    occurrence: CheckedOccurrence,
    source: OccurrenceSource,
}

impl<'p> CompositionMember<'p> {
    /// The admitted typed member, without a second body representation.
    pub fn value(&self) -> CompositionMemberValue<'p> {
        self.value
    }

    /// The member's canonical concatenation coordinate.
    pub fn occurrence(&self) -> &CheckedOccurrence {
        &self.occurrence
    }

    /// The member's authored source region and structural occurrence key.
    /// Compare these regions within a member kind to recover authored order.
    pub fn source(&self) -> &OccurrenceSource {
        &self.source
    }
}

/// A shallow read of one admitted composition, independent of source graphs.
pub struct CompositionView<'p> {
    package: &'p CheckedPackageV2,
    node: &'p CheckedNodeId,
    holder: usize,
    name: &'p ClaimNameRow,
    source: OccurrenceSource,
    rule: &'p str,
    components: &'p [CheckedCompositionComponent],
    guarantees: &'p [CheckedComponentClause],
    relies: &'p [CheckedCompositionPremise],
    liveness_dependencies: &'p [CheckedCompositionPremise],
    supplies: &'p [CheckedFairnessSupply],
    environment: &'p [CheckedEnvironmentEntry],
}

impl<'p> CompositionView<'p> {
    /// The admitted package declaring this composition.
    pub fn package(&self) -> &'p CheckedPackageV2 {
        self.package
    }

    /// The composition's document-held declaration ID.
    pub fn node(&self) -> &'p CheckedNodeId {
        self.node
    }

    /// The admitted declaration name, qualified path and naming occurrence.
    pub fn name(&self) -> &'p ClaimNameRow {
        self.name
    }

    /// The whole declaration's Declaration/0 attribution.
    pub fn source(&self) -> &OccurrenceSource {
        &self.source
    }

    /// The admitted composition rule spelling.
    pub fn rule(&self) -> &'p str {
        self.rule
    }

    /// The six typed arrays in canonical concatenation order. This is not
    /// authored order: each member's source region records that separately.
    /// Attribution is read only as each member is visited.
    pub fn members(
        &self,
    ) -> impl Iterator<Item = Result<CompositionMember<'p>, CompositionViewRefusal>> + '_ {
        self.members_with_work(Ok(ProjectionWork::unmetered()))
    }

    /// Read each visited member's attribution against the same cumulative
    /// native budget as admission and projection. Denial precedes its work.
    pub fn members_with_budget<'a>(
        &'a self,
        budget: &'a mut NativeJoinBudget,
    ) -> impl Iterator<Item = Result<CompositionMember<'p>, CompositionViewRefusal>> + 'a {
        self.members_with_work(ProjectionWork::metered(budget))
    }

    fn members_with_work<'a>(
        &'a self,
        mut work: Result<ProjectionWork<'a>, CompositionViewRefusal>,
    ) -> impl Iterator<Item = Result<CompositionMember<'p>, CompositionViewRefusal>> + 'a {
        self.components
            .iter()
            .map(CompositionMemberValue::Component)
            .chain(
                self.guarantees
                    .iter()
                    .map(CompositionMemberValue::Guarantee),
            )
            .chain(self.relies.iter().map(CompositionMemberValue::Rely))
            .chain(
                self.liveness_dependencies
                    .iter()
                    .map(CompositionMemberValue::LivenessDependency),
            )
            .chain(self.supplies.iter().map(CompositionMemberValue::Supply))
            .chain(
                self.environment
                    .iter()
                    .map(CompositionMemberValue::Environment),
            )
            .enumerate()
            .map(move |(index, value)| {
                let work = work.as_mut().map_err(|error| error.clone())?;
                work.visit()?;
                let occurrence = CheckedOccurrence {
                    role: CheckedOccurrenceRole::Generated,
                    ordinal: u64::try_from(index)
                        .map_err(|_| CompositionViewRefusal::MemberOrdinal)?,
                };
                let source = match &mut work.budget {
                    Some(budget) => occurrence_source::composition_occurrence_source(
                        self.package,
                        self.node,
                        &occurrence,
                        self.source.occurrence.node(),
                        budget,
                    )
                    .map_err(CompositionViewRefusal::from)?,
                    None => occurrence_source::occurrence_source(
                        self.package,
                        self.node,
                        &occurrence,
                        None,
                    )?,
                };
                Ok(CompositionMember {
                    value,
                    occurrence,
                    source,
                })
            })
    }

    /// Resolve a named component's subject in its authenticated owner.
    pub fn component_subject(
        &self,
        component: &str,
    ) -> Result<OwnedNodeRef<'p>, CompositionViewRefusal> {
        self.component_subject_with_work(component, &mut ProjectionWork::unmetered())
    }

    /// Resolve a subject while charging actual component/dependency visits
    /// and the small held reference copy to the caller's native budget.
    pub fn component_subject_with_budget(
        &self,
        component: &str,
        budget: &mut NativeJoinBudget,
    ) -> Result<OwnedNodeRef<'p>, CompositionViewRefusal> {
        self.component_subject_with_work(component, &mut ProjectionWork::metered(budget)?)
    }

    fn component_subject_with_work(
        &self,
        name: &str,
        work: &mut ProjectionWork<'_>,
    ) -> Result<OwnedNodeRef<'p>, CompositionViewRefusal> {
        let component = self.component(name, work)?;
        self.target(component, &component.subject, work)
    }

    /// Resolve a component clause from this composition's typed body in the
    /// component's owner, never in the importing root graph.
    pub fn clause(
        &self,
        clause: &CheckedComponentClause,
    ) -> Result<OwnedNodeRef<'p>, CompositionViewRefusal> {
        self.clause_with_work(clause, &mut ProjectionWork::unmetered())
    }

    /// Resolve an admitted clause using the caller's cumulative work/storage
    /// budget, preserving native limit denials as typed refusals.
    pub fn clause_with_budget(
        &self,
        clause: &CheckedComponentClause,
        budget: &mut NativeJoinBudget,
    ) -> Result<OwnedNodeRef<'p>, CompositionViewRefusal> {
        self.clause_with_work(clause, &mut ProjectionWork::metered(budget)?)
    }

    fn clause_with_work(
        &self,
        clause: &CheckedComponentClause,
        work: &mut ProjectionWork<'_>,
    ) -> Result<OwnedNodeRef<'p>, CompositionViewRefusal> {
        let candidates = self.guarantees.iter().chain(
            self.relies
                .iter()
                .chain(self.liveness_dependencies.iter())
                .flat_map(|premise| [&premise.guarantee, &premise.on]),
        );
        let mut admitted = false;
        for entry in candidates {
            work.visit()?;
            if std::ptr::eq(entry, clause) {
                admitted = true;
                break;
            }
        }
        if !admitted {
            return Err(CompositionViewRefusal::UnknownTarget);
        }
        let component = self.component(&clause.component, work)?;
        let mut target = self.target(component, &clause.clause, work)?;
        let key = target
            .owner
            .structural_key(&target.node)
            .ok_or(CompositionViewRefusal::UnknownTarget)?;
        let mut name = None;
        for row in target.owner.temporal_clause_names().unwrap_or_default() {
            work.visit()?;
            if target.owner.structural_key(&row.declaration) == Some(key) {
                if name.is_some() {
                    work.text(&clause.component)?;
                    work.node(&target.node)?;
                    return Err(CompositionViewRefusal::AmbiguousClause {
                        component: clause.component.to_string(),
                        clause: target.node.clone(),
                    });
                }
                name = Some(row);
            }
        }
        target.name = Some(name.ok_or(CompositionViewRefusal::UnknownTarget)?);
        Ok(target)
    }

    fn component(
        &self,
        name: &str,
        work: &mut ProjectionWork<'_>,
    ) -> Result<&'p CheckedCompositionComponent, CompositionViewRefusal> {
        for component in self.components {
            work.visit()?;
            if component.name.as_ref() == name {
                return Ok(component);
            }
        }
        Err(CompositionViewRefusal::UnknownComponent)
    }

    fn target(
        &self,
        component: &CheckedCompositionComponent,
        target: &CheckedNodeId,
        work: &mut ProjectionWork<'_>,
    ) -> Result<OwnedNodeRef<'p>, CompositionViewRefusal> {
        let mut owner = None;
        for dependency in self.package.direct_dependencies() {
            work.visit()?;
            if dependency.name() == component.package.as_ref() {
                owner = Some(dependency);
                break;
            }
        }
        let owner = owner.ok_or(CompositionViewRefusal::MissingOwner)?;
        work.node(target)?;
        // resolve_node_held copies dependency operand package strings, while
        // local index/structural operands retain shared handles.
        for segment in &target.path {
            if let CheckedNodePathSegment::Operand(slot) = segment {
                let operand = usize::try_from(*slot)
                    .ok()
                    .and_then(|slot| {
                        self.package
                            .resolved_graph()
                            .operand_rows
                            .get(self.holder)
                            .and_then(|row| row.get(slot))
                    })
                    .ok_or(CompositionViewRefusal::UnknownTarget)?;
                if let CheckedOperandEntry::Dependency(operand) = operand {
                    work.text(&operand.package)?;
                }
            }
        }
        let reference = self
            .package
            .resolve_node_held(self.holder, target)
            .ok_or(CompositionViewRefusal::UnknownTarget)?;
        owner
            .structural_key(&reference)
            .ok_or(CompositionViewRefusal::UnknownTarget)?;
        Ok(OwnedNodeRef {
            owner,
            node: reference,
            name: None,
        })
    }
}

pub(crate) fn composition_view<'p>(
    package: &'p CheckedPackageV2,
    selected: &CheckedNodeId,
) -> Result<CompositionView<'p>, CompositionViewRefusal> {
    composition_view_with_work(package, selected, &mut ProjectionWork::unmetered())
}

pub(crate) fn composition_view_with_budget<'p>(
    package: &'p CheckedPackageV2,
    selected: &CheckedNodeId,
    budget: &mut NativeJoinBudget,
) -> Result<CompositionView<'p>, CompositionViewRefusal> {
    composition_view_with_work(package, selected, &mut ProjectionWork::metered(budget)?)
}

fn composition_view_with_work<'p>(
    package: &'p CheckedPackageV2,
    selected: &CheckedNodeId,
    work: &mut ProjectionWork<'_>,
) -> Result<CompositionView<'p>, CompositionViewRefusal> {
    let mut found = None;
    for (holder, node) in package.resolved_graph().nodes.iter().enumerate() {
        work.visit()?;
        if node.node_id == *selected {
            found = Some((holder, node));
            break;
        }
    }
    let (holder, node) = found.ok_or(CompositionViewRefusal::NotComposition)?;
    let CheckedTerm::ComponentComposition {
        rule,
        components,
        guarantees,
        relies,
        liveness_dependencies,
        supplies,
        environment,
    } = package
        .term(&node.node_id)
        .ok_or(CompositionViewRefusal::NotComposition)?
    else {
        return Err(CompositionViewRefusal::NotComposition);
    };
    let occurrence = CheckedOccurrence {
        role: CheckedOccurrenceRole::Declaration,
        ordinal: 0,
    };
    let mut name = None;
    for row in package.claim_names().unwrap_or_default() {
        work.visit()?;
        if row.declaration == node.node_id && row.occurrence == occurrence {
            name = Some(row);
            break;
        }
    }
    let name = name.ok_or(CompositionViewRefusal::MissingName)?;
    let source = if work.budget.is_some() {
        let structural = package
            .structural_key(&node.node_id)
            .ok_or(CompositionViewRefusal::NotComposition)?;
        work.node(structural)?;
        let key = fsl_semantics::check::wire_id::node_key(structural)
            .map_err(|_| CompositionViewRefusal::NotComposition)?;
        occurrence_source::composition_occurrence_source(
            package,
            &node.node_id,
            &occurrence,
            &key,
            work.budget
                .as_deref_mut()
                .ok_or(CompositionViewRefusal::NotComposition)?,
        )?
    } else {
        occurrence_source::occurrence_source(package, &node.node_id, &occurrence, None)?
    };
    Ok(CompositionView {
        package,
        node: &node.node_id,
        holder,
        name,
        source,
        rule,
        components,
        guarantees,
        relies,
        liveness_dependencies,
        supplies,
        environment,
    })
}
