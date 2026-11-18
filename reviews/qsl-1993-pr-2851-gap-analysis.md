---
id: SR-2026111612172400000030
title: "Gap analysis — QSL-1993 / PR #2851"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; PR #2851; FR-031, FR-255, FR-298, FR-360; changed tests and source"
review_set: subset
relationships:
  - { target: "ix://agent-ix/filament-spec-language/FR-031", type: references }
---

## Summary

A manual, planless acceptance and matrix audit of the changed diagnostic, variant-case, extraction-code, and observation-limit paths found FR-031-AC-7 untagged in the computed matrix. No spec or plan files changed; plan completion was not assessed.

## Verdict

**FAIL** — the computed matrix reports an untagged criterion for the new invalid-context producer path.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-031-AC-7 has no tagged test binder for the invalid-context producer | FR-031-AC-7 |

## Coverage

- Reconciliation: `quoin matrix --repo <worktree> --json` (Quoin 0.36.4; Quire 0.51.0 / engine 0.62.2); repository traceability model loaded.
- FR-031-AC-7 is `untagged`; its new behavior test has a doc-comment trace tag, while the changed catalogue test uses source inspection.
- QSL-1993 rendered-line/column behavior and null byte-limit locus were inspected against the architecture ruling; QSL-2009 retains `type-mismatch` and names the variant type/case; QSL-2203 retains both ruled codes and adds producers/catalogue coverage; QSL-2015 normalizes to `observation.input_bytes` and remains partial pending driver parity.
- Plan completion: not assessed
- Semantic review: skipped
- Cargo-based tests, clippy, and rustfmt: not run under the coordinator build hold.

## Findings detail

### FND-001 — FR-031-AC-7 has no tagged test binder

The computed matrix contains no binder for FR-031-AC-7. The behavioral test `quire_context_refusal_preserves_its_diagnostics` in `fsl-command/tests/it/extraction_request_refusals.rs` is present and has a doc-comment trace line, but the active matrix does not recognize it as a binder. The source-inspection catalogue test does not prove that the producer is exercised. Add a matrix-recognized trace binding and confirm the criterion is tagged.
