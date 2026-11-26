---
id: SR-967175026302197768900474181941935084869
title: "Code review — merge-gate recipe named filters and consumer selection"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; docs/merge-gate-recipe.md, xtask/src/merge_gate_recipe.rs"
review_set: subset
---

## Summary

Reviewed PR #3983 for QSL-3427, including the merge recipe rules, fallback routing, tests, and documentation. The named-filter changes are clear, but low-level crate-root and manifest paths still bypass the downstream consumer routes introduced for source-module paths.

## Verdict

**CONDITIONAL** — a medium routing gap remains for low-level crate roots and manifests.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Low-level crate-root and manifest fallbacks omit downstream consumers | xtask/src/merge_gate_recipe.rs:2063 |

## Verification

- `cargo clippy --workspace --all-targets --all-features -- -D warnings` — passed (1.03 min).
- `cargo test --locked -p xtask --lib -- merge_gate_recipe::` — 36 passed in the lead's run on this head.
- `cargo test --locked -p xtask` — failed: the full suite reported `ci_spec_validation_tests::spec_validation_selects_npm_launcher_when_native_quire_is_first` failing at `xtask/src/ci_spec_validation_tests.rs:197` (`native_quire.exists()`), then the test process aborted on stack overflow. The focused failing test reproduces the missing-fixture assertion.
- `cargo fmt --all -- --check` — failed on extensive formatting differences across the workspace; the reviewed source file's focused `rustfmt --edition 2021 --check xtask/src/merge_gate_recipe.rs` passed in the lead's run.
- `cargo deny check` — exit 3; `advisories` and `bans` failed, while `licenses` and `sources` passed.

## Review scope

Inspected the changed paths, `plan_path`, `apply_rule`, `apply_crate_fallback`, `FOUNDATION_CONSUMERS`, `SOURCE_CONSUMERS`, `CST_CONSUMERS`, `FORMS_CONSUMERS`, `PACKAGE_CONSUMERS`, crate-wide fallback tests, and the repository Rust conventions. No workflow file is changed by the PR.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Added named consumer filters for low-level crate roots/manifests and a test covering foundation, source, CST, forms, and workspace lockfile paths; the focused test passes. |

FND-001 `after_excerpt` (verbatim from `xtask/src/merge_gate_recipe.rs`):

```rust
} else if let Some(consumers) = match package {
    "fsl-foundation" => Some(FOUNDATION_CONSUMERS),
    "fsl-source" => Some(SOURCE_CONSUMERS),
    "fsl-cst" => Some(CST_CONSUMERS),
    "fsl-forms" => Some(FORMS_CONSUMERS),
    _ => None,
} {
    add_named_library_modules(workspace_root, recipe, package)?;
    apply_it_filters(workspace_root, consumers, None, recipe)?;
```
