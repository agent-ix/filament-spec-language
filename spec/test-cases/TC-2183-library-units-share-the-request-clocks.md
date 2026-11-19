---
id: TC-2183
title: "Library units share the request's clocks"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-640
    type: verifies
---
# TC-2183: Library units share the request's clocks

## Description

Library units share the request's clocks.

Scope: FR-640-AC-9.

## Test Procedure

Compile a program and a library unit whose clause writes clock `plant-ms`, with and without the `plant-ms` member. Test: `tests/it/clock_derivation.rs` `library_units_share_the_request_clocks`.

Tag the tests `#[trace("TC-2183", "FR-640-AC-9")]`.

## Expected Results

- With the member: both clauses bind to it; no `unused-clock-selection`.
- Without: `missing-clock-selection` naming `plant-ms`.
