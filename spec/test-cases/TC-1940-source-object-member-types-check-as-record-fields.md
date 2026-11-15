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
same code and cause. Scope: FR-569-AC-12 and FR-569-AC-13.

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

## Expected Results

Every FR-569-AC-12 pair checks, with equal member types and no
`unknown_required_feature`. Every FR-569-AC-13 object unit is refused at the
type with the record unit's code and cause, and yields no checked package.
