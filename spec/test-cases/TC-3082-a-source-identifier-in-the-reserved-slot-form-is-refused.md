---
id: TC-3082
title: "A source identifier in the reserved slot form is refused"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-488
    type: verifies
---
# TC-3082: A source identifier in the reserved slot form is refused

## Description

Verify that the checker refuses a source identifier in FSpec FR-402's
reserved slot form wherever it would name a `value`/`parameter` node, and
admits look-alike names outside the form. Scope: FR-488-AC-29.

## Test Procedure

1. Check `function f(op_sa: Integer): Integer pure { op_sa }`.
2. Check the same function with its parameter named `op_s_x41`.
3. Check `pre F using v on Config::ConfigVersion::attemptUpdate
   { forall(slot_sx_sy in self.us: slot_sx_sy.x < 5 ms) }` over a
   `us: Sequence<Timer>` field.
   Check `function g(x: Integer): Integer pure { let op_sa = x in op_sa }`, a
   `fold<I>(op_sa, x in xs: op_sa + x, identity: 0)` accumulator and a case
   arm `B(y, rcv_sa)`.
4. Check the function of step 1 with its parameter renamed, in turn, to
   `slot_sv`, `op_sa_sb`, `rcv_s`, `op_sa_x4`, `op_s_xC3`, `op_s_xc3` and
   `slotted`, and emit each package.

Tag the tests `#[trace("TC-3082", "FR-488-AC-29")]`.

## Expected Results

- Steps 1 to 3 and the binder checks: each refused `ambiguous_declaration`/`ambiguous-name` at the
  identifier's own name token (the text of the refused region is exactly the
  identifier), naming `op_sa` with tag `op`, `op_s_x41` with tag `op` and
  `slot_sx_sy` with tag `slot`; the `let`, `fold` and case-arm binders are
  refused the same, at the binder token; no package is
  emitted.
- Step 4: each admitted; the package holds a `value`/`parameter` node with
  that name.
