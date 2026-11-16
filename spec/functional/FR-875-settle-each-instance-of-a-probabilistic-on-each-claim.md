---
id: FR-875
title: "Settle each instance of a probabilistic on-each claim through analyze"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-022
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-186
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-125
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-127
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-741
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-407
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-331
    type: depends_on
---
# FR-875: Settle each instance of a probabilistic on-each claim through analyze

## Description

When an analyze item (FR-741) names a `probabilistic` claim whose activation
is `on each (x: T)`, optionally with `when (g)`, FSL SHALL form one instance
per key of `T`'s population in the start snapshot, in product with any `over`
parameter, settle each instance under the claim's scheduler and evidence kind,
and settle the item from the instances: `inconclusive` with cause
`no-instance` when there is no instance, `refuted` naming the first failing
instance in key order when one fails, and `proved` or `measured` otherwise.
An `on each` claim settles from source exactly as an `on origin` claim does;
it is never `unsupported` for its activation.

## Use case

A modeller states that every message that is still undelivered is delivered
eventually with probability at least 9/10:
`probabilistic E using inf on M under Even on each (k: M::Msg) when (not k.delivered) { probability >= rational(9, 10) [ eventually holds(k.delivered) ] }`.
Over a start snapshot with one undelivered message the item is proved; over a
snapshot with no message the item is inconclusive rather than vacuously
proved; with a delivered message beside an undelivered one, the delivered
message never activates and does not count against the claim.

## Inputs

- A checked `probabilistic` claim with activation `Each{parameter type,
  guard}` (FR-186 "Activation"), its checked package (FR-740) and the
  analyze request's start snapshots (FR-620).

## Outputs

- One FR-741 item result. A `refuted` item's counterexample names the failing
  instance's key.

## Behavior

### Instances

- FSL SHALL form the instances of an `on each (x: T)` claim from the keys of
  every population of the start snapshot that admits `T` (FR-125
  "Instancing"), in key order, in product with the `over` parameter's
  instances when the claim has one.
- A start snapshot whose populations admitting `T` hold no object gives no
  instance. FSL SHALL settle the item `inconclusive` with cause
  `{"type": "no-instance"}` (FR-127, FSpec FR-331), before any state is
  explored, and SHALL NOT settle it `proved`: a claim about every member of
  an empty population carries no evidence about the model.

### Activation and conditioning

- Each instance SHALL be read from its run's first activating position, the
  first position at which `g` holds for that instance's object, or position 0
  when the claim has no guard, and SHALL be conditioned on the run activating
  (FSpec FR-407 "Activation"). A run that never activates is not counted.
- An instance whose activation has probability 0 under the claim's scheduler
  SHALL hold vacuously: no activated run exists to violate it. Such an
  instance never makes the item `refuted` and never makes it `inconclusive`.

### Settling the item

- Each instance SHALL be settled with the claim's evidence kind (FR-741): an
  exact instance by its exact value against the bound, a statistical instance
  by its measurement.
- The item SHALL be `refuted` when any instance fails, and its counterexample
  SHALL name the first failing instance in key order, so a rerun names the
  same instance.
- Otherwise the item SHALL be `proved` under exact evidence, or `measured`
  under statistical evidence, carrying the instances' evidence in FR-741's
  result members.
- The item SHALL NOT be settled `unsupported` for its activation.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-875-AC-1 | Through `fsl analyze` over the `Link` model, the claim `E` of the use case with a start snapshot whose `msgs` population holds no object settles `inconclusive` with cause `no-instance`, never `proved` and never `unsupported`. | Test (TC-2485) |
| FR-875-AC-2 | With a start snapshot holding `m` undelivered and `n` already delivered, `E` settles `proved`: `n` never activates and holds vacuously. The same claim at `rational(99, 100)` settles `refuted`, its counterexample naming `m` and not `n`. | Test (TC-2486) |
| FR-875-AC-3 | With a start snapshot holding `a` and `b`, both undelivered, `E` at `rational(99, 100)` settles `refuted` with a counterexample naming `a`, the first failing instance in key order; two runs give equal items. | Test (TC-2487) |

## Dependencies

- [FR-186](FR-186-check-probabilistic-claim-forms-at-s3.md) checks and records
  the `on each` activation this requirement settles.
- [FR-125](FR-125-read-a-model-subject-s-behaviours-as-temporal-traces.md)
  defines instancing and `NoInstance`.
- [FR-741](FR-741-request-and-report-probability-items-through-analyze.md)
  carries the item request and result members.
- FSpec FR-407 "Activation" gives the meaning; FSpec FR-331 the result causes.
