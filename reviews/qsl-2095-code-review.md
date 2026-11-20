---
id: SR-1507
title: "Code and Rust review of QSL-2095 checked state-core tests"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; QSL-2095; tests/it/checked_package_state_core.rs, tests/it/bare_recovery_binder.rs, tests/it/bare_recovery_binder/checked_state_core.rs, tests/it/main.rs"
review_set: subset
---

## Summary

Reviewed the four-file test diff at frozen head 4045ba623807db73626192c47504f043917c2a86 for QSL-2095, PR #3181. Applied code-review, rust-review and rust-style to test assertions, real evaluation seams, trace bindings, module registration and fixture lifetime.

## Verdict

**PASS** for this bounded test slice. No actionable code or Rust finding. The three active functions exercise real checking and bytes-backed evaluation with independent expected integers or Booleans. Refusal checks assert classification and exact declaration bytes; recovery checks distinguish receipt multiplicity, totals, emitted calls and activation; the unavailable branch checks the precise binder, typed cause and absent source locus. Assertions fail rather than silently accepting an unexpected outcome. Existing helpers supply real handoff identity/equality rows and admitted checked-package bytes; the new tests introduce no production changes, vendoring, mock semantic implementation, gate weakening or compatibility layer.

Two strict oracles remain ignored and uncounted: QSL-2722 owns state-clause operator loci; QSL-2723 owns conditional-equality witness completion. Their known production failures are explicit tracked work, not represented as passing evidence. A final affected-component gate remains with the author after review.

## Examined scope

Excerpts are verbatim portions of the named units; examined does not mean fully satisfied.

| Unit | Path | Role | Excerpt |
| --- | --- | --- | --- |
| FR-495-AC-4 | spec/functional/FR-495-reach-the-integer-division-zero-divisor-and-state-core-rows-from-fsl-source.md | examined | Under a header profile `filament.state.core/v1` alone, `function f using v(a: Int[0, 9]): Int[0, 9] pure { a div 3 }` refuses `unsupported_construct`/`declaration-form` at the function declaration, naming `filament.state.core/v1`, and no `expression-form` refusal is made for its `div`; |
| FR-520-AC-1 | spec/functional/FR-520-evaluate-state-and-compensation-expressions-through-the-checked-package-evaluation-entry-points.md | examined | For every outcome row of the map, a test over **R** (FR-517) produces the successor outcome: true and false `trigger.guard`, `retry.relation` and `recovery.predicate`; a binder supplied `Unavailable`; a supplied value of the wrong type; a construct the family reports as unsupported; a meter one below measured usage. |
| FR-531 | spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md | examined | This requirement states the one entry that takes a `CheckedPackageV2` and nothing from the source side, what it admits, what it derives from the package's bytes, how it refuses, how its outputs spell identity where the bytes carry no source names or spans, and that it gives the same results and the same charges as the routes that start from source. |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Evidence and limits

Accepted the author's in-lane focused results: three active functions passed; the two strict pending oracles failed at the reported production boundaries before being ignored. The exact command/exit history is preserved in /tmp/qsl-2095-pr-body.md and its named receipts. No test, build, lint or matrix was rerun by this review. This PASS does not assert full FR-495-AC-4 or FR-520-AC-1 coverage, FR-531 fresh-process/API-surface acceptance, or QSL-2095 completion. Other ticket families and accepted prior work remain outside this diff review.
