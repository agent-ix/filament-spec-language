---
id: TC-1596
title: "Protocol analysis items and supplied finite protocol traces"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-636
    type: verifies
---
# TC-1596: Protocol analysis items and supplied finite protocol traces

## Description

Verify FR-636's protocol items and the acceptance of supplied traces. Scope: FR-636-AC-1 to FR-636-AC-4.

## Test Procedure

1. Analyze the order fixture's protocol deadlock item, then the fixture without the `receive`.
2. Analyze `local-projection` onto `Seller` and `realizability`, and compare with step 1.
3. Analyze `finite-replay` over the trace `Placed`, `Got`, `Ship`, and over a trace beginning with `Got`.
4. Analyze a parallel join that overflows a capacity-1 channel, and with `max_messages` 1.

## Expected Results

- Step 1: `proved`; `refuted` ending in the unreceived message, with the separating witness record, replayed.
- Step 2: `proved` for both, agreeing with step 1.
- Step 3: `proved`; `declined` `invalid-input` naming step 0.
- Step 4: the overflow keyed as FR-572 states; `incomplete` naming `max_messages`.
- Assertions compare the outcome document's typed members and the exit code, never message text.
