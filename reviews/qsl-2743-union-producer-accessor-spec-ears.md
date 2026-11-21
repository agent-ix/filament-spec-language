---
id: SR-967175026302197768900474181941935084684
title: "QSL-2743 PR #3398 EARS conformance: FR-531 Selected model union producer"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/filament-spec-language@7cfd1558f632b5f26841cdcc218f21b07638e1be; PR #3398 fix-round-2 spec delta; spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md section Selected model union producer (lines 399-427)"
review_set: subset
relationships:
  - { target: "ix://agent-ix/filament-spec-language/FR-531", type: reviews }
---

## Summary

Ticket: QSL-2743. This is an EARS conformance check of the normative statements added to the FR-531 body in fix round 2. They are the first requirement-statement change in this PR, so EARS now applies. Existing FR-531 statements outside the new section are out of scope.

## Verdict

**PASS** (scoped to the new section). I checked six normative statements, all with a SHALL:
- The event-driven retention statement ("When the checked package reaches one, the checking stage SHALL retain ...").
- The ubiquitous classification and lowering-refusal statements.
- The ubiquitous `Emission` exposure statement.
- The SHALL NOT for stand-in and caller-built rows.

Each statement has one responsible actor (checking stage, lowering, final `Emission`, producer) and an observable response. The rekey and closure-use sentences describe how the `Emission` obligation applies to a monitor-plan emission, which is itself a final `Emission`. The non-goal sentences (no `identities.json` member, no `Value::Clock`) are explicit scope limits, not obligations.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

Examined: `FR-531...md:399-427`, all sentences. Context only: AC-45 to AC-49 (lines 1127-1131), to check that each AC derives from a body statement; integrity is covered in SR-967175026302197768900474181941935084683. The computed matrix axis is still unassessed (prior timeout and a cancelled retry), and no repository-wide assurance is claimed.
