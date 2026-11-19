---
id: TC-2156
title: "An index selection carries its construct payload"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-763
    type: verifies
---
# TC-2156: An index selection carries its construct payload

## Description

An index selection S2 does not build carries the construct payload. `null`
outside a record field value is built and refused at check (FR-986), so it is
not a refusal construct.

Scope: FR-763-AC-2.

## Test Procedure

Run the `parse` operation over a unit whose function body is `xs[0]`. Test: `tests/it/forms_refusal_cause.rs` `an_index_selection_carries_its_construct_payload`.

Tag the tests `#[trace("TC-2156", "FR-763-AC-2")]`.

## Expected Results

- `construct` `index-selection`, `text` `xs[0]`, `declaration` `f`, `profile` `filament.value.complete/v1`, locus the span of `xs[0]`.
