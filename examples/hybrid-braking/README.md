# Hybrid braking example

The hybrid braking example is a `Braking` model with two claims, one held and one violated, that the reference hybrid provider `fsl-hybrid-reference` decides with no external solver configured (FR-1019). A vehicle in `Moving` slows at a constant rate `a` between 1 and 2 from a speed `v` between 0 and 20. When `v` reaches 0 it jumps to `Stopped`, where it stays. All files are FSL-owned and AGPL-3.0-only. A reader copies nothing: it passes these paths.

| File | Role |
| --- | --- |
| `braking.native` | the FSL source (edition `1-draft`): `Braking`, `SlowEnough` (`always [0, 10] holds(h.v < 30)`) and `TooSlow` (`always [0, 10] holds(h.v > 5)`) |
| `slow-enough.analyze.json` | a `native-analyze/1` request (FR-620) with the one item `SlowEnough` |
| `too-slow.analyze.json` | a `native-analyze/1` request with the one item `TooSlow` |

Each request names its source relative to itself and names no backend. Negotiation therefore routes its hybrid item to the one registered hybrid provider, the reference provider.

```sh
fsl analyze examples/hybrid-braking/slow-enough.analyze.json
fsl analyze examples/hybrid-braking/too-slow.analyze.json
```

`SlowEnough` settles `proved`, `bounded-solver`, method `flowpipe-outward-rounded`, `certified`. `TooSlow` settles `refuted`. Its counterexample is the start state `(Moving, v = 0, a = 1)` at time 0, which exact replay reproduces.

The test is `fsl-analyze/tests/it/hybrid_reference.rs` (`the_hybrid_braking_examples_reach_verdicts`). It reads both documents through the command adapter's analyze entry.
