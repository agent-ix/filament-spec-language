---
id: TC-3195
title: "Prove unwindowed atom claims as eventually at horizon zero"
type: TC
relationships:
  - target: "ix://agent-ix/filament-spec-language/FR-201"
    type: verifies
---
# TC-3195: Prove unwindowed atom claims as eventually at horizon zero

## Description

Verify FR-201-AC-57 and FR-201-AC-75: an origin-activated event-position atom
claim `holds(P)` is admitted, certified and settled as `eventually[0,0]
holds(P)`, through both qualified admission points, and the reading does not
reach the other consumers of the shared bounded-shape classification.

## Test Procedure

Fixtures: the engines `Coin` model with `data/coin-tails.json` (one coin,
initially tails), and a fixture the test writes: the same model with its one
coin initially heads. Claims use the `ev` profile under every scheduler on
origin.

1. Over the tails fixture, analyze `probability <= 0.5 [ holds(c.heads) ]`
   and `probability <= 0.5 [ eventually[0,0] holds(c.heads) ]`; check each
   emitted `reachability/v1` document with `fsl-probability-check` in a
   separate process.
2. Over the heads fixture, analyze `probability >= 0.5 [ holds(c.heads) ]`
   and its `eventually[0,0]` spelling, and check each document.
3. Replace the initial value of each document by `1/2` and check again.
4. For a checked root `holds(c.heads)`, call
   `QualifiedBoundedMonitor::supports` and
   `QualifiedBoundedMonitor::supports_observations` under the event-position
   false-extension profile and under the infinite-trace profile; start the
   monitor on the tails and heads initial observations and compare its key
   and verdict with the `eventually[0,0] holds(c.heads)` monitor.
5. For a temporal clause whose formula is a root `holds(c.heads)`, call
   `bounded_shape` and `closure_shape`, and call `BoundedTranslation::clause`
   with that clause as an origin clause under the infinite-trace profile.

Tag the tests `#[trace("TC-3195", "FR-201-AC-n")]`.

## Expected Results

- Step 1: position 0 decides rejection; `ExactValue{0}`; empty `targets`;
  initial value 0 and rank 0; the document is accepted with side `AtMost`;
  the item settles `proved`. Both spellings give the same verdict,
  optimization, target and decided structure, values, ranks and settlement;
  no new profile string or document member appears.
- Step 2: position 0 decides acceptance; `targets` is the initial key;
  `ExactValue{1}` with rank 0; accepted with side `AtLeast`; `proved`; the
  same results for both spellings.
- Step 3: rejected `InequalityFails` at the initial key for each spelling.
- Step 4: both predicates true under the event-position profile and false
  under the infinite-trace profile; the initial key and decided verdict equal
  the explicit spelling's apart from the checked formula context.
- Step 5: `bounded_shape` and `closure_shape` return no bounded shape, and
  `BoundedTranslation::clause` refuses it `NotTranslated`, as before the
  reading.
