---
id: FR-640
title: "Select clock configurations by clock name in compile and run requests"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-033
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-027
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-100
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-233
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-517
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-522
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-642
    type: references
  - target: ix://agent-ix/filament-specification/FR-540
    type: implements
  - target: ix://agent-ix/filament-specification/FR-090
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-252
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-322
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-323
    type: depends_on
---
# FR-640: Select clock configurations by clock name in compile and run requests

## Description

A `1-draft` native-compile/1 request (FR-027) and a `1-draft` native-run/1
request (FR-100, FR-312) SHALL accept an optional top-level `clocks` member that
supplies the clock configuration of every temporal clause and `compensate`
template whose profile reads a clock the program does not define itself. `clocks`
is the request-file form of the trace-subject clock binding that FSpec FR-090
and FR-252 say the observation interface supplies, and of FSpec FR-540's clock
selection. The compile the command performs turns it into one FR-050
`TemporalClockSelection` per declaration and hands those selections to the
emitter, so an event-position, fixed-sample, timestamped-event or trace-subject
timed unit compiles and runs from a request file exactly as it does in process.

## Use case

An operator writes `temporal Deadline using ts over (r: Reading) clock "plant-ms"
on each (q: Reading) { eventually[0,200] holds(r.ack) }` under the
timestamped-event profile and three more clauses on the same `"plant-ms"` clock.
They add `"clocks": {"plant-ms": {"kind": "timestamped_event", "timestamp_unit":
"ms"}}` once to the compile request, and every clause on that clock binds to it.
They never name a clause, a source span or a declaration node.

## Inputs

`clocks`: a JSON object. Each member name is a clock name, the exact text of a
declaration's `clock "<name>"` argument. Each member value is one clock
configuration in the checked package's `TemporalBinding` `clock` wire form
(FSpec FR-322 "Temporal clock bindings"), a closed object tagged by `kind`:

| `kind` | Other members | Profile it serves |
|--------|---------------|-------------------|
| `event_position` | `sequence_authority`: non-empty string | `filament.temporal.event-position.false-extension/v1` |
| `fixed_sample` | `epoch`, `period`: exact numbers (`{"kind": "integer", "decimal": D}` or `{"kind": "rational", "numerator": N, "denominator": M}`), `period` positive; `unit`: non-empty string | `filament.temporal.fixed-sample.false-extension/v1` |
| `timestamped_event` | `timestamp_unit`: non-empty string | `filament.temporal.timestamped-event.finite-window/v1` |
| `timed` | `unit`: non-empty string | `filament.temporal.timed/v1` over a declared record trace subject (FR-642) |

The `timed` alternative is a fourth `ClockConfiguration` variant of the checked
package contract (`{"kind": "timed", "unit": U}`); its `TemporalBinding` row has
the same `declaration`, `occurrence` and `clock_name` members as the other three.

The driver's `compile` and `run` verbs take `--clocks <json-file>`, a file whose
whole content is the same object, and supply it as the request's `clocks`.

## Outputs

The checked package's `temporal_bindings` rows: one row per clock-bearing
declaration occurrence, carrying the declaration's clock name and the selected
configuration (FR-337), or a refusal below. A run request's report is FR-109's,
computed over the package compiled with these rows.

## Behavior

- The command SHALL key a selection by clock name, not by clause name or source
  span. A clock name is the role the source writes; every declaration that writes
  the same clock name reads the same clock, so one configuration per clock name
  makes "two configurations for one clock" unrepresentable (FSpec FR-090-AC-2).
- For each temporal clause and `compensate` template over a declared record
  trace subject (or, for a template, with no model subject) whose profile is
  event-position, fixed-sample, timestamped-event or timed, the command SHALL take the configuration
  of the `clocks` member named by its clock name and SHALL form one
  `TemporalClockSelection` from the declaration's source, its declaration span
  and that configuration, in source order, before the emitter runs.
