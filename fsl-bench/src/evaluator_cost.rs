//! Acceptance binder for the evaluator cost and child-traversal profile.

use crate::kernel;

/// The Criterion axis for checked evaluator kernels.
pub const EVALUATOR_KERNEL_AXIS: &str = "evaluator/kernel";

/// The Criterion axis for linked function-call chains.
pub const EVALUATOR_CALL_CHAIN_AXIS: &str = "evaluator/call_chain";

/// The linked call-chain sizes measured by the evaluator benchmark.
pub const CALL_CHAIN_SIZES: [usize; 4] = [1, 10, 100, 1_000];

/// Axes and sizes required by the evaluator child-traversal profile.
pub const PROFILE_AXES: [(&str, &[usize]); 2] = [
    (EVALUATOR_KERNEL_AXIS, &kernel::SIZES),
    (EVALUATOR_CALL_CHAIN_AXIS, &CALL_CHAIN_SIZES),
];

/// Required columns in the evaluator cost report.
pub const REPORT_FIELDS: [&str; 3] = [
    "wall_time_ns",
    "child_traversal_allocation_count",
    "child_traversal_allocation_share",
];

/// One evaluator cost report row for a benchmark/profile input.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EvaluatorCostReport {
    /// Median or per-round wall time in nanoseconds.
    pub wall_time_ns: f64,
    /// Allocations attributed to checked child traversal.
    pub child_traversal_allocation_count: u64,
    /// Fraction of total allocations attributed to checked child traversal.
    pub child_traversal_allocation_share: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;

    /// QSL-1921's performance evidence covers both axes and reports time plus
    /// the child-traversal allocation count and share at each configured size.
    #[trace("NFR-141-AC-7")]
    #[test]
    fn evaluator_cost_profile_binds_axes_sizes_and_report_fields() {
        assert_eq!(
            PROFILE_AXES,
            [
                ("evaluator/kernel", &[5, 10][..]),
                ("evaluator/call_chain", &[1, 10, 100, 1_000][..]),
            ]
        );
        assert_eq!(REPORT_FIELDS, [
            "wall_time_ns",
            "child_traversal_allocation_count",
            "child_traversal_allocation_share",
        ]);

        let report = EvaluatorCostReport {
            wall_time_ns: 1.0,
            child_traversal_allocation_count: 0,
            child_traversal_allocation_share: 0.0,
        };
        assert!(report.wall_time_ns > 0.0);
        assert_eq!(report.child_traversal_allocation_count, 0);
        assert_eq!(report.child_traversal_allocation_share, 0.0);
    }
}
