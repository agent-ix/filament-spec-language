---
id: SR-2026111612172400000038
title: "Gap analysis — QSL-2431 protocol message bound"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; QSL-2431; FR-225; FR-225-AC-6; FR-636-AC-4; TC-2986; fsl-analyze/src/analyze.rs, fsl-analyze/src/model_check/check.rs, fsl-analyze/src/model_check/liveness.rs, fsl-analyze/src/protocol_analysis/conformance.rs, fsl-analyze/tests/it/protocol_composition.rs, fsl-eval/src/simulation/protocol.rs, fsl-eval/src/simulation/protocol/channel.rs, fsl-eval/src/simulation/subject.rs, fsl-replay/src/lib.rs, fsl-replay/src/model_check/limits.rs, fsl-replay/src/model_check/outcome.rs, fsl-replay/src/model_check/settle.rs, fsl-replay/src/outcome.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-225
    type: reviews
  - target: ix://agent-ix/filament-spec-language/FR-636
    type: reviews
  - target: ix://agent-ix/filament-spec-language/TC-2986
    type: reviews
---

## Summary

The computed repository matrix and a scoped reverse-trace/stub inspection show the new channel-bound behavior is owned by FR-225 and exercised by the TC-2986 tests. The AC-4 test remains trace-bound only through an ignored test, and code inspection found two gaps in handling reachable candidates and reporting independent bound state.

## Verdict

**FAIL** — FR-636-AC-4 has no active trace-bound test in the computed matrix; two implementation gaps also remain in the changed bounded-result paths.

## Coverage and plan

Plan completion: not assessed.

`quoin matrix --repo . --json` completed with Quoin 0.36.6. In the computed matrix, FR-225-AC-6 is `tagged` by the two added tests (`max_messages_bounds_each_channels_occupancy` and `max_messages_does_not_sum_occupancy_across_channels`). FR-636-AC-4 is `tagged-by-ignored-test` only by `parallel_join_overflow_and_message_budget`; the active first test covers its bound-1 incomplete behavior but omits FR-636-AC-4 from its trace attribute. The targeted test runs passed, but no Quoin run-evidence binding was present in the matrix. No plan was supplied, so plan completion is not assessed. The optional intent↔test↔code semantic review was skipped; code review separately examined the scoped behavior. No applicable AssuranceProfile was found.

## Reverse trace

Changed production behavior maps to FR-225's channel-bound decision and FR-225-AC-6; the protocol incomplete settlement also implements FR-636-AC-4. The changed message-bound limit key is part of the same request/result contract. The two new tests exercise bound/default overflow and per-channel independence; the RA regression test covers preservation of the existing RA bound. No unowned changed behavior, production stubs, or test stubs were found in the changed paths.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-636-AC-4 has no active trace-bound test: the matrix binds it only to an ignored test, while the active bound-1 assertion omits FR-636-AC-4 from its trace tag. Add the criterion to the active test tag so the matrix records the exercised acceptance. | fsl-analyze/tests/it/protocol_composition.rs:733; FR-636-AC-4 |
| FND-002 | medium | The pending-finish scan saves a concrete reachable candidate but discards it if any other explored state is bound-limited. This loses a candidate whose own prefix may be within the bound and can settle the item incomplete despite a concrete refutation. | fsl-analyze/src/protocol_analysis/conformance.rs:778; FR-225 |
| FND-003 | medium | The run-stat union of message-bound and memory-bound states causes TSO store-buffer reporting to claim a channel-only cut reached `max_store_buffer`; the message-bound result path also misses reached cuts when another outcome settles. Keep per-bound reached state separate through settlement and output. | fsl-analyze/src/model_check/check.rs:1849; fsl-replay/src/model_check/settle.rs:358; fsl-analyze/src/protocol_analysis/conformance.rs:630; FR-225 |

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | The active bound-1 test now carries the FR-636-AC-4 trace tag; the recomputed matrix lists the active test as a binder. |
| FND-002 | still-open | When `pending_finish_counterexample` finds a candidate, `every_behavior_result` still replaces the bounded check outcome with `Violated { stats: Default::default() }`. That drops `message_bound_limited`; `behavior_report` therefore still reports `max_messages.reached` false for this counterexample path when a different explored state was cut. |
| FND-003 | fixed | `RunStats` now keeps channel-cut counts separate, and the bound serializer reads `message_bound_limited` without treating it as `max_store_buffer` reached. |
