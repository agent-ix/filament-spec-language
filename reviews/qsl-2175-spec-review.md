---
id: SR-2175001
title: "spec-review (base) of ValueTypeRef::Resolved for source object members (filament-spec-language PR 2755)"
type: SpecReview
analysis: base
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

# Spec review (base) of ValueTypeRef::Resolved (PR 2755)

## Summary

Ticket: QSL-2175. One spec-review pass over `git diff origin/main...HEAD` at head
6411877ebca3c0d1e007e19f410e7a274b668acf: the FR-088 Outputs bullet, the new FR-088 Behavior subsection "A model field type
names a package node, except a source object member's", FR-088-AC-8 (five-member union),
the new FR-569 "The source model" paragraphs, FR-569-AC-15, TC-260 (renamed) and TC-1940.
Read against the lead ruling, FR-088-AC-7, FR-092-AC-21, FR-230 "Records and admitted
aggregate paths", FR-569 "Member types" and its FSpec FR-151 redefinition clause, and FSpec
FR-146, FR-149, FR-151, FR-154 and FR-322 at filament-specification origin/main. The
model-side value conformance in `fsl-semantics/src/model/index.rs` (`value_conformance_walk`)
was read as context for the rule the amendment mirrors.

Sub-analyses: integrity (SR-2175002), EARS conformance (SR-2175003) and criterion strength
(SR-2175004). The criterion-strength lens ran as a manual reviewer judgment: the Jev client
the quoin skill needs is not installed, so no calibrated Jev scores were produced.

`PATH= /Users/peter/.cargo/bin/quire validate --scope . 'spec/**/*.md'` exits 0; no warning names
a changed file.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | The Resolved-to-Resolved conformance rule does not cover every pairing. It names interval containment, `Int[..]` within `Integer`, and "component-wise" `Option`/collection conformance, but not identity (an identical `Decimal`, enum or record pair is not covered), not alias expansion, not `Decimal`/`Rational`/`Float`/`Quantity`/`Text`/enum/record/tuple/union/model-owned pairs (identity only, or FSpec FR-149's widening rows?), and for an inner collection not whether the kinds must match or how inner bounds compare. Fix: write the rule as a closed list, for example "S conforms to T exactly when they are the same type after alias expansion; or both are integer types and S's interval lies in T's (`Int[..]` within `Integer`); or both are `Option` and the payloads conform; or both are the same collection kind, the elements conform and S's bounds lie within T's; a `Reference` component conforms by FSpec FR-151. No other pair conforms." This matches `value_conformance_walk`, which admits only integer containment and object reachability beyond equality, so the source route is not narrowed against the model route. | spec/functional/FR-569-declare-object-types-populations-and-operations-in-fsl-source.md:238-244 | missing-requirement |
| FND-002 | medium | The rule is attributed to "FSpec FR-146 interval containment", but FSpec FR-146 is "Check total pure functions" and holds no conformance rule. The integer rows are FSpec FR-149's equality-conversion table, which FSpec FR-151's field value-type axis cites. FR-149 also says no option or collection-kind conversion is an equality conversion, so the component-wise `Option`/collection part is this FR's own rule. Fix: cite FSpec FR-151 (with FR-149's `Int` rows) for the integer part, and state the component-wise part as FR-569's rule for operation parameters and results, where FSpec FR-151's operation rows say only "conforms". | spec/functional/FR-569-declare-object-types-populations-and-operations-in-fsl-source.md:239-241 | wrong-requirement |
| FND-003 | low | The cross-variant rule is one-directional: "A `Resolved` type does not conform to a type of any other variant" says nothing about `Package` conforming to `Resolved`. A result override from `Option<Reference<A>>` (Resolved) to `Reference<A>` (Package) needs `Package` ⊑ `Resolved`, and this text does not decide it. Fix: add "and no type of another variant conforms to a `Resolved` type". | spec/functional/FR-569-declare-object-types-populations-and-operations-in-fsl-source.md:244 | missing-requirement |

## Verdict

REQUEST CHANGES, on FND-001 (and FND-001 of SR-2175002). The modelling is coherent with the
ruling and with the cited requirements:

- `Resolved` adds no second identity beside the checked node id (FR-088-AC-7).
- A `Resolved` member's type is the record field's `ValueType` (FR-092-AC-21).
- Clock places follow the record field (FR-230).
- No domain-package record kind is invented (FSpec FR-154, FR-322).
- "No wire form" fits a `Display` that prints the FSL spelling and an emission that binds the
  record field's type node (FR-569-AC-14). The value is never serialized and only its type node
  is named, so nothing needs a wire encoding.
- Nothing admitted before is narrowed: FR-569-AC-12's member types are unchanged, and the
  integer containment the model route admits is kept.

The remaining work is to close the conformance rule over every pairing.
