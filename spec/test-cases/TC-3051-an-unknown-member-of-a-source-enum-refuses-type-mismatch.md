---
id: TC-3051
title: "An unknown member of a source enum refuses type-mismatch"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-317
    type: verifies
  - target: ix://agent-ix/filament-spec-language/FR-360
    type: verifies
---
# TC-3051: An unknown member of a source enum refuses type-mismatch

## Description

Verify that a qualified member reference whose qualifier names an enum or
ordered enum that does not declare the member refuses `ill_typed`/
`type-mismatch` naming the type and the member, with the code and tag an
undeclared model variant case and an undeclared union member get, while a
qualifier that names no type refuses `missing_declaration`/`missing-name`.
Scope: FR-317-AC-6.

## Test Procedure

1. Check a unit declaring `enum Status { Open, Closed }` with a function
   whose body is `Status::Pending = Status::Open`.
2. Check a unit declaring `ordered enum Level { Low, Mid, High }` with a
   function returning `Level` whose body is `Level::Top`.
3. Check a function whose body is `Config::Tier::Lost` over the variant
   fixture of FR-360-AC-5.
4. Check a function whose body is `Nope::Open = Status::Open`.

## Expected Results

- Steps 1 and 2: one refusal each, `ill_typed`/`type-mismatch` at the
  qualified name, naming `Status` and `Pending`, and `Level` and `Top`.
- Step 3: `ill_typed`/`type-mismatch`, the same code and tag as steps 1 and 2.
- Step 4: `missing_declaration`/`missing-name` at `Nope::Open`.
- The four steps give exactly four refusals, one per unknown member.

## Notes

The live tests
`fsl-semantics/tests/it/variant_fields.rs::undeclared_variant_cases_name_the_declared_owner_for_model_variants_and_source_enums`
and `an_unknown_member_of_a_source_enum_refuses_type_mismatch_like_a_model_variant_case`
both assert `ill_typed`/`type-mismatch` for an unknown source enum member
(`Status::Unknown`, `Status::Pending`), as FR-317-AC-6 states. The first was
renamed from `..._keep_source_enum_names_missing`; its earlier `missing-name`
expectation for `Status::Unknown` is superseded by FR-317-AC-6.
