---
id: SR-202611141554004
title: "Integrity review of the QSL-1982 FR-495 criteria and TC-1421"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language; PR #2708 at 402ca8da65aef4aaafa016888cd1ac451d8e7a77, diff origin/main...HEAD: FR-495 (Kernel bullet lead sentence, AC-14..16; AC-3 as context), spec/test-cases/TC-1421-kernel-mode-drops-the-rational-range-rational-divisor-and-nonempty-reduction-obligations.md"
review_set: subset
relationships:
  - { target: ix://agent-ix/quire-spec-language/FR-495, type: references }
  - { target: ix://agent-ix/quire-spec-language/TC-1421, type: references }
---

## Summary

Ticket: QSL-1982. Structure and consistency checks:
- `quire validate --scope .` passes on FR-495 and TC-1421.
- TC-1421 is a new id. It is not on origin/main, and no other open PR in
  agent-ix/quire-spec-language adds or cites it, adds another TC-142x file, cites
  FR-495-AC-14..16, or edits FR-495.
- AC-14..16 each name TC-1421 in their verification cell. TC-1421 `verifies` FR-495, names
  the three ACs in its scope, and has a step and an expected result (with the `Obligation`
  cause) for every fixture, including the AC-16 `[1, 2]` control.
- The lead-sentence list agrees with TC-1419's description and with boundary four. The
  malformed rational shape stays refused under Kernel (AC-13), and that is a fault, not one
  of the dropped obligations.

## Verdict

PASS with one low finding.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | AC-15 ends "so a zero divisor is met by the evaluator as `Undefined::DivisionByZero` (AC-3)". AC-3's Kernel half covers integer `div`/`mod`, decimal `/` and quantity `/` bodies, with no rational `/` row, so the citation points to a criterion that does not own this case. No TC-1421 step evaluates anything, so this evaluation outcome is stated in AC-15 but no test of AC-15 checks it. Cite the "Zero divisor, two observations" bullet instead, or drop the evaluation clause. | spec/functional/FR-495-reach-the-integer-division-zero-divisor-and-state-core-rows-from-qsl-source.md:252 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | fa9f478c1: AC-15 now cites the "Zero divisor, two observations" bullet, whose Kernel half names "a decimal or rational `/`" evaluating to `Undefined::DivisionByZero` (FR-495 line 146). The bullet, not AC-15, now owns the evaluation outcome, so TC-1421 checking only the Linked/Kernel outcomes is consistent. |
