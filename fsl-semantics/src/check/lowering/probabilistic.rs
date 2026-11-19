// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-498: the `claim`/`probabilistic_claim` node each checked probabilistic
//! claim lowers to, with its one declared `claim` occurrence.
//!
//! The producer keys the node structurally under FSpec FR-322's
//! `{package, path, ordinal}` rule: path `["claim", "probabilistic_claim",
//! name]`, and an ordinal that ranks the claims sharing that name by their
//! written content ([`fsl_forms::SignificantContent`]). No span, source position
//! or declaration order enters the key, so permuting the declarations keys
//! every claim as before; claims written identically are indistinguishable,
//! so which of them takes which rank is unobservable. The node's body is an
//! empty aggregate: no FSpec body is defined for the form yet, and none is
//! fabricated.

use std::collections::hash_map;

use filament_exact::{NodeKey, PathSegment};

use super::temporal::stage_fault;
use super::{Lowering, NodeContent, NodeRef, SemanticNode};
use crate::check::family::OccurrenceRole;
use crate::check::node_key::{BodyTerm, NodeTag};
use crate::check::probabilistic::{CheckedProbabilisticClaim, CheckedProbabilisticModel};
use crate::check::refusal::CheckRefusal;

/// The semantic form of a probabilistic claim's node.
const FORM: &str = "probabilistic_claim";

impl Lowering<'_> {
    /// FSpec FR-640 "Emission": root the nodes a checked random parameter or
    /// workload refers to, so a package that emits it holds them: the model
    /// node of each declaring type and of each workload's model type, and the
    /// type node of each random parameter, which brings the unit of a
    /// quantity and the variant type, with its case nodes, of an enum.
    pub(crate) fn probabilistic_declarations(
        &mut self,
        model: &CheckedProbabilisticModel,
    ) -> Result<(), CheckRefusal> {
        for random in model.random_parameters() {
            let at = random.declaration_location();
            self.charge(1, &at)?;
            if let Some(declaring) = random.operation().declaring_key() {
                self.rooted_model_node(declaring, &at)?;
            }
            if let NodeRef::Admitted(key) = self.type_ref(random.value_type(), &at)? {
                self.record_generated(key, &at);
            }
        }
        for workload in model.workloads() {
            let at = workload.declaration_location();
            self.charge(1, &at)?;
            for (operation, _) in workload.weights() {
                if let Some(declaring) = operation.declaring_key() {
                    self.rooted_model_node(declaring, &at)?;
                }
            }
            if let Some(model) = workload.model_key() {
                self.rooted_model_node(model, &at)?;
            }
        }
        Ok(())
    }

    /// The model node of `declaration`, given a `generated` occurrence at
    /// `at` when nothing else recorded one.
    fn rooted_model_node(
        &mut self,
        declaration: &filament_checked::key::DeclarationKey,
        at: &crate::check::Location,
    ) -> Result<(), CheckRefusal> {
        let key = self.model_node(declaration, at)?;
        self.record_generated(key, at);
        Ok(())
    }

    /// Give `key` a `generated` occurrence at `at` when nothing else recorded
    /// one, which places the nodes it brings with it at the declaration.
    fn record_generated(&mut self, key: NodeKey, at: &crate::check::Location) {
        if !self.occurrences.has(key.clone()) {
            self.record(key, OccurrenceRole::Generated, at.clone());
        }
    }

    /// Key and add the node of each of `claims`, recording its one `claim`
    /// occurrence at the claim's declaration. The keys are in `claims`'
    /// order.
    pub(crate) fn probabilistic_claims(
        &mut self,
        claims: &[CheckedProbabilisticClaim],
    ) -> Result<Vec<(NodeKey, Option<NodeKey>)>, CheckRefusal> {
        let mut ranked: Vec<(usize, &CheckedProbabilisticClaim)> =
            claims.iter().enumerate().collect();
        // Within one name, by content; equal content is equal claims, whose
        // tie order is unobservable.
        ranked.sort_by(|(_, left), (_, right)| {
            left.name
                .cmp(&right.name)
                .then_with(|| left.content.cmp(&right.content))
        });
        let mut keys = Vec::with_capacity(claims.len());
        let mut ordinal = 0_u64;
        let mut previous: Option<&str> = None;
        for (index, claim) in ranked {
            if previous == Some(claim.name.as_str()) {
                ordinal += 1;
            } else {
                ordinal = 0;
            }
            previous = Some(&claim.name);
            keys.push((index, self.probabilistic_claim(claim, ordinal)?));
        }
        keys.sort_by_key(|(index, _)| *index);
        Ok(keys.into_iter().map(|(_, key)| key).collect())
    }

    /// Add `claim`'s node at `ordinal` of its name's path.
    fn probabilistic_claim(
        &mut self,
        claim: &CheckedProbabilisticClaim,
        ordinal: u64,
    ) -> Result<(NodeKey, Option<NodeKey>), CheckRefusal> {
        let at = claim.declaration_location();
        self.charge(1, &at)?;
        let key = super::super::derived_node_key(
            &self.package_name,
            NodeTag::Claim,
            FORM,
            vec![PathSegment::Text(claim.name.clone())],
            ordinal,
        )
        .map_err(|_| stage_fault(&at, "probabilistic-claim-key-is-well-formed"))?;
        match self.graph.nodes.entry(key.clone()) {
            hash_map::Entry::Vacant(slot) => {
                slot.insert(SemanticNode {
                    key: key.clone(),
                    recursion: None,
                    nominal: None,
                    content: NodeContent {
                        node_tag: NodeTag::Claim,
                        semantic_form: FORM,
                        semantic_type: None,
                        declaration: None,
                        owner: None,
                        owner_model: None,
                        body: BodyTerm::aggregate(Vec::new()),
                    },
                });
            }
            hash_map::Entry::Occupied(_) => {
                return Err(stage_fault(&at, "probabilistic-claim-key-is-unheld"));
            }
        }
        self.record(key.clone(), OccurrenceRole::Claim, at);
        let over = claim
            .over
            .as_ref()
            .map(|over| {
                let at = claim.over_location();
                let semantic_type = self.binder_type(&over.value_type, None, &at)?;
                let parameter = self.parameter(&over.name, 0, semantic_type.clone(), &at)?;
                self.record(semantic_type, OccurrenceRole::Type, at);
                Ok::<_, CheckRefusal>(parameter)
            })
            .transpose()?;
        Ok((key, over))
    }
}
