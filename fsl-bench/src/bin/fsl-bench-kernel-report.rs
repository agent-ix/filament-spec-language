// SPDX-License-Identifier: AGPL-3.0-or-later
//! Print per-frame and per-operation kernel estimates from the latest run.

use fsl_bench::kernel::KernelKind;
use std::error::Error;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const MARKER: &str = "evaluator-kernel-run-start-nanos";

fn main() -> Result<(), Box<dyn Error>> {
    let target = std::env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("target"));
    let command = std::env::args()
        .nth(1)
        .ok_or_else(|| io::Error::other("expected start or report"))?;
    match command.as_str() {
        "start" => mark_start(&target),
        "report" => report(&target),
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
    let criterion = target.join("criterion/evaluator_kernel");
    println!("evaluator/kernel: live Criterion estimates");
    println!("kernel size frames ops interpreter ns/frame interpreter ns/op Rust ns/frame Rust ns/op ratio");
    let mut rows = 0;
    for kind in [
        KernelKind::ArithmeticRecursion,
        KernelKind::ComparisonHeavy,
        KernelKind::CollectionFold,
    ] {
        let name = kind.name();
        for size in [5_usize, 10] {
            let interpreter = estimate(&criterion, "interpreter", name, size, marker)?;
            let rust = estimate(&criterion, "rust", name, size, marker)?;
            let (Some(interpreter), Some(rust)) = (interpreter, rust) else {
                continue;
            };
            let frames = kind.frame_count(size);
            let operations = kind.operation_count(size);
            let interpreter_per_frame = interpreter / frames as f64;
            let interpreter_per_operation = interpreter / operations as f64;
            let rust_per_frame = rust / frames as f64;
            let rust_per_operation = rust / operations as f64;
            println!(
                "{name} {size} {frames} {operations} {:.2} {:.2} {:.2} {:.2} {:.2}x",
                interpreter_per_frame,
                interpreter_per_operation,
                rust_per_frame,
                rust_per_operation,
                interpreter_per_operation / rust_per_operation,
            );
            rows += 1;
        }
    }
    if rows == 0 {
        return Err(io::Error::other("no fresh evaluator/kernel Criterion estimates found").into());
    }
    Ok(())
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
