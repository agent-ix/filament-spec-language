---
id: SR-2026111612172400000024
title: "Code review — filament-spec-language PR 2833 protocol decision facts"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; fsl-semantics/src/check/protocol_slots.rs, fsl-semantics/src/check/protocol_slots/tests.rs, spec/functional/FR-218-check-every-protocol-control-construct-at-s3.md, spec/test-cases/TC-1056-s3-checks-channels-relationships-requirements-and-choice-visibility.md"
review_set: subset
relationships:
  - { target: "ix://agent-ix/filament-spec-language/FR-218", type: references }
---

## Summary

Reviewed the frozen PR #2833 diff at `561fe84` against FR-218-AC-17, its existing TC-1056 cases, and the Rust implementation and tests. The new binder-field matching fixes the listed-field path, but it also treats every binder read nested in a composite `visible` entry as individually listed, contrary to FR-218.

## Verdict

**CONDITIONAL** — one medium finding: a guard can read a fact that appears only as an operand inside a composite `visible` entry.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Reads nested inside a composite `visible` entry are admitted as individually listed facts, contrary to FR-218-AC-17 and TC-1056 cases 31–32. | fsl-semantics/src/check/protocol_slots.rs:715-719 |

## Review notes

The new positive test traces to FR-218-AC-17 and checks a channel receiver's individually listed Boolean field. Its negative unlisted-field and overlapping-guard assertions are behavioral. The existing TC-1056 cases 31–32 specifically require a field reachable only through a composite entry to remain unlisted. This review was static; no tests or aggregate gates were run.

The failure is in constructing `listed` by traversing all binder reads of every `visible` expression. That set includes the operands of composite entries. The guard loop then accepts those reads at lines 735–736. A `visible` entry such as `gotA.ready and gotB.ready` therefore makes `gotA.ready` individually visible, although the contract says a composite exposes only its whole value.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | `visible` membership now comes only from `decision_fact(entry.expression.root())`, so composite roots do not add their operands. TC-1056 cases 31–32 are covered by new refusal tests. |
