---
id: SR-2026111613152602
title: "EARS conformance review of the QSL-2311 settlement member"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/filament-spec-language; PR #2752 branch qsl-2311-settlement at 1ba65452c98d984b69ac158aad5352877c69b1ec: new and edited requirement statements in FR-643 (Batch equality, Settlement) and FR-644 (Outputs)"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-643
    type: reviews
  - target: ix://agent-ix/filament-spec-language/FR-644
    type: reviews
---

## Summary

Ticket: QSL-2311. The diff adds two SHALL statements, both in FR-643's Settlement bullet, and extends one Batch equality statement. The FR-644 additions sit in its Outputs section and are descriptive. The Decision bullets carry no SHALL, which is correct for decision text. `quire validate` reports no EARS or quality warning on a changed file. Both statements use the ubiquitous pattern. One of them names a report, not the system, as its subject.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | "A settled clause's report SHALL be exactly `B(s)` ..." makes the report the subject, so the statement allocates to no component. Its neighbour "The session SHALL report the reading ..." names the session. Fix: "The session SHALL report, for a settled clause, exactly `B(s)` ...". | spec/functional/FR-643-monitor-a-trace-incrementally-one-observation-at-a-time.md:185-188 |

## Scope and evidence

- FR-643 Settlement, first statement (spec/functional/FR-643-monitor-a-trace-incrementally-one-observation-at-a-time.md): A settled clause's report SHALL be exactly B(s), the batch report over its deciding prefix, with no member taken from the reading that decided it. Role: examined.
- FR-643 Settlement, second statement (spec/functional/FR-643-monitor-a-trace-incrementally-one-observation-at-a-time.md): The session SHALL report the reading that decided the clause beside it as its Settlement. Ubiquitous, subject named. Role: examined.
- FR-643 Batch equality, settled bullet (spec/functional/FR-643-monitor-a-trace-incrementally-one-observation-at-a-time.md): a clause that settled at call s <= k SHALL report exactly B(s) ... the reading that decided it is its Settlement, never part of its report. Ubiquitous; the subject (clause) is unchanged from the base text. Role: examined.
- FR-644 Outputs event and summary text (spec/functional/FR-644-stream-observations-through-the-monitor-command.md): descriptive output contract, no SHALL. Role: examined.

## Verdict

PASS WITH ONE LOW.
