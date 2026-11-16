---
id: TC-1940
title: "Source object member types check as record fields"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-569
    type: verifies
---
# TC-1940: Source object member types check as record fields

## Description

Verify that a field, parameter and result of a source object type admit every
type a source record field admits, with the record field's checked type, and
that a type a record field refuses is refused for an object member with the
same code and cause, and that the assembled source model view holds each
member type in the representation FR-569 "The source model" gives. Scope:
FR-569-AC-12, FR-569-AC-13 and FR-569-AC-15.

Catches a member type refused `unknown_required_feature` that a record field
admits, and an object member that checks where the record field is refused.

## Test Procedure

For each member type of FR-569-AC-12, assemble and check, on the source
route, a unit holding the declarations the type needs, `record R { f: T; }`
and `object O { f: T; }`, and compare the checked type of `self.f` in
`invariant I using P on O at current { true }` with the record field's type.
For the enum and record cases also check `operation g(p: T): T;` on `O`.
For FR-569-AC-13, assemble the pair with `T` = `Population<O>[3]` and with
`T` the name of a source function, and compare the refusals.
For FR-569-AC-15, assemble the AC-15 unit, read each member record of the
retained source model view and compare its `value_type` and `Multiplicity`
with AC-15. Then check the two `Base` overrides, the `M` override with an
optional reference result, and the clock place of `w`, and compare the clock
place with the one a record field of type `W` records.

## Expected Results

Every FR-569-AC-12 pair checks, with equal member types and no
`unknown_required_feature`. Every FR-569-AC-13 object unit is refused at the
type with the record unit's code and cause, and yields no checked package.
Every FR-569-AC-15 member record holds the representation AC-15 states, and
no member holds a temporary type. The widening override and the `M`
override check, the narrowing override refuses
`ill_typed`/`variance-parameter`, and `w` records the clock place the record
field records.
