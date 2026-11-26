---
id: SR-967187
title: "QSL-3513 integrity analysis of the cross-binding order, conformance objects and policy instances (filament-spec-language PR 4040)"
type: SpecReview
analysis: integrity
scope: "agent-ix/filament-spec-language@adf503520a905d72041960c5832c201e87296279; spec/decisions/ADR-018, spec/functional/FR-126, FR-621, FR-896, FR-990, FR-1012, spec/test-cases/TC-1085, TC-2532, TC-2800"
review_set: subset
date: 2026-11-26
---

## Summary

Ticket: QSL-3513. PR filament-spec-language#4040, head adf503520a905d72041960c5832c201e87296279.

Consistency pass over the amended rules against the rest of the spec tree and the
consumer code. FR-126, ADR-018 UE-4, TC-1085 and TC-1201 agree with one another.
Two conflicts remain, both with text this PR does not edit.

Not run: gap-analysis (the diff has no production code).

## Verdict

CHANGES REQUESTED. FND-001 and FND-002 should be fixed in this PR.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The sentence now requires every engine that returns one outcome over all of a temporal item's instances to report the first refuted instance. FR-315 (lines 370-381 and 558) still has the SMT query cover all instances together and attribute the counterexample to the instance failing at the least position `p`. Failure: instance 0 first violates `always` at position 3 and instance 1 at position 2; at depth 3 or more FR-315 rule 4c names instance 1, while FR-621 and FR-126 require instance 0. The PR scopes the matching ADR-018 text to EN-1 and amends nothing for EN-2 and EN-3. Fix FR-315 here (a search of earlier instances, as FR-126 "Safety items" states for EN-1). | spec/functional/FR-621-write-the-analyze-result-as-one-outcome-document-per-request.md:244-249 |
| FND-002 | medium | The two bullets cover a `subject.protocol` naming the claimed clause and no `protocol` member, not a `protocol` naming another clause. FR-1033 (lines 93-97) says that when the subject carries `protocol` the item is decided over that protocol subject, and otherwise over the model subject. Failure: a request-level default `subject.protocol` A with a `global-conformance` item claiming B is decided over A's subject by FR-1033, over each key of B by one reading of FR-896, and is `invalid_value` at `temporal.subject.protocol` in today's `analyze/document.rs`. State the outcome, as FR-1033 does for race-freedom, and make FR-1033's no-`protocol` sentence agree with the per-key subjects. | spec/functional/FR-896-decide-a-global-conformance-claim-over-the-protocol-system-or-a-recording.md:92-100 |
