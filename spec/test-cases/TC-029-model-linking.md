---
id: TC-029
title: "Clause roots are Boolean"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-006
    type: verifies
  - target: ix://agent-ix/filament-spec-language/FR-104
    type: verifies
---

## Description

Verify that a clause whose root is not Boolean is refused although every part
is well typed. Scope: FR-006-AC-5 and FR-104-AC-3.

## Test Procedure

Over the rule model, check the invariant `self.versionNumber` (a well-typed
field read), then a Boolean comparison of the same field with itself.

## Expected Results

The field read refuses `ill_typed`/`non-boolean-root` at the whole body. The
comparison checks. No rejected root becomes a reference-evaluable clause.
