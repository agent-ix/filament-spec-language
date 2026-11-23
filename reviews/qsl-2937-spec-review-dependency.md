---
id: SR-967175026302197768900474181941935084791
title: "Dependency review of QSL-2937 result texts (PR 3563)"
type: SpecReview
analysis: dependency
scope: "agent-ix/filament-spec-language; PR 3563 head cab8261ac57853eb726bf906c5ba6a42808c7b9c; spec/functional/FR-865-render-every-outcome-document-as-text-the-cli-default.md, spec/functional/FR-866-check-a-unit-against-the-models-and-libraries-it-imports.md, spec/functional/FR-286-serialize-every-outcome-as-one-json-outcome-document.md, spec/functional/FR-1047-name-every-request-defect-and-return-a-request-example.md, spec/test-cases/TC-3197-text-form-names-the-stage-in-words.md, spec/test-cases/TC-3198-compile-success-text-says-what-was-produced-and-where.md, spec/test-cases/TC-3199-missing-import-hint-is-specific-to-its-cause-kind.md, spec/test-cases/TC-3200-unselected-model-message-names-the-selected-identities.md, spec/test-cases/TC-3201-outcome-details-admit-missing-and-accepted.md, spec/test-cases/TC-3202-export-request-that-is-not-an-object-is-refused-by-name.md"
review_set: subset
---

## Summary

Ticket: QSL-2937. Spec-only PR #3563, reviewed as `git diff origin/main...HEAD` at cab8261ac57853eb726bf906c5ba6a42808c7b9c (merge base e2b72b5f67). quire validate over the ten touched files exits 0 (EARS warnings only, all on pre-existing lines). No cargo was run; current-behaviour claims were checked by reading origin/main source. Checked the new relationship edges and the cross-references the new ACs make.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | FR-865 adds FR-027, FR-810 and FR-866 as `references`, but it puts a new obligation on FR-027's `CompileOutcome`: a `render_text` method, plus an output path that the struct does not hold today (fsl-command/src/command.rs:216-222). The Description even states that the path is held as a present fact. That is a `depends_on`, and FR-027 should carry the obligation. The ACs also cite producers in FR-098, FR-862, FR-1044 and FR-108 with no edge, and the Dependencies prose (FR-865:269-280) is not updated. | spec/functional/FR-865-render-every-outcome-document-as-text-the-cli-default.md:17-25,143-146,269-280 |

## Verdict

One low finding. The FR-286-AC-38 and FR-1047-AC-9 behaviour is delivered by the open implementation PR #3561 (QSL-2935, head 827911de23: the outcome schema gains `missing`/`accepted`, and export.rs refuses `/request: expected an object`). That PR's tests tag FR-1047-AC-1..8 only, so there is no AC id collision. Land this spec PR before or with #3561. Mergeable once fixed.

## Scope and evidence

Review set is the subset the dispatch named (base, ears, integrity, evidence, failure-domain, dependency, scope-boundary).

- FR-865#compile-success (spec/functional/FR-865-render-every-outcome-document-as-text-the-cli-default.md, examined): ### The text of a compile success Today the text of a successful `compile` is the one line `package: success (exit 0) at S4`, which does not say that a package was produced or where it went. The `package` outcome document carries no package size and no path (FR-810 keeps `artifacts` empty), so the text of a successful compile is written from the typed `CompileOutcome` (FR-027), which holds the package bytes and, when the request carried `output` (FR-810) or the caller gave `RequestOverrides::output`, the path as the caller wrote it. `CompileOutcome::render_text` returns it, and the driver w...
- FR-865-AC-16 (spec/functional/FR-865-render-every-outcome-document-as-text-the-cli-default.md, examined): The text of FR-862's refusal of a package reference with no matching byte-provision entry holds the line ` hint: Add a byte_provision entry for the named source to the replay request.`.
- FR-865-AC-17 (spec/functional/FR-865-render-every-outcome-document-as-text-the-cli-default.md, examined): The text of a replay of a `function` request naming a `function_owner` the unit does not import holds the line ` hint: Name a function_owner that the program imports directly, or add the import.`.
- FR-865-AC-18 (spec/functional/FR-865-render-every-outcome-document-as-text-the-cli-default.md, examined): The text of FR-1044's refusal of a component package that is no dependency of the request's package holds the line ` hint: Name a component package that the request's package lists as a dependency.`.
- FR-286-AC-38 (spec/functional/FR-286-serialize-every-outcome-as-one-json-outcome-document.md, examined): The outcome document of FR-1047-AC-1 serializes with one diagnostic whose `details` is exactly `{"missing":["/request/models",["/request/program","/request/package"],["/request/call","/request/clause","/request/items"]]}`; the document of FR-1047-AC-5's `native-analyze/1` run refusal serializes with `details` exactly `{"accepted":["native-run/1","native-monitor/1","native-replay/1"],"path":"/format"}`; every other member of both documents is as FR-286 states, and a document whose `details` carries neither member is byte-equal to the bytes written before this row.
- FR-1047-AC-9 (spec/functional/FR-1047-name-every-request-defect-and-return-a-request-example.md, examined): `export_with_limits_and_overrides` with `ExportSelection::Ordinary` over `{"format":"native-run/1","request":3}` refuses `invalid-request`, stage `request`, exit 20, with message exactly `invalid clause request member /request: expected an object`; this requirement adds no `missing` list for it, since a `request` that is not an object has no members to list.
