---
id: TC-2155
title: "An undispatched declaration names its keyword"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-763
    type: verifies
---
# TC-2155: An undispatched declaration names its keyword

## Description

An undispatched declaration names its keyword.

Scope: FR-763-AC-1.

## Test Procedure

Run the `parse` operation over a unit whose one declaration has a leading keyword with no S2 dispatch entry and read its outcome document's diagnostic. Test: `tests/it/forms_refusal_cause.rs` `an_undispatched_declaration_names_its_keyword`.

Tag the tests `#[trace("TC-2155", "FR-763-AC-1")]`.

## Expected Results

- `code` `unsupported_construct`, `cause` `not-yet-implemented`.
- `construct` `declaration`, `text` the keyword, `declaration` the declared name, `profile` the `using` profile identity.
