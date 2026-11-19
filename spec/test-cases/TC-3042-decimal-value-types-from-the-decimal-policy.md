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
filament-core-data decimal policy `{precision, scale}` and the bound
constraints on its resolution walk, giving `Decimal[lo, hi; s, s; exact]`,
whichever node carries the policy (a `scalar` `decimal` value type, an alias
of `ix://filament/native/Decimal`, or the field or parameter itself). Verify
that the typed member checks in clauses, that snapshot values are admitted
by FSpec FR-140 membership, and that every refusal row is reported once at
its node. Scope: FR-056-AC-65, FR-056-AC-66, FR-056-AC-67, FR-056-AC-68.

## Test Procedure

1. Admit and assemble the golden package
   `tests/fixtures/decimal-value-types.semantic-ir.json` under a unit
   `model Config = "test/decimal";`. For each member listed in
   `tests/fixtures/decimal-value-types.expected.json`, read its checked value
   type and compare it with the listed `lo`, `hi`, `smin`, `smax` and `mode`.
2. Over the same package, check each of the invariants `self.soc >= 0.20`,
   `self.soc = self.soc`, `self.fee = self.fee` and `self.rate <= 1` on
   `Config::Meter`.
3. Admit the golden copy `tests/fixtures/o2-battery-dispatch.semantic-ir.json`
   of the FSpec planning fixture under `model M = "example/plan-battery";` and
   check `invariant SocFloor using p on M::Battery at current { self.soc >= 0.20 }`.
4. Over the step 1 package, admit snapshots whose `Meter` supplies the `soc`,
   `margin` and `tax` values AC-67 lists, one value at a time.
5. Apply each single defect AC-68 lists to the step 1 package and admit it;
   then apply the `unit` and `enumValues` defects together.

Tag steps 1, 2-3, 4 and 5 `#[trace("TC-3042", "FR-056-AC-65")]`,
`#[trace("TC-3042", "FR-056-AC-66")]`, `#[trace("TC-3042", "FR-056-AC-67")]`
and `#[trace("TC-3042", "FR-056-AC-68")]` respectively.

## Expected Results

- Step 1: every member's type equals the golden entry, including
  `Meter.soc` `Decimal[0, 100; 2, 2; exact]`, `Meter.fee`
  `Decimal[-999999, 999999; 2, 2; exact]` and `Meter.rate`
  `Decimal[1, 10000; 4, 4; exact]`. No member is left without a checked type.
- Step 2: each invariant checks with no refusal.
- Step 3: intake raises no `unsupported_construct` naming `scalar=decimal`, and
  the invariant checks.
- Step 4: the admitted values admit; each refused value refuses
  `invalid_runtime_input`/`invalid-value` at `fields.<name>`, naming the
  declared Decimal type and the supplied coefficient and scale.
- Step 5: each defect gives exactly one refusal with the code, cause and
  named item AC-68 lists, and no declaration; with `unit` and `enumValues`
  together only `unit` is reported.
