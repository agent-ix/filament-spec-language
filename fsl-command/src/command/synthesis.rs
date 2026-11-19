// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-754: bounded candidate enumeration for value synthesis.

use filament_exact::{Integer, IntegerInterval};

/// The two independent limits applied while enumerating a bounded integer
/// domain. Each candidate consumes one unit from each counter.
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct SearchLimits {
    pub(super) max_candidates: Option<u64>,
    pub(super) max_work: Option<u64>,
}

/// The disposition of a bounded integer search.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum SearchDisposition {
    /// The first candidate accepted by the checked satisfaction expression.
    Candidate(Integer),
    /// Every value in the finite domain was visited without a match.
    NoCandidate,
    /// A request limit stopped the search before its finite domain was done.
    Incomplete { cause: &'static str },
}

/// One bounded integer search result, including the number of visited values.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct SearchResult {
    pub(super) disposition: SearchDisposition,
    pub(super) visited: u64,
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
                },
                visited,
            };
        }
        if limits.max_work.is_some_and(|maximum| visited >= maximum) {
            return SearchResult {
                disposition: SearchDisposition::Incomplete {
                    cause: "synthesis.max_work",
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
                cause: "synthesis.max_candidates"
            }
        );
        assert_eq!(result.visited, 3);
    }
}
