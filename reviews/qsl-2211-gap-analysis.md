---
id: SR-2026111612172400000043
title: "Gap analysis — QSL-2211 observe-only protocol boundaries"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; PR #2932 at c6746094bf0273300f29ab67d0bdd8bc8a9d5c80; FR-218-AC-45..47; FR-206-AC-13; FR-383-AC-69; observe-only tests in fsl-semantics, fsl-eval, fsl-package"
review_set: subset
---

## Summary

Mapped the changed observe-only implementation and its tagged tests to the acceptance criteria. The main focused tests pass, but several expressly required branches lack test evidence; the strict-reader criterion is currently blocked by the known external QSL-2449 wire incompatibility.

## Verdict

**FIX ROUND REQUIRED; NOT MERGEABLE YET** — findings FND-001 through FND-004 identify acceptance branches not covered by the new focused tests. FR-383-AC-69 also has no passing reader evidence on this head because the strict package reader fails on contract-ir v0.1.4 with the known QSL-2449 `MalformedWire`; this external blocker independently prevents merge until resolved or explicitly dispositioned by the owning lane.

## Acceptance mapping

| Unit | Evidence |
| --- | --- |
| FR-218-AC-45 | `an_observe_only_step_checks` covers marker recording and capture named `observe`; it does not assert double-marker parse refusal or parity with the unmarked clause. |
| FR-218-AC-46 | `a_side_effect_under_observe_only_refuses` covers attempt, send, effect, role/control boundary and one outside attempt; commit, event-for, compensation, outside variants, nested choice and source-order multiplicity are not asserted. |
| FR-218-AC-47 | `a_recommendation_result_is_not_a_decision_fact` covers a visible binder read; unknown operation, foreign contract and while guard are not asserted. |
| FR-206-AC-13 | Evaluator test checks `semantic` state and recommendation metadata; it does not compare compensation registrations, test another role's state-changing attempt, or verify runtime refusal before the first step. |
| FR-383-AC-69 | Tagged package test checks required emitted arguments and node-id parity, but its strict reader call fails on contract-ir v0.1.4 at `/semantic_graph/nodes/3` (`MalformedWire`), tracked as QSL-2449. |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-218-AC-45's test omits the required duplicate-`observe` parse refusal and marked/unmarked checked-clause parity assertions. | fsl-semantics/src/check/protocol_clause/tests/observe_only.rs:21-69 |
| FND-002 | medium | FR-218-AC-46's test omits explicit commit, event-for, compensation, nested-choice, source-order multiplicity, and most outside-boundary cases. | fsl-semantics/src/check/protocol_clause/tests/observe_only.rs:72-130 |
| FND-003 | medium | FR-218-AC-47's test checks only a visible binder read, leaving unknown-operation, foreign-contract, and while-guard refusal cases without evidence. | fsl-semantics/src/check/protocol_clause/tests/observe_only.rs:132-150 |
| FND-004 | medium | FR-206-AC-13's evaluator test compares only the `semantic` state-key member and omits compensation registrations, another-role state mutation, and runtime refusal before the first step. | fsl-eval/tests/it/protocol_observe_only.rs:178-215 |

## Evidence limits

FR-383-AC-69 is tagged to the package test and the expected assertions are present, so this is not an implementation-to-test binding gap. However, the required strict-reader success is not demonstrated on this head: the targeted test fails in the external contract-ir v0.1.4 reader at `/semantic_graph/nodes/3`. QSL-2449 tracks that dependency issue; no workaround was inferred or attempted. The repo-wide Quoin matrix has no run-evidence records, so the direct focused test outcomes above are the available execution evidence.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | The test now checks duplicate-marker refusal at the second marker and compares marked/unmarked role and control forms after clearing only the marker field. |
| FND-002 | fixed | The test now covers nested-choice attempt and commit in source order, role-level compensation/attempt/effect/event-for/commit refusals, and checks the recovery flow without an observe role; the send test also checks the unmarked control case. |
| FND-003 | fixed | The test now covers unknown recommended operation, a recommendation with a foreign contract, and a while-guard read of the recommendation binder. |
| FND-004 | fixed | The evaluator tests compare registration keys and values for observe-only transitions, demonstrate the unobserved role's write, and assert observe-attempt checking refuses before evaluator subject construction. |
