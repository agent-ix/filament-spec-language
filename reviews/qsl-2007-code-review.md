---
id: SR-2026111911313600000007
title: "QSL-2007 code-review review"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; QSL-2007; PR 3060 diff and scoped FR-093 correction"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-093
    type: references
examined_units: [{"id":"FR-093-AC-67","path":"spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md","role":"examined","excerpt":"Standalone `convert<Int[0,9]>(n)` over `n: Integer` under `n >= 0 and n <= 9`, and over `q: Int[0,20]` under `q <= 9`, checks and lowers to `filament.op.numeric.narrow` with target member and no mode. Without the respective proof each refuses `undefined_expression`/`unproved-range`. A `let` binding of `rational(-6,-4)` declared `Rational[-20,20;1,20]`, converted to `Rational[0,10;1,10]`, proves normalized `3/2` membership and lowers to `filament.op.rational.narrow`; `11/1` and `1/11` refuse `unproved-range`. `Decimal[0,9;0,0;exact]` to `Integer` is exact; narrowing a scale-zero decimal into `I"},{"id":"FR-093","path":"spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md","role":"examined","excerpt":"The checker SHALL classify an explicit exact-numeric\nconversion by FSpec FR-149's first-match Conversion classes, using the operand's\ndeclared type."},{"id":"FSpec-FR-149","path":"filament-specification/spec/functional/type-model/FR-149-apply-complete-equality-matrix.md","role":"examined","excerpt":"A checked obligation that the converted value is a member of `T` (FR-146 range obligation, `unproved-range`)."},{"id":"FSpec-FR-149","path":"filament-specification/spec/functional/type-model/FR-149-apply-complete-equality-matrix.md","role":"examined","excerpt":"Every other exact-numeric pair has no conversion and is\n`refused { code: ill_typed, cause: operator-ineligible }`."},{"id":"FR-062-AC-13","path":"spec/functional/FR-062-implement-checked-family-contract.md","role":"context_only","excerpt":"A record whose result bound is the application's own result type has no conversion context."},{"id":"FR-096-AC-8","path":"spec/functional/FR-096-stage-limits-refusal-records-and-readers-carry-a-locus.md","role":"context_only","excerpt":"A kernel `CheckedInvariant` builds no record."}]
---

## Summary

Reviewed the complete PR diff through the Rust lane, including classification, definedness, claims, lowering, evaluator refusals, corpus expectations and real integration tests.

Reviewed head: 1937998d4619e0a3c2e8fc3af6a7bc083c59cf50. Ticket: QSL-2007. PR: filament-spec-language#3060.

No applicable AssuranceProfile was found. The reviewer selected the base, integrity and EARS lenses for the scoped FR-093 correction; no relationship/domain-object changes require dependency/object lenses. Optional gap-analysis semantic pass was not requested and was not run; code-review independently checks code/test intent. No aggregate gate, hosted workflow, implementation edit, commit or PR mutation was performed.

## Verdict

CONDITIONAL — one reproduced medium defect remains open.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Known fractional decimal membership is tested through the conservative equality conversion table, so decimal(5,1) narrowed to Rational[1,1;2,2] incorrectly refuses unproved-range despite the exact converted value being 1/2. | fsl-semantics/src/check/facts/conversions.rs:72-99 |

## Reproduction

```fsl
function f using v(): Rational[1,1;2,2] pure { let d: Decimal[0,90;1,1;exact] = decimal(5,1) in convert<Rational[1,1;2,2]>(d) }
```

Linked checking returns `Unproved(RationalRange)` / `undefined_expression/unproved-range`. Replacing the let value and declared type with `rational(1,2)` / `Rational[0,90;1,10]` admits the equivalent conversion. A guarded Integer-to-Rational probe also admits. The Rust probe links the pinned head's existing rlibs; source is /tmp/qsl-2007-probe.rs.

The decimal branch constructs a singleton Decimal type and calls `admits_equality_conversion`. That table deliberately overapproximates decimal numerator/denominator ranges, including zero and powers of ten. It cannot establish exact 1/2 membership. Discharge the known converted value's membership through the shared exact-value APIs while retaining the declared-source conversion class, and add this regression case. No implementation changes were made.

## Validation

The requested qsl-slot cargo test command passed: 11 checker/lowering and 7 evaluator integration tests, no ignores. Scoped rustfmt on changed Rust files other than check/mod.rs passed; including that module reports 178 lines of formatting diffs, also reproduced on the merge-base module. The PR adds only one module declaration there. Diff whitespace check passed. No workflow changes. Clippy/aggregate gates were not run; the coder reports a prior strict Clippy baseline failure, which is not independently verified here.

## Coverage

All 18 changed files were inspected. Existing shared `admits_equality_conversion` and `operand_value` APIs were inspected before judging the new classifier; no existing explicit-class classifier was found by searches for ConversionClass, conversion_class, RangeNarrowing and ScaleReduction. New internals do not expand public APIs or executable languages. No introduced unsafe/panic/concurrency/stub defect was found. All 18 tests bind FR-093-AC-67; the helper assertions exercise production checking/lowering/evaluation. The range-target table's fractional-decimal case uses a broad rational target and does not catch the reproduced normalized-fraction edge.

## Disposition status

FND-001 remains open at the reviewed head. No fix-round head was supplied; no fixed disposition is claimed.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | still-open | Reproduced at pinned head 1937998d4619e0a3c2e8fc3af6a7bc083c59cf50. No fix-round head was supplied; this is the initial unresolved disposition, not verification of a coder fix. |
| FND-001 | fixed | Fix commit efd45eed4f checks a known decimal literal's exact rational value against the target domain (fsl-semantics/src/check/facts/conversions.rs:50-64) while classification stays numeric.narrow; regression test fractional_decimal_proves_exact_rational_membership_and_retains_range_narrowing covers admit plus three refusals. |

## New findings (disposition pass 2)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-002 | low | FR-093-AC-67's verification cell is a bare "Test" with no TC id, unlike AC-64..66 in the same table. | spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md:1140 |
| FND-003 | low | An unproved narrowing into Integer or Decimal[..] reports Obligation::Range with required = unbounded, so the refusal payload states no target bound; only the Int target carries its domain. | fsl-semantics/src/check/facts/conversions.rs:116-126 |