- A declaration over a model subject takes its binding from the model
  (FR-233) and no `clocks` member serves it. The command SHALL derive its
  configuration from the binding and the declaration's profile, so a unit
  whose clocks all come from models compiles with no `clocks` member, under
  every profile:

  | Profile | Binding | Derived configuration |
  |---------|---------|-----------------------|
  | event-position | `model-steps` | `{"kind": "event_position", "sequence_authority": M}` |
  | fixed-sample | `model-steps` | `{"kind": "fixed_sample", "epoch": {"kind": "integer", "decimal": "0"}, "period": {"kind": "integer", "decimal": "1"}, "unit": "step"}` |
  | timestamped-event | `model-steps` | `{"kind": "timestamped_event", "timestamp_unit": "step"}` |
  | timed | `model-time` | `{"kind": "timed", "unit": U}` |

  `M` is the model's declaration identity text and `U` the model's time unit.
  The model's step sequence is the authoritative sequence (FSpec FR-090), so
  each step is one event, one sample of period one step, or one time stamp
  one step after the last; the unit `step` names that count. A `model-time`
  binding under a profile other than timed is FR-233's refusal, not this
  table's.
- A hyper clause over a model's behaviours is a clock-bearing declaration
  under the same rule: its binding comes from the model and the command
  derives its configuration by the table above.
- A request that compiles library units with the program (FR-099) SHALL serve
  every unit from the one `clocks` member: a clock name is a role of the
  request, and a library unit's clause on clock `plant-ms` reads the same
  `plant-ms` member as a program clause. `unused-clock-selection` counts the
  clocks of library units too. Which clock
  names a declaration may write is the rule of FR-233 (model subjects), FR-610
  (trace-subject roles) and FSpec FR-090 "Clock names", not this requirement's.
- The emitter SHALL record, in the program package's lock member
  `dependency_clock_selections` (FSpec FR-322), one `{clock_name, clock}`
  entry for every clock name that a clock-bearing declaration of a library
  unit in the package's closure, direct or transitive, takes from `clocks`,
  with the configuration it was served, in ascending `clock_name` order, and
  SHALL omit the member when there is none. A library package keeps its own
  declarations' rows in its own `temporal_bindings`, but a replay recompiles
  the closure from sources and reads only the proving package, so without
  this member a program whose library uses a clock of its own would compile
  and then fail every replay. The entry for a clock name the program's own
  declarations also use equals their rows, because one `clocks` member serves
  every unit.
- The configuration's `kind` SHALL be the one the declaration's profile serves
  (table above). A clock name written by declarations under two profiles that
  need different kinds cannot be served by one member and refuses.
- A run request that selects a checked package instead of compiling source SHALL
  NOT carry `clocks`: the package already holds its `temporal_bindings` rows.
- A request without `clocks` supplies no selection. Its declarations that need
  one refuse as below; a program with none compiles as before.
- The command SHALL refuse each case below at stage `emit`, code
  `invalid-request`, exit 20, before any package bytes are written. The refusal
  carries a `message` that names the clock and the `clocks` member, `details`
  `{"cause": C, "clock": NAME}` (plus `declaration`, the declaration's name, and
  `profile`, its profile identity, where one declaration is at fault), and the
  locus of the declaration's `clock` argument when one declaration is at fault:

  | Case | `cause` | Message |
  |------|---------|---------|
  | A declaration needs a configuration and `clocks` has no member for its clock name | `missing-clock-selection` | `clock "<name>" used by <declaration> under <profile> has no configuration; add "<name>" to the request's clocks member` |
  | A member names a clock that no declaration taking its configuration from `clocks` writes | `unused-clock-selection` | `the clocks member "<name>" names a clock no temporal declaration uses` |
  | A member's `kind` is not the one a declaration's profile serves | `clock-kind-mismatch` | `clock "<name>" is a <kind> configuration but <declaration> uses <profile>, which needs <expected kind>` |
  | A member named `model-steps` or `model-time` | `reserved-clock-name` | `clock "<name>" is a model binding name; a trace-subject clock role cannot be named model-steps or model-time, and a model subject's binding comes from the model` |

