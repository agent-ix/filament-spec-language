---
id: SR-2026112101551700000002
title: "Gap analysis of QSL-2544 FR-187-AC-4/AC-5 on the package route (filament-spec-language PR 3240)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language@7d352d24b3; PR 3240 delta; FR-187-AC-4, FR-187-AC-5, TC-622, TC-2064"
review_set: subset
date: 2026-11-21
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-187
    type: references
---

## Summary

Ticket: QSL-2544. Planless gap analysis, limited to the ticket delta. Matrix: `quoin matrix --json`
(quoin 0.36.8) from the worktree root, exit 0, empty stderr. FR-187-AC-4 is tagged by
`a_negative_reward_refuses_admission` (source), `the_support_graph_is_the_state_graph` and the
new `a_negative_reward_read_from_the_wire_refuses_admission`. FR-187-AC-5 is tagged by
`zero_weight_workloads_follow_enabled_operation_weights` and the new
`a_hand_built_wire_package_with_a_negative_reward_is_refused_naming_its_arguments`. Both have
static status `tagged` and no run evidence (no evidence store read). No stubs, ignored tests or
unowned code were found in the delta.

## Verdict

CONDITIONAL. The wire-route negative-reward refusal of AC-4/AC-5 is backed by tests that fail on
old main. FR-187's step-reward clause on the package route, which this PR newly enables, is
unbacked, and so is cross-reward order.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-187's statement "A step's reward values SHALL be its operation's rewards evaluated on the transition's arguments" now has a package-route implementation (`DeclaredReward::Read` in `rewards_of`) that no test backs. Only the negative admission refusal runs a read reward | spec/functional/FR-187-give-model-transitions-step-probabilities-and-rewards.md:106-108; fsl-eval/src/simulation/model/probabilistic.rs:506-532 |
| FND-002 | medium | "for the first negative value in canonical order, the same refusal S3 gives" is tested only with one reward. Nothing checks that a model with two rewards that can be negative gets the same record on both routes, and the code diverges (SR-2026112101551700000001 FND-001) | spec/functional/FR-187-give-model-transitions-step-probabilities-and-rewards.md:109-119; fsl-eval/src/simulation/model/probabilistic/tests.rs:920-967 |
| FND-003 | low | TC-2064 step 3 asks for a hand-built checked package. The test emits a compiler-checked source package with S3's sign check answered by a zero evaluator, which is the same fixture and route as the AC-4 test. That is equivalent at the wire boundary, but it adds no case beyond AC-4 | spec/test-cases/TC-2064-weighted-steps-under-a-workload-with-weight-zero.md; fsl-eval/src/simulation/model/probabilistic/tests.rs:947-967 |

## Coverage

Plan completion: not assessed

- FR-187-AC-4 (examined): wire negative reward refuses `invalid_probabilistic_model`/`negative-reward` naming the operation and `refund` (the `n = 0` argument is not asserted on this route; SR-2026112101551700000001 FND-005). The AC's support-graph clause is unchanged by this PR.
- FR-187-AC-5 (examined): wire negative reward refuses naming `n = 0`.
- Mutation reasoning (not executed): on old main `PackageModelInput::of` returns `PendingPackageForm::Form{Reward}` and the tests' `unwrap_or_else` panics; with read rewards dropped from `DeclaredReward::of`, admission is `Ok` and both `let Err(..) else panic` arms fire.
- Sibling failure `a_malformed_source_operation_identity_is_refused_at_the_row` panics inside `ReadIdentityRows::read` (fsl-package/src/bare.rs), outside this diff.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | low | The canonical-order test puts both rewards on one operation, so it orders by name alone. The operation-key half of the (operation key, name) order, with negative rewards on two operations, is untested across routes | fsl-eval/src/simulation/model/probabilistic/tests.rs:1007-1020 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Package-route step rewards are backed by `the_package_route_gives_the_rewards_of_a_two_parameter_reward_as_the_source_route_does` (two parameters, 18 steps, equal to the source route) |
| FND-002 | fixed | `both_routes_refuse_the_same_first_of_several_negative_rewards` checks that two negative rewards give the same record on both routes |
| FND-003 | fixed | The AC-5 test is now a distinct case: `gap = m - n` over two parameters, refused at `n = 1, m = 0` past non-negative vectors. That vector also catches a swapped argument binding, which would give `n = 0, m = 1`. A package emitted with S3's sign check disabled and read back from wire bytes is what TC-2064 asks for at the wire boundary, so hand-assembling the nodes would only duplicate the emitter |
