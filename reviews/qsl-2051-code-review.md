---
id: SR-1534
title: "Code review of source catalog producers"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; QSL-2051; Cargo.lock; fsl-conformance/Cargo.toml; fsl-conformance/cases/catalog-source-structural.native; fsl-conformance/tests/it/catalog_coverage.rs; fsl-conformance/tests/it/main.rs"
review_set: subset
reviewed_commit: d71733b414ac6caf0cac66a7b84c4b25ba0cbc6d
pr: 3198
pr_url: https://github.com/agent-ix/filament-spec-language/pull/3198
module: spec-artifacts-process@0.2.0
reviewer_model: unreported
---

## Summary

Ticket: QSL-2051. Reviewed the Rust integration-test adapter, its real source cases, the authoritative catalog dependency, and test registration. No code defect found in the delivered value-and-structural subset. This is not complete catalog or node-form qualification.

## Examined Scope

| Unit | Role | Excerpt |
| --- | --- | --- |
| source_cases macro | examined | `let package = compile_and_read(&case); assert_producers(&case, &catalog_identities(), &package);` |
| compile_and_read | examined | `spine::parse`, `spine::select`, `spine::check`, then `fsl_package::read_evaluation_package` |
| omission boundary | examined | `read.emission().omitted().is_empty()` |
| operation dispatch | examined | `node["node_tag"] == "expression" && node["body"]["term"] == "application"` |
| catalog membership | examined | `ProducerMismatch::UnpublishedOperation` and `ProducerMismatch::MissingOperation` |
| node-form assertions | examined | `node["node_tag"] == *tag && node["semantic_form"] == *form` |
| added-entry control | examined | `new_gaps.difference(&original_gaps)` must contain precisely the new identity |
| altered-operation control | examined | Compiled Boolean source must reject the demanded `filament.op.integer.add` identity |
| pending strict qualification | examined | `#[ignore = "QSL-2051: source adapters for the remaining catalog operations and node families are not complete"]` |
| pending reader regressions | examined | Sequence index requires successful admission under IR-1491; composite Float64 equality requires successful admission under IR-1320 / STD-654 |
| structural source fixture | examined | `let n = project(p) in increment(n)` |
| dependency and registration | examined | Existing pinned catalog crate becomes a dev dependency; `mod catalog_coverage;` registers the integration module |
| repository instructions | context_only | Preserve authored identity and explicit unsupported/incomplete results; Rust integration tests use the existing `tests/it` module convention |
| FSpec FR-322 operation families | context_only | The normative operation-family rules govern the published machine-readable catalog |

## Verdict

**PASS** for the reviewed diff. Source is parsed, model-selected and checked through public APIs; strict package admission succeeds before producer assertions execute. Assertions inspect actual admitted application bodies and node family/form pairs rather than source substrings, fabricated packages or form-built producer examples. Each active semantic case fails if its required operation disappears or strict admission breaks. The wrong-operation control compiles real source before checking the mismatch.

The catalog is consumed from its authoritative dependency; its entries and expected packages are not copied. The authored source fixture is local test input, not a copied external artifact. The changed Rust code uses deterministic bounded inputs, contains no unsafe code, spawns no tasks and changes no production or CI gate policy. No applicable AssuranceProfile was found in the repository specification.

Six known reader regressions keep strict positive expectations and named owners, remain ignored and untagged, and are excluded from passing coverage. The ignored full-catalog assertion is not represented as complete node-form qualification. Broad FR-350 and FSpec-FR-322 references establish provenance; this review does not infer individual acceptance-criterion closure from them.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Validation Evidence

Accepted the author's focused receipts for 41 source cases and two runner controls, scoped Clippy, exact-file formatting and whitespace checks. Original compile and adapter failures, the failed-only repairs, and the actual reader failures remain preserved. No test, build, lint, matrix, upstream probe or measurement was repeated during this review. The code lead reports the single final affected-component gate passed 43 tests with zero failures and seven explicitly ignored cases; that receipt is accepted without re-execution.

The accepted source is now public PR #3198. This update supplies real PR metadata and Linear review records only; it does not repeat source review or runtime checks. The code lead reports the final affected-component gate passed 43 tests with zero failures and seven explicitly ignored cases.
