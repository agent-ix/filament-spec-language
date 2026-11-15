---
id: TC-1714
title: "A context selection runs a machine invariant on one side"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-652
    type: verifies
---
# TC-1714: A context selection runs a machine invariant on one side

## Description

A context selection runs a machine invariant on one side.

Scope: FR-652-AC-1.

## Test Procedure

Run FR-652-AC-1's `context` selections over TC-1045's `Orders` fixture.

Tag the tests `#[trace("TC-1714", "FR-652-AC-1")]`; they live in `tests/it/run_items.rs`.

## Expected Results

`success` for `pre` and `post`, equal to the in-process reports; the frame-violating post context reports `frame_violation` at stage `admit`, exit 20.
