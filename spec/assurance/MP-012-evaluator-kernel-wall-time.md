---
id: MP-012
title: Evaluator kernel cost per operation against plain Rust
type: MeasurementPlan
status: active
owner: peter
metric: fsl.bench.evaluator.kernel.wall_time
definition_version: fsl.bench.evaluator.kernel.wall_time-v1
stage: baseline
ground_truth_kind: mechanical
objective:
  direction: lower
subject_identity:
  name: fsl-eval S6a evaluator (CheckedPackage::call) and plain-Rust reference
  version: "fsl-bench evaluator/kernel-v1"
statistical_design:
  population: >-
    The fixed generated inputs of fsl-bench/benches/evaluator_kernel.rs:
    arithmetic recursion, comparison-heavy recursion and sequence fold, each
    at sizes 5 and 10. Each kernel and size is one observation dimension;
    Criterion measures the checked interpreter entry and plain-Rust reference
    as a pair.
  sampling: >-
    Criterion defaults unless the bench sets them: 3 s warm-up, then linearly
    growing iteration batches until the sample count is reached. Run the
    evaluator_kernel bench once per repetition. The report uses each run's
    fresh mean point estimates, in nanoseconds per call.
  repetitions: 5
  estimator: median
  error_model: >-
    Between-repetition drift from concurrent builds on the shared machine
    (load average recorded before and after every repetition), CPU frequency
    and cache state; within-repetition sampling noise.
  uncertainty: >-
    For each kernel and size, compute session noise from the five current-main
    and five candidate observations in one interleaved session: the largest
    MAD/median on either side and the widest Criterion 95% confidence-interval
    half-width relative to its estimate. Record the full min-max range for
    both sides.
  decision_rule:
    comparator: lt
    baseline: prior-collection
protected_apparatus:
  - fsl-bench/benches/evaluator_kernel.rs
  - fsl-bench/src/kernel.rs
  - fsl-bench/src/bin/fsl-bench-kernel-report.rs
  - fsl-bench/Cargo.toml
  - Makefile
negative_controls:
  - kind: gain-within-noise
    description: >-
      A candidate counts as faster for a kernel and size only in one
      interleaved session that alternates current main and candidate, five
      rounds each, on one machine: the candidate interpreter median must be
      below the current-main interpreter median by more than the session noise
      computed from those same-session observations, as defined in
      fsl-bench/BENCHMARKING.md. A smaller difference is no change. The
      interpreter-to-Rust ratio is descriptive and is not a decision threshold.
  - kind: apparatus-edit
    description: >-
      The kernel bench, its generator, report, and crate manifest are
      protected apparatus; a change to them requires a fresh current-main and
      candidate comparison.
  - kind: selective-reporting
    description: >-
      Record every round and both backends for each reported kernel and size;
      do not select favorable rounds or reuse measurements from an earlier
      session as a threshold.
  - kind: stale-evidence
    description: >-
      Name the machine for every recorded collection; do not compare a result
      from another machine with the paired session.
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-068
    type: measures
---

# Evaluator kernel cost per operation against plain Rust

## Decision Use

A performance claim compares current main with the candidate in one
interleaved same-session A/B run. It uses only the session noise defined in
`fsl-bench/BENCHMARKING.md`; earlier measurements are not thresholds. The
interpreter-to-Rust ratio is printed as context and does not decide the claim.

## Population

Every checked-package call and plain-Rust reference in
`fsl-bench/benches/evaluator_kernel.rs`: arithmetic recursion
`sum(n) = if n > 0 then n + sum(n - 1) else 0`, the three-comparison recursive
kernel, and a sequence fold over `1..=n`, each at sizes 5 and 10. Each kernel
and size is one observation dimension. Recursive kernels have `n + 1` frames
and `n` logical operations; the fold has one frame and `n` logical operations.
Inputs are generated deterministically before timing. The report prints the
Criterion estimate divided by the frame and operation counts, plus the ratio
of interpreter ns per operation to Rust ns per operation.

## Collection Procedure

Run `make bench-evaluator_kernel` five times on one machine, recording load
average before and after each run. Alternate current main and candidate for
five rounds each. For every benchmark id and backend, the observed value is
the median of the five runs' Criterion mean point estimates, in nanoseconds
per call. The per-frame, per-operation and ratio columns are derived from each
run's fresh paired estimates; the decision rule compares interpreter
wall-time estimates and applies only the same-session noise.

## Interpretation

The decision rule's `prior-collection` is the current-main side of the same
interleaved session. The margin is the session noise computed from that paired
run, as stated in `fsl-bench/BENCHMARKING.md`. No fixed measurement is a
threshold.
