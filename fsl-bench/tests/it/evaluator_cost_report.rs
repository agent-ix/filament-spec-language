// SPDX-License-Identifier: AGPL-3.0-or-later
//! The invocation-local evaluator report is the acceptance evidence artifact.

use fsl_bench::evaluator_cost::{
    self, AllocationProbe, ReportRole, CALL_CHAIN_SIZES, EVALUATOR_CALL_CHAIN_AXIS,
    EVALUATOR_KERNEL_AXIS, PROFILE_AXES, REPORT_FIELDS,
};
use fsl_bench::kernel::{KernelKind, SIZES};
use ix_trace_rs::trace;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);

struct TempTarget(PathBuf);

impl TempTarget {
    fn new() -> Self {
        let sequence = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "fsl-evaluator-cost-report-{}-{sequence}",
            std::process::id()
        ));
        fs::create_dir_all(&path).expect("temporary target directory is created");
        Self(path)
    }

    fn write_estimate(&self, axis: &str, case: &str, size: usize, nanos: f64) {
        let path = estimate_path(&self.0, axis, case, size);
        fs::create_dir_all(path.parent().expect("estimate has a parent"))
            .expect("estimate directory is created");
        fs::write(
            path,
            serde_json::json!({"mean": {"point_estimate": nanos}}).to_string(),
        )
        .expect("Criterion estimate is written");
    }
}

impl Drop for TempTarget {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn estimate_path(target: &Path, axis: &str, case: &str, size: usize) -> PathBuf {
    let group = target.join("criterion").join(axis.replace('/', "_"));
    let path = if axis == EVALUATOR_KERNEL_AXIS {
        group
            .join(format!("interpreter_{case}"))
            .join(size.to_string())
    } else {
        group.join(size.to_string())
    };
    path.join("new/estimates.json")
}

fn expected_cases(axis: &str) -> Vec<(&'static str, usize)> {
    match axis {
        EVALUATOR_KERNEL_AXIS => KernelKind::ALL
            .into_iter()
            .flat_map(|kind| SIZES.into_iter().map(move |size| (kind.name(), size)))
            .collect(),
        EVALUATOR_CALL_CHAIN_AXIS => CALL_CHAIN_SIZES
            .into_iter()
            .map(|size| ("linked_chain", size))
            .collect(),
        _ => panic!("unexpected evaluator axis: {axis}"),
    }
}

fn seed_complete_invocation(target: &TempTarget, child_allocation_count: u64) -> u128 {
    let run_id = evaluator_cost::start_invocation(&target.0)
        .expect("the benchmark invocation is initialized");
    for (axis, _) in PROFILE_AXES {
        for (case, size) in expected_cases(axis) {
            evaluator_cost::record_probe(
                &target.0,
                run_id,
                axis,
                case,
                size,
                AllocationProbe {
                    child_traversal_allocation_count: child_allocation_count,
                    evaluation_allocation_count: 100,
                },
            )
            .expect("the invocation allocation probe is recorded");
            target.write_estimate(axis, case, size, 100.0 + size as f64);
        }
    }
    run_id
}

#[test]
#[trace("NFR-141-AC-7")]
fn emitted_invocation_artifacts_contain_the_required_axes_sizes_and_metrics() {
    let target = TempTarget::new();
    let run_id = seed_complete_invocation(&target, 0);

    for (axis, sizes) in PROFILE_AXES {
        let artifact = evaluator_cost::emit_report(&target.0, axis, ReportRole::Candidate)
            .expect("complete fresh Criterion and allocation data emits a report");
        let report: serde_json::Value =
            serde_json::from_slice(&fs::read(artifact).expect("report artifact exists"))
                .expect("report artifact is JSON");
        assert_eq!(report["run_id"], run_id.to_string());
        assert_eq!(report["axis"], axis);
        assert_eq!(report["role"], "candidate");
        assert_eq!(
            report["fields"],
            serde_json::to_value(REPORT_FIELDS).expect("report fields serialize")
        );

        let rows = report["rows"].as_array().expect("report rows are an array");
        let expected = expected_cases(axis);
        assert_eq!(rows.len(), expected.len());
        assert_eq!(
            rows.iter()
                .map(|row| (
                    row["case"].as_str().unwrap().to_owned(),
                    row["size"].as_u64().unwrap() as usize
                ))
                .collect::<BTreeSet<_>>(),
            expected
                .iter()
                .map(|(case, size)| ((*case).to_owned(), *size))
                .collect()
        );
        assert_eq!(
            expected
                .iter()
                .map(|(_, size)| *size)
                .collect::<BTreeSet<_>>(),
            sizes.iter().copied().collect()
        );
        for row in rows {
            assert!(row["wall_time_ns"].as_f64().unwrap() > 0.0);
            assert_eq!(row["child_traversal_allocation_count"], 0);
            assert_eq!(row["child_traversal_allocation_share"], 0.0);
            assert_eq!(row["evaluation_allocation_count"], 100);
        }
    }
}

#[test]
#[trace("NFR-141-AC-7")]
fn baseline_report_preserves_nonzero_child_traversal_allocations() {
    let target = TempTarget::new();
    seed_complete_invocation(&target, 18);

    let artifact =
        evaluator_cost::emit_report(&target.0, EVALUATOR_CALL_CHAIN_AXIS, ReportRole::Baseline)
            .expect("baseline allocations remain reportable");
    let report: serde_json::Value =
        serde_json::from_slice(&fs::read(artifact).expect("baseline report artifact exists"))
            .expect("baseline report artifact is JSON");

    assert_eq!(report["role"], "baseline");
    for row in report["rows"]
        .as_array()
        .expect("baseline rows are an array")
    {
        assert_eq!(row["child_traversal_allocation_count"], 18);
        assert_eq!(row["child_traversal_allocation_share"], 0.18);
    }
}

#[test]
#[trace("NFR-141-AC-7")]
fn candidate_report_rejects_nonzero_child_traversal_allocations() {
    let target = TempTarget::new();
    seed_complete_invocation(&target, 18);

    let error =
        evaluator_cost::emit_report(&target.0, EVALUATOR_CALL_CHAIN_AXIS, ReportRole::Candidate)
            .expect_err("candidate child traversal must allocate zero times");
    assert!(error
        .to_string()
        .contains("candidate child traversal allocation"));
}

#[test]
#[trace("NFR-141-AC-7")]
fn report_refuses_a_missing_per_size_probe() {
    let target = TempTarget::new();
    let run_id = evaluator_cost::start_invocation(&target.0)
        .expect("the benchmark invocation is initialized");
    for (case, size) in expected_cases(EVALUATOR_CALL_CHAIN_AXIS)
        .into_iter()
        .skip(1)
    {
        evaluator_cost::record_probe(
            &target.0,
            run_id,
            EVALUATOR_CALL_CHAIN_AXIS,
            case,
            size,
            AllocationProbe {
                child_traversal_allocation_count: 0,
                evaluation_allocation_count: 11,
            },
        )
        .expect("remaining size probes are recorded");
        target.write_estimate(EVALUATOR_CALL_CHAIN_AXIS, case, size, 100.0 + size as f64);
    }

    let error =
        evaluator_cost::emit_report(&target.0, EVALUATOR_CALL_CHAIN_AXIS, ReportRole::Candidate)
            .expect_err("missing per-size evidence prevents report emission");
    assert!(error.to_string().contains("missing allocation probe"));
}
