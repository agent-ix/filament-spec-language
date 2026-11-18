---
id: TC-3005
title: "Rules and equality over float model fields follow the IEEE profile"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-056
    type: verifies
  - target: ix://agent-ix/filament-spec-language/FR-397
    type: verifies
---
# TC-3005: Rules and equality over float model fields follow the IEEE profile

## Description

Verify that a rule over a float model field checks and evaluates under the
IEEE profile with the field's strict `exact` rounding mode, that `=`, `!=` and
the ordering operators follow FSpec FR-148 and FR-149 for NaN and signed zero,
and that a float field against an exact type, another width or another
rounding mode stays refused. Scope: FR-056-AC-57 to FR-056-AC-59.

## Test Procedure

Fixture: TC-3004's `ConfigVersion` package with the `ConfigVersion` fields
`reading`, `limit`, `a` and `b` typed `ix://filament/native/Float64`, `f32`
typed `ix://filament/native/Float32` and `count` typed
`ix://filament/native/Integer`, beside its `versionNumber` (`Int[0, 1000]`).
Each clause is the body of
`invariant I using v on Config::ConfigVersion at current { ... }`.

1. Check `self.reading <= self.limit`, `self.reading + self.limit = self.limit`
   and `convert<Float64[nearest-even]>(self.reading) + convert<Float64[nearest-even]>(self.limit) <= convert<Float64[nearest-even]>(self.limit)`,
   and read the law and rounding mode of each IEEE node.
2. Evaluate the two sums of step 1 over `reading` `3ff0000000000000` and
   `limit` `3ca0000000000000`.
3. Check `self.a = self.b`, `self.a != self.b`, `self.a < self.b`,
   `self.a <= self.b` and `self.a = float64(bits: 0x0000000000000000)`.
4. Evaluate `=`, `!=`, `<` and `<=` over `a`, `b` both `7ff8000000000000`, then
   over `a` `0000000000000000` and `b` `8000000000000000`; build a
   `Set<Float64>` of +0 and -0, and the state keys of two states that differ
   only in `a` being +0 or -0.
5. Check `self.a = 0`, `self.a = self.versionNumber`, `self.a < self.count`,
   `self.a = self.f32`, `self.a + 1 = self.a` and
   `self.a = convert<Float64[nearest-even]>(self.a)`.

Tag steps 1–2 `#[trace("TC-3005", "FR-056-AC-57")]`, steps 3–4
`FR-056-AC-58` and step 5 `FR-056-AC-59`.

## Expected Results

- Step 1 checks. Each IEEE node carries the law `ieee_profile` with definition
  `filament.value.ieee754-2019-default/v1`; the bare `+` has rounding mode
  `exact` and the converted `+` `nearest-even`.
- Step 2: the bare `+` refuses `ieee_not_exact`/`rounding-required` with the
  would-be flag `inexact` and no bits; the converted `+` gives
  `3ff0000000000000` with `inexact`.
- Step 3 checks, each clause typed `Boolean`.
- Step 4: over the NaNs `=` is false, `!=` true and `<=` false; over +0 and -0
  `=` is true, `<` false and `<=` true. The set has two members, and the two
  state keys differ.
- Step 5: each clause refuses `ill_typed`/`type-mismatch` at its operator,
  naming both operand types, and the unit yields no checked graph.
