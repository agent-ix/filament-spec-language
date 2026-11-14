---
id: SR-2026111414035506
title: "risk and complexity review of QSL-1913 and QSL-1914 model-checker scaling spec"
type: SpecReview
analysis: risk-complexity
scope: "agent-ix/quire-spec-language; spec/functional/FR-101-explore-finite-models-with-canonical-order-and-the-qspec-sampler.md; spec/functional/FR-120-simulate-a-checked-package-s-state-family.md; spec/functional/FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md; spec/functional/FR-191-bound-and-reproduce-a-statistical-run.md; spec/test-cases/TC-1417-worker-count-hash-indexed-seen-set-and-packed-state-leave-results-unchanged.md; PR quire-spec-language#2705 at a6d625440"
review_set: subset
---

## Summary

Tickets: QSL-1913 and QSL-1914. Technical risk before tasking.

The highest implementation risk is QSL-1835's merge-order charging in the
FR-126 explorers. Their monitor, liveness automaton and partial-order
reduction memos are single-threaded. The spec handles this correctly by
keeping budgets and stepping on the merge path, and the "same result for
every count" ACs pin it. The thread-safety bound gap is recorded as
failure-domain FND-004.

The solved-assignment rule changes `max_candidates` stop points for every
corpus model whose operations are plain equations, `Counter.inc` for
example. Every existing AC that names a `max_candidates` boundary was
checked: FR-120-AC-13, FR-120-AC-22, FR-126-AC-19 and FR-220-AC-16. Each
uses a clause-free operation or a limit of 0 or 1, so each still stops where
it did.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | low | FR-191-AC-6 runs `NoFault` with Okamoto to 9,210,340 samples at four worker counts. That is four times FR-191-AC-5's existing run, on top of the fixed-seed tests of FR-101-AC-16 to AC-19, making TC-1417 the slowest TC in the suite. A smaller `max_samples` stop shows the same merge-order property. | FR-191-AC-6 | wrong-requirement |

## Verdict

No blocking risk beyond the findings recorded in the other reviews.
