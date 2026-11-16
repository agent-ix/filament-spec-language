---
id: SR-202611160951001
title: "Spec review of the QSL-2002 exact mod interval contract"
type: SpecReview
analysis: base
scope: "agent-ix/filament-spec-language; PR #2745 at f3fcd3fc18f1f9277432fef2f59ce4f929669f09, diff origin/main...HEAD: FR-495, FR-062, FR-093, TC-2270"
review_set: subset
relationships:
  - { target: ix://agent-ix/filament-spec-language/FR-495, type: references }
  - { target: ix://agent-ix/filament-spec-language/FR-062, type: references }
  - { target: ix://agent-ix/filament-spec-language/FR-093, type: references }
  - { target: ix://agent-ix/filament-spec-language/TC-2270, type: references }
---

## Summary

Ticket: QSL-2002. This is the base spec-review lens over the diff only. The integrity, EARS and criterion-strength lenses are in SR-202611160951002, SR-202611160951003 and SR-202611160951004.

Checked against: FR-092 (type nodes, one node per structural id), FR-098 "Application claim" (the record supplies the bound), FR-404-AC-2 (the replay-parity settlement), FR-075 (the request item carries the bound's type node and the direct conversion context), the FR-495 smallest-interval and work-ceiling text (W4), FR-495-AC-18 to AC-20, and FR-767 (QSL-1989).

`PATH= quire validate --scope . 'spec/**/*.md'` exits 0, with no warning on a changed file.

## Method

- Read the four-row ruling (direct, derived, Integer consumer, replay) and mapped each row to FR-495 "Representation of an integer division result", FR-062 "Result bound and path condition" and FR-495-AC-21/22.
- No feature is narrowed. The interval is kept in every position, no position loses a bound, and the W4 rule that a node's type is always the smallest interval is restated, not weakened.
- `div` and `rem` (FR-767, QSL-1989): FR-767 only supplies the law, and it says nothing about representation. Its `div` and `rem` fixtures (AC-5, AC-8, AC-9, AC-10) are at bounded result positions or Integer comparison positions, and they keep their outcomes under the new contract. The ruling's scope claim holds. The gap is coverage (FND-002).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The position cases do not cover every position. "Bounded position" covers only a `Coerce` of the application node itself, and "Every other position" is defined as "consumed at `Integer`" with a closed list. An indirect consumer into `Int[..]` falls in neither case: an `if` branch, a `let` binder read or a match arm. FR-495-AC-19's own `if b != 0 then a mod b else 0` into `Int[0, 9]` is an example. The lowered `if` or binder has an `Int[..]` `result_type` over an `Integer`-typed application, and the spec does not say whether a narrow is built, or where. Fix: state the lowering for an indirect consumer, define "every other position" as every position `check` does not coerce into `Int[..]`, and add an AC-21 case such as `: Int[0, 20] { let r = a mod b in r }` or the AC-19 `if`. The record rule (no direct narrow gives the interval with no context) already covers the record. | FR-495:225-235; FR-093:530-531,550 |
| FND-002 | medium | The representation contract is stated "for `div`, `rem` and `mod` alike", but AC-21 and AC-22 exercise only `mod`. No AC checks that a `div` or `rem` lowers at `Integer` while its checked node keeps the interval, or checks its record's bound and type node. Fix: add `div` and `rem` cases to AC-21 under a law header (for example, Euclidean). Include at least one bounded position and one `Integer` position each. Extend TC-2270 to match. | FR-495:216-244,383 |
| FND-003 | medium | TC-2270's test is tagged `FR-062-AC-13`. That AC verifies the RR-1 to RR-17 fixtures, and none of them is in this test. FR-062's new SHALL, that the bound's `ValueType` and its type node name the same type, has no FR-062 AC. Fix: add an FR-062 AC for the type-node rule, or an RR-18 `mod` fixture under AC-13, and tag the test with that AC. Otherwise drop the `FR-062-AC-13` tag. | TC-2270:40; FR-062:305-317,390 |

## Verdict

REQUEST CHANGES (with SR-202611160951002 FND-001, high). The contract itself is sound and agrees with FR-092, FR-098, FR-404, FR-075 and the W4 text. No feature is narrowed. The three findings above are gaps in coverage and in the position cases, not errors in the ruling.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
