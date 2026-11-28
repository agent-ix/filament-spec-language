---
id: SR-5376645
title: "Gap analysis of filament-spec-language PR 3898 (QSL-3296): three red-main fixes"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language@020c4bdf1c8fcfde796e7008a6d50cca2c636f43; PR #3898 diff against merge base addba69c91; fsl-eval/src/simulation/model.rs, fsl-semantics/src/check/check.rs, fsl-semantics/tests/it/refinement_class_catalog.rs, fsl-analyze/tests/it/model_instances.rs"
review_set: subset
relationships:
  - type: references
    target: ix://agent-ix/filament-spec-language/FR-1040
  - type: references
    target: ix://agent-ix/filament-spec-language/FR-125
  - type: references
    target: ix://agent-ix/filament-spec-language/FR-084
---
# Gap analysis of filament-spec-language PR #3898

## Summary

Ticket: QSL-3296. Planless gap analysis scoped to the PR's diff. The computed
matrix was read with `quire matrix --scope . --format json` on the ticket
worktree at 020c4bdf1c (exit 0); no run evidence was read and no tests were
run by the reviewer.

- The production change in `fsl-semantics/src/check/check.rs` (ordering arm for
  two clock references) is owned by FR-1040 ("`<` and the other orderings over
  two place ids refuse `ill_typed`/`operator-ineligible`") and FR-1040-AC-6.
  FR-1040-AC-6 is `tagged` in the matrix, bound among others to
  `clock_places::population_clock_reads_and_ill_typed_refs_are_refused`, which
  asserts `ref(c.x) < ref(c.t.x)` refuses `OperatorIneligible` and passed on
  the change.
- The production change in `fsl-eval/src/simulation/model.rs` (admission keyed on
  universe and key) is owned by FR-125 ("one universe and key") and FR-084
  "Conflicting identity" / FR-084-AC-4. FR-125-AC-13 is `tagged`, bound to
  `model_instances::model_instancing_checks_bounds_initials_and_cross_snapshot_identity_first`,
  which keeps its refusal cases and gains an assertion that an equal key in an
  unrelated hierarchy is admitted. FR-084-AC-4 is `tagged` in fsl-semantics.
  FR-640-AC-12's two-model hyper analysis is exercised by
  `analyze_family_cli::a_hyper_clause_over_two_models_analyzes_and_replays_per_trace`,
  now passing.
- No code in the diff lacks an owning requirement. No stub, placeholder or
  tautological assertion was added: the rewritten catalog test asserts
  hand-written literals and node keys, the new model_instances assertion
  compares against a literal instance list.
- The rewritten fsl-semantics test carries a bare FSpec id that the matrix
  does not bind; that is recorded once, as SR-5590831 FND-001, and not
  repeated here.

## Verdict

PASS (repository claim for the PR scope only): every criterion the changed
code serves is tagged by a test, no reverse gap, no stub.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

- Criteria examined: FR-1040-AC-6 (tagged), FR-125-AC-13 (tagged), FR-084-AC-4 (tagged); FR-640-AC-12 and FR-383 read as context.
- Reverse gap: none in the four changed files.
- Stubs / coverage inflation: none found.
- Semantic review: skipped as a separate pass (reviewer-only dispatch); intent-to-code agreement for the three fixes is recorded in SR-5590831.
- Plan completion: not assessed
