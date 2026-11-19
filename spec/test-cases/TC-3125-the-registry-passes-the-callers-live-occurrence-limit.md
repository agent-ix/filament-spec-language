---
id: TC-3125
title: "The application registry passes the caller's live-occurrence limit"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-485
    type: verifies
---
# TC-3125: The application registry passes the caller's live-occurrence limit

## Description

`ApplicationRegistry::begin_call` passes the caller's `SessionLimits` to the leaf, and the designated adapter counts the calls it evaluates inside one step from the session's `LiveContext`.

Scope: FR-485-AC-19.

## Test Procedure

Run the fixtures of FR-485-AC-19 through `ApplicationRegistry::begin_call`: the generated `p` and an interpreted `p` under `live_occurrences` 10, with a three-argument and a two-argument `c`, and the generated `p` under `live_occurrences` 6. Compare `Incomplete` fields, `location`, `frame_depth` and `peak_live_occurrences`. Test: `fsl-replay/tests/it/application_live_occurrences.rs (new)` `registry_live_occurrence_limit_is_the_callers`.

Tag the tests `#[trace("TC-3125", "FR-485-AC-19")]`.

## Expected Results

- Limit 10, three-argument `c`: both stop `Incomplete { LiveOccurrences, limit: 10, consumed: 7, next_charge: 4, FunctionCall }` at `c`'s call of `n`, with equal `location` and `frame_depth`.
- Limit 10, two-argument `c`: both complete with `peak_live_occurrences` 10.
- Limit 6: the generated `p` stops at its call of `c` with `consumed` 3 and `next_charge` 4.
