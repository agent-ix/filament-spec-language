---
id: SR-3617002
title: "Gap analysis — QSL-2962 PR #3617 call_site on the compile request"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; git diff origin/main...f0f9fc71d346391379b5fa28c1419a2b55e21fe4; FR-027-AC-15..21, TC-3210, FR-121-AC-35/36, FR-865-AC-19, FR-286-AC-40"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-027
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-121
    type: references
---

## Summary

Ticket: QSL-2962. Gap analysis, planless, of PR #3617 at head f0f9fc71d346391379b5fa28c1419a2b55e21fe4. Matrix: `quire matrix --scope . --format tsv`, with no run evidence and no cargo run.

1. **Every new production path has a test.** `request_call_site` is covered by the AC-21 test. The `locate_in_checked` call and the `RunCause::CallSite` refusal arms are covered by the AC-15 to AC-18 tests. Located success is covered by AC-20. The exception is the `Fault` arm (SR-3617001 FND-001). There is no unowned code: every hunk traces to FR-027's `call_site` statement or to the FR-121 locate.
2. **No stubs and no tautologies.** Each test compares the exact diagnostic array with `assert_eq!` and asserts the filesystem state.
3. **The matrix cannot see AC-14 to AC-21 (FND-001).** The FR-027 acceptance table is split by a blank line at spec line 159. The computed matrix therefore lists only FR-027-AC-2 to AC-13, and the TC-3210/FR-027-AC-15..21 tags in this PR bind to nothing. Spec PR #3618 (the `first` -> `pick` rename) does not fix the split. Bindings were checked by grep instead: AC-15, 16, 17, 18, 20 and 21 each have one tagged test in fsl-command/tests/it/cli_contract/compile.rs, and AC-15 has a second in selection_text.rs. AC-19 is known open and pending #3618.
4. **FR-121-AC-35/36 are untagged (FND-002).** The tests exist in fsl-replay/tests/imported_function_site.rs: `a_library_without_a_direct_import_refuses_unknown_import` and `an_undeclared_or_unimportable_function_refuses_paired_with_the_library`. Both carry only `TC-516`.

## Verdict

**FAIL by the verdict rule (untagged criteria). The code is not at fault.** The untagged criteria are FR-027-AC-14..21, hidden by a spec table defect, and FR-121-AC-35/36, a tag-only omission. Neither is a missing test. FND-002 is a two-attribute fix that belongs in this PR. FND-001 belongs to the spec owner and should ride with #3618. The test-coverage gap for `CallSiteRefusal::Fault` is recorded once, in SR-3617001 FND-001.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | A blank line at spec line 159 splits FR-027's acceptance table, so AC-14 to AC-21 are absent from the computed matrix (`quire matrix` lists FR-027-AC-2..13 only) and every TC-3210 FR-027-AC-15..21 tag binds nothing. Spec PR #3618 does not remove the line | spec/functional/FR-027-export-compiled-native-package.md:158-160 |
| FND-002 | medium | FR-121-AC-35 and FR-121-AC-36 are `untagged` in the matrix. Their tests exist but carry only `TC-516`: `a_library_without_a_direct_import_refuses_unknown_import` (:320) and `an_undeclared_or_unimportable_function_refuses_paired_with_the_library` (:399) | fsl-replay/tests/imported_function_site.rs:320; fsl-replay/tests/imported_function_site.rs:399 |

FND-001 fix (spec): delete the blank line between the AC-13 and AC-14 rows, in #3618 or a follow-up spec PR. Then re-run `quire matrix` and confirm AC-15 to AC-21 are `tagged`.

FND-002 fix: add `FR-121-AC-35` to the trace at :320 and `FR-121-AC-36` to the one at :399.

## Coverage

- FR-027-AC-15, 16, 17, 18, 20, 21: examined. Each has an exact-document test tagged TC-3210. Invisible to the matrix (FND-001).
- FR-027-AC-19: examined. Known open, pending #3618. The test asserts the renamed document. An uncommitted local re-tag exists in the worktree but is not at the reviewed head.
- FR-121-AC-35/36: examined. Tests exist but are untagged (FND-002).
- FR-865-AC-19: examined. Tagged, and now exercised through `compile_outcome`.
- FR-286-AC-40 (`unimported-library`): examined. Tagged, and now has a production producer.
- Reverse gap: none.
- Stubs: none.
- Semantic review: not run (not requested).
- Plan completion: not assessed

## Dispositions

Round 1, head 4973631532a165ed9224c593050d5963df9ccbe1. `quire matrix --scope . --format tsv` re-run.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Spec #3620 (bc77821e18) removed the blank line. The matrix now lists FR-027-AC-14..21 as `tagged`: AC-15 at compile.rs:565 and selection_text.rs:139, AC-16..21 at compile.rs:593/607/638/665/693/728, and AC-19 at compile.rs:665 is now TC-3210/FR-027-AC-19 |
| FND-002 | fixed | imported_function_site.rs:320 now carries `FR-121-AC-35` and :399 carries `FR-121-AC-36`. Both are `tagged` in the matrix |
