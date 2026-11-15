---
id: TC-1747
title: "Current selection refuses parameters"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-664
    type: verifies
---
# TC-1747: Current selection refuses parameters

## Description

Verify that a current-snapshot, context or value-invariants selection carrying `parameters` refuses at the request stage. Scope: FR-664-AC-1, FR-664-AC-2.

## Test Procedure

1. Send FR-312's current-snapshot clause-run request with `parameters` `{}` and with one parameter, then with every dependent file absent, then without the member.
2. Send a `Context` and a `ValueInvariants` selection with `parameters` `{}`.

Tag the tests `#[trace("TC-1747", "<AC id>")]`.

## Expected Results

- Step 1: `invalid-request` at stage `request`, exit 20, naming `parameters`, with or without the files; the request without the member runs.
- Step 2: the same refusal.
