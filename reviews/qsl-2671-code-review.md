---
id: SR-202611210437430
title: "Code review — QSL-2671 PR #3258 blank monitor records are refused"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; PR #3258 diff at 4832a2993bb6843a6e9122c33a6fdaf6f2f67a9b: fsl-command/src/command/monitor.rs, fsl-command/tests/it/cli_contract/monitor.rs"
review_set: subset
evaluated_revision: "4832a2993bb6843a6e9122c33a6fdaf6f2f67a9b"
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-644
    type: references
---

## Summary

Ticket: QSL-2671 (parent QSL-2473). PR #3258 at head `4832a299` removes the whitespace-only skip in `stream()` (fsl-command/src/command/monitor.rs:791), so a blank or whitespace-only line goes through `read_record`, fails `serde_json::from_slice` and gives a `refusal` / `invalid-request` event with member `record` at its physical (enumerate) index; `read` and `refused` both count it. `file_lines()` (monitor.rs:921-937) now strips one final `\n` before splitting and returns an empty stream for an empty file. Two tests are added in fsl-command/tests/it/cli_contract/monitor.rs:170-235, tagged FR-644-AC-2. The rust-review lane (rust-style baseline, test conventions and tags, panic surface, integer arithmetic) is folded into this file.

Checked and clean: physical record index is unchanged (the `enumerate` wraps the iterator before the removed skip, so blank lines already advanced it); `position = observed.checked_sub(1)` and `observed` updates are untouched; `read += 1` now runs for blanks, matching "the number of records read"; no new panic or integer conversion. Final newline, CRLF and blank-only streams: the stream path splits with `BufRead::split(b'\n')`, which yields no empty tail, so `a\n` is one record, `a\r\n` is `a\r` (serde_json accepts trailing `\r`), `\r\n` alone and `\n\n` are refused blank records; `file_lines` now gives the same split for a batch file, which is what FR-644 Inputs requires ("a file of records gives the events a live stream of the same records gives"). An empty file and an absent stream are both empty traces. The `Some(path) else` branch of `file_lines` is unreachable from `drive` (a batch run with no `records` is refused earlier at monitor.rs:607-613), so "absent" only arises on the stream path.

Tests: filtered `cli_contract::monitor` run at head, 9 passed. Mutation in a throwaway worktree (whitespace skip re-added at monitor.rs:791): both new tests fail and the 7 existing tests pass, so the new tests can fail on the regression they target.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The `file_lines()` final-newline change has no test: every batch-form test writes `records.join("\n")` with no trailing newline and ends with `{"end":true}`, so reverting `strip_suffix` (a batch file ending in `\n` with no `end` gets a spurious refusal at its last index, breaking batch-equals-stream) passes every test | fsl-command/src/command/monitor.rs:926-936 |
| FND-002 | low | `blank_and_whitespace_lines_are_refused_and_absence_is_not` claims more than it asserts: its doc says the blanks leave the session as the stream without them, but it checks only refused indices and count; and its "absence" case is an empty stdin stream (`b""`), not an absent records source | fsl-command/tests/it/cli_contract/monitor.rs:208-235 |

## Verdict

CONDITIONAL — the code change is correct and the two new tests are real (mutation-verified). FND-001 should get one batch-form test (a records file ending in `\n` with no `{"end":true}`, and one with a trailing blank line, compared with the same stream) in this PR; FND-002 is a test-wording/assertion tightening. Driver-side blank-line splitting is out of scope here.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | fixed | - |

Round 1 at head `e86c995a98` (fix commit e86c995a98, tests only). FND-001: `a_records_file_gives_the_events_of_the_same_stream` (fsl-command/tests/it/cli_contract/monitor.rs:351-371) runs a records file ending in a newline with no `end` record, and one with a trailing blank line, through `command::run` and compares refused count, events, records and clauses with the same bytes as a stream. Reviewer mutation (strip_suffix reverted in file_lines) fails it at monitor.rs:366; the other 9 pass. FND-002: the blank/whitespace test now compares the good records' events and clauses with the stream without the two blanks (indices after 2 shifted by 2), and its doc names the `b""` case an empty stdin stream, not an absent source. Filtered `cli_contract::monitor` at e86c995a98: 10 passed.
