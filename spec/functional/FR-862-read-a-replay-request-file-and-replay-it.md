---
id: FR-862
title: "Read a replay request file and replay it"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-033
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-071
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-072
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-098
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-640
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-644
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-285
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-620
    type: references
  - target: ix://agent-ix/filament-specification/FR-715
    type: implements
  - target: ix://agent-ix/filament-specification/FR-301
    type: implements
  - target: ix://agent-ix/filament-specification/FR-336
    type: references
---
# FR-862: Read a replay request file and replay it

## Description

`fsl_command::command::run` SHALL read a `native-replay/1` request file and
replay it through `fsl_replay::replay` (FR-098), as it reads a
`native-monitor/1` file and runs its record stream (FR-644). The driver's
`fsl replay <request-file> [--format]` verb (FSpec FR-301) calls this entry;
argument parsing and stream writes stay in the driver (ADR-011), and the
request reader, its refusals and the result are the language's.

### Decision: one request encoding, read by `command::run`

The replay request already has one JSON encoding of every
`ReplayRequestWire` field: FSpec FR-336 "Replay request file", which the
conformance corpus uses (FR-350). The product request file is a `native-replay/1` file `{format, request}`
whose `request` carries that same mapping (FSpec FR-715, FR-336), not an
FSpec FR-323 envelope, so a corpus vector's
request and a user's request are read by one reader, and no second encoding
of the replay request is defined. `command::run` dispatches on the
envelope's `format`, so every request file kind is read by one entry, as
`native-monitor/1` already is.

A replay request carries no `clocks` member: the replay recompile reads the
clock configurations from the proving package (FR-640), so a replay always
runs under the configuration the proof ran under (FSpec FR-715).

## Inputs

A request file `{"format": "native-replay/1", "request": R}`, where `R` is a
closed object whose members are exactly FSpec FR-336's "Replay request file"
top-level members (`profile_selections`, `package`, `dependencies`,
`replay_target`, `source`, `obligation_identity`, `backend`,
`state_environment`, `accounting_limits`, `stage_limits`,
`supplied_limits`, `object_environment`, `declared_domains`,
`byte_provision`), each with that mapping's value. Each `byte_provision` row's
`bytes_path` is resolved against the request file's directory, under the
file-count and byte ceilings `command::run` applies to a `native-run/1`
request's files (`RunLimits`).

## Outputs

The outcome document for operation `replay`, holding FR-072's typed replay
result; the exit code is FR-285's for its category.

## Behavior

- `command::run` SHALL read a request whose `format` is `native-replay/1`
  as a replay request, decode `R` into `ReplayRequestWire` by FSpec FR-336's
  mapping, construct the request by FR-071 and replay it by FR-098.
- The reader SHALL refuse at stage `request`, `invalid-request`, naming the
  member, before any replay: a `clocks` member of `R` (FR-640: replay reads
  its clocks from the proving package), any other member outside the list
  above, and a missing member the mapping requires (the first absent one in the
  order of the list above, `profile_selections` first). A `byte_provision` file that is missing or exceeds the ceilings is
  refused at stage `request` naming its `bytes_path`.
- A request that decodes but that FR-071 refuses at construction SHALL be
  reported with FR-071's code and cause at stage `request`; a request that
  FR-098 refuses is reported with FR-098's refusal, and a replay that runs
  is reported with FR-072's result. None of them is reported as a format
  error.
- The analyze reader SHALL refuse a `native-replay/1` file, and the replay
  reader an analyze document, with `invalid-request` naming `format`, as FR-620 refuses a mismatched analyze document.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-862-AC-1 | Through `command::run`, a `native-replay/1` request whose `R` is a complete FSpec FR-336 replay request with an added member `clocks` refuses at stage `request`, `invalid-request`, exit 20, naming `clocks`, with no result on stdout; the same request with an added member `extra` refuses naming `extra`; a `native-replay/1` request with an empty `R` refuses naming the first required member. None is refused as an unknown request format. | Test (TC-2443) |
| FR-862-AC-2 | Through `command::run`, the `native-replay/1` request carrying FR-640-AC-11's replay of the function `entry` (its package with `test/plant` as its one `dependencies` entry and its sources in `byte_provision` files) settles `reproduced-without-witness` with the value the in-process `fsl_replay::replay` gives, exit 0; the same request with the package's `lib-ms` `dependency_clock_selections` entry removed refuses `missing-clock-selection` as FR-640-AC-11 states. | Test (TC-2443) |

## Dependencies

- [FR-071](FR-071-implement-typed-replay-request.md),
  [FR-072](FR-072-implement-typed-replay-result.md),
  [FR-098](FR-098-execute-a-replay-request.md): the request, the result and
  the replay.
- [FR-640](FR-640-select-clock-configurations-in-compile-and-run-requests.md):
  replay reads its clocks from the package.
- [FR-644](FR-644-stream-observations-through-the-monitor-command.md): the
  `native-monitor/1` dispatch this follows.
- FSpec FR-715 (no clock configuration on a replay request), FR-301 (the
  `replay` verb), FR-336 (the replay request encoding).
