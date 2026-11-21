---
id: SR-2026112022020331072808
title: "Spec review (integrity) — FR-953-AC-1 edit in PR #3136"
type: SpecReview
analysis: integrity
scope: "agent-ix/filament-spec-language; PR #3136 at 4f776b5c9a982bf4e2b0821cbc216f5fd20e618c; spec/functional/FR-953-emit-a-presence-quantifier-over-the-slot-domain-of-its-clock-collection.md (AC-1 row only)"
review_set: subset
relationships:
  - { target: "ix://agent-ix/filament-spec-language/FR-953", type: references }
---

## Summary

Ticket: QSL-2178. The PR edits one spec row, FR-953-AC-1, and the program lead allowed exactly that edit: Map<Text, Clock> with presence-optional entries, plus the exact refusal for an option-wrapped clock. No other file under spec/ changed. `quire validate` on the file passes, with only two existing EARS warnings on the statement at line 56, which this PR did not touch.

## Verdict

**PASS with one low finding.** The edit stays within the authorised change and names the refusal code, cause and location exactly.

## Scope

| Unit | Role | Excerpt |
| --- | --- | --- |
| FR-953-AC-1 | examined | ... For `mo: Map<Text, Clock>[0, 2]`, map membership supplies the optional clock-place presence and the slot-domain node is over `Set<Text>` with a `Text` binder. A map value declared as `Option<Clock>` is refused at `TypeDefinition.inner` with `invalid_model_binding`/`malformed-declaration` (`invalid-option-inner`). |
| FR-953 statement | context_only | The quantifier application whose operand 0 is the slot-domain node. |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | FR-953-AC-1 is now compound: the Sequence emission and identity contract, the Map key-domain variant, and an intake refusal in one row, so one test tag cannot show which part is verified. The intake refusal belongs to an admission requirement and the Map variant could be its own AC. | spec/functional/FR-953-emit-a-presence-quantifier-over-the-slot-domain-of-its-clock-collection.md:67 |
