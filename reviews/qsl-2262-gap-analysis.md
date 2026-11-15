---
id: SR-2026111518064800000007
title: "Gap analysis — QSL-2262 FSL repository"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; full spec, source, test and Makefile trees at QSL-2262 PR #2727 head 9462d4b41"
review_set: subset
---

## Summary

Audited the repository's computed specification-to-test matrix and the changed QSL-2262 behavior. The Makefile change and regression test have an owner in NFR-002/TC-920 and the specced QSL-2262 ticket, but the full repository matrix still has pre-existing untagged criteria and criteria backed only by ignored tests; these gaps are outside this PR.

## Verdict

**FAIL** — the full matrix reports 1,117 untagged criteria and 255 criteria tagged only by ignored tests. The changed QSL-2262 behavior does not add a spec-to-code gap or a source/test stub.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The repository matrix has 1,117 criteria with no trace-tagged test, so required behavior across the repository is not backed by runnable, criterion-bound evidence. These criteria predate PR #2727. | full repository Test Matrix at 9462d4b41 |
| FND-002 | high | The repository matrix has 255 criteria whose only tagged tests are ignored, leaving those criteria without runnable test evidence. These criteria predate PR #2727. | full repository Test Matrix at 9462d4b41 |

## Coverage

- Reconciliation: `quoin matrix --repo <repo> --json` (quoin 0.35.0; quire 0.45.0, engine 0.60.0) and `quire matrix --scope <repo> --strict`; no grep fallback. Quoin matrix exited 0. Quire strict matrix exited 1 on matrix findings.
- Matrix rollup: 5,089 criteria across 510 requirements; tagged 3,606; untagged 1,117; tagged-by-ignored-test 255; method-without-symbol 98; retired 13. Evidence status: no run evidence for all 5,089 criteria.
- NFR-002-AC-1 through AC-4 are tagged by existing tests. QSL-2262's Make target change is within the native structural-validation gate owned by NFR-002; its test has behavioral assertions, though it lacks a trace attribute (reported in the code review).
- Reverse-gap inventory for the changed surface: two changed behaviors (resolve the Node executable and pass the Quire launcher through it); both are owned by NFR-002/TC-920 and QSL-2262. No new public API, stub or placeholder was found.
- No applicable AssuranceProfile was found. `spec/spec.md` has no artifact id, so no spec-root relationship was asserted.
- Plan completion: not assessed
- Semantic review: skipped


## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | deferred | This is pre-existing repository-wide trace coverage debt outside PR #2727, which changes only launcher invocation and covers its changed behavior. Address it in separate corpus and trace-remediation work. |
| FND-002 | deferred | This is pre-existing repository-wide ignored-test coverage debt outside PR #2727, which changes only launcher invocation and covers its changed behavior. Address it in separate corpus and trace-remediation work. |

```yaml
dispositions:
  - fnd: FND-001
    outcome: deferred
    reason: "Pre-existing repository-wide trace coverage debt is outside PR #2727, which changes only launcher invocation and covers its changed behavior; address it in separate corpus and trace-remediation work."
  - fnd: FND-002
    outcome: deferred
    reason: "Pre-existing repository-wide ignored-test coverage debt is outside PR #2727, which changes only launcher invocation and covers its changed behavior; address it in separate corpus and trace-remediation work."
```