- A `clocks` value that is not an object, a member value that is not a closed
  well-formed configuration (unknown `kind`, a missing, extra or empty member, a
  non-positive `period`, a non-reduced rational), or both `clocks` in the request
  and `--clocks` on the command line, SHALL refuse `invalid-request` at stage
  `request`, exit 20, naming `clocks` or the member.
- The selection SHALL enter the package identity through its `temporal_bindings`
  rows only: two requests that differ only in a configuration compile to packages
  with different identities (FSpec FR-090-AC-3), and two that differ only in the
  order of `clocks` members compile to equal bytes.
- The command SHALL pass the same selections to every later step that re-emits
  the package (the replay recompile and the monitor's emission, FR-283, FR-643),
  so no step runs with an empty selection list while the request supplied one.
  A replay request has no `clocks` member: the replay recompile SHALL read its
  selections from the proving package's `temporal_bindings` rows and its lock
  `dependency_clock_selections` entries, each row's or entry's `clock_name`
  and `clock` giving one member, and SHALL serve the program and every
  recompiled library unit from that one selection, so it compiles with exactly the
  configuration the proof ran under and cannot be handed a different one.  The replay
  request wire has no `clocks` member, so a replay cannot be handed another
  configuration; a driver that reads a replay request document refuses a
  `clocks` member there `invalid-request` at stage `request`, naming it.
- The replay recompile SHALL refuse, with no result, a library declaration
  whose clock name that selection does not hold (`missing-clock-selection`,
  naming the clock and the library, at `/package/lock/dependency_clock_selections`),
  a `dependency_clock_selections` entry that no recompiled library
  declaration reads (`unused-clock-selection` at
  `/package/lock/dependency_clock_selections/{i}`), and an entry whose kind is
  not the one the reading declaration's profile serves (`clock-kind-mismatch`
  at `/package/lock/dependency_clock_selections/{i}/clock/kind`). Each is a
  `ReplayRefusal::Recompile` carrying the FR-640 cause in its details. A
  package whose entry disagrees with its own `temporal_bindings` row for one
  clock name is refused by the package reader (FSpec FR-322) before any
  recompile.
- The driver's `--clocks <json-file>` reads the file as UTF-8 JSON whose whole
  content is the `clocks` object. An unreadable file SHALL refuse `io-error`
  naming the path; a file that is not one JSON object SHALL refuse
  `invalid-request` naming `--clocks`; its members are then read exactly as a
  request's `clocks` member, with the same refusals. The flag's argument
  handling and tests are the driver's (FSpec FR-301); FSL's part is the
  member it fills.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-640-AC-1 | Through `command::compile`, a unit with three clauses over a record trace subject, one under each of event-position (clock `"events"`), fixed-sample (clock `"sample"`) and timestamped-event (clock `"plant-ms"`), and a `clocks` member configuring each, exits 0; the package holds three `temporal_bindings` rows whose `clock_name` and `clock` equal the members. | Test (TC-1660) |
