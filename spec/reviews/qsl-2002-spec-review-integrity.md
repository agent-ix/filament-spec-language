---
id: SR-202611160951002
title: "Integrity review of the QSL-2002 exact mod interval contract"
type: SpecReview
analysis: integrity
scope: "agent-ix/filament-spec-language; PR #2745 at f3fcd3fc18f1f9277432fef2f59ce4f929669f09, diff origin/main...HEAD: FR-495, FR-062, FR-093, TC-2270"
review_set: subset
relationships:
  - { target: ix://agent-ix/filament-spec-language/FR-495, type: references }
  - { target: ix://agent-ix/filament-spec-language/FR-062, type: references }
  - { target: ix://agent-ix/filament-spec-language/FR-093, type: references }
---

## Summary

Ticket: QSL-2002. This lens checks cross-requirement consistency for the diff.

The following hold:

- FR-062's result bound ("its checked node's type") agrees with FR-075. FR-075 carries the bound as the type node's `WireNodeId`, and gives `None` context when the bound is the application's own result type.
- FR-092 builds a type node for every checked `ValueType`, and equal ids denote one node. So the graph holds the `Int[0, 11]` node, and in `equal` the narrow's `type_argument` node is that same node.
- FR-098 "Application claim" takes the bound from the record. Its defensive out-of-bound arm stays unreachable, because the interval is exact.
- FR-404-AC-2's `Verdicts`/`replay-parity` settlement matches AC-22.
- The W4 smallest-interval and no-fallback text agrees with "the type is the same in every position".

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-093 contradicts the new bounded-position rule. The `Coerce` row applies only to "an integer admitted into an `Int[..]` that does not contain its type", and line 668 says an integer "whose type an `Int[..]` contains is admitted with no node". A `mod` node typed `Int[0, 11]` into `Int[0, 11]` or `Int[0, 20]` is contained, so FR-093 builds no `Coerce`. FR-495 and FR-495-AC-21 (`equal`, `wider`, `arg`) require exactly one narrow. The added sentence ("a `Coerce` of its node ... always builds the narrow") assumes a `Coerce` that the earlier sentence says does not exist. Fix: amend the `Coerce` row condition and the line-668 sentence to except an integer `div`, `rem` or `mod` application. Such an application is admitted into any `Int[..]` by a `Coerce` that lowers to a narrow. | FR-093:550,668-672; FR-495:225-231 |
| FND-002 | low | "The range obligation of that `Coerce` is discharged by the node's interval" is stated without a condition. It holds only when the interval lies inside the target under `CheckMode::Linked`. Outside the target, Linked refuses `unproved-range` ("Range of an integer division"), and `CheckMode::Kernel` records no range obligation for `div`, `rem` or `mod` at all. Fix: add "when the interval lies inside the target; otherwise as 'Range of an integer division' and the Kernel bullet state". | FR-495:229-231,248-256 |

## Verdict

Round 1: REQUEST CHANGES on FND-001. Round 2 (head 4e76e7e2): APPROVE; both findings are fixed.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Round 2 (head 4e76e7e2): FR-093's `Coerce` row and line 668 now make an exception for an operand whose lowered type is `Integer` (a division application, or a read of a `let` binder bound to one). Its `Coerce` into any `Int[..]`, a containing one included, lowers to exactly one narrow. This agrees with FR-495 and AC-21. The `letb` binder-read narrow "holds no record", which agrees with FR-062: a narrow whose operand is a binder read yields no claim, so the application's record has no direct narrow, takes its own interval and has no context, and the KeyFault rule does not apply. |
| FND-002 | fixed | Round 2: "Range obligation of that `Coerce`" is now conditional. Under Linked the obligation is discharged inside the target and refused `unproved-range` at the leaf otherwise (at the operator for a direct `Coerce`), which agrees with AC-19. Kernel records no obligation and still builds the narrow. |
