// SPDX-License-Identifier: AGPL-3.0-or-later
//! Path-based, name-filtered merge-gate recipes for FSL changes.

use std::collections::{BTreeMap, BTreeSet};
use std::ffi::{OsStr, OsString};
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex, PoisonError};

use ix_trace_rs::implements;

use crate::error::{Error, Result};

const LARGE_EMIT_FILTER: &str =
    "emit::tests::protocol_locations::a_function_body_100000_deep_projects_on_a_small_stack";
const EMIT_SKIP: [&str; 2] = [
    "100000",
    "every_nested_form_emits_in_the_stratified_grammar_at_2_and_1000_levels",
];
const ANALYZE_IT_ROOTS: &[(&str, &str)] = &[
    ("it", "fsl-analyze/tests/it/main.rs"),
    (
        "it_monitor_protocol",
        "fsl-analyze/tests/it/monitor_protocol.rs",
    ),
    ("it_probabilistic", "fsl-analyze/tests/it/probabilistic.rs"),
    (
        "it_model_check",
        "fsl-analyze/tests/it/model_check_binary.rs",
    ),
    ("it_refinement", "fsl-analyze/tests/it/refinement.rs"),
    (
        "it_refinement_cx1",
        "fsl-analyze/tests/it_refinement_cx1.rs",
    ),
];
const USAGE: &str =
    "usage: cargo xtask merge-gate-recipe --workspace <path> [--preflight] <changed-path>...";
const BUILD_INPUT_PATHS: &[&str] = &[
    "Cargo.toml",
    "Cargo.lock",
    "xtask/Cargo.toml",
    "xtask/build.rs",
    "xtask/src",
    ".cargo",
    "rust-toolchain.toml",
];

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
    NamedModules,
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
const FOUNDATION_CONSUMERS: &[ItFilter] = &[
    ItFilter {
        package: "fsl-cst",
        rule: ItRule::Fixed("cst_identity"),
    },
    ItFilter {
        package: "fsl-source",
        rule: ItRule::Fixed("quire_source"),
    },
    ItFilter {
        package: "fsl-forms",
        rule: ItRule::Fixed("source_grammar_vectors"),
    },
    ItFilter {
        package: "fsl-semantics",
        rule: ItRule::Fixed("identity_golden_vectors"),
    },
    ItFilter {
        package: "fsl-package",
        rule: ItRule::Fixed("source_object_identity_rows_public"),
    },
    ItFilter {
        package: "fsl-command",
        rule: ItRule::Fixed("check_source_identity"),
    },
    ItFilter {
        package: "fsl-replay",
        rule: ItRule::Fixed("source_diagnostic_outcome"),
    },
    ItFilter {
        package: "filament-spec-language",
        rule: ItRule::Fixed("layer_crate_reexports"),
    },
];
const SOURCE_CONSUMERS: &[ItFilter] = &[
    ItFilter {
        package: "fsl-replay",
        rule: ItRule::Fixed("source_diagnostic_outcome"),
    },
    ItFilter {
        package: "fsl-command",
        rule: ItRule::Fixed("extraction_request_refusals"),
    },
    ItFilter {
        package: "filament-spec-language",
        rule: ItRule::Fixed("formal_source"),
    },
];
const CST_CONSUMERS: &[ItFilter] = &[
    ItFilter {
        package: "fsl-forms",
        rule: ItRule::Fixed("source_grammar_vectors"),
    },
    ItFilter {
        package: "fsl-package",
        rule: ItRule::Fixed("checked_ir_surface"),
    },
    ItFilter {
        package: "fsl-replay",
        rule: ItRule::Fixed("compile_package_facade"),
    },
    ItFilter {
        package: "fsl-replay",
        rule: ItRule::Fixed("compile_text_literals"),
    },
    ItFilter {
        package: "fsl-cst",
        rule: ItRule::Fixed("complete_cst"),
    },
    ItFilter {
        package: "fsl-cst",
        rule: ItRule::Fixed("complete_grammar"),
    },
];
const FORMS_CONSUMERS: &[ItFilter] = &[
    ItFilter {
        package: "fsl-semantics",
        rule: ItRule::Fixed("type_decisions"),
    },
    ItFilter {
        package: "fsl-eval",
        rule: ItRule::Fixed("union_value_profile"),
    },
    ItFilter {
        package: "fsl-analyze",
        rule: ItRule::Fixed("package_forms"),
    },
    ItFilter {
        package: "fsl-command",
        rule: ItRule::Fixed("fr311_s3_forms"),
    },
    ItFilter {
        package: "filament-spec-language",
        rule: ItRule::Fixed("forms_package_contracts"),
    },
];
const PACKAGE_CONSUMERS: &[ItFilter] = &[
    ItFilter {
        package: "fsl-eval",
        rule: ItRule::Fixed("checked_route_owner"),
    },
    ItFilter {
        package: "fsl-replay",
        rule: ItRule::Fixed("emit_admit_agreement"),
    },
    ItFilter {
        package: "fsl-analyze",
        rule: ItRule::Fixed("analyze_package_selection"),
    },
    ItFilter {
        package: "fsl-command",
        rule: ItRule::Fixed("run_requirement_items"),
    },
    ItFilter {
        package: "fsl-probability-check",
        rule: ItRule::Fixed("roundtrip"),
    },
    ItFilter {
        package: "filament-spec-language",
        rule: ItRule::Fixed("run_items"),
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
        it_filters: PACKAGE_CONSUMERS,
        large_input: PROJECTION_LARGE_INPUTS,
        note: None,
        skips: &[],
        threads: None,
    },
    Rule {
        prefix: "fsl-package/src/checked_v2.rs",
        package: Some("fsl-package"),
        lib: Some(LibRule::SourceModule),
        it_filters: PACKAGE_CONSUMERS,
        large_input: PROJECTION_LARGE_INPUTS,
        note: None,
        skips: &[],
        threads: None,
    },
    Rule {
        prefix: "fsl-forms/src/spans.rs",
        package: Some("fsl-forms"),
        lib: Some(LibRule::SourceModule),
        it_filters: FORMS_CONSUMERS,
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
        lib: Some(LibRule::NamedModules),
        it_filters: &[],
        large_input: &[],
        note: None,
        skips: &[],
        threads: None,
    },
    Rule {
        prefix: "fsl-foundation/src/",
        package: Some("fsl-foundation"),
        lib: Some(LibRule::SourceModule),
        it_filters: FOUNDATION_CONSUMERS,
        large_input: &[],
        note: None,
        skips: &[],
        threads: None,
    },
    Rule {
        prefix: "fsl-source/src/",
        package: Some("fsl-source"),
        lib: Some(LibRule::SourceModule),
        it_filters: SOURCE_CONSUMERS,
        large_input: &[],
        note: None,
        skips: &[],
        threads: None,
    },
    Rule {
        prefix: "fsl-cst/src/",
        package: Some("fsl-cst"),
        lib: Some(LibRule::SourceModule),
        it_filters: CST_CONSUMERS,
        large_input: &[],
        note: None,
        skips: &[],
        threads: None,
    },
    Rule {
        prefix: "fsl-forms/src/",
        package: Some("fsl-forms"),
        lib: Some(LibRule::SourceModule),
        it_filters: FORMS_CONSUMERS,
        large_input: &[],
        note: None,
        skips: &[],
        threads: None,
    },
    Rule {
        prefix: "fsl-package/src/",
        package: Some("fsl-package"),
        lib: Some(LibRule::SourceModule),
        it_filters: PACKAGE_CONSUMERS,
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
        if self.package == "fsl-semantics" && matches!(&self.target, TestTarget::Integration(_)) {
            arguments.extend(["--features".to_owned(), "test-support".to_owned()]);
        }
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
/// By default this only prints the recipe. With `--preflight`, each filter is
/// checked with normal and ignored filtered `--list` invocations, and a filter
/// that selects no non-ignored tests fails closed.
#[implements("NFR-142-AC-1")]
pub fn run(workspace_root: &Path, operands: &[OsString]) -> Result<String> {
    let (selected_root, selected_operands) = select_workspace(workspace_root, operands)?;
    let (head, main) = ensure_compatible(workspace_root, &selected_root, &SystemCommandRunner)?;
    run_with_verified_main(
        &selected_root,
        &selected_operands,
        &SystemCommandRunner,
        head,
        main,
    )
}

#[implements("NFR-142-AC-1")]
fn select_workspace(_tool_root: &Path, operands: &[OsString]) -> Result<(PathBuf, Vec<OsString>)> {
    let mut selected = None;
    let mut remaining = Vec::with_capacity(operands.len());
    let mut iter = operands.iter();
    while let Some(operand) = iter.next() {
        if operand == "--workspace" {
            if selected.is_some() {
                return Err(Error::Usage(USAGE));
            }
            selected = Some(PathBuf::from(iter.next().ok_or(Error::Usage(USAGE))?));
        } else {
            remaining.push(operand.clone());
        }
    }
    let selected = selected.ok_or(Error::Usage(USAGE))?;
    let selected = selected
        .canonicalize()
        .map_err(|error| Error::MergeGateRecipe(format!("cannot select workspace: {error}")))?;
    if !selected.join("Cargo.toml").is_file() || !selected.join("xtask/Cargo.toml").is_file() {
        return Err(Error::MergeGateRecipe(format!(
            "selected workspace {} lacks the workspace and xtask manifests",
            selected.display()
        )));
    }
    Ok((selected, remaining))
}

#[implements("NFR-142-AC-2", "NFR-142-AC-4")]
fn ensure_compatible(
    tool_root: &Path,
    target_root: &Path,
    runner: &dyn CommandRunner,
) -> Result<(String, String)> {
    let same_checkout = tool_root
        .canonicalize()
        .ok()
        .zip(target_root.canonicalize().ok())
        .is_some_and(|(tool, target)| tool == target);
    if same_checkout {
        let verified = require_main_merged(target_root, runner)?;
        let mut arguments = vec![
            OsString::from("status"),
            OsString::from("--porcelain"),
            OsString::from("--"),
        ];
        arguments.extend(BUILD_INPUT_PATHS.iter().map(OsString::from));
        let dirty = runner
            .run(target_root, OsStr::new("git"), &arguments)
            .map_err(|error| {
                Error::MergeGateRecipe(format!("cannot inspect selected workspace: {error}"))
            })?;
        if !dirty.success {
            return Err(Error::MergeGateRecipe(format!(
                "cannot inspect selected workspace: {}",
                String::from_utf8_lossy(&dirty.stderr)
            )));
        }
        if !dirty.stdout.is_empty() {
            return Err(Error::MergeGateRecipe(format!(
                "selected workspace {} has uncommitted xtask build inputs",
                target_root.display()
            )));
        }
        return Ok(verified);
    }
    require_tool_on_main_history(tool_root, runner)?;
    let verified = require_main_merged(target_root, runner)?;
    let dirty = runner
        .run(
            tool_root,
            OsStr::new("git"),
            &os_args(&["status", "--porcelain"]),
        )
        .map_err(|error| {
            Error::MergeGateRecipe(format!("cannot inspect tool workspace: {error}"))
        })?;
    if !dirty.success {
        return Err(Error::MergeGateRecipe(format!(
            "cannot inspect tool workspace: {}",
            String::from_utf8_lossy(&dirty.stderr)
        )));
    }
    if !dirty.stdout.is_empty() {
        return Err(Error::MergeGateRecipe(format!(
            "tool workspace {} must be clean before reusing its prebuilt xtask",
            tool_root.display()
        )));
    }
    let tool_head = git_revision(tool_root, runner, "HEAD")?;
    let target_head = git_revision(target_root, runner, "HEAD")?;
    let mut dirty_arguments = vec![
        OsString::from("status"),
        OsString::from("--porcelain"),
        OsString::from("--"),
    ];
    dirty_arguments.extend(BUILD_INPUT_PATHS.iter().map(OsString::from));
    let target_dirty = runner
        .run(target_root, OsStr::new("git"), &dirty_arguments)
        .map_err(|error| {
            Error::MergeGateRecipe(format!("cannot inspect selected workspace: {error}"))
        })?;
    if !target_dirty.success {
        return Err(Error::MergeGateRecipe(format!(
            "cannot inspect selected workspace: {}",
            String::from_utf8_lossy(&target_dirty.stderr)
        )));
    }
    if !target_dirty.stdout.is_empty() {
        return Err(Error::MergeGateRecipe(format!(
            "selected workspace {} has uncommitted xtask build inputs",
            target_root.display()
        )));
    }
    let mut arguments = vec![
        OsString::from("diff"),
        OsString::from("--name-only"),
        OsString::from(&tool_head),
        OsString::from(&target_head),
        OsString::from("--"),
    ];
    arguments.extend(BUILD_INPUT_PATHS.iter().map(OsString::from));
    let changed = runner
        .run(target_root, OsStr::new("git"), &arguments)
        .map_err(|error| {
            Error::MergeGateRecipe(format!("cannot compare xtask build inputs: {error}"))
        })?;
    if !changed.success {
        return Err(Error::MergeGateRecipe(format!(
            "cannot compare xtask build inputs: {}",
            String::from_utf8_lossy(&changed.stderr)
        )));
    }
    if !changed.stdout.is_empty() {
        return Err(Error::MergeGateRecipe(format!(
            "prebuilt xtask is incompatible with selected workspace {}; its Git tree changes xtask build inputs relative to tool workspace {}: {}",
            target_root.display(),
            tool_root.display(),
            String::from_utf8_lossy(&changed.stdout).trim()
        )));
    }
    Ok(verified)
}

#[implements("NFR-142-AC-3")]
fn require_tool_on_main_history(tool_root: &Path, runner: &dyn CommandRunner) -> Result<()> {
    let fetch = runner
        .run(
            tool_root,
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
            "cannot fetch origin/main for tool workspace: {}",
            String::from_utf8_lossy(&fetch.stderr)
        )));
    }
    git_revision(tool_root, runner, "HEAD")?;
    git_revision(tool_root, runner, "origin/main")?;
    let ancestor = runner
        .run(
            tool_root,
            OsStr::new("git"),
            &os_args(&["merge-base", "--is-ancestor", "HEAD", "origin/main"]),
        )
        .map_err(|error| {
            Error::MergeGateRecipe(format!("cannot check tool workspace main history: {error}"))
        })?;
    if !ancestor.success {
        return Err(Error::MergeGateRecipe(
            "tool workspace HEAD must be on origin/main history before reusing its prebuilt xtask"
                .to_owned(),
        ));
    }
    Ok(())
}

