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

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-005 | high | FR-1047 cannot be satisfied for the replay example. Its Behavior requires every value in the example to pass its reader's checks before any file is read, and fixes the replay example's `byte_provision` at `[]`. FR-862 and FSpec FR-336 make every `byte_provision` row a `bytes_path` file. The replay reader requires a non-empty `package.lock.sources`, each source matched by a provision row. No replay example with `byte_provision` `[]` can be admitted. AC-7's "reads no provisioned file" does not say what refusal is expected. FR-1047-AC-7 and the `byte_provision` clause need amending: name one provision row (for example `program.fsl`) and expect FR-862's stage-request refusal naming that `bytes_path` in an empty directory, or state the refusal the example settles. | spec/functional/FR-1047-name-every-request-defect-and-return-a-request-example.md, fsl-replay/src/execute.rs:2092-2110, fsl-replay/src/execute.rs:2408 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | deferred | Spec PR #3563 (FR-286 details `missing` and `accepted`) is commissioned by the coordinator. |
| FND-002 | fixed | The AC-3 test removes `records` from the monitor example and asserts `missing required member /request/records` with `{"missing":["/request/records"]}` via `run_with_providers`. |
| FND-003 | fixed | Unit test `every_missing_member_is_named_at_once` asserts `missing members /profile_selections, /byte_provision at "/profile_selections"`. Its imports cause code-review FND-011. |
| FND-004 | deferred | Spec PR #3563 (FR-1047 AC-9, the export fallback refusal) is commissioned by the coordinator. The fallback now runs the compile reader first, and the `/request` "expected an object" refusal remains. |

## Dispositions (round 2)

| FND | outcome | reason |
| --- | --- | --- |
| FND-005 | deferred | Resolved by spec PR #3565 (head e9f4fb8c, not yet merged). It replaces `byte_provision` `[]` with one `entry.fsl` row and a matching lock source, and states the expected refusals. The amended text is satisfiable, and the code matches it. Merge #3565 before or together with #3561. |
