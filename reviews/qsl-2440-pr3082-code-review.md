---
id: SR-2026111919340478491681403
title: "Code review of QSL-2440 reward clause kind and source-declared rewards (filament-spec-language PR 3082)"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language@012f72fe4d9d9ee112d0588e5ee52c8cc1091e18; git diff origin/main...HEAD (PR 3082): Cargo.toml, Cargo.lock, fsl-forms/src/syntax.rs, fsl-semantics/src/check/lowering.rs, fsl-semantics/src/check/lowering/model.rs, fsl-semantics/src/check/lowering/probabilistic.rs, fsl-semantics/src/check/probabilistic/model.rs, fsl-package/src/emit/probabilistic.rs, fsl-package/src/emit/tests/probabilistic.rs, fsl-command/tests/it/probabilistic_package_locus.rs"
review_set: subset
date: 2026-11-19
---

## Summary

Ticket: QSL-2440. Code review with the rust-review lane folded in, over the PR 3082 diff only.
The PR replaces the TEMPORARY `precondition` clause kind of a reward's clause function with a
new `DeclaredClauseKind::Reward` (spelled `reward`), bumps `filament-contract-model` to
contract-ir dd390f51 (#1065, which admits that kind), gives a reward on a source-declared object
a `SourceOwner` declaration `[type, operation, "reward", name]` at ordinal 0, and carries the
lowered declaring node to the emitter through a new `LoweredReward` / `Reward::declaring_node()`.
Tests: the source-object reward test is un-ignored, the interim refusal test and the reward case
of `probabilistic_refusals_have_locus` are removed, the fsl-command locus test now refuses a
delay, and three new tests cover the reader admitting kind `reward`, reward ordinals by name, and
the source-owned declaration.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `the_reader_admits_the_reward_clause_kind_and_refuses_any_other_for_a_reward` asserts only `!matches!(refused, Read::Verified { .. })` after rewriting the clause binding to `precondition`. FR-640-AC-7 and FR-322-AC-471 name the refusal: `invalid_package`/`invalid-value` at the `reward` node's `/body/expression` (reader rule 4). Any refusal passes the test, including one caused by the hand edit itself (canonical bytes, operand rows, evidence), so it cannot show that rule 4 fires. Other tests in the crate already destructure `Read::Refused(V2ReadRefusal::Envelope { refusal, .. })` to check code and locus (protocol_controls.rs:246, state_graph_claim.rs:397) | fsl-package/src/emit/tests/probabilistic.rs:2148-2193 |
| FND-002 | medium | The ordering rule "the producer lowers every other clause function first and then the package's rewards" (FSpec FR-640 "Reward identity", asserted by FR-640-AC-3) has no test; the new test's doc says so. A real test is feasible: the `operation_bodies.rs` fixtures give a model operation an `operation-body` clause function under the same `ModelOwner` path, so a unit adding `reward r on T::op = ...` to that operation can assert the reward function's ordinal is greater than the body function's, and that writing the reward before or after other declarations changes neither. The new test also covers one operation only, so ranking by operation key (AC-3's `cost`, `load` on `Server::attempt` and `stable` on `Server::reset`) is untested, and it compares ordinals, not the byte-identical packages AC-3 states | fsl-package/src/emit/tests/probabilistic.rs:2095-2120; fsl-semantics/src/check/lowering/probabilistic.rs:75-88 |
| FND-003 | low | The doc of `reward_function` still says "A reward whose operation's declaring type has no `model` node (a source-declared object type) lowers to nothing, and emission refuses it with its locus". This PR makes that false, and the paragraph added below it says the opposite (a source-declared reward is a `SourceOwner` declaration) | fsl-semantics/src/check/lowering/probabilistic.rs:198-210 |
| FND-004 | low | `source_declaration` is computed with `.then(...).flatten()` over `type_identity_segment(...)?`, so if the segment is `None` for a source-declared operation the function silently falls back to the `ModelOwner` path instead of faulting. `model_node` faults on the same `None` first, so it is unreachable today, but the fallback would key a source-type reward as model-owned without a diagnostic if the two ever diverge; a `stage_fault` (as the function already uses for `reward-function-is-keyed`) states the invariant | fsl-semantics/src/check/lowering/probabilistic.rs:217-234 |
| FND-005 | low | Unrelated comment damage: "Renaming a claim changes its identity and no other claim's." became "... and no  claim's." (the word `other` was deleted, leaving a double space), which inverts the sentence's meaning | fsl-package/src/emit/tests/probabilistic.rs:265 |
| FND-006 | low | `From<DeclaredClauseKind> for ClauseKind` maps the new `Reward` to `ClauseKind::Body` without a comment. The choice matters: `Body` refuses a dispatched call that `Precondition` (the previous interim kind) admits (TC-196 D06-D08). No caller passes `Reward` to `FunctionDeclaration::clause` today, so it is not reachable, but the mapping should say why `Body` is the right checking context for a reward expression | fsl-forms/src/syntax.rs:491-498 |

## Scope and evidence

