---
id: TC-3183
title: "Classify end components for cyclic until and always"
type: TC
relationships:
  - target: "ix://agent-ix/filament-spec-language/FR-201"
    type: verifies
---
# TC-3183: Classify end components for cyclic until and always

## Description

Verify FR-201-AC-61: the checker derives and classifies end components
itself, and a producer document that omits one cannot prove a false claim.

## Test Procedure

(i) `s` satisfies A and not B with actions `wait`, to `s`, and `go`, to a B
state `b`. Over every scheduler check `probability >= 1/2 [holds(A) until
holds(B)]` with targets `{s}`, value 1 and rank 0 at `s`, and with empty
targets and values `s = 0`, `b = 0`. (ii) `ok` satisfies P with actions
`stay`, to `ok`, and `leave`, to `down`; `down` does not satisfy P and
returns to `ok`. Check `probability <= 1/2 [always holds(P)]` with targets
`{ok}`, value 1 and rank 0, and with empty targets and values 0. Check
`probability >= 1/2 [always holds(P)]` with targets `{down}`, values
`ok = 1`, `down = 1`, ranks 1, 0, and with `ok = 0`.

## Expected Results

The checker derives `{s}` under `wait` as rejecting and `{ok}` under `stay`
as accepting. The correct documents reject `ThresholdMissed`. The
empty-target documents refuse objective mismatch. The `>= 1/2` always
document rejects `ThresholdMissed`, and `ok = 0` rejects `InequalityFails`
at `ok`.
