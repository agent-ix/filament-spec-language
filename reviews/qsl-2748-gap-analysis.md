---
id: SR-1545
title: "Scoped gap analysis of QSL-2748 bounded delay fractions"
type: SpecReview
analysis: gap-analysis
scope: "FR-254; FSpec-FR-407-AC-19; FSpec-FR-407-AC-20; fsl-analyze/src/statistical/model.rs; fsl-analyze/tests/it/statistical_model.rs; fsl-analyze/tests/it/probability_from_source.rs; fsl-replay/src/statistical.rs"
review_set: subset
pr: "filament-spec-language#3361"
reviewed_head: "718966c9f78684676baf4d14042d34dc06a53597"
---

## Summary

One source-only gap analysis of QSL-2748 at `718966c9f78684676baf4d14042d34dc06a53597`, scoped to bounded statistical delay-weighted fractions and window clipping. The changed behavior has an existing owning contract and substantive model/source assertions; no functional high or medium gap was found in this slice.

## Verdict

**PASS** for the assigned slice. This is not a repository-wide matrix or assurance claim. PR: [filament-spec-language#3361](https://github.com/agent-ix/filament-spec-language/pull/3361), reviewed head `718966c9f78684676baf4d14042d34dc06a53597`.

Plan completion: not assessed

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Requirement, source and test alignment

The authoritative dependency pin is `d6d066094753ffba830aa684fdff5810b6c61fa5`. Its FSpec FR-407 fraction rule explicitly clips a leaving step to h minus stamp and assigns an idle tail the same remaining duration. AC-19 fixes the crossing example at 3/4; AC-20 fixes the idle example at 1/4. Local FR-254 owns timed sampling, exact rational delays, deterministic seeds, typed refusal/budget outcomes and witness replay.

The new timed_fraction reader implements those interval weights on sample_timed's aligned states/stamps, with exact rational accumulation and checked unit-fraction conversion. A final position at h has zero weight; consecutive same-time positions have zero-length intervals. A zero-duration denominator is explicitly undefined. The sampler retains end-boundary sampling/refusal behavior and its existing step accounting. The new fraction horizon in provenance connects the sampling reader to existing timed replay; long-run delay weighting remains explicitly unsupported.

The model test is traced to FSpec-FR-407-AC-19 and AC-20 and asserts the exact independent ratios, exact-end clipping, repeatability and step-budget settlement. The source test is traced to AC-19 and exercises source parsing/checking and the real public run route for both mean and probability-event forms, asserting exact estimate 1/1 and measured/accepted settlement. Neither replaces production semantics, derives its oracle from the producer nor guards its substantive assertions behind an optional path.

Reverse-gap and stub inspection was confined to the changed behavior: it implements the cited bounded fraction requirement, contains no stub or placeholder result, and does not add an unowned capability. The long-run refusal text now accurately identifies the remaining unsupported form. No probability-journey, unrelated traceability, diagnostic or performance work was added.

## Evidence limits

Per the explicit source-only assignment, no computed matrix, tests, build, lint, profiling or aggregate gate was run. Direct trace tags and test bodies were inspected instead of making a repository-wide computed-matrix claim. Focused implementation receipts are accepted as reported. This intentionally bounded review does not assert full repository gap-analysis completion or broader FR-254 acceptance coverage.