Examined (clean unless listed above):
- `DeclaredClauseKind::Reward`: every `match` on the enum is exhaustive (`clause_spelling`,
  `From<DeclaredClauseKind>`); no wildcard arm hides the new variant. The clause binding spells
  `"reward"` (model.rs:2480), which is what FR-640 and FR-322 "Owner joins" require.
- `reward_function`: owner `ModelClause { declaration: operation key, kind: Reward,
  source_declaration }`. For a source-declared operation (`operation.model().is_empty()`) the
  declaration is `[type, operation, "reward", name]`, keyed by `insert_ref` with a `declaration`
  occurrence at ordinal 0 (lowering.rs:4233-4268), which matches FR-640 "Reward identity" and
  FR-322 "Source-declared object types" item 5. A model-owned reward keeps the derived path and
  is lowered after every function group, in (operation key, name) order (FND-002 is about the test,
  not the code).
- `LoweredReward` / `Reward::declaring_node()`: the emitter now takes the declaring node lowering
  recorded instead of re-deriving a `model` node, which is what lets a source object type's node
  be `declaring`. The emitter checks the node exists in the package and refuses with the reward's
  locus otherwise.
- Pin bump: `filament-contract-model` has one rev (dd390f51a3eda0b239e9a67f47c7772e0090a3dc) in
  the workspace `Cargo.toml` and one `source` line in `Cargo.lock`; every member crate uses
  `workspace = true`. dd390f51 is the contract-ir #1065 merge commit on `main`, a descendant of
  the old pin c7b62038 (forward move).
- Removed tests: `a_reward_on_a_source_object_refuses_emission_with_its_locus` asserted the
  interim refusal this PR removes; its replacement `a_reward_on_a_source_object_is_emitted` is
  un-ignored. The reward case of `probabilistic_refusals_have_locus` fed a source-object reward,
  which now emits, so it cannot refuse. The emitter's reward refusal path (`unwritable`) remains
  but no source input reaches it; FR-185-AC-12 stays covered for random parameters, workloads and
  delays (the remaining cases and the renamed fsl-command test). No lost coverage that a source
  unit could still exercise.
- No `#[allow]` or `#[expect]` in the diff. New tests use `#[trace]` with `FSpec-` prefixes and
  real ACs; how much of each AC they cover is in the gap analysis
  (SR-2026111919340478491681404).
- Tests run at 012f72fe4 (name-filtered, `--test-threads=1`, house skips): fsl-package
  `emit::tests::probabilistic::` 37 passed, 1 ignored (pre-existing QSL-2546 ignore, not this
  PR); fsl-command `a_package_stage_refusal_of_a_delay_carries_its_locus` passed.
- `RUSTFLAGS='--cap-lints warn' cargo clippy -p fsl-package -p fsl-command --lib --tests`: no
  warning in this PR's fsl-package or fsl-command files. fsl-semantics lints (inherited, house
  rule) not assessed.

## Verdict

The code change is correct and small: the clause kind is its own, source-declared rewards emit
with the owner the spec requires, the pin bump is single-source and forward, and match arms are
exhaustive. The weak spots are in the tests: the negative reader test cannot show which rule
refuses (FND-001), and the "after every other clause function" ordering, which a fixture already
in the crate could test, is untested (FND-002). FND-003 to FND-006 are low. Mergeable after
FND-001 and FND-002 are fixed in this PR; the low findings should be fixed in the same round
since they are one-line edits.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-007 | medium | A reward on a source-declared object now emits a package that the pinned strict reader (contract-ir dd390f51) refuses at the reward's `declaring` binding: `probabilistic_join::operation` accepts only a `Model` node as `declaring`, and a source reward's `declaring` is the source object type's node. On main, the same unit was refused at S4 with the reward's locus and `not-yet-implemented`. After this PR, `fsl compile` succeeds and hands consumers a package every strict reader refuses, and no FSL test reads it back (the narrowing to QSL-2592 drops the read). The emitter matches FSpec FR-640, so the reader is the side that is behind. The lead must choose: keep the S4 refusal for source-declared rewards until QSL-2592 lands, or ship knowingly with QSL-2592 as the tracked gap | fsl-package/src/emit/probabilistic.rs:543-551; fsl-package/src/emit/tests/probabilistic.rs:1954-1968 |
| FND-008 | low | The doc of `a_reward_on_a_source_object_is_emitted` still ends "and the strict reader admits the package". The fix round states the opposite (the reader refuses it, QSL-2592), and the test performs no read | fsl-package/src/emit/tests/probabilistic.rs:1954-1957 |
| FND-009 | low | `the_reader_admits_the_reward_clause_kind_and_refuses_any_other_for_a_reward` still asserts only `!matches!(refused, Read::Verified { .. })` for the `precondition` rewrite, while tagged FSpec-FR-640-AC-7 and FSpec-FR-322-AC-471. `a_reward_function_with_another_clause_kind_or_owner_is_refused` now asserts the same mutation with code and locus, so the weak half is redundant. Drop it, or reuse `refusal_of` / `summary` to assert the code and locus | fsl-package/src/emit/tests/probabilistic.rs:2101-2147 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | fixed | - |
| FND-003 | fixed | - |
| FND-004 | fixed | - |
| FND-005 | fixed | - |
| FND-006 | fixed | - |
