---
id: SR-967175026302197768900474181941935084876
title: "gap analysis of PR 4061: trace tags and implements claims for the timed oracle and measure readers (QSL-3491)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; fsl-analyze/tests/it/timed_oracle.rs, fsl-analyze/tests/it/timed_oracle/, fsl-eval/src/temporal/timed_measure.rs, fsl-eval/tests/it/timed_measures.rs, spec/functional/FR-204, FR-254, FR-632, FR-852"
review_set: subset
---

## Summary

Ticket: QSL-3491. Head a6fa85472f527b9c3b0e991178c4ac8c5eaeb061. Plan completion: not assessed.
Changed criteria were checked by reading each tag against the criterion text and by searching the repository for other tags of the same id; the full computed matrix was not run. Semantic review was done by hand.
No stub was found in the production readers. timed_fraction (FSpec-FR-407-AC-21) and timed_elapsed (AC-26) carry implements tags that match, with correct hand vectors.

## Verdict

The production readers are covered. Several test tags report coverage that the tagged tests do not give.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Falsifier-only hand tests carry product criterion tags. row_3478_fractional_trap_hand is tagged FR-852-AC-2 and AC-3 but runs only TimedSystem delay and steps, so the matrix reports those criteria exercised by a test that cannot fail on any claim verdict. | fsl-analyze/tests/it/timed_oracle/temporal.rs:237-246 |
| FND-002 | medium | Every FR-204-AC-26 tag in the repository is in this oracle. The tagged tests are hand tests that never run EN-5, ignored tests, or routes tests that also pass on a decline, and none asserts the certificate route region, so the criterion reads covered while an EN-5 regression to unsupported on its own vectors passes. | fsl-analyze/tests/it/timed_oracle/probability.rs:335-540 |
| FND-003 | low | FR-632-AC-2 is about the AC-1 claims on the closed subject settling on en-1 with basis exhaustive and replay. The two QSL-3485 tests use another subject and assert neither. | fsl-analyze/tests/it/timed_oracle/temporal.rs:391-492 |
| FND-004 | low | timed_steps carries implements FSpec-FR-407-AC-23 and its tests the same tag, but only steps is built and tested. accumulate is not, so the matrix reports the whole criterion implemented and covered. | fsl-eval/src/temporal/timed_measure.rs:215-216 |
