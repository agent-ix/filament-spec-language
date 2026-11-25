---
id: SR-967175026302197768900474181941935084846
title: "Code review — QSL-3220 uppercase hex diagnostic"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; fsl-cst/src/parser.rs, fsl-cst/tests/it/syntax_messages.rs"
review_set: subset
relationships:
  - { target: "ix://agent-ix/filament-spec-language/FR-397", type: references }
  - { target: "ix://agent-ix/filament-spec-language/FR-762", type: references }
---

## Summary

Reviewed QSL-3220's parser predicate and uppercase-hex diagnostic test at the pinned PR head. The parser now allows uppercase A–F through the constructor-shape check, so the existing parser diagnostic can report the specific lowercase-hex expectation; the targeted regression test and the pre-existing constructor-location test pass.

## Verdict

**CONDITIONAL** — the behavior and regression assertion are sound, but the test also carries a false trace to an unrelated FR acceptance criterion.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The uppercase-hex test is tagged to unrelated FR-762-AC-4 (missing-semicolon recovery), creating a false matrix binding. | fsl-cst/tests/it/syntax_messages.rs:83 |

## Scope

- `QSL-2198 AC-4` (examined): “Uppercase `0xFF` reports `expected a lowercase hex digit`.” The `TC-2198` trace and exact message assertion cover this ticket criterion; the additional `FR-762-AC-4` tag is unrelated.
- `FR-762-AC-4` (examined): A pair of record declarations where the first omits its final semicolon yields one diagnostic and retains the second declaration.
- `FR-397-AC-11` (examined): Malformed Float32/Float64 bit constructors refuse `invalid_syntax`/`invalid-token` at the constructor; mistyped valid constructors refuse `ill_typed`/`type-mismatch`.
- `fsl-cst/src/parser.rs:960-967` and `fsl-cst/tests/it/syntax_messages.rs:83-98` (examined): uppercase A–F are accepted by the shape predicate and the test requires the expected lowercase-hex message while excluding the generic malformed-constructor message.

## Coverage

- Rust review: inspected changed parser logic, test assertions, relevant call path, and repository Rust conventions.
- Regression test: `cargo test -p fsl-cst --test it syntax_messages::uppercase_hex_reports_the_lowercase_digit_rule -- --exact` passed (1 test).
- Diagnostic shape compatibility: `cargo test -p fsl-cst --test it protocol_memory_refinement::a_history_row_refuses_a_non_literal_initial_value_at_its_first_token -- --exact` passed (1 test); uppercase constructor still reports refusal at the constructor token.
- `git diff --check origin/main...HEAD` passed. No production-code stub, panic, boundary, or duplicated-logic issue found in the changed lines.


## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | The test now carries the owning `FR-397-AC-11` trace instead of unrelated `FR-762-AC-4`; focused test passed 1/1. |
