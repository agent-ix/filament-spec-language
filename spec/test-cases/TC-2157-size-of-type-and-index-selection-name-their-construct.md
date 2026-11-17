---
id: TC-2157
title: "Index selection names its construct"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-763
    type: verifies
---
# TC-2157: Index selection names its construct

## Description

An index selection that S2 does not build names its construct. `size<N>(e)`
is built as a `Size` form (FR-981) and is not a refusal construct.

Scope: FR-763-AC-3.

## Test Procedure

Run the `parse` operation over the function body `size(xs) + xs[1]`. Test:
`tests/it/forms_refusal_cause.rs` `an_index_selection_names_only_its_indexing_expression`.

Tag the tests `#[trace("TC-2157", "FR-763-AC-3")]`.

## Expected Results

- `index-selection` with `text` `xs[1]` and the span of `xs[1]`, not the whole body.
