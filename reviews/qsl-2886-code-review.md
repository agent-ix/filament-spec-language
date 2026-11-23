---
id: SR-2886001
title: "Code and Rust review of FSL PR #3587: on-each lasso replay and FR-338-AC-31 WitnessFails locus"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language@b9628ab7aecc2d490eb1087672456988030215b0; fsl-replay/src/model_check/replay.rs; fsl-analyze/tests/it/protocol_bounded_lasso.rs; fsl-analyze/tests/it/analyze_tp3_package.rs"
review_set: subset
---
# Code and Rust review of FSL PR #3587

## Summary

Ticket: QSL-2886. The PR removes the `check_formula` guard that turned a lasso violated at a non-origin activation into `ReplayParity`, un-ignores two FR-338-AC-31 tests, and changes the `WitnessFails` locus assertion to the product state reached by `send`, `duplicate`, `receive`.

## Verdict

PASS with one medium and two low findings. Removing the guard matches FR-128 ("false exactly when the formula is false from some activation position"). The evaluator still re-derives the violation itself (`TemporalOutcome::Violated.at` is the first false activation, fsl-eval/src/temporal/outcome.rs:42-46). No other parity check in `check_formula` is weakened. No other replay site assumes position 0: `result.rs:1054` compares `at` with the expected position, `hyper_replay.rs:888` reports `at`, and `finite_replay.rs:472` reads `at` as the cut. The checker (closure.rs:1301-1313) takes the minimum `ProductStateRef` (derived `Ord`, `model_state` bytes then `automaton_state` bytes) over `fair_vertices`, the union of every fair component (fsl-eval/src/simulation/progress.rs:223). That is the FR-338-AC-31 ordering. The test's `assert_eq!` on `model_state` is exact and fails on any other fair model state. No `#[ignore]` was added. Rust style is clean: the `step` closure uses `unwrap_or_else(panic!)` with a message, and there are no new unwraps in production code.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | A lasso violated at a later `on each` activation now settles `Reproduced` with `trace_position: payload.trace_position()`. That is `Some(0)` for every non-task subject (fsl-eval/src/simulation/subject.rs:55), not `at`. The record therefore names the origin, where the clause holds, as the refutation position. Nothing compares the payload's position with the violating activation. hyper_replay.rs:888 reports `at` instead. | fsl-replay/src/model_check/replay.rs:2875-2883 |
| FND-002 | low | The test's doc comment still says `WitnessFails` is "at the product state of the loop". The asserted locus is the byte-least fair state reached by `send`, `duplicate`, `receive`, which is not on the engine's `duplicate`, `lose` loop. | fsl-analyze/tests/it/protocol_bounded_lasso.rs:1384-1386 |
| FND-003 | low | Only `at.model_state` is asserted. FR-338-AC-31 orders by the complete product state and then the auxiliary bytes, so a checker that returns the right model state with a non-least `automaton_state` still passes. | fsl-analyze/tests/it/protocol_bounded_lasso.rs:1479-1484 |
