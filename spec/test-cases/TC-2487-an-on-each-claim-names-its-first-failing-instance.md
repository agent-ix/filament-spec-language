---
id: TC-2487
title: "An on-each claim names its first failing instance"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-875
    type: verifies
---
# TC-2487: An on-each claim names its first failing instance

## Description

An on-each claim names its first failing instance.

Scope: FR-875-AC-3.

## Test Procedure

Run `E` at `rational(99, 100)` over a start snapshot holding `a` and `b`, both undelivered, twice. Test: `fsl-analyze/tests/it/probability_from_source.rs` `probabilistic_on_each_names_first_failing_instance`.

Tag the tests `#[trace("TC-2487", "FR-875-AC-3")]`.

## Expected Results

- The item is `refuted` with a counterexample naming `a`, not `b`.
- The two runs give equal items.
