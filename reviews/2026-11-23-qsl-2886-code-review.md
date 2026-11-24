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

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | medium | The new FR-128 sentence sets the replay result's `trace_position` to the first false `on each` activation (1 for Echo). The engine's refuted record still writes the counterexample `trace_position` from `TemporalCounterexample::trace_position()`, which is the origin (`Some(0)`) for every non-task subject (fsl-replay/src/model_check/counterexample.rs:399-403, fsl-eval/src/simulation/subject.rs:55). The record therefore says 0 and its own replay says 1. The sentence does not say which `trace_position` it binds, and no criterion states the record's position for this lasso, so FR-128 and the FSpec FR-364 / ADR-018 CX-2 origin rule now disagree in silence. | spec/functional/FR-128-replay-a-model-counterexample.md:245-247; fsl-replay/src/model_check/counterexample.rs:399-403 |
| FND-005 | low | The condition "and the origin activation holds" does not cover a guarded `on each` whose `when` reads false at the origin, which contributes nothing rather than holding. The code reports `at` whenever `at != 0`. Wording such as "when the first false activation is after the origin" would match the code. | spec/functional/FR-128-replay-a-model-counterexample.md:245-247; fsl-replay/src/model_check/replay.rs:2883-2886 |

## Dispositions

Round 1 at `e6a26e6915161c151a51ab05d126d3bd3da1c242`. The focused run gave 57 passed, 0 failed, 0 ignored. `quire validate` on FR-128 and TC-523 reported one existing EARS warning (FR-128 line 343, `before`) and nothing new.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | replay.rs now reports `Some(at.index())` for a lasso whose first false activation is after the origin. The new test `an_on_each_lasso_replays_at_the_activation_where_the_clause_is_false` asserts `Some(1)`. Under the old code it would get `Some(0)`, because `ProtocolTransition` keeps the default `formula_lasso_trace_position` and only `TaskProductTransition` overrides it, so the test would fail. |
| FND-002 | fixed | The doc comment now names the byte-least fair product state reached by send, duplicate and receive. |
| FND-003 | fixed | The test now asserts `at.automaton_state` equals the minimum `automaton_state` among the certificate's states at the locus model state. This expected value comes from the certificate, not from the checker, so the assertion is exact and not circular. It would fail if the checker returned a larger automaton state. It is stricter than FR-338-AC-31, which takes the least among fair states, and it holds for this fixture. |

Round 2 at `b9c1024f31a5ad7f4a315ee8da8111de14efdd0d`.

| Finding | outcome | reason |
| --- | --- | --- |
| FND-004 | rejected | FR-128 explicitly distinguishes the FR-072 replay result's activation `trace_position` from the counterexample record's lasso-origin `trace_position`, and says the two are not compared. The replay result reports position 1 while the record retains position 0; these fields describe different things. |
| FND-005 | fixed | FR-128 now says “When the first false activation is after the origin,” which matches the implementation for guarded `on each` clauses, including a false condition at the origin. |
| Automaton-state note | strengthened | The test now asserts that the least fair model state reached by send, duplicate and receive has exactly one automaton state among all certificate states. Since the locus has that model state and FR-338 chooses its complete product-state minimum among fair vertices, this unique monitor state is also the fair-locus minimum; a non-fair monitor at that model state cannot silently supply the expected minimum. |

At `b9c1024f31a5ad7f4a315ee8da8111de14efdd0d`, before the round-2 assertion change, the focused acceptance commands passed individually: `cargo test --locked -p fsl-analyze --test it analyze_tp3_package::an_echo_on_each_tp3_claim_settles_from_the_package_as_from_source` (1 passed, 0 failed, 0 ignored) and `cargo test --locked -p fsl-analyze --test it protocol_bounded_lasso::an_on_each_tp3_debt_after_the_origin_is_judged_by_the_fair_uncounted_loop` (1 passed, 0 failed, 0 ignored).

After the round-2 assertion change, both focused commands passed individually at `e6b1fa0b1d0cad850dc612ed2fc9200200265ea9`: `CARGO_BUILD_JOBS=4 CARGO_TARGET_DIR=/home/peter/dev/targets/QSL-2886-r2 cargo test --locked -p fsl-analyze --test it protocol_bounded_lasso::an_on_each_tp3_debt_after_the_origin_is_judged_by_the_fair_uncounted_loop` (1 passed, 0 failed, 0 ignored) and `CARGO_BUILD_JOBS=6 CARGO_TARGET_DIR=/home/peter/dev/targets/QSL-2886-r2 cargo test --locked -p fsl-analyze --test it analyze_tp3_package::an_echo_on_each_tp3_claim_settles_from_the_package_as_from_source` (1 passed, 0 failed, 0 ignored).
