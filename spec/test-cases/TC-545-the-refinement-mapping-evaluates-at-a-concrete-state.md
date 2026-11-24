---
id: TC-545
title: "The refinement mapping builds the abstract state from a concrete state and its history"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-140
    type: verifies
---
# TC-545: The refinement mapping builds the abstract state from a concrete state and its history

## Description

Verify `map_state`: the key-preserving population image, reference lifting,
field rows through the clause evaluator, history reads, hidden slots, and
`MappingFailure::Undefined` for a row that evaluates undefined.

Scope: FR-140-AC-1 to FR-140-AC-4, FR-140-AC-15 and FR-140-AC-19 to
FR-140-AC-22.

## Test Procedure

Fixtures: `CasRefinesCounter` (ADR-020 §8); `RingIsQueue` (FR-139);
`RegisterHistory` (FR-138).

1. Map `(1, 0, f, 0, t)`, `(0, 0, t, 0, f)` and `(0, 0, f, 0, f)` and
   compare keys with states built in `Spec`.
2. Map the ring state of FR-140-AC-2.
3. Map ring states with `size >= 1` under FR-140-AC-3's `only` condition.
4. Map `value = 1`, `last = 0`, with and without the `prev` row.
5. Map a state of `TwoPopulations` (FR-140-AC-15) with both populations undefined, then with only the second in key order undefined.
6. Over `Box`, map ring `a` whose `box.target` is the reference to ring `b`.
7. Over `Pair`, map ring `a` whose pair is `Pair(Reference<Ring>(b), true)`.
8. Over `Holder`, map `Holder::Has(Reference<Ring>(b))` and `Holder::Empty`.
9. Over `Parcel`, map ring `a` whose owners are
   `[Some(Reference<Ring>(b)), None]`.

Tag the tests `#[trace("TC-545", "FR-140-AC-n")]`.

## Expected Results

- Step 1: `c.value = 1` with a key equal to `Spec`'s; the last two keys are
  equal.
- Step 2: one `Queue` `r` with `items = [1, 0]` and no slot objects.
- Step 3: `MappingFailure::Undefined` naming the `items` row, `r` and the
  undefined cause.
- Step 4: `value = 1`, `prev = 0`; then `hidden = [(r, prev)]`.
- Step 5: the `Undefined` of the population first in key order; then of the other.
- Step 6: `Queue a.box.target` is `Reference<Queue>(b)`.
- Step 7: `Queue a.pair` is `Pair(Reference<Queue>(b), true)`.
- Step 8: `Has` retains its tag and maps its payload to `Reference<Queue>(b)`;
  `Empty` retains its tag.
- Step 9: `Queue a.parcel.owners` is
  `[Some(Reference<Queue>(b)), None]`, preserving sequence order and option
  presence.
