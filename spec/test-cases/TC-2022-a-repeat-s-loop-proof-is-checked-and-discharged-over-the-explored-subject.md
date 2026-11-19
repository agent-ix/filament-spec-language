---
id: TC-2022
title: "A repeat's loop proof is checked and discharged over the explored subject"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-218 FR-730
    type: verifies
---
# TC-2022: A repeat's loop proof is checked and discharged over the explored subject

## Description

Verify the variant's type rule at S3 and the discharge of a repeat's invariant and termination obligations by the protocol model checker: proved over a complete exploration, refuted with a replayable counterexample, incomplete under a limit. Scope: FR-218-AC-43 and FR-730-AC-1 to FR-730-AC-4.

## Test Procedure

| # | Change | Expected |
| --- | --- | --- |
| 1 | `invariant { true } variant { n }` with `n` integer-typed; a tuple of two integer-typed terms | checks |
| 2 | a variant of type `Boolean`; of a record type | `ill_typed`/`type-mismatch` at the variant, retaining `expected` and `actual` |
| 3 | `repeat Retry` with no maximum, `invariant { left >= 0 } variant { left }`, body decreasing `left` from `2` | `loop-invariant` and `loop-termination` both `proved`, exploration complete |
| 4 | row 3 with a body that leaves `left` unchanged | `loop-termination` `refuted`, counterexample ending at the second decision of the first back edge, which replays; `loop-invariant` `proved` |
| 5 | row 3 with invariant `left > 0` | `loop-invariant` `refuted` at the decision where `left` is `0` |
| 6 | row 3 with a state limit below the reachable state count | both items `incomplete`, naming the state limit |
| 7 | two repeats with loop proofs and one with a maximum only, under two requests over one subject | four derived items, none for the bounded repeat, derived once |

## Expected Results

- Each verdict, counterexample end and stopping cause is asserted as a field.