fn run_with_runner(
    workspace_root: &Path,
    operands: &[OsString],
    runner: &dyn CommandRunner,
) -> Result<String> {
    let (head, main) = require_main_merged(workspace_root, runner)?;
    run_with_verified_main(workspace_root, operands, runner, head, main)
}

fn run_with_verified_main(
    workspace_root: &Path,
    operands: &[OsString],
    runner: &dyn CommandRunner,
    head: String,
    main: String,
) -> Result<String> {
    let (preflight_requested, operands) = match operands.split_first() {
        Some((flag, paths)) if flag.to_str() == Some("--preflight") => (true, paths),
        _ => (false, operands),
    };
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
    let mut output = format!("# PR head {head}; origin/main {main}\n");
    if paths.iter().all(|path| path.starts_with("spec/")) {
        output.push_str("NO CARGO: spec-only change.\n");
        return Ok(output);
    }
    let mut recipe = plan_paths(workspace_root, &paths)?;
    for path in &paths {
        if is_test_path(path) && ignore_attribute_changed(workspace_root, path, runner)? {
            add_named_library_modules(workspace_root, &mut recipe, "xtask")?;
        }
    }

    if preflight_requested {
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
        if !path_exists(workspace_root, path)
            && !is_virtual_path(path)
            && !is_root_shared_test_path(path)
        {
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
        apply_manifest_rule(workspace_root, recipe)?;
        return Ok(());
    }
    if matches!(path, "src/lib.rs" | "src/main.rs" | "src/mod.rs") {
        add_named_library_modules(workspace_root, recipe, "filament-spec-language")?;
        return Ok(());
    }
    if let Some(relative) = path.strip_prefix("tools/fsl-probability-check/") {
        return plan_probability_check(workspace_root, path, relative, recipe);
    }
    if is_root_shared_test_path(path) {
        return plan_root_shared_test_file(workspace_root, path, recipe);
    }
    if path.ends_with("/Cargo.toml") {
        apply_crate_fallback(path, workspace_root, recipe)?;
        return Ok(());
    }
    if is_integration_root_path(workspace_root, path) {
        // A root is not a module, so a directory's changed-module rule cannot
        // select it: such a root runs every module.
        let rule = matching_rule(path).filter(|rule| {
            !rule
                .it_filters
                .iter()
                .any(|filter| matches!(filter.rule, ItRule::ChangedModule))
        });
        if let Some(rule) = rule {
            apply_rule(workspace_root, path, rule, recipe)?;
        } else {
            add_all_it_modules(workspace_root, test_package(path), recipe)?;
        }
        return Ok(());
    }
    if path.starts_with("xtask/") {
        if path.starts_with("xtask/src/")
            && !path.ends_with("/lib.rs")
            && !path.ends_with("/main.rs")
        {
            add_library(recipe, "xtask", source_module_filter(path)?, true);
        } else {
            add_named_library_modules(workspace_root, recipe, "xtask")?;
        }
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
        return add_changed_module(workspace_root, recipe, "fsl-probability-check", path);
    }
    add_all_it_modules(workspace_root, "fsl-probability-check", recipe)
}

/// A file under the workspace root's `tests/support/` or `tests/fixtures/`,
/// which the test binaries of any package may declare by `#[path]`.
fn is_root_shared_test_path(path: &str) -> bool {
    path.starts_with("tests/support/") || path.starts_with("tests/fixtures/")
}

/// The packages whose integration binaries can declare a root shared file: the
/// root package, the probability checker, and every `fsl-` crate directory.
fn integration_packages(workspace_root: &Path) -> Result<BTreeSet<String>> {
    let unlisted = |error: std::io::Error| {
        Error::MergeGateRecipe(format!(
            "cannot list workspace root {}: {error}",
            workspace_root.display()
        ))
    };
    let mut packages = BTreeSet::from([
        "filament-spec-language".to_owned(),
        "fsl-probability-check".to_owned(),
    ]);
    for entry in fs::read_dir(workspace_root).map_err(unlisted)? {
        let name = entry.map_err(unlisted)?.file_name();
        if let Some(name) = name.to_str().filter(|name| name.starts_with("fsl-")) {
            packages.insert(name.to_owned());
        }
    }
    Ok(packages)
}

/// Selects every module of each integration binary, in any package, that
/// declares the changed root shared file. A file no binary declares (data a
/// test reads, a module only library code includes, or a deleted file) runs
/// every root-package module, with a note.
fn plan_root_shared_test_file(
    workspace_root: &Path,
    path: &str,
    recipe: &mut Recipe,
) -> Result<()> {
    let changed = normalize_components(&workspace_root.join(path));
    let mut declared = false;
    for package in integration_packages(workspace_root)? {
        for (target, root) in integration_roots(&package) {
            if !workspace_root.join(&root).is_file() {
                continue;
            }
            if integration_module_files(workspace_root, &root)?
                .iter()
                .any(|module| module.path == changed)
            {
                declared = true;
                add_target_modules(workspace_root, recipe, &package, &target, &root)?;
            }
        }
    }
    if declared {
        return Ok(());
    }
    recipe.notes.insert(format!(
        "no integration-test binary declares {path}; running every filament-spec-language integration module"
    ));
    add_all_it_modules(workspace_root, "filament-spec-language", recipe)
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
                TestSelector {
                    package: "xtask".to_owned(),
                    target: TestTarget::Integration(target.to_owned()),
                    filter: Some(name),
                    exact: true,
                    skip: Vec::new(),
                    timed: false,
                    threads: None,
                    no_match_is_note: false,
                },
            );
        }
        return Ok(());
    }
    if let Some(package) = crate_name(path) {
        if path.starts_with("xtask/tests/") {
            return Ok(());
        }
        if integration_path(path).is_some() {
            if let Some(rule) = matching_rule(path) {
                apply_rule(workspace_root, path, rule, recipe)?;
            } else {
                add_changed_module(workspace_root, recipe, package, path)?;
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
            add_changed_module(workspace_root, recipe, "filament-spec-language", path)?;
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
                TestSelector {
                    package: package.to_owned(),
                    target: TestTarget::Library,
                    filter: Some(filter.to_owned()),
                    exact: false,
                    skip: rule.skips.iter().map(|skip| (*skip).to_owned()).collect(),
                    timed: false,
                    threads: rule.threads,
                    no_match_is_note: false,
                },
            ),
            LibRule::NamedModules => add_named_library_modules(workspace_root, recipe, package)?,
        }
    }
    apply_it_filters(workspace_root, rule.it_filters, Some(path), recipe)?;
    if path.starts_with("fsl-package/src/") {
        apply_it_filters(workspace_root, PACKAGE_CONSUMERS, None, recipe)?;
        add_package_replay_controls(recipe);
    }
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
            ItRule::Fixed(filter) => {
                add_integration(workspace_root, recipe, it_filter.package, filter)?
            }
            ItRule::ChangedModule => {
                let path = changed_path.ok_or_else(|| {
                    Error::MergeGateRecipe("changed-module rule requires a path".to_owned())
                })?;
                add_changed_module(workspace_root, recipe, it_filter.package, path)?;
            }
            ItRule::AllModules(package) => add_all_it_modules(workspace_root, package, recipe)?,
        }
    }
    Ok(())
}

