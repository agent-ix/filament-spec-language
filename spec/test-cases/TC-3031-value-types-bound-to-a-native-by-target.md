---
id: TC-3031
title: "Value types bound to a native by target are that native"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-056
    type: verifies
  - target: ix://agent-ix/filament-spec-language/FR-230
    type: verifies
---
# TC-3031: Value types bound to a native by target are that native

## Description

Verify that a domain-package value type whose `target` names a native value
type is that native under its own name: a data native gives its own type, a
bare `Text`, `Rational` or `Decimal` gives no checked type and refuses where
used, and `Clock` makes the value type an alias of the clock type with clock
constraints. Scope: FR-056-AC-63, FR-056-AC-64.

## Test Procedure

1. Build package `test/bound` with value types `Flag`, `Count`, `Name`,
   `Reading` and `Sample` (`target` Boolean, Integer, String, Float64,
   Float32) and object type `Holder` with one field typed by each. Admit and
   assemble it, and check `self.count = 1` and `self.flag = true`.
2. Add value types `Memo`, `Ratio`, `Amount` (`target` Text, Rational,
   Decimal) with fields `memo`, `ratio`, `amount`; assemble it, then check the
   clause `self.memo = self.memo`.
3. Build a timed package with value type `Watch` (`target`
   `ix://filament/native/Clock`, constraint `max` `5` applying to `Watch`) and
   object type `Call` with field `w` and optional field `v` typed `Watch`;
   check it under `Dense`.
4. Repeat step 3 with `Watch`'s constraint keyword `min`, then `maxLength`,
   then with an option value type whose `inner` is `Watch`.

Tag steps 1-2 `#[trace("TC-3031", "FR-056-AC-63")]` and steps 3-4
`#[trace("TC-3031", "FR-056-AC-64")]`.

## Expected Results

- Step 1: no `UnsupportedModelMember` refusal; the fields are `Boolean`,
  `Integer`, `String`, `Float64` (binary64, `exact`) and `Float32` (binary32,
  `exact`), and both clauses check.
- Step 2: assembly succeeds; the clause refuses `ill_typed`/`operator-ineligible`
  at the `field` projection, naming `Holder` and `Field.typeRef`.
- Step 3: the checked graph has clock places `Call.w` and `Call.v` (`v`
  guarded by its presence), each with the time invariant `x <= 5`.
- Step 4: `min` refuses `invalid_timed_model`/`time-invariant-shape`;
  `maxLength` refuses `invalid_model_binding`/`malformed-declaration` at the
  constraint; the option refuses `invalid_model_binding`/`malformed-declaration`
  at `TypeDefinition.inner`.
