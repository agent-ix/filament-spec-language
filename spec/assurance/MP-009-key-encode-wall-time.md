---
id: MP-009
title: State-key encoding wall time
type: MeasurementPlan
status: active
owner: peter
metric: fsl.bench.key.wall_time
definition_version: fsl.bench.key.wall_time-v1
stage: baseline
ground_truth_kind: mechanical
objective:
  direction: lower
subject_identity:
  name: fsl-eval ModelSystem::state_key
  version: "fsl-eval/Cargo.toml"
statistical_design:
  population: >-
    The fixed, generated inputs of fsl-bench/benches/key.rs, one dimension per criterion benchmark id key/encode/<objects>x<fields> (unit: seconds per call; throughput: encoded key bytes). Every input is deterministic.
  sampling: >-
    Run `make bench-key` five times on one machine, recording the load average before and after each run. The observed value per benchmark id is the median of the five runs' criterion point estimates, in seconds per call.
  repetitions: 5
  estimator: median
  error_model: >-
    Between-repetition drift from concurrent builds on the shared machine
    (load average recorded before and after every repetition), CPU
    frequency and cache state; within-repetition sampling noise.
  uncertainty: >-
    Per dimension, the session noise computed from the five current-main
    and five candidate observations of one interleaved session, as defined
    in fsl-bench/BENCHMARKING.md; the full min-max range for both sides is
    recorded.
  decision_rule:
    comparator: lt
    baseline: prior-collection
protected_apparatus:
  - fsl-bench/benches/key.rs
  - fsl-bench/src/mc.rs
  - fsl-bench/src/world.rs
  - fsl-bench/Cargo.toml
negative_controls:
  - kind: gain-within-noise
    description: >-
      A candidate counts as an improvement on a dimension only in one
      interleaved session that alternates the baseline revision and the
      candidate revision, five rounds each, on one machine: its median must
      be below the baseline revision's same-session median by more than the
      session noise computed from those same-session observations, as
      defined in fsl-bench/BENCHMARKING.md. A smaller change is recorded as
      no change.
  - kind: apparatus-edit
    description: >-
      The bench file, its input generator and the bench crate manifest are
      protected apparatus; a change that edits them requires a fresh
      current-main/candidate comparison with the changed apparatus.
  - kind: selective-reporting
    description: >-
      Every round of an interleaved session is recorded, both sides; a
      session is not rerun until it passes.
  - kind: stale-evidence
    description: >-
      Every recorded collection names the machine it ran on; a result from
      another machine is not compared with this paired session.
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-101
    type: measures
---

# State-key encoding wall time

## Decision Use

Shows the cost of one canonical state-key encoding by state shape. A performance claim compares current main with the candidate in one
interleaved same-session A/B run, using only the session noise defined in
fsl-bench/BENCHMARKING.md. This plan grades no release and gates no merge.

## Population

The fixed, generated inputs of fsl-bench/benches/key.rs, one dimension per criterion benchmark id key/encode/<objects>x<fields> (unit: seconds per call; throughput: encoded key bytes). Every input is deterministic.

## Collection Procedure

Run `make bench-key` five times on one machine, recording the load average before and after each run. The observed value per benchmark id is the median of the five runs' criterion point estimates, in seconds per call.

## Interpretation

The decision rule's `prior-collection` is the current-main side of the same
interleaved session; no earlier collection is a threshold. The workload each
dimension times is fixed by the generator and by the counter totals its
smallest-size test asserts (`cargo test -p fsl-bench --features counters`).
