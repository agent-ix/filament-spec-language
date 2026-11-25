// SPDX-License-Identifier: AGPL-3.0-or-later
//! Run and report the NFR-141 package A/B measurement session.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::error::Error;
use std::ffi::OsStr;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

const SIZES: [usize; 3] = [1, 10, 100];
const AXES: [&str; 3] = ["package/emit", "package/read", "replay/request"];
const ROUNDS: u8 = 5;
const MARKER: &str = "package-run-start-nanos";

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
        .ok_or_else(|| io::Error::other("expected start, report, run-ab, or report-ab"))?
    {
        "start" => {
            fs::create_dir_all(target)?;
            let probe_report = target.join("package-probe-report");
            if probe_report.exists() {
                fs::remove_dir_all(probe_report)?;
            }
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_nanos();
            fs::write(target.join(MARKER), nanos.to_string())?;
            Ok(())
        }
        "report" => report_single(target, None),
        "report-axis" => {
            let axis = arguments
                .next()
                .ok_or_else(|| io::Error::other("report-axis requires one axis name"))?;
            report_single(target, Some(&axis))
        }
        "run-ab" => {
            let a_target = required_path(&mut arguments, "A target directory")?;
            let b_target = required_path(&mut arguments, "B target directory")?;
            let a_result = required_path(&mut arguments, "A result file")?;
            let b_result = required_path(&mut arguments, "B result file")?;
            run_ab(&a_target, &b_target, &a_result, &b_result).map_err(io::Error::other)?;
            Ok(())
        }
        "report-ab" => {
            let a_result = required_path(&mut arguments, "A result file")?;
            let b_result = required_path(&mut arguments, "B result file")?;
            report_ab_files(&a_result, &b_result).map_err(io::Error::other)?;
            Ok(())
        }
        command => Err(io::Error::other(format!("unknown command: {command}")).into()),
    }
}

