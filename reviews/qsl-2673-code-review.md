---
id: SR-2026112017124277723988803
title: "Code review of QSL-1848 and QSL-2673 in FSL PR 3213"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; QSL-1848, QSL-2673; timed trace and resumed stream changes"
review_set: subset
---

## Summary

Reviewed the frozen Rust diff and its acceptance bindings for timed offline monitor evaluation and checkpointed stream positions. The production paths appear coherent; one new integration test is bound to an unrelated criterion, so the test matrix does not credit the timed-monitor behavior it exercises.

## Verdict

One medium traceability finding. No blocking Rust correctness, safety, resource-bound, or API-convention defect was identified in the changed production code. Existing focused post-rebase checks were supplied by the dispatching brief and were not rerun.

## Review notes

The timed evaluator preserves exact rational stamps, rejects decreasing/negative stamps through `TimedTrace`, closes bounded windows only when a later stamp passes the interval or `through` covers its upper bound, and keeps unbounded future obligations pending. The `Timed` route authenticates against the selected timed clock and admits snapshot/sample positions before evaluation. For resumed monitor streams, the command now initializes and advances the emitted observation position from the session's checkpointed `seen` count; the added regression asserts both resumed and uninterrupted decisions use position 2.

The code review used the supplied focused passing checks: `fsl-analyze --test it monitor_inputs::timed_trace_uses_its_authenticated_clock_and_through_boundary` and `filament-spec-language --test it run_wire::a_checkpointed_stream_resumes_to_the_same_events`. No build or test was started during this review.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The new timed monitor integration test is tagged to the timestamped finite-window criterion FR-283-AC-15, although it tests `TraceInput::Timed`, timed clock authentication, and the timed `through` boundary; the tag does not cover the timed trace criterion. | fsl-analyze/tests/it/monitor_inputs.rs:860 |

## Dispositions

### Round 1

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | The timed trace test now carries the owning timed-trace criteria and no longer claims the unrelated timestamped-trace criterion. |

```yaml
dispositions:
  - fnd: FND-001
    outcome: fixed
    reason: "At PR head bbbec6dde30dbacfc17ae55b7988161792b6b33f, the test tag was corrected to TC-1664 and FR-642-AC-1 / FR-642-AC-2; FR-283-AC-15 was removed."
    after_excerpt: '#[trace("TC-1664", "FR-642-AC-1", "FR-642-AC-2")]'
```
