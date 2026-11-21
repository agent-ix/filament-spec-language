---
id: SR-2026112117241500003347
title: "QSL-2156 PR 3347 code review"
type: SpecReview
date: 2026-11-21
scope: "agent-ix/filament-spec-language; fsl-analyze/src/analyze/exact.rs, fsl-analyze/src/analyze/exact/elapsed.rs, fsl-analyze/src/exact_probabilistic/digital.rs, fsl-replay/src/probability_certificate.rs, fsl-replay/src/probability_certificate/document.rs, fsl-replay/src/probability_certificate/document/key.rs, fsl-replay/src/probability_certificate/document/timed.rs, fsl-replay/src/probability_certificate/document/timed/elapsed.rs"
---
# SR-2026112117241500003347: QSL-2156 PR 3347 code review

## Summary

Ticket: QSL-2156. PR 3347 at b1a346db919fae1734e2edfb3e7d325662e4151f (elapsed-quantile timed product, digital route, proof half of FR-877-AC-1). Method: code-review with rust-review lane, soundness first.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | transform_values walks the whole product with no cancellation poll | fsl-analyze/src/analyze/exact/elapsed.rs:15-97 |

## Verdict

No soundness defect. The checker (fsl-replay timed.rs Watch::Elapsed, timed/elapsed.rs, check_exact_terminal_rewards) enumerates its own digital product from the checked model and claim; it recomputes activation, completion, censoring, the W+1 cap, targets and terminal rewards, takes q, W and c from the claim, fixes Minimum and Bound::AtLeast(0) itself, requires exact rational equality of every state's value with its Bellman minimum, rejects non-unit choice mass, and topological_order returns Err(Cycle) on any cycle. Time-locks and empty choice sets under an active measure are typed Unsupported. Signed values in [-1,1] are admitted only for form elapsed-quantile and are then checked exactly, so an altered value cannot pass. The exact.rs hook sends every elapsed quantile to elapsed::run, which returns NOT_YET for >=, workload, fairness, region route, cycles and a negative minimum; Proved comes only from checker acceptance. No semantic conflict with #3285 (already in the base). No Python or shell.
