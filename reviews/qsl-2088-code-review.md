---
id: SR-1495
title: "QSL-2088 code-review of timed, protocol and monitor behavior cases"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; QSL-2088; timed_model, protocol_slots and monitor_plan behavior_boundaries at 0d04807b257efac20dbd1503095727a7c1a8ba9b"
review_set: subset
---

## Summary

Reviewed the six changed test/registration files at frozen head `0d04807b257efac20dbd1503095727a7c1a8ba9b`. Ticket: QSL-2088. This is one scoped diff review; no production implementation or requirements changed.

## Verdict

PASS for the reviewed diff. No defects found. This is not a repository-wide assurance or ticket-completion verdict. The final affected-component gate is reserved to the lead after review.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Review evidence

The source-checking and protocol helpers lead into real S1/S2/S3/link/evaluation APIs. The monitor cases select `real_time = true`, so the source declares its time binding rather than using the helper's synthetic-time branch. Exact expected ratios, integer buffer ceilings, mirrored comparison enums, finite Boolean truth tables and supplied frozen capture values are independent of the production result. Wrong-kind admission checks its typed catalogue/locus and unchanged meter. The budget test checks a typed WorkUnits stop at work−1, a correct Boolean completion at work, and equal ordered admitted charges. Fixed fixture indexing is guarded by source/shape assertions. No production, CI, public API, dependency, unsafe, warning suppression or external copied fixture changes occur.

The author's accepted receipt records 12 new active tests passed: monitor 4, timed 4, protocol 4. Initial S1 fixture errors and their confined corrections are preserved in the author receipt. Passing cases were not rerun by this reviewer. Known unchanged semantics/analyze lint blockers and compiler warnings remain explicitly unverified here, with no green Clippy claim.

## Coverage

Plan completion: not assessed

- `fsl-semantics/tests/it/timed_model.rs` and `timed_model/behavior_boundaries.rs`.
- `fsl-eval/tests/it/protocol_slots.rs` and `protocol_slots/behavior_boundaries.rs`.
- `fsl-analyze/tests/it/monitor_plan.rs` and `monitor_plan/behavior_boundaries.rs`.

The existing parent helpers were read only as needed to establish the real public path. The review used the Rust style/testing/implementation rules and repository AGENTS.md conventions. By explicit user/lead direction no matrix, repository inventory, broad profile audit, tests, builds, lint, or Done-ticket recheck ran. 

## Examined requirement facets

The excerpts below delimit the exact facets examined, including clean units. Multi-facet acceptance criteria remain multi-facet; these tests claim no exhaustive completion of them.

| Unit | Role | Path | Excerpt |
| --- | --- | --- | --- |
| FR-230 | examined | spec/functional/FR-230-check-time-declarations-clocks-and-clock-constraints.md | S3 confines every clock read to an atomic clock constraint, so no real value reaches the value kernel. |
| FR-230-AC-2 | examined | spec/functional/FR-230-check-time-declarations-clocks-and-clock-constraints.md | `x >= -1 ms` (`invalid_timed_model`/`negative-clock-constant`) |
| FR-230-AC-4 | examined | spec/functional/FR-230-check-time-declarations-clocks-and-clock-constraints.md | S3 converts the guard to two convex parts, `x < 1` and `x > 2`, each with the data predicate `self.ready`. |
| FR-230-AC-9 | examined | spec/functional/FR-230-check-time-declarations-clocks-and-clock-constraints.md | `within(self.x, 3 ms) and within(self.y, 1 ms)` expands to two constraints, and the checked package holds no node for `within`. |
| FR-230-AC-10 | examined | spec/functional/FR-230-check-time-declarations-clocks-and-clock-constraints.md | With `function pick(a: Clock, b: Clock, left: Boolean): Clock { if left then a else b }`, the guard `pick(self.x, self.y, self.l) >= 1 ms` checks to two convex parts, `x >= 1` with the data predicate `self.l` and `y >= 1` with `not self.l` |
| FR-251 | examined | spec/functional/FR-251-derive-a-tick-based-monitor-plan-for-an-embedded-target.md | converts each dense constant to ticks with sound rounding, sizes each buffer from a caller-set maximum event rate |
| FR-251-AC-3 | examined | spec/functional/FR-251-derive-a-tick-based-monitor-plan-for-an-embedded-target.md | A 16-bit target with `max_reading_gap` 65,536 refuses `GapExceedsWidth`. |
| FR-390 | examined | spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md | Evaluation reads the package and the supplied values and nothing else. |
| FR-390-AC-2 | examined | spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md | A capture is a frozen value: a capture slot returns the value its initializer gives from its stage's supply, and a later slot reads only the value the caller supplies for it. |
| FR-390-AC-6 | examined | spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md | A value of the wrong shape, a record of another type, a value outside its type's domain, a reference to another model's population and a dangling reference each refuse with the code and cause TC-1071 lists |
| FR-390-AC-11 | examined | spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md | a refused call leaves the meter as it was. |

## Trace bindings

- `timed_model::behavior_boundaries::fractional_disjunction_retains_both_clock_parts_and_their_data_guard` → `FR-230-AC-4`: correct for the tested facet.
- `timed_model::behavior_boundaries::reversing_a_clock_comparison_mirrors_all_five_relations_exactly` → `FR-230`: correct for the tested facet.
- `timed_model::behavior_boundaries::nested_clock_selection_and_predicate_calls_keep_each_selected_place` → `FR-230-AC-9`: correct for the tested facet.
- `timed_model::behavior_boundaries::nested_clock_selection_and_predicate_calls_keep_each_selected_place` → `FR-230-AC-10`: correct for the tested facet.
- `timed_model::behavior_boundaries::a_negative_fractional_clock_bound_refuses_at_its_written_quantity` → `FR-230-AC-2`: correct for the tested facet.
- `monitor_plan::behavior_boundaries::exact_fractional_bounds_convert_to_each_targets_tick_unit` → `FR-251`: correct for the tested facet.
- `monitor_plan::behavior_boundaries::buffer_capacity_rounds_up_after_both_tick_cover_and_uncertainty` → `FR-251`: correct for the tested facet.
- `monitor_plan::behavior_boundaries::counter_gap_admission_distinguishes_the_last_tick_from_a_full_wrap` → `FR-251-AC-3`: correct for the tested facet.
- `monitor_plan::behavior_boundaries::invalid_rate_uncertainty_and_unit_never_produce_a_plan` → `FR-251`: correct for the tested facet.
- `protocol_slots::behavior_boundaries::activation_helper_calls_match_the_complete_boolean_truth_table` → `FR-390`: correct for the tested facet.
- `protocol_slots::behavior_boundaries::a_structured_activation_source_refuses_wrong_shape_before_helper_execution` → `FR-390-AC-6`: correct for the tested facet.
- `protocol_slots::behavior_boundaries::a_structured_activation_source_refuses_wrong_shape_before_helper_execution` → `FR-390-AC-11`: correct for the tested facet.
- `protocol_slots::behavior_boundaries::recovery_uses_each_supplied_frozen_capture_and_activation_flag` → `FR-390-AC-2`: correct for the tested facet.
- `protocol_slots::behavior_boundaries::a_helper_call_resumes_only_with_a_sufficient_fresh_work_budget` → `FR-390`: correct for the tested facet.

