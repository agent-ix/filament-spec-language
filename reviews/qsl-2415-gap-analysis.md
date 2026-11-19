---
id: SR-2026111612172400000051
title: "Gap analysis — QSL-2415 bounded recovery exploration"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; QSL-2415; PR #3000; FR-207-AC-5 and its changed tests"
review_set: subset
relationships:
  - { target: "ix://agent-ix/filament-spec-language/FR-207", type: references }
---

## Summary

Ticket: QSL-2415. Scoped manual gap check of the changed recovery-view tests against FR-207-AC-5 and the ticket's requirement to bound exploration while preserving the assertion. The bounded-check assertions are present, but the modified analyzer test fails in its replacement walk before it reaches the recovery successors. Plan completion: not assessed.

## Verdict

CONDITIONAL. The test change does not yet provide passing evidence for the preserved recovery-view assertion.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Changed analyzer test stops before asserting the recovery-view successors | fsl-analyze/tests/it/protocol_compensation.rs:1485-1491 |

## Coverage

- Examined FR-207-AC-5, QSL-2415, the diff in `fsl-analyze/tests/it/protocol_compensation.rs` and `fsl-eval/tests/it/protocol_retry_bound.rs`, and the `check_protocol` path and its model-check limits.
- `quoin matrix --repo . --json` completed with exit 0. It is a repository-wide computation with extensive existing untagged/unbacked rows; no trace tags or specification files changed in this PR, so no matrix delta is claimed. The tags on the changed recovery tests remain unchanged.
- The exact changed analyzer test was run with a 16 GiB virtual-memory cap, one thread, and explicit name exclusions. It failed at the `CAttempt` selection before the recovery-result assertions.
- The prohibited fsl-eval recovery-view test was not run. No aggregate tests, full repository gap audit, or optional semantic review were run. Plan completion: not assessed.

## Examined units

```yaml
scope:
  - id: FR-207-AC-5
    path: spec/functional/FR-207-run-compensation-templates-as-protocol-threads.md
    role: examined
    excerpt: "With the recovery predicate `r.bal = 0`, which reads its recovery binder `r` of type `Recovery` (`bal: Int[0, 1]`), `cend` at t6 has exactly two successors: the registration `recovered` with `r.bal` 0 and `unrecovered` with `r.bal` 1, the two transition identities differing only in the record; no item settles `unsupported`, and a claim that every finished instance is `recovered` is refuted with a counterexample carrying `r.bal` 1 that replays."
  - id: QSL-2415
    path: Linear issue
    role: examined
    excerpt: "Find the unbounded exploration or allocation, bound it under FR-109 (a metered stop, not a crash), and keep the test's assertion."
```

## Finding detail

### FND-001 — Changed analyzer test stops before asserting the recovery-view successors

The bounded route assertion in `fsl-analyze/tests/it/protocol_compensation.rs` reaches the subsequent direct walk, but the selected `Attempt`, `Event`, `CAttempt` sequence is not enabled. The exact targeted test fails at line 1491, so the PR's analyzer evidence does not establish the recovery-view behavior required by FR-207-AC-5. The failure is localized to test path selection; no production code is changed by this PR.

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | The bounded BFS now finds the reachable cend successors; the code lead reports the exact filtered analyzer regression passed under the 16 GiB cap. |
