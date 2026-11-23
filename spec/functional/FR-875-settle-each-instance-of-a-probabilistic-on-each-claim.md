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
  - target: ix://agent-ix/filament-spec-language/FR-196
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-201
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
parameter, and settle each instance under the claim's scheduler and evidence
kind when its analysis profile admits it. FSL SHALL settle the item from the
instances: `inconclusive` with cause
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
  `{"kind": "no-instance"}` (FR-127; the outcome document spells a cause under `kind`, FR-291-AC-1, while FSpec FR-331's bare `type` is the provider-entry spelling), before any state is
  explored, and SHALL NOT settle it `proved`: a claim about every member of
  an empty population carries no evidence about the model.
- For a nonempty instance set, FSL SHALL check the analysis profile before
  evaluating any instance's activation, value or evidence. If the form is
  outside the admitted profile, FSL SHALL settle the item by the unsupported
  profile boundary below and SHALL NOT require per-instance evidence. The
  empty-instance `no-instance` result above takes precedence when the set is
  empty.

### Activation and conditioning

- The following activation and conditioning rules apply only after the
  analysis profile admits the request.
- Each instance SHALL be read from its run's first activating position, the
  first position at which `g` holds for that instance's object, or position 0
  when the claim has no guard, and SHALL be conditioned on the run activating
  (FSpec FR-407 "Activation"). A run that never activates is not counted.
- Under an admitted profile, an instance SHALL hold vacuously when no first
  activating state is reachable (FR-196 "Conditioned readings"): under a workload its activation
  has probability 0, and under every scheduler no scheduler activates. No
  activated run exists to violate it. Such an instance never makes the item `refuted` and never makes it `inconclusive`.
- Under an admitted profile, an activated instance's value SHALL be the
  conditioned value of FR-196 "Conditioned readings": under a workload that leaves no choice the exact
  ratio of `Pr(E and act)` to `Pr(act)`, and under every scheduler or a
  workload with free delays the optimum over the schedulers that activate,
  by the reset construction. Its proof carries FR-201's
  `Conditional` certificate.

### Settling the item

- Under an admitted profile, each instance SHALL be settled with the claim's
  evidence kind (FR-741): an exact instance by its exact value against the
  bound, a statistical instance
  by its measurement.
- Under an admitted profile, the item SHALL be `refuted` when any instance
  fails, and its counterexample SHALL name the first failing instance in key
  order, so a rerun names the
  same instance.
- Otherwise, for an admitted analysis profile, the item SHALL be `proved`
  under exact evidence, or `measured`
  under statistical evidence, carrying the instances' evidence in FR-741's
  result members.
- The item SHALL NOT be settled `unsupported` for its activation.
- **Profile boundary.** FR-201's `on-each-reachability/v1` profile admits
  only an exact, untimed, unbounded `eventually` reachability claim under a
  resolved workload that leaves no choice, with no fairness, clocks, rewards
  or cyclic continuation. Outside that profile, the current analyzer settles
  an `on each` item `unsupported` with FR-272's
  `unsupported_construct`/`not-yet-implemented` diagnostic and no `cause`,
  numeric value or certificate. The profile excludes every-scheduler and
  free-delay optima, statistical evidence, cyclic continuation graphs,
  property forms other than unbounded eventuality reachability, timed
  subjects, and authored fairness. This is an evidence/profile boundary, not
  a refusal of the `on each` activation: the source reader admits the
  activation, and the same form without `on each` may have a profile. Profile
  rejection occurs after instance formation but before per-instance
  evaluation; the empty-instance `no-instance` result takes precedence. For
  an unsupported profile form, at least one actual instance is present in the
  acceptance cases below, so `unsupported` is not `no-instance`.
- An in-profile on-each claim whose guard or target read is undefined at a
  reachable state of an instance's phase graph likewise settles `unsupported`
  with the FR-272 diagnostic `unsupported_construct`/`not-yet-implemented`
  (no `cause`) until QSL-3003, which builds FR-741's undefined-path refutation
  for it.
- An in-profile on-each refutation that needs more than `max_witness_paths`
  path sets (FR-202 would then emit subsystem evidence) settles `incomplete`
  instead, not `unsupported`, with `LimitReached` whose `limit` is
  `max_witness_paths`, whose `value` is the count reached and whose `setting`
  is `exact_prob.max_witness_paths` (request member
  `limits.max_witness_paths`; FR-203's "Another limit reached" row, FR-255),
  until QSL-3004 builds FSpec FR-413's subsystem-certificate fallback (lower
  certificates for V and N). This is an implementation gap, not a refusal.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-875-AC-1 | Through `fsl analyze` over the `Link` model, the claim `E` of the use case with a start snapshot whose `msgs` population holds no object settles `inconclusive` with cause `no-instance`, never `proved` and never `unsupported`. | Test (TC-2485) |
| FR-875-AC-2 | With a start snapshot holding `m` undelivered and `n` already delivered, `E` settles `proved`: `n` never activates and holds vacuously. The same claim at `rational(99, 100)` settles `refuted`, its counterexample naming `m` and not `n`. | Test (TC-2486) |
| FR-875-AC-3 | With a start snapshot holding `a` and `b`, both undelivered, `E` at `rational(99, 100)` settles `refuted` with a counterexample naming `a`, the first failing instance in key order; two runs give equal items. | Test (TC-2487) |
| FR-875-AC-4 | With a nonempty `Link` snapshot, an on-each eventuality requested under every scheduler, and an on-each eventuality under a workload whose subject has a free delay, each item settles `unsupported` with `unsupported_construct`/`not-yet-implemented`, no `cause`, numeric value or certificate; neither is reported as unsupported for activation or as `no-instance`. | Test (TC-3211) |
| FR-875-AC-5 | With a nonempty `Link` snapshot, an on-each eventuality requested with statistical evidence settles `unsupported` with `unsupported_construct`/`not-yet-implemented`, no `cause`, measurement or certificate; it is not reported as unsupported for activation or as `no-instance`. | Test (TC-3211) |
| FR-875-AC-6 | With a nonempty `Link` snapshot whose on-each instance has a cyclic continuation graph, the item settles `unsupported` with `unsupported_construct`/`not-yet-implemented`, no `cause`, numeric value or certificate; it is not reported as unsupported for activation or as `no-instance`. | Test (TC-3211) |
| FR-875-AC-7 | With a nonempty `Link` snapshot, on-each `always`, `until`, bounded eventuality, expected-reward, long-run-fraction, measure and quantile claims each settle `unsupported` with `unsupported_construct`/`not-yet-implemented`, no `cause`, numeric value or certificate; none is reported as unsupported for activation or as `no-instance`. | Test (TC-3211) |
| FR-875-AC-8 | With a nonempty `Link` snapshot, an on-each eventuality over a timed subject and one with an authored fairness set each settle `unsupported` with `unsupported_construct`/`not-yet-implemented`, no `cause`, numeric value or certificate; neither is reported as unsupported for activation or as `no-instance`. | Test (TC-3211) |

## Dependencies

- [FR-186](FR-186-check-probabilistic-claim-forms-at-s3.md) checks and records
  the `on each` activation this requirement settles.
- [FR-125](FR-125-read-a-model-subject-s-behaviours-as-temporal-traces.md)
  defines instancing and `NoInstance`.
- [FR-741](FR-741-request-and-report-probability-items-through-analyze.md)
  carries the item request and result members.
- FSpec FR-407 "Activation" gives the meaning; FSpec FR-331 the result causes.
