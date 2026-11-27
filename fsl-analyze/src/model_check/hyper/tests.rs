// SPDX-License-Identifier: AGPL-3.0-or-later

use fsl_eval::hyper::HyperFinding;
use fsl_replay::UndecidedCause;
use ix_trace_rs::trace;

use super::{member_contract_candidates, Candidate, FirstHit};
use crate::model_check::ProductStateId;

#[test]
#[trace("TC-602", "FR-177-AC-7")]
fn qsl3561_refused_only_member_conjunction_is_undecided() {
    let findings = [HyperFinding::MemberContract, HyperFinding::MemberContract];
    let mut candidates = member_contract_candidates(ProductStateId(0), 3, &findings);

    assert_eq!(
        candidates.len(),
        1,
        "refused member findings aggregate once"
    );
    let (depth, rank, candidate) = candidates.pop().expect("one member candidate");
    assert_eq!(depth, 3);
    assert_eq!(rank, 1, "a refusal follows undefined candidates");
    let Candidate::Refused(cause) = candidate else {
        panic!("refused-only member conjunction has no undefined counterexample")
    };
    let hit = FirstHit::Undecided(cause);
    assert!(matches!(
        hit,
        FirstHit::Undecided(UndecidedCause::UndecidedSuccessor)
    ));
}
