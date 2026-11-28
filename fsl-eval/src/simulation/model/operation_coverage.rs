// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-752 operation coverage for the model-based test generator.

use std::collections::BTreeMap;

use serde::{ser::SerializeMap, Deserialize, Deserializer, Serialize, Serializer};

/// A declared operation in the model's canonical declaration order.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelOperation {
    /// The operation declaration identity carried by model transitions.
    pub identity: String,
    /// The operation's declared name.
    pub name: String,
}

/// One evaluated model-based case and the operations it applied.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelOperationCase {
    /// The stable, zero-based case number in the generated result.
    pub case_index: u64,
    /// Operation declaration identities applied by this case, in step order.
    pub operations: Vec<String>,
}

/// Resource limits that can stop operation-reachability exploration.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ModelOperationCoverageLimit {
    /// Reachability exploration exhausted its state budget.
    States,
    /// Reachability exploration exhausted its transition budget.
    Transitions,
    /// Model expansion exhausted its candidate budget.
    Candidates,
    /// Reachability exploration was cancelled by its caller.
    Cancelled,
    /// A scalar accounting counter stopped generation before exploration.
    Accounting(filament_exact::LimitKind),
}

impl Serialize for ModelOperationCoverageLimit {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(match self {
            Self::States => "states",
            Self::Transitions => "transitions",
            Self::Candidates => "candidates",
            Self::Cancelled => "cancelled",
            Self::Accounting(counter) => counter.as_str(),
        })
    }
}

impl<'de> Deserialize<'de> for ModelOperationCoverageLimit {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let name = String::deserialize(deserializer)?;
        match name.as_str() {
            "states" => Ok(Self::States),
            "transitions" => Ok(Self::Transitions),
            "candidates" => Ok(Self::Candidates),
            "cancelled" => Ok(Self::Cancelled),
            other => filament_exact::LimitKind::from_code(other)
                .map(Self::Accounting)
                .ok_or_else(|| serde::de::Error::custom(format!("unknown limit `{other}`"))),
        }
    }
}

/// Reachability information for one declared operation, obtained from the
/// generator's exploration of the model graph.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ModelOperationCoverageReason {
    /// Exhaustive reachability proved no state enables the operation.
    Unreachable,
    /// No state within the trace bound enables the operation, but exploration
    /// did not exhaust the reachable graph beyond that horizon.
    UnreachableWithinBound {
        /// Request bounds that limited the completed reachability search,
        /// in FR-190 order.
        bounds: Vec<String>,
    },
    /// An enabling state was reached, but no evaluated case applied the
    /// operation. The path is the shortest invocation prefix to that state.
    NotGenerated {
        /// Shortest invocation prefix from an initial state to an enabling
        /// state. Each entry is a canonical FR-181 transition identity text,
        /// retaining the receiver and arguments as well as the operation.
        enabled_at: Vec<String>,
    },
    /// Exploration stopped before deciding operation reachability.
    Undetermined {
        /// The exploration limit that stopped the decision.
        limit: ModelOperationCoverageLimit,
        /// The configured limit value used by this exploration.
        value: u128,
    },
}

/// Whether an operation was applied by any evaluated, non-discarded case.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ModelOperationCoverageStatus {
    /// A passing or failing evaluated case applied the operation.
    Covered {
        /// The first evaluated case that applied this operation.
        first_case: u64,
    },
    /// No evaluated case applied the operation.
    #[serde(rename = "uncovered")]
    NotCovered {
        /// Why coverage could not be established.
        reason: ModelOperationCoverageReason,
    },
}

/// Coverage for one declared operation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelOperationCoverageEntry {
    /// The operation this entry describes.
    pub operation: ModelOperation,
    /// Its coverage status and, when uncovered, the reachability reason.
    pub status: ModelOperationCoverageStatus,
}

