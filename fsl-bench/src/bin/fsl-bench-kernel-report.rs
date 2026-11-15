// SPDX-License-Identifier: AGPL-3.0-or-later
//! Print per-frame and per-operation kernel estimates from the latest run.

use fsl_bench::kernel::{KernelKind, SIZES};
use ix_trace_rs::string_edge;
use std::error::Error;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const MARKER: &str = "evaluator-kernel-run-start-nanos";
const GROUP: &str = "evaluator/kernel";

#[string_edge]
fn main() -> Result<(), Box<dyn Error>> {
    let target = std::env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("target"));
    run(&target, std::env::args().skip(1))
}

#[string_edge]
fn run(target: &Path, arguments: impl Iterator<Item = String>) -> Result<(), Box<dyn Error>> {
    let mut arguments = arguments;
    let command = arguments
        .next()
        .ok_or_else(|| io::Error::other("expected start or report"))?;
    match command.as_str() {
        "start" => mark_start(target),
        "report" => report(target),
        _ => Err(io::Error::other(format!("unknown command: {command}")).into()),
    }
}

fn mark_start(target: &Path) -> Result<(), Box<dyn Error>> {
    fs::create_dir_all(target)?;
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    fs::write(target.join(MARKER), nanos.to_string())?;
    Ok(())
}

fn report(target: &Path) -> Result<(), Box<dyn Error>> {
    let marker = fs::read_to_string(target.join(MARKER))?.parse::<u128>()?;
    let rows = collect_rows(target, marker)?;
    println!("{GROUP}: live Criterion estimates");
    println!("kernel size frames ops interpreter ns/frame interpreter ns/op Rust ns/frame Rust ns/op ratio");
    for row in rows {
        println!(
            "{} {} {} {} {:.2} {:.2} {:.2} {:.2} {:.2}x",
            row.kernel,
            row.size,
            row.frames,
            row.operations,
            row.interpreter_per_frame,
            row.interpreter_per_operation,
            row.rust_per_frame,
            row.rust_per_operation,
            row.ratio,
        );
    }
    Ok(())
}

#[derive(Debug, PartialEq)]
struct ReportRow {
    kernel: &'static str,
    size: usize,
    frames: usize,
    operations: usize,
    interpreter_per_frame: f64,
    interpreter_per_operation: f64,
    rust_per_frame: f64,
    rust_per_operation: f64,
    ratio: f64,
}

fn collect_rows(target: &Path, marker: u128) -> Result<Vec<ReportRow>, Box<dyn Error>> {
    let criterion = target.join("criterion").join(GROUP.replace('/', "_"));
    let mut rows = Vec::new();
    for kind in KernelKind::ALL {
        let name = kind.name();
        for size in SIZES {
            let Some(interpreter) = estimate(&criterion, "interpreter", name, size, marker)? else {
                continue;
            };
            let Some(rust) = estimate(&criterion, "rust", name, size, marker)? else {
                continue;
            };
            let frames = kind.frame_count(size);
            let operations = kind.operation_count(size);
            let interpreter_per_frame = interpreter / frames as f64;
            let interpreter_per_operation = interpreter / operations as f64;
            let rust_per_frame = rust / frames as f64;
            let rust_per_operation = rust / operations as f64;
            rows.push(ReportRow {
                kernel: name,
                size,
                frames,
                operations,
                interpreter_per_frame,
                interpreter_per_operation,
                rust_per_frame,
                rust_per_operation,
                ratio: interpreter_per_operation / rust_per_operation,
            });
        }
    }
    Ok(rows)
}

