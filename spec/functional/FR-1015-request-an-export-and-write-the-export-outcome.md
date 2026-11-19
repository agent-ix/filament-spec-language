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
  `export.<counter>`, exit `22`. The outcome carries FSpec FR-817's `cause`
  `limit-reached{limit, value, setting}`: `limit` the counter
  (`output_bytes`, `nodes` or `work_units`), `value` its effective bound
  and `setting` its FR-255 name, with `records` `[]`.
- FSL SHALL charge FSpec FR-817's counters as that requirement counts
  them. Classification charges one `export.nodes` unit and one
  `export.work_units` unit for each distinct node of the union of the
  requested obligations' dependency closures, so a node that two closures
  share is read, and charged, once. A node's `export.nodes` charge is made
  before its `export.work_units` charge, so a node that would exceed both
  names `export.nodes`. Writing the text charges one `export.work_units`
  unit for each output region it writes, beside the `export.output_bytes`
  charge for its bytes.
- An *output region* SHALL be one top-level definition the text writes:
  each datatype, variable declaration, function or operator, axiom or
  assumption, and assertion (an obligation's proposition or theorem) that
  the export emits, which is each definition FSpec FR-814 precedes with one
  `fsl:` comment line. The module header, the namespace lines, comments and
  the lines inside one definition are not regions of their own. FSL SHALL
  charge exactly one `export.work_units` unit for each such definition, in
  the order the text writes them, before it writes that definition, so a
  text of N definitions charges N units for writing and the charge that
  exceeds the effective bound is the one for the definition that would
  have been written next.
- An assembled export SHALL write FSpec FR-817's `package` member: the
  source package reference, the target profile, the generator identity and
  version, and the effective value of each of the three export limits. The
  source package reference is `{"identity", "version"}`: over a `program`,
  the identity the program source declares and the program selection's
  `formal_revision`; over a `package` (FR-1039), the request's
  `package.source_package` exactly as given, which an export over a package
  must carry (FR-1039 "The source package reference of a package export").
  FSL never derives the reference from a package's bytes.
- FSL SHALL write the target file only for an `assembled` status, after the
  text is complete, with no partial file. A file or directory already at the
  path refuses `invalid-request` naming the path, exit `20`, with
  `last_stage` null, and leaves it untouched. A request carrying
  `export.output` with an `output` override refuses `invalid-request` naming
  `output` and `--output`, exit `20`, and writes no file. The outcome's
  `output` member names the path as given and the bytes written. A file
  that cannot be written after the export assembled (its parent directory
  missing, permission denied, the device full) is FR-286's output failure:
  `output-failure`, internal-failure, exit `30`, `last_stage` the stage the
  export reached, with no partial file.
- FSL SHALL lower every construct that FSpec FR-811's coverage table marks
  `preserved` or `conditional` for the selected target, and writes
  `target-unrepresentable` only for a construct that the table or FSpec
  FR-651 "Fidelity" marks `unrepresented`. The cause spells `target` as the
  family (`tla` or `lean`, not the profile), `construct` as the node's
  `node_tag`/`semantic_form`, `node` as its node id and `locus` as its
  region in FR-286's `{"kind": "region", "source", "start", "end"}` locus
  form. `node` SHALL be a JSON string holding the canonical text of the
  node's `CheckedNodeId` (ADR-036 PA-24: the RFC 8785 text of its V2 wire
  object), the same text the outcome's `name_map` entries and the FSpec
  FR-814 `fsl:` comments use, so a reader matches a cause to a name-map
  entry or a comment by string equality. `locus` SHALL be the first region
  the package's source map gives that node, and `null` when the source map
  gives it none, as FR-286 writes an unlocated source refusal; FSL never
  writes another node's region or an offset-zero region in its place (FSpec
  FR-203 "Absence"). FSL has no record, cause or exit for a construct it has not lowered.
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
- A malformed `export` member SHALL refuse with code
  `invalid_runtime_input` and one of FSpec FR-805's causes
  (`missing-member`, `unknown-member`, `duplicate-member`,
  `wrong-value-kind`, `invalid-value`) as the diagnostic's own `code` and
  `cause` members, `details` `{"path": P}` with `P` the member's pointer
  under `/request/export`, exit `20`. These are the FR-286 and FSpec
  FR-301 code and cause pairs every other run-request member refuses with,
  so a caller tells an absent obligation set from an empty one by the
  `cause` member, not by reading the message. `invalid-request` remains the
  code of a request whose members are each well formed but that contradicts
  itself or the file system: a setting given in both `export.limits` and
  `RequestOverrides::limits`, `export.output` beside
  `RequestOverrides::output`, an existing output path and an unknown
  `limits` setting name.
