// SPDX-License-Identifier: AGPL-3.0-or-later
//! Build evaluator allocation probes and emit their invocation-local report.

use fsl_bench::check::{call_head, completed_with_five, linked_chain};
use fsl_bench::evaluator_cost::{
    self, AllocationProbe, ReportRole, CALL_CHAIN_SIZES, EVALUATOR_CALL_CHAIN_AXIS,
    EVALUATOR_KERNEL_AXIS,
};
use fsl_bench::kernel::{KernelCase, KernelKind, SIZES};
use fsl_semantics::check::CheckedNode;
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
        Some(command) => return Err(format!("unknown evaluator report command: {command}").into()),
        None => return Err("expected start, profile, or report".into()),
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
                    let body = case
                        .package
                        .graph()
                        .function_state(0)
                        .expect("kernel function exists")
                        .body;
                    let nodes = checked_nodes([body]);
                    record_case(target, run_id, axis, kind.name(), size, &nodes, || {
                        black_box(case.call(&objects));
                    })?;
                }
            }
        }
        EVALUATOR_CALL_CHAIN_AXIS => {
            for size in CALL_CHAIN_SIZES {
                let package = linked_chain(size);
                let roots = (0..size).map(|index| {
                    package
                        .graph()
                        .function_state(index)
                        .expect("linked-chain function exists")
                        .body
                });
                let nodes = checked_nodes(roots);
                record_case(target, run_id, axis, "linked_chain", size, &nodes, || {
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

fn checked_nodes<'a>(roots: impl IntoIterator<Item = CheckedNode<'a>>) -> Vec<CheckedNode<'a>> {
    let mut pending: Vec<_> = roots.into_iter().collect();
    let mut nodes = Vec::new();
    while let Some(node) = pending.pop() {
        nodes.push(node);
        pending.extend(node.children());
    }
    nodes
}

fn record_case(
    target: &Path,
    run_id: u128,
    axis: &str,
    case: &str,
    size: usize,
    nodes: &[CheckedNode<'_>],
    evaluate: impl FnOnce(),
) -> Result<(), Box<dyn Error>> {
    let child_allocations = allocation_counter::measure(|| {
        for node in nodes.iter().copied() {
            for child in node.children() {
                black_box(child);
            }
        }
    });
    let evaluation_allocations = allocation_counter::measure(evaluate);
    evaluator_cost::record_probe(
        target,
        run_id,
        axis,
        case,
        size,
        AllocationProbe {
            child_traversal_allocation_count: child_allocations.count_total,
            evaluation_allocation_count: evaluation_allocations.count_total,
        },
    )?;
    println!(
        "{axis}/{case}/{size}: child traversal allocations {} of {} evaluation allocations",
        child_allocations.count_total, evaluation_allocations.count_total
    );
    Ok(())
}
