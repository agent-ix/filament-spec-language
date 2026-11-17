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
3. Read the equality decision and the `mod` application of each `mod` case.

## Expected Results

- Step 1: every source compiles; none refuses `MissingEqualityIdentityContext`.
- Step 2: every read admits.
- Step 3: one equality decision per source; an operand's `source_type` is the `mod` application's own `semantic_type`.

Test: `fsl-replay/src/application/division_tests.rs`
`an_equality_over_a_mod_application_emits_and_reads_back`, tagged `#[trace("TC-2873", "FR-495-AC-24")]`.
