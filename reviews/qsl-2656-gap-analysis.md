---
id: SR-202611210637402
title: "manual AC-to-test check of PR 3266 for QSL-2656"
type: SpecReview
analysis: gap-analysis
date: 2026-11-21
scope: "agent-ix/filament-spec-language; FR-498-AC-1, FR-498-AC-2, QSL-2656 done-when; fsl-package/src/emit/tests/probabilistic.rs"
review_set: subset
---

## Summary

Ticket: QSL-2656. Manual acceptance-criteria-to-test check (labelled manual: the quoin gap-analysis matrix and reverse-gap steps were not run, by the dispatching brief's narrow-build order). Done-when 1 (example ids equal under both orders) and 2 (focused test, both orders) are met and the test is proven to fail on the old dedup. Done-when 3 (re-record moved ids) is not met; see SR-202611210637401 FND-001.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | New test is tagged FR-498-AC-2 but asserts none of AC-2's statements (row, node and checked claim exposing the same identity; rename effects; identical twins distinct); it backs AC-1's permutation clause only | fsl-package/src/emit/tests/probabilistic.rs:376 |

## Verdict

AC-1 permutation for repeated-operand expressions: covered and mutation-verified. AC-2: tag over-claims. Done-when 3 open (cross-ref SR-202611210637401 FND-001).
