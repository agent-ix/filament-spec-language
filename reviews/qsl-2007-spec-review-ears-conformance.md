---
id: SR-2026111911313600000011
title: "QSL-2007 spec-review/ears-conformance review"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/filament-spec-language; QSL-2007; PR 3060 diff and scoped FR-093 correction"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-093
    type: references
examined_units: [{"id":"FR-093-AC-67","path":"spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md","role":"examined","excerpt":"Standalone `convert<Int[0,9]>(n)` over `n: Integer` under `n >= 0 and n <= 9`, and over `q: Int[0,20]` under `q <= 9`, checks and lowers to `filament.op.numeric.narrow` with target member and no mode. Without the respective proof each refuses `undefined_expression`/`unproved-range`. A `let` binding of `rational(-6,-4)` declared `Rational[-20,20;1,20]`, converted to `Rational[0,10;1,10]`, proves normalized `3/2` membership and lowers to `filament.op.rational.narrow`; `11/1` and `1/11` refuse `unproved-range`. `Decimal[0,9;0,0;exact]` to `Integer` is exact; narrowing a scale-zero decimal into `I"},{"id":"FR-093","path":"spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md","role":"examined","excerpt":"The checker SHALL classify an explicit exact-numeric\nconversion by FSpec FR-149's first-match Conversion classes, using the operand's\ndeclared type."},{"id":"FSpec-FR-149","path":"filament-specification/spec/functional/type-model/FR-149-apply-complete-equality-matrix.md","role":"examined","excerpt":"A checked obligation that the converted value is a member of `T` (FR-146 range obligation, `unproved-range`)."},{"id":"FSpec-FR-149","path":"filament-specification/spec/functional/type-model/FR-149-apply-complete-equality-matrix.md","role":"examined","excerpt":"Every other exact-numeric pair has no conversion and is\n`refused { code: ill_typed, cause: operator-ineligible }`."}]
---

## Summary

Checked the two added SHALL sentences and their conversion-table context using the engine and the EARS semantic lens.

Reviewed head: 1937998d4619e0a3c2e8fc3af6a7bc083c59cf50. Ticket: QSL-2007. PR: filament-spec-language#3060.

No applicable AssuranceProfile was found. The reviewer selected the base, integrity and EARS lenses for the scoped FR-093 correction; no relationship/domain-object changes require dependency/object lenses. Optional gap-analysis semantic pass was not requested and was not run; code-review independently checks code/test intent. No aggregate gate, hosted workflow, implementation edit, commit or PR mutation was performed.

## Verdict

PASS for the changed requirement statements.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

Quire validation at the explicit FR-093 path: 1/1 docs grammar-clean, 0 grammar findings. The checker is the subject of the classification requirement; unproved narrowing is the subject/condition of the refusal requirement. Both responses are observable through classification and a stable refusal code/cause. No vague response, ambiguous trigger/state distinction or unmeasurable obligation was found. Supporting table and AC text were read as context rather than demanded to repeat SHALL syntax.

