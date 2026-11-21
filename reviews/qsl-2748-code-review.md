---
id: SR-1544
title: "Code and Rust review of QSL-2748 bounded delay fractions"
type: SpecReview
analysis: code-review
scope: "fsl-analyze/src/statistical/model.rs; fsl-analyze/tests/it/statistical_model.rs; fsl-analyze/tests/it/probability_from_source.rs; fsl-replay/src/statistical.rs"
review_set: subset
pr: "filament-spec-language#3361"
reviewed_head: "718966c9f78684676baf4d14042d34dc06a53597"
---

## Summary

One independent source review of frozen branch `codex/qsl-2748-delay-fractions`, commit `718966c9f78684676baf4d14042d34dc06a53597`, for QSL-2748. Bounded delay fractions now use exact sampled timestamp differences and clip the final residence interval to the window end; no functional high or medium defect was found in the assigned change.

## Verdict

**PASS** for the assigned bounded delay-fraction implementation. PR: [filament-spec-language#3361](https://github.com/agent-ix/filament-spec-language/pull/3361), reviewed head `718966c9f78684676baf4d14042d34dc06a53597`.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Code and Rust review

The review applied repository AGENTS.md/CLAUDE.md and the code-review, rust-review and rust-style guidance to the frozen diff. No production API, serialization field, unsafe code, concurrency mechanism, dependency or gate was added. The long-run refusal remains a distinct typed refusal; only its description is narrowed.

The timed sampler starts at timestamp zero. Each included position is weighted by the next included timestamp minus its timestamp, and the final included position by horizon minus timestamp. Sampled states beyond the horizon are not included. This yields 1 + 3 = 4 total duration for both pinned crossing and idle examples. Multiple sampled positions at one timestamp contribute zero; a position exactly at the horizon contributes zero; an all-zero window reaches the explicit DivisionByZero undefined result rather than dividing unchecked. Predicate undefined/refused readings retain their typed settlement. Sampling retains the caller's seed, trace, q and step budget and returns the sampler's actual steps.

The claim retains its timed horizon in provenance. ModelWitnessReader::read_timed recomputes through the same bounded reader and compares actual sampled steps with the replay record, preserving sampled result/replay agreement for the new delay route. Existing sampling treatment of end-boundary races, time locks and free delays is retained.

## Test evidence

The new model regression independently fixes 3/4 for a crossing step and a step ending exactly at the horizon, and 1/4 for an idle tail. The assertions would fail for whole-step weighting, omitted idle weight or position-count weighting; expected ratios are literal mathematical values rather than values computed by the producer. Seed repeatability and the zero step budget are asserted. The source regression exercises the actual run route for mean and probability-event claims and asserts measured, accepted and exact 1/1 outcomes. Its initial JSON assertion-shape failure is preserved in the accepted receipts; production outcomes were not rewritten.

Focused test and formatting receipts supplied by the implementation lane are accepted without rerunning them. No tests, builds, lint, matrix, aggregate gate, profiling or additional review pass was run by this reviewer, as explicitly scoped. This verdict does not claim those checks ran.
