---
id: TC-2160
title: "The parse document renders the construct cause"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-763
    type: verifies
---
# TC-2160: The parse document renders the construct cause

## Description

The parse document renders the construct cause.

Scope: FR-763-AC-6.

## Test Procedure

Serialize the `parse` outcome document of TC-2156's unit. Test: `tests/it/forms_refusal_cause.rs` `the_parse_document_renders_the_construct_cause`.

Tag the tests `#[trace("TC-2160", "FR-763-AC-6")]`.

## Expected Results

- One diagnostic: `code` `unsupported_construct`, `cause` `not-yet-implemented` (not `null`), with the payload of TC-2156.
- Category `unsupported`, exit 21.
