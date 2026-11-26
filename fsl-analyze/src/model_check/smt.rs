// SPDX-License-Identifier: AGPL-3.0-or-later
//! Production settlement of an SMT temporal proof through FSL's FR-314 checker.

use filament_exact::Cancel;
use fsl_foundation::{BackendId, RequestIndex, source::provenance::OccurrenceKey};
use fsl_replay::{
    CertificateCheckLimits, CertificateRequest, ModelCheckItem, SmtProofOutcome,
    SmtProofSettlement, SubjectIdentity,
};

use crate::analyze::ClaimItem;
use crate::analyze::ItemForm;
use fsl_route::request::ItemKind;

/// The request context that binds CG's solver run to one negotiated analyze
/// item. Construct this before invoking the solver and pass the same value to
/// [`settle_temporal_smt_proof`].
pub struct BoundSmtProofRequest<'a> {
    request_index: RequestIndex,
    backend: BackendId,
    claim: OccurrenceKey,
    subject: SubjectIdentity,
    temporal_clock_selections: Vec<fsl_package::TemporalClockSelection>,
    smt_encoding: fsl_replay::SmtEncodingLimits,
    certificate_request: CertificateRequest<'a>,
}

impl<'a> BoundSmtProofRequest<'a> {
    /// Bind a certificate request to the exact temporal analyze item and
    /// backend that will own its result.
    ///
    /// Returns `None` when the item is not a temporal-satisfaction claim, its
    /// selected backend differs, or the request's checked clause is not the
    /// claim occurrence in the request subject's package.
    #[must_use]
    pub fn new(
        item: &ClaimItem,
        backend: BackendId,
        certificate_request: CertificateRequest<'a>,
    ) -> Option<Self> {
        if item.kind != fsl_semantics::check::Capability::TemporalSatisfaction
            || item.backend.as_ref() != Some(&backend)
        {
            return None;
        }
        let Some(ItemForm::Temporal(temporal)) = &item.form else {
            return None;
        };
        if temporal.item_kind != ItemKind::Formula {
            return None;
        }
        let ModelCheckItem::Clause { clause, .. } = certificate_request.item else {
            return None;
        };
        let package = certificate_request.subject.simulation.source_package()?;
        let claim_matches = package.graph().temporal_clauses().iter().any(|node| {
            node.identity() == *item.claim.node()
                && node.claim() == item.claim.origin()
                && node.clause() == clause
        });
        if !claim_matches {
            return None;
        }
        let request_index = item.request_index;
        let claim = item.claim.clone();
        let subject = SubjectIdentity::of(&certificate_request.subject);
        Some(Self {
            request_index,
            backend,
            claim,
            subject,
            temporal_clock_selections: certificate_request.temporal_clock_selections.to_vec(),
            smt_encoding: certificate_request.smt_encoding,
            certificate_request,
        })
    }

    /// The exact request CG must use for its temporal SMT run.
    #[must_use]
    pub fn certificate_request(&self) -> &CertificateRequest<'a> {
        &self.certificate_request
    }
}

/// FR-314 settlement and its opaque association with the request that was
/// checked. Fields cannot be reassigned independently of the settlement.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SmtProofEvidence {
    request_index: RequestIndex,
    backend: BackendId,
    claim: OccurrenceKey,
    subject: SubjectIdentity,
    item_kind: ItemKind,
    temporal_clock_selections: Vec<fsl_package::TemporalClockSelection>,
    smt_encoding: fsl_replay::SmtEncodingLimits,
    settlement: SmtProofSettlement,
}

impl SmtProofEvidence {
    pub(crate) fn request_index(&self) -> RequestIndex {
        self.request_index
    }

    pub(crate) fn backend(&self) -> &BackendId {
        &self.backend
    }

    pub(crate) fn claim(&self) -> &OccurrenceKey {
        &self.claim
    }

    pub(crate) fn subject(&self) -> &SubjectIdentity {
        &self.subject
    }

    pub(crate) fn item_kind(&self) -> ItemKind {
        self.item_kind
    }

    pub(crate) fn temporal_clock_selections(&self) -> &[fsl_package::TemporalClockSelection] {
        &self.temporal_clock_selections
    }

    pub(crate) fn smt_encoding(&self) -> &fsl_replay::SmtEncodingLimits {
        &self.smt_encoding
    }

    pub(crate) fn settlement(&self) -> &SmtProofSettlement {
        &self.settlement
    }
}

/// Settle CG's typed temporal SMT evidence and retain FR-621 checker evidence.
///
/// The returned value is ready for the analyze document projection. A checker
/// identity is present only when FSL verified the certificate.
#[must_use]
pub fn settle_temporal_smt_proof(
    request: &BoundSmtProofRequest<'_>,
    outcome: &SmtProofOutcome,
    limits: &CertificateCheckLimits,
    cancel: &Cancel,
) -> SmtProofEvidence {
    let settlement = fsl_replay::settle_smt_proof_with_evidence(
        &request.certificate_request,
        outcome,
        limits,
        cancel,
    );
    SmtProofEvidence {
        request_index: request.request_index,
        backend: request.backend.clone(),
        claim: request.claim.clone(),
        subject: request.subject.clone(),
        item_kind: ItemKind::Formula,
        temporal_clock_selections: request.temporal_clock_selections.clone(),
        smt_encoding: request.smt_encoding,
        settlement,
    }
}
