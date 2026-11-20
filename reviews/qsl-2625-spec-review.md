---
id: SR-20261120073952
title: "spec-review of FR-264 Behavior 5 and AC-1 edit in PR 3157"
type: SpecReview
analysis: base
scope: "agent-ix/filament-spec-language; spec/functional/FR-264-emit-and-read-v2-packages-at-schema-fixed-depth.md; PR 3157 head e21589a2f285fef2f3db2c688c3094b86f7ccb92"
review_set: subset
---

## Summary

Ticket: QSL-2625 (also QSL-2648, QSL-2652). Review of the sentence added to FR-264 Behavior 5 and the AC-1 change.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Behavior 5 charges "path length" against a byte bound without stating the unit; this conflicts with AC-6 (N canonical bytes emit at bound N) and with the refusal's "byte count the refused write would have reached" | spec/functional/FR-264-emit-and-read-v2-packages-at-schema-fixed-depth.md:65-69 |
| FND-002 | low | AC-1 carries a parenthetical rationale; the 100,000-term refusal would read better as its own AC | spec/functional/FR-264-emit-and-read-v2-packages-at-schema-fixed-depth.md:75 |

## Verdict

One medium. The rest of the edit matches the lead ruling.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | fixed | - |
