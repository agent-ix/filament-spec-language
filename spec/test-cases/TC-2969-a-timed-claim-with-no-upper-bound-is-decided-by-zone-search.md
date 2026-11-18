---
id: TC-2969
title: "A timed claim with no upper bound is decided by zone search"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-1034
    type: verifies
---
# TC-2969: A timed claim with no upper bound is decided by zone search

## Description

Verify the lower-bound-only claim-automaton component for `[q, *]`, its one
clock and constant, and EN-6's verdicts over `Rpc`. Scope: FR-1034-AC-1 to
FR-1034-AC-3.

## Test Procedure

1. Translate each FR-1034-AC-1 formula and compare the automaton with the timed evaluator on 1,000 random timed lassos per formula.
2. Translate the three FR-1034-AC-2 formulas and read their clock counts and constants.
3. Over `Rpc` with client deadline 4 ms, decide the four FR-1034-AC-3 claims, check the certificates and replay the counterexample.

Tests: `fsl-eval/tests/it/timed_trace.rs` (steps 1 and 2), `fsl-analyze/tests/it/zone_mitl.rs` (step 3).

## Expected Results

- Step 1: every formula translates; acceptance equals the evaluator's `false` on every lasso; some lasso holds and some is refuted.
- Step 2: one clock each, constant 2 ms.
- Step 3: the `eventually[1 ms, *]` and `once[1 ms, *]` claims hold with accepted certificates; `once(1 ms, *]` is refuted by `send`, `reply` at delays 0 and 1, which replays; the `until[4 ms, *]` claim is refuted; nothing stops or refuses `OpenTimedInterval`.
