---
id: FR-751
title: "Check hybrid provider certificates in the core in exact interval arithmetic"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-026
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-750
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-450
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-652
    type: depends_on
---
# FR-751: Check hybrid provider certificates in the core in exact interval arithmetic

## Description

`fsl-replay` SHALL implement FSpec FR-652's check of the `inductive-enclosure`,
`flowpipe` and `validated-witness` certificates a hybrid provider returns,
in exact rational interval arithmetic over the `CheckedHybridModel`, so an
accepted certificate settles a hybrid proof `certified` and lets a trace
through a non-polynomial flow refute.

## Inputs

The checked hybrid model and claim, the provider's result entry and its
certificate, and the item's work limit.

## Outputs

`Accepted` or `Rejected { rule, at }`, and the settlement FSpec FR-652
"Settlement" gives.

## Behavior

- The checker SHALL evaluate every flow, guard, reset, invariant and claim
  atom by FR-450's interval evaluator, with no floating-point step.
- It SHALL apply FSpec FR-652's rules in their listed order and report the
  first failing rule and the RFC 6901 pointer of the box or step it failed
  at.
- It SHALL charge each box evaluation to the item's work meter; a check the
  meter stops is `Rejected { rule: work-limit }`.
- An accepted certificate SHALL set the proof's certification to
  `Certified`; a rejected one SHALL keep FR-250's certification and record
  `certificate-rejected`.
- The checker SHALL accept a `validated-witness` only when it has a decisive box (FSpec
  FR-652 "Readings and the decisive box"): a reading the interval evaluator
  returns as unknown, as over a box that straddles an atom's boundary,
  never refutes and is never resolved by a midpoint, endpoint or sample.
- An accepted `validated-witness` SHALL let FR-450 record the trace as the
  item's `HybridCounterexample` and settle `refuted` for a trace exact replay
  cannot decide.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-751-AC-1 | FSpec FR-652-AC-1's two `inductive-enclosure` certificates over `Braking` without `t` are accepted, settling `Certified`, and rejected at rule (a) with the `Moving` box pointer, keeping `Uncertified`. | Test (TC-2101) |
| FR-751-AC-2 | A `flowpipe` step violating the Picard condition is rejected at that step's pointer; FSpec FR-652-AC-3's non-polynomial trace with an accepted witness settles `refuted` and is recorded; with either of its witnesses whose boxes only straddle `v = 30` it stays `inconclusive`, the witness `Rejected { rule: witness-violation }` at its last step. | Test (TC-2101) |
| FR-751-AC-3 | A check given a work limit of one box evaluation is `Rejected { rule: work-limit }`, never `Accepted`. | Test (TC-2101) |

## Dependencies

- FR-450's interval evaluation and replay; FR-750's dispatch; FSpec FR-652.
