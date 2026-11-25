// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-277-AC-2: the public check operation accepts the exact S3 node bound
//! for a 100,000-term sum and refuses one below it on the default thread stack.

use filament_exact::Cancel;
use filament_semantic_value::checking::CheckingLimits;
use fsl_foundation::diagnostic::{LimitKind, StageFailure};
use fsl_foundation::Setting;
use fsl_replay::spine::{self, ParseRequest, SpineLimits};
use fsl_replay::{DependencyInput, SourceIdentity};
use ix_trace_rs::trace;

const TERMS: usize = 100_000;
const NODE_COUNT: u64 = 200_001;

#[trace("TC-758", "FR-277-AC-2")]
#[test]
fn a_100000_term_sum_checks_at_its_node_count_on_the_default_stack() {
    let sum = (1..=TERMS)
        .map(|term| term.to_string())
        .collect::<Vec<_>>()
        .join(" + ");
    let source = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.value.complete/v1\";\n\
         function large using v(): Boolean pure {{ {sum} < 5 }}\n"
    );

    std::thread::spawn(move || {
        let identity = SourceIdentity::new("agent-ix", "test:large-check-node-limit");
        let mut parse_limits = SpineLimits::default();
        parse_limits.source.source_bytes = 1 << 28;
        parse_limits.source.tokens = 1 << 28;
        parse_limits.source.nodes = 1 << 28;
        parse_limits.source.work_units = 1 << 40;
        let cancel = Cancel::new();
        let parsed = spine::parse(
            &ParseRequest {
                source: &identity,
                path: "large.native",
                bytes: source.as_bytes(),
            },
            parse_limits.source,
            &cancel,
        )
        .expect("the 100,000-term source parses")
        .into_value();
        let models = spine::select(&parsed, &Default::default(), parse_limits.model, &cancel)
            .expect("the empty model set selects")
            .into_value();

        let check_at = |s3_nodes| {
            let mut limits = SpineLimits::default();
            limits.checking = CheckingLimits::new(s3_nodes)
                .with_input_bytes(1 << 40)
                .with_work_budget(1 << 40);
            spine::check(
                &parsed,
                "fixture".to_owned(),
                &models,
                &DependencyInput::default(),
                limits,
                &Cancel::new(),
            )
        };
        assert!(
            check_at(NODE_COUNT).is_ok(),
            "the exact 200,001 expression nodes admit"
        );
        let refused = check_at(NODE_COUNT - 1).expect_err("one below the S3 node count refuses");
        let StageFailure::Limit(limit) = refused else {
            panic!("one below s3.nodes must be LimitExceeded, got {refused:?}");
        };
        assert_eq!(limit.setting(), Setting::S3Nodes);
        assert_eq!(limit.kind(), LimitKind::NodeCount);
        assert_eq!(limit.configured_bound(), NODE_COUNT - 1);
        assert_eq!(limit.actual(), u128::from(NODE_COUNT));
    })
    .join()
    .expect("the default stack handles the 100,000-term check");
}
