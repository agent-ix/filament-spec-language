---
id: SR-3595003
title: "spec-review of PR 3595 FR-895 enabledness and incremental assessment text"
type: SpecReview
analysis: base
scope: "agent-ix/filament-spec-language; spec/functional/FR-895, FR-896, spec/test-cases/TC-2531 at c548c7922f72dbe30ec10f6b769bf588a7cc5860"
review_set: subset
---

## Summary

Ticket: QSL-2885. Review of the new FR-895 Enabledness row, "Incremental assessment" paragraph,
AC-9 to AC-21, FR-896 `assessment` text and AC-5, and TC-2531 scope. `quire validate` passes on all three
files (quire 0.52.0 local, 0.52.1 titan).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The repeat rule names "its last completed iteration, the iteration with the most occurrences of any of the body's steps": that is the last begun iteration, which must be complete, not a completed one; it is silent on which `choice` case each iteration took and on the `exhausted` path, so the text cannot settle SR-3595001 FND-006/FND-007 | spec/functional/FR-895-assess-a-recorded-protocol-execution-against-its-obligations.md:70 |
| FND-002 | high | "a verdict changes only from `pending` to a decision, or from `satisfied` to `violated`, never from `violated`" does not hold for array-position prefixes: a later observation that starts a further iteration (or supplies an earlier occurrence) turns a `satisfied` enabledness back to `pending` (SR-3595001 FND-008, reproduced). The author must either require `pending` for such open-prefix verdicts or drop the claim | spec/functional/FR-895-assess-a-recorded-protocol-execution-against-its-obligations.md:126-135 |
| FND-003 | medium | FR-896 does not say what an incremental item gives when one prefix of an admitted recording is refused (for example an `on each` population whose instance appears later in the array) | spec/functional/FR-896-decide-a-global-conformance-claim-over-the-protocol-system-or-a-recording.md:35-41 |
| FND-004 | low | The Enabledness cell holds about ten normative rules in one table cell, not atomic and with no SHALL; AC rows are out of order (AC-11 after AC-21); FR-896-AC-5 "each with `observations` 1 to 6" means the counts 1 to 6 | spec/functional/FR-895-assess-a-recorded-protocol-execution-against-its-obligations.md:70 |

## Verdict

The rulings on any-role predecessors, witness order and the incremental surface are written down and
testable. The repeat rule and the monotonicity claim are not right as written (FND-001, FND-002), and prefix
refusal is unspecified (FND-003). Proposed repeat text: "A step after a `repeat` has as predecessors the steps
before the `repeat`, every step the last iteration the recording begins takes (for each `choice` in it, the case
taken in that iteration), and, when the `exhausted` path ran, its steps; an earlier iteration's steps are not
predecessors. An `exhausted` step has as predecessors the steps of the last iteration."

## Dispositions (fix round 1)

| Finding | Disposition |
| --- | --- |
| FND-001 | Fixed. FR-895 rule 6 uses the proposed wording and rule 7 states the `exhausted` step's predecessors. |
| FND-002 | Fixed by arch ruling: prefixes are in causal order, so "never from `violated`" and "satisfied never to pending" hold for enabledness; FR-895-AC-26 states it. |
| FND-003 | Fixed. FR-895 "Incremental assessment" gives the whole item an `invalid_runtime_input` refusal naming the `prefix` index; prefixes close over begun instances (FR-896-AC-6). |
| FND-004 | Fixed. The Enabledness cell points to ten numbered rules each with one requirement; AC-11 is positioned after AC-10; FR-896-AC-5 says counts 1 to 6. |
