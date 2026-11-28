//! Invocation-local evaluator timing and child-traversal allocation reports.

use crate::kernel::{self, KernelKind};
use std::collections::BTreeSet;
use std::error::Error;
use std::fs::{self, OpenOptions};
use std::io::{self, BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use ix_trace_rs::implements;

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

/// Required per-case fields, including the 95% Criterion CI half-width used by
/// the live A/B noise calculation.
pub const REPORT_FIELDS: [&str; 5] = [
    "wall_time_ns",
    "wall_time_ci_half_width_ns",
    "child_traversal_allocation_count",
    "child_traversal_allocation_share",
    "evaluation_allocation_count",
];

const RUN_MARKER: &str = "evaluator-cost-run-start-nanos";
const PROBES: &str = "evaluator-cost-allocation-probes.jsonl";
const ROUND_MARKER: &str = "evaluator-cost-recorded-round.json";
const AB_SESSION_MANIFEST: &str = "evaluator-cost-ab-session.json";

/// One measured allocation probe for an evaluator input.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AllocationProbe {
    /// The number of allocations made while traversing checked-node children.
    pub child_traversal_allocation_count: u64,
    /// The total allocations made by one evaluation of the same input.
    pub evaluation_allocation_count: u64,
}

/// Evaluate one real case while recording dynamic child traversal allocations.
#[cfg(feature = "allocation-probe")]
#[implements("NFR-141-AC-7")]
pub fn record_evaluation_probe(
    target: &Path,
    run_id: u128,
    axis: &str,
    case: &str,
    size: usize,
    evaluate: impl FnOnce(),
) -> Result<(fsl_semantics::check::ChildTraversalProfile, u64), Box<dyn Error>> {
    use fsl_semantics::check::{begin_child_traversal_profile, end_child_traversal_profile};

    begin_child_traversal_profile(measure_child_allocations);
    let evaluation = allocation_counter::measure(evaluate);
    let evaluation_allocation_count = evaluation.count_total;
    let traversal = end_child_traversal_profile();
    record_probe(
        target,
        run_id,
        axis,
        case,
        size,
        AllocationProbe {
            child_traversal_allocation_count: traversal.allocation_count,
            evaluation_allocation_count,
        },
    )?;
    Ok((traversal, evaluation_allocation_count))
}

#[cfg(feature = "allocation-probe")]
fn measure_child_allocations(operation: &mut dyn FnMut()) -> u64 {
    allocation_counter::measure(operation).count_total
}

/// Whether a report is describing the baseline A or verifying candidate B.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReportRole {
    /// Preserve observed allocations for the unmodified baseline.
    Baseline,
    /// Require zero child-traversal allocations for the candidate.
    Candidate,
}

impl ReportRole {
    fn name(self) -> &'static str {
        match self {
            Self::Baseline => "baseline",
            Self::Candidate => "candidate",
        }
    }
}

/// Start an invocation and return its unique run identifier.
#[implements("NFR-141-AC-7")]
pub fn start_invocation(target: &Path) -> io::Result<u128> {
    fs::create_dir_all(target)?;
    clear_criterion_estimates(target)?;
    match fs::remove_file(target.join(ROUND_MARKER)) {
        Ok(()) => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(error),
    }
    let run_id = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(io::Error::other)?
        .as_nanos();
    fs::write(target.join(RUN_MARKER), run_id.to_string())?;
    fs::write(target.join(PROBES), "")?;
    Ok(run_id)
}

fn clear_criterion_estimates(target: &Path) -> io::Result<()> {
    for (axis, _) in PROFILE_AXES {
        let criterion = target.join("criterion").join(axis.replace('/', "_"));
        let cases = expected_cases(axis).map_err(|error| io::Error::other(error.to_string()))?;
        for (case, size) in cases {
            match fs::remove_file(criterion_path(&criterion, axis, case, size)) {
                Ok(()) => {}
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Err(error) => return Err(error),
            }
        }
    }
    Ok(())
}

/// Start a round that `emit_report` will append to the A or B result file.
#[implements("NFR-141-AC-7")]
pub fn start_recorded_invocation(
    target: &Path,
    side: &str,
    round: usize,
    axis: &str,
    result_file: &Path,
) -> Result<u128, Box<dyn Error>> {
    start_recorded_invocation_with_load(target, side, round, axis, result_file, system_load_average)
}

