---
id: TC-2188
title: "A request member supplies the integer-division law"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-765
    type: verifies
---
# TC-2188: A request member supplies the integer-division law

## Description

A `native-compile/1` or `native-run/1` request's `integer_division` member
supplies the law for `div` and `rem`, composes with a header law, and is
refused when malformed or unknown.

Scope: FR-765-AC-1, FR-765-AC-2, FR-765-AC-3, FR-765-AC-4, FR-765-AC-5, FR-765-AC-6.

## Test Procedure

All tests are in `fsl-command/tests/it/integer_division_request.rs`, through
`fsl_command::command::compile_outcome` and `fsl_command::command::run`.

1. For each law, compile and run FR-670-AC-1's unit without its header, with the member set to the law. Test: `request_member_selects_each_law_for_compile_and_run`.
2. Compile the unit with a floor header and the member `floor`, then `truncating`. Test: `request_member_and_header_agree_or_conflict`.
3. With the program file absent, send the member as `null`, `3`, `ceiling` and `Floor`, and a run request carrying `package` and the member. Test: `malformed_or_unknown_member_refuses_before_any_file_is_read`.
4. Compile the unit with no member and no header, and a `mod`-only unit with no member. Test: `no_member_and_no_header_still_refuses_missing_selection`.
5. Run a request whose program member is `floor` and whose library entry member is `truncating`, then without the library member. Test: `library_entry_takes_its_own_law`.
6. Compile a `mod`-and-`+` unit with and without the member. Test: `member_over_a_unit_without_div_or_rem_leaves_the_package_unchanged`.

Tag the tests `#[trace("TC-2188", "FR-765-AC-N")]` with the criterion each asserts.

## Expected Results

- Step 1: exit 0; `q(-7, 3)`, `r(-7, 3)` are `-2`, `-1` under truncating and `-3`, `2` under floor and Euclidean.
- Step 2: floor compiles; truncating refuses `invalid_package`/`conflicting-definition`, no package bytes.
- Step 3: `invalid-request` naming `integer_division` for `null`, `3` and the `package` case; `unknown_profile`/`unsupported-selection` for `ceiling` and `Floor`; no file read but the request.
- Step 4: `missing_declaration`/`missing-selection` at each operator; the `mod` unit compiles.
- Step 5: `ld(-7, 3)` is `-2` and `q(-7, 3)` is `-3`; without the library member the library's `div` refuses `missing-selection`.
- Step 6: equal package bytes.
