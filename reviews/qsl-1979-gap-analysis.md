---
id: SR-2026111612172400000049
title: "Gap analysis of QSL-1979 export request surface, first slice (filament-spec-language PR 2953)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language@3a3b4fed164701f1ea8ccecdb7a05179a2cb0e9f; git diff origin/main...HEAD (PR 2953); FSL FR-1015 AC-1..4 (TC-2865, TC-2992), FR-255-AC-22 (TC-2993), FR-286-AC-22 (TC-2994); FSpec FR-811-AC-2 (TC-1970), FR-817 at agent-ix/filament-specification origin/main 0bf58005"
review_set: subset
date: 2026-11-18
---

## Summary

Ticket: QSL-1979 (first, partial slice). This is a manual check of each AC against its
tagged tests and the code, limited to the diff. The computed Quoin matrix was not used.
Plan completion: not assessed. The slice was declared partial: FR-1016..1018, FR-1022,
FSpec FR-812..818 lowering, golden files, oracles and the FR-865-AC-7 render are out of
scope, and their stubs stay ignored.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `the_export_settings_exist` is tagged FR-255-AC-22 but only asserts that `Setting::from_name` knows the three names. It checks none of the AC's clauses: the defaults listed by the settings operation (16777216, 100000, 16777216, equal to emit/s3), `export.nodes=7` changing only that limit, and `export.work_units` 1 stopping `incomplete`, bound 1, exit 22. The last clause fails today (code-review SR-2026111612172400000048 FND-001) | tests/it/export_surface.rs:131-137 |
| FND-002 | medium | `below_the_default_floor_an_export_refuses_with_no_text` is tagged FR-1015-AC-1 and FSpec-FR-811-AC-2, but it exports the counter's `Bounded` rather than the AC's `SlowEnough` over `Braking`. FSpec FR-811-AC-2 requires a request holding a failing obligation and a `preserved` one, naming the failing obligation and its cause, with no target file and no package. The test has no preserved record (impossible in this build), and checks neither that `package` is absent nor that no file is written | tests/it/export_surface.rs:19-38 |
| FND-003 | medium | FR-1015-AC-2/3/4 and FR-286-AC-22 are only partly asserted by their tagged tests. Missing: (AC-2) the conflict on `export.output_bytes` naming `limits` (the test uses `export.nodes` and does not check `limits`), and override-only `export.output_bytes` 1 giving `incomplete`, exit 22. (AC-3) the override writing byte-identical bytes, `last_stage` null on the existing-path refusal, and the member+override refusal naming `output`/`--output` with no file. (AC-4) two runs giving equal `package` members, a `{"export.nodes": 99999}` run differing only in that limit, and the exact key set. (FR-286-AC-22) `items`/`artifacts` `[]`, the below-floor document with category unsupported, the output_bytes-1 document with category incomplete, and the existing-path refusal with `last_stage`/`result` null. Reviewer probes show the code behaves correctly on the probed clauses, so the gap is in the tests | tests/it/export_surface.rs:40-129 |
| FND-004 | low | The ignored stubs in plan_export_surface.rs still carry the same TC/AC tags (FR-1015-AC-1..4, FR-255-AC-22, FR-286-AC-22, FSpec FR-811-AC-2/3, FR-817-AC-1..4/6), with ignore reasons that this PR makes false ("export limits are not implemented yet", "FSL does not write the export target file yet", "the export package key is not implemented yet"). Most would pass with `minimum_fidelity: any` and already assert the AC clauses FND-003 lists. The new parallel tests double-tag each criterion with weaker assertions instead of reusing the stubs | tests/it/plan_export_surface.rs:469-699 |
| FND-005 | medium | FR-1015-AC-6, AC-7 and AC-8, added on main by W53 (4ea19785d) with stubs in tests/it/export_rulings.rs keyed to QSL-1979, are surface rulings that belong to this slice (obligation set, floor-before-size order, generator identity). The PR's code contradicts all three (code-review FND-009/010/011), and no test in the PR asserts them. AC-5's cause shape is likewise contradicted (code-review FND-003) | tests/it/export_rulings.rs:102-170 (origin/main) |

## Scope and evidence

Examined:
- FR-1015 Behavior and AC-1..4. Behavior conforms on every probed clause (see the code-review SR scope), except that `initial` is unused (code-review FND-004).
- FR-255 export rows, defaults (export.output_bytes 16777216 = emit, nodes 100000, work_units 16777216). Defaults are correct in code (request.rs:73-78).
- FR-286 "Export outcomes": operation `export`, `last_stage` S4, result `{"kind":"exported","export":E}` with E equal to the run's `export` member. Asserted for success; probed correct for incomplete and unsupported.
- FSpec FR-811 "The fidelity floor": `preserved` / `conditional` / `any` semantics, refusal shape, no package and no text. Correct.
- FSpec FR-817 Request (strict member set, closed `output` record, limit names) and Exits (0/20/21/22). Correct; the mapper-failure exit 30 is unused (code-review FND-002).
- Context only: FR-1016..1018, FR-1022, FR-756-AC-1/2, FR-865-AC-7. These are declared out of slice and their stubs stay ignored; the PR body says so.
- PR body: honest about the partial slice and what is not delivered, except the claim about denied charges (code-review FND-001).

## Verdict

The slice's surface ACs are implemented, but the tagged tests overclaim. FR-255-AC-22 is
tagged with a name-only check and its work_units clause would fail. FR-1015-AC-1..4,
FR-286-AC-22 and FSpec FR-811-AC-2 are partly asserted. Fix by enabling the existing
stubs under `minimum_fidelity: any` (or extending the new tests) and dropping duplicate
tags, in this PR.
