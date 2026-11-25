// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL-2478 at the layer this repository owns. Root and per-verb `--help`
//! text and the argv parser are the driver's (filament-driver
//! `fsl-cli/src/options.rs`); FSL owns two option surfaces the driver hands
//! through unchanged:
//!
//! - the FR-255 settings operation behind `--limit <name>=<value>`, whose
//!   unknown-setting refusal is the list of accepted names the user sees;
//! - the in-process `version` invocation (`--format`, `--kind`).
//!
//! Each is checked in both directions: every name the parser accepts is
//! listed, every listed name is accepted, and everything else is refused.

use fsl_foundation::setting::{parse_operands, AccountingSetting, Setting, UsageCause};
use fsl_foundation::SettingLimits;
use fsl_replay::{CallerLimits, WorkerCounts};
use ix_trace_rs::trace;
use serde_json::{json, Value};

use super::Case;

/// Every name the settings operation accepts.
fn accepted() -> Vec<&'static str> {
    Setting::ALL
        .iter()
        .map(|setting| setting.name())
        .chain(AccountingSetting::ALL.iter().map(|setting| setting.name()))
        .collect()
}

/// The names the unknown-setting refusal lists as accepted.
fn listed() -> Vec<String> {
    let refusal = parse_operands(["no.such_setting=1"]).expect_err("an unknown name refuses");
    assert_eq!(refusal.cause, UsageCause::UnknownSetting);
    let text = refusal.to_string();
    let (_, list) = text
        .split_once("accepted settings:")
        .unwrap_or_else(|| panic!("the refusal lists the accepted settings: {text}"));
    list.split_whitespace().map(str::to_owned).collect()
}

/// FR-255-AC-3, AC-12: every accepted `--limit` name is listed, every listed
/// name is accepted at a valid value, and the two lists are equal.
#[test]
#[trace("FR-255-AC-3", "FR-255-AC-12")]
fn every_accepted_limit_name_is_listed_and_every_listed_name_is_accepted() {
    let accepted = accepted();
    let listed = listed();
    assert_eq!(
        listed,
        accepted
            .iter()
            .map(|name| (*name).to_owned())
            .collect::<Vec<_>>()
    );
    for name in &listed {
        let operand = format!("{name}=1");
        let parsed = parse_operands([operand.as_str()])
            .unwrap_or_else(|refusal| panic!("{name} is listed but refused: {refusal}"));
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].0.name(), name);
    }
}

/// FR-255-AC-5, AC-9: a name with no `=`, an unknown name, a value that is
/// not a non-negative integer and a repeated name each refuse with their
/// cause, naming the operand.
#[test]
#[trace("FR-255-AC-5", "FR-255-AC-9")]
fn every_limit_refusal_cause_is_reached() {
    for (operands, cause, operand) in [
        (vec!["s3.nodes"], UsageCause::NotAnAssignment, "s3.nodes"),
        (vec!["s9.nodes=1"], UsageCause::UnknownSetting, "s9.nodes=1"),
        (
            vec!["work_units=1"],
            UsageCause::UnknownSetting,
            "work_units=1",
        ),
        (
            vec!["s3.nodes=ten"],
            UsageCause::NotAnInteger,
            "s3.nodes=ten",
        ),
        (vec!["s3.nodes=-1"], UsageCause::NotAnInteger, "s3.nodes=-1"),
        (vec!["s3.nodes="], UsageCause::NotAnInteger, "s3.nodes="),
        (
            vec!["s3.nodes=5", "s3.nodes=6"],
            UsageCause::Repeated,
            "s3.nodes=6",
        ),
    ] {
        let refusal = parse_operands(operands.iter().copied()).expect_err("the operand refuses");
        assert_eq!(refusal.cause, cause, "{operands:?}");
        assert_eq!(refusal.operand, operand, "{operands:?}");
    }
}

