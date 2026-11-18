---
id: SR-2026111612172400000029
title: "Code review — QSL-1993 / PR #2851"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; PR #2851; fsl-inspect/, fsl-replay/, fsl-semantics/, fsl-command/, fsl-foundation/"
review_set: subset
relationships:
  - { target: "ix://agent-ix/filament-spec-language/FR-031", type: references }
---

## Summary

Static Rust review of PR #2851 against QSL-1993 and the adjacent QSL-2009, QSL-2203, QSL-2015 work found one traceability gap in the new extraction-context refusal test. The coordinator's Cargo hold prevented running focused tests, clippy, or formatting checks.

## Verdict

**CONDITIONAL** — the new context-refusal behavior test is not bound in the computed matrix, and Cargo-based validation remains pending under the build hold.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The new invalid-Quire-context behavior test is not bound to FR-031-AC-7 in the computed matrix | fsl-command/tests/it/extraction_request_refusals.rs:90-92 |

## Coverage

Static review examined the QSL-1993 rendering and refusal-list changes, QSL-2009 variant-case classification/message, QSL-2203 code producers/catalogue coverage, and QSL-2015 observation input-byte normalization. The architecture ruling for QSL-1993 was applied: rendered text may derive line/column, while JSON byte loci retain null line/column and the byte-limit refusal keeps a null locus. Matrix and gap analysis are recorded separately. The optional semantic review was skipped. Cargo tests, clippy, and rustfmt were not run because the coordinator build hold forbids starting Cargo commands.

## Findings detail

### FND-001 — FR-031-AC-7 has no matrix binder

The new test has a `/// Trace: TC-109, FR-031-AC-7` doc comment, but `quoin matrix --repo <worktree> --json` reports FR-031-AC-7 as `untagged`. The adjacent catalogue test uses `include_str!` to look for producer spellings; it does not exercise the invalid-context producer. The behavioral test exists, but its criterion is not represented in the computed traceability matrix. Give the test a tag form recognized by the active matrix and confirm FR-031-AC-7 becomes tagged.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-002 | medium | FR-031-AC-7 is bound to a source-inspection test that does not exercise the invalid-context producer | fsl-foundation/tests/code_catalogue.rs:151-169 |

The canonical trace marker now makes the matrix report FR-031-AC-7 as tagged, but its only binder is `the_request_reader_produces_the_extraction_refusal_codes`. That test uses `include_str!` to search the command source for code spellings and never submits a request to `read_clause_run_request`. The actual behavior test remains unbound. Bind the runtime behavior test and remove the AC-7 binding from the source-inspection test.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | still-open | The canonical tag was added to a different test. `quire_context_refusal_preserves_its_diagnostics` still has no matrix-recognized binding. |
