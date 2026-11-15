---
id: TC-1585
title: "An explicit-state model-check item reports verdict, counts and counterexample"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-625
    type: verifies
---
# TC-1585: An explicit-state model-check item reports verdict, counts and counterexample

## Description

Verify FR-625's request item and result. Scope: FR-625-AC-1 to FR-625-AC-5.

## Test Procedure

1. Analyze `Bounded` and count the Counter subject's reachable states and steps with an independent breadth-first search.
2. Analyze `always holds(c.value <= 2)`.
3. Analyze `Bounded` with `max_states` 2, then with `max_depth` 1.
4. Analyze invariant clauses `self.value <= 3` and `self.value <= 2`.
5. Analyze over the Counter domain with an unbounded `setTo(n: Integer)`.

## Expected Results

- Step 1: `proved`, `exhaustive`, `certified`, counts equal to the oracle's 4 and 3.
- Step 2: `refuted`, four-state prefix ending at 3, reproduced.
- Step 3: `incomplete` `LimitReached{max_states}` with 2 states; `LimitReached{max_depth}`.
- Step 4: `proved`; `refuted` ending at 3.
- Step 5: `unsupported` `unbounded-extent`.
- Assertions compare the outcome document's typed members and the exit code, never message text.