impl Serialize for ModelOperationCoverageEntry {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut entry = serializer.serialize_map(None)?;
        entry.serialize_entry("operation", &self.operation)?;
        match &self.status {
            ModelOperationCoverageStatus::Covered { first_case } => {
                entry.serialize_entry("status", "covered")?;
                entry.serialize_entry("first_case", first_case)?;
            }
            ModelOperationCoverageStatus::NotCovered { reason } => {
                entry.serialize_entry("status", "uncovered")?;
                match reason {
                    ModelOperationCoverageReason::Unreachable => {
                        entry.serialize_entry("reason", "unreachable")?;
                    }
                    ModelOperationCoverageReason::UnreachableWithinBound { bounds } => {
                        entry.serialize_entry("reason", "unreachable-within-bound")?;
                        entry.serialize_entry("bounds", bounds)?;
                    }
                    ModelOperationCoverageReason::NotGenerated { enabled_at } => {
                        entry.serialize_entry("reason", "not-generated")?;
                        entry.serialize_entry("enabled_at", enabled_at)?;
                    }
                    ModelOperationCoverageReason::Undetermined { limit, value } => {
                        entry.serialize_entry("reason", "undetermined")?;
                        entry.serialize_entry("limit", limit)?;
                        entry.serialize_entry("value", value)?;
                        if let ModelOperationCoverageLimit::Accounting(counter) = limit {
                            entry.serialize_entry(
                                "setting",
                                &format!("accounting.{}", counter.as_str()),
                            )?;
                        }
                    }
                }
            }
        }
        entry.end()
    }
}

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum EntryStatus {
    Covered,
    Uncovered,
}

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum EntryReason {
    Unreachable,
    UnreachableWithinBound,
    NotGenerated,
    Undetermined,
}

/// The members of one coverage entry, each omitted when the entry's form has
/// no use for it.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EntryWire {
    operation: ModelOperation,
    status: EntryStatus,
    #[serde(
        default,
        deserialize_with = "fsl_foundation::serde_object::omitted_not_null"
    )]
    first_case: Option<u64>,
    #[serde(
        default,
        deserialize_with = "fsl_foundation::serde_object::omitted_not_null"
    )]
    reason: Option<EntryReason>,
    #[serde(
        default,
        deserialize_with = "fsl_foundation::serde_object::omitted_not_null"
    )]
    bounds: Option<Vec<String>>,
    #[serde(
        default,
        deserialize_with = "fsl_foundation::serde_object::omitted_not_null"
    )]
    enabled_at: Option<Vec<String>>,
    #[serde(
        default,
        deserialize_with = "fsl_foundation::serde_object::omitted_not_null"
    )]
    limit: Option<ModelOperationCoverageLimit>,
    #[serde(
        default,
        deserialize_with = "fsl_foundation::serde_object::omitted_not_null"
    )]
    value: Option<u128>,
    #[serde(
        default,
        deserialize_with = "fsl_foundation::serde_object::omitted_not_null"
    )]
    setting: Option<String>,
}

