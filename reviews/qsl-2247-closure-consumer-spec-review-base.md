---
id: SR-2026112118004026158501
title: "spec-review base of PR3360 closure-stutter replay consumer contract"
type: SpecReview
analysis: base
scope: "agent-ix/filament-spec-language; PR3360 78e47c7ccb06352af03673036e9e24a37ecb1bb1 over base 158cc4328568a0c864b90b07105a4276f19d6f0e; spec/functional/FR-128-replay-a-model-counterexample.md, spec/functional/FR-1030-request-a-refinement-transfer-from-a-native-run.md, spec/test-cases/TC-2920-request-a-refinement-transfer-from-a-native-run.md"
review_set: subset
---

## Summary

Ticket: QSL-2247. The bounded consumption contract matches FSpec FR-364 Closure stutter and AC-25..29 on alphabet, placement, kind, delay order, post-execution pointers, fairness with real enabledness and no taken identity, refutation only through replay, terminal-stutter unchanged and the observed ModelStepWire unchanged. It adds no request or envelope member, upstream API or pin. FR-1030 rightly turns ClosureStutterLoop unsupported into replayed refuted, and keeps TP-3 on each and protocol as an honest failed limit. Defects: the package-route rule contradicts itself, AC-14 has a vacuous clause, and two criteria have imprecise preconditions.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-128 requires both the source clause route and the package route to read the flag, then lets a route that does not read it keep refusing; the package adapter refuses today, so whether it is in scope is undecided | spec/functional/FR-128-replay-a-model-counterexample.md:201-205 |
| FND-002 | medium | FR-128-AC-14's "a route whose adapter does not read the flag, if any remains" passes whether or not the package route is implemented, and no AC gives the package route a positive closure case | spec/functional/FR-128-replay-a-model-counterexample.md:375 |
| FND-003 | low | FR-128-AC-11 requires a refinement item to refuse "even when the item selects the flag", but FSpec FR-364 says a refinement item never selects stutter_closure | spec/functional/FR-128-replay-a-model-counterexample.md:372 |
| FND-004 | low | FR-128-AC-10 names its empty-stem clause only by description and gives no trace_position for that case | spec/functional/FR-128-replay-a-model-counterexample.md:371 |

FND-001: decide explicitly. Either the package route carries the flag (PackageClauseSubject::item and replay, clause_item.rs:884, 923, 1011) with its own positive and refusal AC, or it is named as a typed-refusal limit with a Remaining-work ticket. Do not do both.

FND-002: replace "if any remains" with a named route and an asserted outcome.

FND-003: state it as a refinement_failure prefix closure step refused by the alphabet rule, since a refinement item never selects the flag.

FND-004: name FORMULA (always eventually holds(c.value = 3), lasso_replay.rs:37) and its trace_position.

## Scope and evidence

- FR-128 (spec/functional/FR-128-replay-a-model-counterexample.md): examined, new Inputs bullet, Behavior lines 171-207, fairness, settlement, named limit and writer/reader bullets
- FR-128-AC-10..AC-14 (spec/functional/FR-128-replay-a-model-counterexample.md): examined
- FR-1030 (spec/functional/FR-1030-request-a-refinement-transfer-from-a-native-run.md): examined, the TP-3 limit wording and the closure bullet; no criterion was weakened and AC-1..AC-5 are unchanged
- FR-1030-AC-6, FR-1030-AC-7: examined
- TC-2920: examined, steps 6 and 7 and the expected results
- FSpec FR-364 (filament-specification): context_only

## Dispositions

Disposition pass 1, checked against the frozen fix 101574eb2b8a5a7102740deee9ef6dad08f5cc67.

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | FR-128 now requires the source and package routes of a bounded ordinary untimed clause to read the existing flag and replay a closure lasso. The package route does this through the existing PackageClauseSubject adapter, which stops refusing the flag, with no new IR, API or pin. A refusal-only package route does not meet the requirement, and a real blocker must be reported, not substituted. This matches root's decision. |
| FND-002 | fixed | The "if any remains" escape is removed from AC-14. The new FR-128-AC-15 requires a positive case on both routes with equal results, and a refusal without the flag. The named harness exists: PackageClauseSubject::new and the both-routes parity test (clause_item.rs:505-583) and over_package_route (587). It is proposed, not run. |
| FND-003 | fixed | AC-11 now says a refinement item never selects the flag, so a closure step in a refinement_failure prefix refuses by the alphabet rule at /prefix/<i>/transition/type. |
| FND-004 | fixed | AC-10 names FORMULA (always eventually holds(c.value = 3), is_three) and trace_position 0. |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-005 | low | The fix splits the code span `temporal.stutter_closure` across a line, so it renders as `temporal. stutter_closure` in the new package-route requirement | spec/functional/FR-128-replay-a-model-counterexample.md:207-208 |
