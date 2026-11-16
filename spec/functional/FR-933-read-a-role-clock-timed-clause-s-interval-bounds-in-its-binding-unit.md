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
  - target: ix://agent-ix/filament-specification/FR-142
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
admitted and SHALL mean the same thing once the clause has a **binding unit**:
the one unit in which the trace's stamps and every bound are compared. A
unit-bearing bound converts exactly to the binding unit, and a unit-free bound
is a magnitude already in it. The binding unit comes from the `timed`
configuration when the request supplies one, and otherwise from the clause
itself, so a clause checked and evaluated in process, with no request, has the
same meaning it has under a configuration naming that unit.

## Inputs

A checked timed-profile clause over a record trace subject with a role clock,
the unit's declared `Time` dimension and units, and, when present, the
request's `timed` configuration for the role (FR-640).

## Outputs

The clause's binding unit and each interval bound as an exact non-negative
rational in that unit; or a refusal.

## Behavior

### The binding unit

- When the request's `clocks` member configures the role as
  `{"kind": "timed", "unit": U}`, the binding unit SHALL be `U`. `U` SHALL name
  a unit the program unit declares in the `Time` dimension; a `U` that names
  no declared unit SHALL refuse at stage `emit`, `invalid-request`, exit 20,
  `cause` `unknown-clock-unit`, naming the clock and `U`, and a `U` of another
  dimension SHALL refuse the same way with `cause` `clock-unit-dimension`.
- With no configuration (an in-process check and evaluation, which FR-640's
  `missing-clock-selection` does not reach), the binding unit SHALL be the
  coarsest unit (the one with the greatest exact factor to the dimension's
  base) that the clause's unit-bearing bounds name, the first of those in
  source order on a tie of equal factors. A clause with no unit-bearing bound
  has no named binding unit: its unit-free bounds and the trace's stamps are
  read as numbers in one and the same unit, whatever unit the recording uses.
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
- When the clause is compiled with a configuration whose unit differs from the
  unit the in-process default would choose, the configured unit SHALL win and
  every bound SHALL be converted to it; the clause's meaning over stamps in the
  configured unit is unchanged.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-933-AC-1 | Checked in process with no configuration, `Ack` with `eventually[0 ms, 200 ms]` has binding unit `ms` and bounds `0` and `200`; `eventually[0 ms, 500 us]` has binding unit `ms` and bounds `0` and `1/2`, and over stamps `0`, `1/3` with `ack` at the second it holds, over `0`, `2/3` it is violated at position 0; `eventually[0, 200]` has no named binding unit and bounds `0` and `200`, and over stamps `0`, `150` with `ack` at the second it holds. | Test (TC-2605) |
| FR-933-AC-2 | Compiled with `clocks` `{"plant": {"kind": "timed", "unit": "ms"}}`, `eventually[0, 200]` and `eventually[0 ms, 200 ms]` give equal bounds and equal FR-255 keys; with `"unit": "us"` (`unit us : Time = rational(1, 1000000) * s;`) `eventually[0 ms, 200 ms]` has bounds `0` and `200000` in `us`, and its key differs from the `ms` one. | Test (TC-2606) |
| FR-933-AC-3 | `"unit": "minute"` with no such unit declared refuses at stage `emit`, `invalid-request`, exit 20, `cause` `unknown-clock-unit` naming `plant` and `minute`; `"unit": "m"` with `dimension Length; unit m : Length = rational(1, 1);` refuses with `cause` `clock-unit-dimension`; a bound `200 m` refuses `ill_typed`/`type-mismatch` at its span. | Test (TC-2606) |

## Dependencies

- FR-642 (role-clock timed clauses), FR-640 (`timed` configuration, emit-stage
  refusals), FR-393 (interval bound forms).
- FSpec FR-142 (exact unit conversion), FR-255 (interval keys), FR-416 (timed
  pointwise meaning).
