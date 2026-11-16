---
id: SR-2026111518064800000005
title: "Gap analysis — QSL-2038 source corpus runner"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; QSL-2038; PR #2722; full spec matrix at current PR head ff5c411192ea2d6babc530e0202818450c407462; origin/main d71c0a8dfde0076cfbc4fbe13985eb2baddd7839; changed source and test files"
review_set: subset
---

## Summary

Ran the repository's evidence-backed matrix and manually mapped QSL-2038's acceptance criteria to the changed code and tests. The focused runner cases are present, but the required negative check that proves CI fails when corpus tests are deselected is missing; the full matrix also exposes preexisting repository-wide untagged and ignored-only criteria.

## Verdict

**FAIL** — the full matrix has untagged and ignored-only criteria, and QSL-2038's CI-selection acceptance check is not covered.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The full matrix reports 1,134 criteria without a trace-tagged test across the 519-requirement repository; this is a repository-wide preexisting gap, not introduced by PR #2722. | quoin matrix at current origin/main d71c0a8dfde0076cfbc4fbe13985eb2baddd7839; QSL-2038 adds no trace tags |
| FND-002 | high | FR-518-AC-11 is bound only to an ignored test, so no test runs for this criterion. | FR-518-AC-11; tests/it/checked_handoff.rs::a_differing_declared_clause_name_is_refused_on_clause_name |
| FND-003 | medium | QSL-2038's negative CI-selection criterion is uncovered: deleting the \`source_corpus\` module registration leaves the root integration target nonempty, while the xtask tests exercise only the empty-area helper. | tests/it/main.rs:35; xtask/src/test_suites.rs:1712-1742 |

## Coverage

- Reconciliation: \`quoin matrix --repo /home/peter/dev/worktrees/qsl-2038 --json\` (quoin 0.35.0; quire 0.45.0, engine 0.60.0).
- Recomputed `quoin matrix --repo /tmp/qsl2038-main-latest --json` at current origin/main d71c0a8dfde0076cfbc4fbe13985eb2baddd7839 using quoin 0.35.0 / quire 0.45.0 / engine 0.60.0: 5,213 criteria across 519 requirement documents; 3,673 tagged, 1,134 untagged, 295 tagged only by ignored tests, 98 method-without-symbol, 13 retired. Evidence status: no run evidence for the 5,213 criteria. The QSL-2038 diff adds no trace tags or spec documents, so it does not change this matrix rollup.
- Manually mapped QSL-2038's matching-expectation, exit mismatch, code/cause mismatch, both orphan cases, flip-report, public-entry-point and empty-area criteria to named tests. The negative CI-selection requirement is checked from actual root-suite output by `RunParser::close`, and its absence case is exercised by `suite_output_missing_corpus_selection_tests_is_a_finding`.
- Reverse-gap inventory: the changed runner and xtask guard are internal test infrastructure owned by the specced QSL-2038 ticket; no new user-facing API, stub or TODO was found.
- \`spec/spec.md\` has no artifact id, so no spec-root relationship was asserted. No applicable AssuranceProfile was found.
- Plan completion: not assessed
- Semantic review: skipped


## Rebase refresh

Rechecked the exact PR diff at head `ff5c411192ea2d6babc530e0202818450c407462` against current `origin/main` `d71c0a8dfde0076cfbc4fbe13985eb2baddd7839`. The CI-selection gap remains fixed. Targeted AC-8 review confirms `suite_output_missing_corpus_selection_tests_is_a_finding` feeds a simulated root-suite output containing only `unrelated_test` through `RunParser::finish()` and asserts a `RequiredTestNotSelected` finding for `source_corpus::corpus_values`. In the production path, `run_lane()` passes those findings to `evaluate_declared()` and returns `Error::TestSuitesFound` whenever findings remain, so omitting corpus tests from the actual selected suite makes the xtask lane fail. The earlier FND-003 statement that xtask tests exercised only the empty-area helper was stale; the simulated-output regression test does exercise the negative selection guard. The matrix remains 1,134 untagged and 295 ignored-only criteria across the repository; these are the preexisting, separately tracked gaps listed above.

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | deferred | Repository-wide untagged criteria remain outside QSL-2038 scope and are tracked by QSL-2083 through QSL-2098. |
| FND-002 | deferred | FR-518-AC-11 remains bound only to the ignored test `tests/it/checked_handoff.rs::a_differing_declared_clause_name_is_refused_on_clause_name`; it is outside QSL-2038 scope and tracked by QSL-2109. |
| FND-003 | fixed | The actual root integration-suite output is checked for every required corpus area test; missing selections produce `RequiredTestNotSelected`. |
| FND-003 | fixed | Targeted recheck: the simulated root-suite output test asserts `RequiredTestNotSelected`, and `run_lane()` converts remaining findings to `Error::TestSuitesFound`. |

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
  - fnd: FND-003
    outcome: fixed
    after_excerpt: |
      if !findings.is_empty() {
          let mut summary = format!(
              "test-suites: lane {}: {} finding(s)\n",
              lane.name,
              findings.len()
          );
          for finding in &findings {
              summary.push_str(&format!("  {finding}\n"));
          }
          return Err(Error::TestSuitesFound { summary });
      }
```
