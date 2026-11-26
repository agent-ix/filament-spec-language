---
id: SR-967183
title: "gap-analysis of PR 4013 measure value-bound claim rules"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; FR-1042-AC-28, FR-1043-AC-10, TC-3151 steps 16-17; PR 4013 head 3bd1330ab159a085d94863acc33b32ad240485e0"
review_set: subset
---

## Summary

Ticket: QSL-3414. PR-scoped gap analysis, planless. `quire matrix --scope .`
on the reviewed head lists FR-1042-AC-28 and FR-1043-AC-10 as `tagged`.
`quire trace` resolves the two new `#[implements]` markers and the three new
criterion tags.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | `a_numeric_quantile_bound_without_activation_is_rejected` is tagged FR-1043-AC-10, whose text names only an all-zero `exact` document and a workload `ratio`, not bound documents; TC-3151's Scope line does not list FR-1043-AC-10 although step 17 and the tag use it | tools/fsl-probability-check/tests/it/value_bounds.rs:1566 |
| FND-002 | low | The refusal of measure bounds over several initial states has no criterion; its test carries only `TC-3151`, so no criterion guards this soundness rule | tools/fsl-probability-check/tests/it/value_bounds.rs:1698 |
| FND-003 | low | TC-3151 step 16 expects `InequalityFails` "at the activated state"; the rejection test asserts the rule only, not the locus | tools/fsl-probability-check/tests/it/value_bounds.rs:1539-1557 |

## Verdict

CONDITIONAL: low findings only. The new criterion FR-1042-AC-28 has a
rejecting test and an accepting test, and both bind to it correctly. The
unreachable-activation test covers both roles, both comparisons, and both
the every-scheduler and workload bases. The two-initial-state test asserts
`unsupported` on both routes for `<=`, and "nothing proved" for every case.
No stub or inflated evidence was found.

## Coverage

- Plan completion: not assessed
- Semantic review: done inline for the PR's criteria only.
- Context: on this head the matrix lists FR-1042-AC-7, AC-9, AC-11, AC-12 and
  AC-15 as untagged. They predate this PR and are not changed here.
- Bindings checked: a_strict_quantile_upper_of_zero_with_tight_activation_is_rejected
  to FR-1042-AC-28 (correct); a_strict_quantile_upper_of_zero_without_tight_activation_is_accepted
  to FR-1042-AC-28 (correct); a_numeric_quantile_bound_without_activation_is_rejected
  to FR-1043-AC-10 (FND-001: it asserts the FR-1043 "Forms and documents" body rule, which
  the AC text does not state).

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | low | `a_strict_quantile_exact_zero_with_tight_activation_is_rejected_at_the_initial_key` is tagged FR-1043-AC-3, a settlement criterion over AC-2's model. The rule it asserts, checker `InequalityFails` at the breadth-first first activated state, is FR-201-AC-5/AC-6's | tools/fsl-probability-check/tests/it/value_bounds.rs:1579 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Test retagged FR-1042-AC-29, a new criterion that names bound documents in both roles; TC-3151 Scope now runs to FR-1042-AC-30 |
| FND-002 | fixed | New FR-1042-AC-30 states the several-initial-states refusal; test tagged `("TC-3151", "FR-1042-AC-30")` |
| FND-003 | fixed | Rejection test now asserts `output["at"]` equals the initial key |