impl<'de> Deserialize<'de> for ModelOperationCoverageEntry {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::Error;
        let wire = EntryWire::deserialize(deserializer)?;
        let missing = |member: &'static str| D::Error::missing_field(member);
        let extra = |present: bool, member: &'static str| {
            if present {
                Err(D::Error::custom(format!("unexpected member `{member}`")))
            } else {
                Ok(())
            }
        };
        let status = match (wire.status, wire.reason) {
            (EntryStatus::Covered, None) => {
                extra(wire.bounds.is_some(), "bounds")?;
                extra(wire.enabled_at.is_some(), "enabled_at")?;
                extra(wire.limit.is_some(), "limit")?;
                extra(wire.value.is_some(), "value")?;
                extra(wire.setting.is_some(), "setting")?;
                let first_case = wire.first_case.ok_or_else(|| missing("first_case"))?;
                ModelOperationCoverageStatus::Covered { first_case }
            }
            (EntryStatus::Covered, Some(_)) => {
                return Err(D::Error::custom("a covered entry has no reason"))
            }
            (EntryStatus::Uncovered, None) => return Err(missing("reason")),
            (EntryStatus::Uncovered, Some(reason)) => {
                extra(wire.first_case.is_some(), "first_case")?;
                let only = |keep: &[&str]| -> Result<(), D::Error> {
                    for (name, present) in [
                        ("bounds", wire.bounds.is_some()),
                        ("enabled_at", wire.enabled_at.is_some()),
                        ("limit", wire.limit.is_some()),
                        ("value", wire.value.is_some()),
                        ("setting", wire.setting.is_some()),
                    ] {
                        if !keep.contains(&name) {
                            extra(present, name)?;
                        }
                    }
                    Ok(())
                };
                let reason = match reason {
                    EntryReason::Unreachable => {
                        only(&[])?;
                        ModelOperationCoverageReason::Unreachable
                    }
                    EntryReason::UnreachableWithinBound => {
                        only(&["bounds"])?;
                        ModelOperationCoverageReason::UnreachableWithinBound {
                            bounds: wire.bounds.ok_or_else(|| missing("bounds"))?,
                        }
                    }
                    EntryReason::NotGenerated => {
                        only(&["enabled_at"])?;
                        ModelOperationCoverageReason::NotGenerated {
                            enabled_at: wire.enabled_at.ok_or_else(|| missing("enabled_at"))?,
                        }
                    }
                    EntryReason::Undetermined => {
                        only(&["limit", "value", "setting"])?;
                        let limit = wire.limit.ok_or_else(|| missing("limit"))?;
                        let value = wire.value.ok_or_else(|| missing("value"))?;
                        let setting = match limit {
                            ModelOperationCoverageLimit::Accounting(counter) => {
                                Some(format!("accounting.{}", counter.as_str()))
                            }
                            _ => None,
                        };
                        if wire.setting != setting {
                            return Err(D::Error::custom(
                                "`setting` is written exactly for an accounting limit",
                            ));
                        }
                        ModelOperationCoverageReason::Undetermined { limit, value }
                    }
                };
                ModelOperationCoverageStatus::NotCovered { reason }
            }
        };
        Ok(Self {
            operation: wire.operation,
            status,
        })
    }
}

/// Reachability exploration limits associated with an incomplete summary.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub struct ModelOperationCoverageLimits {
    /// Number of operations whose reachability remained undetermined.
    pub undetermined: usize,
}

/// A complete or partial operation-coverage summary.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ModelOperationCoverage {
    /// One entry per declared operation, preserving declaration order.
    pub operations: Vec<ModelOperationCoverageEntry>,
    /// Whether every declared operation was covered.
    pub status: ModelOperationCoverageSummaryStatus,
    /// Counts of uncovered operations grouped by reason.
    pub uncovered: BTreeMap<ModelOperationCoverageReasonKind, usize>,
    /// Reachability limitation counts.
    pub limits: ModelOperationCoverageLimits,
}

/// Stable grouping key for uncovered-operation summary counts.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ModelOperationCoverageReasonKind {
    /// Reachability was exhaustively disproved.
    Unreachable,
    /// No enabling state was found within the requested trace bound.
    UnreachableWithinBound,
    /// An enabling state was reached but no test case applied the operation.
    NotGenerated,
    /// Reachability exploration stopped before deciding.
    Undetermined,
}

impl Serialize for ModelOperationCoverageReasonKind {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(match self {
            Self::Unreachable => "unreachable",
            Self::UnreachableWithinBound => "unreachable-within-bound",
            Self::NotGenerated => "not-generated",
            Self::Undetermined => "undetermined",
        })
    }
}

impl<'de> Deserialize<'de> for ModelOperationCoverageReasonKind {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let name = String::deserialize(deserializer)?;
        match name.as_str() {
            "unreachable" => Ok(Self::Unreachable),
            "unreachable-within-bound" => Ok(Self::UnreachableWithinBound),
            "not-generated" => Ok(Self::NotGenerated),
            "undetermined" => Ok(Self::Undetermined),
            other => Err(serde::de::Error::custom(format!(
                "unknown uncovered reason `{other}`"
            ))),
        }
    }
}

