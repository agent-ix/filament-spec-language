---
id: FR-665
title: "Write and verify a checked-package handoff through the command"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-029
    type: traces_to
  - target: ix://agent-ix/filament-spec-language/FR-517
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-518
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-531
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-640
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-027
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-287
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-286
    type: references
  - target: ix://agent-ix/filament-specification/FR-301
    type: references
---
# FR-665: Write and verify a checked-package handoff through the command

## Description

FR-517's writer produces the checked-package handoff directory a downstream
consumer reads: the package, its sources and models, the selection sidecar,
the identity and equality files and, on request, the adverse mutation
corpus (FR-518). A user of the driver reaches it through two commands:

- **Write.** A `native-compile/1` request (FR-027) MAY carry an optional
  top-level `handoff` member. When it does, the compile also writes the
  handoff directory for the compiled unit, and its outcome document is the
  compile's.
- **Verify.** The library operation
  `fsl_command::checked_handoff::verify(directory, limits)` reads a handoff
  directory back: it reads `checked-package.json` with the strict reader,
  using the evidence the files beside it supply, and checks the admitted
  package against `selection.json` with `Selection::check` (FR-517). The
  driver's `inspect` verb calls it for `inspect --handoff <directory>`
  (FSpec FR-301), which settles one input form of `inspect` and leaves its
  other views to their own requirements.

The driver calls the command owner directly and never the FSL root package
(FR-287). The writer, `Selection` and the `emit_checked_v2` entry the
writer calls SHALL therefore be owned by `fsl-command`, present in its
default build, and the root package SHALL neither define nor re-export
them. Both commands are in the driver's default build, not behind a build
option.

## Inputs

The `handoff` member, a closed record:

- `directory`: the path of the new directory, resolved against the request
  file's directory;
- `package_name`: the package name FR-517's request takes;
- `execution_points`: optional object keyed by authored state-clause name,
  each value one FR-104 execution point in FR-105's closed form
  (`{"kind": "initialization", "name"}`, `{"kind": "handler", "name"}`,
  `{"kind": "pre", "operation"}` or `{"kind": "post", "operation"}`): the
  `StateClauseExecutionPoints` that `emit_checked_v2` takes (FR-517); absent
  is `{}`, and the FR-517 sidecar records the same object;
- `corpus`: optional, `"none"` or `"standard"` (FR-518); absent is `"none"`;
- `native_join`: required, `{"work_units": n, "storage": n}`, the identity
  file's native join budget (FR-531), which has no default.

The unit, libraries and domain package documents are the compile
request's `program`, `libraries` and `models`. The writer's clock
configurations are the compile request's `clocks` member
([FR-640](FR-640-select-clock-configurations-in-compile-and-run-requests.md)),
taken unchanged: each clock has one configuration, keyed by its clock name,
and the handoff carries no second clock selection of its own, so the
handoff and the package it holds can never configure one clock two ways.

## Outputs

For the write: the compile's outcome document (FR-286), and the directory
FR-517 lists. For the verify: one `filament-outcome/1` document with
operation `inspect`, category success when the package reads and matches
the sidecar, otherwise refusal carrying the reader's typed refusal or the
`SelectionRefusal` with its `SelectionAxis`.

## Behavior

- The request reader SHALL decode `handoff` under FR-100's closed-envelope
  rules; a missing `native_join`, an unknown member or a `corpus` value
  other than `none` or `standard` refuses `invalid-request` at stage
  `request`, exit 20, naming the member, before any file is read.
- A key of `execution_points` that names no authored state clause of the
  unit, or a value that is not one of FR-105's four closed forms, SHALL
  refuse `invalid-request` at stage `request`, exit 20, at
  `/handoff/execution_points/<name>`, and write no directory. An accepted
  point inapplicable to its clause keeps FR-104's refusal.
- If `directory` already exists, then the command SHALL refuse it naming the
  path, exit 20, leaving its contents untouched (FR-517).
- The command SHALL write the directory only after the package is built and
  read back; a compile refusal writes no directory.
- `verify` SHALL read only the directory's files.
- `verify` SHALL refuse a file with any changed byte that changes what the reader admits or what the
  sidecar records, with the typed refusal the reader or `Selection::check`
  gives.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-665-AC-1 | `fsl_command::command::compile` over a `native-compile/1` request for `tests/fixtures/spine-compile.native` carrying `handoff` `{"directory": "handoff", "package_name": "spine", "native_join": {"work_units": 1000000, "storage": 1000000}}` exits 0 and writes `handoff/` holding `checked-package.json` equal to the compile's package bytes, `selection.json` whose `format` is `filament.checked-handoff.selection/1`, `identities.json`, `equality.json` and `inventory.json`, and no `mutations/`. | Test (TC-1748) |
| FR-665-AC-2 | `verify` over AC-1's directory returns category success with operation `inspect`; over a copy whose `selection.json` names one node count more it refuses with the `SelectionRefusal` axis of that count; over a copy whose `checked-package.json` has one changed byte it refuses with the strict reader's typed refusal. | Test (TC-1748) |
| FR-665-AC-3 | AC-1's request without `native_join`, with `corpus` `"all"`, and with an unknown member `digest` each refuse `invalid-request` at stage `request` naming the member and write no directory; AC-1's request run a second time refuses naming the existing `handoff` path and leaves its files unchanged. | Test (TC-1748) |
| FR-665-AC-4 | A handoff request over `tests/fixtures/parent.native` with `execution_points` `{"CanAttempt": {"kind": "pre", "operation": "attempt"}}` writes a directory whose `selection.json` records that object, and the same request with `execution_points` absent records `{}`; with `{"Missing": {"kind": "pre", "operation": "attempt"}}`, and with `{"CanAttempt": {"kind": "loop", "name": "x"}}`, it refuses `invalid-request` at stage `request` at `/handoff/execution_points/Missing` and `/handoff/execution_points/CanAttempt`, exit 20, and writes no directory. | Test (TC-2627) |

## Dependencies

- [FR-517](FR-517-write-a-checked-package-handoff-directory-from-source-in-process.md), [FR-518](FR-518-write-the-adverse-mutation-corpus-from-checked-packages-and-name-the-destination-of-every-legacy-case.md), [FR-531](FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md): the writer, the corpus and the identity file.
- [FR-027](FR-027-export-compiled-native-package.md): the compile request.
- [FR-287](FR-287-reach-fsl-through-the-driver-cli.md): the driver reaches the command owner, never the root.
- FSpec FR-301: the `inspect --handoff` form.
