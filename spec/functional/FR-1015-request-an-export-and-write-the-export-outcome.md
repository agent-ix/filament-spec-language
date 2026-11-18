---
id: FR-1015
title: "Request a TLA+ or Lean export from a run request and write the export outcome"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-029
    type: implements
  - target: ix://agent-ix/filament-specification/FR-817
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-811
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-299
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-756
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-255
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-286
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-810
    type: references
---
# FR-1015: Request a TLA+ or Lean export from a run request and write the export outcome

## Description

FSL SHALL read a `native-run/1` request's `export` member as FSpec FR-817's
export request and write FSpec FR-817's `filament.export-outcome/v1`
document as the run outcome's `export` member, applying FSpec FR-811's
fidelity floor and the `export.*` limits of FR-255, with FSpec FR-817's
exits. The same request file is the operand of the driver's `fsl export`
verb, whose FR-286 outcome document has operation `export` and carries the
export outcome as its result.

FSL writes the target file, as it writes the checked package for a compile
request's `output` member (FR-810): the request's `export.output` member and
the driver's `--output <file>`, which the driver hands to FSL as the request
override `RequestOverrides::output`, name the one file FSL writes. The driver
writes no target bytes of its own, so the two give byte-identical files.

## Behavior

- The `export` member SHALL carry `profile`, `target_family` (optional),
  `obligations`, `minimum_fidelity` (default `conditional`), `observer`
  (optional), `initial` (optional, a `filament.state.snapshot/v1` file
  resolved against the request file's directory), `limits` (optional) and
  `output` (optional, `{"file": P}`), as FSpec FR-817 states.
- `limits` SHALL be keyed by FR-255 setting name: `export.output_bytes`,
  `export.nodes` and `export.work_units`, each an unsigned integer used as
  given; an absent setting takes its FR-255 default. Any other name refuses
  `invalid-request` at `/request/export/limits/<name>` naming it, exit `20`.
- The driver's `--limit` reaches FSL as the request override
  `RequestOverrides::limits`. A setting given both in `export.limits` and in
  that override SHALL refuse `invalid-request` naming `limits` and the
  setting, exit `20`, before any stage runs, so one export never runs under
  two values of one limit; a setting given in only one of them takes that
  value.
- A request below its floor SHALL write `status` `refused` with
  `below-fidelity-floor`, no `target_text`, exit `21`.
- A denied export charge SHALL write `status` `incomplete` naming
  `export.<counter>`, exit `22`.
- An assembled export SHALL write FSpec FR-817's `package` member: the
  source package reference, the target profile, the generator identity and
  version, and the effective value of each of the three export limits.
- FSL SHALL write the target file only for an `assembled` status, after the
  text is complete, with no partial file. A file or directory already at the
  path refuses `invalid-request` naming the path, exit `20`, with
  `last_stage` null, and leaves it untouched. A request carrying
  `export.output` with an `output` override refuses `invalid-request` naming
  `output` and `--output`, exit `20`, and writes no file. The outcome's
  `output` member names the path as given and the bytes written.
- FSL SHALL lower every construct that FSpec FR-811's coverage table marks
  `preserved` or `conditional` for the selected target, and writes
  `target-unrepresentable` only for a construct that the table or FSpec
  FR-651 "Fidelity" marks `unrepresented`. The cause spells `target` as the
  family (`tla` or `lean`, not the profile), `construct` as the node's
  `node_tag`/`semantic_form`, `node` as its node id and `locus` as its
  region in FR-286's `{"kind": "region", "source", "start", "end"}` locus
  form. FSL has no record, cause or exit for a construct it has not lowered.
  Meeting one is a mapper failure, `internal-failure` with exit `30` and no
  package or file, so the floor refuses (exit `21`) only for constructs the
  target cannot state.
- `obligations` SHALL be a nonempty list of the program's declared
  obligations as FSpec FR-817 states: an absent member refuses
  `missing-member` and `[]` refuses `invalid-value` at
  `/request/export/obligations`, exit `20`. A `temporal` clause such as
  `SlowEnough` is named by its declared name. A `pre`, `post` or
  `constraint` clause, or a name that the program does not declare, gives a
  `refused` record with `not-an-obligation{name}` in request order, not a
  whole-request refusal.
- FSL SHALL stop in FSpec FR-817's order: admission, then classification
  (`export.nodes` and per-node `export.work_units`), then the floor, then
  the text (`export.output_bytes` and per-region `export.work_units`). A
  request below its floor exits `21` under any `export.output_bytes`.
- The `package` member's `generator` SHALL be `{"identity":
  "filament-spec-language", "version": V}`, with `V` the application
  version that `fsl version` and the build inventory report (FR-661). This
  is the same name and version pair as the inventory's `application`, read
  from the same constant, so one build names itself one way everywhere. It
  is not a crate's own version, and no `/export` suffix is added, because
  the target profile beside it already names the mapper.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-1015-AC-1 | Exporting `SlowEnough` over `Braking` under the default floor writes `status` `refused` with `below-fidelity-floor`, no `target_text`, exit `21`; under `minimum_fidelity` `any` it writes `assembled` with the `unrepresented` record claiming no region. | Test (TC-2865) |
| FR-1015-AC-2 | `limits` `{"export.output_bytes": 1}` writes `incomplete` naming `export.output_bytes`, exit `22`; `limits` `{"output_bytes": 1}` refuses `invalid-request` naming `output_bytes`, exit `20`; the same request with `RequestOverrides::limits` setting `export.output_bytes` refuses `invalid-request` naming `limits` and `export.output_bytes`, exit `20`, and the request without that `limits` entry under the override set to 1 writes `incomplete` naming `export.output_bytes`, exit `22`. | Test (TC-2865) |
| FR-1015-AC-3 | Exporting FR-756-AC-1's counter with `output` `{"file": "counter.tla"}` writes a file whose bytes equal the outcome's `target_text`, and an `output` member `{"path": "counter.tla", "bytes_written": N}` with `N` that text's length; the same request through `RequestOverrides::output` naming the same path in a fresh directory writes byte-identical bytes; a path that already exists refuses `invalid-request` naming it, exit `20`, `last_stage` null, bytes unchanged; the member and the override together refuse naming `output` and `--output`, exit `20`, with no file. | Test (TC-2992) |
| FR-1015-AC-4 | The counter export's `package` member is `{"source_package", "target_profile", "generator", "limits"}` with `limits` `{"export.output_bytes": 16777216, "export.nodes": 100000, "export.work_units": 16777216}`; two runs give equal members, and a run with `limits` `{"export.nodes": 99999}` gives a member that differs only in that limit. | Test (TC-2992) |
| FR-1015-AC-5 | The counter's `Bounded` exports `preserved` with no cause under the default floor, exit `0`. `SlowEnough` over `Braking` under `any` gives `target-unrepresentable` causes whose `target` is `tla`, whose `construct` holds a `/`, whose `node` is a node id of the package and whose `locus` has `kind` `region`. | Test (TC-3002) |
| FR-1015-AC-6 | `obligations` `[]` refuses `invalid-value` at `/request/export/obligations`, exit `20`. `["CanInc", "Bounded", "Nope"]` gives three records in that order, with `CanInc` and `Nope` `refused` with `not-an-obligation`, and exit `21`. | Test (TC-3002) |
| FR-1015-AC-7 | `SlowEnough` under the default floor with `limits` `{"export.output_bytes": 1}` refuses `below-fidelity-floor`, exit `21`. | Test (TC-3002) |
| FR-1015-AC-8 | The counter export's `generator` is `{"identity": "filament-spec-language", "version": V}` with `V` equal to `filament_spec_language::VERSION`. | Test (TC-3002) |

## Dependencies

- FSpec FR-817 (the export request, outcome and CLI verb), FSpec FR-811
  (the fidelity floor), FSpec FR-299 (the package key), FR-756 (the
  mappers), FR-255 (the `export.*` settings), FR-286 (the `export`
  operation's outcome document), FR-810 (the request override that names an
  output file).
