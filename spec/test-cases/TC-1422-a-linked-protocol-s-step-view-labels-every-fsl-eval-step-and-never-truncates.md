---
id: TC-1422
title: "A linked protocol's step view labels every fsl-eval step and never truncates"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-578
    type: verifies
---
# TC-1422: A linked protocol's step view labels every fsl-eval step and never truncates

## Description

Verify the step view's labels, final states, assumptions, unsupported
constructs and state limit.

Scope: FR-578-AC-1 to FR-578-AC-4, FR-578-AC-7.

## Test Procedure

1. Build the view of a protocol of sends, receives and a choice; compare
   the steps out of its initial state with the `fsl-eval` system's
   successors; build it twice. Build the view of a `parallel` protocol.
2. Read its final states and assumptions; build the protocol system and
   read its FR-148 assumption set.
3. Build the view of a protocol activated `on each`, of one with a
   replicated role, and of one with a text step binder.
4. Build the view of the first protocol with a state limit one below its
   reachable state count, then at that count.
5. Build the view of a protocol whose send constraint divides by zero at
   the initial state, of a `tso` protocol with five stores in one thread,
   and of a protocol with two sends of one name in two choice cases.

Tag the tests `#[trace("TC-1422", "FR-578-AC-n")]`.

## Expected Results

- Step 1: each step carries its kind, a node exactly when its transition
  names a control, a channel node exactly on `send` and `receive`, and its
  own binder record; the fork carries no record; the initial steps equal
  the successors in order; the two views are equal.
- Step 2: final states are non-empty with no step leaving one; the
  assumptions are equal.
- Step 3: `Unsupported` naming the clause, the role, and
  `UnenumerableRecord`, with no steps.
- Step 4: refused `StateLimit` naming the limit; then a view.
- Step 5: refused `Undetermined` at the initial state, `Cut`, and
  `UnnamedNode` naming the shared name's path.
