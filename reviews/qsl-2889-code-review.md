---
id: SR-3605001
title: "Code and Rust review of elapsed-quantile timed refutation (filament-spec-language PR 3605)"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language@d3a392e8b53578d65752647e20387f56bf75e901; fsl-analyze/src/analyze/exact/elapsed.rs; fsl-replay/src/probability_certificate/document/timed.rs; fsl-replay/src/probability_certificate/document/timed/elapsed.rs; fsl-analyze/tests/it/timed_product_certificate.rs"
review_set: subset
date: '2026-11-23'
---

# Code and Rust review of elapsed-quantile timed refutation (PR 3605)

## Summary

Ticket: QSL-2889. Reviewed `git diff origin/main...HEAD` after merging origin/main locally. Soundness holds: the analyzer builds a witness under a policy attaining the transform extremum, but a refutation settles only when the checker's own replay re-enumerates the elapsed product, prices the paths under the one memoryless choice they select, and confirms the threshold. A tampered witness settles inconclusive. Thresholds match FR-877: `>=` is refuted by the non-strict `counted >= q * decided mass` with counted mass > 0 (AC-4: Pr(M < 2 ms) = 9/10 = q refutes); `<=` is refuted by the strict `counted > (1 - q) * decided mass`. Ran: 9 elapsed_quantile integration tests and 2 pricing unit tests, all pass. Mutation: changing `>=` to `>` in the `>=` threshold fails both unit tests; changing `>` to `>=` in the `<=` threshold fails the threshold test. The two previously ignored tests are enabled, with no assertion weakened. `tamper_witness` is the existing test hook already threaded through `exact.rs::run`; production passes `None`. TC-2494 and FSpec-TC-1820 tags exist. No panics on user input in the added code (paths are indexed by checked `get`; the one `ids[state]` index is over a vector built from the same product).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The producer's quantile search passes `never` all false, so it prices the threshold against total mass 1, while the replay prices against decided mass. When decided mass is below 1 the producer demands more than the replay, so a true refutation can settle `not_yet` instead of `refuted`. Incompleteness only, never unsound. | fsl-analyze/src/analyze/exact/elapsed.rs:233 |
| FND-002 | low | The `>=` refutation is checked only for verdict; the replayed mass is asserted only for the `<=` claim (mass 1). A wrong counted mass for AC-4 would pass. | fsl-analyze/tests/it/timed_product_certificate.rs:1388 |
| FND-003 | low | `refute` takes 12 parameters behind `#[allow(clippy::too_many_arguments)]`, and `replay_timed_paths` repeats the pattern; rust-style prefers a parameter struct. Not a defect today. | fsl-analyze/src/analyze/exact/elapsed.rs:208 |

## Verdict

MERGE. No finding blocks; FND-001 to FND-003 are low and may be fixed forward.
