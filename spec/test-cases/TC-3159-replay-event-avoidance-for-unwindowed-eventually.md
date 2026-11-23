---
id: TC-3159
title: "Replay event avoidance for unwindowed eventually"
type: TC
relationships:
  - target: "ix://agent-ix/filament-spec-language/FR-202"
    type: verifies
---
# TC-3159: Replay event avoidance for unwindowed eventually

## Description

Verify FR-202-AC-34: a cyclic witness scheduler that avoids the event can
refute an unwindowed eventual claim through a subsystem certificate.

## Test Procedure

Run the no-random-line eventuality case under every scheduler with claim
`probability >= 1/2 [eventually E]` and value 0. Replay the counterexample.
Repeat after adding an event state to the named bottom component, removing
the scheduler, and setting the certificate lower bound to `1/2`.

## Expected Results

The valid counterexample has a deterministic memoryless scheduler. Replay
derives the event-free bottom component from the rebuilt finite induced chain,
checks a `lower` certificate for `Pr(not eventually E)` above `1/2`, applies
the strict comparison, and settles `refuted`. The altered component, missing
scheduler, or equality at `1/2` yields no token.
