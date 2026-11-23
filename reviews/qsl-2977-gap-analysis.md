---
id: SR-967175026302197768900474181941935084800
title: "gap-analysis of PR3591 against FR-662, FSpec FR-563, FR-866 and FR-865"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; PR3591 8cd9bd289a2aa573c1fffc5c7fafcfdcb5012ae2 diff over origin/QSL-2805-work; FR-662-AC-1, FR-662-AC-2, FSpec FR-563-AC-1, FSpec FR-563-AC-2, FR-866-AC-3, FR-865-AC-17"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-662
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-866
    type: references
---

## Summary

Ticket: QSL-2977 (with QSL-2976). This is a planless audit scoped to the PR diff. The computed matrix (`quire matrix --scope .`) shows FR-662-AC-1/AC-2 and FR-866-AC-3 tagged. With this PR their root CLI-level binders are no longer ignored, and all four un-ignored tests pass on the head. Root `it` request_surface_s1:: and check_imports:: on the head: 33 passed, 1 failed. The failure is `request_surface_s1::snapshot_and_model_header_refusals_round_trip_document_loci` (tests/it/request_surface_s1.rs:373, left `/populations/0/objects/1/fields/parent`, right `/populations/0/objects/1`). It fails identically on origin/main d4fac184, so it is pre-existing and not a regression of this PR. #3585 (b6af201954, QSL-2899) made a missing-member refusal name the absent member's pointer (FR-106-AC-82) and left the expectation of this root test's `missing-parent` case at the object pointer. QSL-2899 is Done, and no open QSL ticket names the test.

## Verdict

CONDITIONAL. No untagged criterion in scope and no high finding. Two medium spec gaps remain: FSpec FR-563 `--format text` is not served on the argv path (tracked by QSL-2981), and the FSpec FR-563-AC-1 CLI assertions do not check line and column. Plan completion: not assessed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FSpec FR-563 Inputs give `--format json\|text`, but the argv `check` refuses `--format text` with `invalid-request` `format: check supports only json`. A FR-563 operand form is unserved. The coder filed QSL-2981 (Backlog). | fsl-command/src/command.rs:2319-2324 |
| FND-002 | medium | FSpec FR-563-AC-1 and FR-662-AC-1 require the second item to give the Markdown line and column of the offending token, and the first item success with no diagnostic. `check_markdown_names_each_fence_unit` asserts only substrings (`doc#Healthy`, `ill_typed`, `doc#Broken`, `Broken`, `spec.md`), the item count and exit 20. It does not check the line and column, the body span, the per-item category or the empty diagnostics of `Healthy`, so a CLI output that dropped the original region would pass. Library-level fsl-replay tests cover the region (`refused_fence_keeps_body_span_and_original_coordinates`), but the command-layer AC binder is weak. The assertions were deliberately left unchanged by the ticket. | tests/it/request_surface_s1.rs:1144-1176 |
| FND-003 | low | `check_markdown_refuses_doubled_and_headless_fences` asserts `missing_declaration` but not the `missing-name` cause, not the fence's first line FR-662-AC-2 names, and not that the other fences' results are unchanged. | tests/it/request_surface_s1.rs:1182-1206 |

## Coverage

