// SPDX-License-Identifier: AGPL-3.0-or-later
//! Probabilistic satisfaction dispatched over the package's own claim forms.

use fsl_foundation::source::provenance::OccurrenceKey;
use fsl_foundation::{BackendId, RequestIndex};
use fsl_package::CheckedNodeId;
use fsl_route::request::ExtentClassification;
use fsl_route::{EvidenceKind, ItemEvidence};
use fsl_semantics::check::Capability;

use super::super::{candidate_list, invalid_kind, unknown_option, ItemSelection, ResolvedClaim};
use super::{ClaimReads, PackageRunError, Resolution};

impl<'r, 'a: 'r> ClaimReads<'r, 'a> {
    pub(super) fn probability(
        &self,
        index: RequestIndex,
        name: &str,
        declaration: &CheckedNodeId,
        ordinal: u64,
        selection: &ItemSelection,
    ) -> Result<Resolution<()>, PackageRunError> {
        let fault = || PackageRunError::Document(super::AnalyzeDocumentError::Fault);
        let input = self.input.ok_or_else(fault)?;
        let node = input.package().node_key(declaration).ok_or_else(fault)?;
        let occurrence = OccurrenceKey::new(
            node,
            filament_exact::Origin::new(filament_exact::Role::new("claim"), ordinal),
        );
        let form = input
            .package()
            .probability_claim_form(&occurrence)
            .map_err(|failure| match failure {
                fsl_package::BareClauseRefusal::Pending { fact, node } => {
                    PackageRunError::Pending(super::PendingPackageFamily {
                        request_index: Some(index),
                        family: super::PendingFamily::ClauseFact(fact),
                        node: Some(node),
                        note: None,
                    })
                }
                fsl_package::BareClauseRefusal::Selection(_) => fault(),
            })?;
        let kind = match selection.kind.as_deref() {
            Some(label) => match Capability::from_wire(label) {
                Ok(kind) => kind,
                Err(_) => {
                    return Ok(Resolution::Settled(invalid_kind(
                        index,
                        Some(name),
                        "unknown-kind",
                    )))
                }
            },
            None => Capability::ProbabilisticSatisfaction,
        };
        if selection.probabilistic.is_some() && kind != Capability::ProbabilisticSatisfaction {
            return Ok(Resolution::Settled(super::super::invalid_value(
                index,
                name,
                "probabilistic",
            )));
        }
        let member = selection
            .probabilistic
            .clone()
            .unwrap_or_else(|| serde_json::json!({}));
        let probability =
            match super::super::probabilistic_item(&member, EvidenceKind::Exact, false) {
                Ok(settings) => settings,
                Err(()) => {
                    return Ok(Resolution::Settled(super::super::invalid_value(
                        index,
                        name,
                        "probabilistic",
                    )))
                }
            };
        let named = selection.backend.as_deref().map(BackendId::new);
        let backend_options = selection.options.clone().unwrap_or_default();
        if kind.applies_to_declaration(fsl_semantics::check::DeclarationFamily::Probabilistic) {
            if let Some(refused) = unknown_option(
                index,
                Some(name),
                self.registry,
                named.as_ref(),
                &backend_options,
            ) {
                return Ok(Resolution::Settled(refused));
            }
        }
        let candidates = self.registry.candidates_for_evidence(
            kind,
            &ItemEvidence::Kind(probability.evidence),
            named.as_ref(),
        );
        let own_subject = self.own_subject(index, selection)?;
        let item = super::ClaimItem {
            request_index: index,
            claim: form.occurrence,
            kind,
            extent: ExtentClassification::Unbounded {
                finite_bound_available: false,
            },
            backend: named,
            candidates: candidate_list(&candidates),
            options: super::options(selection, backend_options),
            form: (kind == Capability::ProbabilisticSatisfaction)
                .then_some(super::ItemForm::Probabilistic(probability)),
        };
        Ok(Resolution::Claim(
            Box::new(ResolvedClaim {
                item,
                claim_name: name.to_owned(),
                candidates,
                subject: own_subject.clone(),
                unrouted: selection.unrouted_beyond_sampling()
                    || !own_subject.same_model(self.subject),
            }),
            (),
        ))
    }
}

pub(super) fn item<'a>(
    input: &'a fsl_eval::simulation::model::PackageModelInput<'a>,
    binding: &super::SubjectBinding<'a>,
    limits: &super::AnalyzeLimits,
    item: &super::ClaimItem,
    form: &super::super::super::ProbabilisticItem,
) -> super::AnalyzeItem<'a> {
    let (EvidenceKind::Exact, super::super::super::ProbabilisticLimits::Exact(exact)) =
        (form.evidence, form.limits)
    else {
        return super::AnalyzeItem::unadapted(
            item.request_index,
            item.kind,
            Some(fsl_semantics::check::DeclarationFamily::Probabilistic),
        );
    };
    let binding = binding.clone();
    let limits = *limits;
    let occurrence = item.claim.clone();
    super::AnalyzeItem::new(
        item.request_index,
        Capability::ProbabilisticSatisfaction,
        Some(fsl_semantics::check::DeclarationFamily::Probabilistic),
        fsl_route::Mode::Unbounded,
        crate::exact_probabilistic::manifest::BACKEND_ID,
        move |cancel| {
            let model = input.input();
            let Some(claim) = model
                .probabilistic
                .claims()
                .iter()
                .find(|claim| claim.occurrence() == Some(&occurrence))
            else {
                return super::super::super::EngineReport::Refused(
                    super::super::super::Refusal::Failed(super::super::super::items::BROKEN),
                );
            };
            let mut simulation =
                super::super::super::request::simulation(&model, &binding, &limits);
            simulation.expansion.max_candidates = 1_000_000;
            super::super::super::exact::run_package_exact(
                crate::exact_probabilistic::ProductRequest {
                    subject: fsl_eval::model_subject::ModelSubject {
                        simulation,
                        proof_bounds: Vec::new(),
                    },
                    claim,
                    limits: exact,
                },
                cancel,
            )
        },
    )
}
