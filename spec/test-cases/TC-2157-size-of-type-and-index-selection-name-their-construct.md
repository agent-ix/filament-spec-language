---
id: TC-2157
title: "Typed size and nested index forms pass S2"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-763
    type: verifies
---
# TC-2157: Typed size and nested index forms pass S2

## Description

Typed size and index selections build as forms (FR-981, FR-680), retaining
their operands, type arguments and source spans.

Scope: FR-763-AC-3.

## Test Procedure

Run `parse` over `size<Integer>(xs)` and `size(xs) + xs[1]`. Test:
`tests/it/forms_refusal_cause.rs`
`typed_size_and_nested_index_forms_parse_without_construct_refusals`.
Inspect forms in `fsl-forms/tests/it/value_forms.rs` with
`size_with_a_type_argument_builds_with_its_type_and_span` and
`size_and_index_forms_retain_their_nested_spans`.

Tag the tests `#[trace("TC-2157", "FR-763-AC-3")]`.

## Expected Results

- Both parse outcomes exit 0 with no diagnostics.
- Typed size retains its operand, result type and exact span. In the sum,
  `Size` and `Index` retain the spans of `size(xs)` and `xs[1]`, respectively.