fn start_recorded_invocation_with_load(
    target: &Path,
    side: &str,
    round: usize,
    axis: &str,
    result_file: &Path,
    load_average: impl FnOnce() -> Result<serde_json::Value, Box<dyn Error>>,
) -> Result<u128, Box<dyn Error>> {
    if !matches!(side, "A" | "B") || !(1..=5).contains(&round) {
        return Err(
            io::Error::other("recorded round requires side A or B and round 1 through 5").into(),
        );
    }
    if !PROFILE_AXES.iter().any(|(name, _)| *name == axis) {
        return Err(io::Error::other(format!("unknown evaluator profile axis: {axis}")).into());
    }
    let result_file = result_file
        .to_str()
        .ok_or_else(|| io::Error::other("A/B result path is not valid UTF-8"))?;
    let load_before = load_average()?;
    register_ab_result_path(target, side, axis, result_file)?;
    let run_id = start_invocation(target)?;
    let marker = serde_json::json!({
        "format": "fsl-evaluator-ab-round-v1",
        "side": side,
        "round": round,
        "axis": axis,
        "result_file": result_file,
        "load_before": load_before,
    });
    fs::write(
        target.join(ROUND_MARKER),
        serde_json::to_vec_pretty(&marker)?,
    )?;
    Ok(run_id)
}

fn register_ab_result_path(
    target: &Path,
    side: &str,
    axis: &str,
    result_file: &str,
) -> Result<(), Box<dyn Error>> {
    let path = target.join(AB_SESSION_MANIFEST);
    let mut manifest = if path.exists() {
        read_json(&path)?
    } else {
        serde_json::json!({ "format": "fsl-evaluator-ab-session-v1", "paths": {} })
    };
    if manifest["format"].as_str() != Some("fsl-evaluator-ab-session-v1") {
        return Err(io::Error::other("invalid evaluator A/B session manifest").into());
    }
    let key = format!("{side}:{axis}");
    if let Some(existing) = manifest["paths"][&key].as_str() {
        if existing != result_file {
            return Err(io::Error::other(format!(
                "A/B session path for {key} is immutable: expected {existing}"
            ))
            .into());
        }
    } else {
        manifest["paths"][&key] = serde_json::Value::String(result_file.to_owned());
    }
    fs::create_dir_all(target)?;
    write_json_atomically(&path, &manifest)?;
    Ok(())
}

/// Persist a failed recorded round so a retry cannot erase its outcome.
pub fn record_round_failure(target: &Path, diagnostic: &str) -> Result<(), Box<dyn Error>> {
    let marker: serde_json::Value = read_json(&target.join(ROUND_MARKER))?;
    let result_file = marker["result_file"]
        .as_str()
        .ok_or_else(|| io::Error::other("round marker has no result file"))?;
    let side = marker["side"]
        .as_str()
        .ok_or_else(|| io::Error::other("round marker has no side"))?;
    let round = marker["round"]
        .as_u64()
        .and_then(|round| usize::try_from(round).ok())
        .ok_or_else(|| io::Error::other("round marker has no round number"))?;
    let axis = marker["axis"]
        .as_str()
        .ok_or_else(|| io::Error::other("round marker has no axis"))?;
    let load_before = marker["load_before"].clone();
    validate_load(&load_before)?;
    let load_after = system_load_average()?;
    let path = Path::new(result_file);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut results = if path.exists() {
        read_json(path)?
    } else {
        serde_json::json!({
            "format": "fsl-evaluator-ab-v1",
            "side": side,
            "axis": axis,
            "rounds": [],
        })
    };
    if results["format"].as_str() != Some("fsl-evaluator-ab-v1")
        || results["side"].as_str() != Some(side)
        || results["axis"].as_str() != Some(axis)
    {
        return Err(
            io::Error::other("existing A/B result file has a different side or axis").into(),
        );
    }
    let rounds = results["rounds"]
        .as_array_mut()
        .ok_or_else(|| io::Error::other("existing A/B result file has no rounds array"))?;
    if rounds.len() + 1 != round {
        return Err(io::Error::other(format!(
            "expected round {} after {} recorded rounds",
            rounds.len() + 1,
            rounds.len()
        ))
        .into());
    }
    rounds.push(serde_json::json!({
        "round": round,
        "status": "failed",
        "run_id": fs::read_to_string(target.join(RUN_MARKER))?.trim(),
        "load_before": load_before,
        "load_after": load_after,
        "diagnostic": diagnostic,
    }));
    write_json_atomically(path, &results)?;
    Ok(())
}

/// Read the invocation identifier written by [`start_invocation`].
pub fn current_invocation(target: &Path) -> io::Result<u128> {
    fs::read_to_string(target.join(RUN_MARKER))?
        .parse()
        .map_err(io::Error::other)
}

/// Record one input's measured child traversal and evaluator allocation counts.
#[implements("NFR-141-AC-7")]
pub fn record_probe(
    target: &Path,
    run_id: u128,
    axis: &str,
    case: &str,
    size: usize,
    probe: AllocationProbe,
) -> Result<(), Box<dyn Error>> {
    if !PROFILE_AXES.iter().any(|(name, _)| *name == axis) {
        return Err(io::Error::other(format!("unknown evaluator profile axis: {axis}")).into());
    }
    let row = serde_json::json!({
        "run_id": run_id.to_string(),
        "axis": axis,
        "case": case,
        "size": size,
        "child_traversal_allocation_count": probe.child_traversal_allocation_count,
        "evaluation_allocation_count": probe.evaluation_allocation_count,
    });
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(target.join(PROBES))?;
    serde_json::to_writer(&mut file, &row)?;
    file.write_all(b"\n")?;
    Ok(())
}

/// Emit and return the invocation-local report artifact for one evaluator axis.
#[implements("NFR-141-AC-7")]
pub fn emit_report(target: &Path, axis: &str, role: ReportRole) -> Result<PathBuf, Box<dyn Error>> {
    let run_id = current_invocation(target)?;
    let expected = expected_cases(axis)?;
    let probes = read_probes(target, run_id, axis)?;
    let criterion = target.join("criterion").join(axis.replace('/', "_"));
    let mut rows = Vec::with_capacity(expected.len());

    for (case, size) in expected {
        let probe = probes
            .iter()
            .find(|probe| probe.case == case && probe.size == size)
            .ok_or_else(|| {
                io::Error::other(format!("missing allocation probe for {axis}/{case}/{size}"))
            })?;
        let estimate = estimate(&criterion_path(&criterion, axis, case, size))?;
        let wall_time_ns = estimate.point_estimate_ns;
        let child_count = probe.child_traversal_allocation_count;
        let allocation_share = if probe.evaluation_allocation_count == 0 {
            0.0
        } else {
            child_count as f64 / probe.evaluation_allocation_count as f64
        };
        if role == ReportRole::Candidate && (child_count != 0 || allocation_share != 0.0) {
            return Err(io::Error::other(format!(
                "candidate child traversal allocation is nonzero for {axis}/{case}/{size}"
            ))
            .into());
        }
        rows.push(serde_json::json!({
            "axis": axis,
            "case": case,
            "size": size,
            "wall_time_ns": wall_time_ns,
            "wall_time_ci_half_width_ns": estimate.ci_half_width_ns,
            "child_traversal_allocation_count": child_count,
            "child_traversal_allocation_share": allocation_share,
            "evaluation_allocation_count": probe.evaluation_allocation_count,
        }));
    }

    if probes.len() != rows.len() {
        return Err(io::Error::other(format!(
            "{} allocation probes do not match the {} expected {axis} rows",
            probes.len(),
            rows.len()
        ))
        .into());
    }

    let report = serde_json::json!({
        "run_id": run_id.to_string(),
        "axis": axis,
        "role": role.name(),
        "fields": REPORT_FIELDS,
        "rows": rows,
    });
    let path = target.join(format!(
        "evaluator-cost-{}-{run_id}.json",
        axis.replace('/', "_")
    ));
    fs::write(&path, serde_json::to_vec_pretty(&report)?)?;
    let round_marker = target.join(ROUND_MARKER);
    if round_marker.exists() {
        let recording: serde_json::Value = serde_json::from_slice(&fs::read(round_marker)?)?;
        append_recorded_round(
            recording["result_file"]
                .as_str()
                .ok_or_else(|| io::Error::other("round marker has no result file"))?,
            recording["side"]
                .as_str()
                .ok_or_else(|| io::Error::other("round marker has no side"))?,
            recording["round"]
                .as_u64()
                .and_then(|round| usize::try_from(round).ok())
                .ok_or_else(|| io::Error::other("round marker has no round number"))?,
            axis,
            &recording["load_before"],
            &system_load_average()?,
            &report,
        )?;
    }
    Ok(path)
}

