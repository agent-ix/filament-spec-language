---
id: TC-2361
title: "A source model's time member is written with a source object type's name"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-230
    type: verifies
---
# TC-2361: A source model's time member is written with a source object type's name

## Description

Verify that `time O dense unit ms;`, with `O` a source object type, is the
time member of the unit's source model, that it times every clock of the
source model, and that a selected model's time member never times a source
model clock. Scope: FR-230-AC-65.

Catches a time member that resolves only model aliases, a source time source
that times only the named type's own clocks, a second source time member
admitted, and a selected model's dense source accepted for a source clock.

## Test Procedure

1. Check, on the source route, the FR-230-AC-65 unit with
   `time O dense unit ms;`. Read the checked time source and the source
   model's clock places. Waits on QC-30 (target emission) and the FSpec
   FR-322 source-object section: read the emitted timed-model root's
   `model` reference.
2. Check the unit with `time Q dense unit ms;` in place of the dense member.
3. Check the unit with a second member `time P dense unit ms;`.
4. Check the unit with `time O tick O::step period 1 ms unit ms;` in place
   of the dense member.
5. Check the unit with no source time member, selecting a model `M` whose
   unit has `time M dense unit ms;`.

## Expected Results

- Step 1: no refusal; the time source holds the name `O` and the `NodeKey`
  of `O`'s `model`/`object_type` node, the clock places include `w.c` and
  `d`; once emission lands, the root's `model` reference is that node.
- Step 2: `missing_declaration`/`missing-name` at `Q`.
- Step 3: `invalid_timed_model`/`duplicate-time-source` at the second
  member, naming the first.
- Step 4: `invalid_timed_model`/`clock-without-dense-time` at the span of
  the member `w`.
- Step 5: `invalid_timed_model`/`clock-without-dense-time` at the span of
  the member `w`; `M`'s time member times no source model clock.
