---
id: SR-21282002
title: "QSL-2128 bounded reader gap analysis"
type: SpecReview
analysis: gap-analysis
review_set: subset
scope: "agent-ix/filament-spec-language; PR3396 eight-file universal hyper reader/constructor slice; repository matrix attempted"
relationships:
  - {target: "ix://agent-ix/filament-spec-language/FR-940", type: references}
---

## Summary

Ticket: QSL-2128. Gap-analysis procedures ran against frozen c08a0e682f1980f6ec17003422dd5dceba122107. Reverse-gap, stub and test-oracle inspection completed for the eight changed files. Repository-wide engine-backed matrix extraction did not complete, so this artifact cannot certify a full repository audit.

## Verdict

FAIL for audit completeness only: the bounded static matrix attempt timed out. No new implementation defect is asserted by this method beyond code-review SR-21282001. No criterion coverage percentage or repository PASS is claimed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Repository-wide criterion reconciliation is incomplete because the single bounded quire matrix extraction timed out after 25 seconds with no JSON output. | review/matrix.json; review/matrix.stderr |

## Coverage

Plan completion: not assessed

Reconciliation attempted: `timeout 25s quire matrix --scope /home/peter/dev/worktrees/QSL-2128-hp1-certificate --format json`; exit 124. matrix.json is empty; matrix.stderr contains module warnings only. The inherited quoin evidence-store refusal/hang is not treated as coverage evidence. No retries, long loops, grep-derived coverage claims or fabricated PASS were used. Static-status and evidence-status totals are unavailable. The reviewer's dispatch explicitly accepts a bounded attempt and honest limitation.

Reverse-gap and stub inspection: every changed behavior belongs to FR-940's package hyper form or the existing temporal package arena contract. New tests bind FR-940-AC-16 and FR-940-AC-19 with real trace attributes. No source/test stub, ignored new test, mock-only oracle, warning suppression or lowered threshold was found in the changed slice. Semantic intent-to-test-to-code inspection was explicitly requested by the dispatch and performed within code-review, not expanded into a second repository-wide semantic campaign.

Examined: FR-940's hyper inputs/normal form/malformed-node boundary and the Equal subset of AC-16, unknown-trace/strict-duplicate-level subset of AC-19; all eight files listed in SR-21282001. Context only: full AC-16 and AC-19 remaining cases, AC-18 source-free certificate journey and FSpec FR-402's seven-binding facts. None of those remaining acceptance journeys is claimed complete. PR3395's stale refusal prose is under separate spec review; implementation must preserve the pinned strict duplicate-level refusal.

The matrix limitation is an assurance observation, not a request to change production code, broaden this PR, invent a baseline, or implement the named later AC-18 slice. The lead owns disposition of that limitation and custody of this artifact.