fn required_path(
    arguments: &mut impl Iterator<Item = String>,
    label: &str,
) -> Result<PathBuf, io::Error> {
    arguments
        .next()
        .map(PathBuf::from)
        .ok_or_else(|| io::Error::other(format!("missing {label}")))
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct SideResults {
    side: String,
    rounds: Vec<RoundResult>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct LoadAverage {
    one_minute: f64,
    five_minutes: f64,
    fifteen_minutes: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct RoundResult {
    number: u8,
    load_before: Measurement<LoadAverage>,
    load_after: Measurement<LoadAverage>,
    axes: BTreeMap<String, BTreeMap<usize, Measurement<CriterionEstimate>>>,
    probes: BTreeMap<usize, Measurement<ProbeEstimate>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Measurement<T> {
    value: Option<T>,
    error: Option<String>,
    output: Option<String>,
}

impl<T> Measurement<T> {
    fn success(value: T) -> Self {
        Self {
            value: Some(value),
            error: None,
            output: None,
        }
    }

    fn failure(error: impl Into<String>, output: impl Into<String>) -> Self {
        Self {
            value: None,
            error: Some(error.into()),
            output: Some(output.into()),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct CriterionEstimate {
    milliseconds: f64,
    ci_lower_milliseconds: f64,
    ci_upper_milliseconds: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct ProbeEstimate {
    milliseconds: f64,
    peak_rss_kib: u64,
    parse_calls_per_source_document: u64,
    source_documents: u64,
}

fn run_ab(
    a_target: &Path,
    b_target: &Path,
    a_result_path: &Path,
    b_result_path: &Path,
) -> Result<(), String> {
    if a_result_path.exists() || b_result_path.exists() {
        return Err(
            "refusing to overwrite existing A/B result files; start a new session with new paths"
                .to_owned(),
        );
    }
    let a_bench = find_executable(&a_target.join("release/deps"), "package-")?;
    let b_bench = find_executable(&b_target.join("release/deps"), "package-")?;
    let a_probe = find_executable(&a_target.join("release"), "fsl-bench-probe")?;
    let b_probe = find_executable(&b_target.join("release"), "fsl-bench-probe")?;
    let mut a_results = SideResults {
        side: "A".to_owned(),
        rounds: Vec::new(),
    };
    let mut b_results = SideResults {
        side: "B".to_owned(),
        rounds: Vec::new(),
    };
    let a_criterion_home = a_result_path.with_extension("criterion");
    let b_criterion_home = b_result_path.with_extension("criterion");
    fs::create_dir_all(&a_criterion_home)
        .map_err(|error| format!("create A Criterion directory: {error}"))?;
    fs::create_dir_all(&b_criterion_home)
        .map_err(|error| format!("create B Criterion directory: {error}"))?;
    let mut run_round = |side: &str, round: u8| match side {
        "A" => run_side_round(
            side,
            round,
            &a_bench,
            &a_probe,
            &a_criterion_home,
            a_result_path,
            &[
                ("package/emit", "package/emit"),
                ("package/read", "package/read"),
                ("replay/request", "replay/request"),
            ],
            &SIZES,
            true,
        ),
        "B" => run_side_round(
            side,
            round,
            &b_bench,
            &b_probe,
            &b_criterion_home,
            b_result_path,
            &[
                ("package/emit", "package/emit"),
                ("package/read", "package/read"),
                ("replay/request", "replay/request"),
            ],
            &SIZES,
            true,
        ),
        _ => unreachable!("the scheduled sides are A and B"),
    };
    run_scheduled_rounds(
        a_result_path,
        b_result_path,
        &mut a_results,
        &mut b_results,
        &mut run_round,
    )
}

fn run_scheduled_rounds(
    a_result_path: &Path,
    b_result_path: &Path,
    a_results: &mut SideResults,
    b_results: &mut SideResults,
    run_round: &mut impl FnMut(&str, u8) -> RoundResult,
) -> Result<(), String> {
    persist_results(a_result_path, a_results)?;
    persist_results(b_result_path, b_results)?;
    for round in 1..=ROUNDS {
        a_results.rounds.push(run_round("A", round));
        persist_results(a_result_path, a_results)?;
        b_results.rounds.push(run_round("B", round));
        persist_results(b_result_path, b_results)?;
    }
    report_ab(a_results, b_results)
}

fn run_side_round(
    side: &str,
    number: u8,
    bench: &Path,
    probe: &Path,
    criterion_home: &Path,
    result_path: &Path,
    axis_filters: &[(&str, &str)],
    sizes: &[usize],
    include_replay_probes: bool,
) -> RoundResult {
    let load_before = load_average()
        .map(Measurement::success)
        .unwrap_or_else(|error| Measurement::failure(error, ""));
    let mut axes = BTreeMap::new();
    for &(axis, filter) in axis_filters {
        let baseline = format!("package-{side}-round-{number}-{}", axis.replace('/', "-"));
        let output = Command::new(bench)
            .arg("--bench")
            .arg(filter)
            .arg("--save-baseline")
            .arg(&baseline)
            .env("CRITERION_HOME", criterion_home)
            .output();
        let mut estimates = BTreeMap::new();
        match output {
            Ok(output) if output.status.success() => {
                for &size in sizes {
                    let result = criterion_estimate(criterion_home, axis, size, &baseline)
                        .map(Measurement::success)
                        .unwrap_or_else(|error| {
                            Measurement::failure(error, String::from_utf8_lossy(&output.stdout))
                        });
                    estimates.insert(size, result);
                }
            }
            Ok(output) => {
                let diagnostic = format!(
                    "exit={} stderr={} stdout={}",
                    output.status,
                    String::from_utf8_lossy(&output.stderr),
                    String::from_utf8_lossy(&output.stdout)
                );
                for &size in sizes {
                    estimates.insert(
                        size,
                        Measurement::failure("Criterion benchmark failed", diagnostic.clone()),
                    );
                }
            }
            Err(error) => {
                for &size in sizes {
                    estimates.insert(
                        size,
                        Measurement::failure(format!("start Criterion benchmark: {error}"), ""),
                    );
                }
            }
        }
        axes.insert((*axis).to_owned(), estimates);
    }
    let mut probes = BTreeMap::new();
    if include_replay_probes {
        for &size in sizes {
            let probe_report =
                result_path.with_extension(format!("{side}-round-{number}-m-{size}.probe"));
            let output = Command::new(probe)
                .arg("package")
                .arg("replay/request")
                .arg(size.to_string())
                .env("FSL_PACKAGE_PROBE_REPORT", &probe_report)
                .output();
            let measurement = match output {
                Ok(output) if output.status.success() => {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    parse_probe_estimate(&stdout, size)
                        .map(Measurement::success)
                        .unwrap_or_else(|error| Measurement::failure(error, stdout.to_string()))
                }
                Ok(output) => Measurement::failure(
                    format!("replay RSS probe exited {}", output.status),
                    format!(
                        "stderr={} stdout={}",
                        String::from_utf8_lossy(&output.stderr),
                        String::from_utf8_lossy(&output.stdout)
                    ),
                ),
                Err(error) => Measurement::failure(format!("start replay RSS probe: {error}"), ""),
            };
            probes.insert(size, measurement);
        }
    }
    let load_after = load_average()
        .map(Measurement::success)
        .unwrap_or_else(|error| Measurement::failure(error, ""));
    RoundResult {
        number,
        load_before,
        load_after,
        axes,
        probes,
    }
}

fn find_executable(directory: &Path, prefix: &str) -> Result<PathBuf, String> {
    let entries = fs::read_dir(directory)
        .map_err(|error| format!("read executable directory {}: {error}", directory.display()))?;
    let candidates = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(OsStr::to_str)
                .is_some_and(|name| name.starts_with(prefix))
                && path.extension().is_none()
                && path.is_file()
        })
        .collect::<Vec<_>>();
    match candidates.as_slice() {
        [path] => Ok(path.clone()),
        [] => Err(format!(
            "no executable beginning {prefix:?} in {}",
            directory.display()
        )),
        _ => Err(format!(
            "multiple executables beginning {prefix:?} in {}",
            directory.display()
        )),
    }
}

fn criterion_estimate(
    root: &Path,
    axis: &str,
    size: usize,
    baseline: &str,
) -> Result<CriterionEstimate, String> {
    let path = root
        .join(axis.replace('/', "_"))
        .join(size.to_string())
        .join(baseline)
        .join("estimates.json");
    let bytes = fs::read(&path).map_err(|error| format!("read {}: {error}", path.display()))?;
    let value: serde_json::Value = serde_json::from_slice(&bytes)
        .map_err(|error| format!("decode {}: {error}", path.display()))?;
    let mean = &value["mean"];
    let nanos = mean["point_estimate"]
        .as_f64()
        .ok_or_else(|| format!("{} has no mean point estimate", path.display()))?;
    let lower = mean["confidence_interval"]["lower_bound"]
        .as_f64()
        .ok_or_else(|| format!("{} has no mean CI lower bound", path.display()))?;
    let upper = mean["confidence_interval"]["upper_bound"]
        .as_f64()
        .ok_or_else(|| format!("{} has no mean CI upper bound", path.display()))?;
    Ok(CriterionEstimate {
        milliseconds: nanos / 1_000_000.0,
        ci_lower_milliseconds: lower / 1_000_000.0,
        ci_upper_milliseconds: upper / 1_000_000.0,
    })
}

fn parse_probe_estimate(output: &str, size: usize) -> Result<ProbeEstimate, String> {
    let row = output
        .lines()
        .find(|line| line.starts_with("axis=replay/request "))
        .ok_or_else(|| "probe output has no replay/request row".to_owned())?;
    let mut fields = BTreeMap::new();
    for part in row.split_whitespace() {
        if let Some((name, value)) = part.split_once('=') {
            fields.insert(name, value);
        }
    }
    let number = |name: &str| {
        fields
            .get(name)
            .ok_or_else(|| format!("probe report is missing {name}"))?
            .parse::<f64>()
            .map_err(|error| format!("probe report {name} is invalid: {error}"))
    };
    let exact = |name: &str, expected: &str| fields.get(name).copied() == Some(expected);
    let probe_size = fields
        .get("M")
        .ok_or_else(|| "probe report is missing M".to_owned())?
        .parse::<usize>()
        .map_err(|error| format!("probe report M is invalid: {error}"))?;
    if probe_size != size || !exact("source_documents", "1") || !exact("succeeded", "true") {
        return Err(format!(
            "probe report has wrong size or unsuccessful result: {row}"
        ));
    }
    let peak_rss_kib = fields
        .get("peak_rss_kib")
        .ok_or_else(|| "probe report is missing peak_rss_kib".to_owned())?
        .parse::<u64>()
        .map_err(|error| format!("probe report peak_rss_kib is invalid: {error}"))?;
    let parse_calls_per_source_document = fields
        .get("parse_calls_per_source_document")
        .ok_or_else(|| "probe report is missing parse_calls_per_source_document".to_owned())?
        .parse::<u64>()
        .map_err(|error| format!("probe report parse-call count is invalid: {error}"))?;
    if peak_rss_kib == 0 || parse_calls_per_source_document == 0 {
        return Err(format!(
            "probe report contains a zero RSS or parse-call value: {row}"
        ));
    }
    Ok(ProbeEstimate {
        milliseconds: number("wall_ms")?,
        peak_rss_kib,
        parse_calls_per_source_document,
        source_documents: 1,
    })
}

fn load_average() -> Result<LoadAverage, String> {
    let output = Command::new("uptime")
        .output()
        .map_err(|error| format!("read uptime: {error}"))?;
    if !output.status.success() {
        return Err(format!("uptime exited {}", output.status));
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let values = text
        .split_once("load averages:")
        .or_else(|| text.split_once("load average:"))
        .map(|(_, tail)| tail)
        .ok_or_else(|| format!("uptime has no load average: {text}"))?
        .split(|ch: char| ch == ',' || ch.is_whitespace())
        .filter_map(|part| part.parse::<f64>().ok())
        .take(3)
        .collect::<Vec<_>>();
    let [one_minute, five_minutes, fifteen_minutes] = values.as_slice() else {
        return Err(format!("uptime has fewer than three load averages: {text}"));
    };
    Ok(LoadAverage {
        one_minute: *one_minute,
        five_minutes: *five_minutes,
        fifteen_minutes: *fifteen_minutes,
    })
}

fn persist_results(path: &Path, result: &SideResults) -> Result<(), String> {
    let bytes = serde_json::to_vec_pretty(result)
        .map_err(|error| format!("serialize {}: {error}", path.display()))?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("create {}: {error}", parent.display()))?;
    }
    let temporary = path.with_extension("json.tmp");
    fs::write(&temporary, bytes)
        .map_err(|error| format!("write {}: {error}", temporary.display()))?;
    fs::rename(&temporary, path).map_err(|error| {
        format!(
            "rename {} to {}: {error}",
            temporary.display(),
            path.display()
        )
    })
}

fn report_ab_files(a_path: &Path, b_path: &Path) -> Result<(), String> {
    let a: SideResults = read_results(a_path)?;
    let b: SideResults = read_results(b_path)?;
    report_ab(&a, &b)
}

fn read_results(path: &Path) -> Result<SideResults, String> {
    let bytes = fs::read(path).map_err(|error| format!("read {}: {error}", path.display()))?;
    serde_json::from_slice(&bytes).map_err(|error| format!("decode {}: {error}", path.display()))
}

fn report_ab(a: &SideResults, b: &SideResults) -> Result<(), String> {
    if a.side != "A" || b.side != "B" {
        return Err("A/B result files must identify sides A and B".to_owned());
    }
    validate_rounds(a)?;
    validate_rounds(b)?;
    let mut failures = Vec::new();
    println!("round load averages before/after (A and B; 1/5/15 minutes)");
    for index in 0..usize::from(ROUNDS) {
        let ar = &a.rounds[index];
        let br = &b.rounds[index];
        println!(
            "round={} A={:?}->{:?} B={:?}->{:?}",
            ar.number,
            ar.load_before.value,
            ar.load_after.value,
            br.load_before.value,
            br.load_after.value
        );
        for (side, measurement) in [
            ("A before", &ar.load_before),
            ("A after", &ar.load_after),
            ("B before", &br.load_before),
            ("B after", &br.load_after),
        ] {
            if let Some(error) = &measurement.error {
                failures.push(format!("round {} load {side}: {error}", ar.number));
            }
        }
    }
    for axis in AXES {
        for size in SIZES {
            let a_values = metric_values(a, axis, size, &mut failures);
            let b_values = metric_values(b, axis, size, &mut failures);
            if let (Some(a_values), Some(b_values)) = (a_values, b_values) {
                let a_ms = a_values
                    .iter()
                    .map(|estimate| estimate.milliseconds)
                    .collect::<Vec<_>>();
                let b_ms = b_values
                    .iter()
                    .map(|estimate| estimate.milliseconds)
                    .collect::<Vec<_>>();
                print_round_values(axis, size, &a_ms, &b_ms);
                let noise = session_noise(&a_ms, &b_ms, |round| {
                    let ai = &a_values[round];
                    let bi = &b_values[round];
                    max_ci_ratio(ai).max(max_ci_ratio(bi))
                });
                let a_median = median(&a_ms);
                let b_median = median(&b_ms);
                let decision = within_noise(a_median, b_median, noise);
                println!(
                    "{}",
                    metric_summary_line(axis, size, a_median, b_median, noise)
                );
                if !decision {
                    failures.push(format!(
                        "{axis} M={size} exceeds A median plus session noise"
                    ));
                }
            }
        }
        if axis == "package/emit" || axis == "package/read" {
            report_exponents(a, b, axis, &mut failures);
        }
    }
    for size in SIZES {
        let a_values = probe_values(a, size, &mut failures);
        let b_values = probe_values(b, size, &mut failures);
        if let (Some(a_values), Some(b_values)) = (a_values, b_values) {
            for round in 0..usize::from(ROUNDS) {
                println!(
                    "replay/request peak-rss M={size} round={} A_kib={} B_kib={}",
                    round + 1,
                    a_values[round].peak_rss_kib,
                    b_values[round].peak_rss_kib
                );
            }
            let a_rss = a_values
                .iter()
                .map(|probe| probe.peak_rss_kib as f64)
                .collect::<Vec<_>>();
            let b_rss = b_values
                .iter()
                .map(|probe| probe.peak_rss_kib as f64)
                .collect::<Vec<_>>();
            let a_median = median(&a_rss);
            let b_median = median(&b_rss);
            let noise = mad(&a_rss) / a_median.max(f64::MIN_POSITIVE);
            let noise = noise.max(mad(&b_rss) / b_median.max(f64::MIN_POSITIVE));
            let decision = within_noise(a_median, b_median, noise);
            println!("{}", rss_summary_line(a_median, b_median, noise));
            if !decision {
                failures.push(format!(
                    "replay/request M={size} exceeds A peak RSS plus session noise"
                ));
            }
            for (round, (a_probe, b_probe)) in a_values.iter().zip(&b_values).enumerate() {
                if b_probe.parse_calls_per_source_document > a_probe.parse_calls_per_source_document
                {
                    failures.push(format!(
                        "AC-5 replay/request M={size} round={} B parse calls exceed A",
                        round + 1
                    ));
                }
            }
            println!(
                "AC-5 replay/request M={size} parse_calls_per_source_document A={:?} B={:?}",
                a_values
                    .iter()
                    .map(|probe| probe.parse_calls_per_source_document)
                    .collect::<Vec<_>>(),
                b_values
                    .iter()
                    .map(|probe| probe.parse_calls_per_source_document)
                    .collect::<Vec<_>>()
            );
        }
    }
    if !failures.is_empty() {
        for failure in &failures {
            eprintln!("A/B evidence invalid or failed: {failure}");
        }
        return Err(format!(
            "{} A/B evidence failures; no result was replaced: {}",
            failures.len(),
            failures.join("; ")
        ));
    }
    Ok(())
}

fn validate_rounds(results: &SideResults) -> Result<(), String> {
    if results.rounds.len() != usize::from(ROUNDS) {
        return Err(format!(
            "side {} has {} rounds, expected {ROUNDS}",
            results.side,
            results.rounds.len()
        ));
    }
    for (index, round) in results.rounds.iter().enumerate() {
        if round.number != u8::try_from(index + 1).unwrap_or(u8::MAX) {
            return Err(format!(
                "side {} has duplicate or unordered rounds",
                results.side
            ));
        }
    }
    Ok(())
}

fn metric_values<'a>(
    side: &'a SideResults,
    axis: &str,
    size: usize,
    failures: &mut Vec<String>,
) -> Option<Vec<&'a CriterionEstimate>> {
    let mut values = Vec::new();
    for round in &side.rounds {
        match round.axes.get(axis).and_then(|sizes| sizes.get(&size)) {
            Some(Measurement {
                value: Some(value),
                error: None,
                ..
            }) => values.push(value),
            Some(Measurement {
                error: Some(error),
                output,
                ..
            }) => {
                failures.push(format!(
                    "side {} {axis} M={size} round {}: {error}; output={output:?}",
                    side.side, round.number
                ));
            }
            _ => failures.push(format!(
                "side {} missing {axis} M={size} round {}",
                side.side, round.number
            )),
        }
    }
    (values.len() == usize::from(ROUNDS)).then_some(values)
}

fn probe_values<'a>(
    side: &'a SideResults,
    size: usize,
    failures: &mut Vec<String>,
) -> Option<Vec<&'a ProbeEstimate>> {
    let mut values = Vec::new();
    for round in &side.rounds {
        match round.probes.get(&size) {
            Some(Measurement {
                value: Some(value),
                error: None,
                ..
            }) => values.push(value),
            Some(Measurement {
                error: Some(error),
                output,
                ..
            }) => {
                failures.push(format!(
                    "side {} replay/request RSS M={size} round {}: {error}; output={output:?}",
                    side.side, round.number
                ));
            }
            _ => failures.push(format!(
                "side {} missing replay/request RSS M={size} round {}",
                side.side, round.number
            )),
        }
    }
    (values.len() == usize::from(ROUNDS)).then_some(values)
}

fn print_round_values(axis: &str, size: usize, a: &[f64], b: &[f64]) {
    for round in 0..usize::from(ROUNDS) {
        println!(
            "{axis} M={size} round={} A={:.6} B={:.6} unit={}",
            round + 1,
            a[round],
            b[round],
            estimate_unit(axis)
        );
    }
}

fn estimate_unit(axis: &str) -> &'static str {
    if axis == "replay/request" {
        "ms/request"
    } else {
        "ms/package"
    }
}

