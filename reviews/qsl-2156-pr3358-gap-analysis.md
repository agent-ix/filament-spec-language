---
id: SR-2026112117493700013358
title: "QSL-2156 PR 3358 gap analysis"
type: SpecReview
date: 2026-11-21
scope: "agent-ix/filament-spec-language; spec/functional/FR-877-certify-elapsed-quantile-verdicts.md, spec/functional/FR-201-check-a-probability-certificate.md, fsl-analyze/tests/it/timed_product_certificate.rs, fsl-replay/src/probability_certificate.rs"
---
# SR-2026112117493700013358: QSL-2156 PR 3358 gap analysis

## Summary

Ticket: QSL-2156. PR 3358 at e20bf3cf5d224896a25c4879b1290b1de99aa0af. Method: gap-analysis (manual AC-to-test check against FR-877, FR-201, FR-920; no matrix run or builds per brief).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | No test exercises the StrictUpper tight-action path at y(s0) = 0; a checker-level unit test is cheap | fsl-replay/src/probability_certificate.rs (mod tests) |
| FND-002 | medium | FR-201 names the canonical-order first reachable activated state; FR-877 and AC-4 name the first state reached (the initial state); the code does breadth-first order | spec/functional/FR-201-check-a-probability-certificate.md:614-616 |
| FND-003 | low | FR-877 omits acceptance when the tight actions reach no activated state | spec/functional/FR-877-certify-elapsed-quantile-verdicts.md:57-60 |
| FND-004 | low | The tamper test is tagged FR-877-AC-3, whose text has no rejection clause | fsl-analyze/tests/it/timed_product_certificate.rs:742-744 |

## Verdict

Fix round needed (code-review FND-001, gap FND-001, FND-002). AC-3's value -1/2 checks out by hand: the guard x >= 1 ms means no send before 1 ms, so M >= 1 ms on every activated run, [M < 1 ms] = 0 and Y = -1/2 under every scheduler. The un-ignored AC-3 test asserts route region, kind upper, proved_value -1/2 and checker acceptance. The tamper test's lowered case rejects InequalityFails and its raised case ThresholdMissed, both by the checker's own enumeration.
