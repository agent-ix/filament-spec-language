---
id: TC-2873
title: "An equality over a mod application emits and reads back"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-495
    type: verifies
---
# TC-2873: An equality over a mod application emits and reads back

## Description

Verify that an equality whose operand is an integer division application (the shape of codegen's `self.n mod self.aux = 0` clause) emits and is admitted by Contract IR's independent reader. Scope: FR-495-AC-24.

## Test Procedure

1. Compile each source of FR-495-AC-24 through the public `compile_package` (and `compile_package_with_integer_division` for `div` and `rem`) with the `example/config-version` model selected.
2. Read each package with `CheckedPackageV2::read`, supplying the package's required features and the model document as evidence.
3. For each source, find the equality application, the position of its `mod`, `div` or `rem` operand, the builtin `Integer` node, and the equality decision.

## Expected Results

- Step 1: every source compiles; none refuses `MissingEqualityIdentityContext`.
- Step 2: every read admits.
- Step 3: one equality decision per source; the division application's `semantic_type` is the builtin `Integer` node, and the decision at that operand's position has `source_type` equal to that node, for all eight sources.

Test: `fsl-replay/src/application/division_tests.rs`
`an_equality_over_a_mod_application_emits_and_reads_back`, tagged `#[trace("TC-2873", "FR-495-AC-24")]`.
