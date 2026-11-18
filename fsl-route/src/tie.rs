// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-962: the EN-1 exclusion of FR-489's production tie rule, applied
//! wherever an item's matching set is computed.
//!
//! EN-1 advertises `operation-contract` for one claim form only: an
//! `invariant` state clause in a request that names a model subject
//! (FR-489, FR-625). For every other unnamed `operation-contract` item it
//! is excluded from the matching set before any row of the rule applies,
//! so an operation's `pre` or `post` contract, a frame obligation and an
//! FR-146 per-step refinement record never match it by kind alone.

use fsl_foundation::BackendId;
use fsl_semantics::check::Capability;

use crate::CandidateOutcome;

/// EN-1's backend identity, the one provider the exclusion names.
pub const EN1: &str = "en-1";

/// `candidates` of the item of `kind`, with EN-1 removed when the item
/// names no backend, its kind is `operation-contract` and EN-1 may not
/// decide its claim (`en1_claim` false): the claim is not an `invariant`
/// state clause in a request naming a model subject. A requirement record
/// passes whether it is an invariant clause's, and the tie rule, which reads
/// the request's subject, applies the exclusion again with both facts.
/// Every other outcome is returned whole.
pub fn exclude_en1(
    candidates: CandidateOutcome,
    kind: Capability,
    named: Option<&BackendId>,
    en1_claim: bool,
) -> CandidateOutcome {
    if named.is_some() || kind != Capability::OperationContract || en1_claim {
        return candidates;
    }
    match candidates {
        CandidateOutcome::Candidates(set) => {
            CandidateOutcome::Candidates(set.without(&BackendId::new(EN1)))
        }
        other => other,
    }
}
