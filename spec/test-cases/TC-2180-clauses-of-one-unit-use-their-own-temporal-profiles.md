---
id: TC-2180
title: "Clauses of one unit use their own temporal profiles"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-326
    type: verifies
---
# TC-2180: Clauses of one unit use their own temporal profiles

## Description

Clauses of one unit use their own temporal profiles.

Scope: FR-326-AC-7.

## Test Procedure

Check a ConfigVersion unit selecting `ev` (event-position) and `inf` (infinite-trace) with one bounded clause `using ev` and one unbounded clause `using inf`. Test: `fsl-semantics/tests/it/temporal_clock_names.rs` `clauses_of_one_unit_use_their_own_temporal_profiles`.

Tag the tests `#[trace("TC-2180", "FR-326-AC-7")]`.

## Expected Results

- The unit checks; no `unknown_profile` refusal.
