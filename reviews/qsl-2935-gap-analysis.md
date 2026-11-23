---
id: SR-967175026302197768900474181941935084790
title: "gap-analysis of QSL-2935 request diagnostics and request_example (PR 3561)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; QSL-2935; PR3561 head 827911de23e3872e2cfe8eac18058ccfa99436f6; git diff origin/main...HEAD (base 1167ea0bbf); FR-1047, TC-3170, FR-286, FR-862, FR-644; fsl-command/src/command/request_shape.rs; fsl-command/src/replay_request.rs; fsl-command/src/command/export.rs; fsl-replay/schemas/filament-outcome-1.schema.json; fsl-command/tests/it/request_diagnostics.rs"
review_set: subset
---

## Summary

Ticket: QSL-2935, PR filament-spec-language#3561, head 827911de23, base 1167ea0bbf. A planless gap-analysis scoped to FR-1047 and the PR diff.

`quire matrix --scope . --format json` was stopped by a 150 s cap (rc 124, no output). The matrix below is therefore a grep index of `#[trace(...)]` tags, declared as such.

## Verdict

CONDITIONAL. All eight FR-1047 criteria are tagged, and no criterion is untagged. The findings are medium and low: one spec inconsistency that needs an FR-286 amendment, and untested or unowned paths. The high defect in the evidence behind FR-1047-AC-7 (the replay example refused by its own entry) is recorded in the code-review artifact SR-967175026302197768900474181941935084789 FND-001, not duplicated here.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-286 "Diagnostic details" is a closed vocabulary ("no other member is written"; "Every other diagnostic ... SHALL carry details null"; `path` means a pointer inside an argument value or to the request's `value`). It has no `missing` or `accepted` member, and no `path` meaning for `/format`. FR-1047 (merged in #3548) requires `{"missing":[...]}` and `{"path":"/format","accepted":[...]}` and did not amend FR-286. The schema extension in `fsl-replay/schemas` is legitimate, because FSL owns `filament-outcome/1` (FSpec FR-520: "FSL FR-286 publishes the filament-outcome/1 schema"), and it is additive and compatible. FR-286's table still needs a spec amendment (two new members and the widened `path` meaning). | spec/functional/FR-286-serialize-every-outcome-as-one-json-outcome-document.md:452-497, fsl-replay/schemas/filament-outcome-1.schema.json:52 |
| FND-002 | low | TC-3170 step 5 (`run_with_providers` over the monitor example with `records` absent, so that `/request/records` is the one missing entry) has no test. AC-3 covers only the empty monitor request. | fsl-command/tests/it/request_diagnostics.rs:133-159 |
| FND-003 | low | The new aggregate message in `replay_request::closed_members` (`missing members /a, /b`, with the path naming only the first) has no test. `admit` now pre-empts it on `command::run`, so it is reachable only through direct `replay_request::read` callers. | fsl-command/src/replay_request.rs:124-134 |
| FND-004 | low | Reverse gap: the export fallback refusal `ClauseRequest { member: "/request", reason: "expected an object" }`, which replaced the circular text, is owned by no requirement and has no test. | fsl-command/src/command/export.rs:157-162 |

## Coverage

- Plan completion: not assessed
- Matrix: grep index (quire matrix stalled past 150 s). FR-1047-AC-1 to AC-8 are each tagged once by `#[trace("TC-3170", "FR-1047-AC-n")]` in fsl-command/tests/it/request_diagnostics.rs. Tag counts are 2 per AC (doc comment plus attribute) and 3 for AC-7.
- Stubs and coverage inflation: no source stubs. The AC-7 test asserts weakly (code-review FND-005).
- Semantic review: no separate fan-out was run. The intent, test and code checks were done in the code-review pass (SR-967175026302197768900474181941935084789).
