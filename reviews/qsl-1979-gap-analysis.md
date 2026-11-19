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

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-006 | medium | Four new tests in export_surface.rs claim ACs they do not assert. `an_unlowered_invariant_is_a_mapper_failure` is tagged FR-1015-AC-1, which is the `SlowEnough`/`Braking` floor case; the mapper-failure rule is FR-1015 Behavior and has no AC of its own. `a_refused_record_fails_the_floor_before_a_mapper_failure` is tagged FR-1015-AC-7, which is floor before `export.output_bytes`. `the_initial_member_must_name_a_snapshot` is tagged FR-1015-AC-3, which is the output file. `an_observer_is_refused_as_not_run_by_this_build` is tagged FR-1015-AC-2, which is limits. Each tag inflates its AC's coverage with an unrelated behaviour. Untag them, or tag them to the right criterion once one exists | tests/it/export_surface.rs:26-113 |

## Dispositions

Round 1, head d0b22461599864133bc136765ba904d34a4b10ed. The focused run of export_surface::, plan_export_surface:: and export_rulings:: (--test-threads=1, house skips) gave 15 passed, with the rest ignored.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | The FR-255-AC-22 stub `the_export_settings_take_the_compile_defaults` is un-ignored and now asserts the names, the compile counterparts, the `export.nodes=7` parse, and `export.work_units` 1 giving `incomplete` `{limit, bound: 1}`, exit 22. The defaults are asserted through the package key test |
| FND-002 | fixed | `an_export_below_its_fidelity_floor_refuses_with_no_text` is un-ignored over `SlowEnough`/`Braking`. It asserts refused / unsupported_projection / below-fidelity-floor, the failing obligation, no `target_text`, no `package`, no file, category unsupported, and the `any` case. The `preserved` companion record of FSpec FR-811-AC-2 cannot exist without FR-812 lowering; this is stated in the test doc and the remaining case stays ignored |
| FND-003 | fixed | The stubs are un-ignored and cover the missing clauses. AC-2: override-only `incomplete` 22, and the doubled `export.output_bytes` naming `limits`. AC-3: override byte-identical, existing path with `last_stage`/`result` null, member+override naming `output`/`--output` with no file. AC-4: the key set, two equal runs, the 99999 difference. FR-286-AC-22: `items`/`artifacts` `[]`, success, incomplete, unsupported (via the AC-1 test). They run over `Braking` instead of the AC's counter, because the counter's invariant is now a mapper failure until FR-812 |
| FND-004 | fixed | The stale "not implemented" stubs are un-ignored or carry accurate reasons (FR-812 lowering), and the weaker duplicate tests were removed from export_surface.rs |
| FND-005 | fixed | Un-ignored and passing: AC-5's hybrid part (`a_hybrid_claim_is_unrepresented_by_its_flow_nodes`, tla and lean), AC-6 (`obligations_are_nonempty_declared_names`), AC-7 (`the_floor_is_decided_before_the_text_is_charged`) and AC-8 (`the_generator_is_the_application`). The parts that need a preserved Bounded record are split into ignored tests keyed QSL-1979 |
| FND-006 | fixed | The mis-tags are gone: `an_unlowered_invariant_is_a_mapper_failure` and `the_initial_member_must_name_a_snapshot` are untagged; `a_refused_record_fails_the_floor_before_a_mapper_failure` is retagged FR-1015-AC-6 (refused not-an-obligation records failing the floor, exit 21), which it does assert; the observer test was removed in favour of the AC-10 stub |

Round 2, head 94d3a82647b51c8eac60a9316f8f98aa04533076. Focused run (export_surface::, plan_export_surface::, export_rulings::, --test-threads=1, house skips): 20 passed, 38 ignored. FR-1015-AC-9..13 stubs are un-ignored and pass.

## New findings (disposition pass 2)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-007 | low | `an_observer_the_build_cannot_run_is_unsupported` (FR-1015-AC-10) sets `output` `{"file": "m.tla"}` but never asserts AC-10's "no file at its `output` path". It also asserts neither the Behavior's `last_stage` null nor `details.path`, which is how FND-014 of the code review went unnoticed | tests/it/export_rulings.rs:492-515 |
