// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-277-AC-2: the public check operation accepts the exact S3 node bound
//! for a 100,000-term sum and refuses one below it on the default thread stack.

use filament_exact::Cancel;
use filament_exact::NodeKey;
use filament_semantic_value::checking::CheckingLimits;
use fsl_foundation::diagnostic::{LimitKind, StageFailure};
use fsl_foundation::Setting;
use fsl_replay::spine::{self, PackageLimits, ParseRequest, SpineLimits};
use fsl_replay::{DependencyInput, SourceIdentity};
use ix_trace_rs::trace;

const TERMS: usize = 100_000;
const NODE_COUNT: u64 = 200_001;
/// The platform's default stack: the 8 MiB Linux and macOS give a process's
/// main thread.
const DEFAULT_STACK: usize = 8 * 1024 * 1024;

/// Nesting depth is not a limit: this rendering of part of an outcome names
/// none. Pass only values whose size does not grow with the source: a node
/// key renders its whole nested structure, so a checked unit's types render
/// quadratically in the term count.
fn assert_names_no_depth(outcome: &str) {
    assert!(
        !outcome.to_lowercase().contains("depth"),
        "an outcome names a depth"
    );
}

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

    let check = move || {
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
        {
            let Ok(admitted) = check_at(NODE_COUNT) else {
                panic!("the exact 200,001 expression nodes admit");
            };
            assert_names_no_depth(&format!(
                "{:?} {:?}",
                admitted.work(),
                admitted.value().build_provenance(),
            ));
        }
        let Err(StageFailure::Limit(limit)) = check_at(NODE_COUNT - 1) else {
            panic!("one below s3.nodes must be LimitExceeded");
        };
        assert_names_no_depth(&format!("{limit:?}"));
        assert_eq!(limit.setting(), Setting::S3Nodes);
        assert_eq!(limit.kind(), LimitKind::NodeCount);
        assert_eq!(limit.configured_bound(), NODE_COUNT - 1);
        assert_eq!(limit.actual(), u128::from(NODE_COUNT));
    };
    std::thread::Builder::new()
        .stack_size(DEFAULT_STACK)
        .spawn(check)
        .expect("the check thread starts")
        .join()
        .expect("the default stack handles the 100,000-term check");
}

#[test]
#[ignore = "QSL-3256: the 100,000-term package and node-key check does not complete within 20 minutes"]
fn a_100000_term_package_and_node_keys_match_on_512kib_and_8mib_stacks() {
    let sum = (1..=TERMS)
        .map(|term| term.to_string())
        .collect::<Vec<_>>()
        .join(" + ");
    let source = std::sync::Arc::new(format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.value.complete/v1\";\n\
         function large using v(): Boolean pure {{ {sum} < 5 }}\n"
    ));

    let check_on_stack = |stack_size| {
        let source = std::sync::Arc::clone(&source);
        std::thread::Builder::new()
            .stack_size(stack_size)
            .spawn(move || {
                let identity = SourceIdentity::new("agent-ix", "test:large-check-node-limit");
                let mut limits = SpineLimits::default();
                limits.source.source_bytes = 1 << 28;
                limits.source.tokens = 1 << 28;
                limits.source.nodes = 1 << 28;
                limits.source.work_units = 1 << 40;
                limits.checking = CheckingLimits::new(NODE_COUNT)
                    .with_input_bytes(1 << 40)
                    .with_work_budget(1 << 42);
                limits.package = PackageLimits::default().with_output_bytes(1 << 30);

                let parsed = spine::parse(
                    &ParseRequest {
                        source: &identity,
                        path: "large.native",
                        bytes: source.as_bytes(),
                    },
                    limits.source,
                    &Cancel::new(),
                )
                .expect("the 100,000-term source parses")
                .into_value();
                let models =
                    spine::select(&parsed, &Default::default(), limits.model, &Cancel::new())
                        .expect("the empty model set selects")
                        .into_value();
                let checked = spine::check(
                    &parsed,
                    "fixture".to_owned(),
                    &models,
                    &DependencyInput::default(),
                    limits,
                    &Cancel::new(),
                )
                .expect("the 100,000-term source checks")
                .into_value();
                let node_keys = checked
                    .package()
                    .graph()
                    .semantic_graph()
                    .nodes()
                    .map(|node| node.key())
                    .collect::<Vec<NodeKey>>();
                let emitted = spine::package(&checked, limits.package, &Cancel::new())
                    .expect("the checked package emits under the raised output bound")
                    .into_value();
                (emitted.package().bytes().to_vec(), node_keys)
            })
            .expect("the comparison worker starts")
            .join()
            .expect("the source checks on the requested stack size")
    };

    let small_stack = check_on_stack(512 * 1024);
    let large_stack = check_on_stack(8 * 1024 * 1024);
    assert_eq!(small_stack.0, large_stack.0);
    assert_eq!(small_stack.1, large_stack.1);
}
