---
id: TC-1741
title: "Extension definition request refusals"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-660
    type: verifies
---
# TC-1741: Extension definition request refusals

## Description

Verify each refusal of an extension definition supplied through the request. Scope: FR-660-AC-2, FR-660-AC-3, FR-660-AC-4, FR-660-AC-5.

## Test Procedure

1. Compile TC-1740's unit with no `extensions` member, then with only `ext.other` supplied.
2. Supply a document missing `typed_node_schema`, one whose production keyword is `function`, and two mutually dependent entries both selected.
3. Supply `extensions: null`, an entry with member `version`, an entry missing `file`, an entry with a blank `identity`, and two entries with one identity, with every program and model file absent.
4. Make the entry's file push the dependent files one byte over `dependent_bytes`, then raise the ceiling.

Tag the tests `#[trace("TC-1741", "<AC id>")]`.

## Expected Results

- Step 1: `unknown_profile`/`unsupported-selection` at the literal `"ext.retention"`, exit 20.
- Step 2: `invalid_package` with `missing-member`, `conflicting-definition` and `definition-cycle` respectively, naming the definitions, stage `compile`, exit 20.
- Step 3: `invalid-request` at stage `request` naming the member for the first four, `invalid_package`/`conflicting-definition` naming both indices for the last; only the request file is read.
- Step 4: refusal naming `dependent_bytes`; the raised request compiles.