/// FR-255-AC-26: the settings operation accepts and preserves every row's
/// maximum typed bound, and refuses the first integer above that bound.
#[test]
#[trace("FR-255-AC-26")]
fn settings_preserve_each_typed_maximum_and_refuse_values_above_it() {
    use fsl_foundation::setting::SettingLimits;
    use std::collections::BTreeMap;

    const U32_MAX: u64 = 4_294_967_295;
    const U64_MAX: u64 = 18_446_744_073_709_551_615;
    let expected_stage: BTreeMap<_, _> = Setting::ALL
        .iter()
        .copied()
        .map(|setting| {
            (
                setting,
                if setting == Setting::ModelSequenceMaximum {
                    U32_MAX
                } else {
                    U64_MAX
                },
            )
        })
        .collect();
    let operands: Vec<_> = Setting::ALL
        .iter()
        .copied()
        .map(|setting| format!("{}={}", setting.name(), expected_stage[&setting]))
        .chain(
            AccountingSetting::ALL
                .iter()
                .map(|setting| format!("{}={U64_MAX}", setting.name())),
        )
        .collect();
    let input: Vec<_> = operands.iter().map(String::as_str).collect();
    let limits = CallerLimits::from_operands(input).expect("each maximum is representable");
    assert_eq!(
        limits.bounds().into_iter().collect::<BTreeMap<_, _>>(),
        expected_stage
    );
    assert_eq!(
        limits.accounting.bounds(),
        AccountingSetting::ALL
            .into_iter()
            .map(|setting| (setting, U64_MAX))
            .collect::<Vec<_>>()
    );

    for setting in Setting::ALL.iter().copied() {
        let maximum = expected_stage[&setting];
        assert_eq!(setting.greatest_bound(), maximum, "{}", setting.name());
        let above = if maximum == U64_MAX {
            "18446744073709551616".to_owned()
        } else {
            (maximum + 1).to_string()
        };
        let operand = format!("{}={above}", setting.name());
        let refusal = parse_operands([operand.as_str()]).expect_err("the value exceeds its type");
        assert_eq!(refusal.cause, UsageCause::NotAnInteger, "{operand}");
        assert_eq!(refusal.operand, operand);
        let caller_refusal =
            CallerLimits::from_operands([operand.as_str()]).expect_err("caller limits refuse it");
        assert_eq!(caller_refusal.cause, UsageCause::NotAnInteger, "{operand}");
        assert_eq!(caller_refusal.operand, operand);
    }
    for setting in AccountingSetting::ALL {
        let operand = format!("{}=18446744073709551616", setting.name());
        let refusal = parse_operands([operand.as_str()]).expect_err("the value exceeds u64");
        assert_eq!(refusal.cause, UsageCause::NotAnInteger, "{operand}");
        assert_eq!(refusal.operand, operand);
        let caller_refusal =
            CallerLimits::from_operands([operand.as_str()]).expect_err("caller limits refuse it");
        assert_eq!(caller_refusal.cause, UsageCause::NotAnInteger, "{operand}");
        assert_eq!(caller_refusal.operand, operand);
    }
}

