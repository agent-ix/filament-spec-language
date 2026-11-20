// SPDX-License-Identifier: AGPL-3.0-or-later
//! One conformance runner for the standard's published corpora (QSL-2048,
//! QSL-2051, QSL-2052).
//!
//! Each corpus module discovers its cases from the pinned standard package,
//! runs every case through FSL's public source path, and records one
//! [`Outcome`] per case. [`Corpus::settle`] then accepts the run only when the
//! failing cases are exactly the corpus's listed known gaps: an unlisted
//! mismatch fails, a listed gap that now passes fails, a gap without an owner
//! fails, and a corpus whose accounted cases differ from its published cases
//! fails. Nothing is skipped and nothing is ignored.

mod catalog;
mod finite_window;
mod qualification;
mod state_core;

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

/// How one case differed from the standard's expectation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Difference {
    /// The standard's expectation, as the fixture states it.
    pub(crate) expected: String,
    /// What FSL produced, or why it could not produce a comparable result.
    pub(crate) observed: String,
}

impl Difference {
    /// A difference between an expectation and an observation.
    pub(crate) fn new(expected: impl fmt::Display, observed: impl fmt::Display) -> Self {
        Self {
            expected: expected.to_string(),
            observed: observed.to_string(),
        }
    }
}

impl fmt::Display for Difference {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "expected {}, observed {}", self.expected, self.observed)
    }
}

/// A case that does not currently conform, with the area or ticket that owns
/// the difference.
#[derive(Clone, Copy, Debug)]
pub(crate) struct KnownGap {
    /// The case identity as the corpus records it.
    pub(crate) id: &'static str,
    /// The ticket key or suspected owner area. Never empty.
    pub(crate) owner: &'static str,
}

/// Shorthand for a [`KnownGap`] table row.
pub(crate) const fn gap(id: &'static str, owner: &'static str) -> KnownGap {
    KnownGap { id, owner }
}

/// The recorded run of one published corpus.
#[derive(Debug)]
pub(crate) struct Corpus {
    name: &'static str,
    published: usize,
    outcomes: Vec<(String, Result<(), Difference>)>,
}

/// The counts of a settled corpus.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Summary {
    /// Cases accounted.
    pub(crate) cases: usize,
    /// Cases that matched the standard.
    pub(crate) passed: usize,
    /// Cases that are listed known gaps.
    pub(crate) gaps: usize,
}

impl Corpus {
    /// An empty run of the corpus `name`, which publishes `published` cases.
    pub(crate) fn new(name: &'static str, published: usize) -> Self {
        Self {
            name,
            published,
            outcomes: Vec::new(),
        }
    }

    /// Records the outcome of the case `id`.
    pub(crate) fn record(&mut self, id: impl Into<String>, outcome: Result<(), Difference>) {
        self.outcomes.push((id.into(), outcome));
    }

    /// The outcome recorded for `id`, when one is.
    pub(crate) fn outcome(&self, id: &str) -> Option<&Result<(), Difference>> {
        self.outcomes
            .iter()
            .find_map(|(case, outcome)| (case == id).then_some(outcome))
    }

    /// Accepts the run when its failures are exactly `gaps`. Every reason
    /// for rejection is reported, each naming its case.
    pub(crate) fn settle(&self, gaps: &[KnownGap]) -> Result<Summary, Vec<String>> {
        let mut errors = Vec::new();
        if self.published == 0 {
            errors.push(format!("{}: the standard publishes no cases", self.name));
        }
        if self.outcomes.len() != self.published {
            errors.push(format!(
                "{}: {} cases published, {} accounted",
                self.name,
                self.published,
                self.outcomes.len()
            ));
        }
        let mut seen = BTreeSet::new();
        for (id, _) in &self.outcomes {
            if !seen.insert(id.as_str()) {
                errors.push(format!("{}: case {id} recorded twice", self.name));
            }
        }
        let mut listed = BTreeMap::new();
        for gap in gaps {
            if gap.owner.trim().is_empty() {
                errors.push(format!("{}: known gap {} has no owner", self.name, gap.id));
            }
            if listed.insert(gap.id, gap.owner).is_some() {
                errors.push(format!("{}: known gap {} listed twice", self.name, gap.id));
            }
            if !seen.contains(gap.id) {
                errors.push(format!(
                    "{}: known gap {} names no published case",
                    self.name, gap.id
                ));
            }
        }
        let mut failed = 0;
        for (id, outcome) in &self.outcomes {
            match (outcome, listed.get(id.as_str())) {
                (Ok(()), Some(owner)) => errors.push(format!(
                    "{}: {id} now matches the standard; remove its known gap ({owner})",
                    self.name
                )),
                (Err(difference), None) => errors.push(format!(
                    "{}: {id}: {difference} (not a listed known gap)",
                    self.name
                )),
                (Err(_), Some(_)) => failed += 1,
                (Ok(()), None) => {}
            }
        }
        if errors.is_empty() {
            Ok(Summary {
                cases: self.outcomes.len(),
                passed: self.outcomes.len() - failed,
                gaps: failed,
            })
        } else {
            Err(errors)
        }
    }

