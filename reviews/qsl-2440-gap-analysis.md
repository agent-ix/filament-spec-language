---
id: SR-2026111612172400000057
title: "Gap analysis of QSL-2440 reward node emission and read-back (filament-spec-language PR 3011)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language@2429184c6ad10ff2f39da1a9c0e6e6384465d15c; git diff origin/main...HEAD (PR 3011); FSL FR-740 AC-1..AC-4 and Behavior, FR-185-AC-7, FR-185-AC-12, FR-186-AC-5 (TC-2062, TC-621); FSpec FR-640 AC-1, AC-3, AC-4 and Reader rule 4 (TC-1563) at agent-ix/filament-specification 6b98292e"
review_set: subset
date: 2026-11-19
---

## Summary

Ticket: QSL-2440 (QSL-2000 slice C). A manual check of ACs against tests over the tagged tests
and code in the diff. The computed `quoin matrix` was not run, because it is not
name-filterable and cargo slots are scarce. Plan completion: not assessed. Partial delivery is
by design: QSL-2441 (claim under workload), QSL-2463 (rewards on source-declared objects) and
IR-1569 (Boolean/enum supports, fix merged in contract-ir #1052 but the pin has not moved). The
review judges only the delivered reward emission and read-back.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Wrong trace: `reward_order_changes_no_node_and_the_expression_changes_the_identity` is tagged `FR-740-AC-3`. That AC covers a `discrete` delay, a workload with weight 0, and `Steady`'s weight lines written in another order, none of which the test exercises. Its correct tag, `FSpec-FR-640-AC-3`, is already on the test. Drop the FR-740-AC-3 tag. The FR-740 Behavior clause the test does exercise ("reordering ... the unit's declarations changes no node's bytes") has no AC | fsl-package/src/emit/tests/probabilistic.rs:1549-1556 |
| FND-002 | medium | Wrong trace: `a_package_reward_that_breaks_the_reader_rules_is_refused_on_read_back` is tagged `FR-740-AC-4`, which is about a claim `under Steady` resolving to the workload node. The test drives FSL's own `DuplicateReward` and `UnknownOperation` refusals on an edited `PackageProbabilistic`. No FSL AC states those refusals. FSpec-FR-640-AC-4 states IR's `invalid_package`/`conflicting-definition`, not FSL's records. Remove the tag, and either cite the FR-740 Behavior clause in prose or have the lead add an AC | fsl-package/src/emit/tests/probabilistic.rs:1520-1548 |
| FND-003 | medium | Test intent against arch ruling (2). The test doc claims "changing the expression changes the reward's identity and its function's", but the test asserts only the reward `node_id`. The function's id cannot change: the emitter's own comment says the function key "holds no content". The ruling also requires proof that the order never depends on the source order of unrelated declarations. The test reorders the two rewards only against each other; it never moves them relative to the random parameter or to a workload or claim. The FR-640-AC-3 clause that a reward expression change changes "the identity of every claim that reads it" is untested and not marked deferred (QSL-2441) | fsl-package/src/emit/tests/probabilistic.rs:1549-1640; fsl-package/src/emit/probabilistic.rs:575-580 |
| FND-004 | medium | Coverage inflation: `service_reward_is_emitted_beside_its_random_parameter_and_workload` is tagged `FR-740-AC-1` and `FSpec-FR-640-AC-1`. It removes Service's Boolean parameter and asserts only that one `reward` node with name `duration` exists. It does not check AC-1's "whose function returns `d`", the `declaration` occurrence, strict-reader admission, or the rebuilt record. The full AC-1 test stays ignored on IR-1569, correctly. Narrow the tags in prose, or assert the function body and reader admission | fsl-eval/src/simulation/model/probabilistic/tests.rs:709-736 |
| FND-005 | medium | FR-740 Behavior says the reader rebuilds "`RandomParameter`, `Workload`, `Reward` and the delay records equal to S3's", and FR-740-AC-1 says "rebuilds records equal to S3's". The delivery rebuilds a `ReadReward` (identity, parameters, value type node, function node) with no expression, and nothing reads it (see SR-2026111612172400000056 FND-003). The framing is honest, but no ticket or spec note records that the reward half of "equal to S3's" is partial, so FR-740-AC-1 will read as covered once IR-1569 un-ignores the Service test | fsl-semantics/src/check/probabilistic/model.rs:303-322; fsl-semantics/src/check/probabilistic/model/package.rs:268-292 |
| FND-006 | low | `a_reward_reads_back_as_the_s3_record` compares name, operation, parameters and function but not `ReadReward::value_type`, so a reader that bound the wrong position for `value_type` would pass | fsl-package/src/emit/tests/probabilistic.rs:1495-1518 |

## Scope and evidence

Examined (clean unless listed above):
- FR-185-AC-7 / FR-740-AC-1 / FSpec-FR-640-AC-1, delivered half:
  `a_workload_and_a_reward_are_emitted_as_probabilistic_nodes` (un-ignored),
  `a_reward_is_emitted_with_its_clause_function` (binding order, owner `model`, `semantic_type ==
  value_type`, clause kind, parameters `n`, `m` in declared order, one `declaration` occurrence),
  and `reward_round_trips` (IR typed read, function, value type, declaration region).
- FR-186-AC-5 (TC-621): `a_checked_claim_and_a_reward_are_both_emitted` asserts the claim node and
  the reward node, matching the spec's "with a reward declared after it, S4 emits ... the reward's
  node" clause.
- FR-185-AC-12: `a_reward_on_a_source_object_refuses_emission_with_its_locus` asserts the
  `ProbabilisticModelNotEmitted` feature, the name, a non-empty region and code/cause. The fsl-eval
  Service refusal now names `attempt(ok)`/`RandomParameter`. That is accurate: the reward no
  longer refuses, so the Boolean parameter (IR-1569) is the first refusal.
- Ignored stubs: `service_is_emitted_and_its_steps_hold_for_the_emitted_model` (IR-1569, accurate:
  contract-ir #1052 is merged but the pin is not bumped) and `a_reward_on_a_source_object_is_emitted`
  (QSL-2463, accurate). No stub in production code.
- Admission corpus: `Reward` fixture added and its `NOT_EMITTED_BY_QSL` exemption removed.
- `a_checked_claim_and_a_reward_are_both_lowered` (TC-621) replaces a test of the removed
  `first_declaration` API. The tag stays correct (FR-186 Outputs / FR-498).
- Tautologies: none found; the assertions compare emitted wire to the checked graph.
- Code with no owning requirement: none; each new item cites FSpec FR-640 / FSL FR-740.

## Verdict

The delivered emission is well covered at the node level and the IR strict reader admits it. The
gaps are trace accuracy (FND-001, FND-002), a reorder/identity test that proves less than its doc
and less than arch ruling (2) asks (FND-003), a narrow Service test that carries the full AC-1 tag
(FND-004), and an unrecorded partial rebuild of `Reward` (FND-005). All are fixable in this PR
except the claim-identity clause, which belongs to QSL-2441 and only needs to be stated as
deferred.
