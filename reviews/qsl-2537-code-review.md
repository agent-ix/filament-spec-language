---
id: SR-2026111920012100000001
title: "Code review of QSL-2537 PR #3081"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; fsl-inspect/src/render.rs (tests module only); QSL-2537"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-865
    type: reviews
  - target: ix://agent-ix/filament-spec-language/FR-298
    type: reviews
  - target: ix://agent-ix/filament-spec-language/FR-286
    type: reviews
---

## Summary

Ticket: QSL-2537. Reviewed PR #3081 at `2a2dbe5f4bb3a219fb14febec16ddde943a6af3d`, diff `origin/main...HEAD`, which changes only expected strings in four `fsl-inspect/src/render.rs` tests (plus one rustfmt-only reflow of an unrelated test's `OutcomeDocument::new` call). The claim is that the renderer is right and the expectations were stale against FR-865 (header line, lowercase `item N: <result> (<category>)` lines) and FR-286 "Diagnostic details" (`details: null`). The claim holds for all four tests. One assertion change in test 3 drops an ordering check, and FR-298-AC-1 now disagrees with what its tagged tests assert.

## Verdict

APPROVE WITH NITS (mergeable). The renderer, `outcome_text` (render.rs:344-470), writes the FR-865 rule-1 header `<operation>: <category> (exit <code>)[ at <stage>]`, then diagnostics (rule 2), then `item <n>: <result> (<category>)[ clause <c>][ <certification>]` lines (rule 3). That matches FR-865's text exactly. `OutcomeDiagnostic` in fsl-replay serializes `details` as an `Option` with no `skip_serializing_if`, so `"details":null` is written. That matches FR-286's "Every diagnostic SHALL carry `details`" and "a refusal whose row has no `details` writes `null`". The JSON pin test (FR-298-AC-4) compares against FR-286's serialization, so adding `"details":null` corrects it and does not loosen it. Tests 2 and 4 kept exact `assert_eq!` and only gained the header line `analyze: refusal (exit 20)`, which is right for an analyze/refusal document with a null `last_stage`. In test 3 the four item assertions are stricter than before, because they now pin the category too. The `starts_with` to `contains` change keeps the diagnostic-line check but loses the check that diagnostics come before items (FND-001). The trace tags were not changed and still fit: TC-783 FR-298-AC-4, AC-1/AC-6, AC-2 and AC-11 each describe what their test asserts. The exception is the "one line" wording in FR-298-AC-1 (FND-002, a spec defect outside this diff, which should not block the merge). `cargo-slot cargo test -p fsl-inspect --lib -- --test-threads=1` passed on the PR head. The Rust lane was reviewed in this same pass: the change is test-only, with no new panics, unsafe code, conversions or I/O.

## Coverage

Examined FR-865 "The text form of an outcome document" rules 1-4, FR-865-AC-1 and FR-865-AC-2; FR-286 "Diagnostic details"; FR-298 body ("A whole outcome document renders as FR-865 states") and FR-298-AC-1, AC-2, AC-4, AC-6 and AC-11; `outcome_text` in render.rs; `OutcomeDiagnostic` serialization in fsl-replay/src/outcome.rs; the four changed tests and their trace tags; and the existing TC-2455 tests (render.rs:2284-2400) for overlap with ordering coverage. Not run: quoin gap-analysis, because no production code changed (the diff is inside `mod tests`).

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | low | Test 3 changed `starts_with("<no-source>: ill_typed: bad\n")` to `contains(...)`, which drops the only check that a diagnostic line comes before the item lines (FR-865 rule 2 before rule 3). No other test pins that order. The equivalent strict form is `starts_with("prove: success (exit 0)\n<no-source>: ill_typed: bad\n")`. | fsl-inspect/src/render.rs:2132; FR-865 | correct-requirement-no-evidence |
| FND-002 | low | FR-298-AC-1 says the text of FR-286-AC-2's refusal "is one line `<path>:<line>:<column>: ill_typed: <message>`". FR-865 rule 1 (and FR-298's own body) puts a header line before it, so the tests tagged FR-298-AC-1 now correctly assert two lines, contrary to the AC's wording. The AC text is stale and should say the diagnostic line follows the FR-865 header. | spec/functional/FR-298-render-outcomes-and-views-from-typed-values.md:169; fsl-inspect/src/render.rs:2052-2086 | wrong-requirement |
