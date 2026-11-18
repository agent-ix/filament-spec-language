---
id: TC-2959
title: "Analyze items over a selected protocol subject and memory model"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-1033
    type: verifies
---
# TC-2959: Analyze items over a selected protocol subject and memory model

## Description

Verify that a `native-analyze/1` request selects a protocol subject through
`subject.protocol`, reads `memory`, requests race freedom, and refuses bad
selections and bindings. Scope: FR-1033-AC-1 to FR-1033-AC-6.

## Test Procedure

1. Analyze `NoBothZero` over `subject.protocol` `SB` with no `memory`, then
   with `SB::Both` selected `tso`.
2. Analyze the race-freedom item over the racy `SB` variant under `tso`, the
   request with no `items`, and the item with another protocol selected.
3. Analyze `NoBothZero` with no `subject.protocol`.
4. Analyze with the bad `memory` entries of AC-4.
5. Analyze with the bad protocol bindings of AC-5.
6. Analyze FR-225-AC-4's three-store protocol with and without `Zero`.

Test: `fsl-analyze/tests/it/analyze_protocol_subject.rs`.

## Expected Results

- Step 1: `proved`; `refuted`, both stores buffered, reproduced, exit 10;
  different identities.
- Step 2: `refuted` naming both writes; derived item present; `unknown-claim`.
- Step 3: decided over the model subject, no memory bound.
- Step 4: `invalid-request`, exit 20, naming the pointer or entries.
- Step 5: `declined` `invalid-input` with the named codes; unknown member
  refuses the request.
- Step 6: `ConstraintReached{boundary_states: 8}`, 20 states, bound reached;
  then `MemoryBoundReached`.
- Assertions compare typed members and exit codes, never message text.
