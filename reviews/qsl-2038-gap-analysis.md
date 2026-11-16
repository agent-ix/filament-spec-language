---
id: SR-2026111518064800000005
title: "Gap analysis — QSL-2038 source corpus runner"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; QSL-2038; PR #2722; full spec matrix at rebased candidate 1bfdb3efdeab7f9103cd52dd42e2e4c8f1d6eb6c (origin/main 9ed0f3c4dd0fc35e97249a6726fb96bbdf37c696); changed source and test files"
review_set: subset
---

## Summary

Ran the repository's evidence-backed matrix and manually mapped QSL-2038's acceptance criteria to the changed code and tests. The focused runner cases are present, but the required negative check that proves CI fails when corpus tests are deselected is missing; the full matrix also exposes preexisting repository-wide untagged and ignored-only criteria.

## Verdict

**FAIL** — the full matrix has untagged and ignored-only criteria, and QSL-2038's CI-selection acceptance check is not covered.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The full matrix reports 1,120 criteria without a trace-tagged test across the 517-requirement repository; this is a repository-wide preexisting gap, not introduced by PR #2722. | quoin matrix at rebased candidate 1bfdb3efdeab7f9103cd52dd42e2e4c8f1d6eb6c (origin/main 9ed0f3c4dd0fc35e97249a6726fb96bbdf37c696) |
| FND-002 | high | FR-518-AC-11 is bound only to an ignored test, so no test runs for this criterion. | FR-518-AC-11; tests/it/checked_handoff.rs::a_differing_declared_clause_name_is_refused_on_clause_name |
| FND-003 | medium | QSL-2038's negative CI-selection criterion is uncovered: deleting the \`source_corpus\` module registration leaves the root integration target nonempty, while the xtask tests exercise only the empty-area helper. | tests/it/main.rs:35; xtask/src/test_suites.rs:1712-1742 |

## Coverage

- Reconciliation: \`quoin matrix --repo /home/peter/dev/worktrees/qsl-2038 --json\` (quoin 0.35.0; quire 0.45.0, engine 0.60.0).
- Recomputed `quoin matrix --repo /home/peter/dev/worktrees/qsl-2038 --json` at candidate 1bfdb3efdeab7f9103cd52dd42e2e4c8f1d6eb6c using quoin 0.35.0 / quire 0.45.0 / engine 0.60.0: 5,173 criteria across 517 requirement documents; 3,643 tagged, 1,120 untagged, 299 tagged only by ignored tests, 98 method-without-symbol, 13 retired. Evidence status: no run evidence for the 5,173 criteria.
- Manually mapped QSL-2038's matching-expectation, exit mismatch, code/cause mismatch, both orphan cases, flip-report, public-entry-point and empty-area criteria to named tests. The negative CI-selection requirement is checked from actual root-suite output by `RunParser::close`, and its absence case is exercised by `suite_output_missing_corpus_selection_tests_is_a_finding`.
- Reverse-gap inventory: the changed runner and xtask guard are internal test infrastructure owned by the specced QSL-2038 ticket; no new user-facing API, stub or TODO was found.
- \`spec/spec.md\` has no artifact id, so no spec-root relationship was asserted. No applicable AssuranceProfile was found.
- Plan completion: not assessed
- Semantic review: skipped


## Rebase refresh

Recomputed the matrix at rebased head `1bfdb3efdeab7f9103cd52dd42e2e4c8f1d6eb6c`. The prior CI-selection gap is now fixed: `RunParser::close` checks the actual root suite output against the required corpus-area test names, with `suite_output_missing_corpus_selection_tests_is_a_finding` exercising the missing-selection case. Reverse-gap review found no new user-facing API, stub or TODO in the changed runner/xtask infrastructure.

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | deferred | Repository-wide untagged criteria remain outside QSL-2038 scope and are tracked by QSL-2083 through QSL-2098. |
| FND-002 | deferred | FR-518-AC-11 remains bound only to the ignored test `tests/it/checked_handoff.rs::a_differing_declared_clause_name_is_refused_on_clause_name`; it is outside QSL-2038 scope and tracked by QSL-2109. |
| FND-003 | fixed | The actual root integration-suite output is checked for every required corpus area test; missing selections produce `RequiredTestNotSelected`. |

```yaml
dispositions:
  - fnd: FND-001
    outcome: deferred
    reason: "The repository-wide untagged-criteria backlog is outside QSL-2038 scope and tracked by QSL-2083 through QSL-2098."
  - fnd: FND-002
    outcome: deferred
    reason: "FR-518-AC-11's ignored-only test limitation is outside QSL-2038 scope and tracked by QSL-2109."
  - fnd: FND-003
    outcome: fixed
    after_excerpt: |
      self.findings.extend(required_source_corpus_test_findings(&id, &tests));
```
