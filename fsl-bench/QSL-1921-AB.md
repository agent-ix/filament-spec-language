# QSL-1921 evaluator A/B run

The committed recorder writes one append-only result file per side and axis. Use the same fixed paths for the entire A/B session:

- `A-kernel.json`, `B-kernel.json`
- `A-call-chain.json`, `B-call-chain.json`

`start --record` first captures the three load averages, then registers each side/axis path in `target/evaluator-cost-ab-session.json` and writes the run and round markers. macOS reads the values from `uptime`; it does not use `sysctl vm.loadavg`. If load capture fails, no manifest or run markers are written. A different path for an already registered side/axis is rejected. Do not delete the manifest or result files to retry a failed round. A failed Criterion run, profile command, or report command must be followed by `fail <diagnostic>` in the same target before any further work. That appends a failed round with before/after load and the diagnostic. `compare` returns `session_valid: false` and `passed: false` if either input contains such a round. Do not retry or substitute a new result file after a failure; abandon that session and report it.

## Prepare A in the existing worktree

Keep the candidate branch and target in this one worktree. These steps temporarily switch the worktree to the fetched base for A; they do not create another worktree. Replace `<B_HEAD>` with the candidate commit that contains the recorder and harness.

```sh
git show <B_HEAD>:fsl-bench/qsl-1921-a-baseline.patch > /private/tmp/qsl-1921-a-baseline.patch
git fetch origin main
git switch --detach origin/main
git apply --check /private/tmp/qsl-1921-a-baseline.patch
git apply /private/tmp/qsl-1921-a-baseline.patch
git show <B_HEAD>:fsl-bench/src/evaluator_cost.rs > fsl-bench/src/evaluator_cost.rs
git show <B_HEAD>:fsl-bench/src/bin/fsl-bench-evaluator-report.rs > fsl-bench/src/bin/fsl-bench-evaluator-report.rs
git show <B_HEAD>:fsl-bench/tests/it/evaluator_cost_report.rs > fsl-bench/tests/it/evaluator_cost_report.rs
```

Create a local A-only branch at this point if needed. The patch preserves the baseline `CheckedNode::children() -> Vec<CheckedNode>` API and semantics. It wraps the existing vector construction to count dynamically how many child iterators the evaluator actually constructs, including recursive KernelCase revisits and skipped branches. The candidate branch instead instruments its no-allocation iterator. The temporary A setup is local-only; do not push it. Return to the candidate branch for B and after collecting A.

## Record five rounds

Set `CARGO_TARGET_DIR` to a directory inside this worktree for each side (for example `target-a` and `target-b`). Keep result files under `/private/tmp` so branch switches do not remove them. Before each measured Criterion command, start the recorded invocation with the fixed side/round/axis/path. For example:

```sh
cargo run --locked -p fsl-bench --features allocation-probe --bin fsl-bench-evaluator-report -- start --record A 1 evaluator/kernel /private/tmp/A-kernel.json
cargo bench --locked -p fsl-bench --bench evaluator_kernel
cargo run --locked -p fsl-bench --features allocation-probe --bin fsl-bench-evaluator-report -- profile evaluator/kernel
cargo run --locked -p fsl-bench --features allocation-probe --bin fsl-bench-evaluator-report -- report evaluator/kernel
```

For call-chain, use axis `evaluator/call_chain`, `cargo bench --locked -p fsl-bench --bench evaluator`, and the corresponding `profile` and `report` commands. Record rounds 1 through 5 in order for each side/axis. Alternate A and B rounds to reduce load drift. The report command appends the complete raw per-invocation report and after-load metadata to the preselected side/axis file; no manual result assembly is needed. If any Criterion, profile, or report command fails, immediately record it, for example:

```sh
cargo run --locked -p fsl-bench --features allocation-probe --bin fsl-bench-evaluator-report -- fail "Criterion exited with status 101"
```

Do not continue or retry a failed session. After five successful rounds for both sides, compare each axis using the two fixed result files:

```sh
cargo run --locked -p fsl-bench --features allocation-probe --bin fsl-bench-evaluator-report -- compare evaluator/kernel /private/tmp/A-kernel.json /private/tmp/B-kernel.json /private/tmp/kernel-comparison.json
cargo run --locked -p fsl-bench --features allocation-probe --bin fsl-bench-evaluator-report -- compare evaluator/call_chain /private/tmp/A-call-chain.json /private/tmp/B-call-chain.json /private/tmp/call-chain-comparison.json
```
