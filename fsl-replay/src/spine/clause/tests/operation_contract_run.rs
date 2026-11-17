// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-1908 (FR-702): the `frame` run of an operation over one invocation also
//! evaluates the domain operation's own `requires` (at the pre snapshot) and
//! `ensures` (over pre and post), and reports a false clause as a violation
//! naming it. The invocation is the frame tests' changed-version one:
//! `child.versionNumber` moves from 2 to 3 inside `attemptUpdate`'s frame.

use super::frame::{frame_input, object, run};
use super::*;
use filament_semantic_value::catalog::Category;

/// Sets the `pre` and `post` lists of the `attemptUpdate` operation found
/// anywhere under `value` to `requires` and `ensures` as `quire` clauses.
fn set_contract(value: &mut serde_json::Value, requires: &[&str], ensures: &[&str]) -> bool {
    match value {
        serde_json::Value::Object(map) => {
            if map.get("name").and_then(serde_json::Value::as_str) == Some("attemptUpdate")
                && map.contains_key("frame")
            {
                let origin = map
                    .get("origin")
                    .cloned()
                    .unwrap_or(serde_json::Value::Null);
                let clauses = |texts: &[&str]| -> serde_json::Value {
                    texts
                        .iter()
                        .map(|text| json!({"language": "quire", "text": text, "origin": origin.clone()}))
                        .collect()
                };
                map.insert("pre".to_owned(), clauses(requires));
                map.insert("post".to_owned(), clauses(ensures));
                return true;
            }
            map.values_mut()
                .any(|child| set_contract(child, requires, ensures))
        }
        serde_json::Value::Array(items) => items
            .iter_mut()
            .any(|child| set_contract(child, requires, ensures)),
        _ => false,
    }
}

/// The config-version request over a domain package whose `attemptUpdate`
/// carries `requires` and `ensures`.
fn contract_request(
    requires: &[&str],
    ensures: &[&str],
    selection: ClauseRunSelection,
) -> ClauseRunRequest {
    let mut document: serde_json::Value =
        serde_json::from_slice(&config_version_domain_document()).expect("the fixture is JSON");
    assert!(
        set_contract(&mut document, requires, ensures),
        "attemptUpdate is in the fixture"
    );
    let (unit, packages) = config_version_unit_and_packages_for(document.to_string().into_bytes());
    config_version_request_for(unit, packages, "operation-contract.native", selection)
}

fn ensures_unchanged(selection: ClauseRunSelection) -> ClauseRunRequest {
    contract_request(
        &[],
        &["self.versionNumber = pre(self.versionNumber)"],
        selection,
    )
}

fn ensures_increased(selection: ClauseRunSelection) -> ClauseRunRequest {
    contract_request(
        &[],
        &["pre(self.versionNumber) < self.versionNumber"],
        selection,
    )
}

fn requires_above_five(selection: ClauseRunSelection) -> ClauseRunRequest {
    contract_request(
        &["self.versionNumber > 5"],
        &["self.versionNumber = pre(self.versionNumber)"],
        selection,
    )
}

fn requires_below_five(selection: ClauseRunSelection) -> ClauseRunRequest {
    contract_request(
        &["self.versionNumber < 5"],
        &["self.versionNumber = pre(self.versionNumber)"],
        selection,
    )
}

/// The changed-version `attemptUpdate` run over `builder`'s package.
fn changed_version_run(
    builder: fn(ClauseRunSelection) -> ClauseRunRequest,
) -> super::super::ClauseRunReport {
    run(frame_input(
        builder,
        "attemptUpdate",
        &[object("root", 1, None), object("child", 2, Some("root"))],
        &[object("root", 1, None), object("child", 3, Some("root"))],
        |_| {},
    ))
}

/// The report is a stage-`evaluate` violation, `truth: false`, exit 10, whose
/// disposition names `kind` (`requires` or `ensures`).
fn assert_contract_violation(report: &super::super::ClauseRunReport, kind: &str) {
    let disposition = &report.disposition;
    assert_eq!(
        disposition.stage(),
        ClauseRunStage::Evaluate,
        "{disposition:?}"
    );
    assert_eq!(
        disposition.category(),
        Category::Violation,
        "{disposition:?}"
    );
    assert_eq!(disposition.truth(), Some(false), "{disposition:?}");
    assert_eq!(disposition.category().exit_code(), 10);
    let text = format!("{disposition:?}").to_lowercase();
    assert!(
        text.contains(kind),
        "the violation names `{kind}`: {disposition:?}"
    );
    assert!(
        text.contains("attemptupdate"),
        "the violation names the operation: {disposition:?}"
    );
}

/// TC-1908 step 1 (FR-702-AC-1): a false `ensures` is an evaluate-stage
/// violation naming the operation contract.
#[trace("TC-1908", "FR-702-AC-1")]
#[test]
fn a_false_ensures_is_a_violation() {
    assert_contract_violation(&changed_version_run(ensures_unchanged), "ensures");
}

/// TC-1908 step 1 (FR-702-AC-1; W17): a true `ensures` that says the version
/// increased leaves the run successful.
#[trace("TC-1908", "FR-702-AC-1")]
#[test]
fn a_true_ensures_succeeds_when_post_exceeds_pre() {
    let report = changed_version_run(ensures_increased);
    assert_eq!(
        report.disposition.category(),
        Category::Success,
        "{:?}",
        report.disposition
    );
    assert_eq!(report.disposition.truth(), Some(true));
}

/// TC-1908 step 2 (FR-702-AC-2): a false `requires` is reported before any
/// `ensures`; a true `requires` lets the run reach the false `ensures`.
#[trace("TC-1908", "FR-702-AC-2")]
#[test]
fn a_false_requires_is_reported_before_any_ensures() {
    assert_contract_violation(&changed_version_run(requires_above_five), "requires");
    assert_contract_violation(&changed_version_run(requires_below_five), "ensures");
}
