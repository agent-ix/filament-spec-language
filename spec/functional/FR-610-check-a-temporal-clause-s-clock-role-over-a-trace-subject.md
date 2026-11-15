---
id: FR-610
title: "Check a temporal clause's clock role over a trace subject"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-033
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-326
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-233
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-640
    type: references
  - target: ix://agent-ix/filament-specification/FR-090
    type: depends_on
---
# FR-610: Check a temporal clause's clock role over a trace subject

## Description

FSL's S3 checker SHALL read the clock name of a temporal clause or
`compensate` template over a trace subject (an `over` parameter whose type is
not a model object type) as a clock role, and SHALL refuse a role name that
spells a time unit or a model binding name, so that a clock name never appears
to set what an interval bound counts.

## Use case

An author writes `clock "ms"` with `eventually[0,3]` under the infinite-trace
profile and expects three milliseconds. The profile counts positions, so the
clause would mean three positions. The checker refuses the name at the clock
string and says that one tick is one position under the selected profile; the
author renames the role (`clock "events"`) or selects the timestamped-event
profile, whose unit comes from the request's clock configuration.

## Inputs

- A checked temporal clause (FR-326) or `compensate` template (FR-381) over a
  trace subject, with its clock string and its selected profile.
- The unit's time units: every unit of the time dimension the unit declares
  or imports (FR-230).

## Outputs

The clause's clock role, as written, on the checked clause, or a refusal.

## Behavior

- **Role.** Over a trace subject the clock string SHALL be a clock role: any
  non-empty text the author chooses. Every declaration that writes the same
  role reads the same clock (FR-640).
- **What a tick counts.** The role never changes what an interval bound
  counts; the profile fixes it:

  | Profile | One tick of `[a,b]` | Where the clock comes from |
  | --- | --- | --- |
  | event-position false-extension | one admitted event position | the role's `event_position` configuration (FR-640) |
  | fixed-sample false-extension | one sample of the configured period | the role's `fixed_sample` configuration |
  | timestamped-event finite-window | one tick of the configured timestamp unit | the role's `timestamped_event` configuration |
  | infinite-trace | one position of the trace | the trace's own position sequence; no configuration |
  | timed | an exact time quantity in the configured unit | the role's `timed` configuration (FR-642) |

- **Unit-spelled role.** If the role is a unit of the time dimension the unit
  declares or imports, or one of `ps`, `ns`, `us`, `µs`, `ms`, `s`, `min`, `h`
  and `d`, then the checker SHALL refuse `ill_typed`/`type-mismatch` at the
  clock string, with a message naming the role, the selected profile and what
  one tick counts under it. The comparison is exact on the role's bytes.
- **Model binding names.** If the role is `model-steps` or `model-time`, then
  the checker SHALL refuse it the same way: those names bind a model subject's
  steps or time (FR-233), a trace subject has no model, and FR-640 reserves
  both names from configuration.
- **Retired profile spellings.** A header profile that names a temporal
  profile by a retired spelling, such as `quire.temporal.infinite-trace/v1`,
  is an unknown profile: E3 refuses it `unknown_profile`/`unsupported-selection`
  at the identity literal (FR-110) and never maps it to the
  `filament.temporal.*/v1` profile of the same suffix, because FSpec FR-090
  forbids inferring a profile from a similarly spelled identity.
- The refusal is reported at S3 before emission, produces no checked package,
  and joins the clause's other refusals in source order.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-610-AC-1 | Over the record trace subject `c: Counter`, `eventually[0,3] holds(c.level = 3)` with `clock "events"` checks under the infinite-trace, event-position and timestamped-event profiles, and with `clock "sample-clock"` under the fixed-sample profile; each checked clause records its role as written and its interval `[0,3]` as three ticks of the profile's kind. | Test (TC-1541) |
| FR-610-AC-2 | The same clause with `clock "ms"`, `clock "s"` and `clock "min"` under the infinite-trace and event-position profiles, and with a role spelled as a unit `tick` the unit declares in its `Time` dimension, each refuses `ill_typed`/`type-mismatch` at the clock string, with a message naming the role, the profile and "position" (or the profile's tick kind); `clock "msg-clock"` is admitted. | Test (TC-1541) |
| FR-610-AC-3 | The same clause with `clock "model-steps"` and with `clock "model-time"` over the record trace subject refuses `ill_typed`/`type-mismatch` at the clock string under each of the four step-counting profiles. | Test (TC-1541) |
| FR-610-AC-4 | A unit whose temporal header profile is `quire.temporal.infinite-trace/v1`, with a clause `using` it, refuses `unknown_profile`/`unsupported-selection` at the identity literal, naming the alias, and produces no checked clause; the same unit with `filament.temporal.infinite-trace/v1` checks. | Test (TC-1541) |

## Dependencies

- FR-326 (profile admission), FR-233 (model-subject bindings), FR-640 (clock
  configurations keyed by role), FR-642 (timed clauses over a recorded trace).
- FSpec FR-090 "Clock names".
- Existing fixtures that write `clock "model-steps"` over a record subject are
  rewritten to a role name by the ticket that implements this requirement; a
  fixture left on `model-steps` over a record refuses, which is the intended
  change.
