---
id: TC-2410
title: "An analyze request over a selected checked package equals the request over its source"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-850
    type: verifies
---
# TC-2410: An analyze request over a selected checked package equals the request over its source

## Description

Verify that a `native-analyze/1` request selecting a v2 checked package is
analyzed from the package bytes alone, gives the source route's outcome, and
refuses malformed or mismatched package input before any engine runs.
Scope: FR-850-AC-1 to FR-850-AC-5.

## Test Procedure

1. Compile the Counter program with `Bounded`, `BelowThree` and `AtMostTwo` to its package bytes; analyze it from source and from the package, with explicit items and with no `items`.
2. Compare the `AtMostTwo` refutations of the two routes.
3. Select a package file holding `{}`, a v1 document, the bytes with one source-map entry removed, and non-JSON bytes.
4. Add `program`, a `libraries` entry or `integer_division` beside `package`; empty `models`; add a `models` entry the package does not record.
5. Ask for the item `{"claim": "Missing"}` over the package.

## Expected Results

- Step 1: equal outcomes except the `request` selection member; equal exit codes; equal default item order.
- Step 2: equal counterexample and replay target, `reproduced` `true`.
- Step 3: each refuses `invalid_package` at the request stage with the reader's cause and pointer, exit 20, `items` empty.
- Step 4: `invalid-request` naming the member; `missing_import`/`missing-selection` naming `test/counter`; `invalid_package`/`conflicting-definition` naming the extra entry; each exit 20.
- Step 5: the item settles `invalid-request` `unknown-claim` naming `Missing`.

Test: `fsl-analyze/tests/it/analyze_package_selection.rs`.
