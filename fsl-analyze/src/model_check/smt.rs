// SPDX-License-Identifier: AGPL-3.0-or-later
//! Production settlement of an SMT temporal proof through FSL's FR-314 checker.

use filament_exact::Cancel;
use fsl_replay::{CertificateCheckLimits, CertificateRequest, SmtProofOutcome, SmtProofSettlement};

/// Settle CG's typed temporal SMT evidence and retain FR-621 checker evidence.
///
/// The returned value is ready for the analyze document projection. A checker
/// identity is present only when FSL verified the certificate.
#[must_use]
pub fn settle_temporal_smt_proof(
    request: &CertificateRequest<'_>,
    outcome: &SmtProofOutcome,
    limits: &CertificateCheckLimits,
    cancel: &Cancel,
) -> SmtProofSettlement {
    fsl_replay::settle_smt_proof_with_evidence(request, outcome, limits, cancel)
}
