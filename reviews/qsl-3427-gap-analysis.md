---
id: SR-967175026302197768900474181941935084870
title: "Gap analysis — filament-spec-language repository assurance"
type: SpecReview
analysis: gap-analysis
scope: "spec/, source tree, test tree"
review_set: subset
relationships:
  - { target: "ix://agent-ix/filament-spec-language", type: references }
---

## Summary

Ran a planless, repository-wide static matrix review for filament-spec-language. The static matrix reports broad trace gaps; the reverse-gap and stub inventory was not completed, so this artifact does not claim a complete repository audit or delivery against a plan.

## Verdict

**FAIL** — the static matrix reports 989 untagged criteria and 24 criteria backed only by ignored tests; the reverse-gap and stub steps remain incomplete.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-002-AC-9 has no test bound to its criterion id | FR-002-AC-9 |
| FND-002 | high | FR-1012-AC-4 is backed only by an ignored test | FR-1012-AC-4 |

## Coverage

- Reconciliation: `quire matrix` 0.52.1 — no run evidence read. `quoin matrix --repo . --json` 0.31.1 refused with `CORE_REFUSED: request exceeds the accepted size`; the documented static fallback completed.
- Plan completion: not assessed
- Criteria: tagged 5,657 · untagged 989 · tagged-by-ignored-test 24 · method-without-symbol 110 · retired 15 (6,795 total).
- Evidence: no run evidence read.
- `quire coverage` reported 6,613 untracked symbols and 24,977 diagnostics. The inspected untracked sample uses the `FSpec-` prefix that repository AGENTS.md reserves for the other specification repository, so the count is recorded but not classified wholesale as stale tags.
- Untraced behaviors / stubs: not assessed; reverse-gap and stub inventory was not completed.
- Semantic review: skipped (not requested).

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | deferred | QSL-3110 owns the repository-wide untagged criteria; its comment explicitly adds FR-002-AC-9 to that triage scope. |
| FND-002 | deferred | QSL-3110 owns the 24 ignored-only criteria; its comment lists FR-1012-AC-4 among them. |
