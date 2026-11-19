---
id: TC-2524
title: "Realizability with per-trigger instances, retry choices and replicated roles"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-891
    type: verifies
---
# TC-2524: Realizability with per-trigger instances, retry choices and replicated roles

## Description

Verify that realizability with per-trigger instances, retry choices and replicated roles. Scope: FR-891-AC-3, FR-891-AC-4, FR-891-AC-5.

Catches a game that treats a retry as an environment move, or merges observations across instances.

## Test Procedure

1. Decide the `on each` protocol at `max_live_instances` 2 and 1; decide the compensation whose recovery needs a retry with `attempts 2` and `attempts 1`; decide the `Pool` protocol.

## Expected Results

- Step 1: Two instances are `proved` with `Seller`'s table keyed by ordinal, and bound 1 is `inconclusive`, `instance-bound-reached`; `attempts 2` is `proved` with `cattempt` committed at the retry point and `attempts 1` is `EnvironmentDefeat`; `Pool` is `proved` with one class table and no spawn past `max 2`.
