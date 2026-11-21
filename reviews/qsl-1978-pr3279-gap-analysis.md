---
id: SR-202611210022442313144108501
title: "gap-analysis of PR 3279 source plan synthesis wiring (QSL-1978)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; PR3279 head b3b6113ff55cc85fcbadb46386156a387d62e381; spec/functional/FR-1010, FR-1011, FR-1012, FR-1013, FR-1036, FR-1037, FR-710; changed source and tests of the PR"
review_set: subset
relationships:
  - { target: "ix://agent-ix/filament-spec-language", type: references }
---

## Summary

Ticket: QSL-1978. PR filament-spec-language#3279. This audit covered the PR's owning requirements: FSL FR-1010 to FR-1013, FR-1036, FR-1037 and FR-710-AC-5, plus the FSpec criteria the new tests cite (FR-650, FR-800 to FR-806, FR-823). It ran the computed matrix, the coverage report, a reverse-gap pass over the changed source, and a stub check. The audit was repository-driven, and plan completion was not assessed.

Every live FSL criterion in scope carries a tag. The tags overstate coverage in several places. The FSpec FR-800-AC-1 binders test none of its step-record content, and the implementation cannot satisfy that criterion. One FR-1036-AC-2 binder asserts the opposite of the criterion. The FR-1013-AC-2 snapshot-refusal clause is tagged, but the implementation does not meet it and the tagged test only checks the pointer. Several FSpec retags bind tests to criteria they do not verify.

## Verdict

**FAIL**: three high findings: inflated FR-800-AC-1 coverage, an FR-1036-AC-2 binder contradicting its criterion, and an FR-1013-AC-2 clause the implementation fails behind a weak tagged test. Three FR-1012 criteria are tagged only by ignored tests. They belong to separately ordered scope (the lambda-objective PR, and timed or probabilistic planning), so this PR is not asked to implement them; they are recorded under Coverage.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FSpec FR-800-AC-1 is claimed by three tests, and none reads `Reach.steps`, a step `index`, `operation`, `arguments` or the `before`/`after` keys. The implementation cannot satisfy the criterion: `carried()` drops reference-typed parameters, so `arguments.s` refuses `missing-name`. This is coverage inflation behind a tagged criterion. | fsl-semantics/tests/it/synthesis_plan_value.rs:86, tests/it/plan_export_surface.rs:125, tests/it/synthesis_request.rs:540, fsl-semantics/src/check/plan_value.rs:142 |
| FND-002 | high | FR-1036-AC-2 says the plan candidate is typed by the `ConfigVersion` node. The new binder `a_source_plan_candidate_emits_its_record_types_and_source_regions` asserts that the candidate is typed by the generated plan record. The criterion and its tagged test now disagree, and neither the spec nor the QSL-2235-ignored binder was updated. | tests/it/synthesis_request.rs:540, tests/it/synthesis_request.rs:232, spec/functional/FR-1036-emit-a-checked-synthesis-declaration-as-a-v2-synthesis-request-node.md |
| FND-003 | high | FR-1013-AC-2 requires the population-bound snapshot to refuse "with the snapshot's own admission refusal". The implementation flattens that refusal to `invalid_runtime_input`/`invalid-value`, and the tagged test `plan_requests_refuse_at_admission` asserts only exit 20 and the pointer, so the test cannot detect the gap. | tests/it/plan_export_surface.rs:1413, fsl-command/src/command/plan_synthesis.rs:198 |
| FND-004 | medium | `a_plan_objective_is_typed_without_an_expected_domain_type` is traced to FSpec-FR-802-AC-1, the exact decimal step sum of 0.25 with a `potential` certificate. It asserts only that `minimize Reach.length` checks. | fsl-semantics/tests/it/synthesis_plan_value.rs:108 |
| FND-005 | medium | `source_policy_acceptance_requires_closed_truth_and_preserves_vacuity` is traced to FSpec-FR-803-AC-2 (the optimal plan `inc, inc, inc, double`) and FR-803-AC-5 (certified `infeasible` under `always holds(c.value != 8)`). It checks a single-state `read_clause` and empty-population vacuity, not those outcomes. | fsl-analyze/tests/it/plan_search.rs:1170 |
| FND-006 | medium | `the_check_is_not_charged_to_the_search_work_limit` was retagged from FSpec-FR-823-AC-1 to FR-823-AC-5. AC-5 is a checker rejection with exit 30. The test asserts a validated candidate and a work-limited `incomplete`, so the new binding is wrong. | fsl-command/src/command/synthesis.rs:1736 |
| FND-007 | medium | FR-1013 Behavior 1 requires S3 to refuse an unbounded operation parameter. Nothing implements it: `plan_value::declare` has no parameter-bound check, and `carried()` admits unbounded `Integer`. No test is tagged for it (FSpec FR-806-AC-1, FR-800-AC-5). This is an implied requirement missing from code and evidence. | fsl-semantics/src/check/plan_value.rs:178, spec/functional/FR-1013-refuse-plan-declarations-and-requests-by-stage.md |
| FND-008 | low | Reverse gap: the plan/value request discriminator is the presence of `initial` (`Requested::has_initial`). No requirement states it. FR-1010 and FR-823 say the selected declaration's grammar decides. | fsl-command/src/command/synthesis.rs:669 |

## Coverage

- Reconciliation: quire matrix (quire 0.52.0, engine 0.62.2). `quoin matrix --repo` (quoin 0.36.8) ran and reported no run evidence for every in-scope criterion, so the evidence store has not been fed.
- Plan completion: not assessed
- Criteria in scope (FR-1010, FR-1011, FR-1012, FR-1013, FR-1036, FR-1037, FR-710-AC-5), per the engine:
  - tagged: 17
  - untagged: 0
  - tagged-by-ignored-test: 3 (FR-1012-AC-3 lambda decimal step cost, FR-1012-AC-4 timed, FR-1012-AC-5 probabilistic; all separately ordered scope, not demanded here)
  - method-without-symbol: 0
- FSpec-prefixed tags bind FSpec criteria, which this repository's matrix does not compute. They were checked by hand against filament-specification `origin/main`; FND-001 and FND-004 to FND-006 come from that check.
- `quire coverage`: no untracked or stale tags in lines changed by this PR. Its `trace-id-outside-marker` diagnostics are prose references in doc comments, the repo's established convention, and not findings.
- The root plan tests run over the authored four-value model substituted for the spine model; see code-review FND-013 on the same PR. Evidence for "over the spine model" criteria is therefore evidence over that model.
- Untraced behaviours / stubs: 1 reverse gap (FND-008). No source stubs; `tests/support/plan_model.rs` compiles a real model unit. No test stubs; every changed test asserts behaviour, though FND-003 to FND-006 assert less than they claim.
- Semantic review: skipped (not opted in). Intent mismatches found during trace reading are reported above.
- Runtime: none run by the reviewer. Per the PR body, focused current-head runs are pending and Root17 has not been green.
