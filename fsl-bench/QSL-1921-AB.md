# QSL-1921 evaluator A/B run

The committed recorder writes one append-only result file per side and axis. Use the same fixed paths for the entire A/B session:

- `A-kernel.json`, `B-kernel.json`
- `A-call-chain.json`, `B-call-chain.json`

`start --record` first captures the three load averages, removes prior `new/estimates.json` files for the measured axes, then registers each side/axis path in `target/evaluator-cost-ab-session.json` and writes the run and round markers. Clearing the old estimates makes freshness independent of clock resolution or clock shifts between locked-build invocations. macOS reads the values from `uptime`; it does not use `sysctl vm.loadavg`. If load capture fails, no manifest or run markers are written. A different path for an already registered side/axis is rejected while its session is active. A failed Criterion run, profile command, or report command must be followed by `fail <diagnostic>` in the same target before any further work. That appends a failed round with before/after load and the diagnostic. `compare` returns `session_valid: false` and `passed: false` if either input contains such a round. Do not retry or substitute a result file within a failed session; abandon that session and use new result paths for every side/axis in a clean session. Old failed results remain unchanged.

## Prepare A in the existing worktree

Keep the candidate branch and target in this one worktree. These steps temporarily switch the worktree to the candidate's merge-base with `origin/main` for A; they do not create another worktree. Keep patches, logs, and results under `/mnt/build`. Replace `<B_HEAD>` with the candidate commit that contains the recorder and harness, then set `<A_BASE>` to `git merge-base <B_HEAD> origin/main`. Run each Cargo command through `locked-build run --owner peter@luna:luna-codex-3 --ticket QSL-1921 --class point -- cargo <subcommand> ... -j4`; do not set `CARGO_TARGET_DIR` or `TMPDIR` because locked-build sets them on NVMe.

```sh
git show <B_HEAD>:fsl-bench/qsl-1921-a-baseline.patch > /mnt/build/qsl-1921-a-baseline.patch
git fetch origin main
git switch --detach <A_BASE>
git apply --check /mnt/build/qsl-1921-a-baseline.patch
git apply /mnt/build/qsl-1921-a-baseline.patch
git show <B_HEAD>:fsl-bench/src/evaluator_cost.rs > fsl-bench/src/evaluator_cost.rs
git show <B_HEAD>:fsl-bench/src/bin/fsl-bench-evaluator-report.rs > fsl-bench/src/bin/fsl-bench-evaluator-report.rs
git show <B_HEAD>:fsl-bench/tests/it/evaluator_cost_report.rs > fsl-bench/tests/it/evaluator_cost_report.rs
```

Create a local A-only branch at this point if needed. The patch anchors `allocation-probe` inside `[features]` and adds the report binary after the property-generator test target; keep the `git apply --check` step as a prerequisite before building. The patch preserves the baseline `CheckedNode::children() -> Vec<CheckedNode>` API and semantics. It wraps the existing vector construction to count dynamically how many child iterators the evaluator actually constructs, including recursive KernelCase revisits and skipped branches. The candidate branch instead instruments its no-allocation iterator. The temporary A setup is local-only; do not push it. Return to the candidate branch for B and after collecting A.

## Record five rounds

Use locked-build's per-worktree target. Keep result files under `/mnt/build` so branch switches do not remove them. Before each measured Criterion command, start the recorded invocation with the fixed side/round/axis/path. For example:

```sh
locked-build run --owner peter@luna:luna-codex-3 --ticket QSL-1921 --class point -- cargo run --locked -p fsl-bench --features allocation-probe --bin fsl-bench-evaluator-report -j4 -- start --record A 1 evaluator/kernel /mnt/build/A-kernel.json
locked-build run --owner peter@luna:luna-codex-3 --ticket QSL-1921 --class point -- cargo bench --locked -p fsl-bench --bench evaluator_kernel -j4
locked-build run --owner peter@luna:luna-codex-3 --ticket QSL-1921 --class point -- cargo run --locked -p fsl-bench --features allocation-probe --bin fsl-bench-evaluator-report -j4 -- profile evaluator/kernel
locked-build run --owner peter@luna:luna-codex-3 --ticket QSL-1921 --class point -- cargo run --locked -p fsl-bench --features allocation-probe --bin fsl-bench-evaluator-report -j4 -- report evaluator/kernel
```

For call-chain, use axis `evaluator/call_chain`, `cargo bench --locked -p fsl-bench --bench evaluator -j4`, and the corresponding `profile` and `report` commands. Record rounds 1 through 5 in order for each side/axis. Alternate A and B rounds to reduce load drift. The report command appends the complete raw per-invocation report and after-load metadata to the preselected side/axis file; no manual result assembly is needed. If any Criterion, profile, or report command fails, immediately record it, for example:

```sh
locked-build run --owner peter@luna:luna-codex-3 --ticket QSL-1921 --class point -- cargo run --locked -p fsl-bench --features allocation-probe --bin fsl-bench-evaluator-report -j4 -- fail "Criterion exited with status 101"
```

Do not continue or retry a failed session. After five successful rounds for both sides, compare each axis using the two fixed result files:

```sh
locked-build run --owner peter@luna:luna-codex-3 --ticket QSL-1921 --class point -- cargo run --locked -p fsl-bench --features allocation-probe --bin fsl-bench-evaluator-report -j4 -- compare evaluator/kernel /mnt/build/A-kernel.json /mnt/build/B-kernel.json /mnt/build/kernel-comparison.json
locked-build run --owner peter@luna:luna-codex-3 --ticket QSL-1921 --class point -- cargo run --locked -p fsl-bench --features allocation-probe --bin fsl-bench-evaluator-report -j4 -- compare evaluator/call_chain /mnt/build/A-call-chain.json /mnt/build/B-call-chain.json /mnt/build/call-chain-comparison.json
```
