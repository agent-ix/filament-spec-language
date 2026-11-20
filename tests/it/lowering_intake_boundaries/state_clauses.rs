// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::support::config_version_spine::{Case, spine};
use fsl_replay::spine::{ClauseRunSelection, ClauseRunSource, ClauseRunStage, run_clause};
use ix_trace_rs::trace;

fn custom_post(case: Case, body: &str) -> fsl_replay::spine::ClauseRunReport {
    let directory = tempfile::tempdir().unwrap();
    spine::write(directory.path(), case).unwrap();
    let mut request = spine::request(directory.path(), case).unwrap();
    let ClauseRunSource::Program { bytes, .. } = &mut request.source else {
        panic!("the native fixture has program source");
    };
    bytes.extend_from_slice(
        format!(
            "\npost CrossSnapshot using v on Config::ConfigVersion::attemptUpdate {{ {body} }}\n"
        )
        .as_bytes(),
    );
    match &mut request.selection {
        ClauseRunSelection::Clause(selection)
        | ClauseRunSelection::ClauseWithPath { selection, .. } => {
            selection.name = "CrossSnapshot".to_owned();
        }
        other => panic!("the invocation fixture selects a clause: {other:?}"),
    }
    run_clause(request).expect("a valid source and observation request reports its outcome")
}

#[test]
#[trace("FR-104-AC-1", "FR-107-AC-2")]
fn selecting_a_new_postcondition_reads_pre_and_post_from_their_distinct_snapshots() {
    for (case, expected) in [
        (Case::Unchanged, false),
        (Case::Advanced, true),
        (Case::Changed, true),
    ] {
        let report = custom_post(case, "self.versionNumber > pre(self.versionNumber)");
        assert_eq!(
            report.disposition.stage(),
            ClauseRunStage::Evaluate,
            "{}: {:?}",
            case.id(),
            report.disposition
        );
        assert_eq!(
            report.disposition.truth(),
            Some(expected),
            "{}: {:?}",
            case.id(),
            report.disposition
        );
    }
}

#[test]
#[trace("FR-107-AC-2")]
fn pre_selects_the_whole_guarded_parent_expression_before_a_changed_invocation() {
    let report = custom_post(
        Case::Changed,
        "pre(present(self.parent) implies deref(value(self.parent)).versionNumber = 1)",
    );
    assert_eq!(
        report.disposition.stage(),
        ClauseRunStage::Evaluate,
        "{:?}",
        report.disposition
    );
    assert_eq!(
        report.disposition.truth(),
        Some(true),
        "{:?}",
        report.disposition
    );
    // The same changed invocation would violate the original VersionAdvances
    // postcondition. Selecting CrossSnapshot must evaluate its actual body.
    let false_report = custom_post(
        Case::Changed,
        "self.versionNumber = pre(self.versionNumber)",
    );
    assert_eq!(
        false_report.disposition.stage(),
        ClauseRunStage::Evaluate,
        "{:?}",
        false_report.disposition
    );
    assert_eq!(
        false_report.disposition.truth(),
        Some(false),
        "{:?}",
        false_report.disposition
    );
}
