---
id: TC-1584
title: "Analyze refusals and unrouted verbs exit with a named document"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-624
    type: verifies
---
# TC-1584: Analyze refusals and unrouted verbs exit with a named document

## Description

Verify FR-624's refusal table and the unrouted-verb refusal. Scope: FR-624-AC-1 to FR-624-AC-5.

## Test Procedure

1. Analyze a malformed document in JSON and text mode.
2. Analyze a program the S3 checker refuses.
3. Analyze with a snapshot naming an undeclared population.
4. Invoke `generate`, `prove`, `replay`, `inspect`, `render` and `monitor` in a build that does not route them, in JSON mode.
5. Repeat step 4 in text mode.

Steps 4 and 5 run in the driver repository, against the `fsl` binary.

## Expected Results

- Step 1: category `refusal`, empty `items`, diagnostic `invalid-request`, exit 20; text line `fsl analyze: invalid-request:`.
- Step 2: `last_stage` `check`, the refusal's code, exit 20.
- Step 3: the binding refusal's code, exit 20.
- Step 4: each writes a document with that `operation`, `unsupported`, `unsupported_operation`/`operation-not-routed`, payload `verb`, exit 21; stdout never empty.
- Step 5: each prints the named message and exits 21.
- Assertions compare the outcome document's typed members and the exit code, never message text.
