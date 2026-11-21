<!-- reviewer repo=agent-ix/filament-spec-language visibility=public quoin=0.36.7 module=spec-artifacts-process@v0.29.2 id=SR-2026112117241500013347 method=gap-analysis lang=rust pr=filament-spec-language#3347 model=claude-opus-5-5 run=5504B57D-4A0B-4197-9077-AD2D087328E9 -->

Gap analysis of PR 3347 at b1a346db919fae1734e2edfb3e7d325662e4151f (manual AC-to-test check; no matrix run, no builds per brief). Fix round needed for FND-001 and FND-002. The removed Elapsed half of a_timed_form_shape_cause_carries_the_claim_member_path is a correction (FR-498-AC-8 names no elapsed case; FR-498-AC-3 says elapsed claims never take a TimedFormShape cause).

| FND | Severity | Check | Summary |
| --- | --- | --- | --- |
| FND-001 | medium | test-intent | AC-1 test never asserts the initial value 2/5 or kind exact |
| FND-002 | medium | ambiguous | FR-877 says completion is at the first later observation; code, FR-196 and FSpec FR-407 complete at the activating observation (M = 0); censored reward unstated |
| FND-003 | low | trace | Altered-value assertion duplicated in the new AC-2 test and the ignored AC-2 test |
| FND-004 | low | other | Ignore reasons on the three remaining FR-877 tests are stale |

+++ [reviewer data]

