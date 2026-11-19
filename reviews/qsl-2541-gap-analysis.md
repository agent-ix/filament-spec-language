---
id: SR-2026111612172400000059
title: "Gap analysis of QSL-2541 trace tags and coverage for the repaired fsl-eval tests (filament-spec-language PR 3014)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language@131c8865b391c1e13cb8cab3a2a7bb6dc36162c1; git diff origin/main...HEAD (PR 3014); FR-187-AC-4 (TC-622), FR-187-AC-5 (TC-2064, context), FR-185-AC-11 (context); FSpec FR-411-AC-17 (TC-1561) at agent-ix/filament-specification"
review_set: subset
date: 2026-11-19
---

## Summary

Ticket: QSL-2541. The scope is limited to trace tags and coverage of the tests the PR
touches, as the brief asks. This is a manual check of ACs against tests. The computed
`quoin matrix` was not run: it cannot be filtered by name, and cargo slots are reserved
for contract-ir work. Plan completion: not assessed. Of the touched tests, only
`a_negative_reward_refuses_admission` carries criterion tags. The others are untagged
evaluator-invariant tests whose tags the PR leaves alone.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | Pre-existing and not introduced here: FR-187-AC-4 asks for a package "read from the wire whose producer S3 did not check". `a_negative_reward_refuses_admission` stands in for the non-checking producer with `ZeroRewardEvaluator` at S3, then admits on the source route (`ModelCheckInput::source`), never through emission and read-back. The tag is right in intent, and FR-187's admission refusal is exercised. The wire half of the AC (and the hand-built wire package of FR-187-AC-5) has no test. Not blocking for this PR. A follow-up could admit the emitted package on the package route | fsl-eval/src/simulation/model/probabilistic/tests.rs:366-394; fsl-eval/src/simulation/sampled/world.rs:357-371 |

## Scope and evidence

Examined:
- `a_negative_reward_refuses_admission`, tagged `TC-622`/`FR-187-AC-4` and
  `FSpec-TC-1561`/`FSpec-FR-411-AC-17`. The tags are unchanged by the PR, and they are
  accurate: the test asserts `invalid_probabilistic_model`/`negative-reward`, naming
  `Account/pay`, `refund` and `n = 0`, which is what both ACs state. The test is not weakened
  (see SR-2026111612172400000058): the evaluator change touches only S3's sign check, which
  did not exist before #3007, and admission evaluates the reward itself.
- The doc comment on `with_s3_reward_signs_unchecked` says S3's own refusal is covered in
  fsl-semantics and fsl-analyze. This is accurate: `s3_refuses_a_negative_reward_through_the_reference_evaluator`
  (TC-2061/FR-185-AC-11, TC-3027/FR-185-AC-13) and
  `fsl-semantics/tests/it/probabilistic_declarations.rs`.
- Untagged invariant tests in `invariants.rs` (`imported_callable_consumer_uses_a_real_emitted_and_admitted_library`,
  `both_ieee_profile_checks_and_conversion_operand_check_are_real_producers`,
  `query_stop_provenance_rejects_...`, `field_slot_and_record_constructor_fail_...` (TC-917,
  tags unchanged), `an_out_of_type_stored_value_is_refused_...`, and the `checked_dispatch_package`
  and `object_package` helpers). The changes are mechanical. Their assertions are unchanged.
  None is a stub or a tautology.

## Verdict

Trace tags are accurate and coverage is not reduced. The one finding is low and predates
this PR, so it does not block merge.
