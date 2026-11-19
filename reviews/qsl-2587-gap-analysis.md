---
id: SR-2026111920260700000002
title: "Gap analysis of QSL-2587 PR #3084 export state-clause lowering"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language@48774e5d8bfcd8925f14b8b3871ea9b595b2fe39; git diff origin/main...HEAD (PR 3084); FR-1015-AC-5/6/7, FR-865-AC-7, FR-1016-AC-1, FR-1017-AC-1, FR-1035-AC-1..4; FSpec FR-812-AC-1/6/7, FR-813-AC-5, FR-814-AC-3/5 at agent-ix/filament-specification@6b98292e; QSL-2587"
review_set: subset
date: 2026-11-19
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-1015
    type: reviews
  - target: ix://agent-ix/filament-spec-language/FR-865
    type: reviews
  - target: ix://agent-ix/filament-spec-language/FR-1035
    type: reviews
---

## Summary

Ticket: QSL-2587. This is a manual check of acceptance criteria against tests, scoped to the ACs the PR's tests claim or un-ignore. `quoin matrix` was not run because the box is thermally constrained, and the house rules forbid whole-target test runs. Plan completion: not assessed. Under the name filters, the PR merged with main passes 31 tests and ignores 34.

## Verdict

Coverage is partial, and two tags are wrong.

Correctly bound:
- FR-1015-AC-5's first sentence (`only_what_the_target_cannot_state_is_unrepresented`).
- FR-1015-AC-6 (`an_invariant_among_refused_names_is_preserved`).
- FR-865-AC-7's file and incomplete clauses (`an_export_outcome_renders_as_four_lines`, `an_incomplete_export_renders_its_limit`).
- FSpec FR-814-AC-3 (`equal_requests_give_equal_text` plus `assert_canonical`).
- FSpec FR-812-AC-7's no-snapshot and snapshot `Init` (`init_without_a_snapshot_is_the_well_typedness`, the counter tests).

Not covered:
- FR-865-AC-7's no-output-file clause (`bytes written: 0`, `output: none`) has no test.
- FR-1016-AC-1, FR-1017-AC-1 and FR-1035-AC-1..4 remain ignored. That is honest, since the spine refuses at `model/field` and bounds are not read. However, the code now silently drops a `bound` rather than refusing it (code-review SR FND-001).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `a_lowered_text_over_the_byte_limit_is_incomplete` is tagged FR-1015-AC-7 but asserts the counter `incomplete` with exit 22, which is FR-1015-AC-2 / FSpec FR-817-AC-10. AC-7 is `SlowEnough` below the floor with exit 21. | tests/it/export_rulings.rs:182-191 |
| FND-002 | medium | `an_unlowered_state_construct_is_a_mapper_failure` is tagged FR-1015-AC-5, which says nothing about refusing a spine invariant. | tests/it/export_state_lowering.rs:183-195 |
| FND-003 | low | FR-865-AC-7's second clause ("over the same export with no output file named its last two lines are `bytes written: 0` and `output: none`") has no test. The renderer's `"none"` fallback is therefore unasserted. | fsl-inspect/src/render.rs:539-545; tests/it/plan_export_surface.rs:766-803 |
| FND-004 | low | The counter Lean test is tagged FSpec FR-813-AC-5 ("The Lean file of AC-1 and AC-2", the temporal `always holds` exports), but it exports a state invariant. The tag overstates coverage of FR-813. | tests/it/export_state_lowering.rs:89-94 |
