---
id: SR-15208995
title: "gap analysis of PR 2742 clocks override entries against FR-640"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; PR 2742 head de5d2fee68ac93089d115c514c3f6bdd940233b4; FR-640 Behavior and FR-640-AC-1..6; fsl-command/tests/it/request_overrides.rs, tests/it/clock_selection.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-640
    type: references
---

## Summary

Ticket: QSL-2294. This is a planless audit using `quire matrix --scope .`. FR-640-AC-1 to AC-9 are tagged by non-ignored tests. AC-10 and AC-11 are tagged only by ignored tests, which is pre-existing and not changed by this PR. The new tests tag `TC-1660`/`TC-1661` with a bare `FR-640`, and no FR-640 acceptance criterion states the behaviour they verify.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | FR-640 "Behavior" states the both-given refusal and says `--clocks` members are "read exactly as a request's `clocks` member", but no FR-640 AC or test case states either rule. The four new tests therefore carry only the FR-level tag `FR-640` and borrow `TC-1660`/`TC-1661`, whose procedures (three-profile compile and run, emit refusals) are not what these tests do. The matrix records nothing for the override behaviour, and the TC tags overstate what TC-1660 and TC-1661 cover. Fix: add an FR-640 AC and TC for the override parity and the both-given refusal (stage `request`, `invalid-request`, exit 20, message naming `clocks`), then retag the four tests to it. Under the AGENTS.md 2026-09-09 owner update this can go to the assurance campaign instead of blocking the PR. | fsl-command/tests/it/request_overrides.rs:69,107,152,197 |

## Verdict

PASS, with one low traceability finding. The production code has an owning requirement (FR-640 "Behavior", the both-given and `--clocks` bullets), and the new tests assert it with real values, not stubs. The tests check byte equality of override and member compiles, refusal-envelope equality on the run entries, stage, code, exit and message for the both-given refusal on four entries, and unknown-kind and repeated-name override refusals.

Run-AC evidence: the run test compares only refusals (`unused-clock-selection` at emit). That is enough for this PR. The override replaces `request.clocks` before `prepare_run_bytes` branches, so a member and an override reach every later step as the same value. Refusal equality at emit shows the override reaches the run path's selection check, and byte equality on compile shows it gives the same package. The positive run in FR-640-AC-2 is owned by the path-only entry tests (`tests/it/clock_selection.rs:465`, non-ignored, plus the ignored `run_wire.rs:243`), which this PR does not change. A clock-bearing successful run through `run_with_overrides` adds nothing the shared request value does not already guarantee. It can be added when QSL-2133/QSL-2254 land, but it is not required here.

## Coverage

FR-640-AC-1 to AC-6 examined (AC-2 for run evidence), plus the FR-640 Behavior bullets on both-given and `--clocks`. FR-640-AC-7 to AC-11 were read for context only. Reverse gap: none. Every new public function and `ClocksGivenTwice` trace to FR-640. Semantic review was limited to the targeted checks above.

Plan completion: not assessed
