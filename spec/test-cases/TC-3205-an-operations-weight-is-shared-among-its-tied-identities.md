---
id: TC-3205
title: "An operation's weight is shared among its tied identities"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-204
    type: verifies
---
# TC-3205: An operation's weight is shared among its tied identities

## Description

Two identities of one operation tie with a second operation; the first operation's weight is divided between its identities (ADR-026 SD-3, uniformly within an operation).

Scope: FR-204-AC-19.

## Test Procedure

Build `Pair` (FR-204-AC-19) under `Share` (`fire` 4, `G` 1); read the initial state's actions and edges; decide `probability >= 0.45` and `probability <= 0.45` over `eventually[0 ms, 3 ms] holds(n1.hit)`; submit the `Exact` certificate computed with the weight not divided (`17/36`) and the one computed with even ties (`5/12`). Test: `fsl-analyze/tests/it/digital_clocks.rs` `operation_weight_is_shared_among_tied_identities`.

Tag the tests `#[trace("TC-3205", "FR-204-AC-19")]`.

## Expected Results

- Exactly three edges, no other action: `F1` at 2 ms `9/20`, `F2` at 2 ms `9/20`, `G` at 2 ms `1/10`.
- Both claims `proved`, `ExactValue{9/20}`.
- Both certificates rejected `InequalityFails` at the initial state.
