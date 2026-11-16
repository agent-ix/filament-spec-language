---
id: TC-2411
title: "Bounded integer fields are enumerated over their postcondition interval with an unchanged state graph"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-851
    type: verifies
---
# TC-2411: Bounded integer fields are enumerated over their postcondition interval with an unchanged state graph

## Description

Verify that successor generation enumerates a modified integer field only
inside the interval its postcondition bounds state, gives the exploration
full enumeration gives, counts only generated candidates, and lets the
rover mode-machine liveness item finish at default limits.
Scope: FR-851-AC-1 to FR-851-AC-4.

## Test Procedure

1. Explore the rover fixture with narrowing and with forced full enumeration; count the candidates of `loseLidar` from the initial state and `brake` from a `Degraded` state at speed 300.
2. Analyze the rover liveness item with `max_candidates` 400, with no limit, and with 300.
3. Analyze variants whose `loseLidar` bound is `300 - self.speed >= 0` and `self.speed <= 300 or self.stopped`, with `max_candidates` 400.
4. Analyze the rover mode-machine liveness clause with no `temporal.limits`.

## Expected Results

- Step 1: equal `Exploration`s; 301 and 101 candidates against 2001 each.
- Step 2: 400 completes with the unlimited run's `exploration`; 300 settles `incomplete` naming `max_candidates`.
- Step 3: each settles `incomplete` naming `max_candidates`.
- Step 4: `proved`, basis `exhaustive`, no limit reached, within 60 seconds.

Test: `fsl-analyze/tests/it/bounded_successors.rs`.
