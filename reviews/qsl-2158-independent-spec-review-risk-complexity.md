---
id: SR-202611210022442313144108533
title: "Independent QSL-2158 risk and complexity review"
type: SpecReview
analysis: risk-complexity
scope: "Six changed cumulative-work/default contract FRs at 3354b395d7739b3376cad577ddec0f4f667f1814"
review_set: subset
---

## Summary

PASS for contract precision. Statistical admission has high implementation risk from concurrency, with concrete contract tests specified as mitigation; no requirement defect is implied by that risk.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | (placeholder) No requirement defect found by scoped risk analysis. | FR-120, FR-191, FR-192, FR-203, FR-255, FR-741 |

## Risk register

| Requirement | Technical risk | Volatility | Driver | Mitigation |
| --- | --- | --- | --- | --- |
| FR-120 | Medium | Low | Shared meter across nested simulator and predicate work | AC-35/36 exact charge and reset probes |
| FR-191 | High | Low | Ordered entitlement under concurrency and cancellation | AC-8 instrumented adverse completion order and workers 1/2/4/64, retaining AC-7 prefixes |
| FR-192 | Low | Low | Existing typed stop settlement | AC-8 parity with EN-5 and named bound/setting |
| FR-203 | Medium | Low | Lazy product construction and key reuse under outer limits | AC-13 cumulative cases and AC-14 materialization probes |
| FR-255 | Low | Low | Independent finite defaults and caller configuration | AC-23 omission and override checks |
| FR-741 | Medium | Low | Native adapter must preserve caller accounting | AC-6 end-to-end default and explicit-bound tests |

## Top hazards

The principal hazards are speculative work escaping physical accounting, planned representations being allocated before outer admission, and caller accounting being silently reset by adapter/evaluation boundaries. FR-191-AC-8, FR-203-AC-14 and FR-120-AC-35/FR-741-AC-6 respectively expose those failures. The companion failure-domain review found no missing contract for these paths. No stronger performance or worker throughput guarantee is inferred; implementation feasibility and actual example costs remain unmeasured.
