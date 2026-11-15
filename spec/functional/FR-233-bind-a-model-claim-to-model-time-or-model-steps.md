---
id: FR-233
title: "Bind a model claim to model-time or model-steps"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-026
    type: implements
  - target: ix://agent-ix/filament-spec-language/ADR-026
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/ADR-018
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-123
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-230
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-231
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-090
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-252
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-255
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-640
    type: references
---
# FR-233: Bind a model claim to model-time or model-steps

## Description

FSL's S3 checker SHALL give every temporal claim over a model subject one
clock binding, `model-steps` or `model-time` (ADR-026 CB-1 to CB-5). Under
`model-steps` an interval counts steps; under `model-time` it counts time
units of the model's time source. A claim over a timed model binds
`model-time` unless it writes `clock "model-steps"`, and a claim over an
untimed model binds `model-steps`.

## Use case

A verification operator writes "within 5 ms" over a timed controller model
and expects 5 to mean milliseconds, with no clock clause. Over the same
model they state a step-bounded protocol property by writing `clock
"model-steps"`. Over an untimed model a `model-time` claim is refused,
because the model has no time to count.

## Inputs

- A checked temporal clause (FR-123) over a model subject, with its
  optional `clock` clause.
- The model's `TimedModel` (FR-230), or none.

## Outputs

```rust
pub enum ModelClockBinding {
    ModelSteps,
    ModelTime { model: QualifiedName, source: TimeSourceKind, unit: TimeUnit,
                period: Option<ExactQuantity> },
}
```

on the checked clause, its FSpec FR-252 binding key, and each interval's
FSpec FR-255 key under that binding.

## Behavior

- **Supplied clocks.** Over a model subject the clock binding is always
  `model-steps` or `model-time` and comes from the model, under every profile:
  under event-position a `ModelSteps` clause counts the model's steps, and its
  emitted `event_position` configuration names the model's step sequence as
  sequence authority, so no request member configures it
  ([FR-640](FR-640-select-clock-configurations-in-compile-and-run-requests.md)).
  Over a trace subject the clock name is a free role (FR-610), and that role is
  the key of the request's `clocks` member (FR-640), a timed clause's role taking
  a `timed` configuration (FR-642).
- **Default.** When a clause over a timed model writes no `clock` clause,
  the checker SHALL bind `ModelTime`. When a clause over an untimed model
  writes none, the checker SHALL bind `ModelSteps`.
- **Explicit.** `clock "model-steps"` SHALL bind `ModelSteps` over any
  model. `clock "model-time"` SHALL bind `ModelTime` over a timed model.
- **Other names.** Over a model subject the clock name SHALL be `model-steps`
  or `model-time` under every profile; the name never selects what a tick
  counts beyond these two bindings (FSpec FR-090 "Clock names"). If a clause
  over a model subject writes any other name, then the checker SHALL refuse
  `unsupported_construct`/`declaration-form` at the clock name, with a message
  naming the written name and the two admitted names, as a hybrid claim
  refuses `clock "wall"` (FR-447-AC-3). No clause over a model subject is
  checked with a free clock role, so `clock "ms"` never counts steps silently.
  The rule for clock roles over a trace subject is FR-610's.
- If a clause binds `model-time` over a model with no `time` member, then
  the checker SHALL refuse `invalid_timed_model`/`model-time-on-untimed-model`
  at the clock clause's span, naming the model.
- A `ModelTime` clause SHALL select the timed profile (FR-234). A
  `ModelSteps` clause SHALL keep ADR-018's profiles.
- **Identity.** The binding's FSpec FR-252 key SHALL be built from the
  timed profile identity, the model's qualified name and time source kind,
  the unit, the period for a `Tick` source, and the step-order sequence
  authority. Writing the default binding SHALL give the same key as
  omitting it.
- **Intervals.** Under `ModelTime`, each interval bound SHALL be a time
  quantity converted exactly to the model's unit (FSpec FR-142); a bound
  with no unit, or of another dimension, SHALL refuse `ill_typed`/
  `type-mismatch` at its span. Under `ModelSteps`, an interval bound SHALL
  be a step count, and a bound with a time unit SHALL refuse `ill_typed`/
  `type-mismatch`.
- The interval's FSpec FR-255 key SHALL carry the profile identity and the
  binding, so a time interval's key never equals a step interval's.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-233-AC-1 | Over `Rpc`, `Settles` with `clock "model-time"` and the same clause with no clock clause check to equal bindings and equal FR-252 keys; `NoLateReply` with `clock "model-steps"` binds `ModelSteps`. Over the untimed `Counter` model a clause with no clock clause binds `ModelSteps`, and `clock "model-time"` refuses `invalid_timed_model`/`model-time-on-untimed-model` naming `Counter`. | Test (TC-688) |
| FR-233-AC-2 | `eventually[0 ms, 3 ms]` under `model-time` over `Rpc` checks with bounds `0` and `3` in `ms`; `eventually[0 s, 3/1000 s]` checks to equal bounds and an equal FR-255 key. `eventually[0, 3]` under `model-time` refuses `ill_typed`/`type-mismatch`, and `eventually[0 ms, 3 ms]` under `model-steps` refuses `ill_typed`/`type-mismatch`. | Test (TC-688) |
| FR-233-AC-3 | The `Rpc` model with `time dense unit ms` and the same model with `time tick send period 1 ms unit ms` give `model-time` bindings with different FR-252 keys; two tick models that differ only in period give different keys. A `[0,3]` step interval and a `[0 ms, 3 ms]` time interval have different FR-255 keys. | Test (TC-688) |
| FR-233-AC-4 | Over a model subject under the infinite-trace profile, `eventually[0,3] holds(r.stopped)` with `clock "ms"`, with `clock "wall"` and with `clock "steps"` each refuses `unsupported_construct`/`declaration-form` at the clock string, with a message naming the written name, `model-steps` and `model-time`; the same clause with `clock "model-steps"` checks with the `ModelSteps` binding. The same refusal holds under the event-position profile. | Test (TC-1540) |

## Dependencies

- ADR-026 §5 CB-1 to CB-5; ADR-018 §1 as amended by ADR-026.
- [FR-123](FR-123-check-fairness-and-interval-operators-of-infinite-trace-clauses.md),
  [FR-230](FR-230-check-time-declarations-clocks-and-clock-constraints.md),
  [FR-231](FR-231-read-a-timed-subject-s-behaviours-as-timed-traces.md).
- FSpec FR-252 (clock binding key), FSpec FR-255 (interval key), FSpec
  FR-090 (profiles), FSpec FR-142 (units).

## References

- FSpec half: FSpec FR-416 (Linear STD-139) owns the `model-steps` and
  `model-time` bindings in FSpec FR-252, the timed profile identity in FSpec
  FR-255's key, and the default-binding grammar (ADR-026 OV-3, OV-4, OV-7).
