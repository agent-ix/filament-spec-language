// SPDX-License-Identifier: AGPL-3.0-or-later
//! The invocation-local evaluator report is the acceptance evidence artifact.

use fsl_bench::check::{call_head, completed_with_five, linked_chain};
use fsl_bench::evaluator_cost::{
    self, AllocationProbe, ReportRole, CALL_CHAIN_SIZES, EVALUATOR_CALL_CHAIN_AXIS,
    EVALUATOR_KERNEL_AXIS, PROFILE_AXES, REPORT_FIELDS,
};
use fsl_bench::kernel::KernelCase;
use fsl_bench::kernel::{KernelKind, SIZES};
use fsl_semantics::model::object_environment::ObjectEnvironment;
use ix_trace_rs::trace;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

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
            serde_json::json!({
                "mean": {
                    "point_estimate": nanos,
                    "confidence_interval": {
                        "confidence_level": 0.95,
                        "lower_bound": nanos * 0.9,
                        "upper_bound": nanos * 1.1,
                    }
                }
            })
            .to_string(),
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
            assert!(row["wall_time_ci_half_width_ns"].as_f64().unwrap() > 0.0);
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

#[test]
#[trace("NFR-141-AC-7")]
fn starting_invocation_clears_previous_criterion_estimates() {
    let target = TempTarget::new();
    target.write_estimate(EVALUATOR_KERNEL_AXIS, "sum", 5, 100.0);
    let stale = estimate_path(&target.0, EVALUATOR_KERNEL_AXIS, "sum", 5);
    assert!(stale.exists());

    evaluator_cost::start_invocation(&target.0)
        .expect("the benchmark invocation is initialized");

    assert!(!stale.exists(), "prior Criterion estimates are removed");
}

#[test]
#[trace("NFR-141-AC-7")]
fn emitted_candidate_artifact_uses_dynamic_evaluator_traversal_allocations() {
    let target = TempTarget::new();
    let run_id = evaluator_cost::start_invocation(&target.0)
        .expect("the benchmark invocation is initialized");
    let case = KernelCase::new(KernelKind::ArithmeticRecursion, 5);
    let objects = ObjectEnvironment::default();
    let (traversal, evaluation_allocation_count) = evaluator_cost::record_evaluation_probe(
        &target.0,
        run_id,
        EVALUATOR_KERNEL_AXIS,
        "sum",
        5,
        || {
            let result = case.call(&objects);
            assert!(case.matches_rust_reference(&result));
            std::hint::black_box(result);
        },
    )
    .expect("the production evaluator call records its dynamic traversal");
    assert!(traversal.iterator_count > 0);
    assert!(traversal.child_count > 0);

    for (axis, _) in PROFILE_AXES {
        for (case_name, size) in expected_cases(axis) {
            if axis == EVALUATOR_KERNEL_AXIS && case_name == "sum" && size == 5 {
                continue;
            }
            evaluator_cost::record_probe(
                &target.0,
                run_id,
                axis,
                case_name,
                size,
                AllocationProbe {
                    child_traversal_allocation_count: 0,
                    evaluation_allocation_count: 100,
                },
            )
            .expect("the other complete-matrix probes are recorded");
            target.write_estimate(axis, case_name, size, 100.0 + size as f64);
        }
    }
    target.write_estimate(EVALUATOR_KERNEL_AXIS, "sum", 5, 100.0);

    let artifact =
        evaluator_cost::emit_report(&target.0, EVALUATOR_KERNEL_AXIS, ReportRole::Candidate)
            .expect("the production candidate report is emitted");
    let report: serde_json::Value =
        serde_json::from_slice(&fs::read(artifact).expect("report exists"))
            .expect("report is JSON");
    let row = report["rows"]
        .as_array()
        .expect("report rows exist")
        .iter()
        .find(|row| row["case"] == "sum" && row["size"] == 5)
        .expect("the profiled evaluator case is present");
    assert_eq!(row["child_traversal_allocation_count"], 0);
    assert_eq!(row["child_traversal_allocation_share"], 0.0);
    assert_eq!(
        row["evaluation_allocation_count"],
        evaluation_allocation_count
    );
}

