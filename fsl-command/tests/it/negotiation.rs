// SPDX-License-Identifier: AGPL-3.0-or-later
//! The FSpec FR-290 negotiation `fsl_command::command::run` takes (FR-961),
//! as a test double of the driver's adapter over CG's `negotiate_item`.

/// One candidate is `supported`, none `unsupported`, several
/// `invalid-request` `ambiguous-backend`, and an unregistered named backend
/// `invalid-request` `unknown-backend`.
pub(crate) struct Fr290;

impl fsl_analyze::analyze::document::Negotiate for Fr290 {
    fn negotiate(
        &self,
        item: &fsl_analyze::analyze::ClaimItem,
        candidates: &fsl_route::CandidateOutcome,
    ) -> (
        fsl_route::routing::Disposition,
        Option<fsl_analyze::analyze::Diagnostic>,
    ) {
        use fsl_analyze::analyze::{Diagnostic, Payload};
        use fsl_route::routing::Disposition;
        use fsl_route::NegotiationOutcome;
        match candidates {
            fsl_route::CandidateOutcome::UnknownBackend(backend) => (
                Disposition::InvalidRequest,
                Some(Diagnostic {
                    code: NegotiationOutcome::UnknownBackend.code(),
                    payload: Payload::Backend(backend.clone()),
                }),
            ),
            fsl_route::CandidateOutcome::Candidates(set) => match set.candidates() {
                [candidate] => (Disposition::Supported(candidate.clone()), None),
                [] => (
                    Disposition::Unsupported,
                    Some(Diagnostic {
                        code: NegotiationOutcome::UnsupportedRequestedCapability.code(),
                        payload: Payload::Kind(item.kind),
                    }),
                ),
                _ => (
                    Disposition::InvalidRequest,
                    Some(Diagnostic::new(NegotiationOutcome::AmbiguousBackend.code())),
                ),
            },
        }
    }
}
