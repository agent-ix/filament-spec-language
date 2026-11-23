---
id: SR-967175026302197768900474181941935084796
title: "failure-domain review of QSL-2935 replay request example (PR 3565)"
type: SpecReview
analysis: failure-domain
scope: "agent-ix/filament-spec-language; PR 3565 c120f742ded93f5f11af5874cb625bf71e072220; spec/functional/FR-1047-name-every-request-defect-and-return-a-request-example.md, spec/test-cases/TC-3170-request-refusals-teach-the-request-shape.md"
review_set: subset
---

## Summary

Ticket: QSL-2935. Spec-only PR, reviewed as `git diff origin/main...HEAD` at c120f742ded93f5f11af5874cb625bf71e072220 (FR-1047 Behavior replay-example bullets, FR-1047-AC-7, TC-3170 Expected Results for step 4). Code read on origin/main and on PR 3561's head (476da1f464), no cargo run. Read: fsl-command/src/replay_request.rs (read, decode), fsl-command/src/command/replay.rs (run_bytes), fsl-command/src/command/output.rs (stage mapping), fsl-replay/src/request.rs (ReplayRequest::decode_with, ByteProvision::admit), fsl-replay/src/request/package.rs (descriptor, clocks), fsl-replay/src/execute.rs (replay_with_cancel_and_temporal_selections, recompile_with_temporal_selections, canonical_sources), fsl-command/src/command/request_shape.rs and fsl-command/tests/it/request_diagnostics.rs on PR 3561, and FR-862 Behavior.

Order traced. replay_request::read first parses the whole package with ReplayPackage::read (syntax only), then calls decode. decode reads the graph, replay_target, source, stage_limits, accounting_limits, supplied_limits and the obligation ordinal, then calls provide(bytes_path) for each byte_provision row. Only after that does it read dependencies, state_environment, profile_selections, the obligation node_id lookup, object_environment and declared_domains. fsl_replay::replay then runs FR-071's ReplayRequest::decode_with (target rule, profiles, backend, stage limits, lock descriptor, ByteProvision::admit, which is where missing_import IncompleteByteProvision comes from). Then come declared_function_target, request_limits and recompile_with_temporal_selections. In the recompile, canonical_sources refuses an empty lock 'the list is empty' (invalid_package); next come the clocks, run_spine_units over the provisioned bytes, check_recorded_modes and finally ReplayPackage::admit (read_checked_package).

With entry.fsl present, the example passes FR-862 decode and FR-071 construction, both checks the PR names, and the lock source check. The next refusal is FR-098's. For an empty or non-compiling file it is a recompile refusal at stage spine. For a file that compiles it is the mode check or package admission (invalid_package), because the example package is not a complete filament.checked-package/v2 document. The example can never replay. Under the arch ruling's second branch (the spec says what the example consists of) that is acceptable only if the spec says so.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-1047-AC-7 (and TC-3170) says the replay example over an empty directory is refused 'having decoded in full'. That is false. decode calls provide(bytes_path) partway through, so dependencies, state_environment, profile_selections, the obligation_identity node lookup, object_environment and declared_domains are not yet decoded, and FR-071's construction never runs. A test cannot observe 'decoded in full' in that call. FR-1047's own rule (values pass the checks applied before a file is read) is met, because the members decoded before the read do pass. Fix: drop 'having decoded in full' from the empty-directory clause and attach it to the entry.fsl-present clause (see FND-002). | spec/functional/FR-1047-name-every-request-defect-and-return-a-request-example.md:313; fsl-command/src/replay_request.rs:383-404 (PR 3561 decode: provide call at 389, later members from 404) |
| FND-002 | medium | The clause 'with entry.fsl present beside it is refused neither for an empty /package/lock/sources nor as missing_import' is true for a file without imports, but entry.fsl's contents are unstated: an entry.fsl that imports an absent library would make the recompile refuse missing_import, so the AC is not deterministic. The claim is also weaker than what holds and than the ruling asks for. It never says the example decodes and is constructed, and it never says the example still cannot replay. Exact minimal fix for AC-7 and the Behavior bullet: 'with an empty entry.fsl beside it, the example is not refused at stage request (FR-862 decodes it in full and FR-071 constructs it); it is refused by FR-098's recompile or package admission, never as invalid_package for /package/lock/sources and never as missing_import. The example does not replay: its package is no complete checked package.' | spec/functional/FR-1047-name-every-request-defect-and-return-a-request-example.md:313; spec/functional/FR-1047-name-every-request-defect-and-return-a-request-example.md:282-288; fsl-replay/src/execute.rs:2074-2236 (PR 3561) |

## Verdict

Two medium findings. With entry.fsl absent the example is refused at the provision read, before the members decoded after it. With an empty entry.fsl present it gets past both refusals the PR names, then FR-098 refuses it in the recompile or package admission. Mergeable once the AC states the file contents and the true stronger claim. Both fixes are text only.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-003 | high | The entry.fsl-present clause now says the example 'is not refused at stage request' and 'is refused by FR-098's recompile or package admission' (Behavior bullet, FR-1047-AC-7, TC-3170 Expected Results). That wording came from this review's own suggested fix, and it is wrong. output.rs maps a RunCause::Replay refusal to its spine stage only for ReplayRefusal::Recompile / RecompileWithProtocolResults; every other FR-098 refusal, including package admission (PackageWithIdentity) and check_recorded_modes, is reported at Stage::Request. So whenever the empty-file recompile succeeds, the refusal is at stage request and AC-7 fails. Whether an empty unit recompiles is not settled by reading. Fix (text only): in all three places replace 'is not refused at stage request' with 'is not refused invalid-request and not with a read refusal naming entry.fsl'. In the Behavior bullet keep '(FR-862 decodes it in full and FR-071 constructs it)'. In TC-3170 replace 'does not refuse at stage request' the same way. | spec/functional/FR-1047-name-every-request-defect-and-return-a-request-example.md:291-297; spec/functional/FR-1047-name-every-request-defect-and-return-a-request-example.md:321; spec/test-cases/TC-3170-request-refusals-teach-the-request-shape.md:60-64; fsl-command/src/command/output.rs:435-442 (PR 3561) |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | 'having decoded in full' is gone from AC-7 and TC-3170's empty-directory clause. The Behavior bullet now puts 'FR-862 decodes it in full and FR-071 constructs it' only on the case with an empty entry.fsl present, which the code bears out (decode reaches every member once provide succeeds, and ReplayRequest::decode_with admits the one-source provision). |
| FND-002 | fixed | The file contents are now stated as an empty entry.fsl, which has no imports, so the missing_import claim is deterministic. The clause names FR-098's recompile or package admission and says 'The example does not replay: its package is no complete checked package'. The stage wording added with this fix is the new FND-003. |
