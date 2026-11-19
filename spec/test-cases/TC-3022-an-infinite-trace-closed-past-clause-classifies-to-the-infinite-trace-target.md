---
id: TC-3022
title: "An infinite-trace closed-past clause classifies to the infinite-trace target"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-045
    type: verifies
---
# TC-3022: An infinite-trace closed-past clause classifies to the infinite-trace target

## Description

Verify FR-045's infinite-trace bounded-past row and the infinite-trace
clauses that stay outside it. Scope: FR-045-AC-7.

## Test Procedure

1. Under `filament.temporal.infinite-trace/v1`, classify `once[1,1]`, `historically[0,2]`, `since[0,2]` and `triggered[1,2]` alone, and `once[0,2] holds(p) and historically[1,1] not holds(q)`, under a complete and an open surrounding execution.
2. Under the same profile, classify `always[0,2] holds(p)`, `eventually[0,2] holds(p)`, `once holds(p)`, `once[1,*] holds(p)`, `holds(p)` and `once[0,1] eventually[0,1] holds(p)` under both closures.
3. Classify `once[1,1] holds(p)` under the event-position false-extension profile.

Test: `fsl-package/src/emit/tests/mapping_support.rs` `an_infinite_trace_closed_past_clause_maps_to_the_infinite_trace_target`.

## Expected Results

- Step 1: `Supported` with `mltl.infinite-trace/v1`, no total sample valuation and the outstanding premises, under both closures.
- Step 2: `Unsupported` naming `InfiniteTrace`; the last also naming `MixedDirection`.
- Step 3: `Supported` with `mltl.origin-complete-history/v1`.
