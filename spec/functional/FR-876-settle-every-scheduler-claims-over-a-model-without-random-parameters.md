---
id: FR-876
title: "Settle every-scheduler claims over a model without random parameters"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-022
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-185
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-186
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-741
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-406
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-724
    type: depends_on
---
# FR-876: Settle every-scheduler claims over a model without random parameters

## Description

A `probabilistic` claim `under every scheduler` over a model whose unit
declares no `random` line for an operation SHALL check and compile like any
other claim, and `fsl analyze` SHALL settle it with the exact value FSpec
FR-724 gives: every argument of that operation is a choice of the scheduler,
so each deterministic scheduler yields one behaviour and the claim's minimum
and maximum probability are each 0 or 1. The item reports that verdict,
`refuted` with a counterexample where some scheduler's behaviour violates the
event, and never reports a compile-stage "checked" as the claim's result.

A claim over such a model is not refused or warned about: it is a legitimate
claim about a nondeterministic model, and its verdict tells the modeller what
the model does. Adding the `random` line that gives the argument a
distribution changes the model, and the same claim then settles against the
probability that distribution gives.

## Use case

A modeller writes the `Link` model's two sends without the `random` lines
and claims `probabilistic Every using inf on M over (m: M::Msg) under every scheduler on origin { probability >= rational(9, 10) [ eventually holds(m.delivered) ] }`.
The scheduler may choose `lost = true` on both sends, so the minimum
delivery probability is 0 and the item is refuted, with the behaviour that
loses both sends as its counterexample. With the two `random` lines restored,
the minimum over schedulers is `24/25` (two sends through `send_b`, each lost
with `1/5`) and the item is proved.

## Inputs

- A checked `probabilistic` claim under every scheduler (FR-186) over a model
  some or all of whose operation arguments carry no `random` declaration
  (FR-185), its checked package (FR-740) and an FR-741 item with exact
  evidence.

## Outputs

- One FR-741 item result.

## Behavior

- S3 and S4 SHALL check and emit the claim and its unit whether or not the
  unit declares `random` lines; the absence of a `random` line is no
  diagnostic.
- An operation argument with no `random` declaration SHALL be read as a
  choice of the scheduler, made together with the choice of operation
  (FSpec FR-724).
- Exact evidence SHALL compute the claim's minimum (for `>=`) or maximum
  (for `<=`) over schedulers on that reading. Where no argument of the model
  is random, the value is 0 or 1, and the item SHALL be `proved` exactly when
  the event holds on every behaviour (for `>=`), and otherwise `refuted` with
  a counterexample: a behaviour of a deterministic scheduler that attains the
  minimum, which replays (FR-741).
- Statistical evidence over such a claim SHALL settle as FR-741 states for a
  claim under every scheduler; this requirement adds no statistical rule.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-876-AC-1 | The `Link` unit with its two `random` lines removed and the claim `Every` of the use case compiles with exit 0, with no diagnostic about the missing `random` lines. | Test (TC-2488) |
| FR-876-AC-2 | Through `fsl analyze` with exact evidence, `Every` over that unit settles `refuted`, its value 0, with a counterexample whose replay reproduced it; it is never `proved` and never `unsupported`. | Test (TC-2488) |
| FR-876-AC-3 | `Every` over the `Link` unit with its two `random` lines settles `proved` with exact value `24/25`. | Test (TC-2488) |

## Dependencies

- [FR-185](FR-185-declare-random-parameters-workloads-and-rewards.md) declares
  random parameters.
- [FR-741](FR-741-request-and-report-probability-items-through-analyze.md)
  carries the item.
- FSpec FR-406 "Every scheduler" and FSpec FR-724 give the reading.
