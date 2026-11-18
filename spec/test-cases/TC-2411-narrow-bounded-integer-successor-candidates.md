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
Scope: FR-851-AC-1 to FR-851-AC-5.

## Test Procedure

1. Explore the rover fixture with narrowing and with forced full enumeration; count the candidates of `loseLidar` from the initial state and `brake` from a `Degraded` state at speed 300.
2. Analyze the rover liveness item with `max_candidates` 630, with no limit, with 629 and with 300.
3. Analyze variants whose `loseLidar` bound is `300 - self.speed >= 0` and `self.speed <= 300 or self.stopped`, with `max_candidates` 630.
4. Analyze the rover mode-machine liveness clause with no `temporal.limits`.
5. Expand FR-851-AC-5's `Cell` `set()` from `a` 1, `b` 50 with the clause whose `self.nonZero()` call comes first and with the reordered clause, each with narrowing and with forced full enumeration, and with `max_candidates` 404 and 403 for the first.

## Expected Results

- Step 1: equal `Exploration`s; 301 and 101 candidates against 2001 each.
- Step 2: 630 completes with the unlimited run's `exploration`; 629 and 300 each settle `incomplete` naming `max_candidates`.
- Step 3: each settles `incomplete` naming `max_candidates`.
- Step 4: `proved`, basis `exhaustive`, no limit reached, within 60 seconds.
- Step 5: under the first clause `set` generates 400 candidates, charges 401, records 100 `ContractUndetermined` findings for `A` (reason `precondition-false`) and has 11 successors, equal to its enumeration; the expansion charges 404, completing at 404 and stopping with `MaxCandidates` at 403. The reordered clause generates 11 candidates, charges 12 and records no finding, equal to its enumeration.

Test: `fsl-analyze/tests/it/bounded_successors.rs`.
