---
id: SR-96719
title: "QSL-2055 gap analysis of the value-layer property suite (filament-spec-language PR 3578)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language@c8b490ec61671f57a4f5fcbece68df8f4d297235; fsl-eval/tests/it/value_properties.rs; ticket QSL-2055 acceptance list"
review_set: subset
date: 2026-11-23
---

## Summary

Ticket: QSL-2055 (PR filament-spec-language#3578). The PR adds only tests and
changes no spec, so this audit is scoped to the diff. It checks each ticket
acceptance criterion against the tests, looks for hollow evidence, and checks
the trace tags. It does not recompute the repo-wide matrix, because the PR
adds no criterion and no trace tag.

Ticket acceptance list against the tests:

- P1 to P5 exist by exact name at 1,000 cases.
- `reference_mutation_is_caught` exists and asserts that the failure shrinks
  to a one-operator `+` source.
- `near_miss_is_never_evaluated_to_a_value` exists. It also covers the
  zero-spanning `div`, `rem` and `mod` near miss from FR-495-AC-3.
- The runtime is under 10 s on terra (see SR-96718 FND-002).

The P3 family list matches the AC. Int[..] appears as parameters and as the
`reduce` result, and the Decimal mode is selected among all six. Every
omitted family is named in the file header with its ticket or reason: size<T>
(QSL-1966), count<N> over its bound (QSL-2003), and power, floats,
quantities, unions, maps and references (outside the AC's list; FR-093
defines `^` for quantities only). min and max over a Set or Bag cannot be
written: FR-701 admits only catalog steps, and FSL has no min/max aggregate.
P5 covers them over Sequence only, and the header says so. Stubs and coverage
inflation: none found.

Plan completion: not assessed

## Verdict

CONDITIONAL: no criterion is untested and there is no high finding. One low
finding.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | None of the seven new tests carries an `ix_trace_rs::trace` tag, so the computed matrix does not count them. Main's sibling property suites tag theirs (for example equality_matrix.rs `#[trace("FSpec-TC-194", ...)]`). Candidates: FR-495-AC-3 for the zero-spanning near miss, FR-701-AC-1 for P5. | fsl-eval/tests/it/value_properties.rs:1801-2047 |

## Coverage

- Criteria examined: the 8 acceptance items of QSL-2055. All have a test.
- Repo-wide matrix: not recomputed. The PR adds no criterion and no tag.
- Semantic review: not run (opt-in; not requested).
- Plan completion: not assessed

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-002 | low | Three of the new trace tags name criteria their tests do not exercise, which inflates the matrix. P2 and P3 cite FR-091-AC-13 (calling `two` returns 2). P4 cites FR-495-AC-17 (Linked `mod` refusals and types, but P4 runs under Kernel). P5 cites FR-701-AC-1, a `fold` catalog step that P5 never writes. FR-003-AC-1 on P1 and FR-495-AC-3 on P4 and the near-miss test are correct. | fsl-eval/tests/it/value_properties.rs:1868; fsl-eval/tests/it/value_properties.rs:1898; fsl-eval/tests/it/value_properties.rs:1921; fsl-eval/tests/it/value_properties.rs:1960; fsl-eval/tests/it/value_properties.rs:2007 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | - |
