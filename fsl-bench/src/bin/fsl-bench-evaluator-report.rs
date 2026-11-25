// SPDX-License-Identifier: AGPL-3.0-or-later
//! Build evaluator allocation probes and emit their invocation-local report.

use fsl_bench::check::{call_head, completed_with_five, linked_chain};
use fsl_bench::evaluator_cost::{
    self, ReportRole, CALL_CHAIN_SIZES, EVALUATOR_CALL_CHAIN_AXIS, EVALUATOR_KERNEL_AXIS,
};
use fsl_bench::kernel::{KernelCase, KernelKind, SIZES};
use fsl_semantics::model::object_environment::ObjectEnvironment;
use ix_trace_rs::string_edge;
use std::error::Error;
use std::hint::black_box;
use std::path::{Path, PathBuf};

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
    match arguments.next().as_deref() {
        Some("start") => {
            let run_id = evaluator_cost::start_invocation(target)?;
            println!("evaluator cost invocation {run_id}");
        }
        Some("profile") => {
            let axis = arguments.next().ok_or("expected an evaluator axis")?;
            profile(target, &axis)?;
        }
        Some("report") => {
            let axis = arguments.next().ok_or("expected an evaluator axis")?;
            let role = match arguments.next().as_deref() {
                None => ReportRole::Baseline,
                Some("--candidate") => ReportRole::Candidate,
                Some(argument) => return Err(format!("unknown report role: {argument}").into()),
            };
            if let Some(argument) = arguments.next() {
                return Err(format!("unexpected report argument: {argument}").into());
            }
            let path = evaluator_cost::emit_report(target, &axis, role)?;
            println!("wrote evaluator cost report: {}", path.display());
        }
        Some("compare") => {
            let axis = arguments.next().ok_or("expected an evaluator axis")?;
            let a_path = arguments.next().ok_or("expected A result JSON path")?;
            let b_path = arguments.next().ok_or("expected B result JSON path")?;
            let output = arguments.next().map(PathBuf::from);
            if let Some(argument) = arguments.next() {
                return Err(format!("unexpected compare argument: {argument}").into());
            }
            let comparison = evaluator_cost::compare_ab_result_files(
                Path::new(&a_path),
                Path::new(&b_path),
                &axis,
            )?;
            let serialized = serde_json::to_vec_pretty(&comparison)?;
            if let Some(output) = output {
                std::fs::write(&output, serialized)?;
                println!("wrote A/B comparison: {}", output.display());
            } else {
                println!("{}", String::from_utf8(serialized)?);
            }
        }
        Some(command) => return Err(format!("unknown evaluator report command: {command}").into()),
        None => return Err("expected start, profile, report, or compare".into()),
    }
    Ok(())
}

fn profile(target: &Path, axis: &str) -> Result<(), Box<dyn Error>> {
    let run_id = evaluator_cost::current_invocation(target)?;
    let objects = ObjectEnvironment::default();
    match axis {
        EVALUATOR_KERNEL_AXIS => {
            for kind in KernelKind::ALL {
                for size in SIZES {
                    let case = KernelCase::new(kind, size);
                    record_case(target, run_id, axis, kind.name(), size, || {
                        black_box(case.call(&objects));
                    })?;
                }
            }
        }
        EVALUATOR_CALL_CHAIN_AXIS => {
            for size in CALL_CHAIN_SIZES {
                let package = linked_chain(size);
                record_case(target, run_id, axis, "linked_chain", size, || {
                    let evaluation = call_head(&package, &objects);
                    assert!(completed_with_five(&evaluation));
                    black_box(evaluation);
                })?;
            }
        }
        _ => return Err(format!("unknown evaluator profile axis: {axis}").into()),
    }
    Ok(())
}

fn record_case(
    target: &Path,
    run_id: u128,
    axis: &str,
    case: &str,
    size: usize,
    evaluate: impl FnOnce(),
) -> Result<(), Box<dyn Error>> {
    let (traversal, _) =
        evaluator_cost::record_evaluation_probe(target, run_id, axis, case, size, evaluate)?;
    println!(
        "{axis}/{case}/{size}: {} child iterators yielded {} children with {} allocations",
        traversal.iterator_count, traversal.child_count, traversal.allocation_count
    );
    Ok(())
}
