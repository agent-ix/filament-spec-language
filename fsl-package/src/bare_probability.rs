// SPDX-License-Identifier: AGPL-3.0-or-later
//! Exact eventuality claims and predicates read from admitted package nodes.

use filament_contract_model::{
    BoundDirection, CheckedOccurrenceRole, ProbabilisticActivation, ProbabilisticClaimForm,
    ProbabilisticExact, ProbabilisticReachTarget, ProbabilisticScheduler,
};
use filament_exact::{NodeKey, Rational, ValueType};
use fsl_foundation::selection::DefinitionRef;
use fsl_foundation::source::provenance::OccurrenceKey;
use fsl_semantics::check::{Bound, PackageProbabilityClaimForm, TemporalProfile};

use crate::bare_slot::BareSelectionFailure;
use crate::bare_term::{fault, walk_failure, BareClauseRefusal};
use crate::protocol_selection::SlotSelectionFailure;
use crate::protocol_view::{group_member, text};
use crate::reader_protocol_projection::{BareWalker, UnsupportedView};
use crate::{AnalysisView, BarePackage, BareStatePredicate};

impl BarePackage<'_> {
    fn probability_claim_read(
        &self,
        claim: &OccurrenceKey,
    ) -> Result<
        (
            &filament_contract_model::CheckedSemanticNodeV2,
            &filament_contract_model::CheckedProbabilisticClaimNode,
            &filament_contract_model::ClaimNameRow,
        ),
        BareSelectionFailure,
    > {
        let malformed = || {
            BareSelectionFailure::from(SlotSelectionFailure::MalformedNode(claim.node().clone()))
        };
        let node = self.node_by_key(claim.node())?.ok_or_else(malformed)?;
        let row = self
            .package()
            .claim_names()
            .unwrap_or_default()
            .iter()
            .find(|row| {
                row.declaration == node.node_id
                    && row.occurrence.role == CheckedOccurrenceRole::Claim
                    && row.occurrence.ordinal == claim.origin().ordinal()
            })
            .ok_or_else(malformed)?;
        let read = self
            .package()
            .probabilistic_claims()
            .iter()
            .find(|read| self.node_key(&read.node).as_ref() == Some(claim.node()))
            .ok_or_else(malformed)?;
        Ok((node, read, row))
    }

    /// Read one exact, untimed `eventually` claim with an origin activation
    /// and a declared workload. Unsupported probability forms remain typed
    /// pending selections; this never reconstructs a source checked body.
    ///
    /// # Errors
    ///
    /// A selection fault for inconsistent admitted facts, or a pending fact
    /// for a probability form outside this package projection.
    pub fn probability_claim_form(
        &self,
        claim: &OccurrenceKey,
    ) -> Result<PackageProbabilityClaimForm, BareClauseRefusal> {
        let malformed = || {
            BareSelectionFailure::from(SlotSelectionFailure::MalformedNode(claim.node().clone()))
        };
        let (node, read, row) = self.probability_claim_read(claim)?;
        let pending = || BareClauseRefusal::Pending {
            fact: "probabilistic-exact-eventual-form",
            node: node.node_id.clone(),
        };
        let ProbabilisticClaimForm::Reach {
            target: ProbabilisticReachTarget::Eventually(predicate),
            bound,
        } = &read.form
        else {
            return Err(pending());
        };
        let ProbabilisticScheduler::Workload(workload) = &read.scheduler else {
            return Err(pending());
        };
        if !matches!(read.activation, ProbabilisticActivation::Origin)
            || !read.rewards.is_empty()
            || !read.delays.is_empty()
            || read.confidence.is_some()
        {
            return Err(pending());
        }
        let profile = self
            .package()
            .lock()
            .profile_selections
            .iter()
            .find(|selection| {
                selection.definition.identity.as_ref() == TemporalProfile::InfiniteTrace.identity()
            })
            .ok_or_else(pending)?;
        let profile = DefinitionRef::new(
            profile.definition.authority.to_string(),
            profile.definition.identity.to_string(),
        )
        .map_err(|_| malformed())?;
        let workload = self.node_key(workload).ok_or_else(malformed)?;
        let workload = self
            .node_by_key(&workload)?
            .and_then(|node| group_member(&node.body, "name"))
            .and_then(text)
            .ok_or_else(malformed)?
            .to_owned();
        let subject = self.node_key(&read.subject).ok_or_else(malformed)?;
        let subject = self
            .model_selections()
            .into_iter()
            .find(|model| model.identity == subject.package())
            .ok_or_else(malformed)?;
        let over = read
            .over
            .as_ref()
            .map(|over| {
                let key = self.node_key(over).ok_or_else(malformed)?;
                let parameter = self.node_by_key(&key)?.ok_or_else(malformed)?;
                let name = group_member(&parameter.body, "name")
                    .and_then(text)
                    .ok_or_else(malformed)?
                    .to_owned();
                let object = self
                    .exact_type_projections()
                    .get(&parameter.node_id)
                    .and_then(|projection| projection.as_ref().ok())
                    .and_then(|projection| match projection.value_type() {
                        ValueType::Reference(object) => Some(object.clone()),
                        _ => None,
                    })
                    .ok_or_else(malformed)?;
                Ok::<_, BareSelectionFailure>((name, object))
            })
            .transpose()?;
        let rational = match &bound.value {
            ProbabilisticExact::Integer(value) => {
                Rational::from_integer(value.to_string().parse().map_err(|_| malformed())?)
            }
            ProbabilisticExact::Rational(value) => Rational::new(
                value.numerator.parse().map_err(|_| malformed())?,
                value.denominator.parse().map_err(|_| malformed())?,
            )
            .map_err(|_| malformed())?,
            ProbabilisticExact::Quantity { .. } => return Err(pending()),
        };
        let bound = match bound.direction {
            BoundDirection::AtLeast => Bound::AtLeast(rational),
            BoundDirection::AtMost => Bound::AtMost(rational),
        };
        let predicate = self.node_key(predicate).ok_or_else(malformed)?;
        let declaration_index = self.probability_declaration_index(claim)?;
        let projected =
            self.project_probability_predicate_at(claim, &predicate, declaration_index)?;
        Ok(PackageProbabilityClaimForm {
            occurrence: claim.clone(),
            name: row.name.to_string(),
            span: fsl_foundation::Span {
                start: usize::try_from(row.declaration_region.start).map_err(|_| malformed())?,
                end: usize::try_from(row.declaration_region.end).map_err(|_| malformed())?,
            },
            subject: fsl_semantics::check::ModelSubjectRef {
                alias: subject.identity.clone(),
                package: subject,
            },
            profile,
            workload,
            over,
            over_node: read
                .over
                .as_ref()
                .map(|over| self.node_key(over).ok_or_else(malformed))
                .transpose()?,
            predicate,
            slots: projected.projection().frame_slots(),
            bound,
            declaration_index,
        })
    }

    fn probability_declaration_index(
        &self,
        claim: &OccurrenceKey,
    ) -> Result<usize, BareSelectionFailure> {
        let malformed = || {
            BareSelectionFailure::from(SlotSelectionFailure::MalformedNode(claim.node().clone()))
        };
        let node = self.node_by_key(claim.node())?.ok_or_else(malformed)?;
        let own = self
            .package()
            .source_map()
            .iter()
            .find(|entry| {
                entry.node_id == node.node_id
                    && entry.role == CheckedOccurrenceRole::Claim
                    && entry.ordinal == claim.origin().ordinal()
            })
            .and_then(|entry| entry.regions.first())
            .ok_or_else(malformed)?;
        Ok(self
            .package()
            .source_map()
            .iter()
            .filter(|entry| {
                matches!(
                    entry.role,
                    CheckedOccurrenceRole::Claim | CheckedOccurrenceRole::Declaration
                ) && self.package().resolved_graph().nodes.iter().any(|node| {
                    node.node_id == entry.node_id
                        && (node.node_tag.as_ref() == "probabilistic"
                            || node.semantic_form.as_ref() == "probabilistic_claim")
                }) && entry
                    .regions
                    .first()
                    .is_some_and(|region| region.source == own.source && region.start < own.start)
            })
            .count())
    }

    /// Project the target of a supported exact eventuality from the
    /// authenticated claim node, preserving its original predicate site.
    ///
    /// # Errors
    ///
    /// A missing or unsupported claim, or a refused expression projection.
    pub fn project_probability_predicate(
        &self,
        claim: &OccurrenceKey,
    ) -> Result<BareStatePredicate, BareSelectionFailure> {
        let malformed = || {
            BareSelectionFailure::from(SlotSelectionFailure::MalformedNode(claim.node().clone()))
        };
        let (_, read, _) = self.probability_claim_read(claim)?;
        let ProbabilisticClaimForm::Reach {
            target: ProbabilisticReachTarget::Eventually(predicate),
            ..
        } = &read.form
        else {
            return Err(malformed());
        };
        let predicate = self.node_key(predicate).ok_or_else(malformed)?;
        self.project_probability_predicate_at(
            claim,
            &predicate,
            self.probability_declaration_index(claim)?,
        )
    }

    fn project_probability_predicate_at(
        &self,
        claim: &OccurrenceKey,
        predicate: &NodeKey,
        declaration_index: usize,
    ) -> Result<BareStatePredicate, BareSelectionFailure> {
        let malformed = || {
            BareSelectionFailure::from(SlotSelectionFailure::MalformedNode(claim.node().clone()))
        };
        let (_, read, _) = self.probability_claim_read(claim)?;
        let mut keys = Vec::new();
        let mut names = Vec::new();
        if let Some(over) = read.over.as_ref() {
            let key = self.node_key(over).ok_or_else(malformed)?;
            let parameter = self.node_by_key(&key)?.ok_or_else(malformed)?;
            names.push(
                group_member(&parameter.body, "name")
                    .and_then(text)
                    .ok_or_else(malformed)?
                    .to_owned(),
            );
            keys.push(key);
        }
        let (_, identities) = self.witness()?;
        let walker = BareWalker::new(
            self.package(),
            identities,
            self.equality_decisions(),
            self.exact_type_projections(),
            self.dependency_views(),
        )
        .map_err(SlotSelectionFailure::Fault)?;
        let projection = walker
            .project_term(
                predicate,
                &keys,
                filament_semantic_value::location::ExpressionOwner::TypeDeclaration {
                    name: format!("probabilistic {declaration_index} expression 0"),
                },
            )
            .map_err(walk_failure)?;
        let world = self.world()?;
        if projection.has_unrepresented_types(world.types()) {
            return Err(BareSelectionFailure::Unsupported(Box::new(
                UnsupportedView::missing_type(self.package().name(), predicate.clone()),
            )));
        }
        let parameters = names
            .into_iter()
            .zip(&keys)
            .map(|(name, key)| {
                projection
                    .local_slot(key)
                    .map(|slot| (name, slot))
                    .ok_or_else(|| fault("bare-probability-binder-has-a-local-slot"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(BareStatePredicate::new(parameters, projection))
    }
}