- The `observer` member names one of FSpec FR-817's structural observers
  (`sany`, `tlc`, `apalache`, `lean`). A request naming an observer this
  build has no backend for SHALL refuse `unsupported_projection`/
  `unsupported-requested-capability`, naming the observer, with `details`
  `{"path": "/request/export/observer"}`, category unsupported, exit `21`,
  `last_stage` null, no package and no file. A request naming an observer
  the build runs but whose tool the host does not provide SHALL refuse
  `unsupported_projection`/`tool-unavailable` naming the observer and the
  tool, exit `21`, likewise. Neither is `invalid-request`: the request is
  well formed and the build or host lacks the capability, which is FSpec
  FR-272's `unsupported_projection` case and FR-290's negotiation refusal,
  as FR-286 settles an unbuilt engine `unsupported` rather than refusing
  the request. The refusal comes after admission and before
  classification, so a caller who asked for an observer gets its run or a
  refusal naming it, never a package without it.
- The export outcome's `refusal` member for an observer this build has no
  backend for SHALL be the closed object `{"code":
  "unsupported_projection", "cause": "unsupported-requested-capability",
  "capability": "observer", "observer": O, "reason": R, "details":
  {"path": "/request/export/observer"}}`, with `O` the requested observer
  name as given and `R` a non-empty human-readable string. `capability`
  names the kind of capability the build lacks, so a caller tells a missing
  observer backend from any other missing capability by that member, not by
  reading `reason`; `reason` is display text only and no caller decision
  rests on it. The outcome document's diagnostic carries the same `code`,
  `cause` and `details`.
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
| FR-1015-AC-9 | The counter request with `obligations` `[]` refuses with `code` `invalid_runtime_input`, `cause` `invalid-value` and `details.path` `/request/export/obligations`, exit `20`; without the member, `cause` `missing-member` at the same path; with `minimum_fidelity` `7`, `cause` `wrong-value-kind` at `/request/export/minimum_fidelity`. None of the three has `code` `invalid-request`. | Test (TC-3020) |
| FR-1015-AC-10 | `SlowEnough` over `Braking` under `minimum_fidelity` `any` with `observer` `tlc`, in a build with no TLC backend, refuses `unsupported_projection`/`unsupported-requested-capability` naming `tlc`, exit `21`, with no `target_text`, no `package` and no file at its `output` path; its `export.refusal` holds `code` `unsupported_projection`, `cause` `unsupported-requested-capability`, `capability` `observer`, `observer` `tlc`, a non-empty string `reason` and `details` `{"path": "/request/export/observer"}`, and no other member; with `limits` `{"export.nodes": 1}` added it refuses the same way, exit `21`, not `incomplete`; the same request without `observer` assembles, exit `0`. | Test (TC-3020) |
| FR-1015-AC-11 | The counter export under `export.work_units` 1 writes `status` `incomplete`, `records` `[]` and `cause` `{"type": "limit-reached", "limit": "work_units", "value": 1, "setting": "export.work_units"}`, exit `22`; under `export.nodes` 1 its `cause` names `nodes`, value 1 and `export.nodes`, though `export.work_units` 1 is also set. | Test (TC-3020) |
| FR-1015-AC-12 | Over the spine unit with invariants `Low` and `High` over `Config::ConfigVersion`, let `n(S)` be the least `export.nodes` under which the export of obligations `S` is not `incomplete`. Then `n(["Low", "High"])` is less than `n(["Low"]) + n(["High"])` and at least the greater of the two. | Test (TC-3020) |
| FR-1015-AC-13 | Every `target-unrepresentable` cause of `SlowEnough` over `Braking` under `any` has a string `node` that reads back as a `CheckedNodeId` of the package and re-encodes to the same text, and a `locus` that is either `null` or a region with `start < end`; a cause's `node` equals the `node` of the `name_map` entry, if one exists, for the same node. | Test (TC-3020) |
| FR-1015-AC-14 | Over the counter's `Bounded` export to TLA+ and to Lean, let N be the number of `fsl:` comment lines of its `target_text` (one per top-level definition) and C the least `export.nodes` under which the export is not `incomplete` (its classification charge). Under `export.work_units` C + N the export is `assembled`, exit `0`. Under `export.work_units` C + N - 1 it is `incomplete` with `records` `[]` and `cause` `{"type": "limit-reached", "limit": "work_units", "value": C + N - 1, "setting": "export.work_units"}`, exit `22`; the denied charge is the one for the Nth definition. An implementation that charges per byte, per line, per obligation or per node mapped while writing, rather than per definition, assembles or stops at a different bound and fails this criterion. | Test (TC-3020) |

## Dependencies

- FSpec FR-817 (the export request, outcome and CLI verb), FSpec FR-811
  (the fidelity floor), FSpec FR-299 (the package key), FR-756 (the
  mappers), FR-255 (the `export.*` settings), FR-286 (the `export`
  operation's outcome document), FR-810 (the request override that names an
  output file).
