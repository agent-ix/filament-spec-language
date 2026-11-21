---
id: SR-2026112100331600000000002
title: "Gap analysis — QSL-2643 PR #3231 event-position hyper form (FR-173-AC-3, AC-6; FR-640-AC-12)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; PR #3231 at 768d5a6012: FR-173-AC-3, FR-173-AC-6, FR-640-AC-12 against fsl-analyze/tests/it/{hyper_closed,analyze_family_cli}.rs and fsl-semantics/tests/it/hyper_clauses.rs"
review_set: subset
evaluated_revision: "768d5a6012"
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-173
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-640
    type: references
---

## Summary

Ticket: QSL-2643. Plan completion: not assessed. This is a manual AC-to-test check for the criteria the PR claims; a full-repo `quoin matrix` run was not made. I read each tagged test and judged whether it can fail when the AC is broken. I ran the new tests, name-filtered, at the PR head.

## Verdict

**NOT MERGEABLE.** AC-3 is covered. AC-6 is covered for HP-2, HP-3 and HP-6 on the fixtures it names, but it is unmet in two places. Its "replays under its own `temporal_bindings` row" clause has no implementation and no test. Its HP-5 half is covered at the engine only (the route is unbuilt, which is declared). FR-640-AC-12's analyze test accepts either verdict.

## Coverage

| Criterion | Test | Result |
| --- | --- | --- |
| FR-173-AC-3 (event-position `Universal`, fixed-sample no form) | `fsl-semantics/tests/it/hyper_clauses.rs::a_single_existential_clause_admits_no_match_and_the_event_profile_classifies` | covered |
| FR-173-AC-6 HP-2 refuted, four positions, no loop, replays | `hyper_closed.rs::an_event_position_universal_clause_is_refuted_by_closed_traces` | covered, except the per-row replay (FND-001) |
| FR-173-AC-6 HP-2 proved | `hyper_closed.rs::an_event_position_universal_clause_that_holds_settles_proved` | covered |
| FR-173-AC-6 HP-3 proved / refuted at 0 | `hyper_closed.rs::an_event_position_forall_exists_clause_settles_by_its_witness_search` | covered |
| FR-173-AC-6 HP-6 proved / refuted projected, closed, no loop | `hyper_closed.rs::an_event_position_projection_aligned_clause_reaches_a_verdict` | partial: "no loop" and "closed" are not asserted on the projected counterexample (FND-002) |
| FR-173-AC-6 HP-5 proved / refuted | `hyper_closed.rs::an_event_position_single_existential_clause_is_witnessed_or_trapped` | engine outcome only, no settlement (route unbuilt, declared) |
| FR-640-AC-12 analyze and replay per trace | `analyze_family_cli.rs::a_hyper_clause_over_two_models_analyzes_and_replays_per_trace`, `::a_two_model_hyper_counterexample_replays_through_the_engine` | weak (FND-003) |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | No code reads and no test asserts FR-173-AC-6's "`fsl_replay::replay` reads each as a closed trace under its own `temporal_bindings` row" | fsl-analyze/src/analyze/hyper_route.rs:66-70; fsl-analyze/tests/it/hyper_closed.rs:52-87 |
| FND-002 | medium | The HP-6 test does not assert that the projected counterexample has no loop and no terminal-stutter step, which FR-173-AC-6 requires ("two closed traces, no loop") | fsl-analyze/tests/it/hyper_closed.rs:160-195 |
| FND-003 | medium | The FR-640-AC-12 analyze test accepts `proved` or `refuted` for a clause that AC-6 requires to settle `refuted`, so it cannot fail on a wrong `proved` | fsl-analyze/tests/it/analyze_family_cli.rs:907 |

## Findings detail

- **FND-001.** See code-review SR-2026112100331600000000001 FND-004 for the implementation side. The test side is that no assertion distinguishes a replay under the two rows from a replay under one shared system.
- **FND-002.** Add `assert!(trace.steps.iter().all(|s| matches!(s, HyperStep::Model(_))))` for each projected trace, and assert that its kind carries no loop entry.
- **FND-003.** Assert `verdict == "refuted"` and `item["replay"]["reproduced"] == true` without the `if`.

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | Engine, letters and replay now run each trace over its own alias's model system (`alias_system`/`for_alias`; tests use `seams.per_trace()`). The AC-12 CLI test asserts `replay.reproduced == true` on a refuted two-model clause. The event-position row's only content, `sequence_authority`, is that model's identity, so per-model systems are the per-row reading. |
| FND-002 | fixed | `an_event_position_projection_aligned_clause_reaches_a_verdict` asserts kind `Closed`, which has no loop, and that every step of both traces is `HyperStep::Model` (no stutter). The naming mismatch with AC-6's "projected" is code-review FND-013. |
| FND-003 | fixed | `analyze_family_cli.rs:907-909` asserts `verdict == "refuted"` and `item["replay"]["reproduced"] == true` unconditionally. |
