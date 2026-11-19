---
id: SR-2026111601185101
title: "Code review of filament-spec-language PR 2732 examples-check target"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; Makefile, xtask/src/examples_check.rs, xtask/src/error.rs, xtask/src/main.rs, xtask/tests/ci_targets.rs, xtask/tests/examples_check.rs, xtask/tests/examples_check_inputs.rs, xtask/src/ci_feature_lane_tests.rs at 7d806ee84616058e51abaab7a191c50d5152bc32"
review_set: subset
evaluated_revision: "7d806ee84616058e51abaab7a191c50d5152bc32"
---

## Summary

Ticket: QSL-2101. Code review with the rust-review lane (rust-style loaded) of `git diff origin/main...HEAD`. No cargo or make was run; the review is by reading.

## Verdict

REQUEST CHANGES. The Makefile only passes variables through. There is no Python. No pin is added outside Cargo. A missing or empty driver or examples variable is refused with exit 2 before anything is built. Adding `examples-check` to the `-o` list in `ci_feature_lane_tests.rs` is justified: that test asserts only the feature lanes, and every other CI_CHECKS lane is already on the list. No `examples-gate` references remain. The error enum follows the Code/thiserror pattern. Two medium defects remain. The trigger reader misses two valid YAML spellings of `push`. A stale sibling driver silently tests git main instead of this checkout.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The trigger reader misses flow-mapping `on: {push: ...}` and a multi-line flow list `on: [` followed by `push` on the next line. Both read as non-push names, so a hosted push trigger passes the test. Fix: fail closed. Return Err when the inline value starts with `{`, has an unclosed `[`, or yields an empty name, and add both spellings to the reader test. | xtask/tests/ci_targets.rs:79-83 |
| FND-002 | medium | `resolve` accepts any driver folder that has `Cargo.toml` and `tests/examples_gate.rs`. A sibling driver older than driver PR 158 ignores FSL_LANGUAGE_DIR, builds fsl from git main and passes. Nothing then checks this checkout, and the AC-3 test still fails as expected because it edits a record. Fix: have the driver gate print a marker line such as `fsl built from language checkout <path>`, and make `examples_check::run` fail when the marker is missing (or capture the output and check it). | xtask/src/examples_check.rs:33-37,80-96 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | At 6e4efe59, xtask/tests/ci_targets.rs:82-97 returns Err for a flow mapping, an unclosed `[` and an empty name. All three are added as refused cases in the reader test. |
| FND-002 | fixed | At 6e4efe59, xtask/src/examples_check.rs check_built_from requires one `fsl built from language checkout <this path>` line and no other path. The new AC-5 (TC-2194) has three stand-in gate tests, each of which can fail. |
