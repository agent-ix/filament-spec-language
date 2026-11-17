---
id: TC-2244
title: "A clocks override is read as the clocks member"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-640
    type: verifies
---
# TC-2244: A clocks override is read as the clocks member

## Description

Verify that the command's compile and run entries read a `clocks` override exactly as the request's `clocks` member. Scope: FR-640-AC-13.

## Test Procedure

1. Compile a request with no `clocks` member through the compile entry with a `clocks` override, and the same request with that object as its `clocks` member.
2. Run a request through the run entry the same two ways.
3. Hand an override with an unknown `kind`, and one that repeats a clock name.

## Expected Results

- Step 1: equal package bytes.
- Step 2: equal run reports.
- Step 3: `invalid-request` at stage `request`, exit 20, naming the member, as the same `clocks` member refuses.
