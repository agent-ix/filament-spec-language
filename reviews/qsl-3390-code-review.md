---
id: SR-967175026302197768900474181941935084864
title: "Code review — filament-spec-language hybrid replay class walk"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; fsl-replay/src/hybrid_replay/class.rs, tests/it/hybrid_class_recursion.rs, tests/it/main.rs; NFR-141-AC-27"
review_set: subset
relationships:
  - { target: "ix://agent-ix/filament-spec-language/NFR-141", type: references }
---

## Summary

Reviewed PR #3958 at head `519c072bdba7b21ee0e31bc64251aa8140f5ec1d` for QSL-3390 and QSL-3394. The stats gating is structurally sound, but the moved child-process test can mistake an inherited environment variable for its child-mode signal.

## Verdict

**CONDITIONAL** — one medium test-isolation defect remains.

## Assurance Context

No applicable `AssuranceProfile` was found under `spec/assurance/`. The repository has measurement artifacts, but none is a review-policy profile.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The recursion probe treats any inherited `FSL_RECURSIVE_CLASS_WALK_CHILD` value as child mode, so an ambient value can run the stack-overflow path in the parent test process instead of isolating it. | tests/it/hybrid_class_recursion.rs:10 |

## Review Notes

- Rust review folded into code-review. Inspected the cfg-gated stats fields and call paths, child-process test, integration-test registration, repository Rust instructions, and relevant NFR-141 criterion.
- The `NFR-141-AC-27` trace binding is present on the new test and both existing class-walk tests. No CI workflow changed.
- `git diff --check` passed. Cargo gates were not run because the dispatch instructed the reviewer not to use the coder's active Cargo target. No Cargo command was invoked.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | The parent now invokes a dedicated ignored child-mode test using `--ignored --exact`; child-mode selection no longer depends on inherited environment. |