fn append_recorded_round(
    result_file: &str,
    side: &str,
    round: usize,
    axis: &str,
    load_before: &serde_json::Value,
    load_after: &serde_json::Value,
    report: &serde_json::Value,
) -> Result<(), Box<dyn Error>> {
    let role = if side == "A" { "baseline" } else { "candidate" };
    if !matches!(side, "A" | "B")
        || report["axis"].as_str() != Some(axis)
        || report["role"].as_str() != Some(role)
    {
        return Err(io::Error::other("round report does not match its side or axis").into());
    }
    validate_load(load_before)?;
    validate_load(load_after)?;
    let path = Path::new(result_file);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut results = if path.exists() {
        read_json(path)?
    } else {
        serde_json::json!({
            "format": "fsl-evaluator-ab-v1",
            "side": side,
            "axis": axis,
            "rounds": [],
        })
    };
    if results["format"].as_str() != Some("fsl-evaluator-ab-v1")
        || results["side"].as_str() != Some(side)
        || results["axis"].as_str() != Some(axis)
    {
        return Err(
            io::Error::other("existing A/B result file has a different side or axis").into(),
        );
    }
    let rounds = results["rounds"]
        .as_array_mut()
        .ok_or_else(|| io::Error::other("existing A/B result file has no rounds array"))?;
    if rounds.len() + 1 != round {
        return Err(io::Error::other(format!(
            "expected round {} after {} recorded rounds",
            rounds.len() + 1,
            rounds.len()
        ))
        .into());
    }
    rounds.push(serde_json::json!({
        "round": round,
        "load_before": load_before,
        "load_after": load_after,
        "report": report,
    }));
    write_json_atomically(path, &results)?;
    Ok(())
}

fn write_json_atomically(path: &Path, value: &serde_json::Value) -> Result<(), Box<dyn Error>> {
    let temporary = path.with_extension("tmp");
    fs::write(&temporary, serde_json::to_vec_pretty(value)?)?;
    fs::rename(temporary, path)?;
    Ok(())
}

#[cfg(target_os = "macos")]
fn system_load_average() -> Result<serde_json::Value, Box<dyn Error>> {
    let output = std::process::Command::new("uptime").output()?;
    if !output.status.success() {
        return Err(io::Error::other("uptime could not read load averages").into());
    }
    let text = String::from_utf8(output.stdout)?;
    Ok(parse_load_average_output(&text)?)
}

#[cfg(any(target_os = "macos", test))]
fn parse_load_average_output(text: &str) -> Result<serde_json::Value, io::Error> {
    let averages = text
        .rsplit_once("load averages:")
        .or_else(|| text.rsplit_once("load average:"))
        .map(|(_, averages)| averages)
        .ok_or_else(|| io::Error::other("uptime output has no load-average field"))?;
    let values = averages
        .split_whitespace()
        .take(3)
        .map(str::parse::<f64>)
        .collect::<Result<Vec<_>, _>>()
        .map_err(io::Error::other)?;
    Ok(load_average_value(values)?)
}

