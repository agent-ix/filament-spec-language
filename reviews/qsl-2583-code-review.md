---
id: SR-2026111919340478491681419
title: "Code review — filament-spec-language Decimal checked types"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; fsl-semantics/src/check/assemble.rs, fsl-semantics/src/check/lowering/model.rs, fsl-semantics/src/check/source_model.rs, fsl-semantics/src/model/domain_package.rs, fsl-semantics/src/model/index.rs, fsl-semantics/src/model/intake.rs, fsl-semantics/tests/it/decimal_value_types.rs, tests/corpus/operations/parameters/pending/README.md, tests/it/source_corpus.rs; FR-056-AC-43, FR-056-AC-63, FR-056-AC-66, FR-056-AC-67, FR-056-AC-68, FR-056-AC-71, FR-056-AC-72"
review_set: subset
relationships:
  - type: references
    target: ix://agent-ix/filament-spec-language/FR-056-admit-domain-package-model-declarations
---

## Summary

Reviewed PR #3138 at `0b002d1fe3caf0550050e5d88db69f67d33d28d0`, including the Decimal intake, checked-type propagation, tests, and Rust boundary behavior. The Decimal paths are substantially implemented, but recursive alias resolution has no depth bound and can exhaust the intake stack on a sufficiently long valid alias chain.

## Verdict

**CONDITIONAL** — one medium resource-bound defect needs resolution; the assigned focused test/build lane was not run by this reviewer.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | Decimal alias resolution recursively follows caller-controlled type chains without a depth limit, allowing stack exhaustion during package intake | fsl-semantics/src/model/intake.rs:5402-5443 | implementation-bug-despite-evidence |

## Review Notes

`decimal_type_for_identity` recursively follows each alias target and allocates a `Vec<String>` entry per level. The chain is represented by a flat `types[]` array, so the package parser's nesting-depth behavior does not bound this recursion. Package input is byte-limited, but no alias-chain maximum is enforced here; a sufficiently long valid chain can overflow the thread stack rather than returning a refusal. Resolve chains iteratively or enforce a documented bounded depth before recursion. The repeated full-array identity lookup also makes repeated alias walks unnecessarily expensive.

The direct Decimal reader validates policy precision/scale and constraint coefficient bounds, and carries `DecimalType` through the checked domain, lowering, and source model. I did not identify another concrete implementation defect in the reviewed diff. No cargo commands were run per the dispatch instruction; focused checks remain with the lead's combined lane.

## Verdict Scope

- Examined FR-056-AC-43, AC-63, AC-66, AC-67, AC-68, AC-71, and AC-72 and the changed source/test files listed in `scope`.
- Rust style and safety references were applied. No applicable AssuranceProfile was found.
- The workspace CI configuration was not changed.