fn ab_result(side: &str, times: [f64; 5], child_allocations: u64) -> serde_json::Value {
    let role = if side == "A" { "baseline" } else { "candidate" };
    let rounds = (0..5)
        .map(|round_index| {
            let round = round_index + 1;
            let time = times[round_index];
            let rows = expected_cases(EVALUATOR_CALL_CHAIN_AXIS)
                .into_iter()
                .map(|(case, size)| {
                    serde_json::json!({
                        "case": case,
                        "size": size,
                        "wall_time_ns": time,
                        "wall_time_ci_half_width_ns": time * 0.05,
                        "child_traversal_allocation_count": child_allocations,
                        "child_traversal_allocation_share": child_allocations as f64 / 100.0,
                        "evaluation_allocation_count": 100,
                    })
                })
                .collect::<Vec<_>>();
            serde_json::json!({
                "round": round,
                "load_before": {"1m": round as f64, "5m": (round + 1) as f64, "15m": (round + 2) as f64},
                "load_after": {"1m": (round as f64) + 0.5, "5m": (round + 1) as f64 + 0.5, "15m": (round + 2) as f64 + 0.5},
                "report": {
                    "run_id": format!("{side}-{round}"),
                    "axis": EVALUATOR_CALL_CHAIN_AXIS,
                    "role": role,
                    "fields": REPORT_FIELDS,
                    "rows": rows,
                }
            })
        })
        .collect::<Vec<_>>();
    serde_json::json!({
        "format": "fsl-evaluator-ab-v1",
        "side": side,
        "axis": EVALUATOR_CALL_CHAIN_AXIS,
        "rounds": rounds,
    })
}

fn write_ab_result(target: &TempTarget, side: &str, input: &serde_json::Value) -> PathBuf {
    let path = target.0.join(format!("{side}.json"));
    fs::write(
        &path,
        serde_json::to_vec_pretty(input).expect("A/B input serializes"),
    )
    .expect("A/B result file is written");
    path
}

fn compare_ab(
    target: &TempTarget,
    a: &serde_json::Value,
    b: &serde_json::Value,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let a_path = write_ab_result(target, "A", a);
    let b_path = write_ab_result(target, "B", b);
    evaluator_cost::compare_ab_result_files(&a_path, &b_path, EVALUATOR_CALL_CHAIN_AXIS)
}

#[test]
#[trace("NFR-141-AC-7")]
fn comparison_computes_medians_mad_noise_verdict_and_preserves_round_loads() {
    let target = TempTarget::new();
    let a = ab_result("A", [80.0, 100.0, 100.0, 100.0, 120.0], 18);
    let b = ab_result("B", [80.0, 90.0, 100.0, 110.0, 120.0], 0);
    let result = compare_ab(&target, &a, &b).expect("complete five-round inputs compare");
    let row = result["rows"]
        .as_array()
        .expect("comparison rows exist")
        .iter()
        .find(|row| row["case"] == "linked_chain" && row["size"] == 1)
        .expect("linked chain size 1 is present");

    assert_eq!(row["a_median_wall_time_ns"], 100.0);
    assert_eq!(row["b_median_wall_time_ns"], 100.0);
    assert_eq!(row["a_mad_wall_time_ns"], 0.0);
    assert_eq!(row["b_mad_wall_time_ns"], 10.0);
    assert_eq!(row["mad_noise_fraction"], 0.1);
    assert_eq!(row["ci_noise_fraction"], 0.05);
    assert_eq!(row["noise_fraction"], 0.1);
    assert!((row["allowed_b_wall_time_ns"].as_f64().unwrap() - 110.0).abs() < 1e-9);
    assert_eq!(row["timing_passed"], true);
    assert_eq!(row["allocations_passed"], true);
    assert_eq!(row["a_rounds"][0]["load_before"]["15m"], 3.0);
    assert_eq!(row["b_rounds"][4]["load_after"]["1m"], 5.5);
    assert_eq!(result["raw_inputs"]["A"], a);
    assert_eq!(result["raw_inputs"]["B"], b);

    let slow_b = ab_result("B", [130.0; 5], 0);
    let failed = compare_ab(&target, &a, &slow_b).expect("slow candidate is still reportable");
    assert_eq!(failed["rows"][0]["timing_passed"], false);
    assert_eq!(failed["rows"][0]["passed"], false);
}