| FR-640-AC-2 | Through `command::run`, the same unit with the same `clocks` runs each clause over a supplied trace of the matching kind (FR-641) and reports a disposition from evaluation, never a compile refusal. | Test (TC-1660) |
| FR-640-AC-3 | Removing the `"plant-ms"` member refuses at stage `emit`, `invalid-request`, exit 20, `details.cause` `missing-clock-selection`, `details.clock` `plant-ms`, with a message naming `plant-ms` and the locus of that clause's `clock` argument; adding a member `"spare"` that no clause writes refuses `unused-clock-selection`; configuring `"events"` as `timestamped_event` refuses `clock-kind-mismatch`. FR-330-AC-1's `Bounded` (event-position over the `Counter` model, clock `"model-steps"`) compiles with no `clocks` member and its row holds the derived `event_position` configuration. | Test (TC-1661) |
| FR-640-AC-4 | A `clocks` member with an unknown `kind`, a `fixed_sample` with `period` `{"kind": "integer", "decimal": "0"}`, and a request carrying `clocks` with a package selection each refuse `invalid-request` at stage `request`, exit 20, naming the member. | Test (TC-1661) |
| FR-640-AC-5 | Two compile requests differing only in the `"events"` `sequence_authority` give packages with different identities; two differing only in member order give equal bytes. | Test (TC-1660) |
| FR-640-AC-6 | A `clocks` member named `model-steps` or `model-time` refuses at stage `emit`, `invalid-request`, exit 20, `details.cause` `reserved-clock-name`, `details.clock` the name, whatever the unit declares: a model binding name is never a configurable clock role (FR-610). | Test (TC-1661) |
| FR-640-AC-7 | Through `command::compile` with the ConfigVersion model and no `clocks` member, a unit with one fixed-sample clause and one timestamped-event clause over `Config::ConfigVersion` on clock `model-steps` exits 0, and their `temporal_bindings` rows hold `{"kind": "fixed_sample", "epoch": {"kind": "integer", "decimal": "0"}, "period": {"kind": "integer", "decimal": "1"}, "unit": "step"}` and `{"kind": "timestamped_event", "timestamp_unit": "step"}`. | Test (TC-2181) |
| FR-640-AC-8 | The `clocks` object rebuilt from the `temporal_bindings` rows of the package FR-640-AC-1 compiles (each row's `clock_name` naming a member whose value is the row's `clock`) compiles the same program to byte-identical package bytes; this is the selection list the replay recompile builds from the proving package, so a replay recompiles under exactly the configuration the proof ran under. `ReplayRequestWire` has no `clocks` member. | Test (TC-2182) |
| FR-640-AC-9 | A request compiling a program and one library unit whose clause writes clock `plant-ms`, with one `clocks` member `plant-ms`, binds both clauses to that member and refuses no member as `unused-clock-selection`; without the member, the library clause refuses `missing-clock-selection` naming `plant-ms`. | Test (TC-2183) |
| FR-640-AC-10 | Through `command::compile`, a program that imports library `test/plant`, whose timestamped-event clause `Late` is on clock `lib-ms` and which no program declaration uses, compiled with `clocks` `{"lib-ms": {"kind": "timestamped_event", "timestamp_unit": "ms"}}`, exits 0 and its package lock holds `dependency_clock_selections` `[{"clock_name": "lib-ms", "clock": {"kind": "timestamped_event", "timestamp_unit": "ms"}}]`; with `timestamp_unit` `s` the package bytes differ; a program whose library declares no clause has no such member. | Test (TC-2212) |
| FR-640-AC-11 | The package of AC-10, replayed through `fsl_replay::replay` for its function `entry` with `test/plant` as its one `dependencies` entry and no clock in the request, recompiles and settles `reproduced-without-witness` with the value the in-process call gives. The same request with the member removed from the package refuses `ReplayRefusal::Recompile` with `missing-clock-selection` naming `lib-ms` at `/package/lock/dependency_clock_selections`; with an added entry `spare` it refuses `unused-clock-selection` at that entry; with the `lib-ms` entry a `fixed_sample` configuration it refuses `clock-kind-mismatch` at `/package/lock/dependency_clock_selections/0/clock/kind`. None yields a result. | Test (TC-2213) |

## Dependencies

- [FR-027](FR-027-export-compiled-native-package.md) and
  [FR-100](FR-100-run-a-named-function-through-the-spine.md): the request
  envelopes that carry `clocks`.
- FR-050 and FR-337: `TemporalClockSelection` and the `temporal_bindings` rows.
- [FR-517](FR-517-write-a-checked-package-handoff-directory-from-source-in-process.md):
  the in-process handoff writer, whose `--clock-selections` file is keyed by
  declaration name for that tool only.
- FSpec FR-090, FR-252, FR-540.
