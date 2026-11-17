---
id: SR-2026111612172400000021
title: "Gap analysis — protocol batch 1 claimed partial slices"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; claimed partial slices of FR-881, FR-883, FR-884; QSL-1137, QSL-1142, QSL-1997, QSL-2215, QSL-2216, QSL-2219, QSL-2220, QSL-2221"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/spec/spec.md
    type: references
---

## Summary

Compared PR #2815's partial claims with the relevant criteria, tagged tests, implementation, and the computed Quoin matrix at head `c38f1ad828195344ae068b2c1e21e9834214b3a4`. The explicitly blocked reader and end-to-end slices are excluded from the claimed completion set; the claimed FR-881 refusal slice still lacks part of its required retained diagnostic data.

## Verdict

**CONDITIONAL** — the partial boundary is documented, but FR-881-AC-1 is only partially implemented and evidenced.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-881-AC-1's operator and key-text retention are absent from the refusal record and are not checked by the tests | FR-881-AC-1; fsl-semantics/src/check/refusal.rs:414-421; fsl-replay/src/spine.rs:1383-1389 |

## Coverage

Plan completion: not assessed.

The computed matrix marks FR-881-AC-1 tagged, with no bound run evidence; FR-884-AC-2 and FR-884-AC-4 tagged, with no bound run evidence; and FR-883-AC-2 tagged, with no bound run evidence. The matrix marks FR-881-AC-2/3, FR-883-AC-1/3/4, and FR-884-AC-1/3/5 untagged. Those broader criteria are not claimed complete by this partial PR; its body explicitly blocks or excludes the full reader parity, capture, and general refusal corpus. Targeted tests run during this review passed, but the Quoin evidence store has no binding for them.

FR-883-AC-2's tagged test exercises present lookup and an absent lookup under `AbsenceMode::Refused`; the broader acceptance's empty/undefined policies remain outside this partial claim. The allInstances test exercises the low-level protocol evaluation context, not an end-to-end guard or capture, as documented in the PR body.

Quoin matrix was run read-only for the repository; no aggregate build, test, or CI gate was run. Targeted review checks were limited to tests for changed slices, targeted rustfmt, and diff whitespace validation.
