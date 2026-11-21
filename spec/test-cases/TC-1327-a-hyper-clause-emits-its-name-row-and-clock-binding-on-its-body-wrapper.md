---
id: TC-1327
title: "A hyper clause emits its name row and its clock rows on its hyperproperty node"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-488
    type: verifies
  - target: ix://agent-ix/filament-specification/FR-322
    type: verifies
---
# TC-1327: A hyper clause emits its name row and its clock rows on its hyperproperty node

## Description

Verify that a hyper clause's `claim_names` row is keyed by its `hyperproperty` node at its `claim` occurrence, that its body wrapper has no `temporal_clause_names` row, and that its clock rows, one per quantified trace, by the `hyperproperty` node at its `claim` occurrence.

Scope: FR-488-AC-24, FR-488-AC-25 and FR-488-AC-31.

## Test Procedure

1. Compile `NonInterference` and read `claim_names`, `temporal_clause_names` and `temporal_bindings`. Read the package with the strict reader, then add a `temporal_clause_names` row naming the wrapper and read it again.
2. Compile two structurally equal hyper clauses with different names, then rename one.
3. Compile under event-position false-extension; compile under infinite-trace.
4. Compile `NonInterference` in unit `example` over another unit's vault model and read every `claim_names` and `temporal_bindings` row's `source_unit`; compile FSpec FR-322-AC-511's `Fast` with `t` bound to a timestamped-event `ms` clock and admit it.

Tag the tests `#[trace("TC-1327", "FR-488-AC-24", "FR-488-AC-25", "FR-322-AC-343", "FR-322-AC-344", "FR-322-AC-361", "FSpec-FR-322-AC-466")]`.

## Expected Results

- Step 1: one `claim_names` row at the `hyperproperty` node's NodeId and its `claim` occurrence, with name and `qualified_name` `NonInterference` and the whole declaration as `declaration_region`; no `temporal_clause_names` row names the wrapper; the package is admitted, and with the wrapper row added it refuses `invalid_package`/`invalid-value` at `/temporal_clause_names/{i}/declaration`.
- Step 2: one `hyperproperty` node, two rows at distinct ordinals; renaming changes no NodeId.
- Step 3: two binding rows at the `hyperproperty` node's NodeId and `claim` occurrence, `trace` `a` then `b`, each with `clock_name` `model-steps` and the configuration derived from `V`; no binding row names the wrapper; under infinite-trace no binding.
- Step 4: only the hyper clause's `claim_names` row carries `source_unit`, naming `example`; `Fast`'s package is admitted with `ms` resolved in `example`.
