// SPDX-License-Identifier: AGPL-3.0-or-later
//! External consumer compile surface for the FR-127 SMT proof handoff.

use fsl_replay::{
    settle_smt_proof, Cancel, CertificateCheckLimits, CertificateRequest, SmtProofBasis,
    SmtProofCertificate, SmtProofOutcome, TerminalValue,
};

/// Compile the names and call used by a downstream SMT backend adapter.
pub fn settle(
    request: &CertificateRequest<'_>,
    outcome: &SmtProofOutcome,
    limits: &CertificateCheckLimits,
    cancel: &Cancel,
) -> TerminalValue {
    let _: SmtProofBasis = outcome.basis;
    let _: Option<&SmtProofCertificate> = outcome.certificate.as_ref();
    settle_smt_proof(request, outcome, limits, cancel)
}
