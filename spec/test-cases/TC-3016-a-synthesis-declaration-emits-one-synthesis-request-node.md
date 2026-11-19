---
id: TC-3016
title: "A synthesis declaration emits one synthesis-request node"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-1036
    type: verifies
---
# TC-3016: A synthesis declaration emits one synthesis-request node

## Description

Verify the package writer's `claim`/`synthesis_request` node and
`claim_names` row for a value synthesis and a plan synthesis, and that the
package reader reads them back. Scope: FR-1036-AC-1 and FR-1036-AC-2.

## Test Procedure

1. Compile `Pick` over `Digit`, with and without `minimize Pick`, and read
   its node and `claim_names` row.
2. Compile the plan synthesis `Reach` over the spine model and read its
   node back through the package reader.

## Expected Results

- Step 1: one node, path `["Pick"]`, the six bindings in order, `grammar` and
  `domain` the `Digit` node, a `Pick` candidate, `symmetry` `identity`, one
  row named `Pick`; with the objective, `minimize` and the text `Pick`.
- Step 2: `grammar` the `ConfigVersion` object type node, `domain` the
  `Lengths` range node, the candidate typed `ConfigVersion`; the package is
  admitted.
