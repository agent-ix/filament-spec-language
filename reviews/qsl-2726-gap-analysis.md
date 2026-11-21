---
id: SR-202611210022442313144108430
title: "Gap analysis of QSL-2726 PR #3246 (cross_evaluate and clause over a package directory)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language@53e0a81714 on main d84ed5685b; FR-931, FR-1039 and their tests in tests/it/request_surface_s1_16.rs and tests/it/package_run_route.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-931
    type: reviews
  - target: ix://agent-ix/filament-spec-language/FR-1039
    type: reviews
---

## Summary

Ticket: QSL-2726. Gap analysis of PR #3246, scoped to the ACs the PR claims: FR-931-AC-18 (the lasso `cross_evaluate` over `{directory}`) and FR-1039-AC-6 (the `initial` `cross_evaluate` over `{directory}`). `quire matrix --scope <repo> --format json` ran on the PR worktree. FR-931-AC-18 is tagged by `a_tl_lowering_over_a_package_returns_the_source_record` and `a_lasso_cross_evaluation_over_a_handoff_directory_returns_the_source_record`. FR-1039-AC-6 is tagged by five tests, two of them new in this PR. Neither AC is tagged only by an ignored test. FR-1039-AC-7, FR-931-AC-15 and FR-940-AC-13/16-22 are untagged, but none is in this PR's scope.

The positive halves of both ACs are now built and pass with full-document equality. The gaps are smaller AC clauses and one behaviour with no AC.

## Verdict

**CONDITIONAL**: no in-scope AC is untagged and there is no high finding. The one medium finding is new behaviour with no owning criterion. The three low findings are two AC clauses with no test and one wrong trace tag.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | FR-1039-AC-6's clause "`{"clause": "Low", "cross_evaluate": {"initial": S, "max_steps": 2}}` over `{file, equality}` refuses `invalid-request` ... `requires: directory`" has no test. `an_initial_cross_evaluation_over_a_handoff_directory_returns_the_source_record` has no negative. The existing `{file}` negative uses a `finite` trace, and the new lasso test covers the trace form only. Today one check at `prepare_lowering` refuses both forms before `initial` is read, so the risk is low, but the AC's named request is not exercised. | tests/it/request_surface_s1_16.rs:1467-1500, 1296-1309; fsl-command/src/command/package_run.rs:365-369 |
| FND-002 | medium | The new `clause` run over `{directory}` (record samples, infinite-trace profile only) has no owning AC. Its test `a_clause_run_over_a_handoff_directory_returns_the_source_result` is tagged FR-1039-AC-6, which states only `tl_lowering` requests. The run still refuses every other clause form with "this clause run is not implemented over a selected checked package yet" (observations, non-temporal selections, progressed/timestamped/model traces, any non-infinite-trace profile), while FR-1039 Outputs require the source route's outcome document for the `clause` member. | tests/it/request_surface_s1_16.rs:1411-1413; fsl-command/src/command/package_run.rs:240-273, 325-331 |
| FND-003 | low | `a_clause_run_over_a_handoff_directory_returns_the_source_result` asserts only the `{file}` refusal. The `{file, equality}` form of the `requires: directory` refusal for `clause` is untested, although FR-1039 names both forms for a member that builds model states. | tests/it/request_surface_s1_16.rs:1444-1457 |
| FND-004 | low | Trace: `a_clause_run_over_a_handoff_directory_returns_the_source_result` is tagged FR-1039-AC-6, which covers `tl_lowering` only. The clause run has no AC (see FND-002). | tests/it/request_surface_s1_16.rs:1411-1413 |

## Coverage

- FR-931-AC-18: tagged (2 tests), with the positive lasso `{directory}` arm and the `{file, equality}` negative. Examined.
- FR-1039-AC-6: tagged (5 tests). The `initial` `{directory}` arm is byte-equal for max_steps 1 and 2 and for an empty population. The `{file, equality}` + `initial` negative is missing (FND-001). Examined.
- FR-931-AC-9 and FR-931-AC-10: context only. The refactored `a_fairness_constraint_over_a_lasso_settles_missing_fairness_premise` still passes.
- Reverse gap: the `clause` run over a package directory (FND-002).
- Stubs: none on a live path. Several forms refuse with typed "not implemented over a selected checked package yet" refusals (FND-002).
- Semantic review: skipped (not requested).
- Plan completion: not assessed
