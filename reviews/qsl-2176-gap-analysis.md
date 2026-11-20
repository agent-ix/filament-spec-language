---
id: SR-2026111612172400000049
title: "QSL-2176 scoped gap analysis — PR 2967"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; QSL-2176 model intake slice; fsl-semantics/src/check/assemble.rs, fsl-semantics/src/check/assemble/tests.rs, fsl-semantics/tests/it/model_intake.rs"
review_set: subset
relationships:
  - {target: "ix://agent-ix/filament-spec-language/FR-056", type: references}
  - {target: "ix://agent-ix/filament-spec-language/FR-103", type: references}
---

## Summary

Planless, QSL-2176-scoped gap analysis for the model intake changes in PR #2967. The changed float, bare-member and multiplicity behaviors are owned by FR-056; the systems-record test follows W57's FR-103-AC-11. The computed matrix tags each examined acceptance criterion, with no run evidence available; checker-side IEEE rules remain explicitly outside this slice under #2854.

## Verdict

**PASS** — the changed intake behavior has requirement ownership and active test bindings; the checker-side criteria deferred behind #2854 are recorded as context only.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No gaps found (placeholder) | - |

## Coverage

- Reconciliation: `quire matrix --scope . --format json` (quire 0.52.0, engine 0.62.2), static trace matrix; no run evidence read.
- Plan completion: not assessed
- Criteria in this slice: 8 tagged; 0 untagged; 3 tagged only by ignored tests (FR-056-AC-57 through AC-59, deferred behind #2854); no method-without-symbol cases.
- Examined active bindings: FR-056-AC-10, AC-20, AC-43, AC-53 through AC-56, and FR-103-AC-11. The test functions named by the matrix are in `fsl-semantics/src/check/assemble/tests.rs` or `fsl-semantics/tests/it/model_intake.rs` and carry matching trace attributes.
- Reverse gaps and stubs: no new model-intake behavior in the changed production code lacks an owning requirement; the unbounded-multiplicity refusal follows FR-056-AC-10/FSpec FR-322, and the systems record behavior is specified by W57/FR-103-AC-11. The `AssemblyCause` edit changes its existing typed classification and adds no separate behavior.
- Semantic review: skipped; no explicit opt-in was given.
- Scope note: the computed matrix command evaluates the repository's full static matrix; only the QSL-2176 intake criteria listed above were assessed for this review. The separate code-review finding about downstream compilation is recorded in SR-2026111612172400000048.

## Scope examined

- `FR-056-AC-10` — field multiplicity and positive-lower unbounded refusal.
- `FR-056-AC-20`, `AC-53`–`AC-56` — exact Float64/Float32 model-member assembly across fields, operation signatures, tuple positions and union payloads.
- `FR-056-AC-43` — admission of bare native Text, Rational and Decimal members.
- `FR-103-AC-11` — systems Part, Port and Allocation records under W57.
- `FR-056-AC-57`–`AC-59` — read as context only; the related checker/equality behavior remains deferred behind #2854.
