---
id: SR-967175026302197768900474181941935084849
title: "Gap analysis — QSL-3222 PR #3890"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; repository-driven audit; fsl-analyze/src/model_check/liveness.rs; tests/it/temporal_package_route/remaining_routes.rs; FR-126-AC-26; FR-621-AC-19 through FR-621-AC-22"
review_set: subset
relationships:
  - { target: "ix://agent-ix/filament-spec-language", type: references }
  - { target: "ix://agent-ix/filament-spec-language/FR-126", type: reviews }
  - { target: "ix://agent-ix/filament-spec-language/FR-621", type: reviews }
---

## Summary

Planless, repository-driven gap analysis for QSL-3222 PR #3890 at head `17865da498b637679928ee0a9ddd4fe434651302`. The touched liveness behavior has an owning FR-126 criterion and the matrix binds the new test to it, but the same touched fairness test leaves four FR-621 criteria untagged and does not assert the order fields their test case requires. Plan completion was not assessed.

## Verdict

**FAIL** — the computed matrix reports FR-621-AC-19 through AC-22 untagged; the tagged FR-126 regression also omits the exact limit cause/value assertion.

## Coverage

- Reconciliation: `quire matrix --scope . --format json` (quire 0.52.1; engine 0.62.2), no run evidence read.
- Plan completion: not assessed
- Criteria: tagged 5,513; untagged 1,065; tagged-by-ignored-test 24; method-without-symbol 109; retired 15 (repository-wide matrix; existing diagnostics emitted).
- Evidence: no run evidence
- Untraced behaviors / stubs: the changed initial-root admission path is owned by FR-126-AC-26; no new ownerless behavior or stub found in the changed production path.
- Semantic review: skipped (not requested; this pass checked matrix bindings and source/test structure).

## Scope

- FR-126-AC-26 — examined; matrix status tagged through the changed fairness test and existing initial-state tests; the new assertion omits exact cause/value.
- FR-621-AC-19 — examined; matrix status untagged.
- FR-621-AC-20 — examined; matrix status untagged.
- FR-621-AC-21 — examined; matrix status untagged.
- FR-621-AC-22 — examined; matrix status untagged.
- Changed liveness source — examined; behavior is owned by FR-126-AC-26.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The touched public analyze test has no FR-621-AC-19..22 trace tags and does not assert each `request_index` and `claim_name`; the matrix reports all four criteria untagged, and a swap of the two refuted items passes. | tests/it/temporal_package_route/remaining_routes.rs:368-406; FR-621-AC-19..22 |
| FND-002 | medium | The new test tagged FR-126-AC-26 checks incomplete status and state count but not `record.cause.limit == max_states` or its value, so the exact state-limit behavior can regress without this test failing. | tests/it/temporal_package_route/remaining_routes.rs:439-443; FR-126-AC-26 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | The test now asserts each request index and claim name, and traces FR-621-AC-19 through AC-22. |
| FND-002 | fixed | The one-state case now asserts the exact cause kind, limit, value and setting. |

After excerpt (FND-001):

```rust
#[trace(
    "TC-1581",
    "FR-621-AC-19",
    "FR-621-AC-20",
    "FR-621-AC-21",
    "FR-621-AC-22",
    "FR-126-AC-26",
    "FR-330"
)]
```

After excerpt (FND-002):

```rust
assert_eq!(item["record"]["cause"]["kind"], "limit-reached", "{item}");
assert_eq!(item["record"]["cause"]["limit"], "max_states", "{item}");
assert_eq!(item["record"]["cause"]["value"], 1, "{item}");
assert_eq!(item["record"]["cause"]["setting"], "model_check.max_states", "{item}");
```
