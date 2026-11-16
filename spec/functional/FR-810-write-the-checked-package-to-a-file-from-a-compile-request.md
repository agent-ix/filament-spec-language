---
id: FR-810
title: "Write the checked package to a file from a compile request or fsl compile"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-029
    type: traces_to
  - target: ix://agent-ix/filament-spec-language/FR-027
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-287
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-665
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-286
    type: references
  - target: ix://agent-ix/filament-specification/FR-322
    type: references
  - target: ix://agent-ix/filament-specification/FR-301
    type: references
---
# FR-810: Write the checked package to a file from a compile request or fsl compile

## Description

A `native-compile/1` request MAY carry an optional top-level `output`
member, and the driver's `fsl compile` verb MAY carry the option
`--output <file>`. Either SHALL make the compile write the checked package,
the exact FSpec FR-322 `filament.checked-package/v2` bytes FR-027's
`CompileOutcome::package_bytes()` returns, to the named file. A consumer
that needs the package as a file, such as a formal-rule lane that reads the
package from disk, then gets it from the command line with no library call
and no handoff directory.

### Decision: a named output file, not an `artifacts` entry

The outcome document's `artifacts` member stays the empty array FR-286
gives every operation. The outcome names no package identity, because
checking mints no `package_id` (ADR-029 OP-1), and a path the caller chose
needs no echo. An output path keeps the bytes on disk under the caller's
name, as FR-665's `handoff` member does for the full handoff directory, and
the two compose: `output` writes the one package file and `handoff` writes
the directory around it. A request member makes a request file
self-contained for repeated runs; the driver option serves a caller that
reuses one request with different destinations. The option and the member
are one writer, so a run that names both is refused instead of choosing
between two destinations.

## Inputs

- `output`: a closed record `{"package": P}`, `P` a non-empty path of the
  file to create, resolved against the request file's directory.
- The driver option `--output <file>` on `fsl compile`, resolved against the
  working directory; the driver hands it to the library writer below.

## Outputs

- The file at `P` holding exactly the package bytes, with no wrapper and no
  trailing newline (FR-027 "Outputs").
- The compile's outcome document (FR-286), unchanged by the write:
  `artifacts` stays `[]`.

## Behavior

- The request reader SHALL decode `output` under FR-100's closed-envelope
  rules. An unknown member, a missing or empty `package`, or a `package`
  that is not a string refuses `invalid-request` at stage `request`, exit
  20, naming the member, before any source file is read.
- The command SHALL write the file only after the package is built and read
  back. A compile refusal at any stage writes no file.
- If a file or directory already exists at the path, then the command SHALL
  refuse `invalid-request`, exit 20, naming the path, and leave it
  untouched, as FR-665 does for an existing handoff directory: a compile
  never overwrites a file it did not create.
- If the path's parent directory does not exist or the write fails, then
  the command SHALL report the I/O failure with its original code, exit 20
  (FR-027), naming the path, and SHALL leave no partial file: it writes a
  sibling temporary file and renames it into place.
- The library entry is FR-640's `compile_outcome_with_overrides(path,
  overrides)`, whose `RequestOverrides` SHALL gain `output: Option<&Path>`,
  the request member supplied beside the request file, as `clocks` is. A
  request that carries `output` and overrides that also supply it SHALL
  refuse `invalid-request` at stage `request`, exit 20, naming both `output`
  and `--output`, and write nothing, as a doubly supplied `clocks` does.
- The driver SHALL pass `--output` to that entry and write no package bytes
  itself, so the request member and the option produce byte-identical files.
  `--output` on any verb other than `compile` is an unknown option under
  FSpec FR-301.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-810-AC-1 | `fsl_command::command::compile` over a `native-compile/1` request for `tests/fixtures/spine-compile.native` carrying `output` `{"package": "spine.package.json"}` exits 0, returns package bytes equal to the bytes of the written file `spine.package.json` beside the request, and its outcome document has `artifacts` `[]`. | Test (TC-2297) |
| FR-810-AC-2 | AC-1's request run a second time refuses `invalid-request`, exit 20, naming the existing path and leaving its bytes unchanged; a request whose program does not check carries `output` and writes no file; `output` `{"package": ""}`, `{"path": "x"}` and `{"package": 3}` each refuse `invalid-request` at stage `request` naming the member, and write no file; a `package` path under a missing directory fails with its I/O code, exit 20, and leaves no file. | Test (TC-2297) |
| FR-810-AC-3 | `compile_outcome_with_overrides` over AC-1's request without `output` and with `RequestOverrides::output` `Some("cli.package.json")` writes a file byte-equal to AC-1's; over AC-1's request (which carries `output`) with `Some(..)` it refuses at stage `request` `invalid-request` naming `output` and `--output` and writes neither file. The driver's `fsl compile <request> --output <file>` writes the same bytes. | Test (TC-2297) |

## Dependencies

- [FR-027](FR-027-export-compiled-native-package.md): the compile request and
  `CompileOutcome`.
- [FR-287](FR-287-reach-fsl-through-the-driver-cli.md): the driver's verbs
  and options.
- [FR-665](FR-665-write-and-verify-a-checked-package-handoff-through-the-command.md):
  the handoff directory, whose existing-path rule this one follows.
- FSpec FR-322: the package bytes; FSpec FR-301: the CLI's option refusal.
