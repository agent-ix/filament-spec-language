---
id: TC-3190
title: "Check every-scheduler long-run proofs"
type: TC
relationships:
  - target: "ix://agent-ix/filament-spec-language/FR-201"
    type: verifies
---
# TC-3190: Check every-scheduler long-run proofs

## Description

Verify FR-201-AC-70 to FR-201-AC-74: the closed MDP long-run document
profile certifies per-MEC optimal fractions and their reachability
combination, re-derives the MECs itself, and admits the event-position
profile.

## Test Procedure

Build `Avail2` and `Loop` as the ACs describe. Check the stated minimum and
maximum documents and their thresholds with the independent checker, then
the bogus, wrong-gain and wrong-partition variants. Repeat the `Loop` claims
under the event-position false-extension profile, and run the unsupported,
cancellation and limit cases.

## Expected Results

Each valid document accepts at the stated bounds and rejects
`ThresholdMissed` beyond them. Bogus cycle fixed points reject `RankBroken`;
a wrong component gain rejects `InequalityFails` in the component check; an
omitted or altered MEC rejects `InequalityFails` at the smallest differing
key, even when every offered equation holds. The event-position claims
settle the same values as the infinite-trace claims.