#[test]
#[trace("NFR-141-AC-7")]
fn comparison_fails_candidate_child_allocations_and_rejects_inconsistent_share() {
    let target = TempTarget::new();
    let a = ab_result("A", [10.0; 5], 18);
    let mut b = ab_result("B", [10.0; 5], 0);
    b["rounds"][0]["report"]["rows"][0]["child_traversal_allocation_count"] = 1.into();
    b["rounds"][0]["report"]["rows"][0]["child_traversal_allocation_share"] = 0.01.into();
    let result = compare_ab(&target, &a, &b).expect("nonzero B allocation remains a failed result");
    assert_eq!(result["rows"][0]["allocations_passed"], false);
    assert_eq!(result["rows"][0]["passed"], false);

    b["rounds"][0]["report"]["rows"][0]["child_traversal_allocation_share"] = 0.0.into();
    assert!(compare_ab(&target, &a, &b)
        .expect_err("allocation share must match counts")
        .to_string()
        .contains("share does not match"));
}

#[test]
#[trace("NFR-141-AC-7")]
fn comparison_rejects_incomplete_duplicate_and_reused_round_evidence() {
    let target = TempTarget::new();
    let a = ab_result("A", [10.0; 5], 18);
    let b = ab_result("B", [10.0; 5], 0);

    let mut incomplete = a.clone();
    incomplete["rounds"]
        .as_array_mut()
        .expect("rounds are an array")
        .pop();
    assert!(compare_ab(&target, &incomplete, &b)
        .expect_err("all five rounds are required")
        .to_string()
        .contains("must contain five rounds"));

    let mut missing = a.clone();
    missing["rounds"][0]["report"]["rows"]
        .as_array_mut()
        .expect("report rows are an array")
        .pop();
    assert!(compare_ab(&target, &missing, &b)
        .expect_err("a missing row is refused")
        .to_string()
        .contains("missing, duplicate, or unexpected"));

    let mut duplicate = a.clone();
    let row_zero = duplicate["rounds"][0]["report"]["rows"][0].clone();
    duplicate["rounds"][0]["report"]["rows"][3] = row_zero;
    assert!(compare_ab(&target, &duplicate, &b)
        .expect_err("a duplicate row is refused")
        .to_string()
        .contains("missing, duplicate, or unexpected"));

    let mut reused = b.clone();
    reused["rounds"][1]["report"]["run_id"] = "B-1".into();
    assert!(compare_ab(&target, &a, &reused)
        .expect_err("run IDs cannot be reused")
        .to_string()
        .contains("reused run identifier"));

    let mut cross_reused = b.clone();
    cross_reused["rounds"][0]["report"]["run_id"] = "A-1".into();
    assert!(compare_ab(&target, &a, &cross_reused)
        .expect_err("run IDs cannot be shared across A and B")
        .to_string()
        .contains("across A/B inputs"));
}

