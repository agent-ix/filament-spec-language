---
id: SR-202611211854274
title: "EARS conformance of FSL PR3373 QSL-2739 changed requirement statements"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/filament-spec-language; PR3373 780a3d7b890510be4a4e8c83e0b585df46d1c42d; FR-1044 Behavior and Registered provider, FR-530 Description/Behavior, FR-507 family bullet"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-1044
    type: references
---

## Summary

Ticket: QSL-2739. This is an EARS grammar pass over the normative statements PR #3373 changed. Each FR-1044 Behavior bullet has a trigger or condition and one SHALL response:
- the selection
- the no-provider case
- the non-empty assumption set
- the cycle
- the guard (ubiquitous SHALL)
- the provider-supplied cause (ubiquitous SHALL NOT)
- the interface mismatch (event-driven)

FR-530's Description keeps one compound SHALL, and the rewritten clause stays inside the existing sentence. One normative rule sits only in Decisions, with no Behavior statement.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The `assumptions` admission rule ("FSL admits … only when … and otherwise refuses `invalid_runtime_input`/`invalid-value` at `/request/items/<i>/assumptions`") appears only in the Decisions prose and AC-11. No Behavior "If …, then FSL SHALL …" statement backs it, and no response (record, stage, exit) is given. AC-11 is therefore checked against rationale text. This goes away if integrity FND-001 is fixed by deleting the rule. If the rule is kept, add an unwanted-behaviour EARS bullet with its full response. | spec/functional/FR-1044-request-a-component-composition-item-through-analyze.md:101-106 |
| FND-002 | low | FR-530 Settlement opens with a 9-line sentence ("Before the slots, the item settles by … Interface mismatch: decided from … the first found settles …"). It packs the trigger, the ordering rule, the outcome and the exclusion into one clause chain with no SHALL. Fix: split it into "When a declaration holds an interface mismatch, the planner SHALL settle …" and a separate statement of the order. | spec/functional/FR-530-carry-and-check-liveness-and-fairness-premises-in-native-composition.md:145-155 |

## Verdict

**Minor changes.** Both findings are low. Clean: every changed FR-1044 Behavior bullet and the FR-507 family bullet.

## Scope and evidence

Spec-only. The statements were read in the committed text at 780a3d7b89.

- FR-1044 lines 212-332: examined.
- FR-530 lines 37-44, 66-81, 128-167: examined.
- FR-507 lines 105, 132-138: examined.

## Dispositions

Round 1 (final disposition), fix commits 8692503282 and 5a4b61b1925aa3365c1773fe3fb8945784f99af5 over 780a3d7b89, same base deccdc2a50; verified against the committed text at 5a4b61b192, not the author's report. quire validate exit 0 on FR-1044, FR-530, FR-507, TC-1330.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | no_change_needed | Moot: the admission rule was removed (integrity FND-001), so no Behavior statement is required. |
| FND-002 | fixed | FR-530 Settlement is split into 'the planner SHALL decide…' and 'The first mismatch found SHALL settle…', with the order and the not-a-mismatch rule as separate sentences. |
