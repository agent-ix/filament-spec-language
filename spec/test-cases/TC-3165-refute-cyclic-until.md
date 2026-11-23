---
id: TC-3165
title: "Refute cyclic until"
type: TC
relationships:
  - target: "ix://agent-ix/filament-spec-language/FR-202"
    type: verifies
---
# TC-3165: Refute cyclic until

## Description

Verify FR-202-AC-39: unwindowed until over a cyclic workload chain is
refuted through the event-avoidance subsystem certificate on either bound
side.

## Test Procedure

First chain: two states satisfying A and not B alternate deterministically.
Analyze `probability >= 1/2 [holds(A) until holds(B)]`. Second chain: `s`
satisfies A and not B and moves to `s` with `1/2`, to a B-and-not-A state
`b` with `1/4` and to a neither-A-nor-B state `n` with `1/4`; `b` and `n`
each return to `s`. Analyze the same until at `>= 3/5`, `<= 2/5`, `>= 1/2`
and `<= 1/2`. Replay each counterexample.

## Expected Results

The first chain has value 0. Its component has no decided state and is
rejecting; the certificate with value 1 and rank 0 at both states gives
`1 > 1/2` and the item settles `refuted`. The second chain has value `1/2`.
`>= 3/5` is refuted with the rejecting union `{n}` and certificate
`x(s) = 1/2`, `x(n) = 1`, ranks `s` 1 and `n` 0; `<= 2/5` is refuted with
the accepting union `{b}` and certificate `x(s) = 1/2`, `x(b) = 1`, ranks
`s` 1 and `b` 0. `b` and `n` absorb despite their return to `s`. At
`>= 1/2` and `<= 1/2` the same certificates sit at equality, reject
`ThresholdMissed` at the initial key and yield no token.
