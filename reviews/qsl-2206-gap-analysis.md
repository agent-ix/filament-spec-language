---
id: SR-202611030004
title: "Gap analysis of filament-spec-language PR #2778: recovery criteria"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; fsl-eval/tests/it/protocol_retry_bound.rs; PR #2778 head b89a5fc7bd3452a7752a1538dd66ab1003ac4f22"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-730
    type: reviews
---
# Gap analysis of PR #2778

## Summary

Ticket: QSL-2206. Rechecked the frozen head `b89a5fc7bd3452a7752a1538dd66ab1003ac4f22`; no fix commit changed the recovery tests. The new test checks cend record identity and recovered/unrecovered successor statuses, but misses three explicit FR-730 assertions: the claim/counterexample, no-read single successor, and replay/record mutation.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FSpec-FR-426-AC-1 | fsl-eval/tests/it/protocol_retry_bound.rs | examined |
| FSpec-FR-426-AC-9 | fsl-eval/tests/it/protocol_retry_bound.rs | context_only |
| FSpec-FR-427-AC-6 | fsl-eval/tests/it/protocol_retry_bound.rs | examined |
| FSpec-FR-730-AC-1 | fsl-eval/tests/it/protocol_retry_bound.rs | examined |
| FSpec-FR-730-AC-2 | fsl-eval/tests/it/protocol_retry_bound.rs | examined |
| FSpec-FR-730-AC-3 | fsl-eval/tests/it/protocol_retry_bound.rs | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The recovery test checks record/status successors but not FR-730-AC-1's claim and counterexample. | fsl-eval/tests/it/protocol_retry_bound.rs:502-564 |
| FND-002 | medium | No-read recovery predicate single-successor behavior is not asserted. | fsl-eval/tests/it/protocol_retry_bound.rs:472-564 |
| FND-003 | medium | No test replays a recovery counterexample or varies its carried record. | fsl-eval/tests/it/protocol_retry_bound.rs:502-564 |

## Dispositions

Disposition round 1 at `b89a5fc7bd3452a7752a1538dd66ab1003ac4f22`:

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | still-open | The changed integration test checks only cend record/status successors; no claim verdict or FR-730-AC-1 counterexample carrying `bal` 1 is asserted. |
| FND-002 | still-open | The changed tests still lack a no-read predicate such as `triggered`, with exactly one cend successor and unchanged claim verdict. |
| FND-003 | still-open | No changed test replays the recovery counterexample or checks that changing its carried record to `bal` 0 stops reproducing. |

## Verdict

Not mergeable under the gap findings. FND-001 through FND-003 describe unasserted acceptance behaviors, not merely test style. Targeted test execution cannot establish those missing assertions.
