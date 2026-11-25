---
id: SR-20261125172633
title: "gap-analysis of PR 3948 undefined state-graph guard, terminal and constraint"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; PR 3948 head e2e458067d7d47f92708b2ebfb029090d3d342b0; git diff origin/main...HEAD"
review_set: subset
---

## Summary

Ticket: QSL-3360. Gap analysis scoped to the PR diff. `quire matrix --scope .` was computed over the worktree (repo-level `traceability:` model; symbol-id collision warnings on unrelated files only). Criteria in scope: FR-168-AC-11, FR-168-AC-12, FR-168-AC-13, FR-158-AC-3, FR-170-AC-7, FR-169-AC-6, and FR-167 "Open nodes" (`ConstraintBoundary`), FR-169 V-6 (`ConstraintReached`), FSpec FR-391-AC-14.

## Verdict

CONDITIONAL. Every FSL criterion in scope is tagged (matrix status `tagged`): FR-168-AC-11 by `state_graph.rs:2093`, AC-12 by `:2121`, AC-13 by `:2167`, FR-170-AC-7 by `replay_contracts.rs:282` and `analyze_reach_rulings.rs`, FR-158-AC-3 by the two new `state_graph.rs` tests among others. No stub; the new production paths (constraint cut, subject-undefined, boundary count, constraint locus in replay) each have an owning requirement. Trace tags of the four new tests: `undefined_subject_evidence_replays_from_its_expression_locus` (TC-612, FR-170-AC-7) correct; `a_package_state_graph_item_refutes_on_an_undefined_subject_expression` (TC-612, FR-170-AC-7) correct but partial (it asserts the refuted label and the `where` shape, not the locus); `a_boundary_state_of_the_selected_constraint_is_open_and_never_expanded` and `a_state_graph_request_naming_no_constraint_is_refused` (TC-581, FR-158-AC-3) are partial bindings: FR-158-AC-3 is the temporal engine's criterion over ADR-021 §7.3, and the state-graph behaviour they prove (FR-167 `ConstraintBoundary`, FR-169 V-6 `ConstraintReached`) has no criterion of its own (FND-002).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-168-AC-11 is only one-third tested: its tagged test runs `ReachesTwo` alone; the `always possible CanReachTwo` arm and the `ReachesThree` arm (undefined evidence preferred over a trap at the same node, node `va = 2` open `UndecidedSuccessor`) and the per-item `refuted` settlement are not asserted by any test, and they are the verdict-ordering half of the criterion | fsl-analyze/tests/it/state_graph.rs:2090-2115 |
| FND-002 | low | FR-167's `ConstraintBoundary` open cause and FR-169 V-6's `ConstraintReached` mapping for state-graph items have no acceptance criterion; the new tests bind them to the temporal FR-158-AC-3 | fsl-analyze/tests/it/state_graph.rs:2202, spec/functional/FR-167-explore-a-state-graph-subject-and-track-open-nodes.md:147 |

## Coverage

- Criteria in scope: FR-168-AC-11, FR-168-AC-12, FR-168-AC-13, FR-158-AC-3, FR-169-AC-6, FR-170-AC-7: all tagged.
- Untagged behaviour: FR-167 `ConstraintBoundary`, FR-169 V-6 `ConstraintReached` (no AC).
- Reverse gap: none in the diff.
- Stubs: none.
- Semantic review: skipped (not requested).
- Plan completion: not assessed
