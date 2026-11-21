---
id: SR-2026112101551700000001
title: "Code review of QSL-2544 negative-reward admission on the package route (filament-spec-language PR 3240)"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language@7d352d24b3; git diff origin/main...HEAD (PR 3240): fsl-eval/src/simulation/model.rs, fsl-eval/src/simulation/model/input.rs, fsl-eval/src/simulation/model/probabilistic.rs, fsl-eval/src/simulation/model/probabilistic/tests.rs, fsl-eval/src/simulation/sampled/world.rs, fsl-eval/src/value/expression/bare.rs, fsl-eval/src/value/expression/mod.rs, fsl-eval/src/value/mod.rs, fsl-package/src/bare_slot.rs, fsl-package/src/model_input_form.rs, fsl-package/src/reader_protocol_projection.rs"
review_set: subset
date: 2026-11-21
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-187
    type: references
---

## Summary

Ticket: QSL-2544. Code review with the rust-review lane folded in, over the PR diff only. The PR
deletes `UnreadModelForm::Reward`, lets the package route carry `ReadReward`s into the model,
adds `FunctionSelector` and `BarePackage::project_clause_function`, evaluates a read reward
through its clause function with the bare evaluator, and iterates source then read rewards
through one private `DeclaredReward` view in the zero table, step rewards and the FR-187
admission check. Two wire-route tests replace the deleted refusal test.

## Verdict

CHANGES REQUESTED (no high findings). The single-reward wire path is correct: the clause
function's parameters are the operation's parameters in declared order (lowering passes
`reward.parameters()`), so `arguments` bind correctly; `FunctionSelector::Node` looks the key up
in the wire index and filters to a pure function, so a missing key yields `None` and an internal
fault, never a panic or a wrong function; the operation field the record carries is the same
`ProbabilisticOperation` the source route uses. Both new tests fail on old main (the
`PackageModelInput::of` unwrap panics on the `Reward` unread form) and would fail if read rewards
were dropped from `DeclaredReward::of`. The one failing sibling test
(`source_object_probabilistic::a_malformed_source_operation_identity_is_refused_at_the_row`)
panics in `ReadIdentityRows::read` (fsl-package/src/bare.rs), which this PR does not touch; it
cannot be caused by this diff. Open: cross-reward order differs from the source route,
per-evaluation projection is unmetered, and an unprojectable reward function is classified as an
internal fault.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Cross-reward canonical order differs between routes. The source route checks `model.rewards()` in source declaration order; the package route checks `read_rewards()` in package node order, and the emitter sorts probabilistic nodes by `node_id` before writing them. With two rewards that can both be negative (say `a` on `P` declared first, `b` on `Q`), the source route refuses naming `a`, while the package route names whichever reward's node id sorts first. FR-187 requires the first negative value in canonical order and "the same refusal S3 gives", and the arch ruling requires the source route's order. Fix: iterate both routes under one shared key, for example (operation key, reward name), which lowering already uses | fsl-eval/src/simulation/model/probabilistic.rs:381-388; fsl-eval/src/simulation/model/probabilistic.rs:554; fsl-package/src/emit/probabilistic.rs:67; fsl-package/src/probabilistic_read.rs:113-118 |
| FND-002 | medium | Every read-reward evaluation rebuilds the projection. `reward_projection` calls `project_clause_function`, which builds a fresh `WireIndex` over the whole package plus `world()`, on every `evaluate_read_reward` and `read_reward_type` call. That work is uncharged ("Charges nothing") and cannot be cancelled. The admission check makes one call per argument vector, up to `max_candidates`, and `rewards_of` makes one per reward per expanded step. A package of N nodes with a reward over `Int[0,999] x Int[0,999]` does 1e6 whole-package index builds before it reaches the meter. Fix: project each `ReadReward` once (in `PackageModelInput::of` or `Tables::of`) and evaluate against the stored projection | fsl-eval/src/value/expression/bare.rs:567-627; fsl-package/src/bare_slot.rs:733-745; fsl-eval/src/simulation/model/probabilistic.rs:329-333,478-489,517-527,580-582 |
| FND-003 | medium | A reward function that does not project is reported as an internal fault. `reward_projection` throws away the `BareSelectionFailure` (`Unsupported`, `FunctionResultType`, `FunctionParameterType`) with `.ok().flatten()`. `Tables::of` then reports it as `fault("reward-type")`, and evaluation reports it as `BROKEN`. Before this PR, a reward node refused as a typed unread form at the node. The named-call path keeps the typed refusal and its locus (`function_refusal`). A wire package whose reward body uses a form the bare projection does not support now gets a compiler-fault admission failure with no locus, not a refusal. Fix: project in `PackageModelInput::of` and return the selection failure as a pending form or refusal at the reward node | fsl-eval/src/value/expression/bare.rs:567-579; fsl-eval/src/value/expression/bare.rs:47-69; fsl-eval/src/simulation/model/probabilistic.rs:329-333 |
| FND-004 | medium | No test covers a positive reward read from the wire through step rewards. The `DeclaredReward::Read` arm of `rewards_of`, the read-reward zero table, and `evaluate_read_reward` on a non-negative value are new behaviour (old main refused the package outright), and only the negative admission check exercises them. A wrong argument binding for a multi-parameter reward, or a wrong value on a step, would pass every test. FR-187 requires a step's reward values to be the operation's rewards evaluated on the transition's arguments. Fix: compare `weighted_steps` rewards between the package route and the source route for a positive reward, ideally one with two parameters | fsl-eval/src/simulation/model/probabilistic.rs:506-532; fsl-eval/src/simulation/model/probabilistic/tests.rs:920-967 |
| FND-005 | low | The TC-622 test omits the `arguments == "n = 0"` assertion that TC-622 step 4 requires. The TC-2064 test omits `reward` and `operation`. The two tests share the same fixture and route, so each one asserts half of the same record | fsl-eval/src/simulation/model/probabilistic/tests.rs:920-967 |
| FND-006 | low | `function_label` returns an empty string for any function node with no declaration, not only for a clause function. That removes the `reader-function-name-is-admitted` fault from `ReaderBuilder::read_functions` and from both walker paths on every route. A declared function that arrives without its declaration now projects silently with an empty name, which `Callee.name` carries into dispatch selection | fsl-package/src/reader_protocol_projection.rs:1663,3840-3848 |
