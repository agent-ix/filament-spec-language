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

## Dispositions

Round 1, reviewed at frozen head a377cdea54b197ba9427768d6b1d1f627e83eafd.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | The new regression now traces only FR-498-AC-1, which owns declaration permutation; FR-498-AC-2 was removed from both the trace attribute and its doc comment. |

## Disposition evidence

This is the disposition of the original manual AC-to-test check, not a newly completed repository gap audit. Examined FR-498-AC-1 and FR-498-AC-2, the new probabilistic regression's trace/comment/assertions, and QSL-2656's three done-when bullets. FR-498-AC-1 permutation remains the owning criterion for this added regression. The broader AC-2 obligations are not newly claimed by it. The refreshed PR body lists the exact re-recorded reaches key, affected historical expression paths, and newly positional temporal paths.

Plan completion: not assessed

The original limitation remains: computed matrix and repository-wide reverse-gap/stub audit were not run. No full gap-analysis PASS is claimed. This pass checks the existing trace finding and related done-when evidence only. No new finding was observed; frozen-head execution remains pending the lead's Draco TEST-RUN evidence.

```yaml
dispositions:
  - fnd: FND-001
    outcome: fixed
    reason: "The new regression now traces only FR-498-AC-1, which owns declaration permutation; FR-498-AC-2 was removed from both the trace attribute and its doc comment."
    after_excerpt: |-
      /// FR-498-AC-1: same-path expression nodes retain repeated operand
      /// positions, so changing declaration order cannot swap their ordinals or
      /// alter the claims' checked identities.
      #[trace("TC-1288", "FR-498-AC-1")]
```

## Round 1 verdict

All existing findings have explicit outcomes; no finding remains open. Review dispositions are complete. Mergeability remains conditional on final Draco focused-test evidence for the frozen implementation and verbatim custody of these updated artifacts.