```yaml
clean: false
scope:
  - {id: FR-877-AC-1, path: "spec/functional/FR-877-certify-elapsed-quantile-verdicts.md", role: examined, excerpt: "Over Retx (guard self.x >= 1 ms) under every scheduler, with M = elapsed from holds(not c.late) until holds(c.late) within 4 ms: quantile 0.5 of (M) <= 2 ms settles proved with a digital exact document of initial value 2/5 that the checker accepts."}
  - {id: FR-877-AC-2, path: "spec/functional/FR-877-certify-elapsed-quantile-verdicts.md", role: examined, excerpt: "quantile 0.5 of (M) <= 1 ms settles refuted, the transform's minimum being -1/2; AC-1's item with its initial value set to 1/2 settles inconclusive, CertificateRejected, rule InequalityFails."}
  - {id: FR-877-AC-3, path: "spec/functional/FR-877-certify-elapsed-quantile-verdicts.md", role: examined, excerpt: "quantile 0.5 of (M) >= 1 ms settles proved with a region upper document of initial value -1/2 that the checker accepts."}
  - {id: FR-877-AC-4, path: "spec/functional/FR-877-certify-elapsed-quantile-verdicts.md", role: examined, excerpt: "quantile 0.9 of (M) >= 2 ms settles refuted ... an upper document of initial value 0 for it is rejected InequalityFails at the initial state."}
  - {id: FR-877, path: "spec/functional/FR-877-certify-elapsed-quantile-verdicts.md", role: examined, excerpt: "The measure SHALL activate at the first observation where A holds, complete at the first later observation where B holds, and be censored when the elapsed clock passes W first; the elapsed clock SHALL be reset at activation, advance with every delay while active, freeze at completion or censoring, and be capped at W + 1."}
  - {id: FR-498-AC-8, path: "spec/functional/FR-498-*.md", role: examined, excerpt: "A TimedFormShape cause on the claim's form carries body [form] and empty formulas."}
  - {id: FR-498-AC-3, path: "spec/functional/FR-498-*.md", role: examined, excerpt: "An elapsed-time claim and a punctual-interval claim over a timed subject each settle an exact verdict (FR-204), never unsupported and never a PunctualInterval cause"}
  - {id: FR-196, path: "spec/functional/FR-196-*.md", role: context_only, excerpt: "At model position zero, a measure is inactive unless A holds; when A holds it is active, or complete with magnitude zero when B also holds. ... Censored means greater than every finite threshold, not an arbitrarily large finite reward."}
findings:
  - fnd: FND-001
    severity: medium
    confidence: high
    method: gap-analysis
    check: test-intent
    artifact_id: FR-877-AC-1
    related_ids: [TC-2494]
    path: fsl-analyze/tests/it/timed_product_certificate.rs
    lines: "677-688"
    excerpt: |-
      quantile 0.5 of (M) <= 2 ms settles proved with a digital exact document of initial value 2/5 that the checker accepts.
    finding: "elapsed_quantile_at_most_is_certified asserts Proved, form, route and acceptance but not the initial value 2/5 nor kind exact; a wrong but accepted non-negative minimum passes. Add assert_eq!(proved_value(&record), ratio(2, 5)); and assert_eq!(doc[\"kind\"], \"exact\"); after the Proved match."
  - fnd: FND-002
    severity: medium
    confidence: high
    method: gap-analysis
    check: ambiguous
    artifact_id: FR-877
    related_ids: [FR-196, FSpec-FR-407, FR-877-AC-1]
    path: spec/functional/FR-877-certify-elapsed-quantile-verdicts.md
    lines: "50-54"
    excerpt: |-
      The measure SHALL activate at the first observation where A holds, complete at the first later observation where B holds, and be censored when the elapsed clock passes W first
    finding: "first later observation contradicts the implementation (producer deadline_start/control_after and checker Watch::observe complete with M = 0 when B holds at the activating observation) and FR-196 / FSpec FR-407 (a <= b, complete with magnitude zero when B also holds). FR-877 is silent on a censored run's reward. Replace with: complete at the first observation at or after activation where B holds, with M = 0 when B holds at the activating observation (FR-196, FSpec FR-407); and add: A censored measure exceeds every finite threshold, so its terminal reward is -q for both [M <= c] and [M < c]."
  - fnd: FND-003
    severity: low
    confidence: medium
    method: gap-analysis
    check: trace
    artifact_id: FR-877-AC-2
    related_ids: [TC-2494]
    path: fsl-analyze/tests/it/timed_product_certificate.rs
    lines: "690-729"
    excerpt: |-
      quantile 0.5 of (M) <= 1 ms settles refuted, the transform's minimum being -1/2; AC-1's item with its initial value set to 1/2 settles inconclusive, CertificateRejected, rule InequalityFails.
    finding: "The new elapsed_quantile_altered_transform_value_is_rejected repeats the tamper block of the ignored elapsed_quantile_at_most_refutes verbatim. Delete the tamper block from the ignored test so it covers only the refutation half; keep the new test as the sole AC-2 certificate-half evidence. AC-2 stays open until the refutation test runs."
  - fnd: FND-004
    severity: low
    confidence: high
    method: gap-analysis
    check: other
    artifact_id: FR-877-AC-3
    related_ids: [FR-877-AC-2, FR-877-AC-4]
    path: fsl-analyze/tests/it/timed_product_certificate.rs
    lines: "712, 733, 751"
    excerpt: |-
      quantile 0.5 of (M) >= 1 ms settles proved with a region upper document of initial value -1/2 that the checker accepts.
    finding: "The ignore reasons still say elapsed-time quantile claims are not routed to the timed product yet (QSL-2158 source interface), which this PR makes false. Use: line 712 refutation of a <= elapsed quantile needs a timed witness replay; lines 733 and 751 the region route for >= elapsed quantiles is not implemented yet."
bindings:
  - {test_id: elapsed_quantile_at_most_is_certified, ac_id: FR-877-AC-1, trace: correct}
  - {test_id: elapsed_quantile_altered_transform_value_is_rejected, ac_id: FR-877-AC-2, trace: correct}
  - {test_id: digital_elapsed_key_encodes_the_measure_clock_and_rejects_a_second_clock, ac_id: FR-920-AC-1, trace: correct}
  - {test_id: timed_form_shape_names_the_selected_elapsed_claim_among_equal_names, ac_id: FR-498-AC-3, trace: correct}
```

+++
