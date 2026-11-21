---
id: SR-202611210022442313144108500
title: "integrity analysis of the FR-488 alias amendment in PR 2957"
type: SpecReview
analysis: integrity
scope: "agent-ix/filament-spec-language; PR #2957 head ed3f278e4e3ece4f49f6520019d5cc3f14c18d68; commit ed3f278e4e edit of FR-488 'Clauses S4 does not emit' and FR-488-AC-18, against FR-631-AC-4 and every spec restating that list"
review_set: subset
---

## Summary

Ticket: QSL-2239 (PR record). This pass covers only consistency and testability for the arch ruling on code-review FND-005 (FR-631-AC-4 wins). The FR-488 edit removes refusal item 2 and renumbers items 3-5 to 2-4. It states the alias case as emitted, with the ruling cited, and rewrites AC-18. `quire validate` passes on FR-488 and FR-362. The amendment is internally consistent and testable. FR-631-AC-4 and FR-488-AC-2/AC-28 (the `model_root` subject) agree with it. ADR-033 and FR-361-AC-4 refer to the list only by name and stay correct. One restatement was not updated.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-362 Behavior item 8 still lists "has an alias that binds no object parameter" among the cases where a `hyper` clause is refused `NotEmitted` (FR-488 "Clauses S4 does not emit"). That contradicts the amended FR-488 and FR-631-AC-4. A reader implementing FR-362 would restore the removed refusal. Fix: drop that clause from FR-362 item 8, citing the same ruling | spec/functional/FR-362-produce-the-checked-graph-and-the-type-report-from-one-source.md:85-90; spec/functional/FR-488-emit-a-checked-hyper-clause-as-a-v2-hyperproperty-node.md:427-442 |

## Scope and evidence

- Examined: FR-488 "Clauses S4 does not emit" (items 1-4 and the new alias paragraph), FR-488-AC-18, FR-631-AC-4, and FR-362 item 8. Context only: FR-488-AC-2, AC-28, ADR-033 line 237, FR-361-AC-4.
- A `grep` over `spec/` for "binds no object parameter" and "S4 does not emit" found FR-362 as the only stale restatement. FR-488's remaining "Remaining work: QSL-1094" notes (lines 162, 455) concern fairness members and reads, not the alias.
- Atomicity: the new AC-18 holds three assertions (timed refused, alias emits, first refusal reported), as the old one did. That is acceptable for a precedence AC.

## Verdict

The amendment is sound and follows the ruling. One medium cross-spec contradiction remains in FR-362.

## Dispositions

Round 2 (this artifact's first disposition), at frozen head `db0f80895f16a0cc6c0926b99321180ee1c571fb`.

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | 307050eff1: FR-362 item 8 drops the alias case from the `NotEmitted` list and states, citing the ruling, that a clause whose alias binds no object parameter is emitted (FR-631-AC-4 ruling, as FR-488 states). `quire validate` passes on FR-362 |

## Round 3 note

Round 3 (head `aed94771b48007ac1269258193ded2e7bef90b8b`) changes no spec. FND-001 stays fixed.