    /// Settles the run against `gaps`, prints every gap with its difference
    /// and the corpus summary, and panics with every rejection reason.
    pub(crate) fn assert_settled(&self, gaps: &[KnownGap]) -> Summary {
        let summary = self
            .settle(gaps)
            .unwrap_or_else(|errors| panic!("{}", errors.join("\n")));
        for gap in gaps {
            if let Some(Err(difference)) = self.outcome(gap.id) {
                println!(
                    "known gap: {}: {}: {difference} [{}]",
                    self.name, gap.id, gap.owner
                );
            }
        }
        println!(
            "conformance: {} standard {} cases accounted: {} passed, {} known gaps",
            summary.cases, self.name, summary.passed, summary.gaps
        );
        summary
    }
}

fn sample() -> Corpus {
    let mut corpus = Corpus::new("sample", 3);
    corpus.record("a", Ok(()));
    corpus.record("b", Err(Difference::new("true", "false")));
    corpus.record("c", Ok(()));
    corpus
}

/// Provenance: QSL-2048. The listed gaps are exactly the failures.
#[test]
fn a_run_whose_failures_are_its_known_gaps_settles() {
    assert_eq!(
        sample().settle(&[gap("b", "QSL-0")]),
        Ok(Summary {
            cases: 3,
            passed: 2,
            gaps: 1
        })
    );
}

/// Provenance: QSL-2048. An unlisted mismatch names its case and difference.
#[test]
fn an_unlisted_mismatch_is_reported_with_its_difference() {
    assert_eq!(
        sample().settle(&[]),
        Err(vec![
            "sample: b: expected true, observed false (not a listed known gap)".to_owned()
        ])
    );
}

/// Provenance: QSL-2048. A gap list cannot shrink silently or keep a fixed case.
#[test]
fn a_listed_gap_that_passes_or_names_nothing_is_rejected() {
    let errors = sample()
        .settle(&[gap("b", "QSL-0"), gap("a", "QSL-1"), gap("z", "QSL-2")])
        .expect_err("stale gaps are rejected");
    assert_eq!(
        errors,
        vec![
            "sample: known gap z names no published case".to_owned(),
            "sample: a now matches the standard; remove its known gap (QSL-1)".to_owned(),
        ]
    );
}

/// Provenance: QSL-2051. A listed gap needs an owner.
#[test]
fn a_known_gap_without_an_owner_is_rejected() {
    let errors = sample()
        .settle(&[gap("b", " ")])
        .expect_err("ownerless gap");
    assert_eq!(errors, vec!["sample: known gap b has no owner".to_owned()]);
}

/// Provenance: QSL-2048. A dropped case or an empty corpus cannot pass.
#[test]
fn a_dropped_case_or_an_empty_corpus_is_rejected() {
    let mut corpus = sample();
    corpus.published = 4;
    assert_eq!(
        corpus.settle(&[gap("b", "QSL-0")]),
        Err(vec!["sample: 4 cases published, 3 accounted".to_owned()])
    );
    assert_eq!(
        Corpus::new("empty", 0).settle(&[]),
        Err(vec!["empty: the standard publishes no cases".to_owned()])
    );
}
