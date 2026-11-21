---
id: SR-538
title: "QSL-2371 bounded gap audit with unavailable matrix"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; QSL-2371; spec/; source/test tree; changed rational range behavior"
review_set: subset
relationships:
  - { target: "ix://agent-ix/filament-spec-language", type: references }
---

## Summary

Planless gap-analysis attempted the repository matrix and coverage engine, then inspected changed rational behavior for reverse gaps, stubs, and hollow test evidence. Repository assurance remains incomplete; no changed-code gap was found.

## Verdict

CONDITIONAL/INCOMPLETE for an incomplete repository audit, not a demonstrated production defect. A bounded source review cannot establish a repository-wide assurance PASS.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Repository traceability audit is incomplete: bounded matrix and coverage runs timed out without structured results | spec/spec.md:1 |

## Coverage

Plan completion: not assessed

Reconciliation: quire matrix (quire 0.36.2, engine 0.50.2) attempted; timeout exit 124 after 40 seconds. quire coverage likewise timed out exit 124 after 35 seconds. Both JSON outputs are empty. Diagnostics mention duplicate archetypes/inverse edge and an inline-schema advisory; their causal relation to the timeout is unknown. No criterion counts or repository coverage claim can be made. No grep fallback is authorized by these timeouts, and no expensive engine rerun was attempted. Quoin evidence-backed matrix was not run; no run-evidence store was read.

Changed-surface reverse-gap inspection maps literal normalized points and owner interval arithmetic to FR-700 and FSpec FR-146. Both changed production functions have real implementations, no TODO/stub branches, no mocks, and no suppressed gates. New integration assertions use real parse/assemble/check paths and exact refusal tuples. The three FR-700 criteria and external literal rule examined are recorded verbatim in SR-537; their examined roles and bindings also apply here. FSpec-FR-146-AC-9 was context_only because the new test covers only its rational-range slice. Repository-wide source inventory, reverse-gap reconciliation, stale-tag extraction and stub census remain incomplete; this is explicitly a bounded audit, not a completed full repository gap analysis. Semantic review: skipped (not opted in).

Examined audit unit: spec/spec.md repository traceability, role: examined, excerpt: "name: filament-spec-language". Finding FND-001 concerns audit availability only. It does not assert untagged criteria, wrong tags, or a runtime regression.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | deferred | Unavailable assurance limit retained under the parent's explicit focused-only scope instruction: parent reports architecture's equivalent QSL-3310 matrix-unavailability ruling and applies that scope interpretation to QSL-2371. No introduced production defect was found; no source fix, full gate, engine rerun, or new ticket is required for this bounded push. Repository-wide traceability remains unavailable and the verdict remains CONDITIONAL/INCOMPLETE; this is neither fixed nor an engine-backed PASS. |
