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
- When the run request is admitted, FSL SHALL refuse, before any search and with no outcome document: a `declaration` naming no `synthesis`, or one whose grammar is not a state model object type, `missing_declaration`/`missing-name` at `/synthesis/declaration`; a policy name naming no `temporal` declaration, `missing_declaration`/`missing-name` at `/synthesis/policy/<i>`; a policy whose `over` binding is not an object of the grammar's model, `ill_typed`/`type-mismatch` at `/synthesis/policy/<i>`; and an initial snapshot that does not admit for the model, with that snapshot's own admission refusal at `/synthesis/initial` (FSpec FR-806 "Request-time refusals", pointers prefixed as FR-1010 states).

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-1013-AC-1 | A plan `domain` `Int[-1, 3]` refuses `ill_typed`/`type-mismatch` at S3; a goal reading `Reach.elapsed` over the untimed spine model refuses `missing_declaration`/`missing-name` at `elapsed`; each exits `20`. | Test (TC-2863) |
| FR-1013-AC-2 | Over the spine model, a `synthesis` member whose `declaration` names an ordinary `synthesis` over the value type `Digit` refuses `missing_declaration`/`missing-name` at `/synthesis/declaration` (FSpec FR-806-AC-3); a `policy` `["Nowhere"]` refuses `missing_declaration`/`missing-name` at `/synthesis/policy/0`; a `policy` naming a temporal clause over the `Braking` hybrid declaration refuses `ill_typed`/`type-mismatch` at `/synthesis/policy/0` (FSpec FR-806-AC-3); and, with the unit declaring the population bound `bound HistoryBound on Config::config_history [1]`, the healthy spine snapshot, whose `config_history` holds the two objects `root` and `child`, violates that bound and refuses with the snapshot's own admission refusal located at `/synthesis/initial` (FSpec FR-806-AC-4); each exits `20` with no `synthesis` outcome. | Test (TC-2863) |

## Dependencies

- FR-806 filament-specification/FR-807 (filament-specification).
