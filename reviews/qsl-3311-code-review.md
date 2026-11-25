---
id: SR-2026112506280001
title: "Code review — filament-spec-language QSL-3311 intake mapping"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; fsl-semantics/src/model/intake.rs, fsl-semantics/tests/it/model_intake.rs, spec/functional/FR-056-admit-domain-package-model-declarations.md"
review_set: subset
relationships:
  - { target: "ix://agent-ix/filament-spec-language", type: references }
---

## Summary

Reviewed the frozen QSL-3311 diff at `a0f534f418e40afe86ac62b43d7558fd6c5e2a7f`, including the Rust implementation, its three integration tests, the FSL ownership criteria, and Rust/repository conventions. The new paths and assertions implement the requested missing-name mapping, but the implementation duplicates two private FCD diagnostic-code values as local constants.

## Verdict

**FAIL** — one high P0 duplication finding remains under the mandatory code-review rubric.

## Scope

- Examined `FR-056-AC-5`; the three new tests are trace-tagged to it and assert the refusal code, cause, owner, reference, origin, pointer, and member path.
- Examined `FR-056-AC-42` as context; it owns unresolved unit `typeRef`, which is outside the new dimension/unit graph member paths.
- Context only: FSpec FR-154 mappings for `UNRESOLVED_DIMENSION_REF` and `UNRESOLVED_UNIT_REF` in the pinned FCD dependency.
- Examined both changed Rust files and the frozen diff. No CI workflow or spec file changed. No applicable AssuranceProfile was found for this behavior.
- Focused reproduction and post-fix logs were read; no aggregate workspace gate was run.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Local constants restate FCD's private unresolved dimension/unit diagnostic values, creating a second source that can drift from the dependency's diagnostic definitions. | fsl-semantics/src/model/intake.rs:2275-2276 |

## Review notes

The exact pointer/diagnostic pairs are constrained to `dimensions/{i}/terms/{j}/dimension`, `units/{i}/dimension`, and `units/{i}/target`. Ownership is selected from the package's `dimensions` or `units` collection, and graph identities must be within the selected package, use the expected namespace, and contain one valid artifact-id segment. Tests exercise all three FR-154 cases and verify exact refusal payloads. The focused final log reports 3 passed; the refreshed-main reproduction reports the expected 3 failures before the fix.

The duplication was confirmed by searching the pinned `filament-core-data` checkout: `crates/semantic-ir/src/unit_graph.rs` defines the same strings at lines 9–10 as private constants. The code-review skill classifies re-stating upstream values in local constants as duplication regardless of size or provenance.
