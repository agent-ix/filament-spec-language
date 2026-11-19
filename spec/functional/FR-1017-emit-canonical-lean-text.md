---
id: FR-1017
title: "Emit canonical Lean 4 text through the lean4 mapper"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-029
    type: implements
  - target: ix://agent-ix/filament-specification/FR-812 filament-specification/FR-813 filament-specification/FR-814 filament-specification/FR-816 filament-spec-language/FR-756
    type: depends_on
---
# FR-1017: Emit canonical Lean 4 text through the lean4 mapper

## Description

FSL's `filament.output.lean4/v1` mapper SHALL emit the Lean encodings of FSpec FR-812 and FR-813 in FSpec FR-814's canonical layout and naming, stating every obligation as a `def Ob_<clause> : Prop` and writing no `theorem`, no proof and no `sorry`.

## Behavior

- The file SHALL open `namespace` with the module name and use only FSpec FR-816's Lean subset.
- Sets, bags and maps SHALL be sorted canonical lists.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-1017-AC-1 | Exporting `Low` to Lean writes ASCII text with `structure State`, `def Init`, `def Step`, `def Ob_` and `-- fsl:` comments, and with no `theorem` and no `sorry`. | Test (TC-2867) |

## Dependencies

- FR-812 filament-specification/FR-813 filament-specification/FR-814 filament-specification/FR-816 filament-spec-language/FR-756 (filament-specification).
