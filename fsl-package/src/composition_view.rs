// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-940, FSpec FR-491: borrowed admitted composition facts and their owners.

use filament_contract_model::{
    CheckedComponentClause, CheckedCompositionComponent, CheckedCompositionPremise,
    CheckedEnvironmentEntry, CheckedFairnessSupply, CheckedNodeId, CheckedOccurrence,
    CheckedOccurrenceRole, CheckedPackageV2, CheckedTerm, ClaimNameRow,
};

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
    /// A canonical member position cannot be represented as an occurrence.
    #[error("the composition member position is out of range")]
    MemberOrdinal,
    /// The selected occurrence lacks its exact admitted source attribution.
    #[error(transparent)]
    Source(#[from] OccurrenceSourceRefusal),
}

/// A resolved reference together with its exact admitted owning package.
/// Construction is private so an importing graph cannot become its owner.
#[derive(Clone)]
pub struct OwnedNodeRef<'p> {
    owner: &'p CheckedPackageV2,
    node: CheckedNodeId,
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
            .map(|(index, value)| {
                let occurrence = CheckedOccurrence {
                    role: CheckedOccurrenceRole::Generated,
                    ordinal: u64::try_from(index)
                        .map_err(|_| CompositionViewRefusal::MemberOrdinal)?,
                };
                let source = occurrence_source::occurrence_source(
                    self.package,
                    self.node,
                    &occurrence,
                    None,
                )?;
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
        let component = self.component(component)?;
        self.target(component, &component.subject)
    }

    /// Resolve a component clause from this composition's typed body in the
    /// component's owner, never in the importing root graph.
    pub fn clause(
        &self,
        clause: &CheckedComponentClause,
    ) -> Result<OwnedNodeRef<'p>, CompositionViewRefusal> {
        let admitted = self
            .guarantees
            .iter()
            .chain(
                self.relies
                    .iter()
                    .chain(self.liveness_dependencies.iter())
                    .flat_map(|premise| [&premise.guarantee, &premise.on]),
            )
            .any(|entry| std::ptr::eq(entry, clause));
        if !admitted {
            return Err(CompositionViewRefusal::UnknownTarget);
        }
        let component = self.component(&clause.component)?;
        self.target(component, &clause.clause)
    }

    fn component(
        &self,
        name: &str,
    ) -> Result<&'p CheckedCompositionComponent, CompositionViewRefusal> {
        self.components
            .iter()
            .find(|component| component.name.as_ref() == name)
            .ok_or(CompositionViewRefusal::UnknownComponent)
    }

    fn target(
        &self,
        component: &CheckedCompositionComponent,
        target: &CheckedNodeId,
    ) -> Result<OwnedNodeRef<'p>, CompositionViewRefusal> {
        let owner = self
            .package
            .direct_dependencies()
            .find(|owner| owner.name() == component.package.as_ref())
            .ok_or(CompositionViewRefusal::MissingOwner)?;
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
        })
    }
}

pub(crate) fn composition_view<'p>(
    package: &'p CheckedPackageV2,
    selected: &CheckedNodeId,
) -> Result<CompositionView<'p>, CompositionViewRefusal> {
    let (holder, node) = package
        .resolved_graph()
        .nodes
        .iter()
        .enumerate()
        .find(|(_, node)| node.node_id == *selected)
        .ok_or(CompositionViewRefusal::NotComposition)?;
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
    let name = package
        .claim_names()
        .unwrap_or_default()
        .iter()
        .find(|row| row.declaration == node.node_id && row.occurrence == occurrence)
        .ok_or(CompositionViewRefusal::MissingName)?;
    let source = occurrence_source::occurrence_source(package, &node.node_id, &occurrence, None)?;
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
