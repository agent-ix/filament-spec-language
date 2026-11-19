---
id: TC-3042
title: "Decimal value types are read from the decimal policy"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-056
    type: verifies
---
# TC-3042: Decimal value types are read from the decimal policy

## Description

Verify that model intake reads a decimal subject's type from the
filament-core-data decimal policy `{precision, scale}` on its resolution walk.
The policy can sit on a `scalar` `decimal` value type, on an alias of
`ix://filament/native/Decimal`, or on the field or parameter itself. The type
is `Decimal[lo, hi; s, s; exact]`, narrowed by every bound on every node of
the walk. Also verify that:

- the typed member checks in clauses;
- snapshot values are admitted by FSpec FR-140 membership;
- every refusal row is reported once at its node, with no declaration built.

Scope: FR-056-AC-66, FR-056-AC-67, FR-056-AC-68, FR-056-AC-69, FR-056-AC-70.

## Test Procedure

1. Admit and assemble the golden package
   `tests/fixtures/decimal-value-types.semantic-ir.json` under a unit
   `model Config = "test/decimal";`.
   - For each member listed in `tests/fixtures/decimal-value-types.expected.json`
     (fields, the parameter `adjust.by` and the result of `adjust`), read its
     checked value type and compare it with the listed `lo`, `hi`, `smin`,
     `smax` and `mode`.
   - Then replace `Half`'s constraint with `max` `"2"`, `exclusiveMin` `"0.1"`,
     `min` `"0.6"` and `min` `"1.5"` in turn, and read `half` and `soc` again.
2. Over the same package, check each of the invariants `self.soc >= 0.20`,
   `self.soc = self.soc`, `self.fee = self.fee` and `self.rate <= 1.0` on
   `Config::Meter`.
3. Admit the golden copy `tests/fixtures/o2-battery-dispatch.semantic-ir.json`
   of the FSpec planning fixture under `model M = "example/plan-battery";` and
   check `invariant SocFloor using v on M::Battery at current { self.soc >= 0.20 }`.
4. Over the step 1 package, admit current snapshots of population `meters`
   whose `Meter` supplies the `soc`, `margin` and `tax` values FR-056-AC-69
   lists, one value at a time. Every other field holds a member of its type.
5. Apply each single defect FR-056-AC-70 lists to the step 1 package and admit
   it. Each added constraint gets an identity of its own. Then apply the
   `unit` and `enumValues` defects together.

Tag the steps as follows:

| Steps | Tag |
| --- | --- |
| 1, first part | `#[trace("TC-3042", "FR-056-AC-66")]` |
| 1, second part | `#[trace("TC-3042", "FR-056-AC-67")]` |
| 2-3 | `#[trace("TC-3042", "FR-056-AC-68")]` |
| 4 | `#[trace("TC-3042", "FR-056-AC-69")]` |
| 5 | `#[trace("TC-3042", "FR-056-AC-70")]` |

## Expected Results

- Step 1:
  - Every type equals the golden entry, including `Meter.soc`
    `Decimal[0, 100; 2, 2; exact]`, `Meter.fee`
    `Decimal[-999999, 999999; 2, 2; exact]`, `Meter.rate`
    `Decimal[1, 10000; 4, 4; exact]`, `Meter.half`
    `Decimal[0, 50; 2, 2; exact]`, `adjust.by`
    `Decimal[-99999, 99999; 2, 2; exact]` and the result of `adjust`
    `Decimal[0, 100; 2, 2; exact]`.
  - With `Half`'s constraint replaced, `half` is `Decimal[0, 100; 2, 2; exact]`,
    `Decimal[11, 100; 2, 2; exact]` and `Decimal[60, 100; 2, 2; exact]`
    respectively. `min` `"1.5"` refuses once
    `invalid_model_binding`/`malformed-declaration`, naming `constraints`, `150`
    and `100`. `soc` is unchanged throughout.
- Step 2: each invariant checks with no refusal.
- Step 3: intake raises no `unsupported_construct` naming `scalar=decimal`, and
  the invariant checks.
- Step 4:
  - The admitted values admit.
  - Each refused value refuses `invalid-value` at its field, naming the
    declared Decimal type and the supplied coefficient and scale.
- Step 5:
  - Each defect gives exactly one refusal with the code and cause FR-056-AC-70
    lists, and intake builds no declaration.
  - Validator rows keep the validator diagnostic.
  - Reader rows (`unit`, `enumValues`, a second `min`, `lo > hi`) carry no
    validator diagnostic and name their item.
  - With `unit` and `enumValues` together, only `unit` is reported.
