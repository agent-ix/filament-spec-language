---
id: TC-3213
title: "An export request refuses a compile-only member"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-1015
    type: verifies
---
# TC-3213: An export request refuses a compile-only member

## Description

An export request carrying the compile-only top-level `call_site`, `output` or `handoff` is refused naming that member.

Scope: FR-1015-AC-18.

## Test Procedure

Add each of `call_site` (`{"kind": "imported-function", "library": "test/units", "function": "big"}`), `output` and `handoff` in turn to a valid export request and run it through `fsl-command`. Test: `fsl-command/tests/it/cli_contract/export.rs`.

Tag the tests `#[trace("TC-3213", "FR-1015-AC-18")]`.

## Expected Results

- Each refuses `invalid-request` naming the member, exit 20, with no file written and no export run.
