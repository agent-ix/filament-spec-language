---
id: TC-3020
title: "Export admission codes, observer refusal, charges and cause spelling"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-1015
    type: verifies
---
# TC-3020: Export admission codes, observer refusal, charges and cause spelling

## Description

Verify the structured refusal codes of malformed export members, the `unsupported` refusal of an observer the build cannot run, the `limit-reached` cause of a denied charge, the once-per-node charge of shared closure nodes, and the spelling of a cause's `node` and `locus`. Scope: FR-1015-AC-9 to AC-13.

## Test Procedure

1. Export the counter with `obligations` `[]`, without `obligations`, and with `minimum_fidelity` `7`.
2. Export `SlowEnough` over `Braking` under `any` with `observer` `tlc` and an `output` path, then with `export.nodes` 1 added, then without `observer`.
3. Export the counter under `export.work_units` 1, then under `export.nodes` 1 and `export.work_units` 1.
4. Over the spine unit with invariants `Low` and `High`, find the least `export.nodes` for `["Low"]`, `["High"]` and `["Low", "High"]`.
5. Export `SlowEnough` over `Braking` under `any` and read each cause's `node` and `locus`.

## Expected Results

- Step 1: `code` `invalid_runtime_input` with `cause` `invalid-value`, `missing-member` and `wrong-value-kind`, at their `/request/export` paths, exit 20; no `invalid-request`.
- Step 2: `unsupported_projection`/`unsupported-requested-capability` naming `tlc`, exit 21, no text, package or file; the same with `export.nodes` 1, not `incomplete`; then `assembled`, exit 0.
- Step 3: `incomplete`, `records` `[]`, `cause` `limit-reached` with `work_units`, 1 and `export.work_units`, exit 22; then `nodes`, 1 and `export.nodes`.
- Step 4: the pair's value is less than the sum of the singles and at least their maximum.
- Step 5: each `node` is a canonical node-id string of the package, equal to the matching `name_map` entry's; each `locus` is `null` or a region with `start < end`.
