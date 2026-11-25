---
id: SR-967175026302197768900474181941935084825
title: "Code review — filament-spec-language QSL-2832 package schedulability"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; Cargo.toml, Cargo.lock, fsl-analyze/src/analyze/document/package_run/schedule.rs, fsl-analyze/tests/it/analyze_package_schedulability.rs, fsl-analyze/tests/it/task_automaton_lowering.rs, fsl-package/src/emit.rs, fsl-package/src/emit/tests/operation_effects.rs, fsl-semantics/src/check/mod.rs, tests/it/emit_checked_v2.rs; FR-246-AC-4/5, FR-075-AC-23"
review_set: subset
---

## Summary

Initial review was performed against `fc3377f2458a1795838e3c676138eb52d929a29b`. This disposition update covers the release-backed route test fix on the rebased QSL-2832 branch, based on `origin/main` `d4ad338ea0`.

## Verdict

**Disposition update: FND-001 addressed in the test source.** The release-backed package-route test now asserts both the captured `en-6` candidate and the exact later `Pending(TaskProduct)` result. The branch is rebased on `d4ad338ea0`; Cargo.toml and Cargo.lock pin contract-ir to `dc9b220b92390eb8c5999a1b26ac8ff94cfffd33`. The focused rerun is paused by the host build guard; no result is claimed for the new assertion. A fresh independent review remains pending.

## Scope

- Examined FR-246-AC-4, FR-246-AC-5, and FR-075-AC-23 in their owning spec files.
- Inspected package route candidate filtering, schedulability row emission, source requirement construction, relevant integration tests, Cargo pin/lock consistency, and workflow diff.
- No applicable AssuranceProfile was found for these changes.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The release-backed package marker was asserted only in serialized rows; package candidate resolution for `release_backed = true` was unverified. | fsl-analyze/tests/it/analyze_package_schedulability.rs; fsl-analyze/src/analyze/document/package_run/schedule.rs |

## Review Notes

The changed `Cargo.toml` and `Cargo.lock` pin `filament-contract-model` to `dc9b220b92390eb8c5999a1b26ac8ff94cfffd33`, and this branch is rebased onto `origin/main` `d4ad338ea0`.

Before the fix, `cargo test --locked -p fsl-analyze --test it package_schedulability -- --nocapture` passed: 10 passed, 0 failed. The post-fix rerun is pending; the host guard paused its cargo/rustc processes. `quire matrix --scope . --format tsv` completed with exit 0; the three owning ACs remain tagged but matrix-level uncovered due their broader cases.
`cargo fmt --check` failed on widespread existing formatting diffs in unrelated files; the diff contains no workflow changes.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | The package analysis route is exercised for a named release-backed EN-6 request. The test asserts the route captured exactly `en-6` and returned `PackageRunError::Pending` with `PendingFamily::TaskProduct`, proving it reached the later task-product refusal. |
