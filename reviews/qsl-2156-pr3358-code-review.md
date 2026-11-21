---
id: SR-2026112117493700003358
title: "QSL-2156 PR 3358 code review"
type: SpecReview
date: 2026-11-21
scope: "agent-ix/filament-spec-language; fsl-analyze/src/analyze/exact/elapsed.rs, fsl-analyze/src/exact_probabilistic/digital.rs, fsl-replay/src/probability_certificate.rs, fsl-replay/src/probability_certificate/document.rs, fsl-replay/src/probability_certificate/document/timed.rs, fsl-replay/src/probability_certificate/document/timed/elapsed.rs, fsl-replay/src/probability_certificate/document/writer.rs"
---
# SR-2026112117493700003358: QSL-2156 PR 3358 code review

## Summary

Ticket: QSL-2156. PR 3358 at e20bf3cf5d224896a25c4879b1290b1de99aa0af (elapsed-quantile >= claims over the region route, upper documents). Method: code-review with rust-review lane, soundness first.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | At y(s0) = 0 with no activated state reachable under tight actions the checker rejects ThresholdMissed; FR-201 says accept. The producer returns NOT_YET at value >= 0, so a true >= claim with maximum exactly 0 never proves | fsl-replay/src/probability_certificate.rs:1916-1955; fsl-analyze/src/analyze/exact/elapsed.rs:207 |
| FND-002 | low | The (AtLeast, StrictUpper) arm accepts initial > threshold from an upper bound, which would be unsound if ever reached | fsl-replay/src/probability_certificate.rs:1913 |
| FND-003 | low | The acceptance side is the transform's bound (AtMost), not the claim's side (AtLeast) that FR-201 says to report | fsl-replay/src/probability_certificate.rs:1962-1965 |
| FND-004 | low | The checked_report call exceeds rustfmt width | fsl-analyze/src/analyze/exact/elapsed.rs:278 |

## Verdict

No unsound acceptance. For >= the checker uses its own strict [M < c] (strict = at_least), Maximum, Bound::AtMost(0) with StrictUpper (initial < 0). On the acyclic graph (topological_order still runs) y >= reward at terminals and y >= best-choice expectation elsewhere give y >= V by induction, so a too-low value is rejected and a too-high one only misses the threshold. The tight-action search is bounded by a seen set, polls cancel and only rejects. Region keys put the elapsed clock's fraction into the fraction order unless the measure is inactive; censoring happens at W + 1/subdivisions, the encoder admits values up to W + subdivisions, and an inactive waiting loop is bounded by (max cap + 1) * subdivisions + 2 steps. A time-lock under an active measure is Unsupported and cycles give Err(Cycle). Routing: the producer returns NOT_YET unless the region route equals at_least; the checker requires the Digital route for <= and Region for >=. Upper kind is read only with maximum and elapsed-quantile. No Python or shell.
