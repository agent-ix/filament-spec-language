---
id: SR-967175026302197768900474181941935084795
title: "evidence review of QSL-2935 replay request example (PR 3565)"
type: SpecReview
analysis: evidence
scope: "agent-ix/filament-spec-language; PR 3565 c120f742ded93f5f11af5874cb625bf71e072220; spec/functional/FR-1047-name-every-request-defect-and-return-a-request-example.md, spec/test-cases/TC-3170-request-refusals-teach-the-request-shape.md"
review_set: subset
---

## Summary

Ticket: QSL-2935. Spec-only PR, reviewed as `git diff origin/main...HEAD` at c120f742ded93f5f11af5874cb625bf71e072220 (FR-1047 Behavior replay-example bullets, FR-1047-AC-7, TC-3170 Expected Results for step 4). Code read on origin/main and on PR 3561's head (476da1f464), no cargo run. Read: fsl-command/src/replay_request.rs (read, decode), fsl-command/src/command/replay.rs (run_bytes), fsl-command/src/command/output.rs (stage mapping), fsl-replay/src/request.rs (ReplayRequest::decode_with, ByteProvision::admit), fsl-replay/src/request/package.rs (descriptor, clocks), fsl-replay/src/execute.rs (replay_with_cancel_and_temporal_selections, recompile_with_temporal_selections, canonical_sources), fsl-command/src/command/request_shape.rs and fsl-command/tests/it/request_diagnostics.rs on PR 3561, and FR-862 Behavior.

FR-1047-AC-7 keeps method Test (TC-3170), which suits an in-process call through run_with_providers. Implementation follow-up for PR 3561, not a spec defect: request_shape.rs NODE_PACKAGE needs the lock source and REPLAY's byte_provision Leaf needs the row. request_diagnostics.rs's replay branch, which asserts invalid_package 'the list is empty' and files == ['request.json'], must be rewritten to the amended AC.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | TC-3170 Expected Results for step 4 now asserts a second replay call 'with that file written beside request.json', but Procedure step 4 never writes entry.fsl or makes that call, and nothing states the file's contents. The expected result has no procedure step behind it, and the observed refusal depends on content the TC leaves open. The Expected Results also inherit the false stage file and 'having decoded in full' claims (integrity FND-001, failure-domain FND-001). Fix: add to Procedure step 4 'for the replay example, then write an empty entry.fsl beside request.json and call run_with_providers again', and state in Expected Results the stage request refusal naming entry.fsl and, with the file, a refusal past stage request that is neither invalid_package for /package/lock/sources nor missing_import. Rewrap the overlong line 61. | spec/test-cases/TC-3170-request-refusals-teach-the-request-shape.md:34-38; spec/test-cases/TC-3170-request-refusals-teach-the-request-shape.md:58-61 |

## Verdict

One medium finding: the TC procedure must perform and pin the entry.fsl-present call that the amended AC-7 asserts. Text only, no new TC.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | TC-3170 Procedure step 4 now calls the replay example's entry 'again with an empty entry.fsl written beside request.json', and Expected Results state both outcomes. The stage wording of the entry.fsl-present outcome is the subject of failure-domain FND-003. The 86-character line and the short reflowed line are not findings, because no line-length rule applies to spec Markdown here. |