fn estimate(
    criterion: &Path,
    backend: &str,
    kernel: &str,
    size: usize,
    marker: u128,
) -> Result<Option<f64>, Box<dyn Error>> {
    let path = criterion
        .join(format!("{backend}_{kernel}"))
        .join(size.to_string())
        .join("new/estimates.json");
    let Ok(metadata) = fs::metadata(&path) else {
        return Ok(None);
    };
    let modified = metadata.modified()?.duration_since(UNIX_EPOCH)?.as_nanos();
    if modified <= marker {
        return Ok(None);
    }
    let estimates: serde_json::Value = serde_json::from_slice(&fs::read(path)?)?;
    estimates["mean"]["point_estimate"]
        .as_f64()
        .map(Some)
        .ok_or_else(|| io::Error::other("Criterion estimate has no mean point estimate").into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);

    struct TempTarget(PathBuf);

    impl TempTarget {
        fn new() -> Self {
            let sequence = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "fsl-kernel-report-{}-{sequence}",
                std::process::id()
            ));
            fs::create_dir_all(&path).expect("temporary target directory is created");
            Self(path)
        }

        fn write_estimate(&self, backend: &str, kind: KernelKind, size: usize, nanos: f64) {
            let path = self
                .0
                .join("criterion/evaluator_kernel")
                .join(format!("{backend}_{}", kind.name()))
                .join(size.to_string())
                .join("new/estimates.json");
            fs::create_dir_all(path.parent().expect("estimate has a parent"))
                .expect("estimate directory is created");
            fs::write(
                path,
                serde_json::json!({"mean": {"point_estimate": nanos}}).to_string(),
            )
            .expect("estimate file is written");
        }

        fn write_all_estimates(&self) {
            for kind in KernelKind::ALL {
                for size in SIZES {
                    let scale = size as f64;
                    self.write_estimate("interpreter", kind, size, 600.0 * scale / 5.0);
                    self.write_estimate("rust", kind, size, 60.0 * scale / 5.0);
                }
            }
        }

        fn write_marker(&self, nanos: u128) {
            fs::write(self.0.join(MARKER), nanos.to_string()).expect("run marker is written");
        }
    }

    impl Drop for TempTarget {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    #[trace("MP-012")]
    fn fresh_report_normalizes_each_configured_size_and_prints_live_ratio_inputs() {
        let target = TempTarget::new();
        target.write_all_estimates();
        let rows = collect_rows(&target.0, 0).expect("all complete fresh pairs report");
        let sum_five = rows
            .iter()
            .find(|row| row.kernel == "sum" && row.size == 5)
            .expect("size five is included");
        assert_eq!(sum_five.frames, 6);
        assert_eq!(sum_five.operations, 5);
        assert_eq!(sum_five.interpreter_per_frame, 100.0);
        assert_eq!(sum_five.interpreter_per_operation, 120.0);
        assert_eq!(sum_five.rust_per_frame, 10.0);
        assert_eq!(sum_five.rust_per_operation, 12.0);
        assert_eq!(sum_five.ratio, 10.0);
        let sum_ten = rows
            .iter()
            .find(|row| row.kernel == "sum" && row.size == 10)
            .expect("size ten is included");
        assert_eq!(sum_ten.interpreter_per_operation, 120.0);
        assert_eq!(rows.len(), KernelKind::ALL.len() * SIZES.len());
    }

    #[test]
    #[trace("MP-012")]
    fn stale_and_incomplete_pairs_are_excluded() {
        let stale = TempTarget::new();
        stale.write_all_estimates();
        assert!(collect_rows(&stale.0, u128::MAX)
            .expect("stale estimates are ignored")
            .is_empty());

        let incomplete = TempTarget::new();
        incomplete.write_all_estimates();
        let rust_sum_five = incomplete
            .0
            .join("criterion/evaluator_kernel/rust_sum/5/new/estimates.json");
        fs::remove_file(rust_sum_five).expect("one expected backend estimate is removed");
        let rows = collect_rows(&incomplete.0, 0).expect("partial pairs are ignored");
        assert!(!rows.iter().any(|row| row.kernel == "sum" && row.size == 5));
    }

    #[test]
    #[trace("MP-012")]
    fn partial_filter_results_report_only_complete_fresh_pairs() {
        let target = TempTarget::new();
        target.write_estimate("interpreter", KernelKind::ArithmeticRecursion, 5, 600.0);
        target.write_estimate("rust", KernelKind::ArithmeticRecursion, 5, 60.0);
        let rows = collect_rows(&target.0, 0).expect("partial benchmark population succeeds");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].kernel, "sum");
        assert_eq!(rows[0].size, 5);
    }

    #[test]
    #[trace("MP-012")]
    fn malformed_fresh_estimates_remain_errors() {
        let target = TempTarget::new();
        let path = target
            .0
            .join("criterion/evaluator_kernel/interpreter_sum/5/new/estimates.json");
        fs::create_dir_all(path.parent().expect("estimate has a parent"))
            .expect("estimate directory is created");
        fs::write(path, "{}").expect("malformed Criterion estimate is written");
        let error = collect_rows(&target.0, 0).expect_err("a malformed fresh estimate is invalid");
        assert!(error
            .to_string()
            .contains("Criterion estimate has no mean point estimate"));
    }

    #[test]
    #[trace("MP-012")]
    fn no_measurement_criterion_invocations_report_an_empty_result() {
        for arguments in [
            vec!["report", "--exact", "evaluator/kernel"],
            vec!["report", "--ignored"],
            vec!["report", "--list"],
        ] {
            let target = TempTarget::new();
            target.write_marker(0);
            let result = run(&target.0, arguments.into_iter().map(str::to_owned));
            assert!(result.is_ok(), "accepted no-measurement args succeed");
        }
    }
}
