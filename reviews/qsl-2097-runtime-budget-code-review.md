---
id: SR-1517
title: "Code and Rust review of population admission budget cases"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; QSL-2097; fsl-semantics/tests/it/model_population.rs"
review_set: subset
---

## Summary

Reviewed the two added runtime admission tests at frozen head 8d6128e42ca9729f796a365b26be3f85a170daa7, including Rust idioms, independent budget arithmetic, typed outcomes and trace bindings. No findings.

## Verdict

PASS for this confined diff. Four raw records must consume four member slots despite duplicate identity collapse. Three member admissions plus three candidate-width scans cost 3 + 3 × 3 = 12 work units; bound 11 must deny the last scan after 9 units. These expectations follow the fixture arrangement rather than observed implementation output. Fully funded valid and typed refusal controls expose early inspection, collapsed-record accounting, short-circuit scan pricing and lost refusal-locus defects.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

- Examined NFR-012-AC-8, spec/non-functional/NFR-012-bound-model-normalization-and-population-admission.md:92: “At most the selected ceiling (default 100000) per admission”.
- Examined NFR-012-AC-9, spec/non-functional/NFR-012-bound-model-normalization-and-population-admission.md:93: “At most the selected ceiling (default 16777216 work units) per admission”.
- Examined the equal-key/equal-content branch of FR-084-AC-4, spec/functional/FR-084-admit-closed-populations-and-resolve-lookup.md:200: “Given two runtime member records sharing universe and object identity but differing most-specific type, admission refuses conflicting-identity and admits no binding; given two records with equal key and equal content, admission collapses them into one member.” The conflicting-identity branch is outside this diff's claim.
- The new tests call the real admission API over existing domain-package fixtures. Exact incomplete records assert limit domain, bound, consumed count, denied charge and charge point. Funded refusal controls assert stable code and typed cause including member/type or object/field identity. No mocks, source-text assertions, copied artifacts, gate changes or new production API.
- Author reports each exact feature-enabled test passed separately: 1 passed, 0 failed, 1711 filtered, exit 0; formatter exit 0. Accepted without rerun. The initial feature-omitting run selected zero tests and is invalid evidence. No green lint claim; known unchanged lint failures remain inherited. Final affected component gate remains with coder/lead.
- Review used code-review and rust-review with rust-style testing and implementation rules. Exact inherited Codex model variant is unknown. No broader historical ticket completion is claimed.
