//! Invocation-local evaluator timing and child-traversal allocation reports.

use crate::kernel::{self, KernelKind};
use std::error::Error;
use std::fs::{self, OpenOptions};
use std::io::{self, BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

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
pub const REPORT_FIELDS: [&str; 4] = [
    "wall_time_ns",
    "child_traversal_allocation_count",
    "child_traversal_allocation_share",
    "evaluation_allocation_count",
];

const RUN_MARKER: &str = "evaluator-cost-run-start-nanos";
const PROBES: &str = "evaluator-cost-allocation-probes.jsonl";

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
pub fn record_evaluation_probe(
    target: &Path,
    run_id: u128,
    axis: &str,
    case: &str,
    size: usize,
    evaluate: impl FnOnce(),
) -> Result<fsl_semantics::check::ChildTraversalProfile, Box<dyn Error>> {
    use fsl_semantics::check::{begin_child_traversal_profile, end_child_traversal_profile};

    begin_child_traversal_profile();
    let evaluation = allocation_counter::measure(evaluate);
    let traversal = end_child_traversal_profile();
    record_probe(
        target,
        run_id,
        axis,
        case,
        size,
        AllocationProbe {
            child_traversal_allocation_count: traversal.allocation_count,
            evaluation_allocation_count: evaluation.count_total,
        },
    )?;
    Ok(traversal)
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
pub fn start_invocation(target: &Path) -> io::Result<u128> {
    fs::create_dir_all(target)?;
    let run_id = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(io::Error::other)?
        .as_nanos();
    fs::write(target.join(RUN_MARKER), run_id.to_string())?;
    fs::write(target.join(PROBES), "")?;
    Ok(run_id)
}

/// Read the invocation identifier written by [`start_invocation`].
pub fn current_invocation(target: &Path) -> io::Result<u128> {
    fs::read_to_string(target.join(RUN_MARKER))?
        .parse()
        .map_err(io::Error::other)
}

/// Record one input's measured child traversal and evaluator allocation counts.
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
pub fn emit_report(target: &Path, axis: &str, role: ReportRole) -> Result<PathBuf, Box<dyn Error>> {
    let run_id = current_invocation(target)?;
    let expected = expected_cases(axis)?;
    let probes = read_probes(target, run_id, axis)?;
    let criterion = target.join("criterion").join(axis.replace('/', "_"));
    let marker_time =
        UNIX_EPOCH + std::time::Duration::from_nanos(u64::try_from(run_id).unwrap_or(u64::MAX));
    let mut rows = Vec::with_capacity(expected.len());

    for (case, size) in expected {
        let probe = probes
            .iter()
            .find(|probe| probe.case == case && probe.size == size)
            .ok_or_else(|| {
                io::Error::other(format!("missing allocation probe for {axis}/{case}/{size}"))
            })?;
        let wall_time_ns = estimate(&criterion_path(&criterion, axis, case, size), marker_time)?;
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
    Ok(path)
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

fn estimate(path: &Path, marker_time: SystemTime) -> Result<f64, Box<dyn Error>> {
    let metadata = fs::metadata(path)?;
    if metadata.modified()? <= marker_time {
        return Err(
            io::Error::other(format!("stale Criterion estimate: {}", path.display())).into(),
        );
    }
    let estimates: serde_json::Value = serde_json::from_slice(&fs::read(path)?)?;
    estimates["mean"]["point_estimate"]
        .as_f64()
        .filter(|time| time.is_finite() && *time > 0.0)
        .ok_or_else(|| {
            io::Error::other(format!(
                "Criterion estimate has no positive wall time: {}",
                path.display()
            ))
            .into()
        })
}
