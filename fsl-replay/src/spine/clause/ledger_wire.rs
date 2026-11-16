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
                        identity: position.identity.clone(),
                        stamp: position.stamp,
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
        let covered = entry
            .covered
            .into_iter()
            .map(|position| ObservedPosition {
                identity: position.identity,
                stamp: position.stamp,
            })
            .collect::<Vec<_>>();
        if covered.windows(2).any(|pair| pair[0] >= pair[1]) {
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
