// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-010-AC-8: stable native codes round-trip and match their documentation.

use fsl_foundation::diagnostic::Code;
use std::collections::BTreeSet;

const DOCUMENTATION: &str = include_str!("../../docs/native-error-codes.md");
const CST_DIAGNOSTICS: &str = include_str!("../../fsl-cst/src/diagnostic.rs");

/// FR-010-AC-8: every stable code has exactly one documented catalogue row,
/// and documentation cannot retain a code that the Rust catalogue removed.
#[test]
fn documented_rows_match_the_code_catalogue_in_both_directions() {
    let documented = DOCUMENTATION
        .lines()
        .filter_map(|line| line.strip_prefix("| "))
        .filter_map(|row| row.split_once(" |"))
        .map(|(code, _)| code)
        .filter(|code| *code != "Code" && *code != "---")
        .collect::<BTreeSet<_>>();
    let implemented = Code::all()
        .iter()
        .map(|code| code.as_str())
        .collect::<BTreeSet<_>>();

    assert_eq!(
        documented, implemented,
        "documented and implemented codes differ"
    );
}

/// FR-010-AC-8: each code in the complete vocabulary resolves to itself and
/// has a defined outcome category.
#[test]
fn every_code_round_trips_through_the_catalogue() {
    for code in Code::all() {
        assert_eq!(Code::from_code(code.as_str()), Some(*code), "{code}");
        let _ = code.category();
    }
}

/// FR-010-AC-8: every typed complete-source cause has a case in the
/// diagnostic mapping test, including each syntax cause used by recovery.
#[test]
fn every_complete_cause_appears_in_a_diagnostic_test() {
    let variants = CST_DIAGNOSTICS
        .split_once("pub enum CompleteCause {")
        .and_then(|(_, rest)| rest.split_once("\n}"))
        .map(|(body, _)| {
            body.lines()
                .filter_map(|line| {
                    let line = line.trim();
                    let first = line.split_whitespace().next()?;
                    let name = first.split('(').next()?.trim_end_matches(',');
                    name.chars()
                        .next()
                        .filter(|character| character.is_ascii_uppercase())
                        .map(|_| name)
                })
                .collect::<Vec<_>>()
        })
        .expect("CompleteCause enum is present");
    let diagnostic_tests = CST_DIAGNOSTICS
        .split_once("#[cfg(test)]\nmod tests")
        .map(|(_, tests)| tests)
        .expect("diagnostic tests are present");

    for variant in variants {
        assert!(
            diagnostic_tests.contains(&format!("CompleteCause::{variant}")),
            "CompleteCause::{variant} has no diagnostic test case"
        );
    }
}
