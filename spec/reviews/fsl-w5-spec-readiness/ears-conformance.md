---
id: SR-2026111608544803
title: "EARS review of the FSL wave 5 spec readiness batch"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/filament-spec-language; PR #2744 branch fsl-spec-w5 at f38affa78, git diff origin/main...HEAD (FR-132, FR-255, FR-267, FR-501, FR-624, FR-625, FR-627, FR-640, FR-641, NFR-017, TC-1160, TC-1584, TC-1585, TC-1587, TC-2241..TC-2246; stubs in fsl-analyze, fsl-command and tests/it)"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-641
    type: reviews
  - target: ix://agent-ix/filament-spec-language/FR-640
    type: reviews
---

## Summary

Ticket: QSL-2295. New SHALL statements checked: FR-640's four hyper-clause
bullets (ubiquitous, named subjects: emitter, model check and run, replay
recompile), FR-641's half-open and `max_candidates` bullets (event and
unwanted-behaviour forms), and NFR-017's fence rules. `quire validate`
reports EARS warnings on FR-640 lines 150 and 156, which are pre-existing text,
and one on new text, below.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The new half-open bullet ("When a progress assertion's watermark is `{"half_open": 0}`, the reader SHALL refuse ...") is flagged `ears:unclassifiable` and `ears:missing-subject`. The subject comes after a long object clause that quire cannot split. Fix: lead with the trigger and the subject, for example "When the run reader reads a progress watermark `{"half_open": 0}`, it SHALL refuse ...". | spec/functional/FR-641-carry-every-temporal-trace-kind-and-the-progress-ledger-on-the-run-request.md:139 |

## Verdict

PASS with one low finding.

## Dispositions

Round 2, verified at head a158bccc0.

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | The bullet now reads "When a request carries the progress watermark `{"half_open": 0}`, the run reader SHALL refuse ...". `quire validate` reports no EARS warning on FR-641. |
