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
/// name is accepted at value 0, and the two lists are equal.
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
        let operand = format!("{name}=0");
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

/// FR-661-AC-1, AC-3: `version` accepts exactly `--format json` and `--kind`
/// with one of the five inventory lists, each alone and together; an
/// unknown option, a repeated option, an option without its value, a
/// format other than `json` and an unknown list each refuse, exit 20.
#[test]
#[trace("FR-661-AC-1", "FR-661-AC-3")]
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
