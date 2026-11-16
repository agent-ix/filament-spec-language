---
id: SR-2175003
title: "EARS conformance review of ValueTypeRef::Resolved for source object members (filament-spec-language PR 2755)"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/filament-spec-language; spec/functional/FR-088-clause-name-and-type-identity.md; spec/functional/FR-569-declare-object-types-populations-and-operations-in-fsl-source.md; spec/test-cases/TC-260-value-type-ref-is-exactly-five-member-union.md; spec/test-cases/TC-1940-source-object-member-types-check-as-record-fields.md"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-088
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-569
    type: references
  - target: ix://agent-ix/filament-spec-language/TC-260
    type: references
  - target: ix://agent-ix/filament-spec-language/TC-1940
    type: references
---

# EARS conformance review of ValueTypeRef::Resolved (PR 2755)

## Summary

Ticket: QSL-2175. This pass read the new and edited requirement statements at head 6411877ebca3c0d1e007e19f410e7a274b668acf:

- FR-088's new Behavior subsection: "A model field type SHALL be a `ValueTypeRef`", "`Resolved`
  SHALL NOT have a wire form", "Only the assembler's source model SHALL build it", "Domain
  package intake SHALL never produce it", "A diagnostic SHALL print it" and "the binding ...
  SHALL name the type node".
- The new FR-569 "The source model" paragraphs.

quire's grammar lint reports nothing on either file. Most of FR-088's statements are singular
ubiquitous requirements with a named subject.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | low | The new FR-569 rules (a member record's multiplicity and `value_type`, "The assembler fills every member's type", and the `Resolved` conformance and clock-place bullets) are written in the indicative ("is", "holds", "conforms", "fills") with no SHALL. Only the section's opening sentence carries an obligation, and that sentence is about building the records, not about these rules. Fix: "The assembler SHALL record ...", "The assembler SHALL fill ... once the unit's type namespace is assembled", and "The checker SHALL decide conformance of a `Resolved` type as follows". | spec/functional/FR-569-declare-object-types-populations-and-operations-in-fsl-source.md:223-248 | missing-requirement |
| FND-002 | low | "`Resolved` SHALL NOT have a wire form" puts a type as the subject of a SHALL; no component carries the obligation. Fix: "No serializer or wire writer SHALL encode `Resolved`", which is how FR-088-AC-8 already tests it. | spec/functional/FR-088-clause-name-and-type-identity.md:246 | wrong-requirement |

## Verdict

PASS with low findings. Neither finding blocks the review.
