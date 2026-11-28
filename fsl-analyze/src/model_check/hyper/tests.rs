// SPDX-License-Identifier: AGPL-3.0-or-later

use fsl_eval::hyper::HyperFinding;
use fsl_replay::UndecidedCause;
use ix_trace_rs::trace;

use super::{finalize_refused_candidate, member_contract_candidates, Candidate, FirstHit};
use crate::model_check::ProductStateId;

#[test]
#[trace("TC-602", "FR-177-AC-7")]
fn qsl3561_refused_only_member_conjunction_is_undecided() {
    let findings = [HyperFinding::MemberContract];
    let mut candidates = member_contract_candidates(ProductStateId(0), 3, &findings);

    assert_eq!(
        candidates.len(),
        1,
        "one refused member finding is settled once"
    );
    let (depth, rank, candidate) = candidates.pop().expect("one member candidate");
    assert_eq!(depth, 3);
    assert_eq!(rank, 1, "a refusal follows undefined candidates");
    assert!(matches!(candidate, Candidate::Refused(_)));
    let hit = finalize_refused_candidate(depth, candidate).expect("refused result is undecided");
    assert!(matches!(
        &hit.1,
        FirstHit::Undecided(UndecidedCause::UndecidedSuccessor)
    ));
    assert!(!matches!(
        &hit.1,
        FirstHit::Violated(_) | FirstHit::Unstated
    ));
}
