---
id: TC-2722
title: "Late classification by the latest instant, on-time positions and the history cutoff"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-966
    type: verifies
---
# TC-2722: Late classification by the latest instant, on-time positions and the history cutoff

## Description

Verify that a late-data cutoff classifies an observation by the upper end of its uncertainty, that late observations take no position, that an all-late trace refuses, and that a history cutoff and a late-data cutoff stay independent.

Scope: FR-966-AC-3, FR-966-AC-4, FR-966-AC-6.

## Test Procedure

1. Classify the observations of FR-966-AC-3 under `Exact` with intervals, `Bounded { uncertainty: 1 }` and `Counter { uncertainty: 1 }`.
2. Evaluate FR-966-AC-4's request under `AllowedLateness { lateness: 2 }` and `FixedInstant { cutoff: 5 }`, then the all-late trace.
3. Evaluate `once[0,4] holds(p)` `on each` under `HistoryCutoff` with and without the late-data cutoff.

Test: `tests/it/temporal_uncertain_late.rs` `the_cutoff_classifies_the_latest_possible_instant`, `late_observations_take_no_position`, `a_history_cutoff_and_a_late_data_cutoff_stay_independent`.

## Expected Results

- Step 1: the on-time and late classifications of FR-966-AC-3, each late entry with `stamp` 3.
- Step 2: `holds` with support at position 1 and one late entry; `violated` with two late entries; the all-late trace refuses at `admit`.
- Step 3: `indeterminate` with `missing-history` and the stamp-3 entry anchored at 1; without the cutoff, no `late` member and no contradiction.
