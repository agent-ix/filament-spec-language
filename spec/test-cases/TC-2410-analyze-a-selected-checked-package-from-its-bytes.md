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
analyzed from its handoff directory's package, manifest and root/dependency
witness files alone, gives the source route's outcome, and
refuses malformed or mismatched package input before any engine runs.
Scope: FR-850-AC-1 to FR-850-AC-5 and FR-850-AC-12 to AC-16.

## Test Procedure

1. Compile the Counter program with `Bounded`, `BelowThree` and `AtMostTwo` through a compile request carrying the `handoff` member; analyze it from source and from the handoff directory selected as `package` `{"directory": ...}`, with explicit items and with no `items`.
2. Compare the `AtMostTwo` refutations of the two routes.
3. Select a handoff directory whose `checked-package.json` holds `{}`, a v1 document, the bytes with one source-map entry removed, and non-JSON bytes.
4. Add `program`, a `libraries` entry or `integer_division` beside `package`; empty `models`; add a `models` entry the package does not record.
5. Ask for the item `{"claim": "Missing"}` over the package.
6. Compile a root importing `test/z` as `z`, whose `answer` calls `test/a::answer`, returning Integer `42`. The root's function returns `z::answer()` and its Counter temporal claim is `always holds(c.value <= z::answer() - 40)`. Run its function and analyze its claim from source and from the written handoff. Remove source files and `sources/`, then repeat package execution and counterexample replay.
7. Compile a diamond sharing `test/shared`. Check its one shared artifact triple; remove a transitive artifact, supply a foreign/conflicting artifact, and mutate manifest order, uniqueness, closure membership, member shape and canonical paths. Exercise a symlink, non-regular file and lock cycle.
8. Remove each dependency witness in turn, including an empty equality file, then swap non-empty witnesses between distinct packages; retain the unchanged directory as a positive control.
9. Measure the total files/bytes of the selected root and closure once, then run at the exact byte allowance and one below; test totals 64 and 65 files and exact/one-short strict-read/native-join work/storage allowances.
10. Add a domain selection used only by a library in that closure, supplying its matching document beside the root's documents; repeat with that document missing and with content the dependency's recorded selection refuses.

## Expected Results

- Step 1: equal outcomes except the `request` selection member; equal exit codes; equal default item order.
- Step 2: equal counterexample and replay target, `reproduced` `true`.
- Step 3: each refuses `invalid_package` at the request stage with the reader's cause and pointer, exit 20, `items` empty.
- Step 4: `invalid-request` naming the member; `missing_import`/`missing-selection` naming `test/counter`; `invalid_package`/`conflicting-definition` naming the extra entry; each exit 20.
- Step 5: the item settles `invalid-request` `unknown-claim` naming `Missing`.
- Step 6: function value `42` on both routes and equal source/package item records/counterexamples/reproduced replays, including after source deletion; no source compile or source provision is consulted.
- Step 7: the diamond's shared triple is read/admitted once; every invalid variant refuses before any item runs with the exact dependency-admission code/cause and its own file/pointer. A missing manifest refuses even for an import-free unit.
- Step 8: missing indexed witness files refuse `missing-file`; foreign witness joins retain the original refusal at the dependency's file, never joined against the root; the positive control completes.
- Step 9: exact allowances complete; one-short bytes/file-count stops are intake `incomplete`/`resource_exhausted`, exit 22; read/join stops retain the original `i2.*` setting and typed refusal, with no per-dependency budget reset. Shared diamond files count once.
- Step 10: the library-only domain document is admitted against that library; its removal refuses missing selection, and changed content retains the dependency reader's refusal, never an extraneous-root-model refusal or a substituted witness.

Tests: package-route command integration tests and `fsl-analyze/tests/it/analyze_package_selection.rs`.