fn metric_summary_line(
    axis: &str,
    size: usize,
    a_median: f64,
    b_median: f64,
    noise: f64,
) -> String {
    let decision = within_noise(a_median, b_median, noise);
    format!(
        "{axis} M={size} A_median={a_median:.6} B_median={b_median:.6} unit={} noise={:.2}% {}",
        estimate_unit(axis),
        noise * 100.0,
        if decision { "pass" } else { "fail" }
    )
}

fn within_noise(a_median: f64, b_median: f64, noise: f64) -> bool {
    b_median <= a_median * (1.0 + noise)
}

fn rss_summary_line(a_median_kib: f64, b_median_kib: f64, noise: f64) -> String {
    format!(
        "replay/request peak-rss A_median_kib={a_median_kib:.0} B_median_kib={b_median_kib:.0} noise={:.2}% {}",
        noise * 100.0,
        if within_noise(a_median_kib, b_median_kib, noise) {
            "pass"
        } else {
            "fail"
        }
    )
}

fn report_exponents(a: &SideResults, b: &SideResults, axis: &str, failures: &mut Vec<String>) {
    let mut a_exponents = Vec::new();
    let mut b_exponents = Vec::new();
    for round_index in 0..usize::from(ROUNDS) {
        let get = |side: &SideResults, size: usize| {
            side.rounds[round_index]
                .axes
                .get(axis)
                .and_then(|sizes| sizes.get(&size))
                .and_then(|measurement| measurement.value.as_ref())
                .map(|estimate| estimate.milliseconds)
        };
        match (
            get(a, 1),
            get(a, 10),
            get(a, 100),
            get(b, 1),
            get(b, 10),
            get(b, 100),
        ) {
            (
                Some(a_one),
                Some(a_ten),
                Some(a_hundred),
                Some(b_one),
                Some(b_ten),
                Some(b_hundred),
            ) if [a_one, a_ten, a_hundred, b_one, b_ten, b_hundred]
                .into_iter()
                .all(|estimate| estimate > 0.0) =>
            {
                let a_exp = scaling_exponent([a_one, a_ten, a_hundred]);
                let b_exp = scaling_exponent([b_one, b_ten, b_hundred]);
                a_exponents.push(a_exp);
                b_exponents.push(b_exp);
                println!(
                    "{axis} scaling exponent M=1->10->100 round={} A={a_exp:.6} B={b_exp:.6}",
                    round_index + 1
                );
            }
            _ => failures.push(format!(
                "cannot compute {axis} scaling exponent for round {}",
                round_index + 1
            )),
        }
    }
    if a_exponents.len() == usize::from(ROUNDS) && b_exponents.len() == usize::from(ROUNDS) {
        println!(
            "{}",
            exponent_summary_line(axis, median(&a_exponents), median(&b_exponents))
        );
    }
}

