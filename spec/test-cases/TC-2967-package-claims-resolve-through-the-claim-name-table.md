---
id: TC-2967
title: "Package-route claims resolve through the package's claim-name tables"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-850
    type: verifies
  - target: ix://agent-ix/filament-spec-language/FR-940
    type: verifies
---
# TC-2967: Package-route claims resolve through the package's claim-name tables

## Description

Verify that an analyze request over a selected package resolves invariant
and state-graph claims through the package's `claim_names` rows and temporal
clauses through `temporal_clause_names`, with the source route's records,
that an absent name is `unknown-claim`, and that a dangling or duplicate
name table refuses the request. Scope: FR-850-AC-7 and FR-940-AC-7.

## Test Procedure

1. Write the Counter unit with `Bounded`, `AtMostTwo`, the invariant
   `NotAboveThree` and the `possible` claim `ReachThree` into a handoff
   directory; read its `checked-package.json` `claim_names`.
2. Analyze `NotAboveThree`, `ReachThree` and `Missing` over the directory
   and over source.
3. Remove the `NotAboveThree` row from `checked-package.json`; then
   duplicate the `ReachThree` row's `qualified_name` onto the other row.

Test: `fsl-analyze/tests/it/analyze_package_selection.rs`.

## Expected Results

- Step 1: rows with `qualified_name` `["NotAboveThree"]` and `["ReachThree"]`.
- Step 2: the package records equal the source records; `Missing` settles
  `invalid-request` `unknown-claim`.
- Step 3: `invalid_package`/`invalid-value` at `/claim_names`; then
  `ambiguous_declaration`/`ambiguous-name`; no engine runs.