/// Whether all declared operations were covered by evaluated cases.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ModelOperationCoverageSummaryStatus {
    /// Every declared operation was covered.
    Complete,
    /// At least one declared operation was not covered.
    Partial,
}

/// Reachability evidence for each operation, keyed by declaration identity.
///
/// `reachability` is produced from the model graph explored by the generator;
/// generic sampled-graph coverage remains independent and is not rewritten by
/// this function. `cases` must include only evaluated, non-discarded cases.
/// The returned entries retain the order of `operations`.
///
/// # Errors
/// Returns `None` if an operation appears more than once or a declared
/// operation lacks reachability evidence.
pub fn model_operation_coverage(
    operations: &[ModelOperation],
    cases: &[ModelOperationCase],
    reachability: &[(String, ModelOperationCoverageReason)],
) -> Option<ModelOperationCoverage> {
    let mut reachable_by_identity = BTreeMap::new();
    for (identity, reason) in reachability {
        if reachable_by_identity
            .insert(identity.as_str(), reason)
            .is_some()
        {
            return None;
        }
    }

    let mut first_case_by_identity = BTreeMap::new();
    for case in cases {
        for identity in &case.operations {
            first_case_by_identity
                .entry(identity.as_str())
                .and_modify(|first: &mut u64| *first = (*first).min(case.case_index))
                .or_insert(case.case_index);
        }
    }

    let mut seen = BTreeMap::new();
    let mut entries = Vec::with_capacity(operations.len());
    let mut uncovered = BTreeMap::new();
    let mut undetermined = 0_usize;
    for operation in operations {
        if seen.insert(operation.identity.as_str(), ()).is_some() {
            return None;
        }
        let status =
            if let Some(first_case) = first_case_by_identity.get(operation.identity.as_str()) {
                ModelOperationCoverageStatus::Covered {
                    first_case: *first_case,
                }
            } else {
                let reason = (*reachable_by_identity.get(operation.identity.as_str())?).clone();
                let kind = match reason {
                    ModelOperationCoverageReason::Unreachable => {
                        ModelOperationCoverageReasonKind::Unreachable
                    }
                    ModelOperationCoverageReason::UnreachableWithinBound { .. } => {
                        ModelOperationCoverageReasonKind::UnreachableWithinBound
                    }
                    ModelOperationCoverageReason::NotGenerated { .. } => {
                        ModelOperationCoverageReasonKind::NotGenerated
                    }
                    ModelOperationCoverageReason::Undetermined { .. } => {
                        undetermined = undetermined.saturating_add(1);
                        ModelOperationCoverageReasonKind::Undetermined
                    }
                };
                *uncovered.entry(kind).or_insert(0) += 1;
                ModelOperationCoverageStatus::NotCovered { reason }
            };
        entries.push(ModelOperationCoverageEntry {
            operation: operation.clone(),
            status,
        });
    }

    Some(ModelOperationCoverage {
        status: if uncovered.is_empty() {
            ModelOperationCoverageSummaryStatus::Complete
        } else {
            ModelOperationCoverageSummaryStatus::Partial
        },
        operations: entries,
        uncovered,
        limits: ModelOperationCoverageLimits { undetermined },
    })
}

#[cfg(test)]
mod tests {
    use ix_trace_rs::trace;
    use serde_json::json;

    use super::{
        ModelOperation, ModelOperationCoverageEntry, ModelOperationCoverageLimit,
        ModelOperationCoverageReason, ModelOperationCoverageReasonKind,
        ModelOperationCoverageStatus,
    };

    fn entry(status: ModelOperationCoverageStatus) -> ModelOperationCoverageEntry {
        ModelOperationCoverageEntry {
            operation: ModelOperation {
                identity: "ix://test/counters/Counter/jam".to_owned(),
                name: "jam".to_owned(),
            },
            status,
        }
    }

    fn uncovered(reason: ModelOperationCoverageReason) -> ModelOperationCoverageEntry {
        entry(ModelOperationCoverageStatus::NotCovered { reason })
    }

