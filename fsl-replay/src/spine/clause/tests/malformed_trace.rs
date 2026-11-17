// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL-2201: malformed traces carry useful item messages, while nonempty
//! lassos remain admitted.

use super::temporal::{Unit, finite, item_json, lasso, report};
use super::*;
use crate::proof_result::SettlementBasis;
use crate::spine::ClauseRunReport;

#[trace("FR-286-AC-9")]
#[test]
fn timed_clause_over_snapshots_has_a_specific_item_message() {
    use filament_semantic_value::location::{ExpressionLocation, ExpressionOwner};
    use fsl_semantics::check::{CheckCause, CheckRefusal, TemporalCause};

    let refusal = CompileRefusal::Check {
        refusals: vec![CheckRefusal {
            location: ExpressionLocation::root(ExpressionOwner::Expression),
            cause: CheckCause::Temporal(Box::new(TemporalCause::TimedTraceSubject {
                span: fsl_foundation::Span { start: 0, end: 1 },
            })),
        }],
        region: None,
    };
    let report = ClauseRunReport {
        basis: SettlementBasis::Unavailable,
        witness: None,
        state_clause_counterexample: None,
        frame_field_values: None,
        disposition: ClauseDisposition::Compile(Box::new(refusal)),
        usage: super::super::ClauseRunUsage::default(),
        consequent_source_events: crate::spine::ConsequentSourceEvents::Unavailable,
        finite_window: None,
        counterexample: None,
        decisive: None,
        authenticated_temporal_selection: None,
        refused_at: None,
        ledger: None,
    };
    let item = item_json("Timed", &report);

    assert!(
        item["cause"]["message"].as_str().is_some_and(
            |message| message.contains("timed clause over snapshots is not checked yet")
        )
    );
}

#[trace("FR-286-AC-9")]
#[test]
fn empty_loop_and_finite_trace_have_explanatory_messages_and_nonempty_lasso_is_admitted() {
    let empty_loop = item_json(
        "Reaches",
        &report(lasso(Unit::Infinite, "Reaches", 3, &[0, 1, 2])),
    );
    assert!(
        empty_loop["cause"]["message"]
            .as_str()
            .is_some_and(|message| message.contains("loop"))
    );

    let empty_finite = item_json("Reaches", &report(finite(Unit::Infinite, "Reaches", &[])));
    assert!(
        empty_finite["cause"]["message"]
            .as_str()
            .is_some_and(|message| message.contains("trace"))
    );

    let nonempty_lasso = item_json(
        "Reaches",
        &report(lasso(Unit::Infinite, "Reaches", 0, &[0, 1, 2])),
    );
    assert_ne!(nonempty_lasso["result"], "declined");
}
