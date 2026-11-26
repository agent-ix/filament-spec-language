---
id: FR-920
title: "Emit a timed-product probability certificate"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-023
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-201
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-196
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-204
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/ADR-028
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-725
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-322
    type: references
---
# FR-920: Emit a timed-product probability certificate

## Description

EN-5 SHALL certify every exact proof over a timed subject (FR-204) with a
document in the closed profile
`"filament.probability-objective/timed-product/v1"`, FSpec FR-725's
timed-product certificate, and SHALL emit the proof only after FR-921's
checker accepts it. The document attests that its initial value is the
claim's value on the dense-time semantics, over the time-divergent behaviours
ADR-026 TS-4 and TS-5 admit, read through digital clocks or clock regions.

The profile covers every timed form FR-204 decides: reachability, until and
`always` over state predicates, deadline events, every other bounded-window
formula over its monitor, expected elapsed time and quantiles of an elapsed
measure; under a workload and under every scheduler; on the digital and the
region route; with `discrete` delays, free delays, zero-time cycles and
time-locks. No timed proof is emitted `uncertified` or in another profile.

## Use case

A reviewer receives `proved` for `Retx`'s `Deadline` with the guard
`self.x > 1 ms`. The certificate says `route: "region"`; the reviewer runs
`fsl-probability-check` on it and the emitted package, which rebuilds the
region MDP itself, finds the minimum `99/100` at the initial region and
accepts. Had EN-5 read the strict guard through digital clocks, the checker
would have refused the certificate before reading a value.

## Inputs

- An EN-5 exact result over a timed subject (FR-204) and the checked claim.

## Outputs

- The document below, carried as the record's exact certificate bytes
  (`TerminalRecord::exact_certificate_document`).

## Behavior

### Document

- The document SHALL carry FR-201's `identity`, `objective`, `kind`,
  `values`, `ranking` and, where FR-201 requires one, `policy`, and no other
  member. `kind` SHALL be `exact`, `lower` or `upper`.
- `objective` SHALL carry `profile`, the targets or measure of the form,
  `optimization` (`minimum`, `maximum`, `workload`, `workload-minimum` or
  `workload-maximum`), `fairness`, and `timed`. `optimization` SHALL be
  `workload` when the checker's product has exactly one choice at every state;
  otherwise it SHALL be `workload-minimum` for a `>=` bound and
  `workload-maximum` for a `<=` bound, the residual choices being FR-204's
  free-delay choices and the time-passing choices (a unit delay against the
  idle delay), so that a bound is decided on the extremum of the choices the
  workload leaves the scheduler. The checker derives the spelling from its own
  product, and a document that states another spelling is rejected. `timed` is
  `{"route": "digital" | "region", "unit": <UnitId>, "scale": "<n>", "form": <form>}`
  with `form` one of `reach`, `until`, `always`, `deadline`, `bounded-window`,
  `expected-elapsed`, `elapsed-quantile`. The checker compares these with its
  own derivation and never uses them to build the product (FR-921).
- The document SHALL carry no component, exit, time-lock, race, region
  successor, activation or transition member. A list of these would be a
  second, unchecked statement of what the checker computes.

### Keys

- A digital key SHALL be FR-196's complete product key with the model member
  `["digital",discrete,unit,scale,clocks,deadline,tail]`.
- A region key SHALL use the model member
  `["region",discrete,unit,scale,clocks,deadline,tail]` whose `clocks`
  entries are `[clock_identity,integer,cap,fraction]`: `integer` is the
  clock's integer part in scaled units, at most `cap`; `fraction` is `"0"`
  when the fractional part is zero or the clock is at its cap, and otherwise
  the canonical decimal position, from `"1"`, of its fractional part in the
  increasing order of the state's nonzero fractional parts, equal fractions
  sharing a position. `deadline` is `["deadline",D,t,fraction]` with the same
  reading. Clock identity, order and caps are FR-196's.
- For `bounded-window`, whose route is always `"region"` (FR-204 Admission
  "Route"), the key SHALL be the region key with the monitor member FR-196's
  `["window",residual]`. Each obligation's `[kind,o,integer,fraction]` reads
  `integer` and `fraction` as a region clock entry does, and the obligation
  clocks are among "the state's nonzero fractional parts" that order every
  `fraction` of the key: model clocks, the deadline clock and obligations
  alike. Two keys whose residuals differ in any byte are different states.
