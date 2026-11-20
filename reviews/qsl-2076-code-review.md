---
id: SR-1467
title: "Code and Rust review of QSL-2076 grammar near-misses"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language PR3146 at 406d58317fed97f48e981e95e728866cc4ca189f; fsl-cst/tests/it/{time_declaration,hybrid_declaration,probabilistic,synthesis_verify,main}.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language
    type: references
---

## Summary

Reviewed only the seven new behavioral tests, their parser assertion helpers and three integration registrations. The real parser rejects the authored wrong keywords, ordering, missing clauses, separators and EOF cases; valid counterparts assert whole production spans and byte-exact rendering. The formatting cleanup was resolved using the author’s in-lane receipt.

## Verdict

**PASS** — the author reports scoped rustfmt completed at 861fde0009d65000cb71bd5b65378b151cdb3ae4 with inputs, assertions and semantics unchanged. This record-only disposition accepts that receipt; no extra code inspection, review sweep or checks ran.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | New hybrid and verification tables use unformatted tuple bindings and loop bodies; apply rustfmt to the changed Rust files. | fsl-cst/tests/it/hybrid_declaration.rs:12; fsl-cst/tests/it/synthesis_verify.rs:45 |

## Coverage

- Rust review loaded rust-style and its implementation/testing rules; the formatting rule requires the repository formatter. README names formatting as part of the local gate.
- Examined all new time, invariant/clock-bound, urgent, hybrid, delay, synthesis and verification tests, including the UTF-8-prefixed span helper and valid production/render controls. The FSpec-prefixed grammar trace tags use the repository's declared namespace.
- No copied external fixtures, production changes, lifecycle code, stub tests, ignored cases or weakened gates occur in this diff.
- Current FSL FR-002, FR-393 and FR-762 were context for parse-only ownership and diagnostic behavior. No S2/solver completion claim is made.
- Coder's seven focused passing functions and scoped CST Clippy are accepted without rerunning them. No build, product probe, matrix or existing-suite check ran in review. One final applicable component gate remains the coder's post-review step.
- Spec review is inapplicable: no specification or plan file changed. Gap analysis is recorded separately as SR-1468.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Author's in-lane receipt reports scoped rustfmt at 861fde0009d65000cb71bd5b65378b151cdb3ae4; tuple-binding and loop-body spacing only, with all inputs, assertions and semantics unchanged. Accepted without another code read, test, lint or review sweep. |
