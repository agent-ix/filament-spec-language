---
id: TC-2816
title: "Source selection keys are spelled from the source owner"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-995
    type: verifies
---
# TC-2816: Source selection keys are spelled from the source owner

## Description

Verify that every emitted source declaration key uses the declaring unit's SourceOwner identity as package and ix://{identity}/{type} as node. Scope: FR-995-AC-1, FR-995-AC-2.

Catches a key spelled from the checked package's name, a unit position or a bare type name.

## Test Procedure

1. Emit fixture S and read the dispatch table of `Sized`'s `self.size()` call.
2. Emit the unit as the second of two object-declaring units of one package.

## Expected Results

- Step 1: candidates `Order` then `Sub` with keys `{null, {I, ix://I/Order}}` and `{null, {I, ix://I/Sub}}`, `I` the SourceOwner identity, equal to the effects rows' `document`.
- Step 2: no key holds the package name; each unit's keys carry its own identity.