- FR-662-AC-1 (spec/functional/FR-662-check-every-native-fence-of-a-markdown-specification.md, examined): two items in document order, `Healthy` success with body identity `doc#Healthy`, `Broken` `ill_typed` naming `{document, path: spec.md, clause: Broken, body: doc#Broken}`, operation `check`, category refusal, exit 20. Implemented through `check_markdown_document` → `check_native_fences` → `from_native_fences`. The binder passes; see FND-002.
- FR-662-AC-2 (examined): `extraction-clause-count` `{"actual": 2}`, headless fence `missing_declaration`/`missing-name`, determinism and cancel. The CLI binder passes. Determinism and cancel are covered at library level by `duplicate_and_headless_fences_refuse_independently_and_cancel_returns_no_report`.
- FSpec FR-563-AC-1 (examined): same as FR-662-AC-1 through `check --markdown agent-ix doc spec.md --format json`.
- FSpec FR-563-AC-2 (examined): `check` without `--markdown` reads a Markdown file as native source and refuses `invalid_syntax` exit 20. The labels in `check a d spec.md` are ignored and the file is read as native source. `--markdown` over a document with no native fence gives zero items, success, exit 0 (asserted).
- FSpec FR-563 operand grammar (examined): `check --markdown <authority> <identity> <file>` requires the labels, matching the spec form, and the CLI never decides Markdown from the file name. The non-markdown labelled form keeps its pre-PR behaviour of ignoring the labels.
- FR-866-AC-3 (spec/functional/FR-866-check-a-unit-against-the-models-and-libraries-it-imports.md, examined): the compile request with `models` `[]` and `check_outcome` with no model each refuse `missing_import`/`missing-selection` at I1 with a message naming `example/config-version`, `--model` and `models`. The root and fsl-command binders pass.
- FR-866 Refusals "the same message is given by a compile request that omits the model" (examined): satisfied, since both paths now carry the admit detail verbatim and no path appends to it.
- FR-865-AC-17 (spec/functional/FR-865-render-every-outcome-document-as-text-the-cli-default.md, examined): the hint and `selection` `model` are unchanged. The hint is composed from the refusal's selection, not by the removed helper. `missing_model_refusal_carries_its_selection_hint_and_text` was run.
- FR-865-AC-3 (context_only): the text default is a driver test in filament-driver. The argv path writes JSON only.
- FR-866-AC-2, FR-866-AC-5 (context_only): untagged in the computed matrix. AC-5 is a driver-repo test, and neither is touched by this PR.
- Reverse gap: `OutcomeDocument::from_native_fence_failure` is owned by FR-662 Outputs ("A StageFailure covers the document as a whole") but has no test (see the code-review SR FND-003).
- Semantic review: skipped (not opted in). Plan completion: not assessed.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | medium | Spec contradiction, introduced by #3585 (QSL-2899) and not by this PR: FR-663-AC-3 states that the missing-`parent` refusal carries "the pointer to `child`", and FR-267-AC-25 defers to FR-663 for that pointer. FR-106-AC-82 states the pointer of the absent member (`.../fields/NAME`). The fix round's update of `snapshot_and_model_header_refusals_round_trip_document_loci` to `/populations/0/objects/1/fields/parent` follows FR-106-AC-82 and the code. It is correct, but the test still carries the FR-267-AC-25 trace, whose referenced FR-663-AC-3 text says the opposite. FR-663-AC-3 needs amending to the member pointer. | spec/functional/FR-663-*.md (FR-663-AC-3); tests/it/request_surface_s1.rs:1605-1615 |
| FND-005 | low | FR-662 (and FSpec FR-563) define the body identity only for a fence a heading owns (`<identity>#<clause id>`). The identity given to a fence no heading owns is unspecified: the implementation gives `doc#unowned-N`. The new headless test rightly asserts `clause` null and no body identity, but the unit name of such an item is unpinned by any requirement. | spec/functional/FR-662-check-every-native-fence-of-a-markdown-specification.md |
| FND-006 | low | FR-662 Outputs say only that "A StageFailure covers the document as a whole". They do not state the `check` document a whole-document failure projects to: category, `last_stage` (null for extraction refusal, cancel and fault; the limit's stage for a limit), empty `items`, one diagnostic. The new native_fence_failure tests pin the implementation's choices with no requirement behind them. | spec/functional/FR-662-check-every-native-fence-of-a-markdown-specification.md; fsl-replay/tests/it/native_fence_failure.rs |

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | deferred | QSL-2981 (Backlog) owns `--format text` on the argv path; `--format text` stays refused and is pinned by a refusal case. |
| FND-002 | fixed | e761dccd: `check_markdown_items_carry_exact_units_spans_and_markdown_coordinates` asserts category refusal and exit 20; Healthy success with diagnostics `[]` and the full unit; Broken `ill_typed` with the full unit, body-span start and end, and an original region `spec.md` line 16 columns 41-49 with byte offsets. |
| FND-003 | fixed | e761dccd: `check_markdown_headless_fence_refuses_missing_name_at_its_first_line` asserts `missing_declaration`/`missing-name`, the original region on line 4 (the fence's first line), clause null, and the owned fence's result unchanged (success, `[]`, `doc#Good`). |
