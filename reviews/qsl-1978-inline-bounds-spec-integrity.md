---
id: SR-2026112117451800000000003
title: "Integrity review of the QSL-1978 inline-bound Shop inventory correction"
type: SpecReview
analysis: integrity
scope: "agent-ix/filament-spec-language; PR #3355 at a6a5eb2da723f1478bfbec1490f0f802f1fc11d7 over main 70427772a1be1cc4816788f1688a2db1f02cdb33, diff: FR-710-AC-1, TC-1942 step 1 and its expected result; consistency read against FR-710 Description and Compilation, FSpec FR-154 and FR-208, FCD FR-093 and FR-095"
review_set: subset
relationships:
  - { target: ix://agent-ix/filament-spec-language/FR-710, type: references }
  - { target: ix://agent-ix/filament-spec-language/TC-1942, type: references }
---

## Summary

Ticket: QSL-1978. PR filament-spec-language#3355 changes two lines of FR-710-AC-1 and
adds checks to TC-1942 step 1. The old AC-1 required exactly the object type and
population `constructs` entries. That contradicts FR-710 Compilation ("one `constructs`
entry for each kind it uses") once the inline `Int[0, 999]` and `Int[0, 100]` bounds are
kept as FSpec FR-154 scalar-and-constraints value types. The new AC-1 requires `object_type`,
`value_type` and `population`, and pins the two generated definitions `OrderId` and
`OrderTotal`, their bounds, origins and meaning.

Checked against the owners, read-only: FR-710 Description ("every node, member, `origin`,
multiplicity and `constructs` entry that admission requires") and Compilation (used-kind
inventory, value-type meaning row, origin rule, identity rule). FSpec FR-154 "Value types
bound to a native by `target`" names the `scalar`-and-constraints form of bounded Integer
value types (`filament.meaning.model.value-type/v1`), and its admission table refuses a type
whose kind names no `constructs` entry. FCD FR-095 mints the constrained-field alias identity
`ix://<org>/<name>/<Name><Field>`, and FCD FR-093 gives that extraction route a `kind: alias`
shape. Main's `fsl-semantics/src/model/intake/model_unit.rs` already sets `has_value_type`
for named value types. The inline-producer branch is out of scope, so no runtime behavior
is claimed here.

## Verdict

PASS on the inventory correction, with one medium finding. The three-kind inventory, the
`value_type` meaning and the "not extra authored declarations" wording all follow from
FR-710 Compilation and FSpec FR-154/FR-208. None of them adds a kind, a meaning, a grammar
form or a public member. Inline Option uses the core `"option"` kind, which has no FR-208
construct entry, so it is not a precedent for leaving the custom `value_type` kind out of the
inventory, and the PR does not treat it as one. TC-1942 checks each value AC-1 pins: the
exact references, `scalar` `integer`, `min`/`max` operands, origins and the three-kind
inventory. It keeps the admission, invariant and equal-bytes checks.

FND-001: AC-1 now pins generated identities (`ix://example/shop/OrderId`,
`ix://example/shop/OrderTotal`) and inline-type origins. No FR-710 statement produces or
cites these. Fixing it needs one sentence in FR-710 Compilation that cites the existing FCD
FR-095 constrained-field identity and the FSpec FR-154 scalar form. It needs no new kind or
grammar.

Examined and clean: TC-1942 step 1 and its expected result, the AC-1 inventory clause, the
AC-1 meaning clause, and the unchanged admission, invariant and determinism clauses. A
sibling grep (TC-1943, TC-2998, AC-6) found no other stale two-kind expectation.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-710-AC-1 now requires the fields' `typeRef`s to name generated value types at `ix://example/shop/OrderId` and `ix://example/shop/OrderTotal`, with origins at the inline type spans. FR-710 itself has no rule that compiles an inline bounded native `type-ref` to a generated value type. Its identity rule covers only declarations (`ix://<domain identity>/<name>`), members and relationships. Its origin rule names "the span of its declaration". FR-710 has no relationship to FCD FR-095, which owns the `<Name><Field>` minting. The AC is the only normative source for the identity and the origin span. Nothing says what happens when an authored `type OrderId = ...;` sits beside `object Order { id: Int[0, 9]; }`, where both mint `ix://example/shop/OrderId`. Fix: add one sentence to FR-710 Compilation. An inline bounded native field type becomes a value type in FSpec FR-154's scalar-and-constraints form, at FCD FR-095's constrained-field identity `ix://<domain identity>/<Owner><Field>`, with its origin at the inline type's span. Cite the existing collision refusal or state one. | spec/functional/FR-710-author-a-domain-package-in-an-fsl-model-unit.md:160, spec/functional/FR-710-author-a-domain-package-in-an-fsl-model-unit.md:179-184, spec/functional/FR-710-author-a-domain-package-in-an-fsl-model-unit.md:345 |

## Dispositions

Round 1 at frozen head `019cc8da1140009db20c8ee2e0b0fc07ac27f228`.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | FR-710 Compilation now owns inline bounded Integer scalar-and-constraints generation via FSpec FR-154, references FCD FR-095 constrained-field identity with verbatim Owner and capitalized field slug, preserves source identity and inline span for the definition and bounds, and refuses authored/generated collisions at that span with no document. TC-1942 step 4 checks authored/generated collision in both declaration orders. |

No new findings in disposition round 1. Examined: FR-710 Compilation lines 196-208 and the FCD FR-095 references edge; TC-1942 step 4 and expected result; nearest identity, origin and refusal rules; FCD FR-095 constrained-field identity and its existing minter; FSpec FR-154 bounded Integer scalar-and-constraints form. FCD FR-093 extraction alias shape is not substituted for the native scalar form. No implementation behavior is claimed.