#[test]
#[trace("NFR-141-AC-7")]
fn start_profile_report_populates_five_round_files_with_load_metadata() {
    let target = TempTarget::new();
    let a_path = target.0.join("kernel-A.json");
    let b_path = target.0.join("kernel-B.json");
    let objects = ObjectEnvironment::default();

    for (side, result_path) in [("A", &a_path), ("B", &b_path)] {
        let role = if side == "A" {
            ReportRole::Baseline
        } else {
            ReportRole::Candidate
        };
        for round in 1..=5 {
            let run_id = evaluator_cost::start_recorded_invocation(
                &target.0,
                side,
                round,
                EVALUATOR_CALL_CHAIN_AXIS,
                result_path,
            )
            .expect("recorded invocation captures before-load metadata");
            // Some filesystems expose estimate mtimes at one-second resolution.
            // Let the invocation marker cross that boundary before writing fresh estimates.
            std::thread::sleep(Duration::from_secs(1));
            for (case, size) in expected_cases(EVALUATOR_CALL_CHAIN_AXIS) {
                if case == "linked_chain" && size == 1 {
                    let package = linked_chain(size);
                    evaluator_cost::record_evaluation_probe(
                        &target.0,
                        run_id,
                        EVALUATOR_CALL_CHAIN_AXIS,
                        case,
                        size,
                        || {
                            let evaluation = call_head(&package, &objects);
                            assert!(completed_with_five(&evaluation));
                            std::hint::black_box(evaluation);
                        },
                    )
                    .expect("the real evaluator profile records this case");
                    target.write_estimate(
                        EVALUATOR_CALL_CHAIN_AXIS,
                        case,
                        size,
                        100.0 + round as f64 + if side == "B" { 1.0 } else { 0.0 },
                    );
                    continue;
                }
                evaluator_cost::record_probe(
                    &target.0,
                    run_id,
                    EVALUATOR_CALL_CHAIN_AXIS,
                    case,
                    size,
                    AllocationProbe {
                        child_traversal_allocation_count: if side == "A" { 18 } else { 0 },
                        evaluation_allocation_count: 100,
                    },
                )
                .expect("every case probe is recorded");
                target.write_estimate(
                    EVALUATOR_CALL_CHAIN_AXIS,
                    case,
                    size,
                    100.0 + round as f64 + if side == "B" { 1.0 } else { 0.0 },
                );
            }
            evaluator_cost::emit_report(&target.0, EVALUATOR_CALL_CHAIN_AXIS, role)
                .expect("report captures after-load and appends the full report automatically");
        }
    }

    for path in [&a_path, &b_path] {
        let input: serde_json::Value =
            serde_json::from_slice(&fs::read(path).expect("recorded result file exists"))
                .expect("recorded result file is JSON");
        assert_eq!(input["rounds"].as_array().unwrap().len(), 5);
        for round in input["rounds"].as_array().unwrap() {
            for load_key in ["load_before", "load_after"] {
                for interval in ["1m", "5m", "15m"] {
                    assert!(round[load_key][interval].as_f64().unwrap() >= 0.0);
                }
            }
            assert_eq!(round["report"]["rows"].as_array().unwrap().len(), 4);
        }
    }
    let comparison =
        evaluator_cost::compare_ab_result_files(&a_path, &b_path, EVALUATOR_CALL_CHAIN_AXIS)
            .expect(
                "the automatically assembled result files compare without manual JSON assembly",
            );
    assert_eq!(comparison["passed"], true);
}

#[test]
#[trace("NFR-141-AC-7")]
fn recorded_round_setup_writes_load_manifest_and_run_markers() {
    let target = TempTarget::new();
    let result_file = target.0.join("A-kernel.json");
    let run_id = evaluator_cost::start_recorded_invocation(
        &target.0,
        "A",
        1,
        EVALUATOR_KERNEL_AXIS,
        &result_file,
    )
    .expect("load capture and marker setup complete");

    assert_eq!(
        evaluator_cost::current_invocation(&target.0).unwrap(),
        run_id
    );
    let round_marker: serde_json::Value = serde_json::from_slice(
        &fs::read(target.0.join("evaluator-cost-recorded-round.json"))
            .expect("recorded round marker exists"),
    )
    .expect("recorded round marker is JSON");
    assert_eq!(round_marker["format"], "fsl-evaluator-ab-round-v1");
    assert_eq!(round_marker["side"], "A");
    assert_eq!(round_marker["round"], 1);
    assert_eq!(round_marker["axis"], EVALUATOR_KERNEL_AXIS);
    assert_eq!(round_marker["result_file"], result_file.to_str().unwrap());
    for interval in ["1m", "5m", "15m"] {
        assert!(round_marker["load_before"][interval].as_f64().unwrap() >= 0.0);
    }

    let manifest: serde_json::Value = serde_json::from_slice(
        &fs::read(target.0.join("evaluator-cost-ab-session.json"))
            .expect("session manifest exists"),
    )
    .expect("session manifest is JSON");
    assert_eq!(
        manifest["paths"]["A:evaluator/kernel"],
        result_file.to_str().unwrap()
    );
}

