---
id: SR-2026111911313600000008
title: "QSL-2007 gap-analysis review"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; QSL-2007; PR 3060 diff and scoped FR-093 correction"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-093
    type: references
examined_units: [{"id":"FR-093-AC-67","path":"spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md","role":"examined","excerpt":"Standalone `convert<Int[0,9]>(n)` over `n: Integer` under `n >= 0 and n <= 9`, and over `q: Int[0,20]` under `q <= 9`, checks and lowers to `filament.op.numeric.narrow` with target member and no mode. Without the respective proof each refuses `undefined_expression`/`unproved-range`. A `let` binding of `rational(-6,-4)` declared `Rational[-20,20;1,20]`, converted to `Rational[0,10;1,10]`, proves normalized `3/2` membership and lowers to `filament.op.rational.narrow`; `11/1` and `1/11` refuse `unproved-range`. `Decimal[0,9;0,0;exact]` to `Integer` is exact; narrowing a scale-zero decimal into `I"},{"id":"FR-093","path":"spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md","role":"examined","excerpt":"The checker SHALL classify an explicit exact-numeric\nconversion by FSpec FR-149's first-match Conversion classes, using the operand's\ndeclared type."},{"id":"FSpec-FR-149","path":"filament-specification/spec/functional/type-model/FR-149-apply-complete-equality-matrix.md","role":"examined","excerpt":"A checked obligation that the converted value is a member of `T` (FR-146 range obligation, `unproved-range`)."},{"id":"FSpec-FR-149","path":"filament-specification/spec/functional/type-model/FR-149-apply-complete-equality-matrix.md","role":"examined","excerpt":"Every other exact-numeric pair has no conversion and is\n`refused { code: ill_typed, cause: operator-ineligible }`."},{"id":"FR-062-AC-13","path":"spec/functional/FR-062-implement-checked-family-contract.md","role":"context_only","excerpt":"A record whose result bound is the application's own result type has no conversion context."},{"id":"FR-096-AC-8","path":"spec/functional/FR-096-stage-limits-refusal-records-and-readers-carry-a-locus.md","role":"context_only","excerpt":"A kernel `CheckedInvariant` builds no record."},{"id":"FR-001-AC-7","path":"spec/functional/FR-001-read-exact-source.md","role":"examined","excerpt":"Package declarations holding one record `Point` with field `x: Int[0, 9]`, checked under the source reference of bytes `b` admitted as authority `a`, identity `u`, under the reference of bytes `b'` admitted as `a`, `u`, under the reference of `b` admitted as authority `c`, identity `u`, and under the reference of `b` admitted as authority `a`, identity `v`, give `Point` the same node key `{package, path: [\"Point\"], ordinal: 0}` each time: the source reference is no part of a node key."}]
---

## Summary

Ran the complete repository matrix and coverage scan, then inspected the changed behaviors for requirement ownership and hollow implementations. The broad repository assurance result is not a claim that QSL-2007 introduces these baseline gaps.

Reviewed head: 1937998d4619e0a3c2e8fc3af6a7bc083c59cf50. Ticket: QSL-2007. PR: filament-spec-language#3060.

No applicable AssuranceProfile was found. The reviewer selected the base, integrity and EARS lenses for the scoped FR-093 correction; no relationship/domain-object changes require dependency/object lenses. Optional gap-analysis semantic pass was not requested and was not run; code-review independently checks code/test intent. No aggregate gate, hosted workflow, implementation edit, commit or PR mutation was performed.

## Verdict

FAIL for repository-wide traceability; the new FR-093-AC-67 is tagged by all 18 nonignored tests. The baseline finding is deferred from this implementation ticket.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The computed matrix reports no test tag for active criterion FR-001-AC-7; this is an existing repository assurance gap outside the QSL-2007 diff. | spec/functional/FR-001-read-exact-source.md |

## Coverage

Plan completion: not assessed

Reconciliation: quoin matrix (quoin 0.36.7, quire 0.52.0 / engine 0.62.2). 5,918 matrix criteria: 4,329 tagged, 1,200 untagged, 272 tagged-by-ignored-test, 104 method-without-symbol, 13 retired. Every row reports no run evidence. Raw matrix: /tmp/qsl-2007-matrix.json. No aggregate gate was run.

Quire coverage returned exit 0 with 20,654 diagnostics: 20,360 trace-id-outside-marker, 264 untracked-id-has-minted-children, 22 uncatalogued-verification-method, 6 section-matches-nothing, 1 archetype-matches-nothing and 1 catch-all-universal; 4,710 untracked-symbol entries. Raw scan: /tmp/qsl-2007-coverage.json. These limits prevent a repository completeness claim. Several untagged rows explicitly say RETIRED, and the new FSpec-FR-149 context tags are external references under AGENTS.md, not assumed typo findings. FR-001-AC-7 is an inspected active example; the remaining bulk signals have not been individually adjudicated as defects.

The scoped new criterion FR-093-AC-67 is tagged by all 18 real integration tests, none ignored. No claim of fresh execution evidence in the Quoin store is made from the local cargo run. Static tags are traceability, not correctness: code-review FND-001 independently identifies a missed edge.

Reverse-gap review of the changed surface: four conversion classes owned by merged FSpec FR-149/FR-093; membership discharge owned by FR-093; lowering by FR-093; claim conversion context by FR-062-AC-13; runtime backstops by FR-093/FR-096. Seven behavior groups examined, zero unowned changed behaviors, zero changed source stubs, zero changed test stubs. A complete manual reverse inventory of the unrelated repository surface was not performed; the review does not certify it. Optional semantic gap pass skipped.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | deferred | Existing FR-001 traceability is outside QSL-2007. The owner's repository instruction explicitly tracks incomplete assurance separately from engineering delivery. This does not mark the obligation verified. |

