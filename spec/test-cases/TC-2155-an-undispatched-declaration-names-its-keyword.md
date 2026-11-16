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

Run the `parse` operation over a unit holding a `synthesis` declaration and over one holding a `verify` declaration, read each outcome document's diagnostics, and read `leading_keywords()`. Test: `tests/it/forms_refusal_cause.rs` `an_undispatched_declaration_names_its_keyword`.

Tag the tests `#[trace("TC-2155", "FR-763-AC-1")]`.

## Expected Results

- Neither document holds an `unsupported_construct` diagnostic or a diagnostic whose `construct` is `declaration`.
- `leading_keywords()` yields `synthesis` and `verify`.
