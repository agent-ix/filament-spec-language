---
id: SR-599013
title: "Gap analysis — QSL-2937 PR #3576 FSL result texts"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; git diff origin/main...eebfca09d9; FR-865 AC-9..24, FR-866 AC-8..10, FR-286 AC-29/38/39/40, FR-810-AC-6, FR-1047-AC-9; TC-3113, TC-3197..3203"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-865
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-286
    type: references
---

## Summary

Ticket: QSL-2937. This is a planless gap analysis of PR #3576 at head eebfca09d9, scoped to the criteria the merged spec PR #3563 added or changed.

Matrix source: `quire matrix --scope . --format json` timed out after 300 s with no output (exit 124). The host was at load 21 to 47, and the brief limited heavy work, so the run was not retried. This audit therefore falls back to a grep index of `"<criterion-id>"` `#[trace]` tags over the `.rs` files on the branch. That is a substitute for the computed matrix, not the matrix: ignored-test and stale-evidence states are not assessed.

Tag index on the branch (files carrying each criterion id as a `#[trace]` tag):

- FR-865: AC-9 to AC-11 1 each; AC-12 to AC-15 2 each; AC-16 0 (a driver test in filament-driver, out of this repo, and nothing claims it); AC-17 2; AC-18 to AC-22 1 each; AC-23 4; AC-24 8.
- FR-866: AC-8 to AC-10 1 each.
- FR-286: AC-29 1; AC-38 2; AC-39 2; AC-40 11.
- FR-810-AC-6 1; FR-1047-AC-9 1.

Reverse gap: every new production item has an owning requirement:

- FR-865: `stage_word`, `packaged_result_lines`.
- FR-286: `MissingSelection`, `with_selection`, the selection-keyed `HintRow`s, `with_packaged`, `PackagedOutcome`.
- FR-866: `reject_unselected_check_models`.
- FR-1047-AC-9: `require_object_request`, `with_checked_request`.

The exception is `OutcomeDiagnostic::from_call_site_refusal`, whose only caller is a test (see the code-review artifact SR-599012, FND-001).

There are no stubs or placeholder returns. Tests assert exact values with `assert_eq!` throughout.

## Verdict

**CONDITIONAL.** No in-repo criterion is untagged and there is no high finding.

- FND-001 (medium): FR-286-AC-29 was rewritten by #3563, but its new branch is not asserted. The rewritten text says a second row of one `selection` of the missing-selection pair refuses `DuplicatePair`, and TC-3113 step 3 requires that run. The PR changed exactly that duplicate check, yet `command_hint_catalog_covers_typed_pairs_and_rejects_bad_rows` only gained `selection: None` fields.
- FND-002 (low): the text halves of FR-865-AC-17 (compile request) and AC-18 to AC-23 are covered only by composition. FR-865-AC-23 is also tagged on a catalog-only test.

Judged consistent, not findings:

- TC-3203's absolute `output` paths. The tests put the request in a temporary `reqs/` directory, so `output` is `<tmp>/reqs/spine.package.json` and the `--output` override is the absolute path as given. That is FR-286 "Compile results"' join rule applied to an absolute request path. The AC's relative spelling would need a change of working directory inside a test. The empty-parent "no join" branch is not exercised, but no AC names it.
- TC-3202's loop over five non-object values, a superset of the TC's one.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-286-AC-29's rewritten branch is untested: a second `HintRow` for one `selection` of the missing-selection pair must give `Err(DuplicatePair)` (TC-3113 step 3). The PR changed the duplicate check to key on `selection`, but the catalog test adds no such case | fsl-command/src/lib.rs:76-130; fsl-replay/src/outcome/hint.rs:345-353 |
| FND-002 | low | The text halves of FR-865-AC-17 (compile request with `models` `[]`) and AC-18 to AC-23 are asserted only by composition: producer tests check the JSON `hint`, and one fsl-inspect test renders synthetic diagnostics of each kind. The compile test could render, as `check_imports.rs` does, but does not. `each_selection_kind_has_its_own_hint_row` carries FR-865-AC-23 although it exercises no producer and no rendering | fsl-command/tests/it/cli_contract/compile.rs:435-449; fsl-replay/src/outcome.rs:5758-5760; fsl-inspect/src/render.rs:3318-3346 |

FND-001: add a case to the TC-3113 test. Take `HINTS` plus a second `HintRow { code: MissingImport, cause: Some("missing-selection"), selection: Some("model"), .. }` and assert `Err(HintCatalogError::DuplicatePair { .. })`. `check_hint_catalog(HINTS, ..)` returning `Ok` already covers "the nine selection rows together are not duplicates".

FND-002: render `outcome.document()` in `compile_without_the_selected_model_carries_the_model_selection` and assert the `  hint:` line, and drop FR-865-AC-23 from the catalog-only test. fsl-replay cannot depend on fsl-inspect, so for AC-18 to AC-23 composition is the practical route.

## Coverage

- FR-865-AC-9, AC-10, AC-11 (TC-3197): examined; tagged; clean.
- FR-865-AC-12 to AC-15 (TC-3198): examined; tagged (fsl-inspect unit and fsl-command end-to-end); clean.
- FR-865-AC-16: examined; a driver test, correctly untagged here.
- FR-865-AC-17 to AC-24 (TC-3199): examined; tagged; FND-002.
- FR-866-AC-8, AC-9, AC-10 (TC-3200): examined; tagged; exact messages; clean.
- FR-286-AC-29 (TC-3113): examined; tagged; FND-001.
- FR-286-AC-38 (TC-3201): examined; tagged; the run-entry documents validate against the schema; clean.
- FR-286-AC-39 and FR-810-AC-6 (TC-3203): examined; tagged; exact `result` objects, schema-validated, files exist; clean.
- FR-286-AC-40 (TC-3203): examined; tagged; nine kinds and the defensive producer covered; see SR-599012 FND-001 for `unimported-library`.
- FR-1047-AC-9 (TC-3202): examined; tagged; clean.
- TC-2455, TC-2963, TC-2996 updates: examined; header strings updated to stage words; no old `at S<n>` header remains anywhere in the repo.
- Semantic review: not run (not requested).
- Plan completion: not assessed
