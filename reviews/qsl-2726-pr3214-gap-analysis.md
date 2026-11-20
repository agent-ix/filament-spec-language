---
id: SR-2026112019364602
title: "gap-analysis of PR 3214 package route batch 2 second PR"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language@aaa5643860cf44b49faa592a17348f077832d009; git diff origin/main...HEAD; FR-620, FR-931, FR-940, FR-1039 and the FSpec FR-322/FR-789 emitter rows the PR names"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-620
    type: reviews
  - target: ix://agent-ix/filament-spec-language/FR-931
    type: reviews
  - target: ix://agent-ix/filament-spec-language/FR-1039
    type: reviews
---
# SR-2026112019364602: gap-analysis of PR 3214

## Summary

Ticket: QSL-2726. PR agent-ix/filament-spec-language#3214, head aaa5643860. Plan completion: not assessed (planless). Manual acceptance-criterion-to-test check over the ACs the PR claims; `quoin matrix` was not run over the frozen worktree.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Package default items (FR-620) differ from the source route's: one item per claim with a single fixed kind, no task-set claims, and only the deadlock-freedom derived item. The source route gives one item per admitted kind in Capability::ALL order, then task-set claims, then FR-368 (or protocol) derived items. Only the single-kind Counter fixture is tested | fsl-analyze/src/analyze/document/package_run.rs:1505-1555; fsl-analyze/src/analyze/document.rs:2358-2460 |
| FND-002 | medium | Three new emitter tests are traced to ACs they do not assert. `a_source_clock_parameter_emits_its_constraint_and_reset` is traced to FR-230-AC-65 (clock places w.c and d). `a_source_tuple_and_union_clock_place_keep_the_source_spelling` is traced to FR-996-AC-1 (d and w.c rows). `a_source_operation_declares_its_effective_precondition_function` is traced to FR-569-AC-6 (object_type nodes). The matrix counts those ACs as covered by tests that check other behaviour | fsl-package/src/emit/tests/source_objects.rs:1697,1732,1767 |
| FND-003 | medium | The ignored test `a_seq_source_inside_a_clause_emits_a_package_the_strict_read_admits` keeps its FR-931-AC-16 trace tag. An ignored test is counted as coverage for an AC it never runs | tests/it/request_surface_s1_16.rs:2076-2083 |

## Scope and evidence

- FR-931-AC-18 / FR-1039-AC-6 (examined): `the_file_form_prints_the_source_routes_atoms` is un-ignored and covers `{file}` and `{file, equality}`. The print mode's walk falls back to declaration nodes.
- FR-931-AC-16 (examined): the package half and the alias-removed copy are `a_named_count_type_argument_prints_over_the_package`, and both compare byte-equal with the source route.
- FR-1039-AC-5 (examined): `an_equality_function_runs_over_a_file_with_its_equality_file` covers both the `{file, equality}` run and the `{file}` `missing-file` refusal.
- FR-620 default items (examined): see FND-001.
- FSpec FR-789-AC-4, FR-322 Clause functions item 5, FR-322 Population reference item 4 (examined): the emitter matches the quoted spellings. FR-322 item 4 asks for `{qualified_name: [<population name>]}` with SourceOwner, which `population_reference_node` writes.

## Verdict

Gaps: FND-001 is a specified behaviour that is claimed done but only partly met. FND-002 and FND-003 inflate the matrix.
