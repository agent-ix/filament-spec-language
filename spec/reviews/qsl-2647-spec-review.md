---
id: SR-202611200301101
title: "base review of the QSL-2647 FSL export refusal and admission-cause spec (FR-1015, TC-3020)"
type: SpecReview
analysis: base
scope: "agent-ix/filament-spec-language; spec/functional/FR-1015, spec/test-cases/TC-3020 (PR #3133 head 048753fbd00dbe4bf79768d045b2e566b0ee67b7)"
review_set: base
---

## Summary

Ticket: QSL-2647. Base spec review of PR #3133. FR-1015's new text and
FR-1015-AC-15/16 match FSpec FR-817 (the seven-member
`unimplemented_capability`/`unimplemented-meaning` refusal, exit 21, never
30; the admission refusal keeping its own code, cause, locus and exit) and
FR-817-AC-16/17. TC-3020's scope, steps and expected results cover both
criteria. No AC id collides with origin/main. `quire validate` exits 0.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verdict

Mergeable on its own content. It should merge with or after FSpec #764,
whose FR-817 text it cites; the FSpec batch's own findings do not touch
FR-1015.
