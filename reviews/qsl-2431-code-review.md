---
id: SR-2026111612172400000037
title: "Code review — QSL-2431 protocol message bound"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; QSL-2431; fsl-analyze/src/analyze.rs, fsl-analyze/src/model_check/check.rs, fsl-analyze/src/model_check/liveness.rs, fsl-analyze/src/protocol_analysis/conformance.rs, fsl-analyze/tests/it/protocol_composition.rs, fsl-eval/src/simulation/protocol.rs, fsl-eval/src/simulation/protocol/channel.rs, fsl-eval/src/simulation/subject.rs, fsl-replay/src/lib.rs, fsl-replay/src/model_check/limits.rs, fsl-replay/src/model_check/outcome.rs, fsl-replay/src/model_check/settle.rs, fsl-replay/src/outcome.rs"
review_set: subset
---

## Summary

Reviewed PR #2918 at `feca9ae47770cc0d4f6447e8313f79d29298ad86` against the channel-bound decision, FR-225-AC-6, FR-636-AC-4, TC-2986, changed Rust paths, and focused tests. The core bound behavior is exercised, but bound-cut handling drops an independently found pending-finish counterexample, result reporting mixes channel and store-buffer bound state, and the active AC-4 test is not traced to FR-636-AC-4.

## Verdict

**CONDITIONAL** — three medium findings need fixes: preserve concrete counterexamples, keep bound-reached facts distinct, and tag the active AC-4 test.

## Scope and evidence

- FR-225 channel-bound decision: each channel's occupancy is counted independently; a send at the bound is cut before overflow policy; the RA message bound remains separate; channel-bearing subjects report the selected bound and whether it was reached. Examined for implementation and result-report behavior.
- FR-225-AC-6 (spec/functional/FR-225-bound-the-memory-component-by-modelchecklimits-budgets.md): the `Burst` fixture has two unordered sends on capacity-1 `Order`, reject overflow, and no RA access; bound 1 leaves the second send unexpanded and settles incomplete with `max_messages` reached. Examined.
- FR-636-AC-4 (spec/functional/FR-636-request-protocol-analysis-items-and-replay-supplied-protocol-traces.md): a parallel join with two sends on capacity-1 `Order` settles overflow as FR-572 keys it, and `max_messages` 1 settles incomplete naming it. Examined.
- TC-2986 and both added protocol composition tests were examined. The new max-messages tests are tagged to TC-2986 and FR-225-AC-6; the matrix reports FR-636-AC-4 only through its ignored test.
- Focused tests passed: the two `protocol_composition::max_messages_` tests and the RA message-bound regression passed with one test thread and both required skips. `cargo fmt --all -- --check` reports formatting differences only in unchanged `fsl-analyze/tests/it/analyze_cli.rs` and `fsl-eval/src/temporal/tick_monitor.rs`. The coder's recorded `cargo check` and library Clippy passed; aggregate gate was not run.
- No applicable AssuranceProfile was found for these changed paths.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `pending_finish_in` discards a reachable pending-delivery/finish counterexample whenever any explored state hits a channel or memory cut. A cut on another branch is not evidence that the saved candidate path crossed the bound; the helper is the path that turns this condition into a violation, so this can lose a concrete refutation and leave the run incomplete. Preserve a candidate whose own prefix remains within the bound. | fsl-analyze/src/protocol_analysis/conformance.rs:778 |
| FND-002 | medium | Channel cuts are unioned into `RunStats.bound_limited`, then used as `StatedMemoryBound.reached`. Under TSO this marks `max_store_buffer` reached when only a channel's `max_messages` cut; meanwhile the conformance `max_messages.reached` fallback checks only `limit_reached`, so it can report false when a counterexample settles despite a channel cut. Track and report each bound independently. | fsl-analyze/src/model_check/check.rs:1849; fsl-replay/src/model_check/settle.rs:358; fsl-analyze/src/protocol_analysis/conformance.rs:630 |
| FND-003 | medium | The new active test that exercises FR-636-AC-4's bound-1 incomplete result is tagged only to TC-2986 and FR-225-AC-6. The computed matrix still sees FR-636-AC-4 only in the old ignored test, leaving the AC without active trace-bound evidence. Add FR-636-AC-4 to the test trace. | fsl-analyze/tests/it/protocol_composition.rs:733 |
