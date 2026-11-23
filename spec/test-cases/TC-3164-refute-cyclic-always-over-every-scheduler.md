---
id: TC-3164
title: "Refute cyclic always over every scheduler"
type: TC
relationships:
  - target: "ix://agent-ix/filament-spec-language/FR-202"
    type: verifies
---
# TC-3164: Refute cyclic always over every scheduler

## Description

Verify FR-202-AC-38: an every-scheduler `always` refutation carries and
replays its deterministic memoryless witness scheduler.

## Test Procedure

`ok` satisfies P and has actions `stay`, to `ok`, and `leave`, to `down`;
`down` does not satisfy P and returns to `ok`. Analyze
`probability >= 0.9 [always holds(P)]` and
`probability <= 1/2 [always holds(P)]` over every scheduler with no window,
and replay each counterexample. Repeat the first with the scheduler removed,
randomized, or changed to `stay`.

## Expected Results

The minimum is 0. The first counterexample carries the scheduler `leave`
at `ok` and the certificate `x(ok) = 1`, `x(down) = 1`, ranks 1 and 0;
replay rebuilds the `leave`-induced chain and the item settles `refuted`.
The maximum is 1. The second carries the scheduler `stay` and the
accepting-union certificate `x(ok) = 1`, rank 0, and settles `refuted`. A
missing or randomized scheduler, or `stay` with the first certificate,
yields no token.
