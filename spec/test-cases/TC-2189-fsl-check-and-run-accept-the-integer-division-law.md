---
id: TC-2189
title: "fsl check and fsl run accept the integer-division law"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-765
    type: verifies
---
# TC-2189: fsl check and fsl run accept the integer-division law

## Description

The driver's `fsl check` takes `--integer-division <law>`, and `fsl run`
reads the request member; the option is refused where it does not apply.

Scope: FR-765-AC-7.

## Test Procedure

The driver repository owns the binary, so the test is in filament-driver's
`fsl-cli/tests/cli_contract.rs`, `integer_division_option_and_member_reach_check_and_run`.

1. Run `fsl check u.fsl --integer-division floor --format json` over FR-670-AC-1's unit without its header, then without the option, with `ceiling`, with the option twice, and over the unit with a floor header and `--integer-division truncating`.
2. Run `fsl run` over a request with `"integer_division": "floor"` calling `q(-7, 3)`.
3. Run `fsl run --integer-division floor <request>` and `fsl compile --integer-division floor <request>`.

The driver numbers its own artifacts, so its test names this case and criterion in its doc comment (`TC-2189`, `FR-765-AC-7` of filament-spec-language) rather than as a bare trace id.

## Expected Results

- Step 1: exit 0; then exit 20 `missing-selection`; exit 20 `unsupported-selection` naming `--integer-division` and `ceiling` before reading `u.fsl`; exit 20 with usage on stderr and empty stdout; exit 20 `conflicting-definition`.
- Step 2: exit 0, value `-3`.
- Step 3: each exits 20 as an unknown option.
