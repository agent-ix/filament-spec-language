---
id: SR-967175026302197768900474181941935084793
title: "base review of QSL-2935 replay request example (PR 3565)"
type: SpecReview
analysis: base
scope: "agent-ix/filament-spec-language; PR 3565 c120f742ded93f5f11af5874cb625bf71e072220; spec/functional/FR-1047-name-every-request-defect-and-return-a-request-example.md, spec/test-cases/TC-3170-request-refusals-teach-the-request-shape.md"
review_set: subset
---

## Summary

Ticket: QSL-2935. Spec-only PR, reviewed as `git diff origin/main...HEAD` at c120f742ded93f5f11af5874cb625bf71e072220 (FR-1047 Behavior replay-example bullets, FR-1047-AC-7, TC-3170 Expected Results for step 4). Code read on origin/main and on PR 3561's head (476da1f464), no cargo run. Read: fsl-command/src/replay_request.rs (read, decode), fsl-command/src/command/replay.rs (run_bytes), fsl-command/src/command/output.rs (stage mapping), fsl-replay/src/request.rs (ReplayRequest::decode_with, ByteProvision::admit), fsl-replay/src/request/package.rs (descriptor, clocks), fsl-replay/src/execute.rs (replay_with_cancel_and_temporal_selections, recompile_with_temporal_selections, canonical_sources), fsl-command/src/command/request_shape.rs and fsl-command/tests/it/request_diagnostics.rs on PR 3561, and FR-862 Behavior.

Base checklist: no new FR, AC or TC id is added; FR-1047-AC-7 stays backed by TC-3170 (Test). No hash, digest or pin is used. The new bullet is an EARS ubiquitous SHALL. The JSON member names and values match what decode reads: replay_target.members.function 'entry' (name splits on '.'), arguments Boolean, occurrence_key.ordinal the canonical unsigned decimal string '0', role text, over null, byte_provision rows read as bytes_path, authority and identity texts. The package graph and target are byte-identical to PR 3561's NODE_PACKAGE/REPLAY_TARGET apart from the lock source, and PR 3561's AC-7 test shows that request decodes and is constructed by FR-071 (it reaches execute.rs canonical_sources). ByteProvision::get matches a lock source on exact (authority, identity) equality, so ('example','entry') is covered.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The new bullet says the replay example's other request members are 'those the first bullet of this section gives', but the first bullet of 'The request example entry' is the signature bullet and gives no members. The values of profile_selections, dependencies, source, obligation_identity, backend, state_environment, accounting_limits, stage_limits, supplied_limits, object_environment and declared_domains are therefore left to the generic 'pass the value checks' rule, and FR-336 interiors are outside the scope rule. Fix: point at the scope-rule bullet ('the bullet above'), or list the eleven values in the same block (PR 3561 uses source {"assignments":[],"kind":"input"}, obligation_identity {"model":null,"node_id":{...},"ordinal":0}, backend "fsl", [] / {} / null for the rest). | spec/functional/FR-1047-name-every-request-defect-and-return-a-request-example.md:272-274 |
| FND-002 | low | The block fenced as `json` and called 'exactly the JSON below' is three member lines ("package": {...} on separate lines without braces or commas), which is not a JSON value. Fix: wrap it as one object {"package": ..., "replay_target": ..., "byte_provision": ...}, or call it 'these members'. | spec/functional/FR-1047-name-every-request-defect-and-return-a-request-example.md:276-280 |

## Verdict

Base checklist passes apart from two low wording defects. Mergeability rests on the integrity, failure-domain and evidence findings: the high finding in the integrity review (stage file versus FR-862's stage request) blocks the PR.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | At 2d22284bbb the bullet lists the other eleven members: profile_selections [], dependencies [], source {"assignments":[],"kind":"input"}, obligation_identity {"model":null,"node_id":{"ordinal":0,"package":"example","path":["entry"]},"ordinal":0}, backend "fsl", state_environment [], accounting_limits {}, stage_limits {}, supplied_limits null, object_environment null, declared_domains []. In MEMBERS order, each value is equal to PR 3561's REPLAY entries (request_shape.rs:319-343, NODE_OBLIGATION). The dangling 'first bullet' reference is gone. |
| FND-002 | fixed | The fenced block is now one object {"package": ..., "replay_target": ..., "byte_provision": ...}. It parses as JSON with exactly those three keys, and the bullet says 'exactly those of this JSON object'. |
