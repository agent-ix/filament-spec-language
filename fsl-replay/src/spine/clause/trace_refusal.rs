// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-645: a malformed or mismatched trace refused with a reason, a message
//! and the clause's locus. The catalog code and cause are those of every
//! trace input refusal, `invalid_runtime_input`/`invalid-value`; the reason
//! tells the cases apart, and code that decides from a refusal reads it, never
//! the message.

use fsl_foundation::source::provenance::SourceRegion;
use fsl_semantics::check::{TemporalClauseNode, TemporalProfile};

use super::ClauseRunStage;

/// A refusal of the shape or the fit of a supplied trace (FR-645).
#[derive(Clone, Debug)]
pub struct TraceRefusal {
    /// The stage it is reported at: `admit` for a kind mismatch, `evaluate`
    /// for an empty trace.
    pub stage: ClauseRunStage,
    /// FR-645's reason token.
    pub reason: &'static str,
    /// The clause's profile identity.
    pub profile: &'static str,
    /// For `trace-kind-mismatch`: the trace kinds the profile reads, in
    /// FR-641's table order.
    pub expected: Vec<&'static str>,
    /// For `trace-kind-mismatch`: the kind supplied.
    pub supplied: Option<&'static str>,
    /// The position at fault, where one position is.
    pub position: Option<u64>,
    /// The message a person can act on.
    pub message: String,
    /// The source region of the clause's declaration.
    pub locus: Option<SourceRegion>,
}

/// The trace kinds a clause under `profile` reads, in FR-641's table order
/// (FR-645 Outputs).
pub fn profile_reads(profile: TemporalProfile) -> &'static [&'static str] {
    match profile {
        TemporalProfile::EventPositionFalseExtension
        | TemporalProfile::FixedSampleFalseExtension => &["finite", "progressed"],
        TemporalProfile::InfiniteTrace => &["finite", "lasso", "model"],
        TemporalProfile::TimestampedEventFiniteWindow => &["timestamped"],
        TemporalProfile::Timed => &["timed"],
    }
}

/// The kinds spelled for a message: `a`, `a or b`, `a, b or c`.
fn spelled(kinds: &[&str]) -> String {
    match kinds {
        [] => String::new(),
        [only] => (*only).to_owned(),
        [init @ .., last] => format!("{} or {last}", init.join(", ")),
    }
}

/// The package and sources a refusal locates its clause in.
#[derive(Clone, Copy)]
pub struct ClauseSite<'a> {
    /// The compiled package.
    pub package: &'a fsl_package::CheckedPackage,
    /// The temporal clause.
    pub node: &'a TemporalClauseNode,
}

impl ClauseSite<'_> {
    fn refusal(
        &self,
        stage: ClauseRunStage,
        reason: &'static str,
        message: String,
    ) -> TraceRefusal {
        let graph = self.package.graph();
        let region = graph
            .occurrence(self.node.identity(), self.node.claim())
            .and_then(|location| graph.region(location));
        TraceRefusal {
            stage,
            reason,
            profile: self.node.clause().profile().identity(),
            expected: Vec::new(),
            supplied: None,
            position: None,
            message,
            locus: region,
        }
    }

    /// `trace-kind-mismatch`: the clause's profile does not read `supplied`.
    pub fn kind_mismatch(&self, supplied: &'static str) -> TraceRefusal {
        let profile = self.node.clause().profile();
        let expected = profile_reads(profile);
        let message = format!(
            "clause \"{}\" uses profile {}, which reads a {} trace; the request supplies a {} trace",
            self.node.clause().name(),
            profile.identity(),
            spelled(expected),
            supplied,
        );
        TraceRefusal {
            expected: expected.to_vec(),
            supplied: Some(supplied),
            ..self.refusal(ClauseRunStage::Admit, "trace-kind-mismatch", message)
        }
    }

    /// `empty-finite-trace`: a trace of `kind` holds no position.
    pub fn empty_trace(&self, kind: &'static str) -> TraceRefusal {
        self.refusal(
            ClauseRunStage::Evaluate,
            "empty-finite-trace",
            format!("a {kind} trace needs at least one position"),
        )
    }

    /// `empty-lasso-loop`: a lasso whose loop holds no position.
    pub fn empty_loop(&self) -> TraceRefusal {
        self.refusal(
            ClauseRunStage::Evaluate,
            "empty-lasso-loop",
            "a lasso needs at least one loop position".to_owned(),
        )
    }

    /// `unsupported-window-aggregate`: a window aggregate (FR-194) over a
    /// `kind` sample trace, which only the closed finite event-position route
    /// reads (QSL-2138).
    pub fn window_aggregate(&self, kind: &'static str) -> TraceRefusal {
        self.refusal(
            ClauseRunStage::Admit,
            "unsupported-window-aggregate",
            format!(
                "clause \"{}\" reads a window aggregate, which is not yet supported over a {kind} sample trace (QSL-2138)",
                self.node.clause().name(),
            ),
        )
    }

    /// `ledger-without-progress`: a ledger beside a trace that asserts no
    /// progress.
    pub fn ledger_without_progress(&self) -> TraceRefusal {
        self.refusal(
            ClauseRunStage::Admit,
            "ledger-without-progress",
            "a progress ledger applies only to progressed or timestamped traces".to_owned(),
        )
    }
}