- A bounded-window document SHALL hold a row for every tail-entry state of
  FR-196 "Idle tail of a timed window monitor": the region key with `tail`
  `"idle-entry"` and every other member, the unfolded residual included,
  byte for byte that of the quiescent state it is entered from, valued with its tail value for the document's
  optimization (0 or 1). It is a decided state: an entry of value 1 is a
  target and one of value 0 a rejecting decided state (FR-201), neither
  expanded, each with FR-201's decided-state rank 0. The document SHALL carry no stutter time, tail path, layer or
  tail-search member, and no row for an `"idle-ready"` or `"idle-delayed"`
  state of that product; the checker computes the tail value itself. A
  `policy` names a state's move to its tail-entry state by the action body
  `["enter-idle-tail"]` (FR-196 `ProductAction::EnterIdleTail`), never a
  stutter time.
- For `expected-elapsed` on the region route, the key SHALL be the region key
  with the model member's tag `"boundary-region"` and a trailing `corner`, one
  canonical integer string per clock entry in entry order, then the deadline
  clock: a vertex of the closure of the region the state approaches. Each
  delay edge between boundary-region states carries the exact elapsed time
  between their corners.
- For `elapsed-quantile`, the model member's `deadline` slot SHALL be
  `["elapsed",W,e]` (or `["elapsed",W,e,fraction]` on the region route), `e`
  the elapsed clock: `null` before activation, reset to 0 at activation,
  advanced by every delay while active, frozen at completion or censoring,
  capped at `W + 1`. The monitor member SHALL be FR-196's
  `["measure",remaining,phase]` with `remaining` `null`, because the measure's
  window is the clock, not a count of positions.
- Two keys differing in any member are different states; replacing any key
  with its model state key alone, or a region key with the digital key of the
  same integer parts, fails comparison.

### Carriage

- The document SHALL travel in the FR-286 result and the FSpec FR-331
  terminal record, as every EN-5 certificate does. S4 SHALL NOT emit it into
  the checked package (FSpec FR-322): the package carries the timed-model
  nodes, `delay_distribution` nodes and claim the checker rebuilds from.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-920-AC-1 | Over `Retx` (ADR-028 §15.5) with the guard `self.x >= 1 ms`, the exact proof of `Deadline` at 0.99 over every scheduler retains a `timed-product/v1` document whose `objective.timed` is `{route: "digital", unit: ms, scale: "1", form: "deadline"}`, whose keys give `x` the values 0 to 2 and the deadline clock 0 to 5, and whose initial value is `99/100`; `check_probability_certificate_document` accepts it. | Test (TC-2489) |
| FR-920-AC-2 | With the guard `self.x > 1 ms`, the proof of `Deadline` retains a document with `route: "region"` and region keys; two of its keys that differ only in the `fraction` of `x` and of the deadline clock survive a write and read as distinct keys. Temporary (QSL-3324 fallback F-7, while QSL-3444 is open): the `Deadline` proof is withheld (`RegionWalkUnchecked`), so its region document is not emitted, and the key-distinctness clause is not run. | Test (TC-2489) |
| FR-920-AC-3 | AC-1's document with an added `components` member is rejected `Malformed` at `components`; the package S4 emits for AC-1's unit contains no certificate node. | Test (TC-2489) |
| FR-920-AC-4 | Over a workload whose product has a delay choice (a free `tick` guarded `self.x >= 1 ms` racing `send` given 1 or 2 ms under a workload weighing both 1), the proof of a `>=` bound retains `workload-minimum` and of a `<=` bound `workload-maximum`, each with the value of that extremum; a bound strictly between the extrema is refuted with the checker's recomputed mass of the witness scheduler; a document that states `workload` or the other extremum's spelling is rejected. A workload whose product has one choice at every state retains `workload`, and a residual spelling is rejected there. Temporary (QSL-3324 fallback F-4, while it holds): the free `tick`'s window is wider than a single point, so the proof and the refutation settle `Incomplete(TimedRouteWithheld{reason: WorkloadFreeDelay})` and their spelling clauses are not run; the one-choice workload clause is unchanged. | Test (TC-2489) |

## Dependencies

- [FR-201](FR-201-check-a-probability-certificate.md) supplies the
  certificate conditions and the checker entry points.
- [FR-196](FR-196-build-the-probabilistic-product.md) supplies the digital
  key; [FR-204](FR-204-check-probabilistic-timed-automata-through-digital-clocks.md)
  the readings.
- FSpec FR-725 is the standard's half.
