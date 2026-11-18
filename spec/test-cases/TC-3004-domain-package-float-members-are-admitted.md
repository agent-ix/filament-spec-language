---
id: TC-3004
title: "Domain-package Float32 and Float64 members are admitted as strict exact floats"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-056
    type: verifies
---
# TC-3004: Domain-package Float32 and Float64 members are admitted as strict exact floats

## Description

Verify that a selected domain package whose field, operation parameter,
operation result, tuple position or variant case payload is typed
`ix://filament/native/Float32` or `ix://filament/native/Float64` is admitted
and assembles, and that each such member has `ValueType::Float` of its width
with the strict `exact` rounding mode (FSpec FR-322 "Model-owned members").
Scope: FR-056-AC-20, FR-056-AC-53 to FR-056-AC-56.

## Test Procedure

Fixture: the `example/config-version` package (object type `ConfigVersion`),
with an object type `Gauge` added for steps 1 and 2, assembled by a complete-V1 unit that selects it as
`Config` and declares no float form of its own.

1. Give `Gauge` a `[1, 1]` field `reading` typed
   `ix://filament/native/Float64`, and a second fixture where it has `[0, 3]`
   multiplicity. Assemble both.
2. Type `reading` `ix://filament/native/Float32` and assemble.
3. Add the operation `calibrate` to `ConfigVersion`, with parameter `offset` and a
   result each typed `ix://filament/native/Float64`, and assemble.
4. Add the tuple type `Pair` (position 0 typed `ix://filament/native/Float64`,
   position 1 `ix://filament/native/Integer`) and the `ConfigVersion` field `pair`
   typed `Pair`, and assemble.
5. Add the variant type `Sample` with cases `Read` (`payloadType`
   `ix://filament/native/Float32`) and `Missing` (no payload) and the `ConfigVersion`
   field `sample` typed `Sample`, and assemble.

Tag step 1 `#[trace("TC-3004", "FR-056-AC-20")]`, step 2 `FR-056-AC-53`,
step 3 `FR-056-AC-54`, step 4 `FR-056-AC-55` and step 5 `FR-056-AC-56`.

## Expected Results

- Step 1 assembles. `reading` is `ValueType::Float` of width binary64 with
  rounding mode `exact`, and the `[0, 3]` field is `Set<Float64>[0, 3]` over
  the same float type. The unit admits the IEEE profile.
- Step 2 assembles, and `reading` is `ValueType::Float` of width binary32 with
  rounding mode `exact`.
- Step 3 assembles, and `offset` and the result are each `ValueType::Float` of
  width binary64 with rounding mode `exact`.
- Step 4 assembles, and `pair`'s position 0 is `ValueType::Float` of width
  binary64 with rounding mode `exact`.
- Step 5 assembles, `Read`'s payload is `ValueType::Float` of width binary32
  with rounding mode `exact`, and `Missing` has no payload.
- No step refuses `unknown_required_feature`/`unsupported-feature`.
