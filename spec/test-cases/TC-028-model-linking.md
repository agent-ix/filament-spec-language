---
id: TC-028
title: "Ambiguous scalar inference"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-104
    type: verifies
  - target: ix://agent-ix/filament-spec-language/FR-766
    type: verifies
---

## Description

Verify that an expression whose literal operands or collection-literal source
supply no unique type is refused `ill_typed`/`ambiguous-literal`, and that the
same shape checks when the context supplies the type. Scope: FR-104-AC-13,
FR-104-AC-16 and FR-766-AC-9. The retired FR-006-AC-4 and FR-016-AC-4 are
carried by these criteria; the refusal's cause is `ambiguous-literal`, located
at the literal or the arithmetic node, not a bare `ill_typed`.

## Test Procedure

1. Check the decimal bodies of FR-104-AC-13 and FR-104-AC-16 whose operands are
   all `decimal(c, s)` literals with no expected type.
2. Check the invariant of FR-766-AC-9 whose `count` source is a bare
   `sequence[...]` literal, under `Linked` and `Kernel` modes.
3. Check each with a typed operand or an annotated source.

## Expected Results

Steps 1 and 2 refuse `ill_typed`/`ambiguous-literal` at the stated locus. Step 3
checks. No refused clause becomes a checked clause.
