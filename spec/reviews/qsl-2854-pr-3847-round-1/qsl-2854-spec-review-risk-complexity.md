---
id: SR-967175026302197768900474181941935084850
title: "QSL-2854 NFR-141 risk and complexity review"
type: SpecReview
analysis: risk-complexity
scope: "agent-ix/filament-spec-language; spec/non-functional/NFR-141-measure-performance-program-paths.md; NFR-141-AC-25, NFR-141-AC-26, NFR-141-AC-27"
review_set: subset
relationships:
  - {target: "ix://agent-ix/filament-spec-language/NFR-141", type: reviews}
---

## Summary

Risk analysis scored NFR-141's three new QSL-2854 criteria for technical risk and volatility. The criteria contain hard performance/complexity bounds and therefore have high technical risk, low volatility, and named deterministic counter/oracle tests that mitigate the risk.

## Verdict

**PASS** — the high-risk cost and depth guarantees have concrete, deterministic verification methods and depend on no volatile external contract.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No issues found (placeholder) | - |

## Risk register

| Req | Tech Risk | Volatility | Drivers | Mitigation |
| --- | --- | --- | --- | --- |
| NFR-141-AC-25 | High | Low | Graph-build count bound over increasing declaration populations | Deterministic build counter and exact output comparison at three sizes |
| NFR-141-AC-26 | High | Low | Ordered-key comparison upper bound and exact hidden-key order | Deterministic comparison counter and prior-output comparison at three sizes |
| NFR-141-AC-27 | High | Low | Traversal complexity and stack behavior at depth 100,000 | Deterministic visit/child/path counters, bounded behavior oracle and isolated small-stack test |

## Top hazards

1. NFR-141-AC-27 — implementation and test must avoid recursive traversal and recursive path teardown at the specified depth.
2. NFR-141-AC-26 — comparator-count bounds must count actual ordered-key comparisons, not an approximation.
3. NFR-141-AC-25 — the build counter must observe the production `declared_composites()` call.

## Failure-domain gaps

None found for the edited requirements.
