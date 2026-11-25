---
id: SR-967175026302197768900474181941935084858
title: QSL-3309 gap analysis
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language PR #3924; changed Rust behavior and trace matrix for FR-284-AC-4, NFR-141-AC-10/11, FR-752-AC-9/10"
review_set: subset
---

## Summary

Planless focused gap audit for changed behavior in PR #3924. Checked computed matrix output, trace tags, changed implementation, and owning FSL/FSpec criteria. No optional intent-to-code semantic review was run.

## Verdict

FAIL

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Changed AC obligations are reported uncovered by the computed matrix despite tagged test binders; test evidence must be made visible to Quire. | FR-284-AC-4, NFR-141-AC-10/11, FR-752-AC-9/10 |
| FND-002 | high | NFR-141-AC-10's implementation path does not yet connect process entry points to a reused ParserContext. | NFR-141-AC-10 |

## Coverage

Matrix verification: `quoin matrix --repo . --json` exited 1 with CORE_REFUSED request-size refusal. Fallback `quire matrix --scope . --format tsv` exited 0 and marked FR-284-AC-4, FR-752-AC-9/10, and NFR-141-AC-10/11 uncovered while showing tagged test binders for several IDs. Matrix run evidence was unassigned. Reverse-gap and stubs were inspected in changed files; no separate plan was supplied. Plan completion: not assessed. Optional semantic review was skipped because the review brief did not opt in.

Tagged tests include TC-768 at `tools/arch-lint/qualified_core.rs:1357-1402`, NFR-141-AC-10 at `fsl-cst/src/grammar.rs:3782-3823`, and FR-752-AC-10 at `fsl-eval/tests/it/text_domain_limits.rs` (coder-reported focused pass). However, the new ParserContext test checks a field address within one already-created object and does not establish process-entry-point reuse; the TC-768 scanner fixture does not exercise the AtomicU64 blind spot described in the code-review artifact. Therefore tags do not establish criterion-complete evidence.
