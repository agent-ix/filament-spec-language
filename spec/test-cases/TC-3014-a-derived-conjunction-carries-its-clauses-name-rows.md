---
id: TC-3014
title: "A derived conjunction node carries its contributing clauses' name rows"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-105
    type: verifies
---
# TC-3014: A derived conjunction node carries its contributing clauses' name rows

## Description

Verify that each `claim` occurrence of a derived conjunction `state_clause`
node has a `claim_names` row equal in name, path and region to the
contributing clause's own row, and that the package is admitted. Scope:
FR-105-AC-19.

## Test Procedure

1. Compile FR-105-AC-14's unit and read `claim_names`.
2. Read the package with the strict reader and look up `Q1`.

Test: `fsl-package/src/emit/tests/`.

## Expected Results

- Step 1: the derived `postcondition` node has rows for `Q1` and `Q2`, and
  the derived `precondition` node has rows for `P1` and `P2`. Each row equals
  the name, `qualified_name` and `declaration_region` of that clause's row on
  its own node.
- Step 2: the package is admitted, and `Q1` resolves to the row on its own
  node.
