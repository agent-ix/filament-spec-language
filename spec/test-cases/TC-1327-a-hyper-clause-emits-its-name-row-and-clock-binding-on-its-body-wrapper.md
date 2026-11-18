---
id: TC-1327
title: "A hyper clause emits its name row on its body wrapper and its clock rows on its hyperproperty node"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-488
    type: verifies
  - target: ix://agent-ix/filament-specification/FR-322
    type: verifies
---
# TC-1327: A hyper clause emits its name row on its body wrapper and its clock rows on its hyperproperty node

## Description

Verify that a hyper clause's name row is keyed by the body wrapper at the assembled `claim` occurrence ordinal, and its clock rows, one per quantified trace, by the `hyperproperty` node at its `claim` occurrence.

Scope: FR-488-AC-24 and FR-488-AC-25.

## Test Procedure

1. Compile `NonInterference` and read `temporal_clause_names` and `temporal_bindings`.
2. Compile two structurally equal hyper clauses with different names, then rename one.
3. Compile under event-position false-extension; compile under infinite-trace.

Tag the tests `#[trace("TC-1327", "FR-488-AC-24", "FR-488-AC-25", "FR-322-AC-343", "FR-322-AC-344", "FR-322-AC-361")]`.

## Expected Results

- Step 1: one name row at the wrapper's NodeId and the assembled occurrence's ordinal with the name `NonInterference`; the `hyperproperty` node has none.
- Step 2: one wrapper, two rows at distinct ordinals; renaming changes no NodeId.
- Step 3: two binding rows at the `hyperproperty` node's NodeId and `claim` occurrence, `trace` `a` then `b`, each with `clock_name` `model-steps` and the configuration derived from `V`; no binding row names the wrapper; under infinite-trace no binding.
