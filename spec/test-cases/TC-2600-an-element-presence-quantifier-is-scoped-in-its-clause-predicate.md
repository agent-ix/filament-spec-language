---
id: TC-2600
title: "An element-presence quantifier is scoped in its clause predicate"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-930
    type: verifies
---
# TC-2600: An element-presence quantifier is scoped in its clause predicate

## Description

Verify that a quantifier reading only element presences of a clock collection
is written inside the timed clause predicate with its own binder, beside an
unquantified clock constraint, and that its binder is never lifted into the
part's or invariant's `binders`. Scope: FR-930-AC-1 to FR-930-AC-4.

## Test Procedure

1. Over `x: Clock` and `q: Sequence<Option<Clock>>[0, 2]`, emit the guard `self.x >= 1 ms and forall(e in self.q: present(e))`.
2. Emit the time invariant `when { forall(e in self.q: present(e)) } { self.x <= 9 ms }` and the urgency `urgent HasElement when { forall(e in self.q: present(e)) };`.
3. Emit the guard `self.x >= 1 ms and exists(e in self.q: present(e))` and the guard `forall(e in self.q: present(e) implies e >= 1 ms)`.
4. Add a `binders` binding naming the quantifier parameter to the step 1 guard part and read the package with I2.

Test: `fsl-package/src/emit/tests/timed_clauses.rs` `an_element_presence_quantifier_is_scoped_in_its_predicate`.

## Expected Results

- Step 1: one guard, one part, no `binders`; the predicate is the `forall` application whose body is the `presence_read` at its binder; the constraint is `x ge 1/1` with no slot; I2 admits it.
- Step 2: the invariant has no `binders` and its predicate is the same application; the urgency predicate is that application; I2 admits both.
- Step 3: one part whose predicate is the `exists` application; the shared-binder guard's parts carry the binder in `binders`.
- Step 4: `invalid_package`/`timed-node-reference` at the binder target.
