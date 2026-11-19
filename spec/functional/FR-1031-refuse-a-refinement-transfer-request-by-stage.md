---
id: FR-1031
title: "Refuse a refinement transfer request by stage"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-029
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-1030
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-820
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-100
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-109
    type: references
---
# FR-1031: Refuse a refinement transfer request by stage

## Description

FSL SHALL refuse a malformed or unresolvable `refinement_transfer` request
(FR-1030) before any premise or transfer is settled, with the stage, code,
cause and locus below, in the command-error envelope and with the exit code
FR-100 gives refusals before settlement. Each refusal writes no result
document and settles no item.

## Behavior

| Refusal | Stage | Code / cause | Locus | Exit |
| --- | --- | --- | --- | --- |
| `refinement_transfer` together with any of `call`, `clause`, `items`, `synthesis`, `export` or `export_evidence` | `request` | `invalid-request` | names every selection member present | 20 |
| An unknown, duplicate or missing member of `refinement_transfer`, `abstract` or `concrete`; an empty `clauses` or `initial`; a clause named twice; an `initial` key or premise clause key that `observations` or `clauses` does not hold | `request` | `invalid-request` | the member's RFC 6901 pointer, e.g. `/refinement_transfer/clauses/1` | 20 |
| A `model` alias the unit selects but whose package no `models` entry supplies | `compile` | `missing_import` / `missing-selection` | the selection (FR-109-AC-2) | 20 |
| A `model` alias the unit does not select; a `refinement` naming no refinement declaration; a clause naming no declaration | `select` | `missing_declaration` / `missing-name` | `/refinement_transfer/abstract/model`, `/refinement_transfer/concrete/model`, `/refinement_transfer/refinement` or `/refinement_transfer/clauses/<i>` | 20 |
| An `abstract.model` or `concrete.model` that is not the refinement's declared abstract or concrete subject | `select` | `invalid_runtime_input` / `invalid-value` | that `model` pointer, with `details` `{"declared": D, "given": G}` | 20 |
| A clause outside the transferable family: a state clause, a bounded-profile temporal clause, or a protocol clause | `select` | `ill_typed` / `type-mismatch` | `/refinement_transfer/clauses/<i>`, naming the declaration kind and profile | 20 |
| A clause whose `over` parameter is not typed in the refinement's abstract model (FSpec FR-380 "Subject match") | `select` | `invalid_runtime_input` / `invalid-value` | `/refinement_transfer/clauses/<i>` | 20 |
| An `initial` snapshot FR-106 does not admit for its model | `admit` | FR-106's code and cause | `/refinement_transfer/<side>/initial/<i>` | 20 |
| A supplied premise FSpec FR-820 "Admission" refuses (a slot-key mismatch, another subject, package, initial state or universe) | `admit` | `invalid_runtime_input` / FSpec FR-820's FR-272 cause | `/refinement_transfer/premises/refinement` or `/refinement_transfer/premises/claims/<clause>` | 20 |

- A refinement whose step map is not step-for-step, a concrete subject that
  declares `time dense`, an unimplied fairness constraint, a missing stutter
  closure and an unproved premise are outcomes, never refusals: FSpec FR-820
  settles each `inconclusive` with its named cause (FR-1030).
- FSL SHALL raise the first refusal in table order; within a row, at the
  first member in request order.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-1031-AC-1 | A request carrying `refinement_transfer` and `call`, and one carrying `refinement_transfer` and `items`, each refuse at stage `request`, `invalid-request`, naming both members, exit `20`; a `clauses` list naming `PrevSafe` twice refuses `invalid-request` at `/refinement_transfer/clauses/1`. | Test (TC-2921) |
| FR-1031-AC-2 | `abstract.model` `Ghost` refuses at stage `select`, `missing_declaration`/`missing-name` at `/refinement_transfer/abstract/model`; the unit's `Spec` selection with no `models` entry for its package refuses at stage `compile`, `missing_import`/`missing-selection`; `concrete.model` `Impl` for `ThirdRefinesSpec` refuses at stage `select`, `invalid_runtime_input`/`invalid-value` with `declared` `Third`; each exits `20`. | Test (TC-2921) |
| FR-1031-AC-3 | A clause naming a state invariant, and one naming a bounded-profile temporal clause, refuse at stage `select`, `ill_typed`/`type-mismatch` at `/refinement_transfer/clauses/0`; a temporal clause over `Third::Counter` refuses `invalid_runtime_input`/`invalid-value` at that pointer. | Test (TC-2921) |
| FR-1031-AC-4 | A supplied claim premise whose request item is over another abstract package refuses at stage `admit`, `invalid_runtime_input`/`invalid-value` at `/refinement_transfer/premises/claims/PrevSafe`, settling no transfer; `CasRefinesCounter`, which is not step-for-step, is never refused. | Test (TC-2921) |

## Dependencies

- FR-1030 (the member), FSpec FR-820 (premise admission), FR-100 (refusal
  envelope and exits), FR-106 (snapshot admission), FR-109 (spine stage
  refusals).
