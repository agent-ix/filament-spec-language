---
id: TC-2156
title: "Null and index forms pass S2"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-763
    type: verifies
---
# TC-2156: Null and index forms pass S2

## Description

Index selections and `null` build at S2. `null` outside a record field value
is refused at check (FR-986), so neither is an S2 refusal construct.

Scope: FR-763-AC-2.

## Test Procedure

Run `parse` over function bodies `null` and `xs[0]`. Test:
`tests/it/forms_refusal_cause.rs`
`null_and_index_forms_parse_without_construct_refusals`.
Inspect the S2 forms with `value_forms::null_expression_builds_as_its_own_form`
and `value_forms::formerly_unrepresented_value_constructs_build_with_their_spans`
in `fsl-forms/tests/it/value_forms.rs`.

Tag the tests `#[trace("TC-2156", "FR-763-AC-2")]`.

## Expected Results

- Both parse outcomes exit 0 with no diagnostics. S2 produces `Null` and
  `Index`; the latter retains `xs`, `0` and the exact span of `xs[0]`.
