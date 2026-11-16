---
id: SR-202611160951004
title: "Criterion strength review of the QSL-2002 new ACs"
type: SpecReview
analysis: criterion-strength
scope: "agent-ix/filament-spec-language; PR #2745 at f3fcd3fc18f1f9277432fef2f59ce4f929669f09, diff origin/main...HEAD: FR-495-AC-21, FR-495-AC-22, TC-2270"
review_set: subset
relationships:
  - { target: ix://agent-ix/filament-spec-language/FR-495, type: references }
  - { target: ix://agent-ix/filament-spec-language/TC-2270, type: references }
---

## Summary

Ticket: QSL-2002. Jev (typesafe.ai System One) is not installed on this host (`jev` is not on PATH), so this lens was judged by hand over the spec text.

Both ACs are testable, and each can fail.

- FR-495-AC-21 fixes the unit, the law and the mode, and names nine functions. For each one it gives the checked `mod` type, the lowered `result_type`, the narrow count, and the record's bound, type node and conversion context. It also checks the request item's node and context. Each of these defects fails a named assertion: an `Integer` consumer that erases the interval, a derived record that reads the application's `Integer` node, a missing narrow, or a context on an unnarrowed record.
- FR-495-AC-22 fixes the assignment (`a = 100`, `b = 12`, so `100 mod 12` is 4) and the expected `Verdicts{violation, success}` with value `true`. It explicitly excludes `NoValue`, so a weak `ReplayParity(_)` match does not satisfy it.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | AC-22 says "a plugin counterexample" and requires that the item settle "naming the plugin", but names neither the plugin nor the route that settles the item. FR-404-AC-2, which AC-22 follows, names `acme` and the `run_with_replay_context` run. Fix: name the plugin backend and the settling route, for example "the FR-404-AC-1 `acme` plugin through the call replay context", and state the expected plugin name. | FR-495:384 |

## Verdict

PASS for this lens, with one low finding.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
