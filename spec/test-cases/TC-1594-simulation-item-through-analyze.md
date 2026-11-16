---
id: TC-1594
title: "A simulation item reports traces, coverage and frontier"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-634
    type: verifies
---
# TC-1594: A simulation item reports traces, coverage and frontier

## Description

Verify FR-634's simulation item. Scope: FR-634-AC-1 to FR-634-AC-3.

## Test Procedure

1. Enumerate the Counter subject.
2. Enumerate with `max_states` 2.
3. Sample twice with seed 5 and `samples` 3.

## Expected Results

- Step 1: `success`, `coverage` 4 states, 3 transitions and 0 choices, exactly one trace from 0 to 3, empty frontier.
- Step 2: `incomplete` `LimitReached{max_states}`, non-empty frontier.
- Step 3: equal documents.
- Assertions compare the outcome document's typed members and the exit code, never message text.
