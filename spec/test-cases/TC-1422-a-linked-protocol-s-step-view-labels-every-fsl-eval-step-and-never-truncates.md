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

Scope: FR-578-AC-1 to FR-578-AC-4.

## Test Procedure

1. Build the view of a protocol of sends, receives and a choice; compare
   the steps out of its initial state with the `fsl-eval` system's
   successors; build it twice.
2. Read its final states and assumptions; build the protocol system and
   read its FR-148 assumption set.
3. Build the view of a protocol activated `on each`, of one with a
   replicated role, and of one with a text step binder.
4. Build the view of the first protocol with a state limit one below its
   reachable state count, then at that count.

Tag the tests `#[trace("TC-1422", "FR-578-AC-n")]`.

## Expected Results

- Step 1: each step carries its kind and control node, a channel node and
  record exactly on `send` and `receive`; the initial steps equal the
  successors in order; the two views are equal.
- Step 2: final states are non-empty with no step leaving one; the
  assumptions are equal.
- Step 3: `Unsupported` naming the clause, the role, and
  `UnenumerableRecord`, with no steps.
- Step 4: refused `StateLimit` naming the limit; then a view.
