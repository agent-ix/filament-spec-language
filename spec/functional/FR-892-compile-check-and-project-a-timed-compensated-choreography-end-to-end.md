---
id: FR-892
title: "Compile, check and project a timed compensated choreography end to end"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-024
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-640
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-207
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-383
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-636
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-890
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-322
    type: depends_on
---
# FR-892: Compile, check and project a timed compensated choreography end to end

## Description

A protocol that holds a compensation template with a `clock` and a timed
`await` of the same clock, together with a `parallel` fan-out with a join and
a `requires temporal` member, SHALL compile, run and be analyzed end to end.
The steps are:

- **Compile.** The request's `clocks` member (FR-640) supplies the
  configuration for the template's clock and the await's clock. An `await`
  is a clocked declaration in FR-640's sense, as a template is. The compile
  forms one clock selection for each of them. S4 writes the protocol, its
  template, its await and its `requires temporal` member (FR-383, FR-880),
  and writes each selection as one FSpec FR-322 `temporal_bindings` row: the
  template's keyed by the compensation node and the await's keyed by the
  await control node, each on its `declaration` occurrence with its own
  `clock_name` (FSpec FR-322 "Temporal clock bindings", FR-322-AC-305). The
  reader admits the await's row under the same rules as the template's.
- **Run.** The protocol system steps the protocol (FR-206, FR-207, FR-882).
  It reads no clock (ADR-027 "Waiting"). An await's `timeout` is enabled
  while the await waits, and a compensation's `within` window is not read,
  which admits every behaviour that some deadline admits. The explored run
  therefore settles under the untimed reading whatever the configured clock
  says. A recorded execution is checked against the windows by the clock
  that the recording carries (FR-895).
- **Analyze.** Protocol deadlock freedom (FR-636) settles over the run, and
  projection under `bounded` (FR-890) projects the await and the template.

A declaration that needs a clock selection and has none SHALL refuse as
FR-640 states, `missing-clock-selection`. The command
SHALL name the declaration in the refusal.s `<declaration>` text by kind and qualified name: `compensate template
<P>::<T>` for a compensation template, `await <P>::<A>` for an await, and
`temporal clause <N>` for a temporal clause. The text SHALL NOT call a
template or an await a "temporal declaration". The S4 refusal for a
selection missing at emission, which arises only when a compile bypasses the
`clocks` member, SHALL name the declaration the same way.

### Decision: the protocol run stays untimed, and the clock is carried

ADR-027 reads no clock in a protocol run, so a deadline never removes a
behaviour from the explored state space. That reading is sound for safety
and deadlock claims over every deadline at once. The clock configuration is
still required, because the checked package carries each clocked
declaration's binding (FSpec FR-090), and because a recorded run is checked
against the windows by its own clock. A message that called a compensation
template a "temporal declaration" sent the author to the temporal clauses,
where nothing was wrong, so the message names the declaration kind.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-892-AC-1 | The staged saga (fan-out `parallel` of two shipments joined `all`, compensation `Refund` with `clock "c"` and `within [0, 120]`, an `await` on clock `"c"` `within [0, 120]`, and `requires temporal Due`) compiles with `clocks` `{"c": ...}`. The package holds the template, the await and the `requires temporal` edge to `Due`'s clause node, and is admitted. | Test (TC-2525) |
| FR-892-AC-2 | The saga's protocol system explores to a settled deadlock-freedom result. Its reachable states include one in which `Refund`'s registration is `recovered` and one in which the await took `timeout`. | Test (TC-2525) |
| FR-892-AC-3 | Projection of the saga under `bounded` onto the await's owner returns a local await carrying `[0, 120]` and clock `c`. Onto each role, the result is `proved` or `refuted`, never `unsupported`. | Test (TC-2525) |
| FR-892-AC-4 | The saga compiled without `clocks` refuses `missing-clock-selection` with `details.declaration` naming `Refund`, and a message that holds `compensate template` and `Refund` and does not hold `temporal declaration`. With the template's clock configured and the await's missing, the refusal names `await` and the await's name. | Test (TC-2525) |

## Dependencies

- [FR-640](FR-640-select-clock-configurations-in-compile-and-run-requests.md):
  clock selection.
- [FR-207](FR-207-run-compensation-templates-as-protocol-threads.md) and
  [FR-882](FR-882-step-every-specified-protocol-construct.md): the run.
- [FR-383](FR-383-emit-a-checked-protocol-as-v2-protocol-nodes.md): the
  written package.
- [FR-636](FR-636-request-protocol-analysis-items-and-replay-supplied-protocol-traces.md)
  and [FR-890](FR-890-project-every-protocol-construct-under-the-bounded-profile.md):
  the analysis.
- [FR-895](FR-895-assess-a-recorded-protocol-execution-against-its-obligations.md):
  recorded runs and their clocks.
