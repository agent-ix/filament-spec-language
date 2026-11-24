// SPDX-License-Identifier: AGPL-3.0-or-later
//! Print fresh Criterion estimates in milliseconds per package and scaling
//! exponents for the NFR-141 package axes.

use std::error::Error;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const MARKER: &str = "package-run-start-nanos";
const SIZES: [usize; 3] = [1, 10, 100];
const AXES: [&str; 3] = ["package/emit", "package/read", "replay/request"];

#[ix_trace_rs::string_edge]
fn main() -> Result<(), Box<dyn Error>> {
    let target = std::env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("target"));
    run(&target, std::env::args().skip(1))
}

#[ix_trace_rs::string_edge]
fn run(target: &Path, arguments: impl Iterator<Item = String>) -> Result<(), Box<dyn Error>> {
    let mut arguments = arguments;
    match arguments
        .next()
        .as_deref()
        .ok_or_else(|| io::Error::other("expected start or report"))?
    {
        "start" => {
            fs::create_dir_all(target)?;
            let probe_report = target.join("package-probe-report");
            if probe_report.exists() {
                fs::remove_dir_all(probe_report)?;
            }
            let nanos = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
            fs::write(target.join(MARKER), nanos.to_string())?;
            Ok(())
        }
        "report" => report(target, None),
        "report-axis" => {
            let axis = arguments
                .next()
                .ok_or_else(|| io::Error::other("report-axis requires one axis name"))?;
            report(target, Some(axis.as_str()))
        }
        command => Err(io::Error::other(format!("unknown command: {command}")).into()),
    }
}

fn report(target: &Path, only_axis: Option<&str>) -> Result<(), Box<dyn Error>> {
    let marker = fs::read_to_string(target.join(MARKER))?.parse::<u128>()?;
    if let Some(axis) = only_axis {
        if !AXES.contains(&axis) {
            return Err(io::Error::other(format!("unknown package axis: {axis}")).into());
        }
    }
    println!("NFR-141 live Criterion estimates");
    println!("axis M unit estimate");
    let mut missing = Vec::new();
    for axis in AXES {
        if only_axis.is_some_and(|selected| selected != axis) {
            continue;
        }
        let mut rows = Vec::new();
        for size in SIZES {
            let Some(nanos) = estimate(target, axis, size, marker)? else {
                missing.push(format!("{axis}/{size}"));
                continue;
            };
            let millis = nanos / 1_000_000.0;
            let unit = if axis == "replay/request" {
                "ms/request"
            } else {
                "ms/package"
            };
            println!("{axis} {size} {unit} {millis:.6}");
            rows.push((size, millis));
        }
        if rows.len() == SIZES.len() {
            let first = rows[0];
            let last = rows[rows.len() - 1];
            let exponent = (last.1 / first.1).ln() / (last.0 as f64 / first.0 as f64).ln();
            println!("{axis} exponent {}->{} {exponent:.6}", first.0, last.0);
        } else if rows.len() > 1 {
            return Err(io::Error::other(format!(
                "{axis} scaling exponent needs all M sizes; missing estimates: {missing:?}"
            ))
            .into());
        }
    }
    let probes = target.join("package-probe-report");
    for size in SIZES {
        let probe_path = probes.join(format!("replay-{size}"));
        match fs::read_to_string(probe_path) {
            Ok(probe) if complete_probe_report(&probe) => {
                println!("replay/request isolated {probe}");
            }
            _ => missing.push(format!("replay/request probe M={size}")),
        }
    }
    if !missing.is_empty() {
        return Err(io::Error::other(format!(
            "incomplete NFR-141 report; missing fresh rows: {missing:?}"
        ))
        .into());
    }
    Ok(())
}

fn complete_probe_report(report: &str) -> bool {
    let field = |name: &str| {
        report.split_whitespace().find_map(|part| {
            part.strip_prefix(name)
                .and_then(|value| value.strip_prefix('='))
        })
    };
    field("peak_rss_kib")
        .and_then(|value| value.parse::<u64>().ok())
        .is_some_and(|value| value > 0)
        && field("parse_calls_per_source_document")
            .and_then(|value| value.parse::<u64>().ok())
            .is_some_and(|value| value > 0)
        && field("source_documents") == Some("1")
        && field("succeeded") == Some("true")
}

fn estimate(
    target: &Path,
    axis: &str,
    size: usize,
    marker: u128,
) -> Result<Option<f64>, Box<dyn Error>> {
    let path = target
        .join("criterion")
        .join(axis.replace('/', "_"))
        .join(size.to_string())
        .join("new/estimates.json");
    let Ok(metadata) = fs::metadata(&path) else {
        return Ok(None);
    };
    if metadata.modified()?.duration_since(UNIX_EPOCH)?.as_nanos() <= marker {
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
    use super::complete_probe_report;
    use ix_trace_rs::trace;

    #[trace("NFR-141-AC-4")]
    #[test]
    fn isolated_replay_probe_report_requires_numeric_peak_rss() {
        let measured = "M=1 wall_ms=12.500 peak_rss_kib=32768 parse_calls_per_source_document=1 source_documents=1 succeeded=true";
        assert!(complete_probe_report(measured));

        let unavailable = "M=1 wall_ms=12.500 peak_rss_kib=unavailable parse_calls_per_source_document=1 source_documents=1 succeeded=true";
        assert!(!complete_probe_report(unavailable));

        let missing = "M=1 wall_ms=12.500 parse_calls_per_source_document=1 source_documents=1 succeeded=true";
        assert!(!complete_probe_report(missing));
    }
}