#[test]
#[trace("NFR-141-AC-7")]
fn failed_recorded_round_is_immutable_and_allows_session_restart() {
    let target = TempTarget::new();
    let a_path = target.0.join("kernel-A.json");
    let b_path = target.0.join("kernel-B.json");
    evaluator_cost::start_recorded_invocation(
        &target.0,
        "A",
        1,
        EVALUATOR_CALL_CHAIN_AXIS,
        &a_path,
    )
    .expect("start captures the before-load metadata");
    evaluator_cost::record_round_failure(&target.0, "criterion exited with status 101")
        .expect("failure is appended with load-after metadata");
    fs::write(
        &b_path,
        serde_json::to_vec(&serde_json::json!({
            "format": "fsl-evaluator-ab-v1",
            "side": "B",
            "axis": EVALUATOR_CALL_CHAIN_AXIS,
            "rounds": [],
        }))
        .expect("B input serializes"),
    )
    .expect("empty B input is written");

    let recorded: serde_json::Value =
        serde_json::from_slice(&fs::read(&a_path).expect("A failure is persisted"))
            .expect("A failure is valid JSON");
    assert_eq!(recorded["rounds"][0]["status"], "failed");
    assert_eq!(
        recorded["rounds"][0]["diagnostic"],
        "criterion exited with status 101"
    );
    for load_key in ["load_before", "load_after"] {
        for interval in ["1m", "5m", "15m"] {
            assert!(recorded["rounds"][0][load_key][interval].as_f64().unwrap() >= 0.0);
        }
    }

    let comparison =
        evaluator_cost::compare_ab_result_files(&a_path, &b_path, EVALUATOR_CALL_CHAIN_AXIS)
            .expect("comparison reports the failed session rather than comparing results");
    assert_eq!(comparison["session_valid"], false);
    assert_eq!(comparison["passed"], false);
    assert_eq!(
        comparison["invalid_rounds"][0]["diagnostic"],
        "criterion exited with status 101"
    );

    let alternate_path = target.0.join("alternate-kernel-A.json");
    let restarted = evaluator_cost::start_recorded_invocation(
        &target.0,
        "A",
        1,
        EVALUATOR_CALL_CHAIN_AXIS,
        &alternate_path,
    )
    .expect("an invalidated session can restart under a fresh result path");
    assert_ne!(restarted, 0);
    let new_marker: serde_json::Value = serde_json::from_slice(
        &fs::read(target.0.join("evaluator-cost-recorded-round.json"))
            .expect("new round marker exists"),
    )
    .expect("new round marker is JSON");
    assert_eq!(new_marker["result_file"], alternate_path.to_str().unwrap());
    let old_record: serde_json::Value =
        serde_json::from_slice(&fs::read(&a_path).expect("old failure remains persisted"))
            .expect("old failed result remains valid JSON");
    assert_eq!(old_record["rounds"].as_array().unwrap().len(), 1);
    assert_eq!(old_record["rounds"][0]["status"], "failed");

    assert!(evaluator_cost::start_recorded_invocation(
        &target.0,
        "A",
        1,
        EVALUATOR_CALL_CHAIN_AXIS,
        &a_path,
    )
    .is_err());
}
