---
id: SR-2026111612172400000050
title: "Code and Rust review — QSL-2252 reward evaluator port"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; fsl-semantics/src/check/reward_evaluator.rs; fsl-semantics/src/check/reward_validation.rs; fsl-eval/src/value/expression/mod.rs; fsl-replay/src/spine/lifecycle.rs; fsl-semantics/tests/it/probabilistic_declarations.rs; FR-185-AC-13; TC-3027"
review_set: subset
relationships:
  - { target: "ix://agent-ix/filament-spec-language/FR-185", type: reviews }
  - { target: "ix://agent-ix/filament-spec-language/TC-3027", type: reviews }
---

## Summary

Ticket: QSL-2252. Reviewed PR #3007 at `8091662d1d197586521cb9fbe01eb0206668631b` against `origin/main`. The layer-3 evaluator port, S6a adapter, S3 reward traversal, spine wiring and AC-13 test coverage were examined. Rust review was folded into this method. The focused `fsl-replay` test-target compile is still running; no aggregate gate or `fsl-analyze` test was run.

## Verdict

**FAIL** — one high build finding: `fsl-replay`'s lib test target does not compile because four in-repository callers still use the old two-argument `check` signature. One medium test-intent finding remains: both new recording-evaluator tests hit the fixture's existing `duration` reward before the added `refund` reward, so they do not check the intended AC-13 case.

## Scope examined

- `fsl-semantics/src/check/reward_evaluator.rs` (trait and failure record)
- `fsl-semantics/src/check/reward_validation.rs` (finite domains, charging and refusal construction)
- `fsl-semantics/src/check/mod.rs` (required evaluator at S3 entry points and validation placement)
- `fsl-eval/src/value/expression/mod.rs` (`S6aRewardEvaluator` and the shared graph evaluator)
- `fsl-replay/src/spine/lifecycle.rs` (S3 evaluator wiring)
- `fsl-replay/src/result/wire/value.rs`, `fsl-replay/src/spine/clause/clause_state.rs`, and `fsl-replay/src/spine/clause/tests/witness.rs` (test-target API callers)
- `fsl-semantics/tests/it/probabilistic_declarations.rs:3505-3576`
- `spec/functional/FR-185-declare-random-parameters-workloads-and-rewards.md` (reward evaluator port and FR-185-AC-13)
- `spec/test-cases/TC-3027-s3-evaluates-a-reward-sign-through-the-reward-evaluator-port.md` (both test procedure steps and expected results)

## Rust review

S3 delegates evaluation through the required `RewardEvaluator` port and receives the graph, checked reward, argument vector and shared meter. `fsl-eval` implements the port by invoking the existing graph evaluator. The finite-domain walk charges before each vector and returns evaluation failures as structured refusals. The entry points examined do not provide a default evaluator. No separate interpreter, unsafe code, unchecked input conversion or new dependency cycle was found in the reviewed path.

The recording-evaluator test setup does not isolate the reward named in TC-3027. `service(None, extra)` retains the fixture's `DURATION` declaration; the checker sees that reward before the appended `refund`. The negative evaluator therefore refuses `duration`, and the assertion explicitly expects `duration`. The undefined-evaluator test uses the same setup and likewise refuses the baseline reward. This leaves TC-3027 step 2's intended `refund` case unverified by those S3 port tests.

## Findings

| ID      | Severity | Summary | Refs |
| ------- | -------- | ------- | ---- |
| FND-001 | high | Four `fsl-replay` test callers still use the old `check` signature, so its lib test target fails to compile. | fsl-replay/src/result/wire/value.rs:55-58, 189-192; fsl-replay/src/spine/clause/clause_state.rs:2114-2117; fsl-replay/src/spine/clause/tests/witness.rs:2689-2692 |
| FND-002 | medium | Recording-evaluator tests refuse the fixture's `duration` reward instead of `refund`. | fsl-semantics/tests/it/probabilistic_declarations.rs:3509-3532, 3559-3575; FR-185-AC-13 |

## Verification

- Read PR diff and traced the evaluator call path from `PackageDeclarations::check` through the spine and S6a adapter.
- `quoin matrix --repo . --json` — completed; FR-185-AC-13 lists the analyzer S6a test plus both recording-evaluator tests as binders. The matrix reports no run evidence, which this static query does not establish.
- Before the fix, `cargo test -p fsl-replay --lib --no-run` failed with E0061 at the four refs above because the required third `&dyn RewardEvaluator` argument was missing.
- At the fix head, `cargo test -p fsl-replay --lib --no-run` through `~/dev/worktrees/locked-build.sh` with `CARGO_HOME=/home/peter/.cache/qsl-2252-cargo-home`, target `/home/peter/.cache/qsl-2252-target`, debuginfo off and incremental off — passed.
- At the fix head, `cargo test -p fsl-semantics --test it probabilistic_declarations::s3_` with the same locked-build settings — passed, 2 tests (negative-result stopping and undefined-result preservation).

## Dispositions

Round 1 checked against PR head `cfa4a3ccd845e60b3e2c01d8cf4008fbc1fb3e81` and its fix commit.

| FND | outcome | reason | after_excerpt |
| --- | --- | --- | --- |
| FND-001 | fixed | The four fsl-replay test callers now pass the required S6a evaluator; the focused lib test-target compilation passed at this head. | `.check(\n    CheckMode::Linked,\n    filament_semantic_value::checking::CheckingLimits::default(),\n    &fsl_eval::value::S6aRewardEvaluator,\n)` |
| FND-002 | fixed | The tests replace the baseline duration declaration with refund and assert that both the evaluator calls and negative refusal identify refund. | `Some((DURATION, "reward refund on Server::attempt = d - 3;")),` |

