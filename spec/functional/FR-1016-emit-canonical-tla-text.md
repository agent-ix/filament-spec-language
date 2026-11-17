---
id: FR-1016
title: "Emit canonical TLA+ text through the tla2 mapper"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-029
    type: implements
  - target: ix://agent-ix/filament-specification/FR-812 filament-specification/FR-813 filament-specification/FR-814 filament-specification/FR-816 filament-spec-language/FR-756
    type: depends_on
---
# FR-1016: Emit canonical TLA+ text through the tla2 mapper

## Description

FSL's `filament.output.tla2/v1` mapper SHALL emit the TLA+ encodings of FSpec FR-812 and FR-813 in FSpec FR-814's canonical layout and naming, using only FSpec FR-816's syntax subset, and SHALL return the name map with the text.

## Behavior

- The module SHALL be named `M_` plus the mangled package name and end with `====` and one LF.
- Every definition SHALL be preceded by its `\* fsl:` comment.
- Integer division SHALL go through generated `D_` helpers.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-1016-AC-1 | Exporting `Low` writes ASCII text that begins `---- MODULE M_`, holds `VARIABLES`, `Init`, `Next`, `Spec` with `WF_vars(Next)` and one `\* fsl:` comment per definition, and ends with one LF. | Test (TC-2866) |

## Dependencies

- FR-812 filament-specification/FR-813 filament-specification/FR-814 filament-specification/FR-816 filament-spec-language/FR-756 (filament-specification).
