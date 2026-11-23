// SPDX-License-Identifier: AGPL-3.0-or-later
//! Path-based, name-filtered merge-gate recipes for FSL changes.

use std::collections::BTreeSet;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::path::Path;
use std::process::Command;

use crate::error::{Error, Result};

const LARGE_EMIT_FILTER: &str =
    "emit::tests::protocol_locations::a_function_body_100000_deep_projects_on_a_small_stack";
const EMIT_SKIP: [&str; 2] = [
    "100000",
    "every_nested_form_emits_in_the_stratified_grammar_at_2_and_1000_levels",
];
const USAGE: &str = "usage: cargo xtask merge-gate-recipe <changed-path>...";

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum TestTarget {
    Library,
    Integration(String),
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct TestSelector {
    package: String,
    target: TestTarget,
    filter: Option<String>,
    exact: bool,
    skip: Vec<String>,
    timed: bool,
    threads: Option<usize>,
    no_match_is_note: bool,
}

#[derive(Default)]
struct Recipe {
    selectors: BTreeSet<TestSelector>,
    notes: BTreeSet<String>,
}

#[derive(Clone, Copy)]
enum LibRule {
    SourceModule,
    Fixed(&'static str),
    Whole,
}

#[derive(Clone, Copy)]
enum ItRule {
    Fixed(&'static str),
    ChangedModule,
    AllModules(&'static str),
}

#[derive(Clone, Copy)]
struct ItFilter {
    package: &'static str,
    rule: ItRule,
}

#[derive(Clone, Copy)]
struct LargeInput {
    package: &'static str,
    filter: &'static str,
    exact: bool,
}

#[derive(Clone, Copy)]
struct Rule {
    prefix: &'static str,
    package: Option<&'static str>,
    lib: Option<LibRule>,
    it_filters: &'static [ItFilter],
    large_input: &'static [LargeInput],
    note: Option<&'static str>,
    skips: &'static [&'static str],
    threads: Option<usize>,
}

const EMIT_CONSUMERS: &[ItFilter] = &[
    ItFilter {
        package: "fsl-analyze",
        rule: ItRule::Fixed("monitor_"),
    },
    ItFilter {
        package: "fsl-analyze",
        rule: ItRule::Fixed("timed_"),
    },
    ItFilter {
        package: "fsl-analyze",
        rule: ItRule::Fixed("hyper"),
    },
    ItFilter {
        package: "fsl-analyze",
        rule: ItRule::Fixed("probabilistic"),
    },
    ItFilter {
        package: "fsl-command",
        rule: ItRule::Fixed("run_requirement_items"),
    },
];
const PROBABILITY_CHECK_MODULES: ItFilter = ItFilter {
    package: "fsl-probability-check",
    rule: ItRule::AllModules("fsl-probability-check"),
};
const REPLAY_CONSUMERS: &[ItFilter] = &[
    ItFilter {
        package: "fsl-command",
        rule: ItRule::Fixed("dependency_handoff"),
    },
    ItFilter {
        package: "fsl-command",
        rule: ItRule::Fixed("run_requirement_items"),
    },
    ItFilter {
        package: "fsl-command",
        rule: ItRule::Fixed("run_text_witness"),
    },
    ItFilter {
        package: "fsl-analyze",
        rule: ItRule::Fixed("probabilistic_replay"),
    },
    ItFilter {
        package: "fsl-analyze",
        rule: ItRule::Fixed("bounded_probabilistic_replay"),
    },
    ItFilter {
        package: "fsl-analyze",
        rule: ItRule::Fixed("package_probabilistic"),
    },
];
/// Replay certificate and probability paths: the replay consumers plus every
/// `fsl-probability-check` module, which checks certificates through them.
const REPLAY_PROBABILITY_CONSUMERS: &[ItFilter] = &[
    PROBABILITY_CHECK_MODULES,
    ItFilter {
        package: "fsl-command",
        rule: ItRule::Fixed("dependency_handoff"),
    },
    ItFilter {
        package: "fsl-command",
        rule: ItRule::Fixed("run_requirement_items"),
    },
    ItFilter {
        package: "fsl-command",
        rule: ItRule::Fixed("run_text_witness"),
    },
    ItFilter {
        package: "fsl-analyze",
        rule: ItRule::Fixed("probabilistic_replay"),
    },
    ItFilter {
        package: "fsl-analyze",
        rule: ItRule::Fixed("bounded_probabilistic_replay"),
    },
    ItFilter {
        package: "fsl-analyze",
        rule: ItRule::Fixed("package_probabilistic"),
    },
];
/// Probabilistic front-end checks: the checker modules compile edited sources
/// through them.
const SEMANTICS_PROBABILITY_CONSUMERS: &[ItFilter] = &[
    PROBABILITY_CHECK_MODULES,
    ItFilter {
        package: "fsl-analyze",
        rule: ItRule::Fixed("timed_"),
    },
    ItFilter {
        package: "fsl-analyze",
        rule: ItRule::Fixed("timed_within_200"),
    },
    ItFilter {
        package: "fsl-analyze",
        rule: ItRule::Fixed("timed_within_201"),
    },
];
const INSPECT_CONSUMERS: &[ItFilter] = &[ItFilter {
    package: "fsl-command",
    rule: ItRule::Fixed("run_text_witness"),
}];
/// The engine filters every analyze source change runs, then the modules that
/// hold the source-route and package-route parity tests (FSL FR-940) for the
/// shared request settlement: `request.rs` and `analyze.rs` decide an item's
/// disposition for both routes. `document.rs` and `document/**` settle
/// both routes' documents and compositions; `analyze_composition_cli` holds
/// the composition parity tests (FR-1044-AC-10).
const ROUTE_PARITY_CONSUMERS: &[ItFilter] = &[
    ItFilter {
        package: "fsl-analyze",
        rule: ItRule::Fixed("monitor_"),
    },
    ItFilter {
        package: "fsl-analyze",
        rule: ItRule::Fixed("timed_"),
    },
    ItFilter {
        package: "fsl-analyze",
        rule: ItRule::Fixed("hyper"),
    },
    ItFilter {
        package: "fsl-analyze",
        rule: ItRule::Fixed("probabilistic"),
    },
    ItFilter {
        package: "fsl-analyze",
        rule: ItRule::Fixed("analyze_package_selection"),
    },
    ItFilter {
        package: "fsl-analyze",
        rule: ItRule::Fixed("analyze_request_wire"),
    },
    ItFilter {
        package: "fsl-analyze",
        rule: ItRule::Fixed("analyze_package_provider"),
    },
    ItFilter {
        package: "fsl-analyze",
        rule: ItRule::Fixed("analyze_composition_cli"),
    },
];
const EXACT_CONSUMERS: &[ItFilter] = &[
    PROBABILITY_CHECK_MODULES,
    ItFilter {
        package: "fsl-analyze",
        rule: ItRule::Fixed("probabilistic"),
    },
    ItFilter {
        package: "fsl-analyze",
        rule: ItRule::Fixed("timed_"),
    },
];
const COMMAND_PACKAGE_CONSUMERS: &[ItFilter] = &[
    ItFilter {
        package: "fsl-command",
        rule: ItRule::Fixed("package_run"),
    },
    ItFilter {
        package: "fsl-command",
        rule: ItRule::Fixed("run_requirement_items"),
    },
    ItFilter {
        package: "fsl-command",
        rule: ItRule::Fixed("dependency_handoff"),
    },
    ItFilter {
        package: "fsl-command",
        rule: ItRule::Fixed("run_text_witness"),
    },
];
const COMMAND_REQUIREMENT_CONSUMERS: &[ItFilter] = &[ItFilter {
    package: "fsl-command",
    rule: ItRule::Fixed("run_requirement_items"),
}];
const COMMAND_DEPENDENCY_CONSUMERS: &[ItFilter] = &[ItFilter {
    package: "fsl-command",
    rule: ItRule::Fixed("dependency_handoff"),
}];
const COMMAND_TEXT_CONSUMERS: &[ItFilter] = &[ItFilter {
    package: "fsl-command",
    rule: ItRule::Fixed("run_text_witness"),
}];
const COMMAND_CLI_CONSUMERS: &[ItFilter] = &[ItFilter {
    package: "fsl-command",
    rule: ItRule::Fixed("cli_contract"),
}];
const COMMAND_ALL_CONSUMERS: &[ItFilter] = &[ItFilter {
    package: "fsl-command",
    rule: ItRule::AllModules("fsl-command"),
}];
const BARE_CONSUMERS: &[ItFilter] = &[
    ItFilter {
        package: "fsl-analyze",
        rule: ItRule::Fixed("timed_"),
    },
    ItFilter {
        package: "filament-spec-language",
        rule: ItRule::Fixed("bare_"),
    },
];
/// Analyze integration modules that read concrete protocol state keys,
/// message ordinals and delivery identities through `fsl-eval` simulation.
const PROTOCOL_STATE_KEY_CONSUMERS: &[ItFilter] = &[
    ItFilter {
        package: "fsl-analyze",
        rule: ItRule::Fixed("protocol_bounded_lasso"),
    },
    ItFilter {
        package: "fsl-analyze",
        rule: ItRule::Fixed("race_freedom"),
    },
    ItFilter {
        package: "fsl-analyze",
        rule: ItRule::Fixed("native_correlation"),
    },
];
const ABSTRACTION_CONSUMERS: &[ItFilter] = &[
    ItFilter {
        package: "fsl-analyze",
        rule: ItRule::Fixed("timed_"),
    },
    ItFilter {
        package: "filament-spec-language",
        rule: ItRule::Fixed("abstraction_relation_inert"),
    },
];
const TIMED_CONSUMER: &[ItFilter] = &[
    ItFilter {
        package: "fsl-analyze",
        rule: ItRule::Fixed("timed_"),
    },
    ItFilter {
        package: "fsl-analyze",
        rule: ItRule::Fixed("timed_within_200"),
    },
    ItFilter {
        package: "fsl-analyze",
        rule: ItRule::Fixed("timed_within_201"),
    },
];
const ROOT_IT_MODULES: &[ItFilter] = &[ItFilter {
    package: "filament-spec-language",
    rule: ItRule::AllModules("filament-spec-language"),
}];
const ANALYZE_IT_MODULES: &[ItFilter] = &[ItFilter {
    package: "fsl-analyze",
    rule: ItRule::AllModules("fsl-analyze"),
}];
const COMMAND_IT_MODULES: &[ItFilter] = &[ItFilter {
    package: "fsl-command",
    rule: ItRule::AllModules("fsl-command"),
}];
const LARGE_INPUTS: &[LargeInput] = &[
    LargeInput {
        package: "fsl-package",
        filter: LARGE_EMIT_FILTER,
        exact: true,
    },
    LargeInput {
        package: "fsl-package",
        filter: "emit::tests::deep_bodies::a_100000_deep_body_checks_lowers_and_emits_on_a_small_stack",
        exact: true,
    },
    LargeInput {
        package: "fsl-package",
        filter: "emit::tests::deep_bodies::every_nested_form_emits_in_the_stratified_grammar_at_2_and_1000_levels",
        exact: true,
    },
    LargeInput {
        package: "fsl-package",
        filter: "emit::tests::large_cycle::a_100000_node_cycle_emits_and_reads_back_under_supplied_limits",
        exact: true,
    },
    LargeInput {
        package: "fsl-package",
        filter: "emit::tests::protocol_recursion::a_100000_member_function_protocol_cycle_is_budgeted_and_order_invariant",
        exact: true,
    },
    LargeInput {
        package: "fsl-package",
        filter: "emit::tests::protocol_recursion::a_100000_member_function_protocol_cycle_reads_back_at_i2",
        exact: true,
    },
    LargeInput {
        package: "fsl-replay",
        filter: "tc_736_a_100000_term_source_and_list_replay_to_the_proving_verdict",
        exact: false,
    },
    LargeInput {
        package: "fsl-replay",
        filter: "tc_736_a_100000_long_list_replays_under_the_raised_input_bound",
        exact: false,
    },
    LargeInput {
        package: "fsl-replay",
        filter: "tc_736_a_100000_long_list_decodes_and_walks_on_a_small_stack",
        exact: false,
    },
];
const PROJECTION_LARGE_INPUTS: &[LargeInput] = &[LargeInput {
    package: "fsl-package",
    filter: LARGE_EMIT_FILTER,
    exact: true,
}];

const RULES: &[Rule] = &[
    Rule {
        prefix: "fsl-package/src/emit.rs",
        package: Some("fsl-package"),
        lib: Some(LibRule::Fixed("emit")),
        it_filters: EMIT_CONSUMERS,
        large_input: LARGE_INPUTS,
        note: None,
        skips: &EMIT_SKIP,
        threads: Some(1),
    },
    Rule {
        prefix: "fsl-package/src/emit/",
        package: Some("fsl-package"),
        lib: Some(LibRule::Fixed("emit")),
        it_filters: EMIT_CONSUMERS,
        large_input: LARGE_INPUTS,
        note: None,
        skips: &EMIT_SKIP,
        threads: Some(1),
    },
    Rule {
        prefix: "fsl-package/src/reader_protocol_projection.rs",
        package: Some("fsl-package"),
        lib: Some(LibRule::SourceModule),
        it_filters: &[],
        large_input: PROJECTION_LARGE_INPUTS,
        note: None,
        skips: &[],
        threads: None,
    },
    Rule {
        prefix: "fsl-package/src/checked_v2.rs",
        package: Some("fsl-package"),
        lib: Some(LibRule::SourceModule),
        it_filters: &[],
        large_input: PROJECTION_LARGE_INPUTS,
        note: None,
        skips: &[],
        threads: None,
    },
    Rule {
        prefix: "fsl-forms/src/spans.rs",
        package: Some("fsl-forms"),
        lib: Some(LibRule::Whole),
        it_filters: &[],
        large_input: PROJECTION_LARGE_INPUTS,
        note: None,
        skips: &[],
        threads: None,
    },
    Rule {
        prefix: "fsl-analyze/src/analyze/exact.rs",
        package: Some("fsl-analyze"),
        lib: Some(LibRule::SourceModule),
        it_filters: EXACT_CONSUMERS,
        large_input: &[],
        note: None,
        skips: &[],
        threads: None,
    },
    Rule {
        prefix: "fsl-analyze/src/analyze/exact/",
        package: Some("fsl-analyze"),
        lib: Some(LibRule::SourceModule),
        it_filters: EXACT_CONSUMERS,
        large_input: &[],
        note: None,
        skips: &[],
        threads: None,
    },
    Rule {
        prefix: "fsl-analyze/src/monitor/",
        package: Some("fsl-analyze"),
        lib: Some(LibRule::SourceModule),
        it_filters: &[ItFilter {
            package: "fsl-analyze",
            rule: ItRule::Fixed("monitor_"),
        }],
        large_input: &[],
        note: None,
        skips: &[],
        threads: None,
    },
    Rule {
        prefix: "fsl-analyze/src/zone_check/",
        package: Some("fsl-analyze"),
        lib: Some(LibRule::SourceModule),
        it_filters: TIMED_CONSUMER,
        large_input: &[],
        note: None,
        skips: &[],
        threads: None,
    },
    Rule {
        prefix: "fsl-analyze/src/model_check/hyper/",
        package: Some("fsl-analyze"),
        lib: Some(LibRule::SourceModule),
        it_filters: &[ItFilter {
            package: "fsl-analyze",
            rule: ItRule::Fixed("hyper"),
        }],
        large_input: &[],
        note: None,
        skips: &[],
        threads: None,
    },
    Rule {
        prefix: "fsl-analyze/src/probabilistic/",
        package: Some("fsl-analyze"),
        lib: Some(LibRule::SourceModule),
        it_filters: &[ItFilter {
            package: "fsl-analyze",
            rule: ItRule::Fixed("probabilistic"),
        }],
        large_input: &[],
        note: None,
        skips: &[],
        threads: None,
    },
    Rule {
        prefix: "fsl-analyze/src/analyze/request.rs",
        package: Some("fsl-analyze"),
        lib: Some(LibRule::SourceModule),
        it_filters: ROUTE_PARITY_CONSUMERS,
        large_input: &[],
        note: None,
        skips: &[],
        threads: None,
    },
    Rule {
        prefix: "fsl-analyze/src/analyze.rs",
        package: Some("fsl-analyze"),
        lib: Some(LibRule::SourceModule),
        it_filters: ROUTE_PARITY_CONSUMERS,
        large_input: &[],
        note: None,
        skips: &[],
        threads: None,
    },
    Rule {
        prefix: "fsl-analyze/src/analyze/document",
        package: Some("fsl-analyze"),
        lib: Some(LibRule::SourceModule),
        it_filters: ROUTE_PARITY_CONSUMERS,
        large_input: &[],
        note: None,
        skips: &[],
        threads: None,
    },
    Rule {
        prefix: "fsl-analyze/src/",
        package: Some("fsl-analyze"),
        lib: Some(LibRule::SourceModule),
        it_filters: &[
            ItFilter {
                package: "fsl-analyze",
                rule: ItRule::Fixed("monitor_"),
            },
            ItFilter {
                package: "fsl-analyze",
                rule: ItRule::Fixed("timed_"),
            },
            ItFilter {
                package: "fsl-analyze",
                rule: ItRule::Fixed("hyper"),
            },
            ItFilter {
                package: "fsl-analyze",
                rule: ItRule::Fixed("probabilistic"),
            },
        ],
        large_input: &[],
        note: None,
        skips: &[],
        threads: None,
    },
    Rule {
        prefix: "fsl-analyze/tests/it/main.rs",
        package: Some("fsl-analyze"),
        lib: None,
        it_filters: ANALYZE_IT_MODULES,
        large_input: &[],
        note: None,
        skips: &[],
        threads: None,
    },
    Rule {
        prefix: "fsl-analyze/tests/it/",
        package: Some("fsl-analyze"),
        lib: None,
        it_filters: &[ItFilter {
            package: "fsl-analyze",
            rule: ItRule::ChangedModule,
        }],
        large_input: &[],
        note: None,
        skips: &[],
        threads: None,
    },
    Rule {
        prefix: "fsl-replay/src/",
        package: Some("fsl-replay"),
        lib: Some(LibRule::SourceModule),
        it_filters: REPLAY_CONSUMERS,
        large_input: &[],
        note: None,
        skips: &[],
        threads: None,
    },
    Rule {
        prefix: "fsl-replay/src/certificate",
        package: Some("fsl-replay"),
        lib: Some(LibRule::SourceModule),
        it_filters: REPLAY_PROBABILITY_CONSUMERS,
        large_input: &[],
        note: None,
        skips: &[],
        threads: None,
    },
    Rule {
        prefix: "fsl-replay/src/probability_certificate",
        package: Some("fsl-replay"),
        lib: Some(LibRule::SourceModule),
        it_filters: REPLAY_PROBABILITY_CONSUMERS,
        large_input: &[],
        note: None,
        skips: &[],
        threads: None,
    },
    Rule {
        prefix: "fsl-replay/src/exact_probabilistic",
        package: Some("fsl-replay"),
        lib: Some(LibRule::SourceModule),
        it_filters: REPLAY_PROBABILITY_CONSUMERS,
        large_input: &[],
        note: None,
        skips: &[],
        threads: None,
    },
    Rule {
        prefix: "fsl-replay/src/spine",
        package: Some("fsl-replay"),
        lib: Some(LibRule::SourceModule),
        it_filters: REPLAY_PROBABILITY_CONSUMERS,
        large_input: &[],
        note: None,
        skips: &[],
        threads: None,
    },
    Rule {
        prefix: "fsl-replay/src/product_action.rs",
        package: Some("fsl-replay"),
        lib: Some(LibRule::SourceModule),
        it_filters: REPLAY_PROBABILITY_CONSUMERS,
        large_input: &[],
        note: None,
        skips: &[],
        threads: None,
    },
    Rule {
        prefix: "fsl-replay/src/identity.rs",
        package: Some("fsl-replay"),
        lib: Some(LibRule::SourceModule),
        it_filters: REPLAY_PROBABILITY_CONSUMERS,
        large_input: &[],
        note: None,
        skips: &[],
        threads: None,
    },
    Rule {
        prefix: "fsl-replay/src/limits.rs",
        package: Some("fsl-replay"),
        lib: Some(LibRule::SourceModule),
        it_filters: REPLAY_PROBABILITY_CONSUMERS,
        large_input: &[],
        note: None,
        skips: &[],
        threads: None,
    },
    Rule {
        prefix: "fsl-replay/src/proof_result",
        package: Some("fsl-replay"),
        lib: Some(LibRule::SourceModule),
        it_filters: REPLAY_PROBABILITY_CONSUMERS,
        large_input: &[],
        note: None,
        skips: &[],
        threads: None,
    },
    Rule {
        prefix: "fsl-replay/src/outcome",
        package: Some("fsl-replay"),
        lib: Some(LibRule::SourceModule),
        it_filters: REPLAY_PROBABILITY_CONSUMERS,
        large_input: &[],
        note: None,
        skips: &[],
        threads: None,
    },
    Rule {
        prefix: "fsl-replay/src/request",
        package: Some("fsl-replay"),
        lib: Some(LibRule::SourceModule),
        it_filters: REPLAY_PROBABILITY_CONSUMERS,
        large_input: &[],
        note: None,
        skips: &[],
        threads: None,
    },
    Rule {
        prefix: "fsl-replay/src/statistical",
        package: Some("fsl-replay"),
        lib: Some(LibRule::SourceModule),
        it_filters: REPLAY_PROBABILITY_CONSUMERS,
        large_input: &[],
        note: None,
        skips: &[],
        threads: None,
    },
    Rule {
        prefix: "fsl-replay/src/model_check/limits",
        package: Some("fsl-replay"),
        lib: Some(LibRule::SourceModule),
        it_filters: REPLAY_PROBABILITY_CONSUMERS,
        large_input: &[],
        note: None,
        skips: &[],
        threads: None,
    },
    Rule {
        prefix: "fsl-analyze/src/exact_probabilistic",
        package: Some("fsl-analyze"),
        lib: Some(LibRule::SourceModule),
        it_filters: EXACT_CONSUMERS,
        large_input: &[],
        note: None,
        skips: &[],
        threads: None,
    },
    Rule {
        prefix: "fsl-semantics/src/check/probabilistic",
        package: Some("fsl-semantics"),
        lib: Some(LibRule::SourceModule),
        it_filters: SEMANTICS_PROBABILITY_CONSUMERS,
        large_input: &[],
        note: None,
        skips: &[],
        threads: None,
    },
    Rule {
        prefix: "fsl-semantics/src/check/mod.rs",
        package: Some("fsl-semantics"),
        lib: Some(LibRule::SourceModule),
        it_filters: SEMANTICS_PROBABILITY_CONSUMERS,
        large_input: &[],
        note: None,
        skips: &[],
        threads: None,
    },
    Rule {
        prefix: "fsl-inspect/src/",
        package: Some("fsl-inspect"),
        lib: Some(LibRule::Fixed("render::tests::")),
        it_filters: INSPECT_CONSUMERS,
        large_input: &[],
        note: None,
        skips: &[],
        threads: None,
    },
    Rule {
        prefix: "fsl-command/src/command/package_run.rs",
        package: Some("fsl-command"),
        lib: Some(LibRule::SourceModule),
        it_filters: COMMAND_PACKAGE_CONSUMERS,
        large_input: &[],
        note: Some(
            "driver pin move: also run driver targets frontend_cli, binary_process, cli_contract",
        ),
        skips: &[],
        threads: None,
    },
    Rule {
        prefix: "fsl-command/src/command/requirements.rs",
        package: Some("fsl-command"),
        lib: Some(LibRule::SourceModule),
        it_filters: COMMAND_REQUIREMENT_CONSUMERS,
        large_input: &[],
        note: None,
        skips: &[],
        threads: None,
    },
    Rule {
        prefix: "fsl-command/src/command/items.rs",
        package: Some("fsl-command"),
        lib: Some(LibRule::SourceModule),
        it_filters: COMMAND_REQUIREMENT_CONSUMERS,
        large_input: &[],
        note: None,
        skips: &[],
        threads: None,
    },
    Rule {
        prefix: "fsl-command/src/command/dependency_manifest.rs",
        package: Some("fsl-command"),
        lib: Some(LibRule::SourceModule),
        it_filters: COMMAND_DEPENDENCY_CONSUMERS,
        large_input: &[],
        note: None,
        skips: &[],
        threads: None,
    },
    Rule {
        prefix: "fsl-command/src/command/replay.rs",
        package: Some("fsl-command"),
        lib: Some(LibRule::SourceModule),
        it_filters: &[
            ItFilter {
                package: "fsl-command",
                rule: ItRule::Fixed("dependency_handoff"),
            },
            ItFilter {
                package: "fsl-command",
                rule: ItRule::Fixed("run_text_witness"),
            },
        ],
        large_input: &[],
        note: None,
        skips: &[],
        threads: None,
    },
    Rule {
        prefix: "fsl-command/src/command/monitor.rs",
        package: Some("fsl-command"),
        lib: Some(LibRule::SourceModule),
        it_filters: COMMAND_CLI_CONSUMERS,
        large_input: &[],
        note: None,
        skips: &[],
        threads: None,
    },
    Rule {
        prefix: "fsl-command/src/command/analyze.rs",
        package: Some("fsl-command"),
        lib: Some(LibRule::SourceModule),
        it_filters: COMMAND_CLI_CONSUMERS,
        large_input: &[],
        note: None,
        skips: &[],
        threads: None,
    },
    Rule {
        prefix: "fsl-command/src/command/clause.rs",
        package: Some("fsl-command"),
        lib: Some(LibRule::SourceModule),
        it_filters: COMMAND_CLI_CONSUMERS,
        large_input: &[],
        note: None,
        skips: &[],
        threads: None,
    },
    Rule {
        prefix: "fsl-command/src/command/export.rs",
        package: Some("fsl-command"),
        lib: Some(LibRule::SourceModule),
        it_filters: COMMAND_CLI_CONSUMERS,
        large_input: &[],
        note: None,
        skips: &[],
        threads: None,
    },
    Rule {
        prefix: "fsl-command/src/command/output",
        package: Some("fsl-command"),
        lib: Some(LibRule::SourceModule),
        it_filters: COMMAND_TEXT_CONSUMERS,
        large_input: &[],
        note: None,
        skips: &[],
        threads: None,
    },
    Rule {
        prefix: "fsl-command/src/command/settle.rs",
        package: Some("fsl-command"),
        lib: Some(LibRule::SourceModule),
        it_filters: COMMAND_TEXT_CONSUMERS,
        large_input: &[],
        note: None,
        skips: &[],
        threads: None,
    },
    Rule {
        prefix: "fsl-command/src/",
        package: Some("fsl-command"),
        lib: Some(LibRule::SourceModule),
        it_filters: COMMAND_ALL_CONSUMERS,
        large_input: &[],
        note: None,
        skips: &[],
        threads: None,
    },
    Rule {
        prefix: "fsl-command/tests/it/main.rs",
        package: Some("fsl-command"),
        lib: None,
        it_filters: COMMAND_IT_MODULES,
        large_input: &[],
        note: None,
        skips: &[],
        threads: None,
    },
    Rule {
        prefix: "fsl-command/tests/it/",
        package: Some("fsl-command"),
        lib: None,
        it_filters: &[ItFilter {
            package: "fsl-command",
            rule: ItRule::ChangedModule,
        }],
        large_input: &[],
        note: None,
        skips: &[],
        threads: None,
    },
    Rule {
        prefix: "tests/it/main.rs",
        package: Some("filament-spec-language"),
        lib: None,
        it_filters: ROOT_IT_MODULES,
        large_input: &[],
        note: None,
        skips: &[],
        threads: None,
    },
    Rule {
        prefix: "tests/it/",
        package: Some("filament-spec-language"),
        lib: None,
        it_filters: &[ItFilter {
            package: "filament-spec-language",
            rule: ItRule::ChangedModule,
        }],
        large_input: &[],
        note: None,
        skips: &[],
        threads: None,
    },
    Rule {
        prefix: "fsl-eval/src/value/expression/bare.rs",
        package: Some("fsl-eval"),
        lib: Some(LibRule::SourceModule),
        it_filters: BARE_CONSUMERS,
        large_input: &[],
        note: None,
        skips: &[],
        threads: None,
    },
    Rule {
        prefix: "fsl-semantics/src/check/abstraction.rs",
        package: Some("fsl-semantics"),
        lib: Some(LibRule::SourceModule),
        it_filters: ABSTRACTION_CONSUMERS,
        large_input: &[],
        note: None,
        skips: &[],
        threads: None,
    },
    Rule {
        prefix: "fsl-eval/src/simulation/",
        package: Some("fsl-eval"),
        lib: Some(LibRule::SourceModule),
        it_filters: PROTOCOL_STATE_KEY_CONSUMERS,
        large_input: &[],
        note: None,
        skips: &[],
        threads: None,
    },
    Rule {
        prefix: "fsl-eval/src/",
        package: Some("fsl-eval"),
        lib: Some(LibRule::SourceModule),
        it_filters: TIMED_CONSUMER,
        large_input: &[],
        note: Some("unknown fsl-eval source path: add a consumer route after reviewing its tests"),
        skips: &[],
        threads: None,
    },
    Rule {
        prefix: "fsl-semantics/src/",
        package: Some("fsl-semantics"),
        lib: Some(LibRule::SourceModule),
        it_filters: TIMED_CONSUMER,
        large_input: &[],
        note: Some(
            "unknown fsl-semantics source path: add a consumer route after reviewing its tests",
        ),
        skips: &[],
        threads: None,
    },
    Rule {
        prefix: "xtask/",
        package: Some("xtask"),
        lib: Some(LibRule::Whole),
        it_filters: &[],
        large_input: &[],
        note: None,
        skips: &[],
        threads: None,
    },
    Rule {
        prefix: "fsl-foundation/src/",
        package: Some("fsl-foundation"),
        lib: Some(LibRule::Whole),
        it_filters: &[],
        large_input: &[],
        note: None,
        skips: &[],
        threads: None,
    },
    Rule {
        prefix: "fsl-source/src/",
        package: Some("fsl-source"),
        lib: Some(LibRule::Whole),
        it_filters: &[],
        large_input: &[],
        note: None,
        skips: &[],
        threads: None,
    },
    Rule {
        prefix: "fsl-cst/src/",
        package: Some("fsl-cst"),
        lib: Some(LibRule::Whole),
        it_filters: &[],
        large_input: &[],
        note: None,
        skips: &[],
        threads: None,
    },
    Rule {
        prefix: "fsl-forms/src/",
        package: Some("fsl-forms"),
        lib: Some(LibRule::Whole),
        it_filters: &[],
        large_input: &[],
        note: None,
        skips: &[],
        threads: None,
    },
    Rule {
        prefix: "fsl-package/src/",
        package: Some("fsl-package"),
        lib: Some(LibRule::SourceModule),
        it_filters: &[],
        large_input: &[],
        note: None,
        skips: &[],
        threads: None,
    },
];

impl TestSelector {
    fn command(&self, list_only: bool) -> Vec<String> {
        let mut arguments = vec![
            "test".to_owned(),
            "--locked".to_owned(),
            "-p".to_owned(),
            self.package.clone(),
        ];
        match &self.target {
            TestTarget::Library => arguments.push("--lib".to_owned()),
            TestTarget::Integration(name) => {
                arguments.push("--test".to_owned());
                arguments.push(name.clone());
            }
        }
        if let Some(filter) = &self.filter {
            arguments.push("--".to_owned());
            arguments.push(filter.clone());
        } else if self.exact || !self.skip.is_empty() || self.threads.is_some() || list_only {
            arguments.push("--".to_owned());
        }
        if self.exact {
            arguments.push("--exact".to_owned());
        }
        for skip in &self.skip {
            arguments.push("--skip".to_owned());
            arguments.push(skip.clone());
        }
        if let Some(threads) = self.threads {
            arguments.push(format!("--test-threads={threads}"));
        }
        if list_only {
            arguments.push("--list".to_owned());
        }
        arguments
    }

    fn display(&self) -> String {
        let cargo = self.command(false).join(" ");
        if self.timed {
            format!("/usr/bin/time -v cargo {cargo}")
        } else {
            format!("cargo {cargo}")
        }
    }
}

struct ProcessOutput {
    success: bool,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

trait CommandRunner {
    fn run(
        &self,
        cwd: &Path,
        program: &OsStr,
        arguments: &[OsString],
    ) -> std::io::Result<ProcessOutput>;
}

struct SystemCommandRunner;

impl CommandRunner for SystemCommandRunner {
    fn run(
        &self,
        cwd: &Path,
        program: &OsStr,
        arguments: &[OsString],
    ) -> std::io::Result<ProcessOutput> {
        let output = Command::new(program)
            .current_dir(cwd)
            .args(arguments)
            .output()?;
        Ok(ProcessOutput {
            success: output.status.success(),
            stdout: output.stdout,
            stderr: output.stderr,
        })
    }
}

/// Print compile-first and name-filtered test commands for changed paths.
///
/// Each filter is preflighted with a filtered `--list` invocation. A filter
/// that selects no tests fails the command instead of printing a false gate.
pub fn run(workspace_root: &Path, operands: &[OsString]) -> Result<String> {
    run_with_runner(workspace_root, operands, &SystemCommandRunner)
}

fn run_with_runner(
    workspace_root: &Path,
    operands: &[OsString],
    runner: &dyn CommandRunner,
) -> Result<String> {
    if operands.is_empty() {
        return Err(Error::Usage(USAGE));
    }
    let paths = operands
        .iter()
        .map(|operand| {
            operand
                .to_str()
                .map(normalize_path)
                .ok_or(Error::Usage(USAGE))
        })
        .collect::<Result<Vec<_>>>()?;
    let (head, main) = require_main_merged(workspace_root, runner)?;
    let mut output = format!("# PR head {head}; origin/main {main}\n");
    if paths.iter().all(|path| path.starts_with("spec/")) {
        output.push_str("NO CARGO: spec-only change.\n");
        return Ok(output);
    }
    let mut recipe = plan_paths(workspace_root, &paths)?;
    for path in &paths {
        if is_test_path(path) && ignore_attribute_changed(workspace_root, path, runner)? {
            add_whole_library(&mut recipe, "xtask");
        }
    }

    let selectors = recipe.selectors.iter().cloned().collect::<Vec<_>>();
    for selector in selectors {
        let matched = preflight(workspace_root, &selector, runner)?;
        if matched == 0 {
            if selector.no_match_is_note {
                recipe.selectors.remove(&selector);
                recipe.notes.insert(format!(
                    "no named tests match the local module filter {:?} in {}",
                    selector.filter, selector.package
                ));
            } else {
                return Err(Error::MergeGateRecipe(format!(
                    "test filter {:?} matched no non-ignored tests; refusing to print a green recipe",
                    selector.display()
                )));
            }
        }
    }
    for target in compile_targets(&recipe) {
        output.push_str(&format!("{}\n", no_run_command(&target)));
    }
    for selector in &recipe.selectors {
        output.push_str(&format!("{}\n", selector.display()));
    }
    for note in &recipe.notes {
        output.push_str(&format!("NOTE: {note}\n"));
    }
    output.push_str("cargo check --locked --workspace --all-targets\n");
    Ok(output)
}

fn normalize_path(path: &str) -> String {
    let path = path.replace('\\', "/");
    path.trim_start_matches("./")
        .trim_end_matches('/')
        .to_owned()
}

fn plan_paths(workspace_root: &Path, paths: &[String]) -> Result<Recipe> {
    let mut recipe = Recipe::default();
    for path in paths {
        if path.is_empty()
            || Path::new(path).is_absolute()
            || path.split('/').any(|part| part == "..")
        {
            return Err(Error::MergeGateRecipe(format!(
                "invalid changed path {path:?}"
            )));
        }
        if path.starts_with("spec/") {
            recipe
                .notes
                .insert(format!("no targeted cargo tests for {path}"));
            continue;
        }
        if is_non_crate_path(path) {
            recipe
                .notes
                .insert(format!("no targeted cargo tests for {path}"));
            continue;
        }
        if !path_exists(workspace_root, path) && !is_virtual_path(path) {
            recipe.notes.insert(format!(
                "changed path {path} is absent; using crate fallback"
            ));
            apply_crate_fallback(path, workspace_root, &mut recipe)?;
            continue;
        }
        plan_path(workspace_root, path, &mut recipe)?;
    }
    Ok(recipe)
}

fn plan_path(workspace_root: &Path, path: &str, recipe: &mut Recipe) -> Result<()> {
    if path == "Cargo.toml" || path == "Cargo.lock" {
        apply_manifest_rule(recipe);
        return Ok(());
    }
    if matches!(path, "src/lib.rs" | "src/main.rs" | "src/mod.rs") {
        add_whole_library(recipe, "filament-spec-language");
        return Ok(());
    }
    if let Some(relative) = path.strip_prefix("tools/fsl-probability-check/") {
        return plan_probability_check(workspace_root, path, relative, recipe);
    }
    if path.ends_with("/Cargo.toml") {
        apply_crate_fallback(path, workspace_root, recipe)?;
        return Ok(());
    }
    if path == "tests/it/main.rs" || path.ends_with("/tests/it/main.rs") {
        if let Some(rule) = matching_rule(path) {
            apply_rule(workspace_root, path, rule, recipe)?;
        } else {
            add_all_it_modules(workspace_root, test_package(path), recipe)?;
        }
        return Ok(());
    }
    if path.starts_with("xtask/") {
        add_whole_library(recipe, "xtask");
        return Ok(());
    }
    if path.contains("/tests/fixtures/") || path.contains("/tests/support/") {
        let package = path.split('/').next().unwrap_or_default();
        add_all_it_modules(workspace_root, package, recipe)?;
        return Ok(());
    }
    if is_test_path(path) {
        return plan_test_path(workspace_root, path, recipe);
    }
    if let Some(package) = crate_name(path) {
        if is_crate_root_path(path, package) {
            apply_crate_fallback(path, workspace_root, recipe)?;
            return Ok(());
        }
    }
    if let Some(rule) = matching_rule(path) {
        apply_rule(workspace_root, path, rule, recipe)?;
        return Ok(());
    }
    if let Some(package) = crate_name(path) {
        if path.starts_with(&format!("{package}/src/")) {
            add_library(recipe, package, source_module_filter(path)?, true);
            recipe.notes.insert(format!(
                "no explicit path rule for {path}; using its local library module"
            ));
            return Ok(());
        }
        apply_crate_fallback(path, workspace_root, recipe)?;
        return Ok(());
    }
    recipe
        .notes
        .insert(format!("no targeted cargo tests for {path}"));
    Ok(())
}

/// The checker is a tool outside the `fsl-` crate directories: a changed IT module
/// runs that module, and any other change (IT root, fixtures, source) runs
/// every module.
fn plan_probability_check(
    workspace_root: &Path,
    path: &str,
    relative: &str,
    recipe: &mut Recipe,
) -> Result<()> {
    let module = relative
        .strip_prefix("tests/it/")
        .filter(|module| *module != "main.rs");
    if module.is_some() {
        add_integration(
            recipe,
            "fsl-probability-check",
            integration_module_filter(path)?,
        );
        return Ok(());
    }
    add_all_it_modules(workspace_root, "fsl-probability-check", recipe)
}

fn plan_test_path(workspace_root: &Path, path: &str, recipe: &mut Recipe) -> Result<()> {
    if path.starts_with("xtask/tests/") {
        let file = workspace_root.join(path);
        let source = fs::read_to_string(&file).map_err(|error| {
            Error::MergeGateRecipe(format!("cannot read {}: {error}", file.display()))
        })?;
        let syntax = syn::parse_file(&source).map_err(|error| {
            Error::MergeGateRecipe(format!("cannot parse {}: {error}", file.display()))
        })?;
        let mut names = Vec::new();
        collect_test_names(&syntax.items, &mut Vec::new(), &mut names);
        if names.is_empty() {
            return Err(Error::MergeGateRecipe(format!(
                "changed test file {path:?} declares no named tests"
            )));
        }
        let target = path
            .rsplit('/')
            .next()
            .and_then(|file| file.strip_suffix(".rs"))
            .ok_or_else(|| {
                Error::MergeGateRecipe(format!("changed test path {path:?} is not Rust"))
            })?;
        for name in names {
            add_target_filter(
                recipe,
                "xtask",
                TestTarget::Integration(target.to_owned()),
                Some(name),
                true,
                &[],
                false,
                None,
                false,
            );
        }
        return Ok(());
    }
    if path == "tests/it/main.rs" || path.ends_with("/tests/it/main.rs") {
        if let Some(rule) = matching_rule(path) {
            return apply_rule(workspace_root, path, rule, recipe);
        }
        return add_all_it_modules(workspace_root, test_package(path), recipe);
    }
    if let Some(package) = crate_name(path) {
        if path.contains("/tests/fixtures/") || path.contains("/tests/support/") {
            return add_all_it_modules(workspace_root, package, recipe);
        }
        if path.starts_with("xtask/tests/") {
            return Ok(());
        }
        // The module filter reads the path from its `tests/it/` segment, so
        // the rule is applied to the whole changed path.
        if integration_path(path).is_some() {
            if let Some(rule) = matching_rule(path) {
                apply_rule(workspace_root, path, rule, recipe)?;
            } else {
                add_integration(recipe, package, integration_module_filter(path)?);
            }
            return Ok(());
        }
        if path.starts_with(&format!("{package}/src/")) {
            if let Some(rule) = matching_rule(path) {
                apply_rule(workspace_root, path, rule, recipe)?;
            } else {
                add_library(recipe, package, source_module_filter(path)?, true);
            }
            return Ok(());
        }
    }
    if integration_path(path).is_some() {
        if let Some(rule) = matching_rule(path) {
            apply_rule(workspace_root, path, rule, recipe)?;
        } else {
            add_integration(
                recipe,
                "filament-spec-language",
                integration_module_filter(path)?,
            );
        }
        return Ok(());
    }
    recipe
        .notes
        .insert(format!("no targeted cargo tests for {path}"));
    Ok(())
}

fn is_test_path(path: &str) -> bool {
    path.split('/').any(|component| component == "tests")
        || path.ends_with("_tests.rs")
        || path.ends_with("/tests.rs")
}

fn is_non_crate_path(path: &str) -> bool {
    path == "README.md"
        || path == "Makefile"
        || path.starts_with("docs/")
        || path.starts_with(".github/")
}

fn is_virtual_path(path: &str) -> bool {
    matches!(path, "Cargo.toml" | "Cargo.lock") || path.ends_with("/Cargo.toml")
}

fn path_exists(workspace_root: &Path, path: &str) -> bool {
    workspace_root.join(path).exists()
}

fn crate_name(path: &str) -> Option<&str> {
    let name = path.split('/').next()?;
    (name.starts_with("fsl-") || name == "xtask").then_some(name)
}

fn test_package(path: &str) -> &str {
    if path.starts_with("tests/") {
        "filament-spec-language"
    } else {
        path.split('/').next().unwrap_or_default()
    }
}

fn integration_path(path: &str) -> Option<&str> {
    path.strip_prefix("tests/it/")
        .or_else(|| path.split_once("/tests/it/").map(|(_, relative)| relative))
}

fn is_crate_root_path(path: &str, package: &str) -> bool {
    ["lib.rs", "main.rs", "mod.rs"]
        .into_iter()
        .any(|file| path == format!("{package}/src/{file}"))
}

fn matching_rule(path: &str) -> Option<&'static Rule> {
    RULES
        .iter()
        .filter(|rule| path.starts_with(rule.prefix))
        .max_by_key(|rule| rule.prefix.len())
}

fn apply_rule(workspace_root: &Path, path: &str, rule: &Rule, recipe: &mut Recipe) -> Result<()> {
    if let (Some(package), Some(lib)) = (rule.package, rule.lib) {
        match lib {
            LibRule::SourceModule => {
                add_library(recipe, package, source_module_filter(path)?, true)
            }
            LibRule::Fixed(filter) => add_target_filter(
                recipe,
                package,
                TestTarget::Library,
                Some(filter.to_owned()),
                false,
                rule.skips,
                false,
                rule.threads,
                false,
            ),
            LibRule::Whole => add_whole_library(recipe, package),
        }
    }
    apply_it_filters(workspace_root, rule.it_filters, Some(path), recipe)?;
    for control in rule.large_input {
        add_large_input(recipe, control);
    }
    if let Some(note) = rule.note {
        recipe.notes.insert(note.to_owned());
    }
    Ok(())
}

fn apply_it_filters(
    workspace_root: &Path,
    filters: &[ItFilter],
    changed_path: Option<&str>,
    recipe: &mut Recipe,
) -> Result<()> {
    for it_filter in filters {
        match it_filter.rule {
            ItRule::Fixed(filter) => add_integration(recipe, it_filter.package, filter),
            ItRule::ChangedModule => {
                let path = changed_path.ok_or_else(|| {
                    Error::MergeGateRecipe("changed-module rule requires a path".to_owned())
                })?;
                add_integration(recipe, it_filter.package, integration_module_filter(path)?);
            }
            ItRule::AllModules(package) => add_all_it_modules(workspace_root, package, recipe)?,
        }
    }
    Ok(())
}

fn integration_root(package: &str) -> String {
    match package {
        "filament-spec-language" => "tests/it/main.rs".to_owned(),
        "fsl-probability-check" => format!("tools/{package}/tests/it/main.rs"),
        _ => format!("{package}/tests/it/main.rs"),
    }
}

fn add_all_it_modules(workspace_root: &Path, package: &str, recipe: &mut Recipe) -> Result<()> {
    let main = integration_root(package);
    let path = workspace_root.join(&main);
    if !path.exists() {
        add_whole_library(recipe, package);
        recipe.notes.insert(format!(
            "{package} has no integration-test target; falling back to its library tests"
        ));
        return Ok(());
    }
    let source = fs::read_to_string(&path).map_err(|error| {
        Error::MergeGateRecipe(format!(
            "cannot read integration test root {}: {error}",
            path.display()
        ))
    })?;
    let syntax = syn::parse_file(&source).map_err(|error| {
        Error::MergeGateRecipe(format!(
            "cannot parse integration test root {}: {error}",
            path.display()
        ))
    })?;
    let base = path.parent().unwrap_or(workspace_root);
    let mut modules = Vec::new();
    for item in &syntax.items {
        let syn::Item::Mod(module) = item else {
            continue;
        };
        let module_name = module.ident.to_string();
        let module_file = module
            .attrs
            .iter()
            .find_map(|attribute| path_attribute(attribute))
            .map(|relative| base.join(relative))
            .unwrap_or_else(|| base.join(format!("{module_name}.rs")));
        let module_file = if module_file.exists() {
            module_file
        } else {
            base.join(&module_name).join("mod.rs")
        };
        let Ok(module_source) = fs::read_to_string(&module_file) else {
            continue;
        };
        let Ok(module_syntax) = syn::parse_file(&module_source) else {
            continue;
        };
        let mut names = Vec::new();
        collect_test_names(&module_syntax.items, &mut Vec::new(), &mut names);
        if !names.is_empty() {
            modules.push(module_name);
        }
    }
    if modules.is_empty() {
        return Err(Error::MergeGateRecipe(format!(
            "no named integration modules found in {}",
            path.display()
        )));
    }
    for module in modules {
        add_integration(recipe, package, module);
    }
    Ok(())
}

#[cfg(test)]
fn top_level_integration_modules(workspace_root: &Path, package: &str) -> Result<Vec<String>> {
    let main = integration_root(package);
    let path = workspace_root.join(&main);
    let source = fs::read_to_string(&path).map_err(|error| {
        Error::MergeGateRecipe(format!(
            "cannot read integration test root {}: {error}",
            path.display()
        ))
    })?;
    let syntax = syn::parse_file(&source).map_err(|error| {
        Error::MergeGateRecipe(format!(
            "cannot parse integration test root {}: {error}",
            path.display()
        ))
    })?;
    let base = path.parent().unwrap_or(workspace_root);
    let mut modules = Vec::new();
    for item in &syntax.items {
        let syn::Item::Mod(module) = item else {
            continue;
        };
        let name = module.ident.to_string();
        let file = module
            .attrs
            .iter()
            .find_map(|attribute| path_attribute(attribute))
            .map(|relative| base.join(relative))
            .unwrap_or_else(|| base.join(format!("{name}.rs")));
        let file = if file.exists() {
            file
        } else {
            base.join(&name).join("mod.rs")
        };
        let Ok(source) = fs::read_to_string(file) else {
            continue;
        };
        let Ok(syntax) = syn::parse_file(&source) else {
            continue;
        };
        let mut names = Vec::new();
        collect_test_names(&syntax.items, &mut Vec::new(), &mut names);
        if !names.is_empty() {
            modules.push(name);
        }
    }
    Ok(modules)
}

fn path_attribute(attribute: &syn::Attribute) -> Option<String> {
    if !attribute.path().is_ident("path") {
        return None;
    }
    match &attribute.meta {
        syn::Meta::NameValue(value) => match &value.value {
            syn::Expr::Lit(syn::ExprLit {
                lit: syn::Lit::Str(path),
                ..
            }) => Some(path.value()),
            _ => None,
        },
        _ => None,
    }
}

fn apply_manifest_rule(recipe: &mut Recipe) {
    for it_filter in EMIT_CONSUMERS {
        if let ItRule::Fixed(filter) = it_filter.rule {
            add_integration(recipe, it_filter.package, filter);
        }
    }
    recipe.notes.insert(
        "manifest or lockfile change: run workspace check and emitter consumers".to_owned(),
    );
}

fn apply_crate_fallback(path: &str, workspace_root: &Path, recipe: &mut Recipe) -> Result<()> {
    let Some(package) = crate_name(path) else {
        recipe
            .notes
            .insert(format!("no crate-level rule for {path}"));
        return Ok(());
    };
    if matches!(
        package,
        "fsl-foundation" | "fsl-source" | "fsl-cst" | "fsl-forms"
    ) {
        add_whole_library(recipe, package);
        return Ok(());
    }
    if matches!(
        package,
        "fsl-analyze" | "fsl-package" | "fsl-replay" | "fsl-semantics" | "fsl-command"
    ) {
        match package {
            "fsl-analyze" => {
                if let Some(rule) = matching_rule("fsl-analyze/src/fallback.rs") {
                    apply_rule(workspace_root, "fsl-analyze/src/fallback.rs", rule, recipe)?;
                }
            }
            "fsl-package" => {
                if let Some(rule) = matching_rule("fsl-package/src/emit/fallback.rs") {
                    apply_rule(
                        workspace_root,
                        "fsl-package/src/emit/fallback.rs",
                        rule,
                        recipe,
                    )?;
                }
            }
            "fsl-replay" => {
                if let Some(rule) = matching_rule("fsl-replay/src/fallback.rs") {
                    apply_rule(workspace_root, "fsl-replay/src/fallback.rs", rule, recipe)?;
                }
                apply_it_filters(workspace_root, &[PROBABILITY_CHECK_MODULES], None, recipe)?;
                add_library(recipe, package, "execute::".to_owned(), true);
            }
            "fsl-semantics" => {
                if let Some(rule) = matching_rule("fsl-semantics/src/fallback.rs") {
                    apply_rule(
                        workspace_root,
                        "fsl-semantics/src/fallback.rs",
                        rule,
                        recipe,
                    )?;
                }
                add_library(recipe, package, "check::".to_owned(), true);
            }
            "fsl-command" => add_all_it_modules(workspace_root, package, recipe)?,
            _ => {}
        }
        return Ok(());
    }
    if package == "xtask" {
        add_whole_library(recipe, "xtask");
    } else if matches!(package, "fsl-eval" | "fsl-semantics") {
        add_library(
            recipe,
            package,
            source_module_filter(path).unwrap_or_else(|_| "check::".to_owned()),
            true,
        );
        for it_filter in TIMED_CONSUMER {
            if let ItRule::Fixed(filter) = it_filter.rule {
                add_integration(recipe, it_filter.package, filter);
            }
        }
        recipe.notes.insert(format!(
            "unknown {package} source path; added timed analyzer consumer control"
        ));
    } else {
        add_whole_library(recipe, package);
    }
    Ok(())
}

fn add_target_filter(
    recipe: &mut Recipe,
    package: &str,
    target: TestTarget,
    filter: Option<String>,
    exact: bool,
    skips: &[&str],
    timed: bool,
    threads: Option<usize>,
    no_match_is_note: bool,
) {
    recipe.selectors.insert(TestSelector {
        package: package.to_owned(),
        target,
        filter,
        exact,
        skip: skips.iter().map(|skip| (*skip).to_owned()).collect(),
        timed,
        threads,
        no_match_is_note,
    });
}

fn add_library(recipe: &mut Recipe, package: &str, filter: String, no_match_is_note: bool) {
    add_target_filter(
        recipe,
        package,
        TestTarget::Library,
        Some(filter),
        false,
        &[],
        false,
        None,
        no_match_is_note,
    );
}

fn add_whole_library(recipe: &mut Recipe, package: &str) {
    add_target_filter(
        recipe,
        package,
        TestTarget::Library,
        None,
        false,
        &[],
        false,
        None,
        false,
    );
}

fn add_integration(recipe: &mut Recipe, package: &str, filter: impl Into<String>) {
    add_target_filter(
        recipe,
        package,
        TestTarget::Integration("it".to_owned()),
        Some(filter.into()),
        false,
        &[],
        false,
        None,
        false,
    );
}

fn add_large_input(recipe: &mut Recipe, input: &LargeInput) {
    add_target_filter(
        recipe,
        input.package,
        TestTarget::Library,
        Some(input.filter.to_owned()),
        input.exact,
        &[],
        true,
        Some(1),
        false,
    );
}

fn compile_targets(recipe: &Recipe) -> BTreeSet<(String, TestTarget)> {
    recipe
        .selectors
        .iter()
        .map(|selector| (selector.package.clone(), selector.target.clone()))
        .collect()
}

fn no_run_command(target: &(String, TestTarget)) -> String {
    let mut command = format!("cargo test --locked -p {}", target.0);
    match &target.1 {
        TestTarget::Library => command.push_str(" --lib"),
        TestTarget::Integration(name) => command.push_str(&format!(" --test {name}")),
    }
    command.push_str(" --no-run");
    command
}

fn require_main_merged(
    workspace_root: &Path,
    runner: &dyn CommandRunner,
) -> Result<(String, String)> {
    let fetch = runner
        .run(
            workspace_root,
            OsStr::new("git"),
            &os_args(&[
                "fetch",
                "origin",
                "+refs/heads/main:refs/remotes/origin/main",
            ]),
        )
        .map_err(|error| Error::MergeGateRecipe(format!("cannot fetch origin/main: {error}")))?;
    if !fetch.success {
        return Err(Error::MergeGateRecipe(format!(
            "cannot fetch origin/main: {}",
            String::from_utf8_lossy(&fetch.stderr)
        )));
    }
    let head = git_revision(workspace_root, runner, "HEAD")?;
    let main = git_revision(workspace_root, runner, "origin/main")?;
    let ancestor = runner
        .run(
            workspace_root,
            OsStr::new("git"),
            &os_args(&["merge-base", "--is-ancestor", "origin/main", "HEAD"]),
        )
        .map_err(|error| {
            Error::MergeGateRecipe(format!("cannot check origin/main merge: {error}"))
        })?;
    if !ancestor.success {
        return Err(Error::MergeGateRecipe(
            "merge origin/main into the PR head before generating the merge recipe".to_owned(),
        ));
    }
    Ok((head, main))
}

fn git_revision(
    workspace_root: &Path,
    runner: &dyn CommandRunner,
    revision: &str,
) -> Result<String> {
    let output = runner
        .run(
            workspace_root,
            OsStr::new("git"),
            &os_args(&["rev-parse", revision]),
        )
        .map_err(|error| Error::MergeGateRecipe(format!("cannot read {revision}: {error}")))?;
    if !output.success {
        return Err(Error::MergeGateRecipe(format!(
            "cannot read {revision}: {}",
            String::from_utf8_lossy(&output.stderr)
        )));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

fn os_args(arguments: &[&str]) -> Vec<OsString> {
    arguments.iter().map(OsString::from).collect()
}

fn ignore_attribute_changed(
    workspace_root: &Path,
    path: &str,
    runner: &dyn CommandRunner,
) -> Result<bool> {
    let mut changed = false;
    for args in [
        vec![
            "diff",
            "--no-ext-diff",
            "--unified=0",
            "origin/main...HEAD",
            "--",
            path,
        ],
        vec![
            "diff",
            "--no-ext-diff",
            "--unified=0",
            "--cached",
            "--",
            path,
        ],
        vec!["diff", "--no-ext-diff", "--unified=0", "--", path],
    ] {
        let output = runner
            .run(
                workspace_root,
                OsStr::new("git"),
                &args
                    .iter()
                    .map(|arg| OsString::from(*arg))
                    .collect::<Vec<_>>(),
            )
            .map_err(|error| {
                Error::MergeGateRecipe(format!("cannot inspect diff for {path:?}: {error}"))
            })?;
        if !output.success {
            return Err(Error::MergeGateRecipe(format!(
                "git diff failed for {path:?}: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }
        changed |= String::from_utf8_lossy(&output.stdout).lines().any(|line| {
            (line.starts_with('+') || line.starts_with('-'))
                && !line.starts_with("+++")
                && !line.starts_with("---")
                && line.contains("#[ignore")
        });
    }
    Ok(changed)
}

fn source_module_filter(path: &str) -> Result<String> {
    module_filter(path, "/src/")
}

fn integration_module_filter(path: &str) -> Result<String> {
    if let Some(relative) = path.strip_prefix("tests/it/") {
        return module_name_filter(relative, path)
            .map(|filter| filter.trim_end_matches("::").to_owned());
    }
    module_filter(path, "/tests/it/").map(|filter| filter.trim_end_matches("::").to_owned())
}

fn module_filter(path: &str, marker: &str) -> Result<String> {
    let (_, relative) = path
        .split_once(marker)
        .ok_or_else(|| Error::MergeGateRecipe(format!("no module path in {path:?}")))?;
    module_name_filter(relative, path)
}

fn module_name_filter(relative: &str, path: &str) -> Result<String> {
    let mut modules = relative.split('/').map(str::to_owned).collect::<Vec<_>>();
    let Some(file) = modules.pop() else {
        return Err(Error::MergeGateRecipe(format!(
            "no module file in {path:?}"
        )));
    };
    if file == "mod.rs" {
        if modules.is_empty() {
            return Err(Error::MergeGateRecipe(format!(
                "crate-root module {path:?} needs an explicit named test area"
            )));
        }
    } else if let Some(stem) = file.strip_suffix(".rs") {
        if stem == "lib" || stem == "main" {
            return Err(Error::MergeGateRecipe(format!(
                "crate-root module {path:?} needs an explicit named test area"
            )));
        }
        modules.push(stem.to_owned());
    } else {
        return Err(Error::MergeGateRecipe(format!(
            "changed path {path:?} is not a Rust module"
        )));
    }
    if modules.is_empty() {
        return Err(Error::MergeGateRecipe(format!(
            "changed path {path:?} does not identify a test module"
        )));
    }
    Ok(format!("{}::", modules.join("::")))
}

fn collect_test_names(items: &[syn::Item], modules: &mut Vec<String>, names: &mut Vec<String>) {
    for item in items {
        match item {
            syn::Item::Fn(function)
                if function.attrs.iter().any(|attribute| {
                    attribute
                        .path()
                        .segments
                        .last()
                        .is_some_and(|segment| segment.ident == "test")
                }) =>
            {
                let mut path = modules.clone();
                path.push(function.sig.ident.to_string());
                names.push(path.join("::"));
            }
            syn::Item::Mod(module) => {
                if let Some((_, nested)) = &module.content {
                    modules.push(module.ident.to_string());
                    collect_test_names(nested, modules, names);
                    modules.pop();
                }
            }
            _ => {}
        }
    }
}

fn preflight(
    workspace_root: &Path,
    selector: &TestSelector,
    runner: &dyn CommandRunner,
) -> Result<usize> {
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| OsString::from("cargo"));
    let list_arguments = selector
        .command(true)
        .into_iter()
        .map(OsString::from)
        .collect::<Vec<_>>();
    let output = runner
        .run(workspace_root, &cargo, &list_arguments)
        .map_err(|error| {
            Error::MergeGateRecipe(format!(
                "cannot list tests for {:?}: {error}",
                selector.display()
            ))
        })?;
    if !output.success {
        return Err(Error::MergeGateRecipe(format!(
            "test-filter preflight failed for {:?}: {}{}",
            selector.display(),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )));
    }
    let mut ignored_arguments = list_arguments;
    ignored_arguments.insert(ignored_arguments.len() - 1, OsString::from("--ignored"));
    let ignored = runner
        .run(workspace_root, &cargo, &ignored_arguments)
        .map_err(|error| {
            Error::MergeGateRecipe(format!(
                "cannot list ignored tests for {:?}: {error}",
                selector.display()
            ))
        })?;
    if !ignored.success {
        return Err(Error::MergeGateRecipe(format!(
            "ignored-test preflight failed for {:?}: {}{}",
            selector.display(),
            String::from_utf8_lossy(&ignored.stdout),
            String::from_utf8_lossy(&ignored.stderr)
        )));
    }
    let selected = listed_test_names(&output.stdout);
    let ignored = listed_test_names(&ignored.stdout);
    Ok(selected.difference(&ignored).count())
}

fn listed_test_names(output: &[u8]) -> BTreeSet<String> {
    String::from_utf8_lossy(output)
        .lines()
        .filter_map(|line| line.strip_suffix(": test"))
        .map(str::to_owned)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    fn workspace_root() -> &'static Path {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("workspace root")
    }

    struct FakeRunner {
        calls: RefCell<Vec<(String, Vec<String>)>>,
        no_tests: bool,
        ignored_only: bool,
        changed_ignore: bool,
        fetch_fails: bool,
        merge_fails: bool,
    }

    impl FakeRunner {
        fn new() -> Self {
            Self {
                calls: RefCell::new(Vec::new()),
                no_tests: false,
                ignored_only: false,
                changed_ignore: false,
                fetch_fails: false,
                merge_fails: false,
            }
        }
    }

    impl CommandRunner for FakeRunner {
        fn run(
            &self,
            _cwd: &Path,
            program: &OsStr,
            arguments: &[OsString],
        ) -> std::io::Result<ProcessOutput> {
            let args = arguments
                .iter()
                .map(|arg| arg.to_string_lossy().into_owned())
                .collect::<Vec<_>>();
            let is_git = program == OsStr::new("git");
            self.calls
                .borrow_mut()
                .push((program.to_string_lossy().into_owned(), args.clone()));
            let fail =
                (is_git && args.first().is_some_and(|arg| arg == "fetch") && self.fetch_fails)
                    || (is_git
                        && args.first().is_some_and(|arg| arg == "merge-base")
                        && self.merge_fails);
            let text = if is_git && args.first().is_some_and(|arg| arg == "rev-parse") {
                if args.last().is_some_and(|arg| arg == "HEAD") {
                    "head-sha\n"
                } else {
                    "main-sha\n"
                }
            } else if is_git && args.first().is_some_and(|arg| arg == "diff") && self.changed_ignore
            {
                "+#[ignore]\n"
            } else if !is_git && args.contains(&"--list".to_owned()) && !self.no_tests {
                if args.contains(&"--ignored".to_owned()) && self.ignored_only {
                    "sample_case: test\n"
                } else if args.contains(&"--ignored".to_owned()) {
                    ""
                } else {
                    "sample_case: test\n"
                }
            } else {
                ""
            };
            Ok(ProcessOutput {
                success: !fail,
                stdout: text.as_bytes().to_vec(),
                stderr: if fail {
                    b"injected failure".to_vec()
                } else {
                    Vec::new()
                },
            })
        }
    }

    fn plan(root: &Path, paths: &[&str]) -> Recipe {
        plan_paths(
            root,
            &paths
                .iter()
                .map(|path| (*path).to_owned())
                .collect::<Vec<_>>(),
        )
        .expect("valid paths")
    }

    fn selector(package: &str, filter: &str) -> TestSelector {
        TestSelector {
            package: package.to_owned(),
            target: TestTarget::Integration("it".to_owned()),
            filter: Some(filter.to_owned()),
            exact: false,
            skip: Vec::new(),
            timed: false,
            threads: None,
            no_match_is_note: false,
        }
    }

    fn probability_check_modules() -> Vec<String> {
        let modules = top_level_integration_modules(workspace_root(), "fsl-probability-check")
            .expect("probability-check integration modules");
        assert!(!modules.is_empty());
        modules
    }

    fn runs_modules(recipe: &Recipe, modules: &[String]) -> bool {
        modules
            .iter()
            .all(|module| recipe.selectors.contains(&selector("fsl-probability-check", module)))
    }

    #[test]
    fn exact_probabilistic_and_certificate_paths_run_every_probability_check_module() {
        let modules = probability_check_modules();
        for path in [
            "fsl-analyze/src/analyze/exact.rs",
            "fsl-analyze/src/analyze/exact/measure.rs",
            "fsl-analyze/src/exact_probabilistic.rs",
            "fsl-analyze/src/exact_probabilistic/build.rs",
            "fsl-replay/src/certificate.rs",
            "fsl-replay/src/probability_certificate.rs",
            "fsl-replay/src/exact_probabilistic.rs",
            "fsl-replay/src/spine.rs",
            "fsl-semantics/src/check/probabilistic.rs",
            "fsl-semantics/src/check/probabilistic/model.rs",
            "fsl-semantics/src/check/mod.rs",
        ] {
            let recipe = plan(workspace_root(), &[path]);
            assert!(
                runs_modules(&recipe, &modules),
                "{path} must run every fsl-probability-check module"
            );
        }
    }

    #[test]
    fn replay_surface_and_certificate_dependencies_run_every_probability_check_module() {
        let modules = probability_check_modules();
        for path in [
            "fsl-replay/src/lib.rs",
            "fsl-replay/Cargo.toml",
            "fsl-replay/src/product_action.rs",
            "fsl-replay/src/identity.rs",
            "fsl-replay/src/limits.rs",
            "fsl-replay/src/proof_result.rs",
            "fsl-replay/src/outcome.rs",
            "fsl-replay/src/request.rs",
            "fsl-replay/src/statistical.rs",
            "fsl-replay/src/model_check/limits.rs",
        ] {
            let recipe = plan(workspace_root(), &[path]);
            assert!(
                runs_modules(&recipe, &modules),
                "{path} must run every fsl-probability-check module"
            );
        }
    }

    #[test]
    fn unrelated_replay_paths_do_not_run_the_probability_check() {
        let recipe = plan(workspace_root(), &["fsl-replay/src/refinement.rs"]);
        assert!(
            !recipe
                .selectors
                .iter()
                .any(|item| item.package == "fsl-probability-check")
        );
    }

    #[test]
    fn probability_check_tool_changes_route_to_the_tool_package() {
        let modules = probability_check_modules();
        let module = plan(
            workspace_root(),
            &["tools/fsl-probability-check/tests/it/cyclic_mdp.rs"],
        );
        assert_eq!(
            module
                .selectors
                .iter()
                .filter(|item| item.package == "fsl-probability-check")
                .collect::<Vec<_>>(),
            vec![&selector("fsl-probability-check", "cyclic_mdp")]
        );
        assert!(
            !module
                .selectors
                .iter()
                .any(|item| item.package == "filament-spec-language")
        );
        for path in [
            "tools/fsl-probability-check/tests/it/main.rs",
            "tools/fsl-probability-check/src/main.rs",
        ] {
            let recipe = plan(workspace_root(), &[path]);
            assert!(runs_modules(&recipe, &modules), "{path}");
            assert!(
                !recipe
                    .notes
                    .iter()
                    .any(|note| note.contains("no targeted cargo tests")),
                "{path}"
            );
        }
    }

    #[test]
    fn the_file_set_that_broke_the_checker_routes_to_it() {
        let modules = probability_check_modules();
        let recipe = plan(
            workspace_root(),
            &[
                "tools/fsl-probability-check/tests/it/bounded.rs",
                "fsl-semantics/src/check/mod.rs",
                "fsl-semantics/src/check/probabilistic/model.rs",
            ],
        );
        assert!(runs_modules(&recipe, &modules));
    }

    #[test]
    fn exact_and_shared_analyze_paths_include_consumer_integration_filters() {
        for path in [
            "fsl-analyze/src/analyze/exact.rs",
            "fsl-analyze/src/analyze/exact/nested.rs",
        ] {
            let recipe = plan(workspace_root(), &[path]);
            assert!(
                recipe
                    .selectors
                    .contains(&selector("fsl-analyze", "probabilistic"))
            );
            assert!(
                recipe
                    .selectors
                    .contains(&selector("fsl-analyze", "timed_"))
            );
        }
        let recipe = plan(workspace_root(), &["fsl-analyze/src/provider.rs"]);
        for filter in ["monitor_", "timed_", "hyper", "probabilistic"] {
            assert!(recipe.selectors.contains(&selector("fsl-analyze", filter)));
        }
        let zone = plan(workspace_root(), &["fsl-analyze/src/zone_check/divergence.rs"]);
        for filter in ["timed_", "timed_within_200", "timed_within_201"] {
            assert!(zone.selectors.contains(&selector("fsl-analyze", filter)));
        }
    }

    #[test]
    fn request_and_analyze_roots_run_the_source_and_package_parity_modules() {
        for path in ["fsl-analyze/src/analyze/request.rs", "fsl-analyze/src/analyze.rs"] {
            let recipe = plan(workspace_root(), &[path]);
            for filter in [
                "analyze_package_selection",
                "analyze_request_wire",
                "analyze_package_provider",
                "monitor_",
                "probabilistic",
            ] {
                assert!(
                    recipe.selectors.contains(&selector("fsl-analyze", filter)),
                    "{path} must run {filter}"
                );
            }
        }
        let other = plan(workspace_root(), &["fsl-analyze/src/provider.rs"]);
        assert!(
            !other
                .selectors
                .contains(&selector("fsl-analyze", "analyze_request_wire"))
        );
    }

    #[test]
    fn document_settlement_and_package_route_run_the_composition_parity_module() {
        for path in [
            "fsl-analyze/src/analyze/document.rs",
            "fsl-analyze/src/analyze/document/package_run.rs",
            "fsl-analyze/src/analyze/document/composition_item.rs",
        ] {
            let recipe = plan(workspace_root(), &[path]);
            for filter in [
                "analyze_composition_cli",
                "analyze_package_selection",
                "analyze_request_wire",
                "analyze_package_provider",
                "monitor_",
                "timed_",
                "hyper",
                "probabilistic",
            ] {
                assert!(
                    recipe.selectors.contains(&selector("fsl-analyze", filter)),
                    "{path} must run {filter}"
                );
            }
        }
    }

    #[test]
    fn a_changed_integration_module_runs_that_module() {
        let recipe = plan(
            workspace_root(),
            &[
                "fsl-analyze/tests/it/analyze_composition_cli.rs",
                "tests/it/bare_package.rs",
            ],
        );
        assert!(recipe
            .selectors
            .contains(&selector("fsl-analyze", "analyze_composition_cli")));
        assert!(recipe
            .selectors
            .contains(&selector("filament-spec-language", "bare_package")));
    }

    #[test]
    fn emitter_is_serial_skips_heavy_tests_and_splits_them_as_timed_controls() {
        let recipe = plan(workspace_root(), &["fsl-package/src/emit.rs"]);
        let emit = recipe
            .selectors
            .iter()
            .find(|item| item.package == "fsl-package" && item.filter.as_deref() == Some("emit"))
            .expect("emit filter");
        assert_eq!(emit.threads, Some(1));
        assert_eq!(
            emit.skip,
            EMIT_SKIP
                .iter()
                .map(|item| item.to_string())
                .collect::<Vec<_>>()
        );
        assert!(
            recipe
                .selectors
                .iter()
                .any(|item| item.package == "fsl-replay"
                    && item.filter.as_deref()
                        == Some(
                            "tc_736_a_100000_term_source_and_list_replay_to_the_proving_verdict"
                        )
                    && item.timed)
        );
        assert!(
            recipe
                .selectors
                .iter()
                .filter(|item| item.timed)
                .all(|item| item.threads == Some(1))
        );
    }

    #[test]
    fn command_paths_include_local_module_and_forms_spans_runs_whole_library() {
        let command = plan(workspace_root(), &["fsl-command/src/command/items.rs"]);
        assert!(command.selectors.iter().any(|item| {
            item.package == "fsl-command"
                && item.target == TestTarget::Library
                && item.filter.as_deref() == Some("command::items::")
                && item.no_match_is_note
        }));
        assert!(command.selectors.iter().any(|item| {
            item.package == "fsl-command" && item.filter.as_deref() == Some("run_requirement_items")
        }));
        let spans = plan(workspace_root(), &["fsl-forms/src/spans.rs"]);
        assert!(spans.selectors.iter().any(|item| {
            item.package == "fsl-forms"
                && item.target == TestTarget::Library
                && item.filter.is_none()
        }));
        assert!(
            spans
                .selectors
                .iter()
                .any(|item| item.filter.as_deref() == Some(LARGE_EMIT_FILTER))
        );
    }

    #[test]
    fn main_roots_are_checked_before_test_prefixes_and_coverage_is_table_driven() {
        for (package, root) in [
            ("fsl-analyze", "fsl-analyze/tests/it/main.rs"),
            ("fsl-command", "fsl-command/tests/it/main.rs"),
            ("filament-spec-language", "tests/it/main.rs"),
        ] {
            assert!(matching_rule(root).is_some_and(|rule| rule.prefix.ends_with("main.rs")));
            assert!(plan(workspace_root(), &[root]).selectors.len() > 1);
            let modules =
                top_level_integration_modules(workspace_root(), package).expect("module list");
            assert!(!modules.is_empty());
            for module in modules {
                let path = if package == "filament-spec-language" {
                    format!("tests/it/{module}.rs")
                } else {
                    format!("{package}/tests/it/{module}.rs")
                };
                assert!(
                    matching_rule(&path).is_some(),
                    "unrouted module path {path}"
                );
            }
        }
    }

    #[test]
    fn shared_run_fetches_main_and_uses_injected_preflight() {
        let runner = FakeRunner::new();
        let result = run_with_runner(
            workspace_root(),
            &[OsString::from("fsl-analyze/src/analyze/exact.rs")],
            &runner,
        )
        .expect("recipe");
        assert!(result.starts_with("# PR head head-sha; origin/main main-sha\n"));
        let calls = runner.calls.borrow();
        assert_eq!(calls[0].1[0], "fetch");
        assert!(
            calls
                .iter()
                .any(|(_, args)| args.first().is_some_and(|arg| arg == "merge-base"))
        );
        let compile = result.find("--no-run").expect("compile first");
        let tests = result
            .find("\ncargo test --locked -p fsl-analyze --test it -- probabilistic")
            .expect("filtered test after compile");
        assert!(compile < tests);
    }

    #[test]
    fn preflight_rejects_zero_matches_and_ignored_only_matches() {
        let mut runner = FakeRunner::new();
        runner.no_tests = true;
        assert_eq!(
            preflight(
                workspace_root(),
                &selector("fsl-analyze", "explicit"),
                &runner
            )
            .expect("list"),
            0
        );
        runner.no_tests = false;
        runner.ignored_only = true;
        assert_eq!(
            preflight(
                workspace_root(),
                &selector("fsl-analyze", "explicit"),
                &runner
            )
            .expect("list"),
            0
        );
    }

    #[test]
    fn run_fails_closed_when_an_explicit_table_filter_has_no_nonignored_match() {
        let runner = FakeRunner {
            no_tests: true,
            ..FakeRunner::new()
        };
        assert!(
            run_with_runner(
                workspace_root(),
                &[OsString::from("fsl-analyze/src/analyze/exact.rs")],
                &runner
            )
            .is_err()
        );
    }

    #[test]
    fn empty_unknown_manifest_fixture_deleted_and_root_paths_have_fallbacks() {
        assert!(
            plan_paths(workspace_root(), &[])
                .expect("empty planning")
                .selectors
                .is_empty()
        );
        assert!(run_with_runner(workspace_root(), &[], &FakeRunner::new()).is_err());
        assert!(
            plan(workspace_root(), &["untracked/unknown.dat"])
                .notes
                .iter()
                .any(|note| note.contains("unknown.dat"))
        );
        assert!(
            plan(workspace_root(), &["Cargo.lock"])
                .selectors
                .iter()
                .any(|item| item.filter.as_deref() == Some("monitor_"))
        );
        assert!(
            plan(workspace_root(), &["fsl-eval/src/new_unknown.rs"])
                .notes
                .iter()
                .any(|note| note.contains("unknown fsl-eval"))
        );
        assert!(
            plan(workspace_root(), &["fsl-semantics/src/new_unknown.rs"])
                .notes
                .iter()
                .any(|note| note.contains("unknown fsl-semantics"))
        );
        assert!(
            plan(workspace_root(), &["xtask/src/new_task.rs"])
                .selectors
                .iter()
                .any(|item| item.package == "xtask" && item.filter.is_none())
        );
        assert!(
            plan(
                workspace_root(),
                &["fsl-analyze/tests/fixtures/sample.json"]
            )
            .selectors
            .len()
                > 1
        );
        assert!(
            plan(workspace_root(), &["fsl-analyze/src/deleted.rs"])
                .notes
                .iter()
                .any(|note| note.contains("absent"))
        );
        assert!(
            plan(workspace_root(), &["fsl-analyze/src/lib.rs"])
                .selectors
                .iter()
                .any(|item| item.filter.as_deref() == Some("monitor_"))
        );
        assert!(
            plan(workspace_root(), &["src/lib.rs"])
                .selectors
                .iter()
                .any(|item| item.package == "filament-spec-language" && item.filter.is_none())
        );
    }

    #[test]
    fn non_crate_paths_and_this_pr_file_list_produce_recipe() {
        let recipe = plan(
            workspace_root(),
            &[
                "README.md",
                "docs/merge-gate-recipe.md",
                "xtask/src/merge_gate_recipe.rs",
            ],
        );
        assert!(recipe.notes.iter().any(|note| note.contains("README.md")));
        assert!(
            recipe
                .selectors
                .iter()
                .any(|item| item.package == "xtask" && item.filter.is_none())
        );
    }

    #[test]
    fn simulation_changes_run_the_protocol_state_key_modules() {
        let recipe = plan(workspace_root(), &["fsl-eval/src/simulation/protocol/reach.rs"]);
        for filter in ["protocol_bounded_lasso", "race_freedom", "native_correlation"] {
            assert!(
                recipe
                    .selectors
                    .iter()
                    .any(|item| item.package == "fsl-analyze"
                        && item.filter.as_deref() == Some(filter)),
                "{filter}"
            );
        }
    }

    #[test]
    fn semantically_related_rules_and_dedup_are_explicit_and_ordered() {
        let recipe = plan(
            workspace_root(),
            &[
                "fsl-semantics/src/check/abstraction.rs",
                "fsl-eval/src/value/expression/bare.rs",
                "fsl-semantics/src/check/abstraction.rs",
            ],
        );
        for filter in ["abstraction_relation_inert", "bare_", "timed_"] {
            assert!(
                recipe
                    .selectors
                    .iter()
                    .any(|item| item.filter.as_deref() == Some(filter))
            );
        }
        assert_eq!(recipe.selectors.len(), 5);
    }

    #[test]
    fn ignore_changes_force_xtask_library_and_fetch_failures_stop_even_spec_only() {
        let runner = FakeRunner {
            changed_ignore: true,
            ..FakeRunner::new()
        };
        let output = run_with_runner(
            workspace_root(),
            &[OsString::from("tests/it/main.rs")],
            &runner,
        )
        .expect("recipe");
        assert!(output.contains("cargo test --locked -p xtask --lib --no-run"));
        let runner = FakeRunner {
            fetch_fails: true,
            ..FakeRunner::new()
        };
        assert!(
            run_with_runner(workspace_root(), &[OsString::from("spec/new.md")], &runner).is_err()
        );
        let runner = FakeRunner {
            merge_fails: true,
            ..FakeRunner::new()
        };
        assert!(
            run_with_runner(workspace_root(), &[OsString::from("spec/new.md")], &runner).is_err()
        );
    }

    #[test]
    fn non_crate_changes_emit_only_final_workspace_check() {
        let runner = FakeRunner::new();
        let output = run_with_runner(workspace_root(), &[OsString::from("README.md")], &runner)
            .expect("recipe");
        assert_eq!(
            output
                .lines()
                .filter(|line| line.starts_with("cargo "))
                .collect::<Vec<_>>(),
            ["cargo check --locked --workspace --all-targets"]
        );
    }

    #[test]
    fn unfiltered_library_preflight_passes_test_options_after_separator() {
        let whole = TestSelector {
            package: "xtask".to_owned(),
            target: TestTarget::Library,
            filter: None,
            exact: false,
            skip: Vec::new(),
            timed: false,
            threads: None,
            no_match_is_note: false,
        };
        assert_eq!(
            whole.command(true),
            ["test", "--locked", "-p", "xtask", "--lib", "--", "--list"]
        );
    }
}
