// SPDX-License-Identifier: AGPL-3.0-or-later
//! The standard's published fixtures for the two source runners (QSL-2048),
//! and the rule that decides what a case that does not match may do.
//!
//! The standard is read from the checkout `FSPEC_DIR` names when that
//! variable is set, and from the pinned published package otherwise. A set
//! variable that names no directory is an error, never a fallback to the
//! pinned copy: a run that believes it reads one standard and reads another
//! is not evidence.
//!
//! A case that does not match its fixture, or that no source adapter can
//! execute, fails the run unless the checked-in registry in
//! [`crate::fspec_pending`] keys it to an open ticket. A keyed case that
//! starts to match fails the run too, so a fixed gap cannot stay on the list.

use std::collections::BTreeSet;
use std::ffi::OsStr;
use std::path::Path;

use crate::fspec_pending::{Pending, ALLOWED_TICKETS, PENDING, RUNNERS};

/// Reads `path` (relative to the standard's root) from `directory` when one
/// is named, otherwise from the pinned package.
pub(crate) fn read_from(directory: Option<&OsStr>, path: &str) -> Result<Vec<u8>, String> {
    let Some(directory) = directory.filter(|directory| !directory.is_empty()) else {
        return filament_specification::published_file(path)
            .map(<[u8]>::to_vec)
            .ok_or_else(|| format!("the pinned FSpec package does not publish {path}"));
    };
    let root = Path::new(directory);
    if !root.is_dir() {
        return Err(format!(
            "FSPEC_DIR={} is not a directory; unset it to read the pinned FSpec package",
            root.display()
        ));
    }
    std::fs::read(root.join(path))
        .map_err(|error| format!("FSPEC_DIR={} does not hold {path}: {error}", root.display()))
}

/// Reads `path` from the standard this run is configured to read.
pub(crate) fn read(path: &str) -> Result<Vec<u8>, String> {
    read_from(std::env::var_os("FSPEC_DIR").as_deref(), path)
}

/// Reads `path` as JSON.
pub(crate) fn json(path: &str) -> Result<serde_json::Value, String> {
    let bytes = read(path)?;
    serde_json::from_slice(&bytes).map_err(|error| format!("{path} is not JSON: {error}"))
}

/// What running one case against its fixture produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Settled {
    /// The compiler gave the fixture's answer.
    Matched,
    /// The compiler gave another answer; the text names expected and actual.
    Mismatch(String),
    /// No source adapter can execute the case; the text says why.
    Unsupported(String),
    /// The runner itself failed on the case (an adapter error, not an answer
    /// of the compiler). It can never be keyed: it always fails the run.
    Error(String),
}

