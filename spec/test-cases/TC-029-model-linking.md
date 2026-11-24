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
is well typed. Scope: FR-006-AC-5 and FR-104-AC-19; the test is
`tc_029_root_typing_includes_the_real_boolean_operation_result`
(`tests/it/native_checking.rs`).

## Test Procedure

Over the rule model, check the invariant `self.n`, the invariant
`self.n = self.n`, and the postcondition `result` of a Boolean-result
operation.

## Expected Results

`self.n` refuses `ill_typed`/`non-boolean-root` at the whole expression. The two
controls check. No rejected root becomes a reference-evaluable clause.
