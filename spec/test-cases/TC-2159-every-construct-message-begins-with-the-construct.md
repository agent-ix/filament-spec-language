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

For each construct kind source reaches (every kind but `declaration`), run the `parse` operation over a unit holding it and read the message; repeat with a `size<T>(e)` whose source text is 200 bytes. Test: `tests/it/forms_refusal_cause.rs` `every_construct_message_begins_with_the_construct`.

Tag the tests `#[trace("TC-2159", "FR-763-AC-5")]`.

## Expected Results

- Each message begins with its row's sentence filled from `text`.
- The 200-byte construct's `text` is at most 123 bytes and ends with `...`; no `text` is empty.
