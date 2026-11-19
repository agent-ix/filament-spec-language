---
id: MP-011
title: Model-check axes peak RSS and marginal bytes per state
type: MeasurementPlan
status: active
owner: peter
metric: fsl.bench.probe.mc_peak_rss_and_marginal_bytes
definition_version: fsl.bench.probe.mc_peak_rss_and_marginal_bytes-v1
stage: baseline
ground_truth_kind: mechanical
objective:
  direction: lower
subject_identity:
  name: fsl-analyze, fsl-eval model-check, reach, key and sampling axes (probe)
  version: "fsl-analyze/Cargo.toml"
statistical_design:
  population: >-
    The fixed, generated inputs `fsl-bench-probe mc <axis> <sizes...>` builds, one dimension per axis, size and quantity: peak RSS in KiB (Linux `VmHWM`; macOS `getrusage(RUSAGE_SELF)` `ru_maxrss`, converted from bytes) and, from `mc-marginal`, bytes per added state (unit: bytes per state, from the peak RSS added by a larger run after a smaller one in one process). Every input is deterministic.
  sampling: >-
    One probe process per input per repetition, five repetitions, on one machine; `make bench-probe` lists every axis and size. The process builds the input, times one run and reads its own peak RSS afterward. Peak RSS is process-wide and monotone, which is why a plain run measures one input and a marginal run measures an ascending pair. Compare measurements on the same OS; macOS rows using the earlier `vmmap` reader start a separate series.
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
  - fsl-bench/src/bin/fsl-bench-probe.rs
  - fsl-bench/src/mc.rs
  - fsl-bench/src/world.rs
  - fsl-bench/src/rss.rs
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
      The probe binary (`fsl-bench-probe.rs`), the axis generators
      (`mc.rs`, `world.rs`) and the peak-memory reader (`rss.rs`) are
      protected apparatus; a change that edits what they build or read
      requires a fresh current-main/candidate comparison with the changed
      apparatus.
  - kind: selective-reporting
    description: >-
      Every round of an interleaved session is recorded, both sides; a
      session is not rerun until it passes.
  - kind: stale-evidence
    description: >-
      Every recorded collection names the machine it ran on; a result from
      another machine is not compared with this paired session.
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-167
    type: measures
  - target: ix://agent-ix/filament-spec-language/FR-126
    type: measures
  - target: ix://agent-ix/filament-spec-language/FR-189
    type: measures
  - target: ix://agent-ix/filament-spec-language/FR-205
    type: measures
---

# Model-check axes peak RSS and marginal bytes per state

## Decision Use

Shows memory, which the criterion benches do not report; the same runs also print states, edges and samples per second, which are context and not dimensions of this plan (a rate improves upward, this plan's direction is lower). A performance claim compares current main with the candidate in one
interleaved same-session A/B run, using only the session noise defined in
fsl-bench/BENCHMARKING.md. This plan grades no release and gates no merge.

## Population

The fixed, generated inputs `fsl-bench-probe mc <axis> <sizes...>` builds, one dimension per axis, size and quantity: peak RSS in KiB (Linux `VmHWM`; macOS `getrusage(RUSAGE_SELF)` `ru_maxrss`, converted from bytes) and, from `mc-marginal`, bytes per added state (unit: bytes per state, from the peak RSS added by a larger run after a smaller one in one process). Every input is deterministic.

## Collection Procedure

One probe process per input per repetition, five repetitions, on one machine; `make bench-probe` lists every axis and size. The process builds the input, times one run and reads its own peak RSS afterward. Peak RSS is process-wide and monotone, which is why a plain run measures one input and a marginal run measures an ascending pair. Compare measurements on the same OS; macOS rows using the earlier `vmmap` reader start a separate series.

## Interpretation

The decision rule's `prior-collection` is the current-main side of the same
interleaved session; no earlier collection is a threshold. The workload each
dimension times is fixed by the generator and by the counter totals its
smallest-size test asserts (`cargo test -p fsl-bench --features counters`).
