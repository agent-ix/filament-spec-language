---
id: SR-20261107070951249606
title: "Code review — QSL-2193 checked handoff command scope"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; fsl-command/src/command.rs, fsl-command/src/command/wire.rs, fsl-command/src/checked_handoff.rs, tests/it/request_surface_s1.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-665
    type: references
---

## Summary

Reviewed PR #2730's QSL-2193 diff relative to frozen base `peter/qsl-2038-add-a-runner-that-sends-every-corpus-program`. The production path is coherent and the focused Rust gates pass, TC-1748 does not assert all typed-refusal and malformed-input behavior required by FR-665-AC-2/3, and the compile path rereads mutable inputs when creating the handoff.

## Verdict

**CONDITIONAL** — one medium production race and two medium evidence gaps remain.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | TC-1748 never mutates `selection.json` to assert the required `SelectionRefusal` axis, and only asserts that a changed package byte returns some error rather than the strict reader's typed refusal. | tests/it/request_surface_s1.rs:730-753 |
| FND-002 | medium | TC-1748 covers the missing `native_join` case but does not assert refusal for `corpus: "all"` or the unknown `digest` member required by AC-3. | tests/it/request_surface_s1.rs:320-338 |
| FND-003 | medium | Handoff emission rereads source/model files after compile and publishes before comparing its package bytes to the original compile output; a concurrent input change makes compile refuse after leaving the mismatched handoff directory behind. | fsl-command/src/command.rs:792-842 |

## Rust Review

`cargo fmt --check`, `cargo clippy -p fsl-command --all-targets -- -D warnings`, and `cargo deny check` completed successfully. `cargo test -p filament-spec-language --test it handoff` passed (54 passed, 1 ignored, 0 failed). No other changed public API defect was found in the reviewed scope. No CI workflow files changed.
