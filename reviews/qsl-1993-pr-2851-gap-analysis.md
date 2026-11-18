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

The computed matrix initially contained no binder for FR-031-AC-7. The behavioral test `quire_context_refusal_preserves_its_diagnostics` in `fsl-command/tests/it/extraction_request_refusals.rs` is present and has a doc-comment trace line, but the active matrix did not recognize it as a binder. The source-inspection catalogue test did not prove that the producer was exercised. The delta review found the new matrix binding is still only source inspection.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-002 | medium | FR-031-AC-7 appears tagged only because a source-inspection test is bound to it | fsl-foundation/tests/code_catalogue.rs:151-169 |

The updated matrix reports FR-031-AC-7 as tagged, with `the_request_reader_produces_the_extraction_refusal_codes` as its only binder. That test reads source with `include_str!` and checks for spellings; it does not exercise the producer. The separate runtime test is still unbound, so the matrix overstates behavioral coverage. Bind the runtime test and remove the criterion tag from the source-inspection test.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | still-open | The runtime behavior test remains unbound. The new matrix binder is a source-inspection test, not evidence that the invalid-context producer is exercised. |
| FND-001 | fixed | after_excerpt: `#[trace("TC-109", "FR-031-AC-7")] #[test] fn quire_context_refusal_preserves_its_diagnostics()` |
| FND-002 | fixed | after_excerpt: `#[trace("TC-109", "FR-031-AC-6")]` |
