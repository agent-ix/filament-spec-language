---
id: TC-2156
title: "A null primary names null"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-763
    type: verifies
---
# TC-2156: A null primary names null

## Description

A null primary names null.

Scope: FR-763-AC-2.

## Test Procedure

Run the `parse` operation over a unit whose function body is `null`. Test: `tests/it/forms_refusal_cause.rs` `a_null_primary_names_null`.

Tag the tests `#[trace("TC-2156", "FR-763-AC-2")]`.

## Expected Results

- `construct` `null-value`, `text` `null`, `declaration` `f`, `profile` `filament.value.complete/v1`, locus the span of `null`.
