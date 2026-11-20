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

An unrepresented-construct refusal for the `declaration` row carries its
payload, its text cut at 120 bytes, and a message that begins with the row's
sentence. No source reaches the row, so the test builds the cause directly.

Scope: FR-763-AC-5.

## Test Procedure

1. Build the `FormsCause::UnrepresentedConstruct` refusal for the
   `declaration` row over the text `unclaimed` and over a 200-byte text.
   Test: `fsl-forms/src/dispatch.rs`
   `an_unrepresented_declaration_construct_carries_its_payload_and_cut_text`.
2. Render the compile refusal that holds the cause, with and without an owning
   declaration. Test: `fsl-replay/src/spine/forms_message_tests.rs`
   `a_declaration_construct_message_begins_with_the_construct_and_names_its_owner`.

Tag the tests `#[trace("FR-763-AC-5")]`.

## Expected Results

- The catalog code is `unsupported_construct`; the construct spelling is
  `declaration`; `declaration` and `profile` are `null` when no owner is set.
- The 200-byte construct's `text` is at most 123 bytes and ends with `...`;
  the short text is kept whole.
- The message is `` `unclaimed` declarations are not supported yet ``, followed
  by ` (declaration Owner)` when the owner is set.
