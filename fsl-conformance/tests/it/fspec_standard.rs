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

use crate::fspec_pending::{Pending, ALLOWED_TICKETS, PENDING};

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

/// Settles every case of `runner` against `registry`.
///
/// `cases` holds each case's id and what running it produced. The registry
/// must key only cases that exist, with a well-formed ticket key.
pub(crate) fn settle(
    runner: &str,
    cases: Vec<(String, Settled)>,
    registry: &'static [Pending],
) -> Tally {
    let mut tally = Tally::default();
    let known: BTreeSet<&str> = cases.iter().map(|(id, _)| id.as_str()).collect();
    for entry in registry.iter().filter(|entry| entry.runner == runner) {
        if !ALLOWED_TICKETS.contains(&entry.ticket) {
            tally.failures.push(format!(
                "{runner}: registry key {} for {} is not in the allowed set",
                entry.ticket, entry.case
            ));
        }
        if !is_ticket_key(entry.ticket) {
            tally.failures.push(format!(
                "{runner}: registry key {:?} for {} is not a ticket key",
                entry.ticket, entry.case
            ));
        }
        if !known.contains(entry.case) {
            tally.failures.push(format!(
                "{runner}: registry keys {} ({}), which is no case of the fixtures",
                entry.case, entry.ticket
            ));
        }
    }
    for (id, settled) in cases {
        let key = registry
            .iter()
            .find(|entry| entry.runner == runner && entry.case == id)
            .map(|entry| entry.ticket);
        match (settled, key) {
            (Settled::Matched, None) => tally.matched.push(id),
            (Settled::Matched, Some(key)) => tally.failures.push(format!(
                "{runner}: {id} now matches its fixture; remove its registry key {key}"
            )),
            (Settled::Mismatch(reason) | Settled::Unsupported(reason), Some(key)) => {
                tally.keyed.push((id, key, reason));
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
