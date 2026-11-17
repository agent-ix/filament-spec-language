---
id: TC-2297
title: "A compile request or fsl compile writes the checked package to a file"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-810
    type: verifies
---
# TC-2297: A compile request or fsl compile writes the checked package to a file

## Description

Verify that a compile request's `output` member, and the library entry the
driver's `--output` reaches, write the exact checked-package bytes to a new
file, and every refusal writes none. Scope: FR-810-AC-1 to FR-810-AC-5; AC-4 is a driver test in the filament-driver repository.

## Test Procedure

1. Compile `tests/fixtures/spine-compile.native` with `output` `{"package": "spine.package.json"}`; compile it again.
2. Compile a program that does not check with `output`; compile with `output` `{"package": ""}`, `{"path": "x"}`, `{"package": 3}` and a path under a missing directory.
3. Call `compile_outcome_with_overrides` with `RequestOverrides::output` `Some("cli.package.json")` over the request without `output`, and over step 1's request.
4. Through the driver, run `fsl compile <request> --output <file>`, and `fsl run` with `--output` (filament-driver).
5. Compile with FR-665-AC-1's `handoff` and `output` `{"package": "spine.package.json"}`, then with `output` `{"package": "handoff/pkg.json"}`.

## Expected Results

- Step 1: exit 0, file bytes equal the returned package bytes, `artifacts` `[]`; the second run refuses `invalid-request`, exit 20, naming the path, and the file is unchanged.
- Step 2: no file; `invalid-request` at stage `request` naming the member; the missing directory fails with its I/O code, exit 20.
- Step 3: the file equals step 1's; both members together refuse naming `output` and `--output` and write nothing.
- Step 4: the file equals step 1's; `fsl run --output` is refused as an unknown option.
- Step 5: both written, byte-equal; the inside-handoff path refuses `invalid-request` at stage `request` naming `output` and `handoff`, and writes neither.

Test: `tests/it/compile_package_output.rs`.
