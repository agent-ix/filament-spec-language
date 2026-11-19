---
id: SR-2026111923073200000002
title: "Gap analysis of QSL-2546 PR #3121 claim node body and path"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language@3d593a9fd6c39de2d6ba3228e0008cb0b422a4d8; git diff origin/main...HEAD (PR 3121); spec/functional/FR-498; FSpec FR-640 (filament-specification@45b5e1b6, spec/objects/interfaces/FR-640-*.md); fsl-package/src/emit/tests/probabilistic.rs; QSL-2546"
review_set: subset
date: 2026-11-19
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-498
    type: reviews
---

## Summary

Ticket: QSL-2546. This checks acceptance criteria against tests for PR #3121. It covers every `#[trace]` the PR adds or keeps for FR-498-AC-1..8 and FSpec FR-640-AC-3/8/9/10/11, read against the AC text on main.

Method: a manual AC-to-test check. The Quoin matrix was not run for this PR (the cross-repo FSpec ids are not in this repo's matrix scope). Plan completion: not assessed.

Deferred work is tracked in QSL-2641: ClaimMemberPath, FR-322-AC-472, FR-498-AC-6 and FR-640-AC-10. QSL-2547 covers the FR-740-AC-8 CheckoutAvailable half, and QSL-2551 covers FR-186 delay/reward weighting.

## Verdict

REQUEST CHANGES.

Correctly bound tests:
- `probabilistic_claim_identity_survives_reordering` covers FR-498-AC-1/2/4.
- `a_claim_binds_the_workload_it_is_stated_under` covers part of FR-498-AC-5 and FR-640-AC-3.
- `a_claim_binds_every_random_parameter_of_its_subject` covers part of FR-498-AC-5 and FR-640-AC-3.
- `a_claim_that_reads_a_reward_changes_identity_with_the_reward_expression` covers FR-640-AC-3 and is now un-ignored.
- `a_claim_binds_its_fairness_set` covers part of FR-640-AC-8 and FR-640-AC-3.
- `the_reader_refuses_a_malformed_claim_body` covers part of FR-640-AC-9.

Wrong or overclaimed tags:
- The PR tags two criteria it does not deliver: FR-498-AC-8 (QSL-2641 scope) and FR-498-AC-6 (deferred to QSL-2641).
- It tags FR-498-AC-7 although that AC's interval case fails (code-review FND-001), and it tests that AC's reorder clause only with identical formulas.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | `a_claim_path_moves_with_what_it_states_and_not_with_how_it_is_written` is tagged `FR-498-AC-8`. AC-8 is the `PunctualInterval`/`TimedFormShape` cause member path (`ClaimMemberPath {body, formulas}`), which is not delivered (QSL-2641), and the test asserts none of it. It asserts FR-498-AC-7 content (threshold, direction, confidence, spelling). | fsl-package/src/emit/tests/probabilistic.rs:2553-2603 |
| FND-002 | medium | `a_claim_binds_the_reward_its_form_reads` is tagged `FR-498-AC-6`, which the ticket defers to QSL-2641. The test uses the ConfigVersion `cost` integer reward. It covers none of AC-6's `duration` (ms) forms, `P95`, the `2 ms` rewrite, or the timed `delay` half. The tag inflates the matrix for a deferred AC. | fsl-package/src/emit/tests/probabilistic.rs:1298-1336 |
| FND-003 | high | FR-498-AC-7 is tagged on two tests, but its listed change "its event to `always[0,3] holds(true)`" is never asserted: the test substitutes `eventually[0,2]`. A probe shows that change leaves the node_id unchanged (code-review FND-001), so a tagged AC fails on an omitted sub-case. The AC's `P95` quantity sub-cases (`5 ms` to `6 ms`, `0.005 s`, `rational(19, 20)`) and the `on each (m: Service::Server)` change are also unasserted. | fsl-package/src/emit/tests/probabilistic.rs:2505-2650 |
| FND-004 | high | FR-640-AC-3 "Writing the four claims ... in another order changes no byte" is tested only by permuting claims whose formulas are identical (`HOLDS`). With formulas that differ in an interval, a reorder changes both claims' ids (code-review FND-002). The weight-line reorder clause is untested: `Steady` has one weight line. The test's doc comment claims both. | fsl-package/src/emit/tests/probabilistic.rs:1100-1182 |
| FND-005 | medium | The `FSpec-FR-640-AC-9` tag (rule 6 refusals and reader budget) sits on three tests that assert other ACs. `a_claim_states_its_form_over_activation_and_confidence_in_body_and_path` asserts the binding order and statement spelling, which is FR-640-AC-8/AC-10 content. `an_on_each_claim_states_its_trigger_and_guard` asserts activation spelling, which is AC-10 content. `the_reader_refuses_a_claim_statement_that_breaks_rule_7` asserts rule 7 refusals, which are FR-640-AC-11. | fsl-package/src/emit/tests/probabilistic.rs:2505,2609,2658 |
| FND-006 | medium | No test exercises a timed or quantity statement path. Untested: `elapsed` and `expected_elapsed` forms, `within`/`window` as a time quantity, and a quantity `bound` at the reward's unit node (`reward_unit`). FR-498-AC-7's `P95` `5 ms` bound and FR-640-AC-11's tick-model case are therefore unverified, and the AC-11 tick case is where code-review FND-003 bites. | fsl-semantics/src/check/lowering/probabilistic.rs:603-616,650-670 |
| FND-007 | low | The FR-498-AC-5 rename clause ("Renaming `A` to `B` gives two claims named `B` with different paths, each at ordinal 0") is unasserted, although the test is tagged AC-5. FR-940-AC-9 refuses a second claim of one name, so this sub-case may conflict with that rule; flag it to spec rather than test around it. | fsl-package/src/emit/tests/probabilistic.rs:1100-1182; spec/functional/FR-498 AC-5 |
