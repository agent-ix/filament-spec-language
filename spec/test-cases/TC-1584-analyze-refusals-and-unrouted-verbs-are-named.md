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

Verify FR-624's refusal table and the unrouted-verb refusal. Scope: FR-624-AC-1 to FR-624-AC-6.

## Test Procedure

1. Analyze a malformed document in JSON and text mode.
2. Analyze a program the S3 checker refuses.
3. Analyze with a snapshot naming an undeclared population.
4. Invoke `generate`, `prove`, `replay`, `inspect`, `render` and `monitor` in a build that does not route them, in JSON mode.
5. Repeat step 4 in text mode.
6. Analyze a valid request padded with trailing JSON whitespace to exactly `request_bytes` bytes, then the same request with one more whitespace byte.

Steps 4 and 5 run in the driver repository, against the `fsl` binary.

## Expected Results

- Step 1: category `refusal`, empty `items`, diagnostic `invalid-request`, exit 20; text line `fsl analyze: invalid-request:`.
- Step 2: `last_stage` `S3`, the refusal's code, exit 20.
- Step 3: the binding refusal's code, exit 20.
- Step 4: each writes a document with that `operation`, `unsupported`, `unsupported_operation`/`operation-not-routed`, payload `verb`, exit 21; stdout never empty.
- Step 5: each prints the named message and exits 21.
- Step 6: the request at the limit is admitted and gives its own outcome; one byte over gives category `incomplete`, empty `items`, `last_stage` `null`, one `resource_exhausted` diagnostic naming `request_bytes`, exit 22, never `refusal` or 20.
- Assertions compare the outcome document's typed members and the exit code, never message text, except step 6's limit name, which the `resource_exhausted` diagnostic carries only in its message.
