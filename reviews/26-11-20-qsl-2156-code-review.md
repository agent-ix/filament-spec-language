---
id: SR-2026112013172122021003265
title: "Code review of QSL-2156 PR #3204 timed digital certificates"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; QSL-2156; PR #3204 head 85ff4ce2e72ebebcf220671e0ab96f43fc48d49a; 25 changed files across fsl-analyze and fsl-replay"
review_set: subset
---
# Code review: QSL-2156 PR #3204

## Summary

Ticket: QSL-2156. Read-only static review of the frozen PR diff against `origin/main` (25 files, 6,034 additions and 463 deletions). No Rust build or tests were run, per the lead's build-lane hold. `git diff --check` passed. No CI workflow or dependency pin changes are present. The reviewed Rust paths have no unsafe code in the relevant changes, and the checker reconstructs timed products independently as FR-921 requires.

FR-924's zero-time component handling now ignores zero-mass edges when selecting an exit route. The region checker still returns `Unsupported` for a pending quiescent state instead of admitting the specified time-lock terminal. The focused test/build gates remain outstanding.

## Verdict

CONDITIONAL. One medium implementation finding remains; focused checks are pending the explicit build release.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The region-route checker refuses a pending quiescent state, although FR-924 requires the checker to classify such states as time-lock terminals. | fsl-replay/src/probability_certificate/document/timed.rs:1683-1689 |

## Scope examined

- FR-924-AC-1 through FR-924-AC-6: timed-product producer/checker tests and component value/rank handling.
- FR-921-AC-1 through FR-921-AC-4: identity, route/scale, row coverage, and independent checker behavior.
- FR-922-AC-1 through FR-922-AC-3: strict region and boundary-region behavior.
- FR-923-AC-1 through FR-923-AC-3: discrete delays and scheduler ties.
- FR-877-AC-1 through FR-877-AC-4: timed quantile transforms.
- FR-198-AC-5: selected MEC exit policy and replay.

## Finding detail

**FND-001 — medium — quiescent states on the region route remain unsupported.**

`build_region_graph` returns `ProbabilityDocumentError::Unsupported` when `timed.is_quiescent(...)` is true at a pending state (`fsl-replay/src/probability_certificate/document/timed.rs:1683-1689`). FR-924 requires the checker to identify time-locks and treat a state with no admissible delay and no enabled discrete move as terminal. The current branch can therefore refuse a strict-clock/region certificate whose pending state is a time-lock instead of checking the terminal value/rank. Extend the checker-owned region product with the required time-lock transition/terminal treatment and a focused case when the build lane is released.

## Review limits

`cargo check` and targeted tests were not run because the lead has withheld the wave-1-steady build release. The matrix and traceability review is recorded separately in the gap-analysis artifact.

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | At 2b16a60f, the pending region state is inserted with `terminal: choices.is_empty().then_some(false)`, so a quiescent state with no enabled delay or action becomes a nonaccepting terminal instead of returning Unsupported. |

## Disposition pass summary

At frozen head `2b16a60fef4b7382fecdf1f9148961c0baeaecfa`, the early quiescent-state refusal is removed. A pending state with no delay or enabled action is inserted as a terminal rejection (`terminal: choices.is_empty().then_some(false)`), matching FR-924's event time-lock value. No build or test was run, so test verification remains pending.
