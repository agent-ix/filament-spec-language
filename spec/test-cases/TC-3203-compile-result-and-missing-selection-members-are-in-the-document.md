---
id: TC-3203
title: "The compile document names the package and where it was written, and missing selections carry their kind"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-286
    type: verifies
  - target: ix://agent-ix/filament-spec-language/FR-810
    type: verifies
---
# TC-3203: The compile document names the package and where it was written, and missing selections carry their kind

## Description

Verify the `packaged` result and the `selection` member. Scope: FR-286-AC-39, FR-286-AC-40 and FR-810-AC-6.

## Test Procedure

1. Compile FR-810-AC-1's request (file in directory `reqs`) with `output`, without it, with `RequestOverrides::output` `cli.package.json`, and with `handoff`; validate each document against `OUTCOME_SCHEMA_1`.
2. Compile a refusing request, and run `check`.
3. For each of the nine `selection` strings of FR-286 "Missing selections", build its producer's diagnostic, serialize it, validate it, and call `hint_for` and `check_hint_catalog`.

## Expected Results

- Step 1: `result` is `{"kind":"packaged","package":{"bytes":N,"output":"reqs/spine.package.json","handoff":null}}`; then `"output":null`; then `"output":"cli.package.json"`; then `"handoff":"reqs/<directory>"`; the file exists at the named path; `artifacts` is `[]`; each document validates.
- Step 2: `result` is `null`.
- Step 3: each diagnostic carries its `selection` and the table's `hint`; the schema accepts exactly the nine values; `hint_for` returns each text; `check_hint_catalog` returns `Ok`; the `UnresolvedDependency` diagnostic has no `selection` and the pair's catalog `hint`.

Test: `fsl-command/tests/it` and `fsl-replay` outcome tests.
