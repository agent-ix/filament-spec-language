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
   Inspect the `constructs` inventory, `Order` fields' `typeRef`s and their
   generated scalar definitions. For `id`, resolve the exact reference
   `ix://example/shop/OrderId`; for `total`, resolve
   `ix://example/shop/OrderTotal`. Check each definition's scalar, bounds
   and origin against the corresponding inline type in the input source.
2. Compile the AC-2 model unit; admit it; check and evaluate `reaches(a, b,
   next)` over a snapshot of `nodes` with a two-node chain.
3. Compile the AC-3 model unit; admit it; list the meanings of its nodes.

## Expected Results

Step 1: admitted with the members FR-710-AC-1 names, the invariant checks and
the bytes are equal. The inventory has exactly three used kinds:
`object_type`, `value_type` and `population`, with the existing meanings in
FR-710's compilation table. Both referenced generated definitions have
`scalar` `integer` and the used `value_type` kind. `OrderId` has `min` and
`max` constraints with exact operand values `"0"` and `"999"`; `OrderTotal`
has `"0"` and `"100"`. Each field names its own exact definition, rather
than the unbounded native Integer or the other field's bounded type. The
definitions and their bound constraints retain the input model's authored
source identity and the respective inline type's source origin (`Int`
through its closing `]`); field origins and multiplicities remain as AC-1
states. The generated definitions are not extra authored declarations.
Step 2: the relationship is declared and `reaches` holds
from the first node to the second. Step 3: admitted, with one node of each
meaning of FR-710's compilation table.
