---
id: SR-967175026302197768900474181941935084827
title: "Gap analysis of QSL-3178 FR-281 acceptance coverage"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language@f7ac7a4b6af48081a9f3330183e0102d33cb9ac1; git diff origin/main...HEAD; FR-281-AC-9, FR-281-AC-13, FR-281-AC-22; QSL-3178"
review_set: subset
date: 2026-11-25
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-281
    type: reviews
---

## Summary

Ticket: QSL-3178. This targeted acceptance audit checked FR-281-AC-9 and FR-281-AC-13 against the changed tests and computed matrix, and verified the disclosed FR-281-AC-22 gap at final head `f7ac7a4b6af48081a9f3330183e0102d33cb9ac1`. It is a PR-scoped audit, not a full repository reverse-gap inventory. Plan completion: not assessed.

## Verdict

**FAIL** — FR-281-AC-22 remains untagged and has no desired-only test for its proof-bound behavior. This is a disclosed production gap, not introduced by this branch. FR-281-AC-9 and FR-281-AC-13 are tagged and their scoped clauses are behaviorally asserted.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | At the original review head, FR-281-AC-13 had no tagged test. Its bounded-only value-validity descriptor could not be constructed through public `Engine::external`, which rejected value-validity descriptors lacking `unbounded`. This is fixed by the corrected AC and request-level test on the final head. | spec/functional/FR-281-analyze-claims-with-in-process-engines.md:606; fsl-analyze/src/analyze.rs:368-390 |
| FND-002 | high | FR-281-AC-22 has no tagged test and no desired-only test for the bounded proof-bound exploration or empty-proof-bounds unsupported result. | spec/functional/FR-281-analyze-claims-with-in-process-engines.md:615 |

## Coverage

- Reconciliation: `quire matrix --scope . --format json` (quire 0.52.1) — no run evidence read. `quoin matrix --repo . --json` was attempted but failed with `CORE_REFUSED: request exceeds the accepted size`.
- Static matrix totals for the repository: tagged 5482; untagged 1021; tagged-by-ignored-test 24; method-without-symbol 109; retired 15 (6651 criteria total). These repository-wide counts are recorded for context; this artifact's criterion-level findings are restricted to the requested FR-281 review scope.
- FR-281-AC-9: `tagged`; binders are `three_items_give_three_records_the_first_unsupported_naming_its_kind` and `analyze_refuses_an_undeclared_population_before_running_items`. The first asserts order `[0,1,2]`; the second checks `invalid_runtime_input`/`wrong-role-mapping`, returns no outcome records, and observes zero provider calls.
- FR-281-AC-13: `tagged`; binder `handoff_refusals_are_settled_after_claim_family_is_checked`. It asserts the three state-clause request records' result and diagnostic, and for each record asserts `disposition_detail()["family"] == "state-model"` at `fsl-analyze/tests/it/claim_family.rs:797-815`. FR-281-AC-22: `untagged`, no binders.
- Reverse-gap scope: no production source changed; the added implementation is a test-only counting provider. A full source-tree behavior inventory was not part of this PR-scoped audit.
- Plan completion: not assessed
- Semantic review: manual acceptance audit ran for FR-281-AC-9; broader repository semantic review skipped.


## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | The corrected descriptor and complete request-level output assertions close the public-API contradiction; the current matrix tags AC-13. |
| FND-002 | still-open | AC-22 remains a production gap and has no desired-only test or tagged binder on this head. No tracked resolution id was supplied or found in the branch history, so this remains open pending an architecture-owned resolution. |

### Disposition details

```yaml
dispositions:
  - fnd: FND-001
    outcome: fixed
    after_excerpt: |-
      With `engines` holding `fsl.test-a` advertising only (`value-validity`, `unbounded`), a `finite-replay` item on a state clause's claim settles `invalid-request` with `invalid_capability`/`inapplicable-kind`, payload `family` `state-model`, and no engine runs
  - fnd: FND-002
    outcome: still-open
    reason: "AC-22 remains untagged and no tracked resolution id was supplied or found."
```
| FND-002 | still-open | AC-22 remains a production gap and has no desired-only test or tagged binder on this head. No tracked resolution id was supplied or found in the branch history, so this remains open pending an architecture-owned resolution. |

### Round 1 evidence

- Current matrix: FR-281-AC-9 and FR-281-AC-13 are `tagged`; FR-281-AC-22 is `untagged` with no binders.
- AC-13's current criterion says the descriptor advertises only (`value-validity`, `unbounded`); the request-level test constructs that descriptor, asserts the three expected routing outcomes and zero provider calls, and checks `family: state-model` on each state-clause record at `claim_family.rs:797-815`.
- AC-22 still lacks the integer proof-bound exploration and no-bounds unsupported behavior. No change in this branch resolves that production behavior.
