---
id: TC-2323
title: "A finite monitor trace has one reading per profile"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-283
    type: verifies
---
# TC-2323: A finite monitor trace has one reading per profile

## Description

Verify that a finite trace given to `monitor` is read closed under an
event-position profile and as a prefix under infinite-trace, with no member to
change it. Scope: FR-283-AC-17.

## Test Procedure

1. Monitor `Bounded` and `Reaches` over the `Finite` trace `c.value` 0, 1, 2; repeat with `limits.max_window_values` 1 and with an empty `temporal_selections`.
2. Monitor a `Document` trace of the same snapshots carrying `"closed": true`, then one carrying `"open": true`.

## Expected Results

- Step 1: `Bounded` `violation` at position 0, truth `false`; `Reaches` `pending`, no truth; the repeats give the same records.
- Step 2: each refused `invalid_runtime_input`/`unknown-member` naming the member, before any position is admitted.

Test: `fsl-analyze/tests/it/monitor_incremental.rs`.