/// The decision over a whole run.
#[derive(Debug, Default)]
pub(crate) struct Tally {
    /// Cases that matched, in run order.
    pub matched: Vec<String>,
    /// Cases keyed to a ticket that still do not match: id, key, reason.
    pub keyed: Vec<(String, &'static str, String)>,
    /// Everything that must fail the run.
    pub failures: Vec<String>,
}

/// Whether `key` is a Linear ticket key: capitals, a hyphen, digits.
pub(crate) fn is_ticket_key(key: &str) -> bool {
    key.split_once('-').is_some_and(|(team, number)| {
        !team.is_empty()
            && team.bytes().all(|byte| byte.is_ascii_uppercase())
            && !number.is_empty()
            && number.bytes().all(|byte| byte.is_ascii_digit())
    })
}

/// Failures of the registry itself: an unknown runner name (an entry no run
/// would ever check), a duplicate entry, a malformed or foreign key.
pub(crate) fn registry_failures(registry: &[Pending]) -> Vec<String> {
    let mut failures = Vec::new();
    let mut seen: BTreeSet<(&str, &str)> = BTreeSet::new();
    for entry in registry {
        if !RUNNERS.contains(&entry.runner) {
            failures.push(format!(
                "registry entry {} names the unknown runner {:?}; no run would check it",
                entry.case, entry.runner
            ));
        }
        if !seen.insert((entry.runner, entry.case)) {
            failures.push(format!(
                "registry keys {} of {} more than once",
                entry.case, entry.runner
            ));
        }
        if entry.pin.is_empty() {
            failures.push(format!(
                "registry entry {} ({}) pins no failure reason",
                entry.case, entry.ticket
            ));
        }
        if !ALLOWED_TICKETS.contains(&entry.ticket) {
            failures.push(format!(
                "{}: registry key {} for {} is not in the allowed set",
                entry.runner, entry.ticket, entry.case
            ));
        }
        if !is_ticket_key(entry.ticket) {
            failures.push(format!(
                "{}: registry key {:?} for {} is not a ticket key",
                entry.runner, entry.ticket, entry.case
            ));
        }
    }
    failures
}

/// Settles every case of `runner` against `registry`.
///
/// `cases` holds each case's id and what running it produced. The registry
/// must key only cases that exist, once each, under a known runner and a
/// well-formed ticket key. A key pins the reason the case fails: a keyed
/// case that fails with another reason, or whose runner errors, fails the
/// run.
pub(crate) fn settle(
    runner: &str,
    cases: Vec<(String, Settled)>,
    registry: &'static [Pending],
) -> Tally {
    let mut tally = Tally::default();
    tally.failures.extend(registry_failures(registry));
    let known: BTreeSet<&str> = cases.iter().map(|(id, _)| id.as_str()).collect();
    for entry in registry.iter().filter(|entry| entry.runner == runner) {
        if !known.contains(entry.case) {
            tally.failures.push(format!(
                "{runner}: registry keys {} ({}), which is no case of the fixtures",
                entry.case, entry.ticket
            ));
        }
    }
    for (id, settled) in cases {
        let entry = registry
            .iter()
            .find(|entry| entry.runner == runner && entry.case == id);
        match (settled, entry) {
            (Settled::Matched, None) => tally.matched.push(id),
            (Settled::Matched, Some(entry)) => tally.failures.push(format!(
                "{runner}: {id} now matches its fixture; remove its registry key {}",
                entry.ticket
            )),
            (Settled::Error(reason), _) => tally
                .failures
                .push(format!("{runner}: {id} runner error: {reason}")),
            (Settled::Mismatch(reason) | Settled::Unsupported(reason), Some(entry)) => {
                if reason == entry.pin {
                    tally.keyed.push((id, entry.ticket, reason));
                } else {
                    tally.failures.push(format!(
                        "{runner}: {id} (keyed {}) fails differently than pinned: pinned {:?}, now {reason:?}",
                        entry.ticket, entry.pin
                    ));
                }
            }
            (Settled::Mismatch(reason), None) => {
                tally
                    .failures
                    .push(format!("{runner}: {id} mismatch, no ticket key: {reason}"));
            }
            (Settled::Unsupported(reason), None) => tally.failures.push(format!(
                "{runner}: {id} not executed and no ticket key: {reason}"
            )),
        }
    }
    tally
}

/// Prints the keyed cases and returns the failure text, if any.
pub(crate) fn report(runner: &str, tally: &Tally) -> Result<(), String> {
    for (id, key, reason) in &tally.keyed {
        println!("pending ({key}): {runner}: {id}: {reason}");
    }
    if tally.failures.is_empty() {
        Ok(())
    } else {
        Err(tally.failures.join("\n"))
    }
}

/// The registry this build checks in.
pub(crate) fn registry() -> &'static [Pending] {
    PENDING
}

#[cfg(test)]
mod tests {
    use ix_trace_rs::trace;

    use super::{is_ticket_key, registry_failures, settle, Settled};
    use crate::fspec_pending::{Pending, ALLOWED_TICKETS};

    const RUNNER: &str = "state-core";

    const fn entry(case: &'static str, ticket: &'static str, pin: &'static str) -> Pending {
        Pending {
            runner: RUNNER,
            case,
            ticket,
            pin,
        }
    }

    fn mismatch(text: &str) -> Settled {
        Settled::Mismatch(text.to_owned())
    }

    // AC gap: QSL-3074. FR-350 has no AC for the key registry; the tests of
    // this module carry the requirement id only.

    /// Provenance: QSL-2048. A mismatch needs a ticket key, a fixed case
    /// loses it, and an unsupported case needs one too.
    #[trace("FR-350")]
    #[test]
    fn a_mismatch_needs_a_ticket_key_and_a_fixed_case_loses_it() {
        static KEYED: &[Pending] = &[entry("A", "QSL-3027", "wrong"), entry("B", "QSL-3028", "x")];
        let cases = vec![
            ("A".to_owned(), mismatch("wrong")),
            ("B".to_owned(), Settled::Matched),
            (
                "C".to_owned(),
                Settled::Unsupported("no adapter".to_owned()),
            ),
            ("D".to_owned(), mismatch("wrong")),
        ];
        let tally = settle(RUNNER, cases, KEYED);
        assert_eq!(tally.keyed.len(), 1);
        assert_eq!(tally.keyed[0].1, "QSL-3027");
        assert_eq!(tally.failures.len(), 3, "{:?}", tally.failures);
        for needle in ["B now matches", "C not executed", "D mismatch"] {
            assert!(
                tally.failures.iter().any(|text| text.contains(needle)),
                "{needle}: {:?}",
                tally.failures
            );
        }
        assert!(is_ticket_key("QSL-2048"));
        assert!(!is_ticket_key("2048"));
        assert!(!is_ticket_key("QSL-"));
        assert!(ALLOWED_TICKETS.contains(&"QSL-3035"));
    }

