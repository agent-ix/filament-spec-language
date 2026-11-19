---
id: TC-2538
title: "A refinement whose abstract model is timed is decided and certified"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-900
    type: verifies
---
# TC-2538: A refinement whose abstract model is timed is decided and certified

## Description

Verify that a refinement whose abstract model is timed is decided and certified. Scope: FR-900-AC-1, FR-900-AC-2, FR-900-AC-3, FR-900-AC-7, FR-900-AC-8.

Catches abstract clocks guessed instead of derived from the step map.

## Test Procedure

1. Check `TReq2` against timed `TReq`; then the `x >= 0` variant; then an abstract invariant `y <= 3` against a concrete model that waits 4.
2. Check the unit with `time M dense unit ms`, `time A dense unit ms` and the abstract-only `time invariant Bound when { true } { self.y <= 3 ms }`.
3. Check the same unit with `time invariant Both when { exists(c in M::calls : c.inflight) and exists(d in A::calls : d.inflight) } { self.x <= 3 ms }` in place of `Bound`.

Tag steps 2 and 3 `#[trace("TC-2538", "FR-900-AC-7")]` and `#[trace("TC-2538", "FR-900-AC-8")]`.

## Expected Results

- Step 1: The safety half is `proved` with a certificate over auxiliary clock `y` that FR-245 accepts; `AbstractClockGuard{send}` with a reproducing prefix that fails when its delay is set to 1; `AbstractTimeInvariant` at the delay crossing 3.
- Step 2: S3 admits the unit; `A`'s timed model holds `Bound` with the one bound `y <= 3`, and `M`'s holds no `Bound`.
- Step 3: S3 refuses `ambiguous_declaration`/`ambiguous-name` at `A::calls`.