#[cfg(target_os = "linux")]
fn system_load_average() -> Result<serde_json::Value, Box<dyn Error>> {
    let values = fs::read_to_string("/proc/loadavg")?
        .split_whitespace()
        .take(3)
        .map(str::parse::<f64>)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(load_average_value(values)?)
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
fn system_load_average() -> Result<serde_json::Value, Box<dyn Error>> {
    Err(io::Error::other("load average capture is unsupported on this platform").into())
}

fn load_average_value(values: Vec<f64>) -> Result<serde_json::Value, io::Error> {
    if values.len() != 3
        || values
            .iter()
            .any(|value| !value.is_finite() || *value < 0.0)
    {
        return Err(
            io::Error::other("system load average did not contain three valid values").into(),
        );
    }
    Ok(serde_json::json!({ "1m": values[0], "5m": values[1], "15m": values[2] }))
}

#[derive(Debug)]
struct ProbeRow {
    case: String,
    size: usize,
    child_traversal_allocation_count: u64,
    evaluation_allocation_count: u64,
}

fn read_probes(target: &Path, run_id: u128, axis: &str) -> Result<Vec<ProbeRow>, Box<dyn Error>> {
    let path = target.join(PROBES);
    let file = fs::File::open(path)?;
    let mut probes = Vec::new();
    let run_id = run_id.to_string();
    for line in BufReader::new(file).lines() {
        let line = line?;
        let value: serde_json::Value = serde_json::from_str(&line)?;
        if value["run_id"].as_str() != Some(run_id.as_str()) || value["axis"].as_str() != Some(axis)
        {
            continue;
        }
        probes.push(ProbeRow {
            case: value["case"]
                .as_str()
                .ok_or_else(|| io::Error::other("allocation probe has no case"))?
                .to_owned(),
            size: value["size"]
                .as_u64()
                .and_then(|size| usize::try_from(size).ok())
                .ok_or_else(|| io::Error::other("allocation probe has no valid size"))?,
            child_traversal_allocation_count: value["child_traversal_allocation_count"]
                .as_u64()
                .ok_or_else(|| {
                    io::Error::other("allocation probe has no child allocation count")
                })?,
            evaluation_allocation_count: value["evaluation_allocation_count"].as_u64().ok_or_else(
                || io::Error::other("allocation probe has no evaluation allocation count"),
            )?,
        });
    }
    Ok(probes)
}

fn expected_cases(axis: &str) -> Result<Vec<(&'static str, usize)>, Box<dyn Error>> {
    match axis {
        EVALUATOR_KERNEL_AXIS => Ok(KernelKind::ALL
            .into_iter()
            .flat_map(|kind| {
                kernel::SIZES
                    .into_iter()
                    .map(move |size| (kind.name(), size))
            })
            .collect()),
        EVALUATOR_CALL_CHAIN_AXIS => Ok(CALL_CHAIN_SIZES
            .into_iter()
            .map(|size| ("linked_chain", size))
            .collect()),
        _ => Err(io::Error::other(format!("unknown evaluator profile axis: {axis}")).into()),
    }
}

/// Compare the five recorded rounds in side A and B result files.
///
/// Each input has `format: "fsl-evaluator-ab-v1"`, its `side` and `axis`, and
/// five ordered rounds. A round contains `round`, `load_before`, `load_after`,
/// and the complete per-invocation report object under `report`. Each load
/// object has numeric `1m`, `5m`, and `15m` fields.
#[implements("NFR-141-AC-7")]
pub fn compare_ab_result_files(
    a_path: &Path,
    b_path: &Path,
    axis: &str,
) -> Result<serde_json::Value, Box<dyn Error>> {
    let a_input = read_json(a_path)?;
    let b_input = read_json(b_path)?;
    let invalid_rounds = [(&a_input, "A"), (&b_input, "B")]
        .into_iter()
        .flat_map(|(input, side)| {
            input["rounds"]
                .as_array()
                .into_iter()
                .flatten()
                .filter(|round| round["status"].as_str() == Some("failed"))
                .map(move |round| {
                    serde_json::json!({
                        "side": side,
                        "round": round["round"],
                        "run_id": round["run_id"],
                        "diagnostic": round["diagnostic"],
                        "load_before": round["load_before"],
                        "load_after": round["load_after"],
                    })
                })
        })
        .collect::<Vec<_>>();
    if !invalid_rounds.is_empty() {
        return Ok(serde_json::json!({
            "format": "fsl-evaluator-ab-comparison-v1",
            "axis": axis,
            "session_valid": false,
            "passed": false,
            "invalid_rounds": invalid_rounds,
            "raw_inputs": { "A": a_input, "B": b_input },
        }));
    }
    let a = parse_side(a_input, "A", axis)?;
    let b = parse_side(b_input, "B", axis)?;
    let mut run_ids = BTreeSet::new();
    for round in a.rounds.iter().chain(&b.rounds) {
        let run_id = round.report["run_id"]
            .as_str()
            .ok_or_else(|| io::Error::other("round report has no run identifier"))?;
        if !run_ids.insert(run_id) {
            return Err(io::Error::other(format!(
                "reused run identifier across A/B inputs: {run_id}"
            ))
            .into());
        }
    }
    let expected = expected_cases(axis)?;
    let mut rows = Vec::with_capacity(expected.len());

    for (case, size) in expected {
        let a_points = points_for(&a, case, size)?;
        let b_points = points_for(&b, case, size)?;
        let a_times: Vec<_> = a_points.iter().map(|point| point.wall_time_ns).collect();
        let b_times: Vec<_> = b_points.iter().map(|point| point.wall_time_ns).collect();
        let a_median = median(&a_times);
        let b_median = median(&b_times);
        let a_mad = median(&absolute_deviations(&a_times, a_median));
        let b_mad = median(&absolute_deviations(&b_times, b_median));
        let mad_noise = (a_mad / a_median).max(b_mad / b_median);
        let ci_noise = a_points
            .iter()
            .chain(&b_points)
            .map(|point| point.ci_half_width_ns / point.wall_time_ns)
            .fold(0.0_f64, f64::max);
        let noise = mad_noise.max(ci_noise);
        let allowed_b_wall_time_ns = a_median * (1.0 + noise);
        let timing_passed = b_median <= allowed_b_wall_time_ns;
        let allocations_passed = b_points.iter().all(|point| {
            point.child_traversal_allocation_count == 0
                && point.child_traversal_allocation_share == 0.0
        });
        rows.push(serde_json::json!({
            "case": case,
            "size": size,
            "a_rounds": a_points.iter().map(point_json).collect::<Vec<_>>(),
            "b_rounds": b_points.iter().map(point_json).collect::<Vec<_>>(),
            "a_median_wall_time_ns": a_median,
            "b_median_wall_time_ns": b_median,
            "a_mad_wall_time_ns": a_mad,
            "b_mad_wall_time_ns": b_mad,
            "mad_noise_fraction": mad_noise,
            "ci_noise_fraction": ci_noise,
            "noise_fraction": noise,
            "allowed_b_wall_time_ns": allowed_b_wall_time_ns,
            "timing_passed": timing_passed,
            "allocations_passed": allocations_passed,
            "passed": timing_passed && allocations_passed,
        }));
    }

    let passed = rows.iter().all(|row| row["passed"] == true);

    Ok(serde_json::json!({
        "format": "fsl-evaluator-ab-comparison-v1",
        "axis": axis,
        "noise_rule": "max(A MAD/median, B MAD/median, widest 95% CI half-width / estimate)",
        "threshold_rule": "B median <= A median * (1 + noise_fraction)",
        "a_rounds": a.rounds.iter().map(|round| round.raw.clone()).collect::<Vec<_>>(),
        "b_rounds": b.rounds.iter().map(|round| round.raw.clone()).collect::<Vec<_>>(),
        "rows": rows,
        "passed": passed,
        "raw_inputs": { "A": a.raw, "B": b.raw },
    }))
}

#[derive(Debug)]
struct SideInput {
    raw: serde_json::Value,
    rounds: Vec<RecordedRound>,
}

#[derive(Debug)]
struct RecordedRound {
    raw: serde_json::Value,
    report: serde_json::Value,
}

#[derive(Debug)]
struct Point {
    round: usize,
    run_id: String,
    load_before: serde_json::Value,
    load_after: serde_json::Value,
    wall_time_ns: f64,
    ci_half_width_ns: f64,
    child_traversal_allocation_count: u64,
    child_traversal_allocation_share: f64,
    evaluation_allocation_count: u64,
}

fn parse_side(
    raw: serde_json::Value,
    expected_side: &str,
    axis: &str,
) -> Result<SideInput, Box<dyn Error>> {
    if raw["format"].as_str() != Some("fsl-evaluator-ab-v1")
        || raw["side"].as_str() != Some(expected_side)
        || raw["axis"].as_str() != Some(axis)
    {
        return Err(io::Error::other(format!(
            "expected fsl-evaluator-ab-v1 side {expected_side} for {axis}"
        ))
        .into());
    }
    let rounds = raw["rounds"]
        .as_array()
        .ok_or_else(|| io::Error::other("A/B input has no rounds array"))?;
    if rounds.len() != 5 {
        return Err(io::Error::other(format!(
            "A/B side {expected_side} must contain five rounds, got {}",
            rounds.len()
        ))
        .into());
    }
    let expected_rows: BTreeSet<_> = expected_cases(axis)?
        .into_iter()
        .map(|(case, size)| (case, size))
        .collect();
    let expected_role = if expected_side == "A" {
        "baseline"
    } else {
        "candidate"
    };
    let mut run_ids = BTreeSet::new();
    let mut parsed_rounds = Vec::with_capacity(5);

    for (index, raw_round) in rounds.iter().enumerate() {
        let expected_round = index + 1;
        if raw_round["round"].as_u64() != Some(expected_round as u64) {
            return Err(io::Error::other(format!(
                "A/B side {expected_side} rounds must be numbered 1 through 5 in order"
            ))
            .into());
        }
        validate_load(&raw_round["load_before"])?;
        validate_load(&raw_round["load_after"])?;
        let report = raw_round["report"].clone();
        if report["axis"].as_str() != Some(axis) || report["role"].as_str() != Some(expected_role) {
            return Err(io::Error::other(format!(
                "round {expected_round} report does not match side {expected_side} axis {axis}"
            ))
            .into());
        }
        let run_id = report["run_id"]
            .as_str()
            .ok_or_else(|| io::Error::other("round report has no run identifier"))?;
        if !run_ids.insert(run_id.to_owned()) {
            return Err(io::Error::other(format!(
                "reused run identifier in side {expected_side}: {run_id}"
            ))
            .into());
        }
        let report_fields: BTreeSet<_> = report["fields"]
            .as_array()
            .ok_or_else(|| io::Error::other("round report has no fields array"))?
            .iter()
            .filter_map(serde_json::Value::as_str)
            .collect();
        for field in REPORT_FIELDS {
            if !report_fields.contains(field) {
                return Err(
                    io::Error::other(format!("round report omits required field {field}")).into(),
                );
            }
        }
        let report_rows = report["rows"]
            .as_array()
            .ok_or_else(|| io::Error::other("round report has no rows array"))?;
        let actual_rows: BTreeSet<_> = report_rows
            .iter()
            .map(|row| {
                let case = row["case"]
                    .as_str()
                    .ok_or_else(|| io::Error::other("report row has no case"))?;
                let size = row["size"]
                    .as_u64()
                    .and_then(|size| usize::try_from(size).ok())
                    .ok_or_else(|| io::Error::other("report row has no valid size"))?;
                Ok((case.to_owned(), size))
            })
            .collect::<Result<_, io::Error>>()?;
        let expected_owned: BTreeSet<_> = expected_rows
            .iter()
            .map(|(case, size)| ((*case).to_owned(), *size))
            .collect();
        if actual_rows != expected_owned || report_rows.len() != expected_owned.len() {
            return Err(io::Error::other(format!(
                "round {expected_round} report has a missing, duplicate, or unexpected {axis} row"
            ))
            .into());
        }
        for row in report_rows {
            validate_report_row(row)?;
        }
        parsed_rounds.push(RecordedRound {
            raw: raw_round.clone(),
            report,
        });
    }
    Ok(SideInput {
        raw,
        rounds: parsed_rounds,
    })
}

fn points_for(side: &SideInput, case: &str, size: usize) -> Result<Vec<Point>, io::Error> {
    side.rounds
        .iter()
        .enumerate()
        .map(|(index, round)| {
            let row = round.report["rows"]
                .as_array()
                .and_then(|rows| {
                    rows.iter().find(|row| {
                        row["case"].as_str() == Some(case)
                            && row["size"].as_u64() == Some(size as u64)
                    })
                })
                .ok_or_else(|| io::Error::other(format!("missing report row {case}/{size}")))?;
            Ok(Point {
                round: index + 1,
                run_id: round.report["run_id"]
                    .as_str()
                    .ok_or_else(|| io::Error::other("round report has no run identifier"))?
                    .to_owned(),
                load_before: round.raw["load_before"].clone(),
                load_after: round.raw["load_after"].clone(),
                wall_time_ns: row["wall_time_ns"]
                    .as_f64()
                    .ok_or_else(|| io::Error::other("round report has invalid wall time"))?,
                ci_half_width_ns: row["wall_time_ci_half_width_ns"]
                    .as_f64()
                    .ok_or_else(|| io::Error::other("round report has invalid CI half-width"))?,
                child_traversal_allocation_count: row["child_traversal_allocation_count"]
                    .as_u64()
                    .ok_or_else(|| {
                        io::Error::other("round report has invalid child allocations")
                    })?,
                child_traversal_allocation_share: row["child_traversal_allocation_share"]
                    .as_f64()
                    .ok_or_else(|| io::Error::other("round report has invalid child share"))?,
                evaluation_allocation_count: row["evaluation_allocation_count"]
                    .as_u64()
                    .ok_or_else(|| io::Error::other("round report has invalid eval allocations"))?,
            })
        })
        .collect()
}

fn point_json(point: &Point) -> serde_json::Value {
    serde_json::json!({
        "round": point.round,
        "run_id": point.run_id,
        "load_before": point.load_before,
        "load_after": point.load_after,
        "wall_time_ns": point.wall_time_ns,
        "ci_half_width_ns": point.ci_half_width_ns,
        "child_traversal_allocation_count": point.child_traversal_allocation_count,
        "child_traversal_allocation_share": point.child_traversal_allocation_share,
        "evaluation_allocation_count": point.evaluation_allocation_count,
    })
}

fn validate_report_row(row: &serde_json::Value) -> Result<(), io::Error> {
    let positive = |field: &str| {
        row[field]
            .as_f64()
            .filter(|value| value.is_finite() && *value > 0.0)
            .ok_or_else(|| io::Error::other(format!("invalid positive report field: {field}")))
    };
    positive("wall_time_ns")?;
    row["wall_time_ci_half_width_ns"]
        .as_f64()
        .filter(|value| value.is_finite() && *value >= 0.0)
        .ok_or_else(|| io::Error::other("invalid confidence-interval half-width"))?;
    let evaluation_allocations = row["evaluation_allocation_count"]
        .as_u64()
        .ok_or_else(|| io::Error::other("invalid evaluation allocation count"))?;
    let child_allocations = row["child_traversal_allocation_count"]
        .as_u64()
        .ok_or_else(|| io::Error::other("invalid child traversal allocation count"))?;
    let child_share = row["child_traversal_allocation_share"]
        .as_f64()
        .filter(|value| value.is_finite() && *value >= 0.0)
        .ok_or_else(|| io::Error::other("invalid child traversal allocation share"))?;
    let expected_share = if evaluation_allocations == 0 {
        0.0
    } else {
        child_allocations as f64 / evaluation_allocations as f64
    };
    if child_share != expected_share {
        return Err(io::Error::other(
            "child allocation share does not match counts",
        ));
    }
    Ok(())
}

fn validate_load(load: &serde_json::Value) -> Result<(), io::Error> {
    for interval in ["1m", "5m", "15m"] {
        load[interval]
            .as_f64()
            .filter(|value| value.is_finite() && *value >= 0.0)
            .ok_or_else(|| io::Error::other(format!("invalid {interval} load average")))?;
    }
    Ok(())
}

fn read_json(path: &Path) -> Result<serde_json::Value, Box<dyn Error>> {
    Ok(serde_json::from_slice(&fs::read(path)?)?)
}

fn median(values: &[f64]) -> f64 {
    let mut values = values.to_vec();
    values.sort_by(f64::total_cmp);
    values[values.len() / 2]
}

fn absolute_deviations(values: &[f64], median: f64) -> Vec<f64> {
    values.iter().map(|value| (value - median).abs()).collect()
}

fn criterion_path(criterion: &Path, axis: &str, case: &str, size: usize) -> PathBuf {
    let path = if axis == EVALUATOR_KERNEL_AXIS {
        criterion
            .join(format!("interpreter_{case}"))
            .join(size.to_string())
    } else {
        criterion.join(size.to_string())
    };
    path.join("new/estimates.json")
}

#[derive(Clone, Copy, Debug)]
struct Estimate {
    point_estimate_ns: f64,
    ci_half_width_ns: f64,
}

fn estimate(path: &Path) -> Result<Estimate, Box<dyn Error>> {
    let estimates: serde_json::Value = serde_json::from_slice(&fs::read(path)?)?;
    let point_estimate_ns = estimates["mean"]["point_estimate"]
        .as_f64()
        .filter(|time| time.is_finite() && *time > 0.0)
        .ok_or_else(|| {
            io::Error::other(format!(
                "Criterion estimate has no positive wall time: {}",
                path.display()
            ))
        })?;
    let lower_bound = estimates["mean"]["confidence_interval"]["lower_bound"]
        .as_f64()
        .filter(|time| time.is_finite() && *time >= 0.0 && *time <= point_estimate_ns)
        .ok_or_else(|| {
            io::Error::other(format!(
                "Criterion estimate has no valid 95% confidence-interval lower bound: {}",
                path.display()
            ))
        })?;
    let upper_bound = estimates["mean"]["confidence_interval"]["upper_bound"]
        .as_f64()
        .filter(|time| time.is_finite() && *time >= point_estimate_ns)
        .ok_or_else(|| {
            io::Error::other(format!(
                "Criterion estimate has no valid 95% confidence-interval upper bound: {}",
                path.display()
            ))
        })?;
    if estimates["mean"]["confidence_interval"]["confidence_level"].as_f64() != Some(0.95) {
        return Err(io::Error::other(format!(
            "Criterion estimate is not a 95% confidence interval: {}",
            path.display()
        ))
        .into());
    }
    Ok(Estimate {
        point_estimate_ns,
        ci_half_width_ns: (point_estimate_ns - lower_bound).max(upper_bound - point_estimate_ns),
    })
}

#[cfg(test)]
mod load_average_tests {
    use super::{parse_load_average_output, start_recorded_invocation_with_load};
    use ix_trace_rs::trace;
    use std::fs;
    use std::io;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);

    struct TempTarget(PathBuf);

    impl TempTarget {
        fn new() -> Self {
            let sequence = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
            Self(std::env::temp_dir().join(format!(
                "fsl-evaluator-load-average-{}-{sequence}",
                std::process::id()
            )))
        }
    }

    impl Drop for TempTarget {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    #[trace("NFR-141-AC-7")]
    fn uptime_output_parses_three_load_averages_and_rejects_invalid_output() {
        let load = parse_load_average_output("2:14 4 users, load averages: 6.96 4.19 3.68\n")
            .expect("uptime load averages parse");
        assert_eq!(load["1m"], 6.96);
        assert_eq!(load["5m"], 4.19);
        assert_eq!(load["15m"], 3.68);

        for output in [
            "uptime output without load data",
            "load averages: 1.0 invalid 3.0",
            "load averages: 1.0 -2.0 3.0",
            "load averages: 1.0 2.0 NaN",
        ] {
            assert!(parse_load_average_output(output).is_err(), "{output}");
        }
    }

    #[test]
    #[trace("NFR-141-AC-7")]
    fn failed_load_capture_leaves_no_manifest_or_run_markers() {
        let target = TempTarget::new();
        let result_file = target.0.join("A-kernel.json");
        let error = start_recorded_invocation_with_load(
            &target.0,
            "A",
            1,
            "evaluator/kernel",
            &result_file,
            || Err(io::Error::other("load capture failed").into()),
        );

        assert!(error.is_err());
        assert!(!target.0.exists());
    }
}
