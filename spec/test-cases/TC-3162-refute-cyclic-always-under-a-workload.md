---
id: TC-3162
title: "Refute cyclic always under a workload"
type: TC
relationships:
  - target: "ix://agent-ix/filament-spec-language/FR-202"
    type: verifies
---
# TC-3162: Refute cyclic always under a workload

## Description

Verify FR-202-AC-36: an unwindowed `always` claim on a cyclic workload chain
is refuted through an event-avoidance subsystem certificate, without a
path-set search.

## Test Procedure

Use a two-state workload chain: `ok` satisfies P and moves to `ok` and to
`down` with probability `1/2` each; `down` does not satisfy P and returns to
`ok`. Analyze `probability >= 0.9 [always holds(P)]` with no window, with
the path-set search instrumented. Replay the counterexample through the
checked-subject subsystem entry. Repeat with the certificate value at `ok`
set to `1/10`, and with `down` changed to satisfy P.

## Expected Results

The exact value is 0. The acyclicity admission finds the cycle and the
path-set search is never called. The counterexample has no scheduler and a
`lower` certificate with `x(ok) = 1`, `x(down) = 1` and ranks 1 and 0.
Replay derives `{down}` as the only rejecting component from its own rebuilt
chain, checks the certificate and `1 > 1/10`, and the item settles
`refuted`, `decisive-counterexample`, retaining the replayed counterexample.
It does not settle `unsupported` or stop `incomplete`. The `1/10`
certificate rejects `ThresholdMissed` at the initial key and settles
`inconclusive`. With `down` satisfying P the value is 1: the claim is not
refuted, EN-5 emits no counterexample and starts no path-set search, and its
proof-side outcome is the one FR-201 gives today, unchanged by FR-202.
