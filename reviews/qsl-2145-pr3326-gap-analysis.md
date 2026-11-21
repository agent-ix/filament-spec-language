---
id: SR-2026112115323300000002
title: "Gap analysis of QSL-2145 owner routing against FR-944 and FR-861 (filament-spec-language PR 3326)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language@1ebf1a70f4b58987bb09e928fd2682a43400a3e0; gh pr diff 3326; FR-944 AC-2..AC-4, AC-8..AC-16; FR-861 AC-1..AC-4"
review_set: subset
date: 2026-11-21
---

## Summary

Ticket: QSL-2145. This is a manual AC-to-test check over the diff; no Quoin
matrix run was made (the brief allowed no builds or test runs). It covers
FR-944 and FR-861 ACs touched or claimed by the PR body. Plan completion: not
assessed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | Trace tags over-claim. `completeness_revision_vectors_keep_boundary_and_fact_regressions_distinct` is tagged FR-944-AC-4 but checks only BoundaryRegressed, CompleteRegressed and one Incomplete acceptance against the owner directly. It does not check UncoveredMember, MemberObservationChanged, the relabelling check, acceptance of 100/140, or agreement with FR-502's ledger. The observation_owner activation test is tagged FR-944-AC-3, but no repair plan exists. `caller_admits_the_fixed_producer_selection` (producer.rs) is tagged AC-8 but asserts AC-9's bundle content. `bare_reader_route_keeps_the_exact_admitted_package_bytes` is tagged AC-2 but asserts AC-10 content | fsl-eval/src/temporal/evidence/owner.rs:1663; fsl-eval/tests/it/observation_owner.rs:334; fsl-eval/src/temporal/evidence/producer.rs:45; tests/it/bare_package.rs:18 |
| FND-002 | medium | The PR body lists FR-944-AC-10 as done, but no test evaluates the bare route with retained bytes. Nothing compares the owner `package` selection or the `Context` authorities between the source and bare routes, and the bare route has no time-unit scales (see the code-review SR, FND-003). Only byte retention is tested | tests/it/bare_package.rs:18-33; fsl-eval/src/temporal/evidence/package.rs:344-350 |
| FND-003 | low | Spec/code mismatch: FR-944-AC-8 says a position with no arrival carries `ingestion_time_nanos` equal to its event time under the interim rule, but the code passes `None` (the owner's unrecorded ingestion instant). Either AC-8 needs updating through the spec lane, or the code must follow it | fsl-eval/src/temporal/evidence/owner.rs:192-197 |

## Scope and evidence

- FR-944-AC-2, AC-11, AC-12, AC-13: unit tests in owner.rs. The record value
  shape, undefined atoms with no partial fact, `ms/4` units and the
  nonintegral-ns refusal are present and tagged correctly.
- FR-944-AC-9: the owner.rs test tagged AC-9 covers the limit refusals
  (`node-count-exceeded`, `input-bytes-exceeded`).
- FR-944-AC-14: the observation_owner and owner.rs activation tests cover the
  empty-scope (`without_trigger`), capture-record ordering and incomplete or
  missing evidence (no completeness view). The code matches the AC text.
- FR-944-AC-15: the syn-based scan test and the required parameter in
  `evaluate_temporal_evidence` (compile_fail doctest) are present.
- FR-944-AC-16: the completeness selection over all members and the boundary
  identity are tested at unit level.
- FR-861-AC-1..AC-3 (T-064 agreement) and FR-861-AC-4 (scan test still
  `#[ignore]`) are unwritten, as the PR's Not-done list states. The owner's
  `completeness::revise` and `repair::plan` exist at rev 1e356f9 (the owner.rs
  test calls `revise`), so no upstream symbol blocks the remainder.

## Verdict

AC coverage is partial, as the PR declares. Fix the tags (FND-001), and either
test AC-10 or drop it from the Done list (FND-002). Route FND-003 to the spec
lane.
