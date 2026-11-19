// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-754: bounded candidate enumeration for value synthesis.

use filament_exact::{Integer, IntegerInterval};
use serde::Serialize;
use serde_json::Value;

const DEFAULT_MAX_CANDIDATES: u64 = 10_000_000;
const DEFAULT_MAX_WORK: u64 = 100_000_000;

/// The two independent limits applied while enumerating a bounded integer
/// domain. Each candidate consumes one unit from each counter.
#[derive(Clone, Copy, Debug)]
pub(super) struct SearchLimits {
    pub(super) max_candidates: Option<u64>,
    pub(super) max_work: Option<u64>,
}

impl Default for SearchLimits {
    fn default() -> Self {
        Self {
            max_candidates: Some(DEFAULT_MAX_CANDIDATES),
            max_work: Some(DEFAULT_MAX_WORK),
        }
    }
}

/// The disposition of a bounded integer search.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum SearchDisposition {
    /// The first candidate accepted by the checked satisfaction expression.
    Candidate(Integer),
    /// Every value in the finite domain was visited without a match.
    NoCandidate,
    /// A request limit stopped the search before its finite domain was done.
    Incomplete { cause: &'static str, value: u64 },
}

/// One bounded integer search result, including the number of visited values.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct SearchResult {
    pub(super) disposition: SearchDisposition,
    pub(super) visited: u64,
}

/// The synthesis outcome's always-present `explored` object (FSpec FR-823).
#[derive(Clone, Debug, Serialize)]
struct Explored {
    visited: u64,
    work: u64,
}

/// The synthesis outcome's always-present `limits` object (FSpec FR-823).
#[derive(Clone, Debug, Serialize)]
struct OutcomeLimits {
    #[serde(rename = "synthesis.max_candidates")]
    max_candidates: u64,
    #[serde(rename = "synthesis.max_work")]
    max_work: u64,
}

/// A lineage entry identifying one candidate and both in-process engines.
#[derive(Clone, Debug, Serialize)]
struct Lineage {
    node: Value,
    index: u64,
    symmetry: String,
    synthesizer: &'static str,
    checker: &'static str,
}

/// The strict value-synthesis outcome document. Optional members serialize as
/// `null`, so each status keeps FR-823's one closed key set.
#[derive(Clone, Debug, Serialize)]
pub(super) struct SynthesisOutcomeDocument {
    format: &'static str,
    package: String,
    declaration: String,
    status: String,
    check: Option<String>,
    candidate: Option<Value>,
    objective: Option<Value>,
    optimal: Option<bool>,
    explored: Explored,
    lineage: Option<Lineage>,
    counterexample: Option<Value>,
    limits: OutcomeLimits,
    cause: Option<Value>,
}

impl SynthesisOutcomeDocument {
    /// Build the common outcome envelope, including the complete limit set.
    pub(super) fn new(
        package: impl Into<String>,
        declaration: impl Into<String>,
        status: impl Into<String>,
        visited: u64,
        work: u64,
        max_candidates: u64,
        max_work: u64,
    ) -> Self {
        Self {
            format: "filament.synthesis-outcome/v1",
            package: package.into(),
            declaration: declaration.into(),
            status: status.into(),
            check: None,
            candidate: None,
            objective: None,
            optimal: None,
            explored: Explored { visited, work },
            lineage: None,
            counterexample: None,
            limits: OutcomeLimits {
                max_candidates,
                max_work,
            },
            cause: None,
        }
    }

    /// Attach the independent check result and its typed candidate value.
    pub(super) fn with_candidate(
        mut self,
        value: Value,
        check: impl Into<String>,
        optimal: Option<bool>,
    ) -> Self {
        self.candidate = Some(value);
        self.check = Some(check.into());
        self.optimal = optimal;
        self
    }

    /// Attach the expression and evaluated value for an authored objective.
    pub(super) fn with_objective(mut self, objective: Value) -> Self {
        self.objective = Some(objective);
        self
    }

    /// Attach the source node and deterministic candidate lineage.
    pub(super) fn with_lineage(
        mut self,
        node: Value,
        index: u64,
        symmetry: impl Into<String>,
    ) -> Self {
        self.lineage = Some(Lineage {
            node,
            index,
            symmetry: symmetry.into(),
            synthesizer: "filament.synthesis.enumerate/v1",
            checker: "filament.native.value-validity/v1",
        });
        self
    }

