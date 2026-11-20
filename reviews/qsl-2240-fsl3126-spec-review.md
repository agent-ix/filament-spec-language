---
id: SR-20261120013403002
title: "Spec review: QSL-2240 FR-237 task-trace wire reader"
type: SpecReview
analysis: base
scope: "agent-ix/filament-spec-language; PR #3126 head 6349fa10a0e6858108913153ae3451fad58e1786; spec/functional/FR-237-replay-a-timed-counterexample.md, spec/test-cases/TC-704-task-automata-lower-to-timed-subjects.md"
review_set: base
relationships:
  - { target: "ix://agent-ix/filament-spec-language/FR-237", type: references }
  - { target: "ix://agent-ix/filament-spec-language/TC-704", type: references }
---

## Summary

Ticket: QSL-2240. Base spec review of FR-237's new "Reading the wire",
"Document form", "Typed forms" and "Task-step equality" rules, FR-237-AC-10 to
AC-12 and TC-704 steps 16 to 18, against the Linear arch ruling on QSL-2240,
FSpec FR-364 as changed by PR #763, FSpec FR-331's schema
(`TemporalCounterexample`, `ModelStep`, `TaskTransitionIdentity`), FSL FR-249
AC-8/AC-9 and FR-237-AC-1. Integrity and consistency checks were folded into
this base pass; no separate analysis lens was run.

## Verdict

**PASS (mergeable).** The reader, the two typed forms, the document form, the
`TraceAlphabet` refusal and the alphabet-from-item rule match the ruling, and
the FSL ACs agree with FSpec FR-364-AC-23/AC-24 (pointers, refusal codes,
`Tick` task-name order). The `malformed_wire` causes match the schema's
`additionalProperties`, enum, `budget_delta` if/else and task-step `delay`
constraints. AC-10 to AC-12 are new ids with no collision; TC-704's scope
and steps cover all three. `quire validate` passes. No high findings.

## Findings

| ID      | Severity | Summary | Refs |
| ------- | -------- | ------- | ---- |
| FND-001 | medium | AC-12's `Tick` fixture gives the two `release` declarations but not the task set `Two`'s task parameters, policy, or the refuted claim and continuation, so the counterexample that "replays" is not determined. | spec/functional/FR-237-replay-a-timed-counterexample.md:223 |
| FND-002 | medium | "Analyze SHALL replay each EN-6 task counterexample through this form ... and not through a product self-replay" has no acceptance criterion that fails if analyze keeps the self-replay; AC-10 and TC-704 step 16 replay the written JSON from the test, not from analyze. | spec/functional/FR-237-replay-a-timed-counterexample.md:144-147 |
| FND-003 | low | `terminal-stutter` is called a member of the FR-331 `TransitionIdentity` union; in the schema it is the second arm of `ModelStep.transition`, outside `TransitionIdentity`. | spec/functional/FR-237-replay-a-timed-counterexample.md:52-55 |
| FND-004 | low | The reader both refuses shape faults (`malformed_wire`) and returns alphabet refusals (`invalid-value`); which wins when a payload has an alphabet fault at an early step and a shape fault at a later one is not stated. | spec/functional/FR-237-replay-a-timed-counterexample.md:131-136 |
| FND-005 | low | New text cites FR-286 and FR-217 with no frontmatter relationship to either. | spec/functional/FR-237-replay-a-timed-counterexample.md:1-31 |
| FND-006 | low | TC-704's scope lists FSpec FR-364-AC-21, AC-23, AC-24, then AC-22, out of order. | spec/test-cases/TC-704-task-automata-lower-to-timed-subjects.md:31-32 |
