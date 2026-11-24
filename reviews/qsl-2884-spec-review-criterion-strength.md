---
id: SR-3594003
title: "Criterion-strength review of FR-204-AC-18 (filament-spec-language PR 3594)"
type: SpecReview
analysis: criterion-strength
scope: "agent-ix/filament-spec-language@505fbf2d161f07d040d647541f69434cbb4c94dc; spec/functional/FR-204-check-probabilistic-timed-automata-through-digital-clocks.md (FR-204-AC-18)"
review_set: subset
date: '2026-11-23'
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-204
    type: references
---

# Criterion-strength review of FR-204-AC-18 (PR 3594)

## Summary

Ticket: QSL-2884. No Jev (typesafe.ai System One) client is installed on this
host, so the calibrated semantic judgment was skipped. This review was done by
hand over the spec text only.

FR-204-AC-18 is falsifiable:

- **The three tie-sensitive claims.** The b claim (`>= 0.13`), the c claim
  (`>= 0.07`) and the a claim (`<= 0.8`) are each proved under weighted ties
  (7/48, 5/64, 149/192). The same three claims are refuted under even ties
  (11/96, 5/96, 5/6) and under canonical-order ties (0, 0, 1). Each threshold
  separates the correct rule from both wrong rules.
- **The edge masses.** The four masses (1/2, 53/192, 7/48, 5/64) are exact,
  and each differs under either wrong rule.
- **The 1 ms claim.** The `[0 ms, 1 ms]` a claim gives `1/2` under every tie
  rule. It checks the uncontested 1 ms edge and the inclusive bound, but not
  the tie rule. That is a deliberate control, not a weakness.
- **Adverse cases.** The certificate rejection covers an adverse case for
  FR-204.

Weakness kind per AC: FR-204-AC-18 is `strong`. Its remaining limits, the
unstated certificate kind and the untested within-operation and second-pick
SD-3 branches, are recorded as SR-3594001 FND-001 to FND-003 and are not
repeated here. Adverse-case coverage for FR-204's weighted-tie rule is
partial: it covers the winner's marginal only (SR-3594001 FND-002 and
FND-003).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verdict

Clean. Every conjunct of AC-18 that depends on the tie rule is refuted by both
plausible wrong implementations.