fn integration_roots(package: &str) -> Vec<(String, String)> {
    if package == "fsl-analyze" {
        return ANALYZE_IT_ROOTS
            .iter()
            .map(|(target, path)| ((*target).to_owned(), (*path).to_owned()))
            .collect();
    }
    let path = match package {
        "filament-spec-language" => "tests/it/main.rs".to_owned(),
        "fsl-probability-check" => format!("tools/{package}/tests/it/main.rs"),
        "fsl-hybrid-test-provider" => format!("tools/{package}/tests/it/main.rs"),
        _ => format!("{package}/tests/it/main.rs"),
    };
    vec![("it".to_owned(), path)]
}

fn is_integration_root_path(workspace_root: &Path, path: &str) -> bool {
    integration_roots(test_package(path))
        .iter()
        .any(|(_, root)| root == path && workspace_root.join(root).is_file())
}

/// One module file of an integration-test binary, declared by its root or by
/// another module file.
struct ModuleFile {
    path: PathBuf,
    /// The module path from the root, as libtest names it.
    name: String,
    /// Every test in the module, including those of its child module files.
    tests: Vec<String>,
}

fn parse_integration_file(path: &Path) -> Result<syn::File> {
    let source = fs::read_to_string(path).map_err(|error| {
        Error::MergeGateRecipe(format!(
            "cannot read integration test file {}: {error}",
            path.display()
        ))
    })?;
    syn::parse_file(&source).map_err(|error| {
        Error::MergeGateRecipe(format!(
            "cannot parse integration test file {}: {error}",
            path.display()
        ))
    })
}

/// `path` with its `.` and `..` components resolved by name, so that a
/// `#[path]` module compares equal to the changed path that names its file.
fn normalize_components(path: &Path) -> PathBuf {
    let mut normal = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normal.pop();
            }
            other => normal.push(other),
        }
    }
    normal
}

/// One root's cache slot: empty until a walk of that root succeeds.
type WalkSlot = Arc<Mutex<Option<Arc<[ModuleFile]>>>>;

/// The walks this process has made, keyed by the root file's full path, so that
/// workspaces never share one. A recipe run does not write sources, so a walk
/// stays current for the life of the process.
static MODULE_WALKS: Mutex<BTreeMap<PathBuf, WalkSlot>> = Mutex::new(BTreeMap::new());

/// Every module file of the integration binary rooted at `root`, parsed on the
/// first request for it. A failed walk is not kept.
fn integration_module_files(workspace_root: &Path, root: &str) -> Result<Arc<[ModuleFile]>> {
    let path = workspace_root.join(root);
    let slot = Arc::clone(
        MODULE_WALKS
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .entry(path.clone())
            .or_default(),
    );
    // Only this root's slot is held while parsing: other roots walk in
    // parallel, and concurrent requests for this one wait for one walk.
    let mut cached = slot.lock().unwrap_or_else(PoisonError::into_inner);
    if let Some(files) = cached.as_ref() {
        return Ok(Arc::clone(files));
    }
    let syntax = parse_integration_file(&path)?;
    let directory = path.parent().unwrap_or(workspace_root);
    let mut walk = ModuleWalk::default();
    walk.items(&syntax.items, directory, directory)?;
    let files: Arc<[ModuleFile]> = walk.files.into();
    *cached = Some(Arc::clone(&files));
    Ok(files)
}

/// A walk of one binary's module tree, in declaration order.
#[derive(Default)]
struct ModuleWalk {
    modules: Vec<String>,
    files: Vec<ModuleFile>,
}

impl ModuleWalk {
    /// Returns the tests of `items`. A `#[path]` among them is relative to
    /// `path_base`, and a module without one is a file under `child_base`.
    fn items(
        &mut self,
        items: &[syn::Item],
        path_base: &Path,
        child_base: &Path,
    ) -> Result<Vec<String>> {
        let mut tests = Vec::new();
        for item in items {
            let syn::Item::Mod(module) = item else {
                collect_test_names(std::slice::from_ref(item), &mut self.modules, &mut tests);
                continue;
            };
            let name = module.ident.to_string();
            self.modules.push(name.clone());
            if let Some((_, nested)) = &module.content {
                let directory = child_base.join(&name);
                tests.extend(self.items(nested, &directory, &directory)?);
            } else {
                tests.extend(self.file(module, &name, path_base, child_base)?);
            }
            self.modules.pop();
        }
        Ok(tests)
    }

    /// Reads the file of the out-of-line `module`, records it and its child
    /// files, and returns its tests.
    fn file(
        &mut self,
        module: &syn::ItemMod,
        name: &str,
        path_base: &Path,
        child_base: &Path,
    ) -> Result<Vec<String>> {
        let explicit = module.attrs.iter().find_map(path_attribute);
        // A `#[path]` file and a `mod.rs` own their directory; any other
        // module file keeps its children in a directory named after it.
        let (file, owns_directory) = match explicit {
            Some(relative) => (path_base.join(relative), true),
            None => {
                let file = child_base.join(format!("{name}.rs"));
                if file.exists() {
                    (file, false)
                } else {
                    (child_base.join(name).join("mod.rs"), true)
                }
            }
        };
        let file = normalize_components(&file);
        let syntax = parse_integration_file(&file)?;
        let directory = file.parent().map(Path::to_path_buf).unwrap_or_default();
        let children = if owns_directory {
            directory.clone()
        } else {
            directory.join(name)
        };
        let tests = self.items(&syntax.items, &directory, &children)?;
        self.files.push(ModuleFile {
            path: file,
            name: self.modules.join("::"),
            tests: tests.clone(),
        });
        Ok(tests)
    }
}

/// The top-level modules of the binary rooted at `root` that hold a test, each
/// with its tests.
fn modules_in_root(
    workspace_root: &Path,
    package: &str,
    target: &str,
    root: &str,
) -> Result<Vec<(String, Vec<String>)>> {
    let modules = integration_module_files(workspace_root, root)?
        .iter()
        .filter(|module| !module.name.contains("::") && !module.tests.is_empty())
        .map(|module| (module.name.clone(), module.tests.clone()))
        .collect::<Vec<_>>();
    if modules.is_empty() {
        return Err(Error::MergeGateRecipe(format!(
            "no named integration modules found in {} ({package} --test {target})",
            workspace_root.join(root).display()
        )));
    }
    Ok(modules)
}

fn add_target_modules(
    workspace_root: &Path,
    recipe: &mut Recipe,
    package: &str,
    target: &str,
    root: &str,
) -> Result<()> {
    for (module, _) in modules_in_root(workspace_root, package, target, root)? {
        add_integration_target(recipe, package, target, module);
    }
    Ok(())
}

