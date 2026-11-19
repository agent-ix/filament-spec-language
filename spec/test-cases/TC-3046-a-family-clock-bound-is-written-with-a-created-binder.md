---
id: TC-3046
title: "A family clock bound with no source binder is written with a binder the producer creates"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-230
    type: verifies
---
# TC-3046: A family clock bound with no source binder is written with a binder the producer creates

## Description

Verify that a clock bound the domain package declares on a collection
element's clock, and a quantifier over a collection with no declared
maximum, are written in the quantified form. Scope: FR-230-AC-70.

## Test Procedure

1. Over `ts: Sequence<Timer>[0, 3]` and a domain package declaring the clock
   bound `x <= 9 ms` on `Timer`'s clock `x`, emit the package; emit it again.
2. Emit a unit with the source invariant
   `time invariant All when { true } { forall(e in self.ts: e.x <= 9 ms) }`
   over the same model without the package bound.
3. Add `ws: Sequence<Holder>[0, 3]`, `Holder` holding `ys: Sequence<Timer>[0, 2]`,
   and emit.
4. Over `us: Sequence<Timer>` with no declared maximum, emit
   `pre F { forall(e in self.us: e.x < 5 ms) }` on `attemptUpdate`, then the
   same with `exists`.

## Expected Results

- Step 1: one `time_invariant`, predicate `true`, one slot `parameter`
  binder `ts#0` in `binders`, one upper-bound `clock_constraint` on `ts`'s
  element clock template whose slot term is that binder; both emissions give
  the same node ids.
- Step 2: the same node shape as step 1 except the binder name `e`.
- Step 3: a second `time_invariant` with binders `ws#0` then `ys#1`.
- Step 4: one `clock_guard` part with the slot `parameter` binder `e` and a
  `clock_constraint` whose slot term is `e`, for `forall` and for `exists`.
- No step leaves an unwritten member.

Test: `fsl-package/src/emit/tests/timed_clauses.rs`.
