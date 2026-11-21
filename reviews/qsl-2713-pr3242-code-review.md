---
id: SR-20261121021842
title: "code review of PR3242 constrained liveness, stack-safe state keys, QSL-2692 un-ignore"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; fsl-analyze/src/model_check/liveness.rs, fsl-analyze/tests/it/reduction/completion_boundaries.rs, fsl-eval/src/simulation/model/encode.rs, fsl-eval/src/automaton/key_state.rs, tests/it/checker_value_boundaries/state_keys.rs, tests/it/error_producer_inputs.rs; PR3242 head 8909b6fa71e6b4e9cdb42512b2e8fea33eee283c"
review_set: subset
---

## Summary

Ticket: QSL-2713 (this review covers the batch: QSL-2713, QSL-2704, QSL-2692). Code-review with the rust-review lane folded in; diff only, no build. Constrained liveness on a model subject is sound: a boundary product state returns no successors, no `Enabled` and no stutter, so it is on no cycle and no stem; any stored boundary state settles `ConstraintReached` before `HoldsReduced`/`Holds`, so the run never proves a claim cut at the boundary. The iterative `Enc` writer emits the same event sequence as the recursive one, and the manual `Drop` is sound.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Model-only pass skips the letter-undefined check at a boundary state: an undefined letter at a model-only boundary state settles `ConstraintReached` instead of `Violated(UndefinedEvaluation)` | fsl-analyze/src/model_check/liveness.rs:2202-2207 |
| FND-002 | low | `ConstraintReached` is decided after the memory and channel bound outcomes, against ADR-025 MB-4 / ADR-027 AE-4 precedence (unreachable today, model subjects have no memory bound) | fsl-analyze/src/model_check/liveness.rs:1752-1781 |

## Verdict

Mergeable once FND-001 is fixed; FND-002 is a one-block move.

Checked clean: boundary states have no out-edges in the product, so no lasso includes or passes through one and no stutter is added (FR-158 SC-2, AC-2). `ConstraintUndefined` refutes with `UndefinedEvaluation` and is not counted as a boundary state (AC-3). Fairness reads `Enabled` from expanded states only, and those expansions are unchanged by the cut. POR receives the constraint, so `Por::new` treats its reads as visible (FR-155). The canonical-finding check treats boundary states as fully examined, which is correct. The boundary count is the union of model state keys across bindings and the model-only pass.

Oracle: FR-126 "Bindings and ranking" shares `max_states` across bindings and sums `product_states`. With three `Counter` bindings, each storing the one initial boundary state, `max_states: 3` gives `product_states == 3` and `ConstraintReached{1}` (one distinct model state). `max_states: 1` stops the second binding on `MaxStates 1`. The corrected test is right.

Protocol subjects: FR-158 "Protocol subjects" says "Every other rule of this requirement holds for a protocol subject as stated", so the spec does not exempt a protocol liveness claim. The remaining `ReducedLiveness` refusal (check.rs:741, unchanged by this PR) is an existing gap against FR-158, not a defect this diff introduces. The new `ProtocolSystem::constraint_value` is never reached until that gap is closed.

QSL-2704: for every variant the writer pushes the events in the old order (Array: begin, items, end. Object: begin, then name and value per member, then end. Map: begin, `entries`, begin array, then per sorted entry begin, `key`, key, `value`, value, end, then end array, `type`, `map`, end). Leaves and identities are written when popped, so the bytes are identical and an `Unencodable` refuses at the same point. `Drop` moves the children into a work list. Each popped node is shallow when it drops. A `Shared` child is taken only when `Arc::get_mut` shows it is unique; otherwise the last handle's inner `Enc` drops through the same iterative `Drop`. There is no leak and no double drop. `key_state.rs` removes the first element in place through `&mut` and encodes the same `Array`, so its bytes are unchanged. `Canonicalizer::write` and the derived `Clone`/`Ord`/`PartialEq`/`Debug` still recurse once per nesting level. The ticket's 1,000-link journey passes at 512 KiB per the PR. A much deeper key (the 100,000-link evaluator case) can still overflow through `Canonicalizer::write` or a derived comparison. That is residual risk outside this ticket's oracle. The test's independent byte oracle guards the key bytes.

QSL-2692: the un-ignored test asserts the ticket's strict source-type and universe expectations through the public spine; no product change is needed.

## Scope and evidence

Read the PR diff and, via `git show` at the PR head, liveness.rs (successors, decide, the model-only pass), check.rs `check_protocol_constrained`, encode.rs (`Enc`, writer, `Canonicalizer`), FR-158, FR-126 "Bindings and ranking", and ADR-021 SC-2, SC-3, RV-1, RV-4 and RV-5. Gap-analysis was not run (one proportionate pass per the brief). No cargo.

- FR-158-AC-2 (spec/functional/FR-158-cut-the-search-with-a-state-constraint.md): examined
- FR-158-AC-3 (spec/functional/FR-158-cut-the-search-with-a-state-constraint.md): examined
- FR-126 Bindings and ranking (spec/functional/FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md): examined
- FR-158 Protocol subjects (spec/functional/FR-158-cut-the-search-with-a-state-constraint.md): context_only
