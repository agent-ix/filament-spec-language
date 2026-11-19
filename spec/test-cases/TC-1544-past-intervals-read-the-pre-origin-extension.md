---
id: TC-1544
title: "Past intervals read the pre-origin extension"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-327
    type: verifies
---
# TC-1544: Past intervals read the pre-origin extension

## Description

Verify the pre-origin reading of closed past intervals and the origin-bounded reading of unbounded past operators.

Scope: FR-327-AC-14.

## Test Procedure

Fixture: the `Counter` trace with value 1 at positions 0, 1, 2, closed under event-position and as an infinite-trace prefix.

1. Evaluate `once[1,1] holds(c.value = 1)` and `once[1,1] not holds(c.value = 1)` on origin, and `historically[0,2] holds(c.value = 1)` and `historically[0,2] true` at position 1.
2. Under infinite-trace evaluate `historically holds(c.value = 1)` and `once[2,*] not holds(c.value = 1)` at position 1.

Tag the tests `#[trace("TC-1544", "FR-327-AC-14")]`.

## Expected Results

- Step 1: false, true, false, true; no atom read before position 0.
- Step 2: true, false.
