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
2. Build the function bodies `xs[0]`, `null` and `reaches(a, b, M::R)`.
   Test: `fsl-forms/tests/it/value_forms.rs`
   `a_construct_no_variant_represents_refuses_the_unit`.

## Expected Results

- Step 1: `o` and `a` check; `n` refuses `ill_typed`/`type-mismatch`; the `let`
  refuses `ill_typed`/`ambiguous-literal`; `r` refuses
  `wrong_snapshot`/`wrong-anchor`; none refuses `unsupported_construct`.
- Step 2: each refuses `UnrepresentedConstruct` at its construct.