fn exponent_summary_line(axis: &str, a_exponent: f64, b_exponent: f64) -> String {
    format!("{axis} scaling exponent median M=1->10->100 A={a_exponent:.6} B={b_exponent:.6}")
}

fn scaling_exponent(estimates_ms: [f64; 3]) -> f64 {
    let sizes = [1.0_f64, 10.0, 100.0];
    let log_sizes = sizes.map(f64::ln);
    let log_estimates = estimates_ms.map(f64::ln);
    let mean_log_size = log_sizes.iter().sum::<f64>() / 3.0;
    let mean_log_estimate = log_estimates.iter().sum::<f64>() / 3.0;
    let covariance = log_sizes
        .iter()
        .zip(log_estimates.iter())
        .map(|(size, estimate)| (size - mean_log_size) * (estimate - mean_log_estimate))
        .sum::<f64>();
    let variance = log_sizes
        .iter()
        .map(|size| (size - mean_log_size).powi(2))
        .sum::<f64>();
    covariance / variance
}

fn median(values: &[f64]) -> f64 {
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let middle = sorted.len() / 2;
    if sorted.len() % 2 == 0 {
        (sorted[middle - 1] + sorted[middle]) / 2.0
    } else {
        sorted[middle]
    }
}

fn mad(values: &[f64]) -> f64 {
    let middle = median(values);
    let deviations = values
        .iter()
        .map(|value| (value - middle).abs())
        .collect::<Vec<_>>();
    median(&deviations)
}