    /// Attach a checker's replayable counterexample.
    pub(super) fn with_counterexample(mut self, value: Value) -> Self {
        self.counterexample = Some(value);
        self
    }

    /// Attach a limit, cancellation, or engine cause.
    pub(super) fn with_cause(mut self, value: Value) -> Self {
        self.cause = Some(value);
        self
    }

    /// Serialize the closed document to its JSON value.
    pub(super) fn into_value(self) -> Value {
        serde_json::to_value(self).expect("the synthesis outcome is serializable")
    }
}

/// Enumerate a closed integer interval in ascending value order and evaluate
/// each value once. The evaluator is the caller's native checked-expression
/// path; limits stop before the next candidate is evaluated.
pub(super) fn search_integer_domain(
    domain: &IntegerInterval,
    limits: SearchLimits,
    mut satisfies: impl FnMut(&Integer) -> bool,
) -> SearchResult {
    let lower = domain.lower().clone();
    let upper = domain.upper().clone();
    let mut candidate = lower;
    let mut visited = 0_u64;

    loop {
        if candidate > upper {
            return SearchResult {
                disposition: SearchDisposition::NoCandidate,
                visited,
            };
        }
        if limits
            .max_candidates
            .is_some_and(|maximum| visited >= maximum)
        {
            return SearchResult {
                disposition: SearchDisposition::Incomplete {
                    cause: "synthesis.max_candidates",
                    value: limits.max_candidates.expect("limit was checked"),
                },
                visited,
            };
        }
        if limits.max_work.is_some_and(|maximum| visited >= maximum) {
            return SearchResult {
                disposition: SearchDisposition::Incomplete {
                    cause: "synthesis.max_work",
                    value: limits.max_work.expect("limit was checked"),
                },
                visited,
            };
        }

        visited = visited.saturating_add(1);
        if satisfies(&candidate) {
            return SearchResult {
                disposition: SearchDisposition::Candidate(candidate),
                visited,
            };
        }
        candidate = candidate.add(&Integer::one());
    }
}

/// Run the integer candidate loop and encode its FR-823 result without
/// coupling the search helper to request selection or package admission.
pub(super) fn search_integer_outcome(
    package: &str,
    declaration: &str,
    node: Value,
    domain: &IntegerInterval,
    limits: SearchLimits,
    mut satisfies: impl FnMut(&Integer) -> bool,
    mut validate: impl FnMut(&Integer) -> Result<(), Value>,
) -> (SynthesisOutcomeDocument, u8) {
    let max_candidates = limits.max_candidates.unwrap_or(DEFAULT_MAX_CANDIDATES);
    let max_work = limits.max_work.unwrap_or(DEFAULT_MAX_WORK);
    let result = search_integer_domain(domain, limits, &mut satisfies);
    let mut document = SynthesisOutcomeDocument::new(
        package,
        declaration,
        "",
        result.visited,
        max_work,
        max_candidates,
        max_work,
    );
    let exit = match result.disposition {
        SearchDisposition::Candidate(value) => {
            document.status = "candidate".to_owned();
            document.candidate = Some(integer_value(&value));
            document.lineage = Some(Lineage {
                node,
                index: result.visited.saturating_sub(1),
                symmetry: "identity".to_owned(),
                synthesizer: "filament.synthesis.enumerate/v1",
                checker: "filament.native.value-validity/v1",
            });
            match validate(&value) {
                Ok(()) => {
                    document.check = Some("validated".to_owned());
                    0
                }
                Err(counterexample) => {
                    document.check = Some("rejected".to_owned());
                    document = document.with_counterexample(counterexample);
                    30
                }
            }
        }
        SearchDisposition::NoCandidate => {
            document.status = "no-candidate".to_owned();
            10
        }
        SearchDisposition::Incomplete { cause, value } => {
            let setting = cause;
            let limit = setting
                .strip_prefix("synthesis.")
                .expect("a synthesis search limit");
            document.status = "incomplete".to_owned();
            document = document.with_cause(serde_json::json!({
                "kind": "limit-reached",
                "limit": limit,
                "value": value,
                "setting": setting,
            }));
            22
        }
    };
    (document, exit)
}

fn integer_value(value: &Integer) -> Value {
    serde_json::json!({"type": "integer", "value": value.to_string()})
}

#[cfg(test)]
mod tests {
    use super::{search_integer_domain, SearchDisposition, SearchLimits};
    use filament_exact::{Integer, IntegerInterval};

