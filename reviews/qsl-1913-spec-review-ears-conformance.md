---
id: SR-2026111414035502
title: "EARS conformance review of QSL-1913 and QSL-1914 model-checker scaling spec"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/quire-spec-language; spec/functional/FR-101-explore-finite-models-with-canonical-order-and-the-qspec-sampler.md; spec/functional/FR-120-simulate-a-checked-package-s-state-family.md; spec/functional/FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md; spec/functional/FR-191-bound-and-reproduce-a-statistical-run.md; PR quire-spec-language#2705 at a6d625440"
review_set: subset
---

## Summary

Tickets: QSL-1913 and QSL-1914. This review checks the requirement grammar of
the new statements. FR-126's and FR-191's additions use SHALL, as their
files do. FR-101's additions are declarative prose, which matches that
file's style ("The engine orders successors itself"). FR-120's additions use
MAY plus bullets. The MAY conflicts with its own ACs. That defect is recorded
once, as failure-domain FND-001, and is not repeated here.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | low | A single FR-126 bullet holds six requirements: where the worker count comes from, equal results for every count, the `workers` 0 refusal, merge-order budgets, cancellation as a prefix, and exclusion from identity. A failing AC cannot point at one of them. Split it into one bullet each. FR-191's Workers bullet 3 combines result equality and cancellation the same way. | FR-126:328-340, FR-191:101-106 | wrong-requirement |
| FND-002 | low | The eligibility bullet says `e` may read "`self` and its pre-state fields". In a `post` clause, `self.f` reads the post-state, and AC-31 treats `self.a = self.b` as reading the candidate. As worded, one implementer could accept `self.b` as a pre read. Say "`pre(self.g)` only". | FR-120:694-705, FR-120-AC-31 | wrong-requirement |

## Verdict

No blocking EARS defect beyond the MAY recorded in the failure-domain review.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | The FR-126 bullet is split into six SHALL bullets (worker source, equal results, refusal, merge-order budgets, cancellation, identity exclusion), and FR-191 separates equal results from cancellation. |
| FND-002 | fixed | Eligibility now says "In a `post` clause `self.f` names the post-state, and `pre(self.g)` names the pre-state", and `e` may read only `pre(self.g)` terms and arguments. |
