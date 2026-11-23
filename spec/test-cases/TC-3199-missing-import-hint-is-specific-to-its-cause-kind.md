---
id: TC-3199
title: "The missing_import hint is specific to the missing selection"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-865
    type: verifies
  - target: ix://agent-ix/filament-spec-language/FR-286
    type: verifies
---
# TC-3199: The missing_import hint is specific to the missing selection

## Description

Verify the `selection` and `hint` of each missing_import producer and the text line. Scope: FR-865-AC-17 to FR-865-AC-24.

## Test Procedure

1. Refuse FR-866-AC-3's `check_outcome` and its compile request with `models` `[]`.
2. Refuse FR-099-AC-3's import with no library supplied.
3. Refuse FR-121-AC-35's `{test/other, big}`.
4. Refuse FR-098-AC-33's `function_owner` `test/absent`.
5. Refuse FR-071-AC-5's package reference with no byte-provision entry.
6. Refuse a unit importing a supplied sibling unit as a library.
7. Build the diagnostic of the `source`, `extension-definition` and `package-reference` producers of FR-286 "Missing selections" and render it.
8. Serialize each document to JSON.

## Expected Results

- Step 1: `  hint: Supply the model in the request's models member, or with --model <identity>=<file> on fsl check.`, `selection` `model`.
- Step 2: `  hint: Supply the library in the request's libraries member, or with --library <identity>=<file> on fsl check.`, `selection` `library`.
- Step 3: `  hint: Call a function of a library that the unit imports directly, or add the import.`, `selection` `unimported-library`.
- Step 4: `  hint: Name a function_owner that the program imports directly, or add the import.`, `selection` `function-owner`.
- Step 5: `  hint: Add a byte_provision entry for the named source to the replay request.`, `selection` `byte-provision`.
- Step 6: `  hint: Supply the imported unit as a library, not as a sibling unit.`, `selection` `sibling-unit`.
- Step 7: the hint text of the table's row for `source`, `extension-definition` and `package-reference`.
- Step 8: each JSON `hint` equals the text line byte for byte; no line is `  hint: Import the package that supplies the selected name.`.

Test: `fsl-replay`, `fsl-semantics`, `fsl-package`, `fsl-analyze` and `fsl-command` tests (one per producer).
