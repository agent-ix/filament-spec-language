---
id: SR-967175026302197768900474181941935084875
title: "code review of PR 4061: dense timed falsification oracle and fsl-eval timed measure readers (QSL-3491)"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; fsl-analyze/tests/it/timed_oracle.rs, fsl-analyze/tests/it/timed_oracle/, fsl-analyze/tests/it/timed_world, fsl-analyze/tests/it/main.rs, fsl-eval/src/temporal/timed_measure.rs, fsl-eval/src/temporal/timed.rs, fsl-eval/src/temporal/mod.rs, fsl-eval/tests/it/timed_measures.rs"
review_set: subset
---

## Summary

Ticket: QSL-3491. Head a6fa85472f527b9c3b0e991178c4ac8c5eaeb061, base 499e6c5d1d8142d7724938224479fa6d90095347. Rust review is folded into this file.
Every row's expected value was re-derived by hand against FSpec FR-407, FR-416, FR-420 and FR-421 and found correct; no value was taken from tool output.
The falsifier uses dense enumeration over TimedSystem and the fsl-eval evaluator and no zone, region or digital helper.
The reviewer ran the oracle with ignored tests included: 41 passed, 28 failed. The 28 failures are exactly the 28 ignored tests, each with the symptom in its ignore reason apart from the two ownership findings below.
The new readers match FR-407-AC-21, AC-23 (steps half) and AC-26. The timed.rs change is visibility only.

## Verdict

Not mergeable as it stands. The expected values and the readers are sound. The comparison helpers accept a decline or a harness failure as agreement, so several rows cannot fail on the wrong answer they exist to catch.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The Burst routes row is tagged FR-254-AC-8, which requires Unsupported(NotStochasticIdleTail), and does not assert it. An EN-4 regression to Rejected with any estimate passes, because the claim is false and min differs from max. | fsl-analyze/tests/it/timed_oracle/probability.rs:1145-1149 |
| FND-002 | medium | Unsettled agrees with every hand value. Rows whose criterion requires a definite answer stay green if the route stops answering, ignored rows turn green on a fix that only declines, and the retx-offsets, later-window and on-each routes rows pass today only by decline without asserting it. | fsl-analyze/tests/it/timed_oracle/probability.rs:228 |
| FND-003 | medium | Harness failures read as an honest decline: any sampler Err or stop, an InvalidRequest disposition and a missing command item. EN-4 stops with established-invariant-broken on drawn nested rows LS and NS and is counted as agreement, so that ignored test goes green once EN-5 alone is fixed. | fsl-analyze/tests/it/timed_oracle/routes.rs:134 |
| FND-004 | medium | The unbounded two-clock maximum is ignored under QSL-3444, but its failure (CertificateRejected Malformed at Values) is the symptom QSL-3444 itself assigns to QSL-3443. Fixing QSL-3444 leaves it failing under a closed ticket and nothing tracks it under QSL-3443. | fsl-analyze/tests/it/timed_oracle/probability.rs:574-579 |
| FND-005 | low | The Split routes row is ignored under QSL-3494, an admission-table step whose text and comments never mention Split or a minimum over a reachable time-lock, so the step can close without this symptom being checked. | fsl-analyze/tests/it/timed_oracle/probability.rs:1105-1110 |
| FND-006 | low | A frozen time-lock is closed with a 1000 ms idle tail that adds new positions and stamps. FSpec FR-416 says a time-locked run adds no new state and no later stamp, so a future row with a window or count past the lock gets a wrong hand check. No current value is affected. | fsl-analyze/tests/it/timed_oracle/falsifier.rs:236-258 |
| FND-007 | low | The QSL-3485 test injects the undefined reading into whichever atom first reads false and asserts only that one did. It does not assert the state is the time-locked block state, so a change of exploration order can leave the test no longer exercising the ticket. | fsl-analyze/tests/it/timed_oracle/temporal.rs:405-492 |

## Checks

cargo fmt --all -- --check exits 1 on both base and head with the same 391 files and 1016 hunks; the only text difference is the new pub use line appearing as context.
xtask string-edge, xtask canonical-types and arch-lint qualified-core exit 1 on both base and head with identical output, so the PR adds no finding to any of them.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-008 | medium | The retx-offsets and later-start nested-window routes rows pass by asserting EN-5 unsupported TimedFormShape. FR-204 Admission and FSpec FR-421 require an exact verdict, or incomplete while the region process is unbuilt, and never unsupported. The offsets row is FR-204-AC-26's own vector, which states refuted on the maximum 1. Ignore both under QSL-3318 with the hand verdict. | fsl-analyze/tests/it/timed_oracle/probability.rs:565 |
| FND-009 | medium | The on-each, mean-fraction, steps, elapsed and Burst rows pass by asserting EN-5 unsupported not-yet-implemented, which FSpec FR-421 also forbids for these claim forms. A conforming change to incomplete or to the exact verdict fails them. No ticket was found that names these symptoms. | fsl-analyze/tests/it/timed_oracle/probability.rs:720-855 |
| FND-010 | low | The EN-1 decline reader compares result, cause and rule only. FR-243-AC-5 also requires the failing operator's span, so the tagged row goes green on a decline that names the wrong operator. | fsl-analyze/tests/it/timed_oracle/temporal.rs:43-46 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Burst names the NotStochasticIdleTail decline exactly. |
| FND-002 | fixed | Unsettled is gone; each row names one outcome per route. |
| FND-003 | fixed | Stops, sampler errors, invalid requests and missing records are Broken. |
| FND-004 | fixed | The ignore names QSL-3443. |
| FND-005 | fixed | QSL-3494 comment fa0691ea records the Split row as a done condition. |
| FND-006 | fixed | A locked behaviour is a closed trace with no later position. |
| FND-007 | fixed | The undefined reading is keyed to the block state, asserted time-locked. |
