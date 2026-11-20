---
id: SR-1488
title: "Gap analysis — QSL-2086 replay and simulation test bindings"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; QSL-2086; the seven changed test/helper files at 3e3c4f8127c0191cc216573a9af53bf0438d3db9; exact FR-120, FR-124, FR-127, FR-128, FR-170 and FR-577 criteria cited by the diff"
review_set: subset
---

## Summary

Compared the changed test assertions and strict pending cases with their exact current normative criteria. One new live trace tag claims a criterion whose distinctive behavior the test does not assert.

## Verdict

**CONDITIONAL** — remove the unsupported FR-127-AC-4 tag; the existing FR-127-AC-1 tag correctly covers the tested MaxStates settlement. This is a diff-scoped semantic and binding review under the explicit user scope, not a full repository gap audit.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The Undefined/MaxStates test cites FR-127-AC-4 without asserting its automaton-state, cancellation, depth, work-unit or persisted three-limit behavior; remove that tag and retain FR-127-AC-1. | fsl-analyze/tests/it/model_settle/replay_contracts.rs:121 |

## Coverage

Plan completion: not assessed

The exact new FR-170 bindings back actual replay, evaluated targets, malformed initial paths, disagreement, trap closure and settlement facets. FR-124/127/128 tests use real model outcomes, reproduced deadlocks, typed invalid initial state, Undefined replay, and resource-incomplete settlement. Swap and increment exercise actual optimization/reference paths with independent scalar/candidate expectations and canonical transition/effect comparisons. The guarded increment tests cover the supported assignment behavior with explicit FR-577 preconditions; they do not establish admission of FR-120's unguarded example, which remains separately pending.

Four new strict pending cases remain excluded from completion evidence under QSL-2702/2703. Missing actual worker/packed/hash execution modes remain owned by QSL-2700/1833, untagged and unclaimed. The pure settlement mapper has no duplicate-settlement lifecycle promise, so no invented refusal is tested. Done specification tickets and explicit-precondition implementation work are not reopened. All original failure receipts remain authoritative; the unexecuted out-of-domain case is not represented as a reached failure or pass.

The user explicitly prohibits another matrix, measurement, whole-repository inventory, or rechecking peer results. Accordingly the matrix, reverse-gap inventory, global stub scan, and plan completeness portions of the general skill are not executed. Existing program measurements are not recomputed. This artifact claims only the seven-file assertion-to-criterion review described here, not a repository-level engine-backed PASS.

## Finding detail

The function `returned_undefined_and_actual_state_limit_runs_settle_without_invented_outcomes` asserts genuine Undefined replay and a MaxStates stop with value 1. FR-127-AC-1 owns the MaxStates map. FR-127-AC-4 instead names max_automaton_states, cancellation sources, max_depth, work-unit limits, and recording the three limits used; the body reads none of those fields or scenarios. Removing the extra tag is sufficient and requires no runtime test rerun.
