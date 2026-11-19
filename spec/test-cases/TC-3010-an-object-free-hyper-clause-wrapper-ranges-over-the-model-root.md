---
id: TC-3010
title: "An object-free hyper clause's wrapper ranges over the model root"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-488
    type: verifies
---
# TC-3010: An object-free hyper clause's wrapper ranges over the model root

## Description

Verify that a hyper clause with no atom slot emits a body wrapper whose
`over` argument references the `model`/`model_root` node of its first
subject, that the strict reader admits it, and that a model-root `over` is
refused on a wrapper whose body reads a slot and on a root no subject names.
Scope: FR-488-AC-28, FSpec FR-402-AC-11.

## Test Procedure

1. Emit `hyper Whole using inf over behaviours () forall trace a of V forall
   trace b of V { always holds(true) }` over the vault model and read it
   strictly.
2. Emit `NonInterference` over the vault model.
3. Edit step 2's wrapper `over` to the vault package's `model_root` node and
   read the package.
4. Edit step 1's wrapper `over` to the `model_root` node of a second
   selected package that no `subjects` binding names and read the package.

Tag every step `#[trace("TC-3010", "FR-488-AC-28")]` and
`#[trace("FSpec-TC-2034", "FSpec-FR-402-AC-11")]`.

## Expected Results

- Step 1: the wrapper's `over` targets the vault `model_root` node, which is
  among the wrapper's `dependencies`; the package is admitted.
- Step 2: the wrapper's `over` targets the slot node `slot_sv_sa`.
- Steps 3 and 4: `invalid_model_binding`/`malformed-declaration` at
  `/semantic_graph/nodes/{w}/body/arguments/0`.
