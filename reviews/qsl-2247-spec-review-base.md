---
id: SR-202611210022442313144108503
title: "Spec review (base) — FR-1030 AC-1/AC-5 edit and TC-2920 step 5 for QSL-2247"
type: SpecReview
analysis: base
scope: "agent-ix/filament-spec-language; spec/functional/FR-1030-request-a-refinement-transfer-from-a-native-run.md (AC-1, AC-5, Rationale); spec/test-cases/TC-2920-request-a-refinement-transfer-from-a-native-run.md (Description, step 5)"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-1030
    type: references
  - target: ix://agent-ix/filament-spec-language/TC-2920
    type: references
---

## Summary

Ticket: QSL-2247. Base checklist over the PR's spec edit: FR-1030-AC-1's fixture
formula change and the new FR-1030-AC-5, both citing arch ruling ef6f2f49
(2026-11-20), and TC-2920's added step 5. IDs, AC/TC linkage and the ruling
citation are in order; `quire validate` on both files exits 0. One criterion is
weaker than the ruling it encodes.

## Verdict

**CONDITIONAL** — one medium finding. Clean: FR-1030-AC-1's new PrevSafe formula
`always (holds(c.value = 1) implies once[1,1] holds(c.value = 0))` is the
ruling's own example and SoonTwo is kept as the ruling allows; TC-2920 scope is
updated to AC-1..AC-5 and step 5 covers AC-5; the Rationale cites the ruling and
states the semantics openly rather than adapting the spec silently; ID formats
valid; no hash, digest, pin or tracking-record antipattern.

## Findings

| ID      | Severity | Summary | Refs |
| ------- | -------- | ------- | ---- |
| FND-001 | medium | FR-1030-AC-5 does not pin what the ruling pins. Ruling ef6f2f49 item 2 says the PrevStutter row asserts `premise-unproved` "with the stutter witness"; the AC requires only `premise-unproved` naming slot 1, so an abstract premise that settled `failed`, `incomplete` or `inconclusive` (not the refuted terminal-stutter witness) satisfies it. State the slot-1 result (`refuted`) and the witness shape. | spec/functional/FR-1030-request-a-refinement-transfer-from-a-native-run.md:140 |

## Dispositions

Round 1 — head c410fe3190700371f7a5dc2c5da261e90c7ea938.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | FR-1030-AC-5 now requires `premise-unproved` naming slot 1 with `result` `refuted` and the slot's own record showing `refuted`. A refuted result needs a replayed terminal-stutter counterexample, which pins the witness the ruling names. |

## Dispositions (round 2)

Round 2: head d4aeb3439e7b741b0ba8ce1828f385e667d32ae2. No finding is open (FND-001 was fixed in round 1). The new FR-1030 Behavior bullet (FR-1030:133-141) states the current typed outcomes (`failed` for an uncertifiable TP-3 premise, `unsupported` `ClosureStutterLoop`) and cites the rulings. It is consistent with FR-1030-AC-5 and does not narrow AC-1, which stays open under ruling A. No new findings.

## Dispositions (round 3)

Round 3: head 9ae17da354abebc355818a6fd9ffd55c2d422da6. The mechanical commit touches no spec file. Nothing open; no new findings.

## Dispositions (round 4)

Round 4: head e859e681ce37b0b9192771411909073b0055aa9e. The commit touches only fsl-command/src/command.rs and export.rs (CR018). Nothing open in this artifact; no new findings.
