---
id: MP-008
title: Protocol reach by worker count wall time
type: MeasurementPlan
status: active
owner: peter
metric: fsl.bench.reach.wall_time
definition_version: fsl.bench.reach.wall_time-v1
stage: baseline
ground_truth_kind: mechanical
objective:
  direction: lower
subject_identity:
  name: fsl-eval ProtocolSystem::reachable
  version: "fsl-eval/Cargo.toml"
statistical_design:
  population: >-
    The fixed, generated inputs of fsl-bench/benches/reach.rs, one dimension per criterion benchmark id reach/workers/<workers>/<branches>, for 1, 2, 4 and all available workers over 2, 4 and 6 parallel branches (unit: seconds per call; throughput: reached states per call). Every input is deterministic and every worker count reaches the same graph.
  sampling: >-
    Run `make bench-reach` five times on one machine, recording the load average before and after each run. The observed value per benchmark id is the median of the five runs' criterion point estimates, in seconds per call.
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
  - fsl-bench/benches/reach.rs
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
  - target: ix://agent-ix/filament-spec-language/FR-205
    type: measures
  - target: ix://agent-ix/filament-spec-language/FR-206
    type: measures
---

# Protocol reach by worker count wall time

## Decision Use

Shows how breadth-first protocol reachability scales with worker threads. A performance claim compares current main with the candidate in one
interleaved same-session A/B run, using only the session noise defined in
fsl-bench/BENCHMARKING.md. This plan grades no release and gates no merge.

## Population

The fixed, generated inputs of fsl-bench/benches/reach.rs, one dimension per criterion benchmark id reach/workers/<workers>/<branches>, for 1, 2, 4 and all available workers over 2, 4 and 6 parallel branches (unit: seconds per call; throughput: reached states per call). Every input is deterministic and every worker count reaches the same graph.

## Collection Procedure

Run `make bench-reach` five times on one machine, recording the load average before and after each run. The observed value per benchmark id is the median of the five runs' criterion point estimates, in seconds per call.

## Interpretation

The decision rule's `prior-collection` is the current-main side of the same
interleaved session; no earlier collection is a threshold. The workload each
dimension times is fixed by the generator and by the counter totals its
smallest-size test asserts (`cargo test -p fsl-bench --features counters`).
