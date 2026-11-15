---
id: TC-1942
title: "Compile a model unit to an admitted domain package"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-710
    type: verifies
---
# TC-1942: Compile a model unit to an admitted domain package

## Description

Verify that a model unit compiles to a domain package document that FSpec
FR-154 admits, covering the smallest model, relationships and every
declaration form. Scope: FR-710-AC-1 to FR-710-AC-3.

## Test Procedure

1. Compile the AC-1 model unit; admit the document by FR-154; check the AC-1
   invariant in a unit selecting it; compile it again and compare bytes.
2. Compile the AC-2 model unit; admit it; check and evaluate `reaches(a, b,
   next)` over a snapshot of `nodes` with a two-node chain.
3. Compile the AC-3 model unit; admit it; list the meanings of its nodes.

## Expected Results

Step 1: admitted with the members FR-710-AC-1 names, the invariant checks and
the bytes are equal. Step 2: the relationship is declared and `reaches` holds
from the first node to the second. Step 3: admitted, with one node of each
meaning of FR-710's compilation table.
