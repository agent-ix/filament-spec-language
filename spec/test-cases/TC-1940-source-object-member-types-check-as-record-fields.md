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
with AC-15, and compare each `value_type` with the checked type of the same
form as a record field. Then check the two `Base` integer overrides; the `M`
result override narrowing `Option<Reference<M::Node>>` to
`Option<Reference<M::Leaf>>` and, over a base declaring
`Option<Reference<M::Leaf>>`, the override widening it to
`Option<Reference<M::Node>>`; the result override from `Reference<Order>` to
`Option<Reference<Order>>`; and the parameter override from
`Decimal[0, 100; 2, 2; exact]` to `Decimal[0, 200; 2, 2; exact]`. Last,
check the AC-15 clock unit (`record W { c: Clock; }`, `object O { w: W; }`,
`dimension Time;`, `unit ms : Time = rational(1, 1);`, `time O dense unit
ms;`) through the whole S3 check, with `Sequence<W>[0, 2]` and record-nested
variants of `w`, and compare each clock place with the one the record field
of the same type records.

## Expected Results

Every FR-569-AC-12 pair checks, with equal member types and no
`unknown_required_feature`. Every FR-569-AC-13 object unit is refused at the
type with the record unit's code and cause, and yields no checked package.
Every FR-569-AC-15 member record holds the representation AC-15 states, and
every member's `value_type` equals the checked type of the same form as a
record field. The integer widening override and the narrowing `M` override
check. The integer narrowing override and the `Decimal` override refuse
`ill_typed`/`variance-parameter`. The widening `M` override and the
`Reference<Order>` to `Option<Reference<Order>>` override refuse
`ill_typed`/`variance-result`. The clock units check with no refusal (no
`clock-without-dense-time`), and `w`, its collection and its nested variants
record the clock places their record fields record.
