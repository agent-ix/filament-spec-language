---
id: FR-942
title: "Check a model claim with no clock member under its profile's default binding"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-026
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-233
    type: specifies
  - target: ix://agent-ix/filament-specification/FR-765
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-252
    type: depends_on
---
# FR-942: Check a model claim with no clock member under its profile's default binding

## Description

FSL's S3 checker SHALL bind a temporal clause over a model subject that writes
no `clock` member to the default binding FSpec FR-765 gives its selected
profile: `ModelTime` under `filament.temporal.timed/v1`, and `ModelSteps` under
the event-position, fixed-sample, timestamped-event and infinite-trace
profiles. It SHALL then check the clause exactly as the same clause with that
binding written. A clause with no `clock` member is never refused for the
missing member.

### Decision: the profile chooses, the model only enables

Before this rule the default came from the model alone, and two combinations
had no binding: a timed model under a step profile (the model's `model-time`
default selects the timed profile, which the alias did not select), and an
untimed model under the timed profile (the model's `model-steps` default is a
step count the timed profile does not read). Both are refused today as a
missing clock, which tells the author nothing true. The clause already names
its profile, and the profile already fixes what an interval counts, so the
profile chooses the binding (FSpec FR-765). The first combination then checks
as a step-counted claim over a timed model, which FR-233 "Explicit" already
supports when the clock is written. The second has no time to count and
refuses as an explicit `clock "model-time"` over that model does, located at
the `using` alias because no clock member exists to locate it at.

## Inputs

A clause over a model subject with no `clock` member, its profile alias, and
the model's `TimedModel` or none (FR-230).

## Outputs

The clause's `ModelClockBinding` (FR-233), or a refusal.

## Behavior

- Under the timed profile, the checker SHALL bind `ModelTime` over a timed
  model; over an untimed model it SHALL refuse
  `invalid_timed_model`/`model-time-on-untimed-model` at the span of the
  clause's `using` alias, with a message naming the model, the timed profile,
  and that a step-counted claim is written under a step profile.
- Under every other profile, the checker SHALL bind `ModelSteps` over a timed
  and an untimed model alike, and check interval bounds as step counts (FR-233
  "Intervals").
- The checker SHALL NOT emit `MissingClock` for a clause over a model subject.
- The binding and its FSpec FR-252 key SHALL equal those of the same clause
  with the default written.

## Acceptance Criteria

The timed model is FR-233-AC-5's ConfigVersion model timed by `dimension
Time; unit ms : Time = rational(1, 1); time Config dense unit ms;`.

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-942-AC-1 | Over the timed model under the event-position profile, `eventually[0,3] holds(c.versionNumber = 2)` with no `clock` member checks with `ModelSteps` and the FR-252 key of the same clause with `clock "model-steps"`; `eventually[0 ms, 3 ms] holds(c.versionNumber = 2)` there refuses `ill_typed`/`type-mismatch` at the interval, never `MissingClock`. | Test (TC-2622) |
| FR-942-AC-2 | Over the untimed ConfigVersion model under the timed profile, `eventually[0 ms, 3 ms] holds(c.versionNumber = 2)` with no `clock` member refuses `invalid_timed_model`/`model-time-on-untimed-model` at the `using` alias, naming `Config`, category refusal, exit 20; under the infinite-trace profile `eventually holds(c.versionNumber = 2)` with no `clock` member checks with `ModelSteps`. | Test (TC-2622) |

## Dependencies

- FR-233 owns the model clock bindings; FSpec FR-765 owns the default rule.
