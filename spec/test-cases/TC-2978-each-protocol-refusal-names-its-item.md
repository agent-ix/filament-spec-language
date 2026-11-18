---
id: TC-2978
title: "Each protocol decision refusal names its item by declared name at its span"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-218
    type: verifies
---
# TC-2978: Each protocol decision refusal names its item by declared name at its span

## Description

Verify that a protocol decision refusal names its offending item by its
typed cause (the decision's declared name and slot, or the undeclared
owner) at the item's span, and that the FR-865 text line shows that name and
location. Scope: FR-218-AC-17.

## Test Procedure

1. Check, with `check_outcome` and its model, a `choice Pick` whose guard reads `input.code` of the `over` input.
2. Check a `choice Pick` whose guard reads an event binder `seen` that no `visible` entry lists.
3. Check a `choice Pick by Nobody` with no role `Nobody`.
4. Render each refusal in the text form.

## Expected Results

- Step 1: `ill_typed`/`operator-ineligible` at exactly `input.code`, its message naming `Pick` and the guard slot.
- Step 2: `wrong_snapshot`/`wrong-observation` at exactly the read `seen`, naming `Pick` and `seen`.
- Step 3: `missing_declaration`/`missing-name` at exactly `Nobody`, naming it.
- Step 4: each text line begins `<path>:<line>:` with the item's line and shows the same names; no line has to quote the item's source text.

Test: `fsl-command/tests/it/protocol_refusal_names.rs`.
