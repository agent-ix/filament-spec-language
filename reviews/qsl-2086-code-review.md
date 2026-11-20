---
id: SR-1487
title: "Code review — returned replay evidence and direct assignment simulation"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; QSL-2086; fsl-analyze/tests/it/graph_world.rs, model_settle.rs, model_settle/replay_contracts.rs, state_graph.rs, state_graph/replay_contracts.rs; fsl-eval/src/simulation/model/frames/narrowing_tests.rs; fsl-eval/src/value/expression/evaluate/invariants.rs"
review_set: subset
---

## Summary

Reviewed the seven-file frozen diff at `3e3c4f8127c0191cc216573a9af53bf0438d3db9`, applying code-review and its Rust lane. New replay and simulation assertions use actual returned evidence, checked subjects, typed failures, and independently expected scalar values and candidate charges.

## Verdict

**PASS** — no code or Rust design defect found in the changed tests and helpers. The separate SR-1488 records one trace-tag correction.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Reviewed behavior

The graph helper delegates the original witness route to one source-recompiling replay implementation, preserving typed adverse results and caller limits. Its new tests compare real sampled/explored paths, evaluated target predicates, repeated replay, a genuinely truncated path, invalid initial paths, a changed real post-state key, and a returned trap's actual forward closure and first target.

Model settlement uses actual model-check outcomes and the original replay function. Enabled states remain enabled despite terminal predicates; actual deadlocks carry the concrete transition and canonical post-state key. The Undefined case preserves Kernel mode and observes division by zero at the reached position. The MaxStates case checks the exact incomplete result rather than manufacturing a stop. The wrong-initial case stays active; the reached PostState defect remains a strict QSL-2702 pending test.

Direct swap and guarded increment compare actual optimized and enumerated execution, with independently expected values and charges, keyed transitions and StepEffects, and public candidate-budget outcomes. The full bounded exploration comparison has explicit 301-state, 20,100-transition, depth-three expectations. Guards are explicitly separate from the original unguarded cases, which remain strict pending under QSL-2703. The unexecuted out-of-domain case is not claimed as passing. The existing dispatch fixture's required_arity 0 agrees with its empty parameter list.

All changes remain test-only. There is no copied external fixture, alternate production evaluator, fabricated proof, fake worker-mode label, warning suppression, gate removal, or production panic path. Panic and expect uses establish test fixture invariants or fail incorrect result arms. Existing fixture defaults are preserved through the new constructor delegation.

## Evidence and limits

Accepted the author's focused receipts without re-execution: five graph replay cases; the executed model settlement checks and retained QSL-2702 failure; actual swap and two guarded simulation cases, with original S3 failures retained. No test, build, lint, matrix, repository audit, or main-refresh check was run by this reviewer. Known analyze lint blockers and the evaluator charged dead-code warning remain disclosed; this is not a green lint claim. The lead retains the single final affected-component gate.

No assurance-profile inventory was performed: the dispatch explicitly confines review to this diff and its exact current criteria. No profile-based qualification or whole-repository assurance claim is made.
