// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-185-AC-13 (TC-3027): S3 refuses a reward that can be negative by
//! evaluating it through the layer-3 `RewardEvaluator` port, which
//! `fsl-eval`'s S6a evaluator implements, so the sign is computed by the one
//! interpreter that model admission and the engines also use.

use crate::prob_world::{self, World};
use fsl_foundation::diagnostic::Code;
use ix_trace_rs::trace;

/// `Service`'s `d` ranges over its support `{1, 3, 12}`, so `d - 3` is
/// negative at `d = 1` and S3 refuses it; `d - 1` is never negative.
#[test]
#[trace("TC-3027", "FR-185-AC-13")]
#[trace("TC-2061", "FR-185-AC-11")]
#[ignore = "QSL-2252: the S3 negative-reward check through the RewardEvaluator port is not implemented yet"]
fn s3_refuses_a_negative_reward_through_the_reference_evaluator() {
    let Err(refusals) = World::try_new(&prob_world::service(
        "reward refund on Server::attempt = d - 3;",
    )) else {
        panic!("a reward that can be negative must refuse at S3");
    };
    let [refusal] = refusals.as_slice() else {
        panic!("one refusal: {refusals:?}");
    };
    assert_eq!(refusal.cause.code(), Code::InvalidProbabilisticModel);
    assert_eq!(refusal.cause.cause(), Some("negative-reward"));
    let text = format!("{refusal:?}");
    for name in ["attempt", "refund", "d"] {
        assert!(text.contains(name), "names {name}: {text}");
    }
    assert!(
        World::try_new(&prob_world::service(
            "reward refund on Server::attempt = d - 1;"
        ))
        .is_ok(),
        "a reward that is never negative checks"
    );
}
