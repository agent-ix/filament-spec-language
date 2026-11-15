---
id: FR-633
title: "Request schedulability and response-time items through analyze"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-029
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-620
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-246
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-247
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-248
    type: depends_on
---
# FR-633: Request schedulability and response-time items through analyze

## Description

A `native-analyze/1` item naming a checked schedulability or response claim
(FR-246) SHALL run on EN-7, which `Engines::fsl()` registers under `en-7`
(FR-620), and be decided in closed form: fixed-priority response-time
analysis with its busy period, EDF with the processor-demand test and QPA,
and AMC-rtb (FR-247). The verdict settles only after EN-7's evidence is
recomputed independently (FR-248).

## Inputs

### Request item

- `claim`: the schedulability or response claim's name; `kind`
  `schedulability`.
- No subject member: the task set, with its WCETs, blocking terms and
  jitter, is part of the checked claim and of its obligation identity
  (FR-246).
- `options`: none required; EN-7's published bounds apply to its iteration
  counts.

## Outputs

- `record`: `proved` with basis `closed-form{analysis}`, `analysis`
  `fixed-priority`, `edf` or `amc-rtb`, certification `certified` once the
  recomputation agrees; `refuted` with the miss evidence.
- `counterexample`: for a fixed-priority miss, the task, its response time
  and its deadline; for EDF, `Miss(Utilization(U))` or the first deadline at
  which demand exceeds supply (FR-247).
- No `axes`: the item has no activation scope (FR-622).

## Behavior

- **Budgets.** An iteration that reaches EN-7's bound settles `incomplete`
  with the limit. **Cancellation** settles `incomplete` with its cause.
- **Recomputation.** Evidence the independent recomputation does not
  reproduce settles `inconclusive` `replay-parity` (FR-248).
- **Refusals.** A schedulability item settled `Supported` on any engine but
  EN-7 settles as FR-281 rule 4 states.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-633-AC-1 | ADR-026 §12's `Ctl` task set with `schedulable CtlFp on Ctl under fixed_priority;` settles `proved` with basis `closed-form` `fixed-priority`, certification `certified`, on backend `en-7`. | Test (TC-1593) |
| FR-633-AC-2 | A response claim that holds at 12 ms settles `proved`, and the same claim at 11 ms settles `refuted` with the task, response time 12 and deadline 11 in its counterexample. | Test (TC-1593) |
| FR-633-AC-3 | `schedulable CtlEdf on Ctl under edf;` settles `proved` with basis `closed-form` `edf`. | Test (TC-1593) |

## Dependencies

- FR-246 to FR-248 own task sets, EN-7 and recomputation.
