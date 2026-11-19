---
id: SR-2026111606410700000001
title: "Code review of filament-spec-language PR 2739 QSL-1976 analyze stubs"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; fsl-analyze/tests/it/analyze_cli.rs, fsl-analyze/tests/it/analyze_coverage_cli.rs at c5914fc3348ab556d53f0ecd8a56256b774633fa"
review_set: subset
evaluated_revision: "c5914fc3348ab556d53f0ecd8a56256b774633fa"
---

## Summary

Ticket: QSL-1976. This is a full review of PR 2739, as part of the QSL-1976 review across filament-driver#159 and filament-examples#33. The PR removes the four `#[ignore = "QSL-1976: ..."]` attributes, from `a_malformed_request_is_a_named_refusal`, `a_refused_program_refuses_at_its_stage`, `analyze_is_routed_and_never_refused_as_not_routed` and `a_binding_refusal_refuses_the_request`. It also changes one assertion from `last_stage` `"check"` to `"S3"` and adds an `ill_typed` code check. Evidence: `cargo test -p fsl-analyze --test it` on the four tests gave 4 passed. No ignore keyed to QSL-1976 remains under fsl-analyze/tests.

## Verdict

APPROVE. All four stubs now run, and they are the library-side evidence for FR-624-AC-1 to AC-3 and FSpec FR-522-AC-2. Changing the stub to `S3` is right. FR-286 owns the outcome document, and FR-286-AC-6 states that an `ill_typed` refusal has `last_stage` S3. FSL writes S3, and a stub must not assert a value that FR-624-AC-2's prose gets wrong. The conflict between the two ACs belongs to the spec lane, which has QSL-2288. The new `ill_typed` assertion makes the test fail on a refusal at the wrong stage with a different code. One low wording nit remains.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The doc comment still says the program "refuses the request at `check`" above an assertion of `S3`. Until QSL-2288 lands, the test traces to FR-624-AC-2, whose text says `check`. Fix: reword the comment to "at the checker stage (S3, FR-286-AC-6; FR-624-AC-2 wording is QSL-2288)". | fsl-analyze/tests/it/analyze_cli.rs:37 |
