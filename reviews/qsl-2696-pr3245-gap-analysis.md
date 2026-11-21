---
id: SR-2026112102482702
title: "QSL-2696/QSL-2697 PR 3245 gap analysis: FR-512 AC-to-test check"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language@a4de58ba954a55ff537d5a0dfcd2b358e5c513f6; spec/functional/FR-512-keep-operation-names-parameters-and-frame-entries-unique-at-intake.md AC-4,5,7,8,9,10,11,12,13,15,16; fsl-semantics/tests/it/operation_uniqueness.rs"
review_set: subset
---

## Summary

Ticket: QSL-2696, QSL-2697. This was a focused manual AC-to-test check for FR-512 AC-4, 5, 7, 8, 9, 10, 11, 12, 13, 15 and 16. The quoin matrix sweep was not run. Plan completion: not assessed. Every AC in scope has a tagged test whose assertion can fail. The gaps are owners that the "Unique by (owner, name)" rule covers but no test exercises, plus one record/event precedence case.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | No test covers an operation-name collision under a state machine, process or persistence-interface owner. The FR-512 Behavior rule covers every admitted owner. The state-machine case is broken (code-review FND-001), and the process and persistence-interface call sites are new and unexercised. | fsl-semantics/tests/it/operation_uniqueness.rs:101-115 |
| FND-002 | medium | No test checks that a record or event owner refusal precedes a `result` or mismatched parameter name in one of its operations. FR-512 Order and AC-16 say the owner rule is reported first, and the PR currently refuses the parameter at intake instead (code-review FND-002). | fsl-semantics/tests/it/operation_uniqueness.rs:496-546 |
| FND-003 | low | `repeated_frame_entries_deduplicate_without_losing_distinct_entries` is tagged FR-512-AC-12 but now also loops over `creates`/`deletes`. That is AC-13 content, already covered by the two AC-13 tests. The test name promises distinct entries are kept, but the test asserts none (that is covered by `two_distinct_modifies_grants_keep_their_declared_order`). | fsl-semantics/tests/it/operation_uniqueness.rs:201-207 |

## Scope and evidence

AC to test bindings checked. Each test asserts the code, the cause tag, the key node and the member path, unless a different assertion is noted:
- AC-4: `duplicate_operation_identity_refuses_at_the_second_operation` (operations[1]).
- AC-5: `equal_operation_names_with_distinct_identities_refuse_at_the_second_name` (operations[1].name, both identities in the detail).
- AC-7: `duplicate_parameter_identity_preempts_an_unresolved_frame` (params[1], DUPLICATE_PARAM retained).
- AC-8: `duplicate_parameter_names_preserve_the_readers_exact_repeated_name` (one refusal, the reader pointer, params[1].name).
- AC-9: `mismatched_parameter_name_preempts_frame_resolution` (params[0].name).
- AC-10 and AC-11: `result_parameter_is_reserved_with_and_without_a_return_type` and `result_parameter_without_a_return_type_refuses_before_frame_resolution` (params[0].name, with and without `returns`).
- AC-12: the receiver and modifies dedup tests compare the effect with the one-entry effect. `two_distinct_modifies_grants_keep_their_declared_order` covers the declared order.
- AC-13: the creates, deletes and deletes_parameters dedup tests.
- AC-15: the three preemption tests above, each with an unresolved `modifies`.
- AC-16: `repeated_operation_name_preempts_the_second_operations_reserved_parameter` and `event_owner_`/`record_owner_refuses_before_operation_name_collisions`.
- All 11 removed `#[ignore]`s cite QSL-2696 or QSL-2697. Trace tags are bare `TC-1307`/`FR-512-AC-n` ids. No test was weakened.

## Verdict

CONDITIONAL. AC coverage is complete for the main owner (object type). Add tests for FND-001 and FND-002 alongside the code fixes.
