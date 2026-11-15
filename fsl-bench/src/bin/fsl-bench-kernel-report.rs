// SPDX-License-Identifier: AGPL-3.0-or-later
//! Print per-frame and per-operation kernel estimates from the latest run.

use fsl_bench::kernel::{KernelKind, SIZES};
use ix_trace_rs::string_edge;
use regex::Regex;
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
    let mut arguments = std::env::args().skip(1);
    let command = arguments
        .next()
        .ok_or_else(|| io::Error::other("expected start or report"))?;
    match command.as_str() {
        "start" => mark_start(&target),
        "report" => {
            let criterion_args = arguments.collect::<Vec<_>>();
            let filter = criterion_filter(&criterion_args)?;
            report(&target, filter.as_deref())
        }
        _ => Err(io::Error::other(format!("unknown command: {command}")).into()),
    }
}

fn mark_start(target: &Path) -> Result<(), Box<dyn Error>> {
    fs::create_dir_all(target)?;
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    fs::write(target.join(MARKER), nanos.to_string())?;
    Ok(())
}

#[string_edge]
fn criterion_filter(arguments: &[String]) -> Result<Option<String>, io::Error> {
    let mut arguments = arguments.iter().peekable();
    while let Some(argument) = arguments.next() {
        if argument == "--" {
            continue;
        }
        if matches!(
            argument.as_str(),
            "--sample-size"
                | "--measurement-time"
                | "--warm-up-time"
                | "--nresamples"
                | "--noise-threshold"
                | "--confidence-level"
                | "--significance-level"
                | "--baseline"
                | "--load-baseline"
                | "--save-baseline"
                | "--baseline-compare"
                | "--color"
                | "--format"
                | "--profile-time"
                | "--plotting-backend"
        ) {
            arguments.next();
            continue;
        }
        if argument.starts_with('-') {
            continue;
        }
        return Ok(Some(argument.clone()));
    }
    Ok(None)
}

fn report(target: &Path, filter: Option<&str>) -> Result<(), Box<dyn Error>> {
    let marker = fs::read_to_string(target.join(MARKER))?.parse::<u128>()?;
    let filter = filter.map(Regex::new).transpose()?;
    let rows = collect_rows(target, marker, filter.as_ref())?;
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

fn collect_rows(
    target: &Path,
    marker: u128,
    filter: Option<&Regex>,
) -> Result<Vec<ReportRow>, Box<dyn Error>> {
    let criterion = target.join("criterion").join(GROUP.replace('/', "_"));
    let mut rows = Vec::new();
    for kind in KernelKind::ALL {
        let name = kind.name();
        for size in SIZES {
            let interpreter_id = format!("{GROUP}/interpreter/{name}/{size}");
            let rust_id = format!("{GROUP}/rust/{name}/{size}");
            let interpreter_selected =
                filter.is_none_or(|pattern| pattern.is_match(&interpreter_id));
            let rust_selected = filter.is_none_or(|pattern| pattern.is_match(&rust_id));
            if !interpreter_selected && !rust_selected {
                continue;
            }
            // A filter that selects only one backend deliberately requests a partial
            // Criterion run, so it has no complete report row to emit.
            if interpreter_selected != rust_selected {
                continue;
            }
            let interpreter = estimate(&criterion, "interpreter", name, size, marker)?
                .ok_or_else(|| missing_estimate("interpreter", name, size))?;
            let rust = estimate(&criterion, "rust", name, size, marker)?
                .ok_or_else(|| missing_estimate("rust", name, size))?;
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

fn missing_estimate(backend: &str, kernel: &str, size: usize) -> io::Error {
    io::Error::other(format!(
        "expected fresh {backend} Criterion estimate for {kernel}/{size}"
    ))
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
        let rows = collect_rows(&target.0, 0, None).expect("all complete fresh pairs report");
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
    fn stale_and_incomplete_expected_pairs_are_rejected() {
        let stale = TempTarget::new();
        stale.write_all_estimates();
        let error = collect_rows(&stale.0, u128::MAX, None)
            .expect_err("old estimates cannot satisfy a new run marker");
        assert!(error.to_string().contains("expected fresh interpreter"));

        let incomplete = TempTarget::new();
        incomplete.write_all_estimates();
        let rust_sum_five = incomplete
            .0
            .join("criterion/evaluator_kernel/rust_sum/5/new/estimates.json");
        fs::remove_file(rust_sum_five).expect("one expected backend estimate is removed");
        let error = collect_rows(&incomplete.0, 0, None)
            .expect_err("a selected complete pair requires both fresh estimates");
        assert!(error
            .to_string()
            .contains("expected fresh rust Criterion estimate for sum/5"));
    }

    #[test]
    #[trace("MP-012")]
    fn filter_with_no_complete_pair_succeeds_without_reusing_stale_data() {
        let target = TempTarget::new();
        target.write_all_estimates();
        let unrelated = Regex::new("parser").expect("filter regex compiles");
        assert!(collect_rows(&target.0, u128::MAX, Some(&unrelated))
            .expect("an unrelated benchmark filter is successful")
            .is_empty());

        let one_backend = Regex::new("interpreter").expect("filter regex compiles");
        assert!(collect_rows(&target.0, 0, Some(&one_backend))
            .expect("a filtered partial backend run is successful")
            .is_empty());

        let no_filter = criterion_filter(&["--".to_owned(), "--noplot".to_owned()])
            .expect("Criterion flags parse");
        assert_eq!(no_filter, None);
        let with_filter = criterion_filter(&[
            "--".to_owned(),
            "--sample-size".to_owned(),
            "10".to_owned(),
            "parser".to_owned(),
        ])
        .expect("filter and value-taking flags parse");
        assert_eq!(with_filter.as_deref(), Some("parser"));
    }
}
