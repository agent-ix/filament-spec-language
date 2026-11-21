---
id: TC-3000
title: "Hyper clauses sharing a body wrapper emit their own trace rows"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-488
    type: verifies
  - target: ix://agent-ix/filament-specification/FR-322
    type: verifies
---
# TC-3000: Hyper clauses sharing a body wrapper emit their own trace rows

## Description

Verify that two hyper clauses with equal bodies and reversed quantifier prefixes share one body wrapper and emit their per-trace clock rows on their own `hyperproperty` nodes, each in its own quantifier order. Scope: FR-488-AC-26, FR-488-AC-30.

## Test Procedure

1. Compile, under event-position false-extension over one model `V`, `Fwd` over `forall trace a of V forall trace b of V` and `Rev` over `forall trace b of V forall trace a of V` with equal bodies.
2. Read the two `hyperproperty` nodes, their `body` targets and `temporal_bindings`.
3. Read the wrapper's slot nodes, their levels and `over`; compile `Rev` alone and `Fwd` alone and compare the wrapper `node_id`s; compile a clause whose body reads only `v @ b`, and FR-488-AC-16's `Linked`, and read their slot nodes and the `let` binder level.

Tag the tests `#[trace("TC-3000", "FR-488-AC-26", "FR-488-AC-30", "FSpec-TC-2027", "FSpec-FR-322-AC-463")]`.

## Expected Results

- Step 1: exit 0.
- Step 2: both nodes reference one wrapper; rows `a`, `b` at `Fwd`'s node and `b`, `a` at `Rev`'s node; no row names the wrapper.
- Step 3: `slot_sv_sa` at level 0 (`over`) and `slot_sv_sb` at level 1 in both clauses and one wrapper `node_id`; the `v @ b` clause has no `slot_sv_sa` node and `slot_sv_sb` at level 0; `Linked`'s `r` at level 1; every package admitted.
