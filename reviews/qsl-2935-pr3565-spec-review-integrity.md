---
id: SR-967175026302197768900474181941935084794
title: "integrity review of QSL-2935 replay request example (PR 3565)"
type: SpecReview
analysis: integrity
scope: "agent-ix/filament-spec-language; PR 3565 c120f742ded93f5f11af5874cb625bf71e072220; spec/functional/FR-1047-name-every-request-defect-and-return-a-request-example.md, spec/test-cases/TC-3170-request-refusals-teach-the-request-shape.md"
review_set: subset
---

## Summary

Ticket: QSL-2935. Spec-only PR, reviewed as `git diff origin/main...HEAD` at c120f742ded93f5f11af5874cb625bf71e072220 (FR-1047 Behavior replay-example bullets, FR-1047-AC-7, TC-3170 Expected Results for step 4). Code read on origin/main and on PR 3561's head (476da1f464), no cargo run. Read: fsl-command/src/replay_request.rs (read, decode), fsl-command/src/command/replay.rs (run_bytes), fsl-command/src/command/output.rs (stage mapping), fsl-replay/src/request.rs (ReplayRequest::decode_with, ByteProvision::admit), fsl-replay/src/request/package.rs (descriptor, clocks), fsl-replay/src/execute.rs (replay_with_cancel_and_temporal_selections, recompile_with_temporal_selections, canonical_sources), fsl-command/src/command/request_shape.rs and fsl-command/tests/it/request_diagnostics.rs on PR 3561, and FR-862 Behavior.

Checked the new text against FR-862 and against the code. Also checked collisions: no new ids. PR 3563 inserts a section before 'The request example entry' and appends FR-1047-AC-9 (TC-3202) after AC-8. git merge-tree of this head with PR 3563 and with PR 3561 is clean both ways, so a rebase either way is mechanical. FR-1047's existing depends_on FR-862 edge covers the replay reader.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The PR says the replay example, over an empty directory, 'is refused at stage file naming entry.fsl', in the Behavior bullet, in FR-1047-AC-7 and in TC-3170. FR-862 Behavior already says 'A byte_provision file that is missing or exceeds the ceilings is refused at stage request naming its bytes_path', and the code does that on main and on PR 3561: replay.rs run_bytes maps the provide failure to RunCause::ReplayBytes, and output.rs maps RunCause::ReplayBytes to Stage::Request with Details::None. The code is the inner read cause's (io-error for a missing file) and the message is 'replay byte_provision file entry.fsl: ...'. So FR-1047 now contradicts FR-862 and AC-7 cannot pass. The other examples' precedent does not carry over: their files are read by the run Intake at stage file, while FR-862 puts the replay provision read at stage request. Fix (text only): in the Behavior bullet, AC-7 and TC-3170 replace 'at stage file naming entry.fsl' with 'at stage request, exit 20, with the missing file's read refusal naming entry.fsl (FR-862)', and change 'as the other examples name a file' to 'as the other examples are refused only for a file they name'. | spec/functional/FR-1047-name-every-request-defect-and-return-a-request-example.md:284-288; spec/functional/FR-1047-name-every-request-defect-and-return-a-request-example.md:313; spec/test-cases/TC-3170-request-refusals-teach-the-request-shape.md:58-61; spec/functional/FR-862-read-a-replay-request-file-and-replay-it.md:86-87; fsl-command/src/command/output.rs:431-433 (PR 3561), 425-427 (main) |

## Verdict

Not mergeable: one high finding. The stage claim contradicts FR-862 and the code, and AC-7 as written fails. The fix is text only and needs no new id. Rebasing over PR 3561 or PR 3563 is conflict-free.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | The Behavior bullet, FR-1047-AC-7 and TC-3170 Expected Results now say the empty-directory replay example 'is refused at stage request, exit 20, with the missing file's read refusal naming entry.fsl (FR-862)'. That matches FR-862 Behavior and output.rs (RunCause::ReplayBytes to Stage::Request), and io-error exits 20 (FR-1039-AC-9). |
