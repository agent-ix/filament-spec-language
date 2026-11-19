---
id: TC-2165
title: "A shallow recursion completes and the live-occurrence member is read strictly"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/NFR-130
    type: verifies
---
# TC-2165: A shallow recursion completes and the live-occurrence member is read strictly

## Description

A shallow recursion completes and the live-occurrence member is read strictly.

Scope: NFR-130-AC-4.

## Test Procedure

Run `d` with `n = 1000` under default limits; with `call.evaluation.live_occurrences` 0; and with `call.evaluation` holding `live_occurrences` `-1`, `1.5`, `"x"` and an unknown member. Test: `tests/it/spine_run.rs` `native_run_live_occurrence_limit_defaults_and_refusals`.

Tag the tests `#[trace("TC-2165", "NFR-130-AC-4")]`.

## Expected Results

- Default: completes with `0`.
- Zero: exit 22, incomplete naming `evaluation.live_occurrences`.
- Each malformed member: `invalid-request` at stage `request`, exit 20.