    fn digit_domain() -> IntegerInterval {
        IntegerInterval::new(Integer::from(0_i64), Integer::from(9_i64)).expect("digit range")
    }

    #[test]
    fn visits_values_through_the_first_satisfying_integer() {
        let result = search_integer_domain(&digit_domain(), SearchLimits::default(), |value| {
            value == &Integer::from(7_i64)
        });
        assert_eq!(
            result.disposition,
            SearchDisposition::Candidate(Integer::from(7_i64))
        );
        assert_eq!(result.visited, 8);
    }

    #[test]
    fn exhausts_a_finite_integer_domain_before_reporting_no_candidate() {
        let result = search_integer_domain(&digit_domain(), SearchLimits::default(), |_| false);
        assert_eq!(result.disposition, SearchDisposition::NoCandidate);
        assert_eq!(result.visited, 10);
    }

    #[test]
    fn request_limits_stop_before_evaluating_the_next_value() {
        let result = search_integer_domain(
            &digit_domain(),
            SearchLimits {
                max_candidates: Some(3),
                max_work: None,
            },
            |_| false,
        );
        assert_eq!(
            result.disposition,
            SearchDisposition::Incomplete {
                cause: "synthesis.max_candidates",
                value: 3
            }
        );
        assert_eq!(result.visited, 3);
    }

    #[test]
    fn search_outcome_carries_typed_candidate_and_limit_cause() {
        let (candidate, exit) = super::search_integer_outcome(
            "pkg",
            "Pick",
            serde_json::json!({"package": "pkg", "path": ["Pick"], "ordinal": 0}),
            &digit_domain(),
            SearchLimits::default(),
            |value| value == &Integer::from(7_i64),
            |_| Ok(()),
        );
        assert_eq!(exit, 0);
        let candidate = candidate.into_value();
        assert_eq!(candidate["status"], "candidate");
        assert_eq!(
            candidate["candidate"],
            serde_json::json!({"type": "integer", "value": "7"})
        );
        assert_eq!(candidate["check"], "validated");
        assert_eq!(candidate["lineage"]["index"], 7);
        assert_eq!(candidate["lineage"]["symmetry"], "identity");

        let (limited, exit) = super::search_integer_outcome(
            "pkg",
            "Pick",
            serde_json::json!({"package": "pkg", "path": ["Pick"], "ordinal": 0}),
            &digit_domain(),
            SearchLimits {
                max_candidates: Some(3),
                max_work: Some(DEFAULT_MAX_WORK),
            },
            |_| false,
            |_| Ok(()),
        );
        assert_eq!(exit, 22);
        let limited = limited.into_value();
        assert_eq!(limited["status"], "incomplete");
        assert_eq!(limited["explored"]["visited"], 3);
        assert_eq!(
            limited["cause"],
            serde_json::json!({
                "kind": "limit-reached", "limit": "max_candidates", "value": 3,
                "setting": "synthesis.max_candidates"
            })
        );
    }

    #[test]
    fn candidate_outcome_serializes_every_fr823_member() {
        let outcome = SynthesisOutcomeDocument::new(
            "pkg",
            "Pick",
            "candidate",
            8,
            8,
            10_000_000,
            100_000_000,
        )
        .with_candidate(
            serde_json::json!({"type": "integer", "value": "7"}),
            "validated",
            None,
        )
        .with_lineage(
            serde_json::json!({"package": "pkg", "path": ["Pick"], "ordinal": 0}),
            7,
            "identity",
        )
        .into_value();
        assert_eq!(
            outcome
                .as_object()
                .unwrap()
                .keys()
                .map(String::as_str)
                .collect::<Vec<_>>(),
            [
                "candidate",
                "cause",
                "check",
                "counterexample",
                "declaration",
                "explored",
                "format",
                "lineage",
                "limits",
                "objective",
                "optimal",
                "package",
                "status"
            ]
        );
        assert_eq!(outcome["format"], "filament.synthesis-outcome/v1");
        assert_eq!(
            outcome["explored"],
            serde_json::json!({"visited": 8, "work": 8})
        );
        assert_eq!(outcome["lineage"]["symmetry"], "identity");
        assert_eq!(outcome["lineage"]["index"], 7);
        assert_eq!(outcome["optimal"], Value::Null);
        assert_eq!(outcome["objective"], Value::Null);
        assert_eq!(outcome["counterexample"], Value::Null);
        assert_eq!(outcome["cause"], Value::Null);
    }
}
