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

Verify the structured refusal codes of malformed export members, the `unsupported` refusal of an observer the build cannot run, the `limit-reached` cause of a denied charge, the once-per-node charge of shared closure nodes, and the spelling of a cause's `node` and `locus`. Scope: FR-1015-AC-9 to AC-17.

## Test Procedure

1. Export the counter with `obligations` `[]`, without `obligations`, and with `minimum_fidelity` `7`.
2. Export `SlowEnough` over `Braking` under `any` with `observer` `tlc` and an `output` path, then with `export.nodes` 1 added, then without `observer`.
3. Export the counter under `export.work_units` 1, then under `export.nodes` 1 and `export.work_units` 1.
4. Over the spine unit with invariants `Low` and `High`, find the least `export.nodes` for `["Low"]`, `["High"]` and `["Low", "High"]`.
5. Export `SlowEnough` over `Braking` under `any` and read each cause's `node` and `locus`.
6. Export the counter's `Bounded` to TLA+ and to Lean; count its export definitions N (the blank-line-separated blocks other than the header block and the footer) and find the least `export.nodes` C; export again under `export.work_units` C + N and C + N - 1.
7. In a build that does not lower protocol subjects, export a temporal clause that a protocol requires, under the default floor and under `any`.
8. Export a program whose checked package admission refuses `invalid_package`, and one with a missing import.
9. Export with a `protocols` binding an analyze request accepts, then with an unknown member in it.

## Expected Results

- Step 1: `code` `invalid_runtime_input` with `cause` `invalid-value`, `missing-member` and `wrong-value-kind`, at their `/request/export` paths, exit 20; no `invalid-request`.
- Step 2: `unsupported_projection`/`unsupported-requested-capability` naming `tlc`, exit 21, no text, package or file; `export.refusal` holds exactly `code`, `cause`, `capability` `observer`, `observer` `tlc`, a non-empty string `reason` and `details` `{"path": "/request/export/observer"}`; the same with `export.nodes` 1, not `incomplete`; then `assembled`, exit 0.
- Step 3: `incomplete`, `records` `[]`, `cause` `limit-reached` with `work_units`, 1 and `export.work_units`, exit 22; then `nodes`, 1 and `export.nodes`.
- Step 4: the pair's value is less than the sum of the singles and at least their maximum.
- Step 5: each `node` is a canonical node-id string of the package, equal to the matching `name_map` entry's; each `locus` is `null` or a region with `start < end`.
- Step 6: `assembled`, exit 0, under C + N; `incomplete` with `limit-reached` `work_units`, value C + N - 1, `export.work_units`, exit 22, under C + N - 1.
- Step 7: `refused` with the seven-member `unimplemented_capability`/`unimplemented-meaning` refusal naming the protocol clause construct, exit 21, no text, package or file, under both floors.
- Step 8: each keeps its own code, cause, locus and exit; neither exits 30.
- Step 9: admitted with the binding; then the analyze reader's refusal at `/request/export/protocols/0`, exit 20.
