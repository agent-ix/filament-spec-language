---
id: TC-2159
title: "Every construct message begins with the construct"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-763
    type: verifies
---
# TC-2159: Every construct message begins with the construct

## Description

Every construct message begins with the construct.

Scope: FR-763-AC-5.

## Test Procedure

Run `parse` over a temporal clause holding the hyper condition
`x @ a = x @ b`, the remaining construct kind source reaches; repeat with a
200-byte condition consisting of a 188-byte identifier followed by
` @ a = x @ b`. Test: `tests/it/forms_refusal_cause.rs`
`every_construct_message_begins_with_the_construct`.

Tag the tests `#[trace("TC-2159", "FR-763-AC-5")]`.

## Expected Results

- Each message begins with its row's sentence filled from `text`.
- Each outcome is `unsupported`, exit 21, with one diagnostic whose code is
  `unsupported_construct`, cause is `not-yet-implemented`, construct is
  `hyper-condition`, declaration is `T`, profile is
  `filament.value.complete/v1`, and locus starts at the authored condition.
- The 200-byte construct's `text` is at most 123 bytes and ends with `...`; no `text` is empty.
