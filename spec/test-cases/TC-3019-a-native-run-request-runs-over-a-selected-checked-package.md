---
id: TC-3019
title: "A native-run request runs over a selected checked package"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-1039
    type: verifies
---
# TC-3019: A native-run request runs over a selected checked package

## Description

Verify that a package-only `native-run/1` request runs from the package's
own records, never reads or compiles source, and refuses a package that
cannot run with the code and locus stated. Scope: FR-1039-AC-1 to
FR-1039-AC-3.

## Test Procedure

1. Compile `half` (`-7 div 2`) with `integer_division` `floor`, delete the
   source, and call it from the package file; repeat with `integer_division`;
   send a request with neither `program` nor `package`.
2. Run `Pick` from the handoff directory and from the package file; send a
   `clause` member over the package file.
3. Send a missing file, a v1 document, a package with an unknown member, no
   `models` entry for a spine-model package, and a spine `models` entry for
   the value package.

## Expected Results

- Step 1: `-4`; `invalid-request` naming `integer_division`, no file opened;
  `invalid-request` naming `program`.
- Step 2: the source route's outcomes; `invalid-request` with `requires`
  `directory`.
- Step 3: each refusal at stage `request`, exit 20, at its row's locus.
