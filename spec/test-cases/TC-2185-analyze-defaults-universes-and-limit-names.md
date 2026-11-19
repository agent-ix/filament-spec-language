---
id: TC-2185
title: "Analyze default order, universes and limit names"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-620
    type: verifies
---
# TC-2185: Analyze default order, universes and limit names

## Description

Analyze default order, universes and limit names.

Scope: FR-620-AC-10.

## Test Procedure

Analyze a Counter request without `items` whose unit declares `BelowThree` then `Bounded`; one with an unprovisioned `subject.universes` entry; one with `limits.normalization` `{"work_units": 5000000}` and one with `{"bogus": 1}`. Test: `fsl-analyze/tests/it/analyze_request_wire.rs` `analyze_defaults_universes_and_limit_names`.

Tag the tests `#[trace("TC-2185", "FR-620-AC-10")]`.

## Expected Results

- Default items: `BelowThree`, `Bounded`, then deadlock-freedom.
- The unprovisioned universe and `bogus` each refuse `invalid-request`, exit 20; `work_units` is admitted.
