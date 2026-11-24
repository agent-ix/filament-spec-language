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

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-005 | medium | Rule 6 picks "the last iteration the recording begins ... the one with the most occurrences of any of the body's steps" without reference to the step, so an iteration begun concurrently with a step after the `repeat` reopens that step (SR-3595001 FND-014). Rule 9 keeps "observed but not yet ordered before the step" `pending`, but under causal-order prefixes and an append-only recording such a pair can never become ordered. Proposed: "the last iteration begun causally before the step"; a later iteration begun concurrently is a violation of that iteration's own steps; an observed predecessor not ordered before the step is `violated`, cause `unexpected`, at once | spec/functional/FR-895-assess-a-recorded-protocol-execution-against-its-obligations.md:146-160 |
| FND-006 | low | "Each rule below is one requirement" but rules 4, 5 and 6 each hold two or three (for example rule 4's join selection and the unordered branches); FR-896-AC-6's last sentence states the absence of a test, not a criterion | spec/functional/FR-895-assess-a-recorded-protocol-execution-against-its-obligations.md:126-150 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | rules 5 to 7 state the per-iteration case, the last begun iteration and the exhausted path as an alternative exit (ADR-027 FO-1); residual relativity defect is FND-005 |
| FND-002 | still-open | the prose still claims every obligation changes only from pending to a decision or from satisfied to violated; reproduced enabledness satisfied to pending (SR-3595001 FND-014) and the coder's channel-premise satisfied to pending (FND-015) contradict it; the author must scope the claim or require pending |
| FND-003 | fixed | Incremental assessment paragraph: prefixes hold the population of begun instances; a refused prefix refuses the item invalid_runtime_input with the prefix index |
| FND-004 | fixed | Enabledness cell replaced by ten numbered SHALL rules; AC-11 moved into order; FR-896-AC-5 says "counts 1 to 6" |

## New findings (disposition pass 2)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-007 | medium | Rule 5 ("an earlier `parallel` under any other join SHALL supply the steps of the branches whose steps were all observed") is not relative to the step: under `outstanding continue` (ADR-027 ST-7) a branch may complete after the join, and rule 5 with rule 18 then marks a legitimate recording `violated` (SR-3595001 FND-017). Proposed: "the branches whose steps were all observed causally before the step; the step's predecessors are satisfied when those branches number at least what the join policy requires" | spec/functional/FR-895-assess-a-recorded-protocol-execution-against-its-obligations.md:140-141 |

## Dispositions (round 2)

| FND | outcome | reason |
| --- | --- | --- |
| FND-002 | fixed | channel premise pending while not closed (Pending, never passed); rules 16-18 make observed-but-unordered predecessors violated at once; AC-26 and AC-27 check every obligation across prefixes |
| FND-005 | fixed | rules 10 and 12: last iteration begun causally before the step; a concurrently begun iteration is judged on its own steps; rule 17 at once |
| FND-006 | fixed | 18 numbered rules, one SHALL each; FR-896-AC-6 now states the refused-prefix criterion |

## New findings (disposition pass 3)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-008 | medium | Rule 5 gives `predicate` joins the threshold one, and no rule covers `outstanding cancel`: a step of a cancelled branch after the join is never an obligation failure (SR-3595001 FND-019, FND-020). Proposed: a `predicate` join is reached when its function is true over the dispositions of the branches completed causally before the step; under `outstanding cancel` an observation of a cancelled branch causally after the join step is `violated`, cause `unexpected` | spec/functional/FR-895-assess-a-recorded-protocol-execution-against-its-obligations.md:140-145 |

## Dispositions (round 3)

| FND | outcome | reason |
| --- | --- | --- |
| FND-007 | fixed | rule 5 now counts branches observed causally before the step up to the policy threshold, excludes branches finishing after the join under outstanding continue, and needs every branch until reached; AC-28 |