/// Selects what a changed integration module file can affect in every binary
/// that declares it: the module itself, or, when neither the file nor its child
/// files hold a test, every module of the binary. A file no binary declares has
/// nothing to select and fails closed.
fn add_changed_module(
    workspace_root: &Path,
    recipe: &mut Recipe,
    package: &str,
    path: &str,
) -> Result<()> {
    let changed = normalize_components(&workspace_root.join(path));
    let mut declared = false;
    for (target, root) in integration_roots(package) {
        if !workspace_root.join(&root).is_file() {
            continue;
        }
        for module in integration_module_files(workspace_root, &root)?.iter() {
            if module.path != changed {
                continue;
            }
            declared = true;
            if module.tests.is_empty() {
                add_target_modules(workspace_root, recipe, package, &target, &root)?;
            } else {
                add_integration_target(recipe, package, &target, &module.name);
            }
        }
    }
    if declared {
        Ok(())
    } else {
        Err(Error::MergeGateRecipe(format!(
            "no {package} integration target declares changed file {path:?}; refusing to print a recipe that would run none of its tests"
        )))
    }
}

fn add_all_it_modules(workspace_root: &Path, package: &str, recipe: &mut Recipe) -> Result<()> {
    let roots = integration_roots(package);
    let existing = roots
        .iter()
        .filter(|(_, root)| workspace_root.join(root).is_file())
        .collect::<Vec<_>>();
    if existing.is_empty() {
        add_named_library_modules(workspace_root, recipe, package)?;
        recipe.notes.insert(format!(
            "{package} has no integration-test target; falling back to its library tests"
        ));
        return Ok(());
    }
    for (target, root) in existing {
        add_target_modules(workspace_root, recipe, package, target, root)?;
    }
    Ok(())
}