    /// FR-752-AC-4, AC-6, AC-11: each form of an operation's coverage entry is
    /// written with the members the spec names for it and read back as the
    /// same entry.
    #[trace("TC-2956", "FR-752-AC-4")]
    #[trace("TC-2956", "FR-752-AC-6")]
    #[trace("TC-3068", "FR-752-AC-11")]
    #[test]
    fn each_operation_coverage_form_round_trips() {
        let operation = json!({"identity": "ix://test/counters/Counter/jam", "name": "jam"});
        let forms = [
            (
                entry(ModelOperationCoverageStatus::Covered { first_case: 3 }),
                json!({"operation": operation, "status": "covered", "first_case": 3}),
            ),
            (
                uncovered(ModelOperationCoverageReason::Unreachable),
                json!({"operation": operation, "status": "uncovered", "reason": "unreachable"}),
            ),
            (
                uncovered(ModelOperationCoverageReason::UnreachableWithinBound {
                    bounds: vec!["max_trace_length".to_owned()],
                }),
                json!({"operation": operation, "status": "uncovered",
                    "reason": "unreachable-within-bound", "bounds": ["max_trace_length"]}),
            ),
            (
                uncovered(ModelOperationCoverageReason::NotGenerated {
                    enabled_at: vec!["add(by=300)".to_owned()],
                }),
                json!({"operation": operation, "status": "uncovered",
                    "reason": "not-generated", "enabled_at": ["add(by=300)"]}),
            ),
            (
                uncovered(ModelOperationCoverageReason::Undetermined {
                    limit: ModelOperationCoverageLimit::States,
                    value: 1,
                }),
                json!({"operation": operation, "status": "uncovered",
                    "reason": "undetermined", "limit": "states", "value": 1}),
            ),
            (
                uncovered(ModelOperationCoverageReason::Undetermined {
                    limit: ModelOperationCoverageLimit::Accounting(
                        filament_exact::LimitKind::WorkUnits,
                    ),
                    value: 1_000_000,
                }),
                json!({"operation": operation, "status": "uncovered",
                    "reason": "undetermined", "limit": "work_units", "value": 1_000_000,
                    "setting": "accounting.work_units"}),
            ),
        ];
        for (typed, wire) in forms {
            assert_eq!(serde_json::to_value(&typed).expect("writes"), wire);
            let read: ModelOperationCoverageEntry =
                serde_json::from_value(wire.clone()).expect("reads");
            assert_eq!(read, typed, "{wire}");
        }
    }

    /// An unknown reason, an unknown limit, a member the entry's form does
    /// not carry, a missing one, and a `setting` that does not follow the
    /// limit are refused.
    #[trace("TC-2956", "FR-752-AC-4")]
    #[test]
    fn a_malformed_operation_coverage_entry_is_refused() {
        let operation = json!({"identity": "ix://test/counters/Counter/jam", "name": "jam"});
        for wire in [
            json!({"operation": operation, "status": "uncovered", "reason": "lost"}),
            json!({"operation": operation, "status": "uncovered", "reason": "undetermined",
                "limit": "rows", "value": 1}),
            json!({"operation": operation, "status": "covered", "first_case": 0,
                "reason": "unreachable"}),
            json!({"operation": operation, "status": "covered"}),
            json!({"operation": operation, "status": "uncovered", "reason": "unreachable",
                "bounds": []}),
            json!({"operation": operation, "status": "uncovered", "reason": "not-generated"}),
            json!({"operation": operation, "status": "uncovered", "reason": "undetermined",
                "limit": "work_units", "value": 1}),
            json!({"operation": operation, "status": "uncovered", "reason": "undetermined",
                "limit": "states", "value": 1, "setting": "accounting.states"}),
            json!({"operation": operation, "status": "uncovered", "reason": "unreachable",
                "extra": 1}),
        ] {
            assert!(
                serde_json::from_value::<ModelOperationCoverageEntry>(wire.clone()).is_err(),
                "{wire}"
            );
        }
        assert!(serde_json::from_value::<ModelOperationCoverageReasonKind>(json!("lost")).is_err());
    }
}
