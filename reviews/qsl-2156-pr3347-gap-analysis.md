---
id: SR-2026112117241500013347
title: "QSL-2156 PR 3347 gap analysis"
type: SpecReview
date: 2026-11-21
scope: "agent-ix/filament-spec-language; spec/functional/FR-877-certify-elapsed-quantile-verdicts.md, fsl-analyze/tests/it/timed_product_certificate.rs, fsl-analyze/tests/it/digital_clocks.rs"
---
# SR-2026112117241500013347: QSL-2156 PR 3347 gap analysis

## Summary

Ticket: QSL-2156. PR 3347 at b1a346db919fae1734e2edfb3e7d325662e4151f. Method: gap-analysis (manual AC-to-test check against FR-877, FR-920, FR-921, FR-498; quoin matrix not run per brief: no builds).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | AC-1 test never asserts the initial value 2/5 or kind exact | fsl-analyze/tests/it/timed_product_certificate.rs:677-688 |
| FND-002 | medium | FR-877 says completion is at the first later observation; code, FR-196 and FSpec FR-407 complete at the activating observation; censored reward unstated | spec/functional/FR-877-certify-elapsed-quantile-verdicts.md:50-54 |
| FND-003 | low | Altered-value assertion duplicated in the new test and the ignored AC-2 test | fsl-analyze/tests/it/timed_product_certificate.rs:690-729 |
| FND-004 | low | Ignore reasons on the remaining FR-877 tests are stale | fsl-analyze/tests/it/timed_product_certificate.rs:712,733,751 |

## Verdict

Fix round needed (FND-001, FND-002). The removed Elapsed half of a_timed_form_shape_cause_carries_the_claim_member_path is a correction: FR-498-AC-8 names no elapsed case and FR-498-AC-3 says an elapsed-time claim settles an exact verdict, never a TimedFormShape cause. The un-ignored FR-498-AC-3 elapsed test only builds the product, which is now true.
