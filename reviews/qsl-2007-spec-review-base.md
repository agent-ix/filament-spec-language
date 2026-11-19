---
id: SR-2026111911313600000009
title: "QSL-2007 spec-review/base review"
type: SpecReview
analysis: base
scope: "agent-ix/filament-spec-language; QSL-2007; PR 3060 diff and scoped FR-093 correction"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-093
    type: references
examined_units: [{"id":"FR-093-AC-67","path":"spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md","role":"examined","excerpt":"Standalone `convert<Int[0,9]>(n)` over `n: Integer` under `n >= 0 and n <= 9`, and over `q: Int[0,20]` under `q <= 9`, checks and lowers to `filament.op.numeric.narrow` with target member and no mode. Without the respective proof each refuses `undefined_expression`/`unproved-range`. A `let` binding of `rational(-6,-4)` declared `Rational[-20,20;1,20]`, converted to `Rational[0,10;1,10]`, proves normalized `3/2` membership and lowers to `filament.op.rational.narrow`; `11/1` and `1/11` refuse `unproved-range`. `Decimal[0,9;0,0;exact]` to `Integer` is exact; narrowing a scale-zero decimal into `I"},{"id":"FR-093","path":"spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md","role":"examined","excerpt":"The checker SHALL classify an explicit exact-numeric\nconversion by FSpec FR-149's first-match Conversion classes, using the operand's\ndeclared type."},{"id":"FSpec-FR-149","path":"filament-specification/spec/functional/type-model/FR-149-apply-complete-equality-matrix.md","role":"examined","excerpt":"A checked obligation that the converted value is a member of `T` (FR-146 range obligation, `unproved-range`)."},{"id":"FSpec-FR-149","path":"filament-specification/spec/functional/type-model/FR-149-apply-complete-equality-matrix.md","role":"examined","excerpt":"Every other exact-numeric pair has no conversion and is\n`refused { code: ill_typed, cause: operator-ineligible }`."}]
---

## Summary

Reviewed the changed ConvertScalar row, Explicit numeric narrowing paragraph and FR-093-AC-67 against merged FSpec FR-149 and the issue's readiness correction.

Reviewed head: 1937998d4619e0a3c2e8fc3af6a7bc083c59cf50. Ticket: QSL-2007. PR: filament-spec-language#3060.

No applicable AssuranceProfile was found. The reviewer selected the base, integrity and EARS lenses for the scoped FR-093 correction; no relationship/domain-object changes require dependency/object lenses. Optional gap-analysis semantic pass was not requested and was not run; code-review independently checks code/test intent. No aggregate gate, hosted workflow, implementation edit, commit or PR mutation was performed.

## Verdict

PASS for the scoped requirement correction.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

FR-093's existing US/ADR/type/operation dependencies remain intact; no relationship changes. Exact, scale-reduction, rational-narrowing and range-narrowing classification follows declared types. The new criterion states target identity/member/mode, missing-proof refusal, normalized rational cases, scale-zero decimal admission, scale-2 refusal, float/direct-equality refusal and runtime backstops. Verification is Test without a new TC bookkeeping document. No new hash/pin/tracking process is specified. The new criterion is tagged by 18 tests. Quire validation of the explicit FR-093 path passed: 1/1 grammar-clean; 0 grammar findings. Repository baseline matrix gaps are recorded separately; this PASS does not assert full repository coverage.

