---
id: SR-967175026302197768900474181941935084768
title: "Code review of QSL-2928 timed horizon projection"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; PR 3543 head 5adeed81b9787ad27ff24826b3e922db9afc0371; docs/merge-gate-recipe.md, fsl-analyze/src/zone_check/divergence.rs, fsl-analyze/tests/it/zone_search.rs, xtask/src/merge_gate_recipe.rs"
review_set: subset
---

## Summary

Reviewed the four-file Rust and documentation diff in PR #3543. The horizon projection is semantically sound for the bounded-window escape path, and the merge-gate selectors include both dedicated timed controls. One medium coverage finding remains: the new tests assert the primary search graph's state count, not the FR-240 escape-zone count.

Focused reviewer checks passed: `cargo test --locked -p fsl-analyze --test it --no-run`; `cargo test --locked -p fsl-analyze --test it timed_within_` (2 passed); `cargo test --locked -p xtask --lib --no-run`; `cargo test --locked -p xtask --lib exact_and_shared_analyze_paths_include_consumer_integration_filters` (1 passed). No whole suite or crate was run. The ticket-specific coverage check found the regression gap below; a full repository-wide Quoin gap-analysis was not run.

## Verdict

**CONDITIONAL** — the implementation projection is sound, but the required regression does not assert the escape-zone count and would not catch the original expensive behavior.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The 200/201 tests assert primary search states rather than FR-240 escape zones, so restoring the redundant horizon clock would still pass | fsl-analyze/tests/it/zone_search.rs:1483-1486 |
