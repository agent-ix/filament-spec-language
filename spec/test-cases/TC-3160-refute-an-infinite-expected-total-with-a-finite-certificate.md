---
id: TC-3160
title: "Refute an infinite expected total with a finite certificate"
type: TC
relationships:
  - target: "ix://agent-ix/filament-spec-language/FR-202"
    type: verifies
  - target: "ix://agent-ix/filament-spec-language/FR-203"
    type: verifies
---
# TC-3160: Refute an infinite expected total with a finite certificate

## Description

Verify FR-202-AC-35 and FR-203-AC-15: positive infinity on the failing side
of a finite expected-reward bound is refuted with replayable finite evidence,
without encoding infinity as an exact result value.

## Test Procedure

Run an expected-total claim `<= 1` whose solved exact expectation is
`+infinity`. Inspect the result and replay its lower expected-reward
certificate. Repeat with a lower certificate whose checked initial bound is
exactly 1 and with an engine-only infinity result.

## Expected Results

The valid finite lower certificate checks with initial bound greater than 1;
replay returns matching evidence and settlement is `refuted` with that
counterexample. The result contains no infinite `ExactEntry` and no
fabricated finite exact value. The equality case and engine-only value do not
settle `refuted`; proof-side checks retain their original inclusive rule.
