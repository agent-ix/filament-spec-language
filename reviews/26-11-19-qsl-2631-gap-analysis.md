---
id: SR-980
title: "gap analysis of PR 3134 against FR-488, FR-517-AC-25/26 and FR-317-AC-7/8"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language@56f5af1d3674f74f735d034cf543884996d25d3c; FR-488 Clause names and clock bindings, FR-488-AC-25/26, FR-517-AC-25/26, FR-317-AC-7/8, FR-640-AC-12"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-488
    type: reviews
  - target: ix://agent-ix/filament-spec-language/FR-517
    type: reviews
  - target: ix://agent-ix/filament-spec-language/FR-317
    type: reviews
---
# SR-980: gap analysis of PR 3134 against FR-488, FR-517-AC-25/26 and FR-317-AC-7/8

## Summary

Ticket: QSL-2631 (also QSL-2632). Plan completion: not assessed. Each AC was traced by hand to the tagged tests on the branch. FR-317-AC-7/8 and FR-517-AC-26 are covered. FR-517-AC-25 is partly covered. The compile-path stubs for FR-488-AC-25, FR-488-AC-26 and FR-640-AC-12 (one model) are still ignored as "not implemented yet", although this PR implements the rows they assert.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Ignored stubs `hyper_binding_rows_are_keyed_by_the_hyperproperty_node` (FR-488-AC-25), `hyper_clauses_sharing_a_wrapper_emit_their_own_trace_rows` (FR-488-AC-26) and `a_hyper_clause_over_one_model_derives_the_single_trace_authority` (FR-640-AC-12) assert the rows this PR emits, through the compile path that derives the FR-640 clock, and stay `#[ignore]`d; QSL-2631's "clock FR-640 derives" done-when has no running test | tests/it/clock_derivation.rs:289-322,449-481,483-520 |
| FND-002 | medium | FR-517-AC-25: no test covers the unit with a hybrid claim (count 3) or the `native-compile/1` handoff exit 0 | tests/it/checked_handoff.rs:2991-3014 |

## Scope and evidence

- FR-317-AC-7: `a_union_payload_argument_takes_the_exact_conversion_a_call_argument_takes` and `a_payload_argument_left_unconverted_is_refused_by_the_strict_reader` (fsl-package union_nodes.rs). Trace is correct.
- FR-317-AC-8: `a_union_payload_that_does_not_fit_gets_the_function_call_verdict` (fsl-semantics union_payload_widening.rs). Trace is correct.
- FR-517-AC-26: `a_clocked_hyper_clause_binds_each_quantifier_on_its_hyperproperty_node` (hyper_clause.rs) and `a_clocked_hyper_clause_handoff_carries_one_binding_row_per_quantifier` (checked_handoff.rs). Trace is correct.
- FR-517-AC-25: `a_handoff_of_a_unit_with_a_hyper_clause_passes_its_selection_check`. Trace is correct but covers only part of the AC (FND-002).
- FR-640-AC-12 two-model half: stays with QSL-2277 (in progress), and its ignore is correct.

## Verdict

No high gaps. Un-ignore the three compile-path stubs, which now have their implementation, and add the two missing AC-25 halves.