#[cfg(test)]
fn top_level_integration_modules(
    workspace_root: &Path,
    package: &str,
) -> Result<Vec<(String, String)>> {
    let mut modules = Vec::new();
    for (target, root) in integration_roots(package) {
        if workspace_root.join(&root).is_file() {
            modules.extend(
                modules_in_root(workspace_root, package, &target, &root)?
                    .into_iter()
                    .map(|(module, _)| (target.clone(), module)),
            );
        }
    }
    if modules.is_empty() {
        return Err(Error::MergeGateRecipe(format!(
            "no named integration modules found for {package}"
        )));
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

fn apply_manifest_rule(workspace_root: &Path, recipe: &mut Recipe) -> Result<()> {
    for it_filter in EMIT_CONSUMERS {
        if let ItRule::Fixed(filter) = it_filter.rule {
            add_integration(workspace_root, recipe, it_filter.package, filter)?;
        }
    }
    for filters in [
        FOUNDATION_CONSUMERS,
        SOURCE_CONSUMERS,
        CST_CONSUMERS,
        FORMS_CONSUMERS,
        PACKAGE_CONSUMERS,
    ] {
        apply_it_filters(workspace_root, filters, None, recipe)?;
    }
    add_package_replay_controls(recipe);
    recipe.notes.insert(
        "manifest or lockfile change: run workspace check and named consumer controls".to_owned(),
    );
    Ok(())
}

fn apply_crate_fallback(path: &str, workspace_root: &Path, recipe: &mut Recipe) -> Result<()> {
    let Some(package) = crate_name(path) else {
        recipe
            .notes
            .insert(format!("no crate-level rule for {path}"));
        return Ok(());
    };
    if matches!(path, "src/lib.rs" | "src/main.rs" | "src/mod.rs") {
        add_named_library_modules(workspace_root, recipe, package)?;
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
                apply_it_filters(workspace_root, PACKAGE_CONSUMERS, None, recipe)?;
                add_package_replay_controls(recipe);
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
        add_named_library_modules(workspace_root, recipe, "xtask")?;
    } else if let Some(consumers) = match package {
        "fsl-foundation" => Some(FOUNDATION_CONSUMERS),
        "fsl-source" => Some(SOURCE_CONSUMERS),
        "fsl-cst" => Some(CST_CONSUMERS),
        "fsl-forms" => Some(FORMS_CONSUMERS),
        _ => None,
    } {
        add_named_library_modules(workspace_root, recipe, package)?;
        apply_it_filters(workspace_root, consumers, None, recipe)?;
    } else if matches!(package, "fsl-eval" | "fsl-semantics") {
        add_library(
            recipe,
            package,
            source_module_filter(path).unwrap_or_else(|_| "check::".to_owned()),
            true,
        );
        for it_filter in TIMED_CONSUMER {
            if let ItRule::Fixed(filter) = it_filter.rule {
                add_integration(workspace_root, recipe, it_filter.package, filter)?;
            }
        }
        recipe.notes.insert(format!(
            "unknown {package} source path; added timed analyzer consumer control"
        ));
    } else {
        add_named_library_modules(workspace_root, recipe, package)?;
    }
    Ok(())
}

fn add_target_filter(recipe: &mut Recipe, selector: TestSelector) {
    recipe.selectors.insert(selector);
}

fn add_library(recipe: &mut Recipe, package: &str, filter: String, no_match_is_note: bool) {
    add_target_filter(
        recipe,
        TestSelector {
            package: package.to_owned(),
            target: TestTarget::Library,
            filter: Some(filter),
            exact: false,
            skip: Vec::new(),
            timed: false,
            threads: None,
            no_match_is_note,
        },
    );
}

fn add_package_replay_controls(recipe: &mut Recipe) {
    add_target_filter(
        recipe,
        TestSelector {
            package: "fsl-replay".to_owned(),
            target: TestTarget::Library,
            filter: Some(
                "tc_736_a_100000_term_source_and_list_replay_to_the_proving_verdict".to_owned(),
            ),
            exact: false,
            skip: Vec::new(),
            timed: false,
            threads: None,
            no_match_is_note: true,
        },
    );
}

fn add_named_library_modules(
    workspace_root: &Path,
    recipe: &mut Recipe,
    package: &str,
) -> Result<()> {
    let root = match package {
        "filament-spec-language" => PathBuf::from("src/lib.rs"),
        "xtask" => PathBuf::from("xtask/src/lib.rs"),
        "fsl-probability-check" => PathBuf::from("tools/fsl-probability-check/src/lib.rs"),
        _ => PathBuf::from(package).join("src/lib.rs"),
    };
    let source = fs::read_to_string(workspace_root.join(&root)).map_err(|error| {
        Error::MergeGateRecipe(format!(
            "cannot read library root {} for named test selection: {error}",
            root.display()
        ))
    })?;
    let file = syn::parse_file(&source).map_err(|error| {
        Error::MergeGateRecipe(format!(
            "cannot parse library root {} for named test selection: {error}",
            root.display()
        ))
    })?;
    let mut modules = file
        .items
        .iter()
        .filter_map(|item| match item {
            syn::Item::Mod(module) => Some(module.ident.to_string()),
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    if modules.is_empty() {
        return Err(Error::MergeGateRecipe(format!(
            "no named modules found in {} for test selection",
            root.display()
        )));
    }
    for module in std::mem::take(&mut modules) {
        add_library(recipe, package, format!("{module}::"), true);
    }
    Ok(())
}

fn add_integration(
    workspace_root: &Path,
    recipe: &mut Recipe,
    package: &str,
    filter: impl Into<String>,
) -> Result<()> {
    let filter = filter.into();
    let targets = integration_targets_for_filter(workspace_root, package, &filter)?;
    for target in targets {
        add_integration_target(recipe, package, &target, filter.clone());
    }
    Ok(())
}

fn integration_targets_for_filter(
    workspace_root: &Path,
    package: &str,
    filter: &str,
) -> Result<Vec<String>> {
    let mut targets = BTreeSet::new();
    for (target, root) in integration_roots(package) {
        if !workspace_root.join(&root).is_file() {
            continue;
        }
        if modules_in_root(workspace_root, package, &target, &root)?
            .into_iter()
            .any(|(_, tests)| tests.iter().any(|test| test.contains(filter)))
        {
            targets.insert(target);
        }
    }
    if targets.is_empty() {
        return Err(Error::MergeGateRecipe(format!(
            "test filter {filter:?} matches no test in any {package} integration target; refusing to print a recipe that would run nothing"
        )));
    }
    Ok(targets.into_iter().collect())
}

fn add_integration_target(
    recipe: &mut Recipe,
    package: &str,
    target: &str,
    filter: impl Into<String>,
) {
    add_target_filter(
        recipe,
        TestSelector {
            package: package.to_owned(),
            target: TestTarget::Integration(target.to_owned()),
            filter: Some(filter.into()),
            exact: false,
            skip: Vec::new(),
            timed: false,
            threads: None,
            no_match_is_note: false,
        },
    );
}

fn add_large_input(recipe: &mut Recipe, input: &LargeInput) {
    add_target_filter(
        recipe,
        TestSelector {
            package: input.package.to_owned(),
            target: TestTarget::Library,
            filter: Some(input.filter.to_owned()),
            exact: input.exact,
            skip: Vec::new(),
            timed: true,
            threads: Some(1),
            no_match_is_note: false,
        },
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

#[implements("NFR-142-AC-3")]
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
    use ix_trace_rs::trace;
    use std::cell::RefCell;

    fn workspace_root() -> &'static Path {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("workspace root")
    }

    fn tool_workspace() -> &'static Path {
        Path::new("/stable-tool-workspace")
    }

    fn pr_workspace() -> &'static Path {
        Path::new("/selected-pr-workspace")
    }

    struct FakeRunner {
        calls: RefCell<Vec<(String, Vec<String>)>>,
        no_tests: bool,
        ignored_only: bool,
        changed_ignore: bool,
        fetch_fails: bool,
        merge_fails: bool,
        changed_build_inputs: bool,
        dirty_tool: bool,
        dirty_target: bool,
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
                changed_build_inputs: false,
                dirty_tool: false,
                dirty_target: false,
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
            let text = if is_git
                && args.first().is_some_and(|arg| arg == "status")
                && self.dirty_tool
            {
                " M xtask/src/main.rs\n"
            } else if is_git
                && args.first().is_some_and(|arg| arg == "status")
                && self.dirty_target
                && args.contains(&"--".to_owned())
            {
                " M xtask/src/main.rs\n"
            } else if is_git
                && args.first().is_some_and(|arg| arg == "diff")
                && args.get(1).is_some_and(|arg| arg == "--name-only")
                && self.changed_build_inputs
            {
                "xtask/src/merge_gate_recipe.rs\n"
            } else if is_git && args.first().is_some_and(|arg| arg == "rev-parse") {
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

    fn integration_selector(package: &str, target: &str, filter: &str) -> TestSelector {
        TestSelector {
            package: package.to_owned(),
            target: TestTarget::Integration(target.to_owned()),
            filter: Some(filter.to_owned()),
            exact: false,
            skip: Vec::new(),
            timed: false,
            threads: None,
            no_match_is_note: false,
        }
    }

    /// The binary holding the module or tests a live-workspace filter is named
    /// for.
    fn owning_target(package: &str, filter: &str) -> &'static str {
        if package != "fsl-analyze" {
            return "it";
        }
        match filter {
            "analyze_composition_cli" | "analyze_package_provider" | "analyze_request_wire" => "it",
            "analyze_package_selection"
            | "native_correlation"
            | "protocol_bounded_lasso"
            | "timed_"
            | "timed_within_200"
            | "timed_within_201" => "it_model_check",
            "package_forms" => "it_model_check",
            "monitor_" => "it_monitor_protocol",
            "hyper" | "probabilistic" => "it_probabilistic",
            "race_freedom" => "it_refinement",
            other => panic!("name the binary that owns the filter {other:?}"),
        }
    }

    /// `recipe` runs `package`'s `filter` in the binary that owns it. A test in
    /// another binary may widen a live-workspace selection, so exact target
    /// sets are asserted over temporary workspaces only.
    fn assert_selects_owner(recipe: &Recipe, package: &str, filter: &str) {
        let owner = integration_selector(package, owning_target(package, filter), filter);
        assert!(recipe.selectors.contains(&owner), "{package} {filter}");
    }

    fn displays(recipe: &Recipe) -> Vec<String> {
        recipe.selectors.iter().map(TestSelector::display).collect()
    }

    /// A workspace whose binaries hold known tests. `it_refinement_cx1` holds
    /// none that a consumer filter names.
    fn filter_workspace() -> tempfile::TempDir {
        let workspace = tempfile::tempdir().expect("a temporary workspace");
        write_files(
            workspace.path(),
            &[
                ("fsl-analyze/src/provider.rs", ""),
                ("fsl-eval/src/simulation/reach.rs", ""),
                ("fsl-eval/src/value/expression/bare.rs", ""),
                ("fsl-semantics/src/check/abstraction.rs", ""),
                ("tests/it/main.rs", "mod language;\n"),
                (
                    "tests/it/language.rs",
                    "#[test]\nfn bare_value() {}\n#[test]\nfn abstraction_relation_inert_holds() {}\n",
                ),
                ("fsl-analyze/tests/it/main.rs", "mod package_api;\n"),
                (
                    "fsl-analyze/tests/it/package_api.rs",
                    "#[test]\nfn monitor_from_package() {}\n#[test]\nfn hyper_from_package() {}\n#[test]\nfn race_freedom_from_package() {}\n",
                ),
                ("fsl-analyze/tests/it/monitor_protocol.rs", "mod wire;\n"),
                (
                    "fsl-analyze/tests/it/wire.rs",
                    "#[test]\nfn monitor_on_the_wire() {}\n#[test]\nfn timed_wire() {}\n",
                ),
                (
                    "fsl-analyze/tests/it/probabilistic.rs",
                    "mod helper;\nmod measure;\n",
                ),
                (
                    "fsl-analyze/tests/it/measure.rs",
                    "#[test]\nfn probabilistic_hyper() {}\n",
                ),
                ("fsl-analyze/tests/it/model_check_binary.rs", "mod zones;\n"),
                (
                    "fsl-analyze/tests/it/zones.rs",
                    "#[test]\nfn timed_within_200_states() {}\n#[test]\nfn protocol_bounded_lasso_found() {}\n#[test]\nfn native_correlation_kept() {}\n",
                ),
                (
                    "fsl-analyze/tests/it/refinement.rs",
                    "mod helper;\nmod races;\n",
                ),
                (
                    "fsl-analyze/tests/it/races.rs",
                    "#[test]\nfn race_freedom_refined() {}\n",
                ),
                (
                    "fsl-analyze/tests/it/helper.rs",
                    "pub(crate) fn fixture() {}\n",
                ),
                ("fsl-analyze/tests/it_refinement_cx1.rs", "mod isolated;\n"),
                (
                    "fsl-analyze/tests/isolated.rs",
                    "#[test]\nfn liveness_alone() {}\n",
                ),
            ],
        );
        workspace
    }

    fn probability_check_modules() -> Vec<String> {
        let modules = top_level_integration_modules(workspace_root(), "fsl-probability-check")
            .expect("probability-check integration modules");
        assert!(!modules.is_empty());
        modules.into_iter().map(|(_, module)| module).collect()
    }

    fn runs_modules(recipe: &Recipe, modules: &[String]) -> bool {
        modules.iter().all(|module| {
            recipe
                .selectors
                .contains(&integration_selector("fsl-probability-check", "it", module))
        })
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
        assert!(!recipe
            .selectors
            .iter()
            .any(|item| item.package == "fsl-probability-check"));
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
            vec![&integration_selector(
                "fsl-probability-check",
                "it",
                "cyclic_mdp"
            )]
        );
        assert!(!module
            .selectors
            .iter()
            .any(|item| item.package == "filament-spec-language"));
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
            assert_selects_owner(&recipe, "fsl-analyze", "probabilistic");
            assert_selects_owner(&recipe, "fsl-analyze", "timed_");
        }
        let recipe = plan(workspace_root(), &["fsl-analyze/src/provider.rs"]);
        for filter in ["monitor_", "timed_", "hyper", "probabilistic"] {
            assert_selects_owner(&recipe, "fsl-analyze", filter);
        }
        let zone = plan(
            workspace_root(),
            &["fsl-analyze/src/zone_check/divergence.rs"],
        );
        for filter in ["timed_", "timed_within_200", "timed_within_201"] {
            assert_selects_owner(&zone, "fsl-analyze", filter);
        }
    }

    #[test]
    fn request_and_analyze_roots_run_the_source_and_package_parity_modules() {
        for path in [
            "fsl-analyze/src/analyze/request.rs",
            "fsl-analyze/src/analyze.rs",
        ] {
            let recipe = plan(workspace_root(), &[path]);
            for filter in [
                "analyze_package_selection",
                "analyze_request_wire",
                "analyze_package_provider",
                "monitor_",
                "probabilistic",
            ] {
                assert_selects_owner(&recipe, "fsl-analyze", filter);
            }
        }
        let other = plan(workspace_root(), &["fsl-analyze/src/provider.rs"]);
        assert!(!integration_runs(&other, "fsl-analyze")
            .iter()
            .any(|(_, filter)| filter == "analyze_request_wire"));
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
                assert_selects_owner(&recipe, "fsl-analyze", filter);
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
        assert_selects_owner(&recipe, "fsl-analyze", "analyze_composition_cli");
        assert_selects_owner(&recipe, "filament-spec-language", "bare_package");
    }

    /// The `(target, filter)` pairs a recipe runs for `package`'s integration
    /// tests.
    fn integration_runs(recipe: &Recipe, package: &str) -> Vec<(String, String)> {
        recipe
            .selectors
            .iter()
            .filter(|item| item.package == package)
            .filter_map(|item| match (&item.target, &item.filter) {
                (TestTarget::Integration(target), Some(filter)) => {
                    Some((target.clone(), filter.clone()))
                }
                _ => None,
            })
            .collect()
    }

    fn pair(target: &str, filter: &str) -> (String, String) {
        (target.to_owned(), filter.to_owned())
    }

    fn write_files(root: &Path, files: &[(&str, &str)]) {
        for (path, source) in files {
            let path = root.join(path);
            fs::create_dir_all(path.parent().expect("a parent directory")).expect("directories");
            fs::write(path, source).expect("fixture file");
        }
    }

    #[test]
    fn a_changed_nested_module_file_runs_in_the_binary_that_declares_it() {
        let recipe = plan(
            workspace_root(),
            &["fsl-analyze/tests/it/weak_memory/replay_boundary_cases.rs"],
        );
        assert!(integration_runs(&recipe, "fsl-analyze")
            .contains(&pair("it_refinement", "weak_memory::replay_boundary_cases")));
    }

    #[test]
    fn a_changed_helper_only_module_runs_every_module_of_each_binary_that_declares_it() {
        let recipe = plan(
            workspace_root(),
            &["fsl-analyze/tests/it/liveness_support.rs"],
        );
        let runs = integration_runs(&recipe, "fsl-analyze");
        for expected in [
            pair("it_model_check", "component_certificate"),
            pair("it_model_check", "liveness"),
            pair("it_probabilistic", "exact_probabilistic"),
        ] {
            assert!(runs.contains(&expected), "{expected:?}");
        }
        assert!(runs.iter().all(|(_, filter)| filter != "liveness_support"));

        // The two refinement binaries declare this one.
        let recipe = plan(
            workspace_root(),
            &["fsl-analyze/tests/it/refinement_liveness_support.rs"],
        );
        let runs = integration_runs(&recipe, "fsl-analyze");
        assert!(runs.contains(&pair("it_refinement", "refinement_liveness")));
        assert!(runs.contains(&pair("it_refinement", "refinement_certificate")));
        assert!(runs.contains(&pair("it_refinement_cx1", "refinement_liveness_cx1")));
    }

    #[test]
    fn a_consumer_filter_selects_exactly_the_binaries_holding_a_matching_test() {
        let workspace = filter_workspace();
        let runs = |path: &str| integration_runs(&plan(workspace.path(), &[path]), "fsl-analyze");
        assert_eq!(
            runs("fsl-analyze/src/provider.rs"),
            [
                pair("it", "hyper"),
                pair("it", "monitor_"),
                pair("it_model_check", "timed_"),
                pair("it_monitor_protocol", "monitor_"),
                pair("it_monitor_protocol", "timed_"),
                pair("it_probabilistic", "hyper"),
                pair("it_probabilistic", "probabilistic"),
            ]
        );
        assert_eq!(
            runs("fsl-eval/src/simulation/reach.rs"),
            [
                pair("it", "race_freedom"),
                pair("it_model_check", "native_correlation"),
                pair("it_model_check", "protocol_bounded_lasso"),
                pair("it_refinement", "race_freedom"),
            ]
        );
        assert_eq!(
            runs("fsl-analyze/tests/it/helper.rs"),
            [
                pair("it_probabilistic", "measure"),
                pair("it_refinement", "races"),
            ]
        );
    }

    #[test]
    fn a_root_shared_file_runs_every_module_of_each_binary_that_declares_it() {
        let workspace = tempfile::tempdir().expect("a temporary workspace");
        write_files(
            workspace.path(),
            &[
                (
                    "tests/it/main.rs",
                    "mod language;\n#[path = \"../support/mod.rs\"]\nmod support;\n",
                ),
                ("tests/it/language.rs", "#[test]\nfn words() {}\n"),
                ("tests/support/mod.rs", "mod child;\n"),
                ("tests/support/child.rs", "pub(crate) fn fixture() {}\n"),
                ("tests/support/shared.rs", "pub(crate) fn fixture() {}\n"),
                ("tests/fixtures/data.native", "package data\n"),
                (
                    "fsl-analyze/tests/it/refinement.rs",
                    "#[path = \"../../../tests/support/shared.rs\"]\nmod shared;\nmod refined;\n",
                ),
                (
                    "fsl-analyze/tests/it/refined.rs",
                    "#[test]\nfn refines() {}\n",
                ),
                ("fsl-analyze/tests/it/probabilistic.rs", "mod measure;\n"),
                (
                    "fsl-analyze/tests/it/measure.rs",
                    "#[test]\nfn measures() {}\n",
                ),
                (
                    "fsl-replay/tests/it/main.rs",
                    "#[path = \"../../../tests/support/shared.rs\"]\nmod shared;\nmod replayed;\n",
                ),
                (
                    "fsl-replay/tests/it/replayed.rs",
                    "#[test]\nfn replays() {}\n",
                ),
            ],
        );
        let shared = plan(workspace.path(), &["tests/support/shared.rs"]);
        assert_eq!(
            displays(&shared),
            [
                "cargo test --locked -p fsl-analyze --test it_refinement -- refined",
                "cargo test --locked -p fsl-replay --test it -- replayed",
            ]
        );
        assert!(shared.notes.is_empty());

        // Declared through the root binary's `support` module.
        let root_modules = ["cargo test --locked -p filament-spec-language --test it -- language"];
        let child = plan(workspace.path(), &["tests/support/child.rs"]);
        assert_eq!(displays(&child), root_modules);
        assert!(child.notes.is_empty());

        // No binary declares a data file, and a deleted file is not declared.
        for path in ["tests/fixtures/data.native", "tests/support/deleted.rs"] {
            let recipe = plan(workspace.path(), &[path]);
            assert_eq!(displays(&recipe), root_modules, "{path}");
            assert!(
                recipe
                    .notes
                    .iter()
                    .any(|note| note
                        .contains(&format!("no integration-test binary declares {path}"))),
                "{path}"
            );
        }
    }

    #[test]
    fn a_live_root_support_file_selects_declaring_binaries_across_packages() {
        let recipe = plan(workspace_root(), &["tests/support/refinement_aux.rs"]);
        for (package, target) in [
            ("fsl-analyze", "it_refinement"),
            ("fsl-analyze", "it_refinement_cx1"),
            ("fsl-replay", "it"),
        ] {
            assert!(
                integration_runs(&recipe, package)
                    .iter()
                    .any(|(selected, _)| selected == target),
                "{package} --test {target}"
            );
        }
    }

    #[test]
    fn changed_files_of_a_single_binary_package_follow_the_module_tree() {
        let workspace = tempfile::tempdir().expect("a temporary workspace");
        write_files(
            workspace.path(),
            &[
                ("tests/it/main.rs", "mod helper;\nmod used;\n"),
                ("tests/it/helper.rs", "pub(crate) fn fixture() {}\n"),
                (
                    "tests/it/used.rs",
                    "mod nested;\n#[path = \"moved_cases.rs\"]\nmod renamed;\n",
                ),
                ("tests/it/used/nested.rs", "#[test]\nfn deep() {}\n"),
                ("tests/it/moved_cases.rs", "#[test]\nfn moved() {}\n"),
                ("tests/it/orphan.rs", "#[test]\nfn never_built() {}\n"),
            ],
        );
        let runs = |path: &str| {
            integration_runs(&plan(workspace.path(), &[path]), "filament-spec-language")
        };
        assert_eq!(runs("tests/it/helper.rs"), [pair("it", "used")]);
        assert_eq!(
            runs("tests/it/used/nested.rs"),
            [pair("it", "used::nested")]
        );
        assert_eq!(
            runs("tests/it/moved_cases.rs"),
            [pair("it", "used::renamed")]
        );
        let orphan = plan_paths(workspace.path(), &["tests/it/orphan.rs".to_owned()]);
        assert!(
            orphan.is_err(),
            "a file no binary declares must fail closed"
        );
    }

    #[test]
    fn a_module_walk_is_read_once_per_root_and_never_shared_between_workspaces() {
        let names = |workspace: &Path| {
            integration_module_files(workspace, "tests/it/main.rs").map(|files| {
                files
                    .iter()
                    .map(|module| module.name.clone())
                    .collect::<Vec<_>>()
            })
        };
        let first = tempfile::tempdir().expect("a temporary workspace");
        write_files(first.path(), &[("tests/it/main.rs", "mod words;\n")]);
        // A walk that fails is made again.
        assert!(names(first.path()).is_err());
        write_files(first.path(), &[("tests/it/words.rs", "")]);
        assert_eq!(names(first.path()).expect("a walk"), ["words"]);

        let second = tempfile::tempdir().expect("a temporary workspace");
        write_files(
            second.path(),
            &[
                ("tests/it/main.rs", "mod numbers;\n"),
                ("tests/it/numbers.rs", ""),
            ],
        );
        assert_eq!(names(second.path()).expect("a walk"), ["numbers"]);

        let walk = || integration_module_files(first.path(), "tests/it/main.rs").expect("a walk");
        let cached = walk();
        fs::remove_file(first.path().join("tests/it/words.rs")).expect("remove a module");
        assert!(Arc::ptr_eq(&cached, &walk()), "the root was parsed again");
    }

    #[test]
    fn a_selection_that_would_match_no_test_fails_closed() {
        for package in ["fsl-analyze", "fsl-command", "filament-spec-language"] {
            assert!(
                integration_targets_for_filter(workspace_root(), package, "no_such_test_name")
                    .is_err(),
                "{package}"
            );
        }
        let workspace = tempfile::tempdir().expect("a temporary workspace");
        write_files(
            workspace.path(),
            &[
                ("tests/it/main.rs", "mod helper;\n"),
                ("tests/it/helper.rs", "pub(crate) fn fixture() {}\n"),
            ],
        );
        let helper = plan_paths(workspace.path(), &["tests/it/helper.rs".to_owned()]);
        assert!(
            helper.is_err(),
            "a binary with no test cannot cover a helper"
        );
    }

    #[test]
    fn every_table_filter_matches_a_test() {
        let tables = RULES
            .iter()
            .flat_map(|rule| rule.it_filters)
            .chain(EMIT_CONSUMERS)
            .chain(TIMED_CONSUMER);
        let filters = tables
            .filter_map(|it_filter| match it_filter.rule {
                ItRule::Fixed(filter) => Some((it_filter.package, filter)),
                ItRule::ChangedModule | ItRule::AllModules(_) => None,
            })
            .collect::<BTreeSet<_>>();
        assert!(!filters.is_empty());
        for (package, filter) in filters {
            integration_targets_for_filter(workspace_root(), package, filter)
                .unwrap_or_else(|error| panic!("{package}: {error}"));
        }
    }

    #[test]
    fn the_isolated_cx1_module_and_root_select_the_cx1_binary() {
        for path in [
            "fsl-analyze/tests/it/refinement_liveness_cx1.rs",
            "fsl-analyze/tests/it_refinement_cx1.rs",
        ] {
            let recipe = plan(workspace_root(), &[path]);
            assert!(
                recipe.selectors.iter().any(|selector| {
                    selector.package == "fsl-analyze"
                        && selector.target
                            == TestTarget::Integration("it_refinement_cx1".to_owned())
                        && selector.filter.as_deref() == Some("refinement_liveness_cx1")
                }),
                "{path} must run fsl-analyze --test it_refinement_cx1"
            );
        }
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
        assert!(recipe
            .selectors
            .iter()
            .any(|item| item.package == "fsl-replay"
                && item.filter.as_deref()
                    == Some("tc_736_a_100000_term_source_and_list_replay_to_the_proving_verdict")
                && item.timed));
        assert!(recipe
            .selectors
            .iter()
            .filter(|item| item.timed)
            .all(|item| item.threads == Some(1)));
    }

    #[test]
    fn command_paths_include_local_modules_and_forms_spans_selects_its_module() {
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
                && item.filter.as_deref() == Some("spans::")
        }));
        assert!(spans
            .selectors
            .iter()
            .any(|item| item.filter.as_deref() == Some(LARGE_EMIT_FILTER)));
    }

    #[test]
    fn main_roots_are_checked_before_test_prefixes_and_coverage_is_table_driven() {
        for package in ["fsl-analyze", "fsl-command", "filament-spec-language"] {
            let roots = integration_roots(package);
            assert!(!roots.is_empty());
            for (_, root) in &roots {
                assert!(is_integration_root_path(workspace_root(), root), "{root}");
                assert!(
                    plan(workspace_root(), &[root]).selectors.len() > 1,
                    "{root}"
                );
            }
            let modules =
                top_level_integration_modules(workspace_root(), package).expect("module list");
            assert!(!modules.is_empty());
            let mut routed = 0;
            for (target, module) in modules {
                let directory = if package == "filament-spec-language" {
                    "tests/it".to_owned()
                } else {
                    format!("{package}/tests/it")
                };
                // A module a root loads by `#[path]` from outside `tests/it`
                // is routed by its own file's rules.
                let Some(path) = [
                    format!("{directory}/{module}.rs"),
                    format!("{directory}/{module}/mod.rs"),
                ]
                .into_iter()
                .find(|path| workspace_root().join(path).is_file()) else {
                    continue;
                };
                routed += 1;
                assert!(
                    matching_rule(&path).is_some(),
                    "unrouted module path {path}"
                );
                let recipe = plan(workspace_root(), &[&path]);
                assert!(
                    recipe.selectors.iter().any(|selector| {
                        selector.package == package
                            && selector.target == TestTarget::Integration(target.clone())
                            && selector.filter.as_deref() == Some(module.as_str())
                    }),
                    "{path} must route to {package} --test {target}"
                );
            }
            assert!(routed > 0, "{package}");
        }
        let analyze = plan(workspace_root(), &["fsl-analyze/tests/it/main.rs"]);
        for (target, _) in ANALYZE_IT_ROOTS {
            assert!(
                analyze.selectors.iter().any(|selector| {
                    selector.package == "fsl-analyze"
                        && selector.target == TestTarget::Integration((*target).to_owned())
                }),
                "the root recipe must select fsl-analyze --test {target}"
            );
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
        assert!(calls
            .iter()
            .any(|(_, args)| args.first().is_some_and(|arg| arg == "merge-base")));
        let compile = result.find("--no-run").expect("compile first");
        let tests = result
            .find("\ncargo test --locked -p fsl-analyze --test it_probabilistic -- probabilistic")
            .expect("filtered test after compile");
        assert!(compile < tests);
    }

    #[test]
    fn default_run_prints_recipe_without_invoking_cargo() {
        let runner = FakeRunner::new();
        let result = run_with_runner(
            workspace_root(),
            &[OsString::from("fsl-analyze/src/analyze/exact.rs")],
            &runner,
        )
        .expect("print recipe");
        assert!(result.contains(
            "cargo test --locked -p fsl-analyze --test it_probabilistic -- probabilistic"
        ));
        assert!(runner
            .calls
            .borrow()
            .iter()
            .all(|(program, _)| program == "git"));
    }

    #[test]
    fn preflight_flag_runs_ignored_aware_preflight_and_fails_closed() {
        let runner = FakeRunner {
            no_tests: true,
            ..FakeRunner::new()
        };
        assert!(run_with_runner(
            workspace_root(),
            &[
                OsString::from("--preflight"),
                OsString::from("fsl-analyze/src/analyze/exact.rs"),
            ],
            &runner,
        )
        .is_err());
        assert!(runner
            .calls
            .borrow()
            .iter()
            .any(|(program, args)| { program != "git" && args.contains(&"--ignored".to_owned()) }));
    }

    #[test]
    fn preflight_rejects_zero_matches_and_ignored_only_matches() {
        let mut runner = FakeRunner::new();
        runner.no_tests = true;
        assert_eq!(
            preflight(
                workspace_root(),
                &integration_selector("fsl-analyze", "it", "explicit"),
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
                &integration_selector("fsl-analyze", "it", "explicit"),
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
        assert!(run_with_runner(
            workspace_root(),
            &[
                OsString::from("--preflight"),
                OsString::from("fsl-analyze/src/analyze/exact.rs"),
            ],
            &runner
        )
        .is_err());
    }

    #[test]
    fn empty_unknown_manifest_fixture_deleted_and_root_paths_have_fallbacks() {
        assert!(plan_paths(workspace_root(), &[])
            .expect("empty planning")
            .selectors
            .is_empty());
        assert!(run_with_runner(workspace_root(), &[], &FakeRunner::new()).is_err());
        assert!(plan(workspace_root(), &["untracked/unknown.dat"])
            .notes
            .iter()
            .any(|note| note.contains("unknown.dat")));
        assert!(plan(workspace_root(), &["Cargo.lock"])
            .selectors
            .iter()
            .any(|item| item.filter.as_deref() == Some("monitor_")));
        assert!(plan(workspace_root(), &["fsl-eval/src/new_unknown.rs"])
            .notes
            .iter()
            .any(|note| note.contains("unknown fsl-eval")));
        assert!(
            plan(workspace_root(), &["fsl-semantics/src/new_unknown.rs"])
                .notes
                .iter()
                .any(|note| note.contains("unknown fsl-semantics"))
        );
        assert!(plan(workspace_root(), &["xtask/src/new_task.rs"])
            .selectors
            .iter()
            .any(|item| item.package == "xtask" && item.filter.is_some()));
        assert!(
            plan(
                workspace_root(),
                &["fsl-analyze/tests/fixtures/sample.json"]
            )
            .selectors
            .len()
                > 1
        );
        assert!(plan(workspace_root(), &["fsl-analyze/src/deleted.rs"])
            .notes
            .iter()
            .any(|note| note.contains("absent")));
        assert!(plan(workspace_root(), &["fsl-analyze/src/lib.rs"])
            .selectors
            .iter()
            .any(|item| item.filter.as_deref() == Some("monitor_")));
        assert!(plan(workspace_root(), &["src/lib.rs"])
            .selectors
            .iter()
            .any(|item| item.package == "filament-spec-language" && item.filter.is_some()));
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
        assert!(recipe
            .selectors
            .iter()
            .any(|item| item.package == "xtask"
                && item.filter.as_deref() == Some("merge_gate_recipe::")));
    }

    #[test]
    fn low_level_changes_select_named_downstream_modules() {
        let foundation = plan(workspace_root(), &["fsl-foundation/src/source_map.rs"]);
        assert!(foundation.selectors.iter().any(|item| {
            item.package == "fsl-cst" && item.target == TestTarget::Integration("it".to_owned())
        }));
        assert!(foundation.selectors.iter().any(|item| {
            item.package == "filament-spec-language"
                && item.target == TestTarget::Integration("it".to_owned())
        }));

        let source = plan(workspace_root(), &["fsl-source/src/preflight.rs"]);
        assert!(source.selectors.iter().any(|item| {
            item.package == "fsl-command" && item.target == TestTarget::Integration("it".to_owned())
        }));

        let cst = plan(workspace_root(), &["fsl-cst/src/cst.rs"]);
        assert!(cst.selectors.iter().any(|item| {
            item.package == "fsl-forms" && item.target == TestTarget::Integration("it".to_owned())
        }));

        let forms = plan(workspace_root(), &["fsl-forms/src/spans.rs"]);
        assert!(forms.selectors.iter().any(|item| {
            item.package == "fsl-semantics"
                && item.target == TestTarget::Integration("it".to_owned())
        }));

        let package = plan(workspace_root(), &["fsl-package/src/probabilistic_read.rs"]);
        assert!(package.selectors.iter().any(|item| {
            item.package == "fsl-eval" && item.target == TestTarget::Integration("it".to_owned())
        }));
        assert!(package.selectors.iter().any(|item| {
            item.package == "fsl-replay"
                && item.target == TestTarget::Library
                && item.filter.as_deref()
                    == Some("tc_736_a_100000_term_source_and_list_replay_to_the_proving_verdict")
                && !item.exact
        }));
        assert!(foundation
            .selectors
            .iter()
            .chain(source.selectors.iter())
            .chain(cst.selectors.iter())
            .chain(forms.selectors.iter())
            .chain(package.selectors.iter())
            .all(|item| item.filter.is_some()));
    }

    #[test]
    fn crate_wide_fallbacks_emit_only_named_library_filters() {
        for path in [
            "xtask/src/lib.rs",
            "xtask/Cargo.toml",
            "src/lib.rs",
            "fsl-foundation/Cargo.toml",
            "fsl-source/src/lib.rs",
            "fsl-cst/src/lib.rs",
            "fsl-forms/src/lib.rs",
            "fsl-package/src/lib.rs",
        ] {
            let recipe = plan(workspace_root(), &[path]);
            assert!(!recipe.selectors.is_empty(), "{path}");
            assert!(
                recipe.selectors.iter().all(|item| item.filter.is_some()),
                "{path} selected an unfiltered library target: {}",
                displays(&recipe).join("\n")
            );
        }
    }

    #[test]
    fn low_level_crate_roots_and_manifests_include_named_consumers() {
        for (path, package, filter) in [
            (
                "fsl-foundation/src/lib.rs",
                "fsl-command",
                "check_source_identity",
            ),
            (
                "fsl-foundation/Cargo.toml",
                "fsl-command",
                "check_source_identity",
            ),
            (
                "fsl-source/src/lib.rs",
                "fsl-command",
                "extraction_request_refusals",
            ),
            (
                "fsl-source/Cargo.toml",
                "fsl-command",
                "extraction_request_refusals",
            ),
            ("fsl-cst/src/lib.rs", "fsl-replay", "compile_text_literals"),
            ("fsl-cst/Cargo.toml", "fsl-replay", "compile_text_literals"),
            ("fsl-forms/src/lib.rs", "fsl-analyze", "package_forms"),
            ("fsl-forms/Cargo.toml", "fsl-analyze", "package_forms"),
        ] {
            let recipe = plan(workspace_root(), &[path]);
            assert_selects_owner(&recipe, package, filter);
            assert!(
                recipe
                    .selectors
                    .iter()
                    .all(|selector| selector.filter.is_some()),
                "{path}"
            );
        }

        let manifest = plan(workspace_root(), &["Cargo.lock"]);
        for (package, filter) in [
            ("fsl-command", "check_source_identity"),
            ("fsl-command", "extraction_request_refusals"),
            ("fsl-replay", "compile_text_literals"),
            ("fsl-analyze", "package_forms"),
            ("fsl-eval", "checked_route_owner"),
        ] {
            assert_selects_owner(&manifest, package, filter);
        }
        assert!(manifest
            .selectors
            .iter()
            .all(|selector| selector.filter.is_some()));
    }

    #[test]
    fn semantics_integration_commands_enable_test_support() {
        let source_objects = plan(
            workspace_root(),
            &["fsl-semantics/tests/it/source_objects.rs"],
        );
        let source_objects = source_objects
            .selectors
            .iter()
            .find(|selector| {
                selector.package == "fsl-semantics"
                    && selector.filter.as_deref() == Some("source_objects")
            })
            .expect("source_objects selector");
        assert_eq!(
            source_objects.command(false).join(" "),
            "test --locked -p fsl-semantics --features test-support --test it -- source_objects"
        );

        let manifest = plan(workspace_root(), &["Cargo.lock"]);
        for selector in manifest.selectors.iter().filter(|selector| {
            selector.package == "fsl-semantics"
                && matches!(&selector.target, TestTarget::Integration(_))
        }) {
            let command = selector.command(false);
            let features = command
                .windows(2)
                .any(|pair| pair[0] == "--features" && pair[1] == "test-support");
            assert!(features, "{}", command.join(" "));
        }

        let semantics_library = plan(workspace_root(), &["fsl-semantics/src/check/check.rs"]);
        assert!(semantics_library.selectors.iter().all(|selector| {
            selector.package != "fsl-semantics"
                || matches!(&selector.target, TestTarget::Integration(_))
                || !selector.command(false).contains(&"--features".to_owned())
        }));
    }

    #[test]
    fn simulation_changes_run_the_protocol_state_key_modules() {
        let recipe = plan(
            workspace_root(),
            &["fsl-eval/src/simulation/protocol/reach.rs"],
        );
        for filter in [
            "protocol_bounded_lasso",
            "race_freedom",
            "native_correlation",
        ] {
            assert_selects_owner(&recipe, "fsl-analyze", filter);
        }
    }

    #[test]
    fn semantically_related_rules_and_dedup_are_explicit_and_ordered() {
        let workspace = filter_workspace();
        let recipe = plan(
            workspace.path(),
            &[
                "fsl-semantics/src/check/abstraction.rs",
                "fsl-eval/src/value/expression/bare.rs",
                "fsl-semantics/src/check/abstraction.rs",
            ],
        );
        // The repeated path adds nothing, and `timed_`, which both rules name,
        // is selected once in each analyzer target holding such a test.
        assert_eq!(
            displays(&recipe),
            [
                "cargo test --locked -p filament-spec-language --test it -- abstraction_relation_inert",
                "cargo test --locked -p filament-spec-language --test it -- bare_",
                "cargo test --locked -p fsl-analyze --test it_model_check -- timed_",
                "cargo test --locked -p fsl-analyze --test it_monitor_protocol -- timed_",
                "cargo test --locked -p fsl-eval --lib -- value::expression::bare::",
                "cargo test --locked -p fsl-semantics --lib -- check::abstraction::",
            ]
        );
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

    #[test]
    #[trace("NFR-142-AC-1")]
    fn explicit_workspace_is_selected_and_removed_from_changed_paths() {
        let root = workspace_root();
        let args = [
            OsString::from("--workspace"),
            root.as_os_str().to_owned(),
            OsString::from("--preflight"),
            OsString::from("xtask/src/main.rs"),
        ];
        let (selected, remaining) = select_workspace(root, &args).expect("workspace");
        assert_eq!(selected, root.canonicalize().expect("canonical workspace"));
        assert!(select_workspace(root, &[OsString::from("xtask/src/main.rs")]).is_err());
        assert_eq!(
            remaining,
            [
                OsString::from("--preflight"),
                OsString::from("xtask/src/main.rs")
            ]
        );
    }

    #[test]
    #[trace("NFR-142-AC-2")]
    fn tool_workspace_refuses_different_recipe_inputs_and_dirty_sources() {
        let runner = FakeRunner {
            changed_build_inputs: true,
            ..FakeRunner::new()
        };
        let error = ensure_compatible(tool_workspace(), pr_workspace(), &runner)
            .expect_err("different xtask source trees must refuse reuse");
        assert!(error.to_string().contains("xtask/src/merge_gate_recipe.rs"));

        let runner = FakeRunner {
            dirty_tool: true,
            ..FakeRunner::new()
        };
        let error = ensure_compatible(tool_workspace(), pr_workspace(), &runner)
            .expect_err("dirty tool sources must refuse reuse");
        assert!(error.to_string().contains("must be clean"));

        let runner = FakeRunner {
            dirty_target: true,
            ..FakeRunner::new()
        };
        let error = ensure_compatible(tool_workspace(), pr_workspace(), &runner)
            .expect_err("uncommitted target sources must refuse reuse");
        assert!(error.to_string().contains("uncommitted xtask build inputs"));
    }

    #[test]
    #[trace("NFR-142-AC-2", "NFR-142-AC-3")]
    fn stable_tool_workspace_accepts_matching_clean_recipe_inputs() {
        let runner = FakeRunner::new();
        ensure_compatible(tool_workspace(), pr_workspace(), &runner)
            .expect("same clean Git tree is compatible");
        let calls = runner.calls.borrow();
        assert!(calls.iter().any(|(_, args)| {
            args.first().is_some_and(|arg| arg == "fetch")
                && args.get(1).is_some_and(|arg| arg == "origin")
        }));
        assert_eq!(
            calls
                .iter()
                .filter(|(_, args)| args.first().is_some_and(|arg| arg == "fetch"))
                .count(),
            2,
            "both tool and PR workspaces fetch origin/main"
        );
        assert!(calls.iter().any(|(_, args)| {
            args.first().is_some_and(|arg| arg == "diff")
                && args.get(1).is_some_and(|arg| arg == "--name-only")
                && args.iter().any(|arg| arg == "xtask/src")
        }));
        assert!(calls.iter().any(|(_, args)| {
            args.first().is_some_and(|arg| arg == "merge-base")
                && args.get(2).is_some_and(|arg| arg == "HEAD")
                && args.get(3).is_some_and(|arg| arg == "origin/main")
        }));
    }

    #[test]
    #[trace("NFR-142-AC-3")]
    fn tool_or_target_fetch_and_ancestry_failures_refuse_reuse() {
        let runner = FakeRunner {
            fetch_fails: true,
            ..FakeRunner::new()
        };
        let error = ensure_compatible(tool_workspace(), pr_workspace(), &runner)
            .expect_err("tool fetch failure must refuse reuse");
        assert!(error.to_string().contains("cannot fetch origin/main"));

        let runner = FakeRunner {
            merge_fails: true,
            ..FakeRunner::new()
        };
        let error = ensure_compatible(tool_workspace(), pr_workspace(), &runner)
            .expect_err("ancestry failure must refuse reuse");
        assert!(error
            .to_string()
            .contains("tool workspace HEAD must be on origin/main history"));
    }

    #[test]
    #[trace("NFR-142-AC-4")]
    fn same_workspace_tooling_change_uses_runtime_main_check_without_stable_tree_comparison() {
        let runner = FakeRunner::new();
        ensure_compatible(workspace_root(), workspace_root(), &runner)
            .expect("tooling PR may build and run in its own clean workspace");
        let calls = runner.calls.borrow();
        assert_eq!(
            calls
                .iter()
                .filter(|(_, args)| args.first().is_some_and(|arg| arg == "fetch"))
                .count(),
            1,
            "same workspace only needs the PR's runtime main check"
        );
        assert!(!calls.iter().any(|(_, args)| {
            args.first().is_some_and(|arg| arg == "diff")
                || (args.first().is_some_and(|arg| arg == "merge-base")
                    && args.get(2).is_some_and(|arg| arg == "HEAD")
                    && args.get(3).is_some_and(|arg| arg == "origin/main"))
        }));
    }
}
