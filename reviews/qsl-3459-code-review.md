---
id: SR-241016471313536418021891063182158177098
title: "Code review — QSL-3459 PR 4021 writer and strict-reader round trips"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; fsl-analyze/tests/it/model_trace_run.rs, fsl-analyze/tests/it/probability_from_source.rs, reviews/26-11-25-qsl-3459-code-review.md"
review_set: subset
---

## Summary

Independent review (code-review with the rust-review lane) of the test-only PR 4021 at head 8fdd9d9705d361f0cd5700eeb905f6cfffe3c64e. Ticket: QSL-3459. The new assertions run real writers and strict readers, but the certificate comparison cannot detect a value lost or changed on the first read.

## Verdict

**CONDITIONAL** — two medium and two low findings; no high finding and no product change.

The run-result expected values (`incomplete`, `work_units`) are written by hand and agree with FR-267's incomplete row. The unchanged history-initials test does call the strict reader and asserts admission; the PR body states its limit correctly. `rustfmt --check` on the two changed files and `git diff --check` pass. No build or test was run in this review; the 69-test result is the author's. quoin:gap-analysis was not run (no production code changed) and quoin:spec-review was not run (no spec or plan change).

## Findings

| ID      | Severity | Summary | Refs |
| ------- | -------- | ------- | ---- |
| FND-001 | medium | Certificate round trip compares decode 1 with decode 2 only, so a member the reader drops or alters on the first read passes; no decoded value is compared with the emitted bytes or a hand-written value | fsl-analyze/tests/it/probability_from_source.rs:1924-1944 |
| FND-002 | medium | The PR adds the author's own SpecReview with a PASS verdict and no findings; it is stale, overstates the comparison and contradicts the PR body | reviews/26-11-25-qsl-3459-code-review.md:1-39 |
| FND-003 | low | Run-result read-back compares category and limit only; stage, basis and truth are exposed by the reader and not asserted | fsl-analyze/tests/it/model_trace_run.rs:275-286 |
| FND-004 | low | New run-result read-back assertions are bound only to FR-641-AC-4, which says nothing about reading the written record; the reading-back criterion is FR-267-AC-14 | fsl-analyze/tests/it/model_trace_run.rs:239-240 |
