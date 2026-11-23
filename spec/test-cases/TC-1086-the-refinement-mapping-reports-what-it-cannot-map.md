---
id: TC-1086
title: "The refinement mapping reports an unmapped type, a wrong system and the first failure, and never drops an object"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-140
    type: verifies
  - target: ix://agent-ix/filament-spec-language/FR-120
    type: verifies
---
# TC-1086: The refinement mapping reports an unmapped type, a wrong system and the first failure, and never drops an object

## Description

Verify the rules of `map_state` that TC-545 and TC-555 do not reach: an
unmapped concrete type reached through an object and through a reference,
the two `ModelSystem` arguments and the `over_model` refusal, the order in
which failures are returned, the `only-not-one` reason, and a mapped state
with no relationship extent.

Scope: FR-140-AC-6 to FR-140-AC-14, FR-140-AC-16, FR-140-AC-17, FR-120-AC-14.

## Test Procedure

Fixtures: `RingIsQueue` (FR-139) with the variants FR-140-AC-6 and AC-9
describe; `CasRefinesCounter` (ADR-020 §8) with the `SubCounter` subtype of
FR-140-AC-7.

1. Map the FR-140-AC-6 variant with `q` of type `SubRing`, then with `q` of
   type `Ring`.
2. Map the FR-140-AC-7 state, in which `c` has type `SubCounter`.
3. Map with the two systems of FR-140-AC-1, then with them swapped and with a
   system over a third model.
4. Map the FR-140-AC-9 variants.
5. Map the ring state of FR-140-AC-2 and read its relationship extents.
6. Call `ModelSystem::over_model` over `example/not-selected`, and over each
   of the unit's selected models.
7. Map the ring state of FR-140-AC-2 under the FR-140-AC-12 variant.
8. Parse and check the FR-140-AC-11 row as `head: Int[0, 1]?` and as
   `head: Option<Int[0, 1]>`; then map `head = 1`, `size = 2` and
   `head = 0`, `size = 0` under both variants.
9. Map AC-9's variant with its rows written in another source order. Separately,
   map a state whose synthesized observation hits a broken evaluator invariant,
   with an earlier mapping row whose expression would be undefined for the
   corresponding state with a valid observation.
10. Map the FR-140-AC-16 variant, then with one ring's `chain` holding a
    reference to a `SubRing`.
    Lift an `Option<Reference<_>>`, a `Sequence<Option<Reference<_>>>` and a
    `Map<Reference<_>, Reference<_>>` value, with an unmapped reference in
    each (unit tests of `lift_nested`; no validated package declares a map
    type yet).
11. Choose the row that lifts a reference to a type two object rows name,
    with the declared type naming the first, the second, a supertype and
    neither (unit test of the row choice), and map the ring world's `twin`
    variant, whose `Queue` and `Cell` rows both name `Ring`, in both row
    orders.

Tag the tests `#[trace("TC-1086", "<AC id>")]`.

## Expected Results

- Step 1: `Undetermined` naming the `next` row and object `a`, with
  `UnmappedType(SubRing)`; then `a.next` maps to the `Queue` reference `q`.
- Step 2: `Undetermined` naming `Object` for `SubCounter` and `c`, with
  `UnmappedType(SubCounter)`; no object dropped.
- Step 3: a state whose key is the abstract system's; then `WrongSystem` with
  `invalid_runtime_input`/`wrong-role-mapping` for each, with no row read.
- Step 4: `Undefined` whose `UndefinedRecord` reason is `only-not-one`;
  `Undefined` also when a `SubRing` sorts first; `Undetermined` for `SubRing`
  with the original `items` row.
- Step 5: no relationship extent.
- Step 6: `invalid_runtime_input`/`wrong-role-mapping`; then a system that
  reports its model.
- Step 7: `cells` holds the `Cell` references `s0` and `s1`, in key order.
- Step 8: S2 parses and S3 checks both rows; `Some(self.head)` has type
  `Option<Int[0, 1]>` and `none` takes that contextual type. At `head = 1`,
  `size = 2` the mapped `head` is 1; at `size = 0` it is unset and not hidden
  (`?`), or an explicit `none` (`Option<T>`).
- Step 9: the same reported row and object under the row-order permutation;
  for the broken-observation state, `Fault` with the catalog code before any
  row is read, rather than `Undefined` or `Undetermined`.
- Step 10: `chain` maps to the `Queue` references `[b, a]` in order; with the `SubRing` reference, `Undetermined` naming the
  `chain` row and the holding ring with `UnmappedType(SubRing)`.
  `some` payloads lift and `none` stays `none`; the sequence of options lifts
  in order; the map's keys and values lift with the entries in order; an
  unmapped reference in any of them is `Unmapped`.
- Step 11: `twin` lifts to the `Cell` reference `r` whichever row is written
  first; the declared type picks its row, a supertype of one candidate picks
  that one, of both the more specific, and a declared type that picks none is
  `Ambiguous`.
