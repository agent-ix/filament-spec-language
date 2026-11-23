---
id: SR-3613003
title: "Criterion-strength review of FR-186-AC-10 (filament-spec-language PR 3613)"
type: SpecReview
analysis: criterion-strength
scope: "agent-ix/filament-spec-language@fcdd6028cb15c8d3ce2e80f481b24ab870512b14; spec/functional/FR-186-check-probabilistic-claim-forms-at-s3.md (FR-186-AC-10)"
review_set: subset
date: '2026-11-23'
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-186
    type: references
---

# Criterion-strength review of FR-186-AC-10 (PR 3613)

## Summary

Ticket: QSL-2979. No Jev client is installed on this host, so this review was
done by hand over the spec text only.

FR-186-AC-10 is `strong`:

- A checker that copied the on-origin entry's `binding` (the old
  basis-equality reading) fails the null-`binding` conjunct.
- A wrong exact value fails the 391/400 conjunct.
- The four conjuncts in one AC follow the repo's convention for compound ACs.

The FR-875 sentence is a "(current)" gap statement with no AC, the same as
the merged forms sentence. The AC's dependency on the unstated `over` is
recorded in SR-3613001 FND-001.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verdict

Clean.
