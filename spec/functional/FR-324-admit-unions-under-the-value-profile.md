---
id: FR-324
title: "Admit unions and case under the value profile that admits records and tuples"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-032
    type: implements
  - target: ix://agent-ix/filament-spec-language/ADR-012
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-110
    type: traces_to
  - target: ix://agent-ix/filament-spec-language/FR-316
    type: traces_to
  - target: ix://agent-ix/filament-specification/AD-015
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-143
    type: depends_on
---
# FR-324: Admit unions and case under the value profile that admits records and tuples

## Description

When a declaration's `using` alias selects a value profile that admits
records and tuples (`filament.value.complete/v1`), the compiler SHALL admit
union declarations, union constructions and `case` in that declaration on
the same basis, under exactly the profile it
uses for records and tuples, as FSpec FR-143 "Value profile" defines (FSpec
AD-015; ADR-012 §16.9 "Profiles").

## Inputs

- A complete-V1 unit's header profile selections (FR-110) and its
  declarations.

## Outputs

- Checked declarations that use unions.

## Behavior

- The compiler SHALL admit a union, a union construction or a `case` in any
  declaration where it admits a record or tuple under the same profile
  selection.
- A unit that uses unions SHALL resolve exactly the profile it would resolve
  with each union replaced by a record (FR-110).

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-324-AC-1 | **RETIRED (the comparison of two units' emitted selection rows is withdrawn).** No replacement. | Test (TC-834), retired |

## Dependencies

- FSpec FR-143 ("Value profile"), AD-015, AD-005.
- FR-110 (header profile selections), FR-316.

## References

- Owning ticket: QSL-383. Design: ADR-012 §16.9, §16.10.
- Value profile naming `union` (SC-G7): FSpec FR-143 "Value profile"
  (specification ticket STD-115).
