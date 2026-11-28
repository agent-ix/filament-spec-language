---
id: SR-967175026302197768900474181941935084874
title: "code-review review of PR4031 shared max_states budget test over two decided bindings"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; fsl-analyze/tests/it/liveness.rs; PR4031 head f1e7e470bac922efb902424692613f03e4778d58; ticket QSL-3413"
review_set: subset
---

## Summary

Ticket: QSL-3413. The PR rewrites one stale liveness test so that both bindings are decided and both model-only passes run, and asserts `Holds` at `max_states` 32 and at the default limits and `Stopped{ResourceExhausted, {MaxStates, 31}}` at 31, each with 0 product states and edges. The expected values follow from the fixture and FR-126, and the test fails when the shared budget is not charged across bindings. One low trace-text gap: the test is now tagged `TC-1085`, but TC-1085's procedure has no step for it.

## Verdict

**CONDITIONAL** — one low finding; no wrong expectation, no lost coverage that must come back in this PR. Mergeable once FND-001 is fixed in this PR (spec-only edit) or the lead records it as accepted-no-change.

Hand derivation. The fixture is the bounded `Counter` document (`value` in `0..=3`, `inc` requires `value < 3`, its postcondition binds only the receiver and its frame grants every counter's `value`), with counters `a` and `b` both 0. TC-521's explicit enumeration (`model_check.rs` `pairs()`) gives 16 reachable states and 96 edges for this subject. The atom reads true at every state, so the negation of `eventually holds(c.value = 1)` has no move at position 0. FR-126 "Model-only pass (TP-4)" then creates no product state for either binding, and each binding's pass reads the initial state and every state reached from it: 16 states each. FR-126-AC-12's "the pass's second state exhausting the budget" at `max_states` 1 fixes that the initial state is charged. FR-126 "Bindings and ranking (TP-4)" makes `a` `Holds{Exhaustive}` before `b` runs and shares `max_states` across bindings, and "Model-only pass" charges every pass state to it and counts none in a product total. Total charge is 16 + 16 = 32: 32 and the default limits hold with (0, 0) product states and edges; 31 stops in `b`'s pass naming `MaxStates` 31.

Detection. If the budget were per binding, or if pass charges were not carried from `a` to `b` (`Totals::remaining` subtracting only product states, or `dead_phase` not accumulating `charged_states`), `b`'s pass would get the full 31 and the 31 case would return `Holds`. The coder's mutant log shows exactly that failure ("31 states do not cover both passes"). If `b` were never evaluated, 16 states would be charged and 31 would hold. If pass states joined the product totals, the (0, 0) assertions fail. The test therefore cannot pass for another reason short of a contrived double-charge within one binding combined with `b` skipped.

Lost coverage. The old test's distinctive claim was that `a`'s product states plus `a`'s pass are charged before `b`'s pass. Product-state sharing across bindings stays covered by `liveness::the_state_budget_is_shared_by_every_instance` (two 16-state products, budget 20 stops; it would hold under per-binding budgets). Product and pass charges go through the one `Totals::remaining` (`used(states, charged_states)`), used by both the product exploration and `dead_phase`; dropping either term fails one of the two tests. The mixed case (an earlier binding's product states reducing a later binding's pass) adds no independent mechanism and is not required in this PR. The old test's `(1..=200)` search for the least finishing budget, with an inequality, is replaced by exact values, which is stronger.

Tags. `TC-1085` fits by its description ("the read is charged to the run's budgets and joins no product total"); a TC-only tag matches repo practice (`example_units::a_later_binding_is_not_evaluated_or_cancelled_after_the_first_refutes`), and `quire trace --file` reads it as a structural `verifies` binding. No FR-126 acceptance criterion states cross-binding charging of model-only states (AC-12 is one binding, AC-15 is about not evaluating `b` after a refutation), so no AC tag is correct. That FR-126's shared-budget SHALL has no AC predates this PR and is not a finding on it.

"Closes QSL-3413" is justified: the ticket is the red test; the PR makes it green while keeping the requirement it was written for, and the ticket's own classification (stale test, not a wrong answer) matches the code at `c43c11406c`.

Rust lane (rust-review folded in): test-only change, no unsafe, no panics outside test expectations, removed imports and the test-local `ByKey` letters have no other users (workspace `cargo check --all-targets --all-features` and `cargo clippy -p fsl-analyze --all-targets -D warnings` pass on the head). `cargo fmt --check` fails only in files outside the diff. Gap analysis was not run: the diff adds no production code and no acceptance criterion. No AssuranceProfile applies.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The test is tagged `TC-1085` in this PR, but TC-1085's Test Procedure, Expected Results and Scope line (FR-126-AC-12 to AC-15) contain no step for its fixture (two Counter bindings, always-true letter, `max_states` 32 / default / 31), so a reader of TC-1085 cannot find the expectations the tag claims | fsl-analyze/tests/it/liveness.rs:526; spec/test-cases/TC-1085-liveness-model-only-pass-and-unsupported-refusals.md |

## Scope and evidence

Examined: the full diff of `fsl-analyze/tests/it/liveness.rs`; `liveness_support.rs` (`Fixture::run`, `fixture`); `counter.rs` (`CONTRACT`, `document`, `Letters`); `model_check.rs` `pairs()` and TC-521's 16/96 counts; `fsl-analyze/src/model_check/liveness.rs` `Totals::remaining`, `dead_phase`, `charge_lifted`; FR-126 "Bindings and ranking (TP-4)", "Model-only pass (TP-4)", AC-12, AC-14, AC-15; TC-1085.

Coder's runs on head `f1e7e470ba` (not re-run): the edited test with `--exact`, 1 passed; the `liveness` filter, 81 passed including `the_state_budget_is_shared_by_every_instance`; the mutant run (no `charged_states` accumulation), 0 passed, 1 failed at the 31 case.

Fix for FND-001: in TC-1085 add a step 6, "Over the Counter subject with counters `a` and `b` both 0, `eventually holds(c.value = 1)` with a letter that reads true at every state, with `max_states` 32, the default limits, and `max_states` 31", with the expected result "Step 6: `Holds{Exhaustive}` with 0 product states and edges at 32 and at the default limits; `Stopped{ResourceExhausted, {MaxStates, 31}}` with 0 product states and edges at 31: both bindings' 16-state model-only passes are charged to the one run budget", and extend the Scope line to name FR-126 "Bindings and ranking (TP-4)" and "Model-only pass (TP-4)".

## Dispositions

Round 1, head `708192a396779bbac3ed37d4dcdbd5743b0990ee` (fix commit `d99f481a27`, then a merge of main that changes no file this review covers). TC-1085 now has step 6 with the requested procedure and expected result, its Scope line names FR-126 "Bindings and ranking (TP-4)" and "Model-only pass (TP-4)", and the shared fixture line now reads "Fixtures for steps 1 to 5", which is correct because step 6 uses the Counter subject. The wording `Holds{Exhaustive}` in step 6 is accurate although the test matches only the `Holds` variant: in this API `Holds` has no basis member and is the exhaustive outcome by definition ("every reachable product state was examined"). Bounded and reduced runs are the separate `BoundReached` and `HoldsReduced` variants, so matching `Holds` asserts the `Exhaustive` basis. Step 6 claims no certificate kind, so the test leaving the certificate uninspected leaves no claim untested. No new findings.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | TC-1085 step 6, its expected result and the Scope line added in d99f481a27 |
