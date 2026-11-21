---
id: SR-202611210022442313144108499
title: "Code review of filament-spec-language PR 3255 (QSL-2657 check dependent_bytes charges only dependent documents)"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; git diff origin/main...be60a060ddc8cebe5c6e1d9b21e4a5611414d7b8 (fsl-command/src/command.rs, fsl-command/tests/it/check_imports.rs)"
review_set: subset
evaluated_revision: "be60a060ddc8cebe5c6e1d9b21e4a5611414d7b8"
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-866
    type: reviews
---

## Summary

Ticket: QSL-2657. Code review with the rust-review lane, diff only. The
review pass ran on another machine and posted its findings to the ticket; this
file is reconstructed from that comment and carries the round 1 dispositions.

## Verdict

Round 1: the identity bind is back before the read in both intake readers,
the check source is read under the fixed 8 MiB intake ceiling (the same
TOTAL_BYTES constant compile's fixed intake uses, capped by take(limit+1), so
not unbounded) and s1.input_bytes now refuses at S1 through the parse. One
part of FND-005 remains and one new low finding was added.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Identity bind moved after file read in shared intake; TC-450 (FR-100-AC-3) now fails with Io NotFound | fsl-command/src/command.rs:1593-1636 |
| FND-002 | medium | Over-s1 check source refuses via RunCause::Source (last_stage null), not compile's S1 limit document; untested | fsl-command/src/command.rs:336-340 |
| FND-003 | low | source_unmetered duplicates read_file's capped read; name says what it is not | fsl-command/src/command.rs:1602-1626 |
| FND-004 | low | Stale doc comments on check_outcome and CheckInputs::dependent_bytes | fsl-command/src/command.rs:225-226, 275-276 |
| FND-005 | low | Test padding branch is dead; redundant assert_ne | fsl-command/tests/it/check_imports.rs:148-157, 188 |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-006 | low | Check source read ceiling is the fixed TOTAL_BYTES (8 MiB), not tied to s1.input_bytes: with s1.input_bytes raised above 8 MiB a larger source refuses at read with the generic file-bytes limit (last_stage null), so FR-866's "read under s1.input_bytes" cannot be honoured past 8 MiB and check has no way to raise it | fsl-command/src/command.rs:1649-1650 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Intake::source and source_outside_budget both call selected.identity.bind before any read and pass the identity into admit_source (command.rs:1635, 1648) |
| FND-002 | fixed | source_outside_budget reads under TOTAL_BYTES, the parse enforces s1.input_bytes; check_source_over_s1_input_bytes_refuses_at_s1_like_compile asserts incomplete, S1, one stage_limit_exceeded naming s1.input_bytes, and success at the limit. Asserts fields only, not equality with a compile document |
| FND-003 | fixed | Renamed source_outside_budget and uses the shared read_file |
| FND-004 | fixed | Both doc comments rewritten |
| FND-005 | still-open | Padding now always runs and the redundant assert_ne is gone, but the AC's unpadded FR-866-AC-2 program at dependent_bytes N is no longer run; only the padded source is checked at N |
