---
id: TC-639
title: "Closed probabilistic timed automata are decided exactly through digital clocks, with the workload resolving delays"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-204
    type: verifies
  - target: ix://agent-ix/filament-spec-language/FR-201
    type: verifies
  - target: ix://agent-ix/filament-spec-language/FR-202
    type: verifies
  - target: ix://agent-ix/filament-spec-language/FR-1042
    type: verifies
---
# TC-639: Closed probabilistic timed automata are decided exactly through digital clocks, with the workload resolving delays

## Description

Verify the digital MDP for §15.5, its deadline and expected-time results, the workload-resolved DTMC with discrete delays, and every unsupported cause of the route.

Scope: FR-204-AC-1 to FR-204-AC-4 and FR-204-AC-21 to FR-204-AC-25; FR-201-AC-81 to FR-201-AC-84; FR-1042-AC-11.

## Test Procedure

Fixtures: §15.5's `Retx` with `Deadline` and `MeanTime`; its strict, `discrete`-delay, `exponential`-delay and zero-delay-cycle variants; an `always[0 ms, 4 ms]` claim.

1. Decide `Deadline` at 0.99 and 0.995 over every scheduler; replay the refutation.
2. Decide `MeanTime`; decide both with the strict guard.
3. Decide the deadline claim on the `discrete`-delay variant under `One` and over every scheduler; on the `exponential` variant under `One`.
4. Decide the zero-delay-cycle variant and the `always[0 ms, 4 ms]` claim.
5. On FR-196's no-operation `x >= 1 ms`, cap-2 oracle, check the geometric
   `IdleDelay` 3/4, `IdleObserve` 1/4 `MeanTime` lower-certificate document,
   then remove its ranking and break one required rank decrease. Submit the
   `expected elapsed until holds(B) <= 3 ms` claim with and without the
   accepted finite lower certificate, and replay the accepted witness. Tag
   the certificate checks with `FR-201-AC-81` and settlement with
   `FR-204-AC-22`.
6. On timed `Retx`, submit `expected elapsed until holds(false) >= 3 ms` under
   workload `One` and under every scheduler. For the no-operation `MeanTime`
   oracle, force partial exploration after the finite lower document is
   available, emit the `ValueBounds` partial result with `upper: "infinity"`,
   and replay its lower document against `expected elapsed until holds(B) <=
   3 ms`.

Tag tests for the first four steps with the FR-204 acceptance criterion they
exercise. Tag step 5's certificate checks with `FR-201-AC-81` and its
settlement checks with `FR-204-AC-22` and `FR-204-AC-23`. Tag step 6 with
`FR-204-AC-24`, `FR-204-AC-25`, and `FR-1042-AC-11`.

## Expected Results

- Step 1: clock ranges 0 to 2 and 0 to 5; `ExactValue{99/100}`; `refuted` with probability `1/100`, replay checking the 4 ms horizon.
- Step 2: `ExactValue{20/9 ms}`; with the strict guard, the region-MDP maximum `20/9 ms` and infimum `10/9 ms`, never `unsupported`.
- Step 3: `ExactValue{159129/160000}`; under every scheduler the distribution applied, minimum and maximum `159129/160000`; with `exponential` and exact evidence `DelayDistribution` naming `send`.
- Step 4: the zero-delay variant decided with minimum and maximum `1`; the `always` window decided as a complement with minimum `1/10000` and maximum `1/100`, settling `refuted`.
- FR-204-AC-21's bounded-window forms: the values and verdicts that criterion states, each derived by hand in FR-204 "Worked example: pointwise values"; predicates are read at positions only, never at a delay move.
- Step 5: the lower document's initial value is 5 ms and its properness ranking is accepted; missing ranking is `Malformed` at `ranking`, and broken progress is `RankBroken` at the first failing product key. The oracle's maximum elapsed value is `+infinity`; the `<= 3 ms` item settles `refuted` only with the replayed finite lower certificate, carries the witness, and carries no infinite `ExactValue`. Without that certificate the infinite engine value alone does not settle `refuted`.
- Step 6: the lower claims settle `proved` for `One` and every scheduler without an infinite `ExactValue`; partial exploration emits `ValueBounds` with `exploration: Partial`, a finite lower certificate, and `upper: "infinity"`; replay settles the `<= 3 ms` item `refuted` only after accepting that finite lower certificate.
