---
id: TC-028
title: "Ambiguous scalar inference"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-104
    type: verifies
  - target: ix://agent-ix/filament-spec-language/FR-006
    type: verifies
---

## Description

Verify that a clause whose literal operands or `size` count supply no unique
type is refused, and that an explicit typed context makes the same shapes
check. Scope: FR-104-AC-19. The behavior of the retired FR-006-AC-4 and
FR-016-AC-4 is carried by FR-104-AC-19; the test is
`tc_028_literal_and_size_inference_require_an_explicit_nominal_context`
(`tests/it/native_checking.rs`).

## Test Procedure

Over the rule model, check the invariants `1 + 2 = 3` and
`size(self.items) = 2`, then `size(self.items) = self.count` and
`let x = 2 in x = self.n`.

## Expected Results

The first two refuse `ill_typed`. The last two check, the `size` count and the
bound `x` taking their type from the typed field they are compared with. No
refused clause becomes a checked clause.