    /// Provenance: QSL-2048. A key outside the allowed set, a malformed key
    /// and a key naming no case each fail the run.
    #[trace("FR-350")]
    #[test]
    fn a_stray_malformed_or_foreign_key_fails_the_run() {
        static STRAY: &[Pending] = &[entry("Z", "bad", "p")];
        let tally = settle(RUNNER, Vec::new(), STRAY);
        assert_eq!(tally.failures.len(), 3, "{:?}", tally.failures);
        static FOREIGN: &[Pending] = &[entry("A", "QSL-9999", "x")];
        let tally = settle(RUNNER, vec![("A".to_owned(), mismatch("x"))], FOREIGN);
        assert!(tally
            .failures
            .iter()
            .any(|text| text.contains("not in the allowed set")));
    }

    /// Provenance: QSL-2048. A keyed case that fails differently than its pin
    /// fails the run, for a mismatch and for an unsupported reason alike.
    #[trace("FR-350")]
    #[test]
    fn a_keyed_case_that_fails_differently_than_pinned_fails_the_run() {
        static KEYED: &[Pending] = &[entry("A", "QSL-3027", "expected X, actual Y")];
        let same = settle(
            RUNNER,
            vec![("A".to_owned(), mismatch("expected X, actual Y"))],
            KEYED,
        );
        assert!(same.failures.is_empty(), "{:?}", same.failures);
        assert_eq!(same.keyed.len(), 1);
        let other = settle(
            RUNNER,
            vec![("A".to_owned(), mismatch("expected X, actual Z"))],
            KEYED,
        );
        assert_eq!(other.failures.len(), 1, "{:?}", other.failures);
        assert!(other.failures[0].contains("differently than pinned"));
        let unsupported = settle(
            RUNNER,
            vec![(
                "A".to_owned(),
                Settled::Unsupported("expected X, actual Y".to_owned()),
            )],
            KEYED,
        );
        assert!(
            unsupported.failures.is_empty(),
            "{:?}",
            unsupported.failures
        );
        let changed = settle(
            RUNNER,
            vec![(
                "A".to_owned(),
                Settled::Unsupported("new reason".to_owned()),
            )],
            KEYED,
        );
        assert_eq!(changed.failures.len(), 1);
    }

    /// Provenance: QSL-2048. A runner error on a keyed case fails the run: an
    /// error is never an answer a key can pin.
    #[trace("FR-350")]
    #[test]
    fn a_runner_error_on_a_keyed_case_fails_the_run() {
        static KEYED: &[Pending] = &[entry("A", "QSL-3027", "boom")];
        let tally = settle(
            RUNNER,
            vec![("A".to_owned(), Settled::Error("boom".to_owned()))],
            KEYED,
        );
        assert_eq!(tally.failures.len(), 1, "{:?}", tally.failures);
        assert!(tally.failures[0].contains("runner error"));
        assert!(tally.keyed.is_empty());
    }

    /// Provenance: QSL-2048. An entry with an unknown runner name, a
    /// duplicate (runner, case) entry and an entry with no pin each fail.
    #[trace("FR-350")]
    #[test]
    fn the_registry_rejects_unknown_runners_duplicates_and_empty_pins() {
        static UNKNOWN: &[Pending] = &[Pending {
            runner: "state_core",
            case: "A",
            ticket: "QSL-3027",
            pin: "p",
        }];
        let failures = registry_failures(UNKNOWN);
        assert!(
            failures.iter().any(|text| text.contains("unknown runner")),
            "{failures:?}"
        );
        // The entry is dead for both real runners; each run still reports it.
        let tally = settle(RUNNER, Vec::new(), UNKNOWN);
        assert!(tally
            .failures
            .iter()
            .any(|text| text.contains("unknown runner")));
        static DUPLICATE: &[Pending] = &[entry("A", "QSL-3027", "p"), entry("A", "QSL-3028", "q")];
        let failures = registry_failures(DUPLICATE);
        assert!(
            failures.iter().any(|text| text.contains("more than once")),
            "{failures:?}"
        );
        static UNPINNED: &[Pending] = &[entry("A", "QSL-3027", "")];
        assert!(registry_failures(UNPINNED)
            .iter()
            .any(|text| text.contains("pins no failure reason")));
        assert!(registry_failures(crate::fspec_pending::PENDING).is_empty());
    }
}
