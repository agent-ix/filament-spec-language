---
id: SR-2026111612172400000025
title: "Gap analysis — filament-spec-language repository"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; spec/, source crates, tests"
review_set: subset
relationships:
  - { target: "ix://agent-ix/filament-spec-language/FR-218", type: references }
---

## Summary

Ran the repository-wide static Test Matrix and coverage inventory for filament-spec-language, plus a reverse-gap and stub inspection of the changed protocol decision checker and its tests. The repository matrix reports 1,219 untagged criteria; the focused changed behavior is specified by FR-218-AC-17 and has a tagged test. Plan completion was not assessed.

## Verdict

**FAIL** — the repository-wide matrix reports untagged acceptance criteria, which the gap-analysis gate treats as high-severity coverage gaps.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Quire's repository matrix reports 1,219 criteria as untagged; the whole-repository gap-analysis gate fails on any untagged criterion. | spec/functional/ |

## Coverage

- Reconciliation: quire matrix (quire 0.51.0) — no run evidence read. `quoin matrix --repo . --json` returned no coverage-matrix criteria because the module declares no `obligations:` source; its reported reason was recorded.
- Criteria: tagged 3,967 · untagged 1,219 · tagged-by-ignored-test 328 · method-without-symbol 103 · retired 13.
- Evidence: no run evidence read.
- Untraced behaviors / stubs: no new unowned behavior or stub found in the changed protocol checker and test; broader code-to-spec inventory was not expanded beyond the changed area.
- Semantic review: skipped (optional intent↔test↔code review was not requested).
- Plan completion: not assessed

## Review notes

The selected FR-218-AC-17 entry is tagged by the new `a_choice_can_decide_on_a_listed_field_of_its_channel_receive` test. Static coverage does not establish that the assertions cover all of AC-17: TC-1056 cases 31–32 state that operands inside a composite `visible` entry remain unlisted, and the changed implementation admits those operands (code-review SR-2026111612172400000024).

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | deferred | Pre-existing repository-wide matrix work is tracked in QSL-2089; FR-001-AC-7 is within its range, per planner. |
