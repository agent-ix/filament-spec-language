---
id: TC-540
title: "S3 checks a refinement declaration's subjects, population map and object map"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-135
    type: verifies
---
# TC-540: S3 checks a refinement declaration's subjects, population map and object map

## Description

Verify that S3 checks a refinement declaration's header, population rows and
object rows into a `CheckedRefinement`, refuses each missing, duplicated,
misplaced or ill-typed row at its span, lifts references, records hidden
fields and puts the declaration in the checked package as a node whose body
holds its rows and whose node key does not.

Scope: FR-135-AC-1 to FR-135-AC-19.

## Test Procedure

Fixtures: ADR-020 §8's `CasRefinesCounter` unit, with `Spec` local and with
`Spec` supplied as a dependency package; ADR-020 §8's `RingIsQueue` unit.

1. Check `CasRefinesCounter` with `Spec` local, then with `Spec` as a
   dependency.
2. Check each refusal variant of FR-135-AC-2.
3. Check `value = self.busyA`; check `RingIsQueue` with a
   `Reference<Q::Queue>` field mapped by `self.ring` and by an expression of
   type `Reference<R::Slot>`.
4. Check `CasRefinesCounter` with the `value` row removed; check it with
   `value = self.tmpA`, in a package of step 1's identity and version, and
   compare with step 1's local check the refinement's node key, the `value`
   field row's expression, the hidden fields and the package identity.
5. Check `CasTwice` and `CasRefinesCounter`, each in the build as it stands
   (FR-135-AC-5); check a refinement given a non-empty resolved internal list
   (FR-135-AC-9); check `RegisterHistory`, whose `history` feature the build
   holds for an `Int[0, 1]` row. The step does not choose a support set; it runs
   only while a feature is unbuilt, and is rewritten when the feature lands.
6. Check `RingIsQueue` with a nested `Sequence<Reference<Ring>>[0, 2]` row,
   an `Option<Reference<Ring>>` row and the same field as a plain reference;
   then the `Sequence<Reference<Queue>>[0, 2]` field mapped by a
   `Sequence<Reference<Slot>>[0, 2]` row. Also a `Sequence<Option<Reference<
   Ring>>>[0, 2]` row (two carriers deep); then the `Box` record row
   `box = self.box` with same-named `target` fields of types
   `Reference<Ring>` and `Reference<Queue>`, the `Pair` tuple row
   `pair = self.pair` with `(Reference<Ring>, Boolean)` and
   `(Reference<Queue>, Boolean)`, and the `Holder` union row
   `holder = self.holder` with `Has` payloads carrying those references.
   Finally check `Parcel { owners: Sequence<Option<Reference<Ring>>>[0, 2] }`
   mapped to the same record field with `Reference<Queue>`; its value is
   `[Some(ring b), None]`. The map-key and map-value cases in FR-135-AC-6
   are the `lift_nested` unit cases in TC-1086/FR-140-AC-16; no validated
   package declares a map type.
7. Check `RingIsQueue` with a relationship declared on the abstract `Queue`.
8. Check `RingIsQueue` in a unit that selects no layer of its own, then with
   a Boolean row naming a protocol of the unit.
9. Read the headers `abstract Spec::Twice concrete Impl` and `abstract Spec
   concrete Impl`; check a refinement whose step rows name one protocol clause
   of `Impl`, and one whose rows name none.
10. Give `Spec` and `Impl` one `state_machine` declaration each, with a
    `value` field on each, and map them with one `object` row and no
    `population` row. Remove that row, duplicate it, and pair the abstract
    machine with a concrete object type. Also check `CasRefinesCounter` over
    its object-type-only models.
11. With abstract `Box { target: Reference<Queue> }` and concrete
    `Box { link: Reference<Ring> }`, check `box = self.box`; then change
    the concrete field name to `target` and check again.
12. With abstract `tuple Pair(Reference<Queue>, Boolean)` and concrete
    `tuple Triple(Reference<Ring>, Boolean, Boolean)`, check
    `pair = self.triplet` and record the first unmatched tuple position.
13. With abstract `union Holder { Empty, Has(Reference<Queue>) }` and
    concrete `union State { Empty, Points(Reference<Ring>) }`, check
    `holder = self.state` when the active tag is `Points`; then with concrete
    `union State { Empty, Has(Reference<Ring>) }`, check the same row when
    the active tag matches `Has`.

Tag the composite positive cases with `FR-135-AC-13` through `AC-16`, and
the name, arity and tag refusals with `FR-135-AC-17` through `AC-19`.
Tag all tests `#[trace("TC-540", "FR-135-AC-n")]` for their owning criteria.

## Expected Results

- Step 1: a `CheckedRefinement` with one population row, one object row with
  `value = self.value` and no hidden field; then `AbstractSide::Dependency`
  with the dependency's `package_id`.
- Step 2: each variant refuses with FR-135-AC-2's code and subcode at its
  span, and no `CheckedRefinement`.
- Step 3: `ill_typed`/`type-mismatch` at the expression; the `self.ring` row
  checks; the `Reference<R::Slot>` row refuses `ill_typed`/`type-mismatch`.
- Step 4: with the row removed, `hidden = [value]`. With `value = self.tmpA`,
  a node key equal to step 1's (equal `package` and `path`, ordinal 0 in
  both); the `value` field row reads `self.tmpA` where step 1's reads
  `self.value`; neither check has a hidden field; the same package identity.
- Step 5: `CasTwice` refuses `unsupported_construct`/`not-yet-implemented` at `Twice` naming `protocol-abstract-side`, `CasTwice` and the profile, with no `subject` member; the internal-list refinement refuses the same pair naming `refinement-internal-list`; each classifies `unsupported`; no `CheckedRefinement` for either; `CasRefinesCounter` and `RegisterHistory` check.
- Step 6: the sequence, option, plain-reference and depth-two rows check, each
  nested reference lifted through the `Queue` from `Ring` row; the `Slot` row
  refuses `ill_typed`/`type-mismatch` at the row's expression with no
  `CheckedRefinement`. The record pairs `target` by name, the tuple pairs
  element 0 by position and retains element 1, the union pairs `Has` by tag,
  and the nested record/sequence/option preserves `None` and lifts its
  `Some` payload through the same object row.
- Step 7: `unsupported_construct`/`declaration-form` at the alias `Q`; no
  `CheckedRefinement`.
- Step 8: the rows check; then `unsupported_construct`/`expression-form` at
  the `protocol` token naming `filament.protocol.complete/v1`.
- Step 9: the protocol and model subjects of the two headers; `concrete_protocol`
  naming the clause for the first refinement and absent for the second.
- Step 10: the machine fields check through one `ObjectRow` without a
  population row; omitting the row refuses `missing_declaration`/`missing-name`
  naming the abstract machine; duplicating it refuses
  `invalid_model_binding`/`conflicting-binding`; pairing a machine with an
  object type refuses `invalid_model_binding`/`malformed-declaration`.
  `CasRefinesCounter` checks through its existing population and object rows
  and has no state-machine requirement.
- Step 11: matching `target` fields lift; replacing the concrete field name
  with `link` refuses `ill_typed`/`type-mismatch` at record component path
  `box.target`, with no `CheckedRefinement`.
- Step 12: unequal tuple arity refuses `ill_typed`/`type-mismatch` at the
  first unmatched position `[2]`, with no `CheckedRefinement`.
- Step 13: active `Points` has no paired abstract tag and refuses
  `ill_typed`/`type-mismatch` at `holder.variant`; matching active `Has`
  succeeds and lifts its payload through the object row.
