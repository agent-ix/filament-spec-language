---
id: FR-1013
title: "Refuse plan declarations at S3 and plan requests at admission"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-029
    type: implements
  - target: ix://agent-ix/filament-specification/FR-806 filament-specification/FR-807
    type: depends_on
---
# FR-1013: Refuse plan declarations at S3 and plan requests at admission

## Description

FSL SHALL raise FSpec FR-806's check-time refusals at S3, located at the source name, and its request-time refusals when the run request is admitted, located at the request member's pointer, and SHALL build the plan space only from the checked-package nodes FSpec FR-807 lists.

## Behavior

- S3 SHALL refuse an unbounded or negative `domain`, an unbounded operation parameter, a non-Boolean goal, a non-numeric objective and a read of `elapsed` or `probability` over a model without that member.
- Admission SHALL refuse a policy naming no temporal clause, a policy over another model and an inadmissible initial snapshot.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-1013-AC-1 | A plan `domain` `Int[-1, 3]` refuses `ill_typed`/`type-mismatch` at S3; a goal reading `Reach.elapsed` over the untimed spine model refuses `missing_declaration`/`missing-name` at `elapsed`; each exits `20`. | Test (TC-2863) |

## Dependencies

- FR-806 filament-specification/FR-807 (filament-specification).
