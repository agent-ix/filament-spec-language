---
id: SR-2026111612480002
title: "Gap analysis of QSL-1977 monitor stepping, PR 2748"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; PR #2748 head bd5dcfbf34db10a922dcb2a57560963d297c12c8: FR-643-AC-1..11, FR-644-AC-1..6, FR-283-AC-17, FR-820-AC-1..4 against fsl-analyze/tests/it/monitor_incremental.rs, tests/it/run_wire.rs and fsl-analyze/src/monitor_session.rs, fsl-command/src/command/monitor.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-643
    type: reviews
  - target: ix://agent-ix/filament-spec-language/FR-644
    type: reviews
---

## Summary

Ticket: QSL-1977. This is a planless gap analysis of the PR diff. Plan completion: not assessed. Each criterion was mapped by hand from the `#[trace]` tags in the diff, and `quoin matrix` was not used for the verdict.

Mapped and adequate:
- FR-643-AC-2, AC-3, AC-5 (watermark and closure parts), AC-6, AC-8;
- FR-644-AC-1, AC-2, AC-3, AC-4, AC-5, and AC-6 (the `end` and `through` refusal parts).

Accepted as out of scope:
- FR-643-AC-5 `Recent`, ignored under QSL-2307;
- FR-643-AC-9 and FR-820-AC-1..4, which wait on the FR-820 `ClauseState` (QSL-2307);
- FR-643-AC-10 and FR-644-AC-6 `timed`, which wait on QSL-2254.

No production code in the diff lacks an owning requirement. No stubs were found beyond the documented `assert_through` and `Timed` refusals.

## Findings

| ID | Severity | Summary | Refs |
|----|----------|---------|------|
| FND-001 | medium | FR-643-AC-4 tests compare only `(verdict, settled)` for a `Finite` resume and `finite_window` for a settled finite-window clause. No test resumes a progress-carrying session with a pending clause after an unasserted step, the case that diverges (code-review SR-2026111612480001 FND-001) | fsl-analyze/tests/it/monitor_incremental.rs:528-614, 867-954 |
| FND-002 | low | FR-643-AC-1's tagged test covers `Reaches` and `Never` over samples only. The AC's `Bounded` and `always holds(c.value <= 1)` over snapshots, with their settle steps 2 and 3, are not exercised | fsl-analyze/tests/it/monitor_incremental.rs:292-338 |
| FND-003 | low | The FR-643-AC-11 test asserts categories only. It does not check the limit record's setting `monitor.retained_identities`, `Never` citing three positions, or `Below`'s report unchanged by the further observation | fsl-analyze/tests/it/monitor_incremental.rs:758-796 |
| FND-004 | low | The FR-643-AC-7 test does not compare the settled report with `B(3)`. That comparison would expose the ledger and `finite_window` divergence (code-review FND-004) | fsl-analyze/tests/it/monitor_incremental.rs:707-756 |
| FND-005 | low | FR-283-AC-17 (TC-2323) has no tagged test. Its `closed`/`open` refusal comes from the existing closed member set of the trace document | fsl-analyze/src/monitor.rs:965 |
| FND-006 | low | `a_kind_the_profile_does_not_read_refuses_at_open` is tagged FR-643-AC-1 but tests FR-643's `open` refusal, which has no AC. It also asserts only `ClauseRefused`, not the cause `trace-kind-mismatch` | fsl-analyze/tests/it/monitor_incremental.rs:616-633 |

## Verdict

REQUEST CHANGES. Coverage is broad and the oracles are mostly strong: per-prefix batch comparison, ledger equality and contradiction dimensions. FND-001 leaves the defect in code-review FND-001 uncaught. The low findings are assertions to add to existing tests.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
|----|----------|---------|------|
| FND-007 | low | `the_opening_ledger_orders_stamps_and_assertions` is tagged FR-643-AC-4 (checkpoint), but it tests FR-643 Order (AC-5's watermark refusal, seeded from the ledger). Retag it to FR-643-AC-5 | fsl-analyze/tests/it/monitor_incremental.rs:867 |

## Dispositions

| FND | outcome | reason |
|-----|---------|--------|
| FND-001 | fixed | `a_resumed_timestamped_session_keeps_the_pending_window_report` resumes after an unasserted step and compares reports with the original. |
| FND-002 | fixed | `snapshots_fed_one_at_a_time_equal_the_batch_with_settled_clauses_frozen` (FR-643-AC-1) runs `Bounded` and `always holds(c.value <= 1)` over snapshots. |
| FND-003 | fixed | The AC-11 test now asserts `Never`'s three-position counterexample, the limit record and `Below` unchanged at the fifth observation. |
| FND-004 | fixed | The AC-7 test compares the settled report with the batch `B(3)`: the same basis, no `finite_window`, no ledger. |
| FND-005 | fixed | `a_finite_session_needs_no_selection_for_a_closed_trace_clause` is tagged TC-2323, FR-283-AC-17. |
| FND-006 | fixed | The open-refusal test is retagged TC-1671, FR-645-AC-3. |
