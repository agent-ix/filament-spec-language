---
id: TC-2500
title: "Every run control and every relationship of a checked protocol is written"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-880
    type: verifies
---
# TC-2500: Every run control and every relationship of a checked protocol is written

## Description

Verify that S4 writes each FSpec FR-466 control node at depth and the clause's relationships set, with no not-yet-implemented refusal. Scope: FR-880-AC-1, FR-880-AC-2.

Catches a writer that handles only `sequence`, drops a nested control, or refuses relationships.

## Test Procedure

1. Compile a protocol whose `run` nests, at depth two or more, `parallel` (each join policy), `choice`, `repeat`, `await`, `attempt`, `send`, `receive`, `check`, `effect`, `event`, `commit` and `fence`; read it back.
2. Compile `relationship Pipe = M::pipe;` over the architecture fixture, then with a second relationship; read both back.

## Expected Results

- Step 1: one control node per control; the package is admitted; no refusal names a keyword.
- Step 2: `relationships` holds `pipe` once, then two entries in ascending node-id order; both are admitted.
