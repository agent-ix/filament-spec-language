---
id: SR-2972
title: "Code review of PR #3940: fsl-replay clippy too_many_arguments grouping, code-review"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language@6df4690fdd43c488c38fdb48f4ada4498dc725a8; git diff origin/main...6df4690fdd (PR #3940): fsl-replay/src/refinement/step.rs, fsl-replay/src/model_check/refinement_replay.rs; ticket QSL-3357"
review_set: subset
date: '2026-11-25'
---

## Summary

Ticket: QSL-3357. PR: filament-spec-language#3940, head 6df4690fdd. Method: code-review with the rust-review lane. Lint-only refactor for clippy `too_many_arguments`: `decide_step` and `check_step_inner` take one `StepProduct { allow_protocol_node, merge_candidate_states }` (Copy) instead of two bools; `judge_operation` takes one `AbstractCall { operation, arguments }`; `replay_lasso` reads `mapped` from `Walk` instead of a separate parameter. Cargo was not run by this reviewer (brief); the PR body reports clippy -D warnings and the refinement test filters passing at 6df4690fdd.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| check_step, check_step_for_model, check_protocol_step call sites of check_step_inner | fsl-replay/src/refinement/step.rs | examined |
| StepProduct, check_step_inner, decide_step destructure | fsl-replay/src/refinement/step.rs | examined |
| AbstractCall, judge_operation and its single call site in decide_step | fsl-replay/src/refinement/step.rs | examined |
| settle_positions Walk construction and replay_protocol_divergence branch | fsl-replay/src/model_check/refinement_replay.rs | examined |
| Walk.mapped, replay_lasso partial moves of walk.positions and walk.mapped | fsl-replay/src/model_check/refinement_replay.rs | examined |
| merge with origin/main 3d08d7b95f (#3927, #3934) | fsl-replay/src/** | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verdict

Clean. Behaviour is identical:

- Flag values: check_step passes (false, false), check_step_for_model (false, true), check_protocol_step (protocol_node, false) in the same field order as the old positional (allow_protocol_node, merge_candidate_states) pair; no swap. `decide_step` destructures `product` once at entry into the same two names; it has one caller and no recursion, so no value or timing changes. StepProduct is Copy over two bools.
- `AbstractCall` carries the same `&DeclarationKey` borrow and the same owned `Arguments` value, moved at the same point (the only call of judge_operation); destructured at entry into the old names.
- `mapped` is moved into `Walk` right where it was previously moved into `replay_lasso`; nothing reads or mutates it between. The `replay_protocol_divergence` early return still takes `mapped` before the Walk is built, and the non-lasso path after the `if let` is unaffected. Inside `replay_lasso`, `walk.mapped` is never read or mutated before the three settlement arms, each of which moves it once; the existing partial move of `walk.positions` into `loop_states_of` is disjoint. Walk has no Drop impl and only one construction site.
- Merge: `git merge-tree` of origin/main 3d08d7b95f with the head is conflict-free; #3927 and #3934 touch fsl-replay probability_certificate/lib/outcome files but neither changed file. A heuristic parameter count over the merged tree finds no other fsl-replay fn above 7 parameters without an `allow`, so the change stays sufficient against current main (cargo not run here).
