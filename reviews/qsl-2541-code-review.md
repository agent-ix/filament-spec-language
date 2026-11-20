---
id: SR-2026111612172400000058
title: "Code review of QSL-2541 fsl-eval lib test repair after the S3 reward-evaluator change (filament-spec-language PR 3014)"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language@131c8865b391c1e13cb8cab3a2a7bb6dc36162c1; git diff origin/main...HEAD (PR 3014): fsl-eval/src/simulation/model/probabilistic/tests.rs, fsl-eval/src/simulation/sampled/world.rs, fsl-eval/src/value/expression/evaluate/invariants.rs"
review_set: subset
date: 2026-11-19
---

## Summary

Ticket: QSL-2541. Code review with the rust-review lane folded in, over the PR diff only.
The PR repairs fsl-eval lib tests that stopped compiling when #3007 (QSL-2252) added a
third `&dyn RewardEvaluator` argument to `PackageDeclarations::check`, and repairs two
tests that the compile break had been hiding: their packages declare nothing, so FR-746's
`EmptyPackage` refusal (added in #2916) refuses them.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Scope and evidence

Examined (all clean):
- `World::with_s3_reward_signs_unchecked` and `World::checked(.., evaluator)`
  (`fsl-eval/src/simulation/sampled/world.rs:239-341`). The whole `world` module is
  `#[cfg(test)] pub(crate) mod world;` (`fsl-eval/src/simulation/sampled.rs:565-566`), so
  nothing reaches production. `fsl_semantics::check::test_support` is
  `#[cfg(any(test, feature = "test-support"))]` (`fsl-semantics/src/check/mod.rs:168-188`),
  and fsl-eval enables `test-support` only on its dev-dependency (`fsl-eval/Cargo.toml:66-71`),
  which the root `no_shipped_dependency_enables_test_support` check enforces. No leak.
- `a_negative_reward_refuses_admission` still proves FR-187 admission refusal. The zero
  evaluator only answers S3's FR-185 sign check (`check_reward_signs`). Model admission computes
  the sign itself through `reward_value_with_cancel` and refuses with `NEGATIVE_REWARD`
  (`fsl-eval/src/simulation/model/probabilistic.rs:529-563`), so the evaluator handed to S3
  does not affect the path under test. All four assertions are unchanged (code/cause,
  `reward`, `operation`, `arguments = "n = 0"`). Before #3007, S3 ran no sign check, so the
  `World::new` the test used then is what `with_s3_reward_signs_unchecked` builds now. The
  test is not weakened.
- Soundness against production: S3 now refuses negative finite-domain rewards earlier. FR-187
  admission is still needed in production, and not only as defence in depth.
  `check_reward_signs` defers "a reward that reads an unbounded domain ... to model admission
  (FR-187)" (`fsl-semantics/src/check/reward_validation.rs:54-55`). A package from another
  producer or read from the wire (FR-187-AC-4/AC-5, FSpec FR-411-AC-17) also never passed
  through this S3 run. S3's own refusal is tested in
  `fsl-analyze/tests/it/reward_evaluator_port.rs` (TC-2061/FR-185-AC-11, TC-3027/FR-185-AC-13)
  and `fsl-semantics/tests/it/probabilistic_declarations.rs`, as the doc comment says.
- The eight `.check` calls in `invariants.rs` and the `with_profiles`/`with_queries` paths
  use `crate::value::S6aRewardEvaluator`. That is the production evaluator, it matches every
  other fsl-eval `.check` call site (`bare.rs`, `family.rs`, `evaluate.rs`, `union_case.rs`,
  `expression/mod.rs`), and none of these fixtures declares a reward, so the choice cannot change
  a result. Only the one test that needs S3 blind to the sign uses `ZeroRewardEvaluator`.
- The two `EmptyPackage` fixes: `declarations.types = a_declared_type()` adds one empty
  `Placeholder` record. In `imported_callable_consumer_uses_a_real_emitted_and_admitted_library`
  it goes on the consumer package only, which otherwise holds just an import. The library package
  already has `Body` and is untouched. The probes (`evaluation-callable-unresolved`, admitted
  imported call returning `true`, `machine.imported` empty) read only the imported callable.
  In `both_ieee_profile_checks_and_conversion_operand_check_are_real_producers`, the
  `ieee-operation-rejected` probe reads only the IEEE profile. Neither test reads the type
  environment, so neither proves less than it did.
- No `#[allow]` in the diff. Trace tags are unchanged.
- Tests (name-filtered, `--test-threads=1`, both standing skips): the
  `value::expression::evaluate::invariants`, `simulation::model::probabilistic` and
  `simulation::model::frames::narrowing_tests` filters pass. `cargo clippy -p fsl-eval
  --all-targets -- --cap-lints warn` reports no warning in the changed files (see Verdict).

## Overlap with #3012 (main 4d30cfb85)

#3012 has since landed on main. It adds the same eight `&crate::value::S6aRewardEvaluator`
lines to `invariants.rs`, byte for byte. `git merge-tree --write-tree HEAD origin/main` is
clean (exit 0), and the merged diff against main is down to the parts still needed: the two
`a_declared_type()` fixes plus the helper (13 lines), the `world.rs` evaluator seam, and the
one-line test change. #3012 did not fix the red. On origin/main 4d30cfb85,
`a_negative_reward_refuses_admission` panics in `World::checked` (S3 refuses the reward,
world.rs:300), and the two EmptyPackage tests panic at invariants.rs:1095 and :663. With
the PR merged locally onto 4d30cfb85, the filtered run passes (57 passed, 2 ignored).

## Verdict

Clean. The test-only API is gated and the evaluator choices fit their tests. FR-187
admission coverage is intact. The PR is still needed after #3012, and it merges onto current
main without conflict. Mergeable from a code-review standpoint.
