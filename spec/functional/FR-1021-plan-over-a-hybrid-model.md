---
id: FR-1021
title: "Plan over a hybrid model through the negotiated hybrid provider"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-029
    type: implements
  - target: ix://agent-ix/filament-specification/FR-819 filament-spec-language/FR-750 filament-spec-language/FR-1012
    type: depends_on
---
# FR-1021: Plan over a hybrid model through the negotiated hybrid provider

## Description

FSL SHALL run FSpec FR-819's hybrid plan synthesis: S3 SHALL admit a `synthesis` whose `grammar` names a `hybrid` declaration, the run request SHALL require `synthesis.dwell_grid`, and each candidate schedule SHALL be validated as a bounded-reachability item on the negotiated hybrid provider (FR-750, FR-1019).

## Behavior

- Only a `flowpipe-outward-rounded` or `inductive-enclosure` enclosure SHALL validate a candidate; `infeasible` SHALL require an enclosure closure accepted by FR-751's core check.
- No hybrid plan outcome SHALL be `optimal`.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-1021-AC-1 | The `Fill`/`Drain` tank plans `feasible` with dwell 8 into `Drain` under the reference provider; the goal `h >= 12` is `infeasible` with a `certified` closure; with no hybrid provider the outcome is `unsupported` naming `hybrid-dynamics`; without `dwell_grid` the request refuses at `/synthesis/dwell_grid`. | Test (TC-2871) |

## Dependencies

- FR-819 filament-spec-language/FR-750 filament-spec-language/FR-1012 (filament-specification).
