---
id: TC-2214
title: "none and result build and check decides their position"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-091
    type: verifies
---
# TC-2214: none and result build and check decides their position

## Description

Verify that S2 builds `none` as `Absent` and `result` as `Result` instead of
refusing them as unrepresented, that check gives each its typed outcome by
position, and that the unrepresented constructs that remain still refuse the
unit. Scope: FR-091-AC-7 and FR-091-AC-76.

## Test Procedure

1. Check the four functions of FR-091-AC-76 and `function a using v():
   Option<Integer> pure { absent }`. Test:
   `fsl-semantics/tests/it/option_values.rs`
   `none_and_result_build_and_check_decides_their_position`.
2. Build the function bodies `xs[0]`, `size<Int[0, 1]>(c)`, `null` and
   `reaches(a, b, M::R)`.
   Test: `fsl-forms/tests/it/value_forms.rs`
   `formerly_unrepresented_value_constructs_build_with_their_spans`,
   `null_expression_builds_as_its_own_form` and
   `a_qualified_edge_in_a_function_builds_with_its_name_and_span`.

## Expected Results

- Step 1: `o` and `a` check; `n` refuses `ill_typed`/`type-mismatch`; the `let`
  refuses `ill_typed`/`ambiguous-literal`; `r` refuses
  `wrong_snapshot`/`wrong-anchor`; none refuses `unsupported_construct`.
- Step 2: `xs[0]` builds as `Index` retaining its operand, index and exact
  span (FR-680); `size<Int[0, 1]>(c)` builds as `Size` retaining its operand,
  result type and exact span (FR-981);
  `null` builds as `Null`; `reaches(a, b, M::R)` builds as `Reaches`
  retaining both operands, the full edge name and the edge's exact span.
