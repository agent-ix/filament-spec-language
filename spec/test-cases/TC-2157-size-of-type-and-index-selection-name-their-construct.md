---
id: TC-2157
title: "Size-of-type and index selection name their construct"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-763
    type: verifies
---
# TC-2157: Size-of-type and index selection name their construct

## Description

Size-of-type and index selection name their construct.

Scope: FR-763-AC-3.

## Test Procedure

Run the `parse` operation over function bodies `size<Integer>(xs)` and `xs[0]`. Test: `tests/it/forms_refusal_cause.rs` `size_of_type_and_index_selection_name_their_construct`.

Tag the tests `#[trace("TC-2157", "FR-763-AC-3")]`.

## Expected Results

- `size-of-type` with `text` `size<Integer>(xs)`.
- `index-selection` with `text` `xs[0]`.
