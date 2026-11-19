---
id: FR-933
title: "Read a role-clock timed clause's interval bounds in its binding unit"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-033
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-642
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-640
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-393
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-233
    type: references
  - target: ix://agent-ix/filament-specification/FR-090
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-142
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-252
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-416
    type: depends_on
---
# FR-933: Read a role-clock timed clause's interval bounds in its binding unit

## Description

A clause under `filament.temporal.timed/v1` over a declared record trace subject
names a clock role, and its stamps come from a recorded trace (FR-642). Its
interval bounds may be written with a time unit (`[0 ms, 200 ms]`,
`[0 ms, 500 us]`) or as plain numbers (`[0, 200]`). Both forms SHALL be
admitted and SHALL be read in the clause's **binding unit**: the unit of the
role's `timed` configuration (`{"kind": "timed", "unit": U}`, FR-640), in which
the trace's stamps and every bound are compared. A unit-bearing bound converts
exactly to the binding unit, and a unit-free bound is a magnitude already in it.

The binding unit is always the configured one. FSpec FR-252 makes a clock
binding's unit exactly one value that is never defaulted, and FSpec FR-090 does
not infer a unit, so neither the clause's own bounds nor the recording supply
it. The in-process check takes the same configuration the request path takes,
so a clause checked and evaluated in process means what it means from a request
file (FR-640).

## Inputs

A timed-profile clause over a record trace subject with a role clock, the
unit's declared `Time` dimension and units, and the role's `timed`
configuration: the request's `clocks` member, or, in process, the same
`ClockConfiguration` value passed to the check.

## Outputs

The checked clause with its binding unit and each interval bound as an exact
non-negative rational in that unit; or a refusal.

## Behavior

### The binding unit

- The binding unit SHALL be the `unit` of the role's `timed` configuration,
  whichever entry checks the clause: the command adapter builds the
  configuration from `clocks`, and an in-process caller of the S3 check passes
  the same configuration for each role.
- When no configuration is given for the role, the check SHALL refuse with
  cause `missing-clock-selection`, naming the clock, the declaration and its
  profile, with FR-640's message, in process exactly as through the request
  path (where it is `invalid-request` at stage `emit`, exit 20). It SHALL NOT
  take a unit from the clause's bounds, from the trace or from a default.
- The configured unit SHALL name a unit the program unit declares in the
  `Time` dimension; a unit that names no declared unit SHALL refuse with cause
  `unknown-clock-unit`, and a unit of another dimension with cause
  `clock-unit-dimension`, each naming the clock and the unit (through the
  request path, `invalid-request` at stage `emit`, exit 20).
- The binding unit SHALL be recorded on the checked clause and in its FSpec
  FR-255 interval keys, so two clauses whose bounds are equal in different
  binding units have unequal keys.

### Bounds

- A unit-bearing bound SHALL be admitted when its unit is a declared unit of
  the `Time` dimension, and SHALL be converted exactly to the binding unit by
  FSpec FR-142 (`500 us` in `ms` is `1/2`, `200 ms` in `us` is `200000`); the
  result is a rational and is never rounded. A bound whose unit is of another
  dimension SHALL refuse `ill_typed`/`type-mismatch` at the bound's span.
- A unit-free bound SHALL be admitted and SHALL be the magnitude it writes in
  the binding unit (`[0, 200]` under a `ms` configuration is `[0 ms, 200 ms]`).
  A clause MAY write unit-free bounds on some operators and unit-bearing bounds
  on others; each interval keeps one kind of bound at both ends (FR-393, S1).

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-933-AC-1 | Checked in process with the configuration `{"kind": "timed", "unit": "ms"}` for `plant`, `eventually[0 ms, 200 ms] holds(r.ack)` has binding unit `ms` and bounds `0` and `200`; `eventually[0 ms, 500 us] holds(r.ack)` has bounds `0` and `1/2`, and over stamps `0`, `1/3` with `ack` at the second it holds, over `0`, `2/3` it is violated at position 0; `eventually[0, 200] holds(r.ack)` has bounds `0` and `200`, holds over stamps `0`, `150` and is violated over `0`, `250`. Checked in process with no configuration for `plant`, each of these refuses `missing-clock-selection` naming `plant`, `Ack` and `filament.temporal.timed/v1`. | Test (TC-2605) |
| FR-933-AC-2 | Compiled with `clocks` `{"plant": {"kind": "timed", "unit": "ms"}}`, `eventually[0, 200]` and `eventually[0 ms, 200 ms]` give equal bounds and equal FR-255 keys; with `"unit": "us"` (`unit us : Time = rational(1, 1000000) * s;`) `eventually[0 ms, 200 ms]` has bounds `0` and `200000` in `us`, and its key differs from the `ms` one. | Test (TC-2606) |
| FR-933-AC-3 | `"unit": "minute"` with no such unit declared refuses at stage `emit`, `invalid-request`, exit 20, `cause` `unknown-clock-unit` naming `plant` and `minute`; `"unit": "m"` with `dimension Length; unit m : Length = rational(1, 1);` refuses with `cause` `clock-unit-dimension`; a bound `200 m` refuses `ill_typed`/`type-mismatch` at its span. | Test (TC-2606) |

## Dependencies

- FR-642 (role-clock timed clauses), FR-640 (`timed` configuration, the
  `missing-clock-selection` refusal and in-process parity), FR-393 (interval
  bound forms).
- FSpec FR-090 (no inferred unit), FR-252 (the binding's unit is never
  defaulted), FR-142 (exact unit conversion), FR-255 (interval keys), FR-416
  (timed pointwise meaning).
