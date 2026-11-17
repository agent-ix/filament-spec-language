// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL-2201: item causes share the `kind` and `code` wire members.

use super::temporal::{finite, item_json, lasso, report, Unit};
use super::*;
use crate::{OutcomeItem, RequestIndex};

#[trace("FR-286-AC-4")]
#[test]
fn undefined_evaluation_uses_the_shared_item_cause_members() {
    let item =
        OutcomeItem::undefined_evaluation(RequestIndex::new(0), "position 1", "division-by-zero");
    let value = serde_json::to_value(item).expect("serializes");

    assert_eq!(value["cause"]["kind"], "undefined-evaluation");
    assert_eq!(value["cause"]["code"], "division-by-zero");
    assert!(value["cause"].get("cause").is_none());
}

#[trace("FR-286-AC-4")]
#[test]
fn item_cause_corpus_uses_kind_and_code_without_the_old_cause_member() {
    let undefined =
        OutcomeItem::undefined_evaluation(RequestIndex::new(0), "position 1", "division-by-zero");
    let undefined = serde_json::to_value(undefined).expect("serializes");
    let empty_loop = item_json(
        "Reaches",
        &report(lasso(Unit::Infinite, "Reaches", 3, &[0, 1, 2])),
    );
    let empty_finite = item_json("Reaches", &report(finite(Unit::Infinite, "Reaches", &[])));

    for cause in [
        &undefined["cause"],
        &empty_loop["cause"],
        &empty_finite["cause"],
    ] {
        assert!(cause.get("kind").is_some(), "{cause}");
        assert!(cause.get("code").is_some(), "{cause}");
        assert!(cause.get("cause").is_none(), "{cause}");
    }
}
