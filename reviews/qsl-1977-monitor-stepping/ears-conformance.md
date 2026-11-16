---
id: SR-2026111611450302
title: "EARS conformance review of the QSL-2306 monitor stepping rulings"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/filament-spec-language; PR #2750 branch qsl-2306-monitor-spec at 0e9321f13bc364f95141619f57fdae15d9866974, new or amended requirement statements in FR-283, FR-643, FR-644 and FR-820"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-643
    type: reviews
  - target: ix://agent-ix/filament-spec-language/FR-820
    type: reviews
---

## Summary

Ticket: QSL-2306. The statements examined:

- FR-643 Batch equality, Finality, Settling, Trace end and Retained state.
- FR-644's amended event statement and `end` record.
- FR-283 "Finite traces".
- FR-820's Description and its five Behavior bullets.

The evidence is the quire grammar lint from `PATH= quire validate --scope . 'spec/**/*.md'`, which exits 0 and reports no warning on any of the four changed FRs. Each new SHALL statement is ubiquitous, with the session, the state, the events or the spine evaluator as its subject. Two statements carry a rule with no SHALL, so the lint cannot see them.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | "A clause that never meets the rule settles at `finish`." is a normative rule written as a plain indicative with no SHALL, so it is not a requirement statement and the lint skips it. Fix: "The session SHALL settle, at `finish`, every clause that has not met the rule." | spec/functional/FR-643-monitor-a-trace-incrementally-one-observation-at-a-time.md:166 |
| FND-002 | low | FR-283 "Finite traces" states only rationale ("carries no closed or open member", "A caller cannot choose another reading"). FR-283-AC-17 then asserts a refusal, `invalid_runtime_input`/`unknown-member` for a `Document` trace carrying `closed` or `open`, that no Behavior statement requires. Fix: add "If a `Document` trace carries a `closed` or `open` member, then `monitor` SHALL refuse it `invalid_runtime_input`/`unknown-member`." or cite the existing closed-object rule of the trace document. | spec/functional/FR-283-monitor-a-supplied-trace-offline.md:133-144, :538 |

## Verdict

PASS WITH TWO LOWS. The amended FR-643 bullets keep their subject and modal. The Settling profile sub-bullets are definitions under one SHALL, which is acceptable.
