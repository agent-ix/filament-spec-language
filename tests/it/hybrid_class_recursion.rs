// SPDX-License-Identifier: AGPL-3.0-or-later
//! Keep the deliberate stack-overflow probe in the test tree, outside shipped source scans.

use ix_trace_rs::trace;

#[test]
#[trace("NFR-141-AC-27")]
fn recursive_depth_failure_is_isolated_in_a_child_process() {
    let status = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--ignored",
            "--exact",
            "hybrid_class_recursion::recursive_depth_failure_child_mode",
            "--nocapture",
        ])
        .status()
        .expect("test binary starts as an isolated child");
    assert!(
        !status.success(),
        "the old recursive walk unexpectedly survived"
    );
}

#[test]
#[ignore = "overflows its stack: only the parent recursion test, under make ci-default-features, runs it as a child process"]
#[trace("NFR-141-AC-27")]
fn recursive_depth_failure_child_mode() {
    fn recurse_to_depth(depth: usize) -> usize {
        if depth == 0 {
            0
        } else {
            1 + recurse_to_depth(depth - 1)
        }
    }

    std::thread::Builder::new()
        .stack_size(512 * 1024)
        .spawn(|| std::hint::black_box(recurse_to_depth(100_000)))
        .expect("small-stack child thread starts")
        .join()
        .expect("100,000 recursive calls exceed this isolated stack");
}
