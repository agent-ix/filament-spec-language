---
id: MP-006
title: S3 checker one-shot wall time and peak RSS
type: MeasurementPlan
status: active
owner: peter
metric: fsl.bench.probe.check_wall_time_and_rss
definition_version: fsl.bench.probe.check_wall_time_and_rss-v1
stage: baseline
ground_truth_kind: mechanical
objective:
  direction: lower
subject_identity:
  name: fsl-semantics S3 checker (PackageDeclarations::check)
  version: "fsl-semantics/Cargo.toml"
statistical_design:
  population: >-
    The fixed, generated inputs `fsl-bench-probe check <shape> <n>` builds,
    one observation dimension per shape, size and quantity (`wall_time` or
    `peak_rss`); every input is deterministic.
  sampling: >-
    One probe process per input per repetition: the process builds the
    input, times one `PackageDeclarations::check` over the whole package,
    and reads its own peak RSS after the check: `VmHWM` on Linux and, on macOS,
    the peak physical footprint from `vmmap -summary`.
  repetitions: 5
  estimator: median
  error_model: >-
    Between-repetition drift from concurrent builds on the shared machine
    (load average recorded before and after every repetition), CPU
    frequency and cache state. Peak RSS is near-deterministic for a given
    build; wall time is not.
  uncertainty: >-
    Per dimension, the median absolute deviation of the five repetitions
    relative to their median; the full min-max range is recorded beside it.
  decision_rule:
    comparator: lt
    baseline: prior-collection
protected_apparatus:
  - fsl-bench/src/bin/fsl-bench-probe.rs
  - fsl-bench/src/check.rs
  - fsl-bench/src/rss.rs
negative_controls:
  - kind: gain-within-noise
    description: >-
      A candidate improves on a dimension only in one interleaved session
      that alternates the baseline revision and the candidate revision,
      five rounds each, on one machine: its median must be below the
      baseline revision's same-session median by more than the larger of
      either side's MAD/median in that session. A smaller change is
      recorded as no change, using the live session-noise rule in
      fsl-bench/BENCHMARKING.md.
  - kind: apparatus-edit
    description: >-
      The probe, its input generator and its RSS reader are protected
      apparatus; a change that edits what they build or read requires a fresh
      current-main/candidate comparison using the changed apparatus.
  - kind: selective-reporting
    description: >-
      Every round of an interleaved session is recorded, both sides; a
      session is not rerun until it passes.
  - kind: stale-evidence
    description: >-
      Every recorded collection names the machine it ran on; a result from
      another machine is not compared with this paired session.
relationships:
  - target: ix://agent-ix/filament-specification/FR-146
    type: measures
---

# S3 checker one-shot wall time and peak RSS

## Decision Use

Inputs too slow for criterion to sample repeatedly, such as call chains of
4,000 and 8,000 functions, and the memory a check needs. This plan
showed the termination fix's gain on long chains, and the memory growth
before and after it. The plan grades no release and gates no merge.

## Population

Each `fsl-bench-probe check <shape> <n>` run, where `<shape>` is `chain`,
`independent` or `self-recursive`. Each (shape, size, quantity) triple is one
observation dimension:

- `wall_time`, in seconds: one `PackageDeclarations::check` over the whole
  package. Building the input is excluded.
- `peak_rss`, in KiB: `VmHWM` of the probe process on Linux, which builds and
  checks only that one input. `fsl-bench/src/rss.rs` also has a macOS reader
  (QSL-1906), which reports the peak physical footprint from `vmmap -summary`;
  that figure counts compressed pages a resident set omits, so macOS and Linux
  figures are never compared. The Linux path is unchanged by that edit, so the
  Linux rows of this plan need no fresh A/B; macOS rows start a new series.

## Collection Procedure

Run `fsl-bench-probe check <shape> <n>` once per size per round, one process
each, five rounds, on one machine. Record
`/proc/loadavg` before and after each round. The observed value per dimension
is the median of the five rounds.

## Interpretation

The wall-time figures are wall time on a shared, loaded machine. The decision
rule's `prior-collection` is the current-main collection from the same
interleaved session. Peak RSS is process-wide and monotone, which is why each
process measures a single input.
