---
id: TC-2988
title: "Protocol analysis item profile and bounds members"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-636
    type: verifies
---
# TC-2988: Protocol analysis item profile and bounds members

## Description

Verify that `local-projection` and `realizability` items carry `temporal.profile`, `temporal.max_depth` and `temporal.max_history` with no default. Scope: FR-636-AC-5 to FR-636-AC-7.

## Test Procedure

1. Analyze the order fixture with a complete `realizability` item and a complete `local-projection` item onto `Seller`, under profile `bounded-static`, then under `bounded`.
2. Analyze `realizability` items each lacking one of `temporal.max_history`, `temporal.profile` and `temporal.max_depth`, a `local-projection` item lacking `temporal.profile`, and a complete `realizability` item, in one request.
3. Analyze `realizability` items with profile `static`, `max_history` 0 and `max_depth` 0, and a `local-projection` item carrying `max_history` 16.

## Expected Results

- Step 1: every item `proved`; exit 0.
- Step 2: each incomplete item `invalid-request` with `invalid_runtime_input`/`missing-member` naming its missing member; the complete item `proved`; exit 20.
- Step 3: `invalid-value` naming `temporal.profile`, `temporal.max_history` and `temporal.max_depth`; `unknown-member` naming `temporal.max_history`; exit 20.
- Assertions compare the outcome document's typed members and the exit code, never message text.
