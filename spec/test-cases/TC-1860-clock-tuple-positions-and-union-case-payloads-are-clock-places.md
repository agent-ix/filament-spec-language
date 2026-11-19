---
id: TC-1860
title: "Clock tuple positions and union case payloads are clock places"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-230
    type: verifies
  - target: ix://agent-ix/filament-spec-language/FR-056
    type: verifies
---
# TC-1860: Clock tuple positions and union case payloads are clock places

## Description

Verify that a domain package whose tuple position or variant case
`payloadType` is `ix://filament/native/Clock` is admitted, that the checker
lists those clocks as places (unguarded under a tuple position, guarded by the
active case under a union case), and that they are read and written only as
clock places.

Scope: FR-230-AC-63; FR-056-AC-25 (the `Clock` payload half).

## Test Procedure

Fixture: a timed domain package (`time dense`, unit `ms`) with tuple
`Slot(Clock, Integer)`, variant type `Phase` with cases `Armed`
(`payloadType` `ix://filament/native/Clock`) and `Idle`, and an object type
with fields `s: Slot` and `p: Phase`. Also the `Tier` fixture of TC-995 with
the `Gold` case's `payloadType` replaced by `ix://filament/native/Clock`.

1. Admit the `Tier` fixture and read the `Gold` case.
2. Admit the timed fixture and list its clock places.
3. Check the guard `present(self.p.Armed) and value(self.p.Armed) >= 1 ms`, then
   `value(self.p.Armed) >= 1 ms` alone.
4. Check the postcondition `self.p = Phase::Armed(0 ms)` and the guard
   `self.s.0 >= 2 ms`.
5. Check `p1 = p2` over two `Phase` operands.

## Expected Results

1. Admitted, the `Gold` case carrying a `Clock` payload and no data payload.
2. Admitted, with the places `s.0` (unguarded) and `p.Armed` (guarded by
   `present(self.p.Armed)`).
3. One convex part carrying `present(self.p.Armed)`; then
   `undefined_expression`/`unproved-presence` at the read.
4. A reset of `p.Armed`; a guard on `s.0`.
5. `invalid_timed_model`/`clock-read-outside-constraint`.
