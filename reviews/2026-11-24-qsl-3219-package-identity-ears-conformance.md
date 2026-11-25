---
id: SR-123
title: "EARS conformance analysis of FR-121 package identity"
type: SpecReview
analysis: ears-conformance
scope: "spec/functional/FR-121-locate-a-function-call-site-through-the-replay-facade.md"
review_set: subset
---

## Summary

The scoped engine check emitted two EARS diagnostics and one ambiguous-term diagnostic in unchanged FR-121 behavior prose. The package identity AC is declarative and directly testable; the diagnostics below concern surrounding wording rather than its meaning.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The function-selection refusal continuation is flagged unclassifiable and missing a named subject; rewrite the sentence with an explicit `call_site` subject and an event/condition pattern. | FR-121 Behavior, lines 197-202 |
| FND-002 | low | “Effective attribute set” is flagged as ambiguous; name the inherited/declared attribute set precisely or point to its defining contract. | FR-121 Behavior, lines 291-296 |
