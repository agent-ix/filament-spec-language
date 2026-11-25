---
id: SR-2026112574410382
title: "spec-review of QSL-3138 FR-640 timed protocol clock selections"
type: SpecReview
analysis: integrity
scope: "agent-ix/filament-spec-language; PR #3920 at 88b436fd (commit d3ff1e98); spec/functional/FR-640-select-clock-configurations-in-compile-and-run-requests.md, spec/test-cases/TC-1660-clock-selections-compile-and-run-trace-subject-clauses.md"
review_set: subset
---

## Summary

Ticket: QSL-3138. Spec review of the FR-640 sentence, the new FR-640-AC-15 and TC-1660 step 5 that commit d3ff1e98 adds for timed-profile `compensate` and `await` clock selections, checked against FSpec FR-322 "Temporal clock bindings" at a6968fb8 and against the emitter. Nothing was compiled or run for this review.

## Examined scope

- FR-640 new bullet (examined): "A `compensate` template or an `await` under the timed profile takes its selection the same way, and the package holds no `temporal_bindings` row for it, since the checked package admits none for a protocol declaration under that profile (FSpec FR-322 "Temporal clock bindings"). The emitter SHALL check the selection's kind and unit against the declaration, refuse a mismatch as for any other selection, and write no row."
  - Owned by FSpec. FR-322 creates rows for a compensation or await only when the lock's `temporal_profile` selects one of "the three clock-configured profiles", and `filament.temporal.timed/v1` is not one of them. "No row" is therefore FSpec's rule, and the pinned reader admits that package (AC-15 test).
  - The new part is FSL request handling: a supplied `timed` selection is accepted and checked instead of refused as surplus. That belongs to FSL's FR-640 and does not contradict FSpec.
- FR-640-AC-15 (examined): "A protocol under the timed profile with a `compensate` template and an `await`, emitted with a `timed` selection for each, is admitted by the strict reader and holds no `temporal_bindings` member, as is the same protocol emitted with no selection; event-position selections for them refuse as a profile mismatch." Traced to `a_timed_protocol_clock_selection_is_checked_and_writes_no_row` (tagged TC-1660 and FR-640-AC-15), which asserts each clause.
- TC-1660 step 5 and its expected result (examined): they agree with AC-15.

## Verdict

The behaviour is owned and consistent: FSpec gives no row, the pinned reader admits the package, and FSL decides what happens to the request's selection. One wording defect.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The FR-640 sentence says the emitter checks "the selection's kind and unit against the declaration". The emitter checks the kind against the profile (`check_selected_definition`). It checks the unit and clock name only as well-formed values (`temporal_clock_invalid_field`). Nothing in a timed `compensate` or `await` declaration carries a unit to compare against. Fix: "SHALL check the selection's kind against the profile and its values as for any other selection, refuse a mismatch, and write no row." | spec/functional/FR-640-select-clock-configurations-in-compile-and-run-requests.md:95-100; fsl-package/src/emit.rs:10606-10615 |
