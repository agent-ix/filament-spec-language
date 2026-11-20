---
id: SR-2026111612172400000050
title: "Code review — QSL-2415 bounded recovery exploration"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; QSL-2415; PR #3000; fsl-analyze/tests/it/protocol_compensation.rs and fsl-eval/tests/it/protocol_retry_bound.rs"
review_set: subset
relationships:
  - { target: "ix://agent-ix/filament-spec-language/FR-207", type: references }
---

## Summary

Ticket: QSL-2415. PR: filament-spec-language#3000 at 4df42c0ca8def8d33ef11431857e204f0efd961d. The diff replaces unbounded graph enumeration with bounded test walks and adds explicit model-check limit assertions. The exact changed `fsl-analyze` recovery-view test fails before its recovery assertions: its manually selected transition sequence has no `CAttempt` successor.

## Verdict

CONDITIONAL. The proposed change is scoped and avoids the former graph-wide traversal, but the added focused analyzer test does not pass.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Recovery-view test selects a transition sequence that does not reach CAttempt | fsl-analyze/tests/it/protocol_compensation.rs:1485-1491 |

## Scope and evidence

- Examined FR-207-AC-5, QSL-2415, and both changed test files. No production source or CI files changed.
- Ran the exact changed analyzer test under a 16 GiB virtual-memory cap, one thread, and the required name exclusions: `cargo test -p fsl-analyze --test it protocol_compensation::a_recovery_predicate_over_the_recovery_view_is_evaluated -- --exact --skip protocol_constructs::two_protocols_declaring_one_channel_name_keep_their_queues_apart --skip protocol_retry_bound::a_recovery_predicate_over_the_recovery_view_is_evaluated --test-threads=1`. It failed at line 1491 with `the finite recovery path has a CAttempt step`.
- The preceding bounded `check_protocol` assertions completed; the failure is in the subsequent direct transition walk. The repo's tests and implementation were not broadly built or run. The prohibited fsl-eval recovery-view test was not run.
- Rust review was static plus this focused failure reproduction. No aggregate gate was run.

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

### FND-001 — Recovery-view test selects a transition sequence that does not reach CAttempt

`protocol_compensation::a_recovery_predicate_over_the_recovery_view_is_evaluated` steps through the first `Attempt`, then the first `Event`, then expects a `CAttempt`. The focused run reproduces that no `CAttempt` is enabled at the selected successor, so the PR's modified analyzer integration target fails before checking the recovery records. The prior fixture-driven `fail`/`explore` path selected the required failure transition from the correct state. Use the fixture helper or select the actual enabled transition path before asserting the recovery successors.

## Coverage

The PR addresses the ticket's bounded-exploration intent in the changed tests and adds `check_protocol` limit assertions. The recovery-record behavior assertion remains represented, but the analyzer test currently cannot reach it. The fsl-eval test with the prohibited name was not run; no result is claimed for it.

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | The test now uses a bounded BFS to locate the enabled cend successors instead of assuming an Attempt/Event/CAttempt sequence. |
