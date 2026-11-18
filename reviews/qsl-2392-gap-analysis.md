---
id: SR-2026111612172400000034
title: "Gap analysis of the FSL main baseline gate repair (filament-spec-language PR 2885)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language@282db8836beddb453c46e24e7b71b80bf9827fe6; git diff origin/main...HEAD (PR 2885); tagged tests in the diff: fsl-package/src/emit/tests/protocol_graph.rs, fsl-package/src/emit/tests/owner_model.rs, fsl-analyze/src/protocol_analysis/conformance.rs; spec/functional/FR-383, FR-880; ticket QSL-2392"
review_set: subset
date: 2026-11-18
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-383
    type: reviews
  - target: ix://agent-ix/filament-spec-language/FR-880
    type: reviews
---

## Summary

Ticket: QSL-2392. PR: filament-spec-language#2885, head 282db8836. The PR
changes no requirement and adds no behaviour; it is mechanical gate repair.
This was a manual, diff-scoped gap check: for every trace-tagged test the diff
touches, confirm the test still asserts its tagged criterion and that no tag,
test or assertion was lost. The full `quoin matrix` was not run, because the
spec and its trace tags are unchanged. `Plan completion: not assessed`.

- `an_over_input_that_is_no_object_type_writes_the_clause_by_what_it_reaches`
  (owner_model.rs, `FR-383-AC-27`, `TC-1063`) is the same as on main except for
  rustfmt. It asserts that `Boolean` and `Integer` have no `owner_model`, and
  that `M::Order`, `Option<M::Order>` and `Sequence<M::Order>` name `Order`'s
  node, which is what AC-27 requires.
- `a_non_object_over_is_written` (protocol_graph.rs, `FR-880-AC-7`, `TC-2502`)
  is still `#[ignore]` under QSL-2220, as on main. Run with `--ignored` at
  HEAD, it fails at protocol_graph.rs:2023, so the ignore is still warranted.
  AC-7 also names `Parts::Handoff`, which the test does not exercise. That gap
  is already on main and is not introduced here.
- `recording_assessment_maps_to_the_shared_conformance_output` (conformance.rs,
  `FR-990-AC-1`, `TC-2805`) moved inside the file, with its assertions
  unchanged.
- The `#[trace` count is the same as on main for every changed `.rs` file.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-383-AC-27 | spec/functional/FR-383-emit-a-checked-protocol-as-v2-protocol-nodes.md:639 | examined |
| FR-880-AC-7 | spec/functional/FR-880-write-every-checked-protocol-construct-at-s4.md:111 | examined |
| FR-990-AC-1 | fsl-analyze/src/protocol_analysis/conformance.rs:1193 (tag) | context_only |

## Verdict

**PASS.** The PR loses no coverage. The FR-880-AC-7 test stays ignored, as it is
on main, and is still blocked by QSL-2220. The code-review SR records the PR
body's claim that it was un-ignored.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |
