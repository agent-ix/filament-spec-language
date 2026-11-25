---
id: SR-2026112508191403
title: "failure-domain review of PR 3876 snapshot tuple and union components spelling"
type: SpecReview
analysis: failure-domain
scope: "agent-ix/filament-spec-language; FR-106 tuple and payload-bearing union paragraphs; FR-230 clock union cases (context); PR 3876 head 46864ca27adae0679c423bc8f07c89f86377e13e"
review_set: subset
---

## Summary

Ticket: QSL-3263. Unstated failure modes in the new refusal rules for tuple and payload-bearing union values. Each single-defect case has a cause; multi-defect precedence and Clock-typed union components are not covered.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | No precedence for a tuple or union payload with several defects: FR-360 item 9 orders the nullary form (missing, extra, wrong type, undeclared case) and FR-106-AC-7 promises first-defect-in-order, but the new paragraphs give no order, so e.g. `{"tuple":{"type":"<Other>","components":[r]}}` can be wrong-value-kind or invalid-value, and `{"variant":{"type":"<Holder>","case":"Has","x":1}}` unknown-member or missing-member, depending on the implementer | spec/functional/FR-106-admit-snapshots-and-invocations.md:676-680, spec/functional/FR-106-admit-snapshots-and-invocations.md:694-700 |
| FND-002 | medium | "a member that declares components" does not say whether a Clock-typed case payload (FR-230-AC-63 `Phase { Armed(Clock), Idle }`, a clock place, no data) counts; the current reader admits `Armed` in the nullary form, and read literally the new text makes that form `missing-member` and demands a Clock value the grammar has no data form for | spec/functional/FR-106-admit-snapshots-and-invocations.md:690-696, spec/functional/FR-230-check-time-declarations-clocks-and-clock-constraints.md:1304 |

## Verdict

AC-84's controls are all single-defect values, so neither finding blocks its testability. Both should be settled before the QSL-3263 reader lands; FND-001 is one sentence (extend FR-360 item 9's order with the components checks after case resolution).

## Scope

- FR-106 tuple paragraph: examined.
- FR-106 union paragraph: examined.
- FR-106-AC-84: examined.
- FR-360 item 9 order; FR-230-AC-63: context_only.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-003 | low | The new eight-step defect order has no place for a payload member with the wrong JSON kind: a non-string `type` or `case`, a `fields` that is not an object, or a `components` that is not an array. For example, `{"tuple":{"type":"<Pair>","components":{}}}` could be refused wrong-value-kind or invalid-value (step 7). The collection rule (FR-106:618) and today's reader both give wrong-value-kind | spec/functional/FR-106-admit-snapshots-and-invocations.md:717-735 |

## Dispositions

Round 1, fix head b0345aee9f5ba7a25271c61478596a3f8620e313.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | FR-106:717-735 adds an eight-step first-defect order for record, tuple and union that extends FR-360 item 9 |
| FND-002 | fixed | FR-106:687-693 excludes clock entries from `components` and arity; FR-106:703-705 makes an all-clock member such as Armed use the nullary form |
