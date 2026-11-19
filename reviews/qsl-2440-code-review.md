---
id: SR-2026111612172400000056
title: "Code review of QSL-2440 reward node emission and read-back (filament-spec-language PR 3011)"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language@2429184c6ad10ff2f39da1a9c0e6e6384465d15c; git diff origin/main...HEAD (PR 3011): fsl-semantics/src/check/lowering/probabilistic.rs, fsl-semantics/src/check/probabilistic/model.rs, fsl-semantics/src/check/probabilistic/model/package.rs, fsl-semantics/src/check/mod.rs, fsl-semantics/src/check/probabilistic.rs, fsl-semantics/tests/it/probabilistic_declarations.rs, fsl-package/src/emit.rs, fsl-package/src/emit/probabilistic.rs, fsl-package/src/probabilistic_read.rs, fsl-package/src/emit/tests.rs, fsl-package/src/emit/tests/admission_corpus.rs, fsl-package/src/emit/tests/probabilistic.rs, fsl-eval/src/simulation/model/probabilistic/tests.rs"
review_set: subset
date: 2026-11-19
---

## Summary

Ticket: QSL-2440 (QSL-2000 slice C). Code review with the rust-review lane folded in, over the
PR diff only. The PR delivers: a reward's expression lowered to a clause function owned by the
operation's `ModelOwner` (clause kind `precondition`), the emitter's `probabilistic`/`reward`
node, the FSL reader's `PackageReward` -> `ReadReward` rebuild with a `DuplicateReward` refusal,
lowering in (operation key, name) order, and the reward half of
`fsl-package/src/emit/tests/probabilistic.rs` un-ignored. Out of scope by design: QSL-2441,
QSL-2463 and IR-1569.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Arch ruling (1) says rewards get their own clause kind `reward` (QSL-2535, which blocks QSL-2440), and `precondition` stays only as a stand-in that must be marked TEMPORARY with QSL-2535 in code comments and in the PR body. The `reward_function` doc says only "Whether that is the intended kind is a spec question", and the PR body calls it an open spec question. Neither has the TEMPORARY / QSL-2535 marking. Until that fix lands, every IR consumer that joins functions by `ModelOwner` and clause kind sees an integer-typed `precondition` on each rewarded operation | fsl-semantics/src/check/lowering/probabilistic.rs:191-209 |
| FND-002 | medium | The reward identity puts `expression_spelling()` into the node key as a `PathSegment::Text`. That string is built from `format!("{:?}")` of each checked node's `NodeKind<NodeId>` and `ValueType`. (a) It turns the `Debug` derive of another crate's types (filament-checked, filament-exact) into a byte-stability contract: if a dependency bump changes a field name or a variant spelling, every reward's node id changes and nothing fails. (b) `spell` recurses on the host stack once per nesting level, which the module's own walks avoid (`CheckedNode::descendants` uses `filament_walk`). (c) The key grows with the size of the expression and is not charged to any limit, which breaks the QSL-381 invariant in `admitted_node_key` that "a key's size never depends on how deep its operands nest". A content operand that is already canonical and bounded, the function body's expression node key (`walked_operands`), would meet FR-640-AC-3 without these costs | fsl-semantics/src/check/probabilistic/model.rs:239-255; fsl-package/src/emit/probabilistic.rs:575-580 |
| FND-003 | medium | This PR is the first that lets a package carry `reward` nodes, and the package route drops them without saying so. `PackageModelInput` rebuilds the model from `package.probabilistic()`, and the rewards land in `read_rewards`. But `ModelSystem` reads only `model.rewards()` (the S3 records): reward names at probabilistic.rs:320, values in `rewards_of`. `first_unread_model_form` has no reward arm, so a reward-bearing package is admitted on the package route with no reward names and every step reward missing. The doc at input.rs:253-256 ("A package carries no reward, delay or claim of the model") is now false for rewards. Until an engine reads `ReadReward`, the package route should refuse a `reward` node as an unread form, or the deferral should be stated with a ticket | fsl-eval/src/simulation/model/input.rs:250-257; fsl-package/src/model_input_form.rs:54-68; fsl-eval/src/simulation/model/probabilistic.rs:320-326 |
| FND-004 | low | Doc and code disagree. `Reward::function_mode` says "the requested mode, as for an operation `body` clause function (FR-952)", but `mod.rs` always sets `CheckMode::Linked`. That is correct for a declared non-`body` clause (FR-952; IR's owner join refuses `Kernel` on a `precondition` function), so the doc is wrong, not the code. Separately, the doc of `a_reward_is_emitted_with_its_clause_function` says "and the `body` clause kind", while the test asserts `precondition` | fsl-semantics/src/check/probabilistic/model.rs:216-218; fsl-semantics/src/check/mod.rs:7017-7023; fsl-package/src/emit/tests/probabilistic.rs:1381-1387 |

## Scope and evidence

Examined (clean unless listed above):
- `reward_function`: owner `ModelClause { declaration: operation key, kind: Precondition,
  source_declaration: None }`, the operation's parameters in declared order including random
  ones, result the reward's value type, and `index = functions.len()`, which writes no function
  table slot (`functions.get_mut(index)` is `None`). A reward on a type with no `model` node
  lowers to `None`, and emission refuses with its locus.
- Ordering (arch ruling 2): the rewards are lowered in a fixed phase (`probabilistic_declarations`,
  check/mod.rs:6262), after every function group (check/mod.rs:5375). Their ordinals on the shared
  derived path `(model identity, function, pure_function, [])` therefore depend on how many
  model-owned clause functions exist before them, not on source order, and on the (operation key,
  name) rank among rewards. That meets the ruling's condition, since reordering unrelated
  declarations changes no reward key. Adding a reward or a dispatch clause function does shift
  later ordinals; the ruling accepts this.
- Emitter `reward_node`: bindings `declaring, operation, name, value_type, expression` in FR-640
  order; `value_type` taken from the function's `semantic_type` and checked against the six
  admitted type forms; every failure is `refusal(...)` with the reward's declaration locus
  (FR-185-AC-12). The removed blanket `ProbabilisticModelNotEmitted` guard covered rewards only,
  and every remaining feature is written or refused node by node.
- `function_check_mode` closure: a function node with no table index falls back to
  `reward_function_check_mode(key)`, and anything else still refuses "no recorded check mode".
- FSL reader: `PackageReward` is bound by position (0..4), the operation join reuses
  `StateModel::operation` (`UnknownOperation`), and `DuplicateReward` is refused. FSpec FR-640
  reader rule 4 (value type form, function owner, parameters, type, no state reads) is enforced by
  the IR strict reader at the pinned contract-ir v0.1.5 (`probabilistic_join.rs:400-465`), which
  runs first, so FSL's reader does not need to repeat it.
- Framing: `ReadReward` names its contents honestly (identity, parameters, value type node and
  function node) and says it holds no checked expression. It is not presented as a `Reward` or a
  `CheckedBody`. FND-003 covers the consumer side.
- No `#[allow]` or `#[expect]` in the diff. Every new test has `#[trace(...)]`; trace accuracy is
  in the gap analysis (SR-2026111612172400000057).
- `cargo clippy -p fsl-package -p fsl-eval --lib --tests -- --cap-lints warn`: see the verdict
  for the result. fsl-semantics clippy was not run (house rule; about 565 inherited errors).

## Verdict

The lowering, emission and reader are sound, and the IR strict reader admits the emitted package
(`Read::Verified`). Ruling (2), the ordinal plus content spelling, holds by construction: the
lowering phase fixes the ordinals regardless of source order. Ruling (1) is not yet followed in
form: the `precondition` stand-in is not marked TEMPORARY / QSL-2535 (FND-001). The content
spelling works but depends on `Debug` output and on host recursion (FND-002). The package route
now silently drops rewards (FND-003). Not mergeable until FND-001 is fixed. FND-002 and FND-003
should also be fixed in this PR, or FND-003 deferred to a named ticket.
