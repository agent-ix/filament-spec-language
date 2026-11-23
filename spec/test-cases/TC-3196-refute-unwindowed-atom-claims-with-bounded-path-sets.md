---
id: TC-3196
title: "Refute unwindowed atom claims with bounded path sets"
type: TC
relationships:
  - target: "ix://agent-ix/filament-spec-language/FR-202"
    type: verifies
---
# TC-3196: Refute unwindowed atom claims with bounded path sets

## Description

Verify FR-202-AC-40: the bounded checked-subject path-set entry replays an
atom claim `holds(P)` exactly as `eventually[0,0] holds(P)`.

## Test Procedure

Fixtures: the engines `Coin` model with `data/coin-tails.json` (one coin,
initially tails), and a fixture the test writes: the same model with its one
coin initially heads. Claims use the `ev` profile under every scheduler on
origin.

1. Over the tails fixture, replay for `probability >= 0.5 [ holds(c.heads) ]`
   a counterexample whose `initial` and `initial_key` are the tails state and
   its rejecting decided key, with one empty path and an empty scheduler.
2. Rebuild the step 1 counterexample's `item` and complete keys for
   `probability <= 0.5 [ holds(c.heads) ]`, keeping its empty path, and replay
   it for that claim.
3. Over the tails fixture, replay a counterexample whose `initial` is the
   heads state and whose `initial_key` is its accepting decided key.
4. Over the heads fixture, replay for `probability <= 0.5 [ holds(c.heads) ]`
   a counterexample with one empty path.
5. Repeat steps 1 to 4 with the claims spelled `eventually[0,0]
   holds(c.heads)`, and replay step 1 twice.

Tag the tests `#[trace("TC-3196", "FR-202-AC-40")]`.

## Expected Results

- Step 1: `refuted` with `R = 1 > 1 - 1/2` and a reproduced token.
- Step 2: `ProbabilityWitnessReplay::Parity`, no token.
- Step 3: refused, no token.
- Step 4: `refuted` with `A = 1 > 1/2` and a reproduced token.
- Step 5: each outcome, token, refusal and parity equals its atom
  counterpart; the repeated replay gives equal semantic outcomes.
