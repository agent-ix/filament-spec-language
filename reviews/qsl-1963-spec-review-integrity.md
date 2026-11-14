---
id: SR-202611141459004
title: "Integrity review of the QSL-1963 FR-495 criteria and TC-1419, TC-1420"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language; PR #2707 at 3b0bc910d964198a45d79d8f7c6f4489b93fc179, diff origin/main...HEAD: FR-495 (Kernel bullet, AC-10..13, AC-3 unchanged), spec/test-cases/TC-1419-kernel-mode-drops-a-function-body-s-presence-obligation-and-nothing-else.md, spec/test-cases/TC-1420-kernel-mode-still-refuses-a-malformed-rational-shape-and-dispatch-declaration.md"
review_set: subset
relationships:
  - { target: ix://agent-ix/quire-spec-language/FR-495, type: references }
  - { target: ix://agent-ix/quire-spec-language/TC-1419, type: references }
  - { target: ix://agent-ix/quire-spec-language/TC-1420, type: references }
---

## Summary

Ticket: QSL-1963. Structure and consistency checks:
- `quire validate --scope .` passes on FR-495, TC-1419 and TC-1420.
- TC-1419 and TC-1420 are new ids. Neither is on origin/main, and no other open PR in
  agent-ix/quire-spec-language adds or cites them, cites FR-495-AC-10..13, or edits FR-495.
- Each new AC row's verification cell names its TC (AC-10..12 name TC-1419, AC-13 names
  TC-1420). Each TC `verifies` FR-495 and lists the ACs it covers, with a step and an expected
  result for every fixture the AC names.
- FR-495-AC-3 is unchanged by the diff and agrees with the new bullet: Linked refuses the
  integer nonzero obligation, which QSL-1962 tracks as unimplemented, and Kernel drops it.
- The cross-references FR-103-AC-1, FR-104-AC-1 (`ParentOrder`), FR-104 "Definedness facts",
  QSpec FR-146 and FR-091-AC-30 all exist and say what the new text cites them for.
  FR-091-AC-30 names the FR-146 `missing-measure` obligation for a recursive predicate.

## Verdict

PASS with one low finding. TC-1420 inherits the AC-13 constructibility defect
(SR-202611141459001 FND-001), which is not repeated here.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | TC-1419's title and file name say Kernel "drops a function body's presence obligation and nothing else". FR-495 states that Kernel also drops a function body's range, nonzero, rational-range and nonempty-reduction obligations, so the title contradicts the requirement it verifies. Its scope is presence against state clauses and measures. | spec/test-cases/TC-1419-kernel-mode-drops-a-function-body-s-presence-obligation-and-nothing-else.md:3 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | f66c0b693: TC-1419 retitled and renamed to `...-presence-obligation-and-keeps-state-clauses-and-measures.md`. Its description now lists every obligation Kernel drops. Outside this review's own SR files, no file in the tree names the old filename or title. |
