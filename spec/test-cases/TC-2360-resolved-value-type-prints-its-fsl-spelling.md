---
id: TC-2360
title: "A Resolved value type prints its FSL spelling through the source spelling registry"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-088
    type: verifies
---
# TC-2360: A Resolved value type prints its FSL spelling through the source spelling registry

## Description

Verify that a source model keeps a `SourceSpellings` registry, that
`fsl_spelling` prints each `Resolved` member type in its FSL spelling with
aliases kept, that `ValueTypeRef`'s `Display` prints only the `resolved:`
identity form, that refusal messages use the formatter, and that emitted
`resolved_path` rows never do. Scope: FR-088-AC-13 and FR-088-AC-14.

Catches a formatter that prints a node key, a `VariantId` or `Debug` text in
place of a name, an alias lost to its target, a registry that takes part
in identity or emission, and a `resolved_path` row spelled by the formatter.

## Test Procedure

1. Assemble and check, on the source route, the FR-088-AC-13 unit.
2. For each member slot of `Order` (`a`, `b`, `c`, `d`, `e`, `g`, `h`, the
   parameter `p` and the result of `f`), call `fsl_spelling` with the
   source model's registry and the slot, and compare the text with
   FR-088-AC-13. Call it for `a` with no slot and compare.
3. Walk every `Resolved` type in the source model view and confirm the
   registry has an entry for each `NodeKey`, enum owner key, `EffectiveId`
   and declared unit it holds.
4. Format `Display` of each member type twice, in two separate runs. Confirm
   each starts with `resolved:`, the two runs agree, and the text holds no
   `Debug` text such as `Composite(` or `VariantId(`.
5. In a test, replace the registry entry of `Point`'s key with `Spot`, and
   compare the spelling of `b`, the records, the `ValueType`s and the
   conformance verdicts with step 1. Waits on QC-30 (target emission): emit
   the checked package with each registry and compare the bytes.
6. Check the `Rush` override of FR-088-AC-13 and read the refusal message.
7. Check a unit holding `record W { c: Clock; }`, `object O { w: W; }` and
   `time O dense unit ms;` over a source `Time` dimension with unit `ms`,
   and also `record R { w: W; }` with the member `r: R` on `O`. Read the
   `member_type` of the in-process `PlaceRow` of the step `field(w)` of the
   place `w.c` and of the record field step `field(w)` of the place `r.w.c`.
   Waits on QC-30 (target emission): read the emitted package back through
   intake.

## Expected Results

- Step 2: every spelling equals FR-088-AC-13's, `Money` kept at every
  depth, and `a` without a slot prints `Decimal[0, 100; 2, 2; exact]`.
- Step 3: no key is missing.
- Step 4: only the identity form, the same in both runs.
- Step 5: only the spelling of `b` changes, to `Spot`; once emission lands,
  the emitted package bytes are equal.
- Step 6: refused `ill_typed`/`variance-parameter`; the message names
  `Money` and `Decimal[0, 200; 2, 2; exact]` and holds no `resolved:` text.
- Step 7: the row's `member_type` is the FR-322 identity rendering of
  the type node that `W`'s binding names, equal to the record field's
  row, with no `resolved:` or `Debug` text; it is the next row's
  `declaring_type`; once emission lands, intake joins every row.
- Step 8: `resolved:Population[3]`, `resolved:Population` (each equal to
  `Display`) and `Option<resolved:Population[3]>`, naming no member type;
  no member type holds a `Population` node.