fn max_ci_ratio(estimate: &CriterionEstimate) -> f64 {
    let half_width = (estimate.ci_upper_milliseconds - estimate.ci_lower_milliseconds).abs() / 2.0;
    half_width / estimate.milliseconds.max(f64::MIN_POSITIVE)
}

fn session_noise(a: &[f64], b: &[f64], ci_ratio: impl Fn(usize) -> f64) -> f64 {
    let a_median = median(a).max(f64::MIN_POSITIVE);
    let b_median = median(b).max(f64::MIN_POSITIVE);
    let ci = (0..a.len()).map(ci_ratio).fold(0.0, f64::max);
    (mad(a) / a_median).max(mad(b) / b_median).max(ci)
}

fn report_single(target: &Path, only_axis: Option<&str>) -> Result<(), Box<dyn Error>> {
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
            let Some(nanos) = single_estimate(target, axis, size, marker)? else {
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
        }
    }
    for size in SIZES {
        let path = target
            .join("package-probe-report")
            .join(format!("replay-{size}"));
        match fs::read_to_string(path) {
            Ok(row) if parse_probe_estimate(&row, size).is_ok() => {
                println!("replay/request isolated {row}")
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

fn single_estimate(
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
    if metadata
        .modified()?
        .duration_since(std::time::UNIX_EPOCH)?
        .as_nanos()
        <= marker
    {
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
    use super::{
        complete_probe_report, exponent_summary_line, median, metric_summary_line, report_ab_files,
        scaling_exponent, session_noise, CriterionEstimate, LoadAverage, Measurement,
        ProbeEstimate, RoundResult, SideResults, AXES, ROUNDS, SIZES,
    };
    use ix_trace_rs::trace;
    use std::collections::BTreeMap;
    use std::path::{Path, PathBuf};

    #[trace("NFR-141-AC-4")]
    #[test]
    fn isolated_replay_probe_report_requires_numeric_peak_rss() {
        let measured = "axis=replay/request M=1 wall_ms=12.500 peak_rss_kib=32768 parse_calls_per_source_document=1 source_documents=1 succeeded=true";
        assert!(complete_probe_report(measured));
        let unavailable = "axis=replay/request M=1 wall_ms=12.500 peak_rss_kib=unavailable parse_calls_per_source_document=1 source_documents=1 succeeded=true";
        assert!(!complete_probe_report(unavailable));
    }

    #[trace("NFR-141-AC-1", "NFR-141-AC-2")]
    #[test]
    fn report_uses_five_round_median_and_full_m_range_exponent() {
        let estimates = [10.0, 20.0, 100.0, 200.0, 1000.0];
        assert_eq!(median(&estimates), 100.0);
        assert!((scaling_exponent([1.0, 10.0, 100.0]) - 1.0).abs() < 1e-12);
        assert!((scaling_exponent([1.0, 100.0, 10_000.0]) - 2.0).abs() < 1e-12);
        assert!((scaling_exponent([1.0, 10.0, 10_000.0]) - 2.0).abs() < 1e-12);

        let a_times = [98.0, 99.0, 100.0, 101.0, 102.0];
        let b_times = [97.0, 98.0, 99.0, 100.0, 101.0];
        let a_ci = CriterionEstimate {
            milliseconds: 100.0,
            ci_lower_milliseconds: 98.0,
            ci_upper_milliseconds: 102.0,
        };
        let b_ci = CriterionEstimate {
            milliseconds: 99.0,
            ci_lower_milliseconds: 97.02,
            ci_upper_milliseconds: 100.98,
        };
        let noise = session_noise(&a_times, &b_times, |_| {
            super::max_ci_ratio(&a_ci).max(super::max_ci_ratio(&b_ci))
        });
        assert!((noise - 0.02).abs() < 1e-12, "CI width determines noise");
        assert!(!super::within_noise(100.0, 103.0, noise));
        assert!(!super::within_noise(50_000.0, 51_500.0, noise));
        assert!(metric_summary_line("package/emit", 1, 100.0, 103.0, noise).ends_with("fail"));
        assert!(super::rss_summary_line(50_000.0, 51_500.0, noise).ends_with("fail"));
    }

    #[trace("NFR-141-AC-1")]
    #[test]
    fn run_side_round_executes_bench_mode_and_reads_the_selected_estimate() {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let workspace_root = manifest_dir.parent().expect("FSL workspace root");
        let target_dir = std::env::var_os("CARGO_TARGET_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| workspace_root.join("target"));
        let build = std::process::Command::new("cargo")
            .args([
                "build",
                "--locked",
                "-j",
                "4",
                "-p",
                "fsl-bench",
                "--bench",
                "package",
            ])
            .current_dir(workspace_root)
            .env("CARGO_TARGET_DIR", &target_dir)
            .output()
            .expect("package Criterion binary builds");
        assert!(
            build.status.success(),
            "package bench build failed: stdout={} stderr={}",
            String::from_utf8_lossy(&build.stdout),
            String::from_utf8_lossy(&build.stderr)
        );
        let bench = super::find_executable(&target_dir.join("debug/deps"), "package-")
            .expect("package Criterion executable exists");
        let (root, result_path, _) = result_paths();
        let criterion_home = root.join("criterion");
        std::fs::create_dir_all(&criterion_home).expect("Criterion result directory creates");
        let round = super::run_side_round(
            "A",
            1,
            &bench,
            Path::new("unused-probe"),
            &criterion_home,
            &result_path,
            &[("package/emit", "package/emit/1")],
            &[1],
            false,
        );
        let estimate = round.axes["package/emit"][&1]
            .value
            .as_ref()
            .expect("--bench run writes a Criterion estimate");
        assert!(estimate.milliseconds > 0.0);
        assert!(estimate.ci_upper_milliseconds >= estimate.ci_lower_milliseconds);
        assert!(round.probes.is_empty());
        std::fs::remove_dir_all(root).expect("temporary Criterion artifacts are removed");
    }

    #[trace(
        "NFR-141-AC-1",
        "NFR-141-AC-2",
        "NFR-141-AC-3",
        "NFR-141-AC-4",
        "NFR-141-AC-5"
    )]
    #[test]
    fn report_ab_reads_complete_five_round_matrices_and_summarizes_them() {
        let (root, a_path, b_path) = result_paths();
        let a = synthetic_side("A", 1.0);
        let b = synthetic_side("B", 0.99);
        super::persist_results(&a_path, &a).expect("A result file writes");
        super::persist_results(&b_path, &b).expect("B result file writes");

        assert_eq!(a.rounds.len(), usize::from(ROUNDS));
        assert_eq!(b.rounds.len(), usize::from(ROUNDS));
        for round in a.rounds.iter().chain(&b.rounds) {
            assert!(round.load_before.value.is_some());
            assert!(round.load_after.value.is_some());
            assert_eq!(round.axes.len(), AXES.len());
            for axis in AXES {
                assert_eq!(round.axes[axis].len(), SIZES.len());
            }
            assert_eq!(round.probes.len(), SIZES.len());
        }
        report_ab_files(&a_path, &b_path).expect("complete A/B results report successfully");

        let a_emit_m10 = a
            .rounds
            .iter()
            .map(|round| {
                round.axes["package/emit"][&10]
                    .value
                    .as_ref()
                    .unwrap()
                    .milliseconds
            })
            .collect::<Vec<_>>();
        let b_emit_m10 = b
            .rounds
            .iter()
            .map(|round| {
                round.axes["package/emit"][&10]
                    .value
                    .as_ref()
                    .unwrap()
                    .milliseconds
            })
            .collect::<Vec<_>>();
        let a_estimates = a
            .rounds
            .iter()
            .map(|round| round.axes["package/emit"][&10].value.as_ref().unwrap())
            .collect::<Vec<_>>();
        let b_estimates = b
            .rounds
            .iter()
            .map(|round| round.axes["package/emit"][&10].value.as_ref().unwrap())
            .collect::<Vec<_>>();
        let expected_noise = session_noise(&a_emit_m10, &b_emit_m10, |index| {
            super::max_ci_ratio(a_estimates[index]).max(super::max_ci_ratio(b_estimates[index]))
        });
        assert_eq!(median(&a_emit_m10), 10.0);
        assert_eq!(median(&b_emit_m10), 9.9);
        assert_eq!(
            metric_summary_line(
                "package/emit",
                10,
                median(&a_emit_m10),
                median(&b_emit_m10),
                expected_noise,
            ),
            format!(
                "package/emit M=10 A_median=10.000000 B_median=9.900000 unit=ms/package noise={:.2}% pass",
                expected_noise * 100.0
            )
        );
        assert_eq!(
            scaling_exponent([3.0, 30.0, 300.0]),
            1.0,
            "the 1→10→100 workload has linear scaling"
        );
        assert_eq!(
            exponent_summary_line("package/emit", 1.0, 1.0),
            "package/emit scaling exponent median M=1->10->100 A=1.000000 B=1.000000"
        );

        let loaded_round: RoundResult = super::read_results(&a_path).unwrap().rounds[2].clone();
        assert_eq!(loaded_round.number, 3);
        assert_eq!(
            loaded_round.load_before.value.as_ref().unwrap().one_minute,
            3.0
        );
        assert_eq!(
            loaded_round.load_after.value.as_ref().unwrap().one_minute,
            3.5
        );
        std::fs::remove_dir_all(root).expect("temporary A/B results are removed");
    }

    #[trace("NFR-141-AC-1", "NFR-141-AC-4")]
    #[test]
    fn report_ab_retains_a_failed_loaded_round_and_rejects_substitution() {
        let (root, a_path, b_path) = result_paths();
        let a_base = synthetic_side("A", 1.0);
        let b_base = synthetic_side("B", 1.0);
        let mut a = SideResults {
            side: "A".to_owned(),
            rounds: Vec::new(),
        };
        let mut b = SideResults {
            side: "B".to_owned(),
            rounds: Vec::new(),
        };
        let mut run_round = |side: &str, number: u8| {
            let base = if side == "A" { &a_base } else { &b_base };
            let mut round = base.rounds[usize::from(number - 1)].clone();
            if side == "A" && number == 3 {
                round.load_before = Measurement::success(LoadAverage {
                    one_minute: 97.0,
                    five_minutes: 88.0,
                    fifteen_minutes: 79.0,
                });
                round.axes.get_mut("package/emit").unwrap().insert(
                    10,
                    Measurement::failure("loaded round failed", "load=97.0; preserve this failure"),
                );
            }
            round
        };
        let error = super::run_scheduled_rounds(&a_path, &b_path, &mut a, &mut b, &mut run_round)
            .expect_err("the five-round report rejects the failed round persisted by run_ab");
        assert!(error.contains("A/B evidence failures"));
        let retained = super::read_results(&a_path).expect("A results remain readable");
        assert_eq!(retained.rounds.len(), usize::from(ROUNDS));
        assert_eq!(
            retained.rounds[2]
                .load_before
                .value
                .as_ref()
                .unwrap()
                .one_minute,
            97.0
        );
        let failed = &retained.rounds[2].axes["package/emit"][&10];
        assert_eq!(failed.error.as_deref(), Some("loaded round failed"));
        assert_eq!(
            failed.output.as_deref(),
            Some("load=97.0; preserve this failure")
        );
        assert!(
            failed.value.is_none(),
            "no later value replaced the failure"
        );
        std::fs::remove_dir_all(root).expect("temporary A/B results are removed");
    }

    #[trace("NFR-141-AC-5")]
    #[test]
    fn report_ab_rejects_a_parse_call_regression() {
        let (root, a_path, b_path) = result_paths();
        let a = synthetic_side("A", 1.0);
        let mut b = synthetic_side("B", 1.0);
        b.rounds[0]
            .probes
            .get_mut(&10)
            .unwrap()
            .value
            .as_mut()
            .unwrap()
            .parse_calls_per_source_document = 2;
        super::persist_results(&a_path, &a).expect("A result file writes");
        super::persist_results(&b_path, &b).expect("B result file writes");

        let error = report_ab_files(&a_path, &b_path)
            .expect_err("the parse-call acceptance gate rejects a regression");
        assert!(error.contains("A/B evidence failures"));
        assert_eq!(
            super::read_results(&b_path).unwrap().rounds[0].probes[&10]
                .value
                .as_ref()
                .unwrap()
                .parse_calls_per_source_document,
            2
        );
        std::fs::remove_dir_all(root).expect("temporary A/B results are removed");
    }

    #[trace("NFR-141-AC-1", "NFR-141-AC-3", "NFR-141-AC-4")]
    #[test]
    fn report_ab_rejects_timing_and_rss_regressions_beyond_noise() {
        let (timing_root, timing_a_path, timing_b_path) = result_paths();
        let timing_a = synthetic_side("A", 1.0);
        let timing_b = synthetic_side("B", 1.1);
        super::persist_results(&timing_a_path, &timing_a).expect("timing A results write");
        super::persist_results(&timing_b_path, &timing_b).expect("timing B results write");
        let timing_error = report_ab_files(&timing_a_path, &timing_b_path)
            .expect_err("timing regression exceeds relative noise");
        assert!(timing_error.contains("package/emit M=1 exceeds A median plus session noise"));
        assert!(!timing_error.contains("peak RSS plus session noise"));
        std::fs::remove_dir_all(timing_root).expect("timing results are removed");

        let (rss_root, rss_a_path, rss_b_path) = result_paths();
        let rss_a = synthetic_side("A", 1.0);
        let mut rss_b = synthetic_side("B", 0.99);
        for round in &mut rss_b.rounds {
            for probe in round.probes.values_mut() {
                probe.value.as_mut().unwrap().peak_rss_kib += 10_000;
            }
        }
        super::persist_results(&rss_a_path, &rss_a).expect("RSS A results write");
        super::persist_results(&rss_b_path, &rss_b).expect("RSS B results write");
        let rss_error = report_ab_files(&rss_a_path, &rss_b_path)
            .expect_err("RSS regression exceeds relative noise");
        assert!(rss_error.contains("replay/request M=1 exceeds A peak RSS plus session noise"));
        assert!(!rss_error.contains("package/emit M=1 exceeds A median plus session noise"));
        std::fs::remove_dir_all(rss_root).expect("RSS results are removed");
    }

    #[trace(
        "NFR-141-AC-1",
        "NFR-141-AC-2",
        "NFR-141-AC-3",
        "NFR-141-AC-4",
        "NFR-141-AC-5"
    )]
    #[test]
    fn report_ab_rejects_missing_rows_and_incomplete_rounds() {
        let (root, a_path, b_path) = result_paths();
        let a = synthetic_side("A", 1.0);
        let mut missing = synthetic_side("B", 1.0);
        missing.rounds[3]
            .axes
            .get_mut("package/read")
            .unwrap()
            .remove(&10);
        missing.rounds[3].probes.remove(&100);
        super::persist_results(&a_path, &a).expect("A result file writes");
        super::persist_results(&b_path, &missing).expect("B result file writes");
        assert!(report_ab_files(&a_path, &b_path).is_err());

        missing.rounds.pop();
        super::persist_results(&b_path, &missing).expect("truncated B result file writes");
        let error =
            report_ab_files(&a_path, &b_path).expect_err("incomplete sessions are not accepted");
        assert!(error.contains("expected 5"));
        std::fs::remove_dir_all(root).expect("temporary A/B results are removed");
    }

    fn synthetic_side(side: &str, multiplier: f64) -> SideResults {
        let rounds = (1..=ROUNDS)
            .map(|number| {
                let round_factor = 1.0 + (f64::from(number) - 3.0) * 0.01;
                let axes = AXES
                    .into_iter()
                    .map(|axis| {
                        let axis_factor = match axis {
                            "package/emit" => 1.0,
                            "package/read" => 0.5,
                            "replay/request" => 2.0,
                            _ => unreachable!(),
                        };
                        let estimates = SIZES
                            .into_iter()
                            .map(|size| {
                                let milliseconds =
                                    size as f64 * axis_factor * round_factor * multiplier;
                                (
                                    size,
                                    Measurement::success(CriterionEstimate {
                                        milliseconds,
                                        ci_lower_milliseconds: milliseconds * 0.99,
                                        ci_upper_milliseconds: milliseconds * 1.01,
                                    }),
                                )
                            })
                            .collect::<BTreeMap<_, _>>();
                        (axis.to_owned(), estimates)
                    })
                    .collect::<BTreeMap<_, _>>();
                let probes = SIZES
                    .into_iter()
                    .map(|size| {
                        (
                            size,
                            Measurement::success(ProbeEstimate {
                                milliseconds: size as f64 * round_factor * multiplier,
                                peak_rss_kib: 50_000 + u64::from(number),
                                parse_calls_per_source_document: 1,
                                source_documents: 1,
                            }),
                        )
                    })
                    .collect::<BTreeMap<_, _>>();
                RoundResult {
                    number,
                    load_before: Measurement::success(LoadAverage {
                        one_minute: f64::from(number),
                        five_minutes: 2.0 * f64::from(number),
                        fifteen_minutes: 3.0 * f64::from(number),
                    }),
                    load_after: Measurement::success(LoadAverage {
                        one_minute: f64::from(number) + 0.5,
                        five_minutes: 2.0 * f64::from(number) + 0.5,
                        fifteen_minutes: 3.0 * f64::from(number) + 0.5,
                    }),
                    axes,
                    probes,
                }
            })
            .collect();
        SideResults {
            side: side.to_owned(),
            rounds,
        }
    }

    fn result_paths() -> (PathBuf, PathBuf, PathBuf) {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "fsl-bench-package-report-{}-{stamp}",
            std::process::id()
        ));
        std::fs::create_dir_all(&root).expect("temporary result directory creates");
        (root.clone(), root.join("a.json"), root.join("b.json"))
    }
}

#[cfg(test)]
fn complete_probe_report(report: &str) -> bool {
    parse_probe_estimate(report, 1).is_ok()
}