/// FR-255-AC-21: the public settings operation returns independent worker
/// counts, leaves engine bounds unchanged, and refuses invalid worker names.
#[test]
#[trace("FR-255-AC-21")]
fn worker_settings_change_only_their_own_counts() {
    let limits = CallerLimits::from_operands([
        "explore.workers=4",
        "model_check.workers=2",
        "statistical.workers=8",
    ])
    .expect("worker settings are accepted");
    assert_eq!(
        limits.workers,
        WorkerCounts {
            explore: 4,
            model_check: 2,
            statistical: 8,
        }
    );
    assert_eq!(limits.explore.max_states, 10_000_000);
    let model_check_only = CallerLimits::from_operands(["model_check.workers=2"])
        .expect("the model-check worker setting is accepted independently");
    assert_eq!(model_check_only.workers.model_check, 2);
    assert_eq!(model_check_only.explore.max_states, 10_000_000);
    let explore_only = CallerLimits::from_operands(["explore.workers=2"])
        .expect("the exploration worker setting is accepted independently");
    assert_eq!(explore_only.workers.explore, 2);
    assert_eq!(explore_only.explore.max_states, 10_000_000);
    assert!(limits
        .engines
        .bounds()
        .iter()
        .all(|(setting, _)| !setting.name().ends_with(".workers")));
    for (operand, workers) in [
        (
            "explore.workers=4",
            WorkerCounts {
                explore: 4,
                ..WorkerCounts::default()
            },
        ),
        (
            "model_check.workers=2",
            WorkerCounts {
                model_check: 2,
                ..WorkerCounts::default()
            },
        ),
        (
            "statistical.workers=8",
            WorkerCounts {
                statistical: 8,
                ..WorkerCounts::default()
            },
        ),
    ] {
        let actual =
            CallerLimits::from_operands([operand]).expect("a single worker setting is accepted");
        let mut expected = CallerLimits::default();
        expected.workers = workers;
        assert_eq!(actual, expected, "{operand}");
    }
    for name in [
        "explore.workers",
        "model_check.workers",
        "statistical.workers",
    ] {
        assert_eq!(Setting::from_name(name), None);
    }
    let zero = CallerLimits::from_operands(["explore.workers=0"]).expect_err("zero workers refuse");
    assert_eq!(zero.operand, "explore.workers=0");
    assert_eq!(zero.cause, UsageCause::InvalidWorkerValue);
    let unknown = CallerLimits::from_operands(["explore.threads=4"])
        .expect_err("unknown worker setting refuses");
    assert_eq!(unknown.operand, "explore.threads=4");
    assert_eq!(unknown.cause, UsageCause::UnknownSetting);
}

/// The in-process `version` invocation over `arguments`: exit code and
/// stdout document, or the refusal value.
fn version(arguments: &[&str]) -> (u8, Value) {
    let case = Case::new();
    let argv: Vec<&str> = std::iter::once("version")
        .chain(arguments.iter().copied())
        .collect();
    let path = case.json("argv.json", &json!({"argv": argv}));
    match fsl_command::command::run(&path, &crate::negotiation::Fr290) {
        Ok(result) => (result.exit_code, result.value.as_value().clone()),
        Err(error) => (
            error.category().exit_code(),
            error.value().expect("the refusal serializes"),
        ),
    }
}

/// FR-287: `version` accepts exactly `--format json` and `--kind`
/// with one of the five inventory lists, each alone and together; an
/// unknown option, a repeated option, an option without its value, a
/// format other than `json` and an unknown list each refuse, exit 20.
#[test]
#[trace("FR-287")]
fn version_accepts_exactly_its_documented_options() {
    let lists = ["editions", "profiles", "families", "features", "consumers"];
    let (exit, whole) = version(&[]);
    assert_eq!(exit, 0, "{whole}");
    assert_eq!(whole["format"], "filament.build-inventory/v1", "{whole}");
    let (exit, json_form) = version(&["--format", "json"]);
    assert_eq!(exit, 0, "{json_form}");
    assert_eq!(json_form, whole);
    for list in lists {
        for arguments in [
            vec!["--kind", list],
            vec!["--format", "json", "--kind", list],
            vec!["--kind", list, "--format", "json"],
        ] {
            let (exit, selected) = version(&arguments);
            assert_eq!(exit, 0, "{arguments:?}: {selected}");
            assert!(selected[list].is_array(), "{arguments:?}: {selected}");
            assert_eq!(selected[list], whole[list], "{arguments:?}");
        }
    }
    for arguments in [
        vec!["--color", "never"],
        vec!["--help"],
        vec!["--format"],
        vec!["--kind"],
        vec!["--format", "json", "--format", "json"],
        vec!["--kind", "profiles", "--kind", "editions"],
        vec!["--format", "text"],
        vec!["--kind", "widgets"],
    ] {
        let (exit, refusal) = version(&arguments);
        assert_eq!(exit, 20, "{arguments:?}: {refusal}");
    }
}
