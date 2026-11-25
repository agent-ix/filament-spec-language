// SPDX-License-Identifier: AGPL-3.0-or-later
//! Public settings-operation clauses of TC-1250.

use fsl_foundation::UsageCause;
use fsl_replay::{CallerLimits, SmtEncodingLimits, SmtRunLimits};
use ix_trace_rs::trace;

/// TC-1250 partial: the public settings operation returns literal SMT values,
/// defaults omitted values, and names rejected zero bounds.
#[trace("TC-1250")]
#[test]
fn smt_settings_keep_their_typed_values_defaults_and_usage_refusals() {
    let encoding = CallerLimits::from_operands(["smt_encoding.max_constants=8"])
        .expect("max constants is a supported setting");
    assert_eq!(
        encoding.engines.smt_encoding,
        SmtEncodingLimits {
            max_constants: 8,
            ..SmtEncodingLimits::default()
        }
    );

    let run = CallerLimits::from_operands(["smt.max_solver_steps=1000", "smt.max_solver_time=5"])
        .expect("solver settings are supported");
    assert_eq!(
        run.engines.smt,
        SmtRunLimits {
            max_solver_steps: 1000,
            max_solver_time: 5,
        }
    );

    let defaults = CallerLimits::from_operands([]).expect("defaults are available");
    assert_eq!(defaults.engines.smt, SmtRunLimits::default());
    assert_eq!(defaults.engines.smt_encoding, SmtEncodingLimits::default());

    for operand in ["smt.max_solver_time=0", "smt_encoding.max_monitor_states=0"] {
        let refusal = CallerLimits::from_operands([operand]).expect_err("zero is refused");
        assert_eq!(refusal.operand, operand);
        assert_eq!(refusal.cause, UsageCause::NotAnInteger);
        assert_eq!(
            refusal.to_string(),
            format!("usage: NotAnInteger in `--limit {operand}`")
        );
    }
}
