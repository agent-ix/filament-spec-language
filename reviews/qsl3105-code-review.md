---
id: SR-984
title: "Code review (Rust lane) of QSL-3105 PR #3699"
type: SpecReview
analysis: base
scope: "agent-ix/filament-spec-language@0e4877989f7b7adb205add067edeefb32df692fe; fsl-eval/tests/it/main.rs, fsl-eval/tests/it/unit_diagnostic_vectors.rs, fsl-semantics/src/check/assemble/tests.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-091
    type: reviews
  - target: ix://agent-ix/filament-spec-language/FR-142
    type: reviews
  - target: ix://agent-ix/filament-spec-language/FR-480
    type: reviews
  - target: ix://agent-ix/filament-spec-language/FR-514
    type: reviews
---
# SR-984: Code review (Rust lane) of QSL-3105 PR #3699

## Summary

Ticket: QSL-3105. PR #3699, exact head `0e4877989f7b7adb205add067edeefb32df692fe`; scope is `git diff origin/main...HEAD`. Rust review is folded into this artifact. Changed files are two Rust test files and the integration test module registration. The review checked the new vector adapter against TC-187 U11b/e/f/j/k/l/m/q and the named FSL/FSpec criteria, including independent expected payloads, byte spans, and authored order.

No applicable AssuranceProfile was found under `spec/`. The new tests use published vector tokens and literal expected causes, topology edges, names, scale/offset payloads, and source slices. No invented `QuantityCycle` or `TargetCycle` identifiers were found in the changed code. The U11q checks do not assert the document source reference required by FR-480-AC-6 and FR-514-AC-5 (FND-001).

## Verdict

**CONDITIONAL**: one medium test coverage finding remains.

`git diff --check origin/main...HEAD` passed (exit 0). `cargo fmt --check` failed on widespread workspace formatting differences, including reordered imports in the new file; the installed rustfmt also reports many untouched files. No cargo test was run in this review pass; the code lead reported the focused adapter tests and final all-features workspace check passing on this exact head. No source or branch edits were made.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The U11q adapter verifies only the refusal code, typed topology payload, and a span sliced from `texts[0]`; it never asserts that the refusal identifies the source reference of the document that owns the declaration. For `[doc1, doc2]`, it must name doc1; for `[doc2, doc1]`, it must name doc2. Both FR-480-AC-6 and FR-514-AC-5 require this source reference, and no other changed test checks it. The current helper discards source identity from its returned evidence, so a regression that attributes the refusal to the wrong document can pass. Add an assertion through the source-bearing diagnostic/compilation path that compares each refusal's source reference to the expected document reference. | fsl-eval/tests/it/unit_diagnostic_vectors.rs:335-380; fsl-semantics/src/check/assemble/tests.rs:3451-3508; FR-480-AC-6; FR-514-AC-5 |

## Methods

- `code-review` and its Rust lane: completed; FND-001.
- `quoin:gap-analysis`: not applicable by the reviewer method-selection rule because the diff changes no production code (only Rust tests and module registration). The AC-to-test audit was performed manually.
- `quoin:spec-review` and sub-analyses: not applicable; no specification or plan files changed.
- Tracker lookup/posting: Linear unavailable per dispatch context; GitHub CLI unauthenticated. No findings were posted to GitHub.
