---
id: TC-2390
title: "Compared and element presences are written as presence reads"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-230
    type: verifies
---
# TC-2390: Compared and element presences are written as presence reads

## Description

Compared and element presences are written as presence reads.

Scope: FR-230-AC-66.

## Test Procedure

Emit the post written with `!= absent` and with `present`, and the guard, invariant and urgency that read element presences. Test: `fsl-package/src/emit/tests/timed_clauses.rs` `compared_and_element_presences_are_written`.

Tag the tests `#[trace("TC-2390", "FR-230-AC-66")]`.

## Expected Results

- The two posts write equal `presence_read` bodies; each element-presence member emits a `presence_read`.
