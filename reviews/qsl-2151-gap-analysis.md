---
id: SR-2026111605245700000003
title: "Gap analysis — QSL-2151/2152/2153/2157 changed behavior"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; changed Rust in fsl-semantics/, fsl-package/, fsl-replay/, fsl-analyze/; FR-186, FR-195"
review_set: subset
relationships:
  - { target: "ix://agent-ix/filament-spec-language/FR-186", type: references }
  - { target: "ix://agent-ix/filament-spec-language/FR-195", type: references }
---

## Summary

This planless, change-scoped audit compared the four issue criteria with the changed Rust, local FR/TC criteria, trace tags, and targeted test evidence. The in-repository semantics/package criteria have corresponding assertions; source-level activation remains context only because QSL-2158 owns analyzer source routing and is explicitly outside this PR's acceptance scope.

## Verdict

**PASS** for the changed in-repository acceptance scope; this does not claim repository-wide matrix completion or plan completion.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

- Reconciliation: `quoin matrix --repo . --json` (quoin 0.35.0; quire 0.45.0, engine 0.60.0) — run evidence unavailable; no grep fallback.
- Plan completion: not assessed.
- Full repository matrix: 5,173 criteria; 3,660 tagged, 1,120 untagged, 282 tagged only by ignored tests, and 98 method-without-symbol; all 5,173 report no run evidence. These are repository-wide totals, not findings attributed to this PR.
- Changed-scope matrix: FR-186-AC-8, AC-9, AC-11, AC-12, AC-13, AC-14 and FR-195-AC-7 are statically tagged by active tests. The matrix reports FR-186-AC-10 as untagged; its added analyzer test has namespaced `ix_trace_rs::trace` attributes, but it fails with `unknown-claim` before checking the claim. Per the dispatch boundary, AC-10/source routing is context only and remains QSL-2158 work.
- Manual issue mapping: QSL-2151's semantics check, package activation record, and non-population refusal map to active local tests; the empty-population source test is deferred with source routing. QSL-2152's untimed, timed, and locus cases map to `expected_elapsed_untimed_is_ill_typed`, `elapsed_untimed_is_ill_typed`, `timed_window_untimed_is_ill_typed`, and `elapsed_timed_checks`. QSL-2153's timed-profile admission and standard-cause refusal map to `probabilistic_claim_timed_profile_checks` and `probabilistic_claim_timed_profile_untimed_model`; the driver-cause case is deferred to `filament-driver`. QSL-2157's dimensioned, wrong-dimension, and non-quantity positions map to `quantity_literal_positions_check_and_refuse_by_dimension`, `quantity_literal_timed_positions_check_and_refuse_by_dimension`, and `quantity_literal_non_quantity_position_refused`; the obsolete `CheckCause::Probabilistic` path is removed.
- Reverse gap in changed production behavior: 7 behavior groups reviewed (activation/type/guard checking, time-source typing, expected elapsed conversion, elapsed measures, timed windows, timed event intervals, typed refusal mapping); 0 untraced groups, 0 source stubs, 0 test stubs.
- Semantic review: skipped; not requested.

## Round 2 recheck

The final diff adds no new source or test behavior beyond the round 1 test deferral and the committed review artifacts. The same in-repository FR-186/FR-195 criteria remain mapped to active semantics/package tests. Analyzer source routing and cross-repository driver/example checks remain outside this PR's acceptance scope. No new gap was found.
