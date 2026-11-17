// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-641, FSpec FR-352: the `filament.progress-ledger/1` form of a progress
//! ledger, the file a request's `ledger` member names and the `ledger` member
//! of a result. One closed object whose entries are in ascending binding
//! order, so writing a read ledger reproduces its bytes.

use serde::{Deserialize, Serialize};

use super::progress::{
    LedgerBinding, ObservedPosition, ProgressAssertion, ProgressClosure, ProgressCompleteness,
    ProgressLedger, ProgressLedgerEntry,
};

/// The ledger document's `format` member.
pub const LEDGER_FORMAT: &str = "filament.progress-ledger/1";

/// A ledger document that cannot be read.
#[derive(Debug, thiserror::Error)]
#[error("{member}: {reason}")]
pub struct LedgerReadError {
    /// The member at fault.
    pub member: String,
    /// What is wrong with it.
    pub reason: String,
}

fn invalid(member: impl Into<String>, reason: impl Into<String>) -> LedgerReadError {
    LedgerReadError {
        member: member.into(),
        reason: reason.into(),
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Document {
    format: String,
    entries: Vec<Entry>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Entry {
    binding: Binding,
    watermark: u64,
    completeness: Completeness,
    scope: Closure,
    execution: Closure,
    covered: Vec<Covered>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Binding {
    package: String,
    clause: String,
    occurrence: u64,
    clock: String,
    profile: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Completeness {
    Complete { through: u64 },
    Incomplete,
}

#[derive(Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Closure {
    Closed,
    Open,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Covered {
    identity: String,
    stamp: u64,
    /// FR-993: present on a record-sample entry only, and never `null`.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present"
    )]
    fingerprint: Option<String>,
}

/// A member that, when written, is a string.
fn present<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<Option<String>, D::Error> {
    String::deserialize(deserializer).map(Some)
}

/// Whether `identity`, a covered entry's ledger identity, names a record
/// sample: its authority is FR-641's `record-sample`.
fn names_a_sample(identity: &str) -> bool {
    let authority = super::temporal::SAMPLE_AUTHORITY;
    identity
        .strip_prefix(&format!("{}:", authority.len()))
        .is_some_and(|rest| rest.starts_with(authority))
}

/// Whether `fingerprint` is FR-993's form: `sha256:` and 64 lowercase
/// hexadecimal digits.
fn is_fingerprint(fingerprint: &str) -> bool {
    fingerprint.strip_prefix("sha256:").is_some_and(|digits| {
        digits.len() == 64
            && digits
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    })
}

impl From<Closure> for ProgressClosure {
    fn from(closure: Closure) -> Self {
        match closure {
            Closure::Closed => Self::Closed,
            Closure::Open => Self::Open,
        }
    }
}

impl From<ProgressClosure> for Closure {
    fn from(closure: ProgressClosure) -> Self {
        match closure {
            ProgressClosure::Closed => Self::Closed,
            ProgressClosure::Open => Self::Open,
        }
    }
}

/// The ledger as its document value.
///
/// # Errors
///
/// A clause identity that has no text spelling.
pub fn write_ledger(ledger: &ProgressLedger) -> Result<serde_json::Value, LedgerReadError> {
    let entries = ledger
        .entries()
        .map(|(binding, entry)| {
            let clause = fsl_foundation::node_wire::node_key_to_text(&binding.clause)
                .map_err(|_| invalid("entries.binding.clause", "has no text spelling"))?;
            Ok(Entry {
                binding: Binding {
                    package: binding.package.clone(),
                    clause,
                    occurrence: binding.occurrence,
                    clock: binding.clock.clone(),
                    profile: binding.profile.clone(),
                },
                watermark: entry.watermark(),
                completeness: match entry.completeness() {
                    ProgressCompleteness::Complete { through } => {
                        Completeness::Complete { through }
                    }
                    ProgressCompleteness::Incomplete => Completeness::Incomplete,
                },
                scope: entry.scope().into(),
                execution: entry.execution().into(),
                covered: entry
                    .covered()
                    .map(|position| Covered {
                        identity: position.identity,
                        stamp: position.stamp,
                        fingerprint: position.fingerprint,
                    })
                    .collect(),
            })
        })
        .collect::<Result<Vec<_>, LedgerReadError>>()?;
    serde_json::to_value(Document {
        format: LEDGER_FORMAT.to_owned(),
        entries,
    })
    .map_err(|error| invalid("ledger", error.to_string()))
}

/// Read a ledger document.
///
/// # Errors
///
/// A document that is not a closed `filament.progress-ledger/1` object, whose
/// entries are out of binding order or repeat a binding, or whose `covered`
/// positions are out of order.
pub fn read_ledger(bytes: &[u8]) -> Result<ProgressLedger, LedgerReadError> {
    // The canonical reader refuses duplicate members before serde sees them.
    quire_canonical::read(bytes, u64::MAX).map_err(|error| invalid("ledger", error.to_string()))?;
    let document: Document =
        serde_json::from_slice(bytes).map_err(|error| invalid("ledger", error.to_string()))?;
    if document.format != LEDGER_FORMAT {
        return Err(invalid("ledger.format", document.format));
    }
    let mut entries: Vec<(LedgerBinding, ProgressLedgerEntry)> = Vec::new();
    for (index, entry) in document.entries.into_iter().enumerate() {
        let member = |name: &str| format!("ledger.entries[{index}].{name}");
        let clause = fsl_foundation::node_wire::node_key_from_text(&entry.binding.clause)
            .map_err(|_| invalid(member("binding.clause"), "is not a node key"))?;
        let binding = LedgerBinding {
            package: entry.binding.package,
            clause,
            occurrence: entry.binding.occurrence,
            clock: entry.binding.clock,
            profile: entry.binding.profile,
        };
        if let Some((last, _)) = entries.last() {
            if *last >= binding {
                return Err(invalid(
                    member("binding"),
                    "entries are in ascending binding order with no binding repeated",
                ));
            }
        }
        let mut covered = Vec::with_capacity(entry.covered.len());
        for (at, position) in entry.covered.into_iter().enumerate() {
            // FR-641, FR-993: a record-sample entry carries its fingerprint,
            // and no other entry does.
            let sample = names_a_sample(&position.identity);
            match (&position.fingerprint, sample) {
                (Some(fingerprint), true) if is_fingerprint(fingerprint) => {}
                (None, false) => {}
                (Some(_), true) => {
                    return Err(invalid(
                        member(&format!("covered[{at}].fingerprint")),
                        "is `sha256:` and 64 lowercase hexadecimal digits",
                    ))
                }
                (None, true) => {
                    return Err(invalid(
                        member(&format!("covered[{at}]")),
                        "a record-sample entry carries its fingerprint",
                    ))
                }
                (Some(_), false) => {
                    return Err(invalid(
                        member(&format!("covered[{at}].fingerprint")),
                        "only a record-sample entry carries a fingerprint",
                    ))
                }
            }
            covered.push(ObservedPosition {
                identity: position.identity,
                stamp: position.stamp,
                fingerprint: position.fingerprint,
            });
        }
        fn key(position: &ObservedPosition) -> (&str, u64) {
            (position.identity.as_str(), position.stamp)
        }
        if covered
            .windows(2)
            .any(|pair| key(&pair[0]) >= key(&pair[1]))
        {
            return Err(invalid(
                member("covered"),
                "positions are in identity-then-stamp order with none repeated",
            ));
        }
        let assertion = ProgressAssertion {
            watermark: entry.watermark,
            completeness: match entry.completeness {
                Completeness::Complete { through } => ProgressCompleteness::Complete { through },
                Completeness::Incomplete => ProgressCompleteness::Incomplete,
            },
            scope: entry.scope.into(),
            execution: entry.execution.into(),
        };
        entries.push((binding, ProgressLedgerEntry::new(assertion, covered)));
    }
    Ok(ProgressLedger::from_entries(entries))
}
