---
id: FR-1047
title: "Name every missing request member and every accepted format, and return a request example"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-029
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-026
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-027
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-100
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-312
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-620
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-644
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-862
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-1015
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-1022
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-031
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-096
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-640
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-1039
    type: references
  - target: ix://agent-ix/filament-specification/FR-562
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-660
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-662
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-661
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-286
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-287
    type: references
---
# FR-1047: Name every missing request member and every accepted format, and return a request example

## Description

A caller who has only FSL's refusals cannot learn what a request document
holds. Today a request missing members is refused for the first one serde
meets, with a line and column and no member path; a `format` an entry does
not read is refused `unsupported native command format` without saying what
the entry reads; an export request without its `export` member is refused
`invalid clause request member export: the selected export invocation
requires export`, which names the member twice and its shape not at all; and
there is no way to obtain a valid request to start from.

This requirement fixes those refusals for every request entry of
`fsl_command::command`, and adds the library entry `request_example`, which
returns a minimal request document for each request format. The driver's
`--example` option (driver FR-028) prints what this entry returns.

The members each format carries, their spellings and their values stay with
the requirement that owns the format: FR-100 and FR-312 (`native-run/1` read
by the run entries), FR-1015 and FR-1022 (`native-run/1` read by the export
entry), FR-027 (`native-compile/1`), FR-620 (`native-analyze/1`), FR-644
(`native-monitor/1`) and FR-862 (`native-replay/1`). This requirement owns
how a reader reports which members are missing and which formats it reads,
the member order it reports them in, and the example entry.

## Inputs

- A request file handed to one of the request entries below.
- For `request_example`: a request format identifier and an optional export
  selection.

### Request entries and the formats they read

| Entry family | Entries | Formats read |
| --- | --- | --- |
| run | `run`, `run_with_overrides`, `run_with_limits`, `run_with_limits_and_overrides`, `run_with_providers`, `run_with_providers_and_limits_and_overrides` | `native-run/1`, `native-monitor/1`, `native-replay/1` |
| run-reader | `prepare_run`, `prepare_run_with_overrides`, `read_clause_run_request` | `native-run/1` |
| compile | `compile`, `compile_outcome`, `compile_with_overrides`, `compile_outcome_with_overrides`, `compile_outcome_with_limits_and_overrides` | `native-compile/1` |
| analyze | `analyze`, `analyze_with_limits`, `prepare_analyze`, `analyze_with_providers`, `analyze_with_providers_and_limits` | `native-analyze/1` |
| monitor | `monitor` | `native-monitor/1` |
| export | `export_with_limits_and_overrides` | `native-run/1` |

There is no `native-export/1` format: the export entry reads a `native-run/1`
request carrying an `export` member, or with `ExportSelection::Evidence` an
`export_evidence` member.

## Outputs

- A request-stage refusal: code `invalid-request`, stage `request`, exit
  20, with the diagnostic `message` and `details` this requirement states.
- From `request_example`: the text of one JSON request document, or a
  `RequestExampleError`.

## Definitions

- **Reading**: the reader that reads a document, chosen by the entry family
  and the document's `format`. A `native-run/1` request has four readings:
  *run* (the run entries; also the run-reader entries), *export* (the export
  entry with `ExportSelection::Ordinary`, or a run entry over a request
  carrying `export`), *evidence* (the export entry with
  `ExportSelection::Evidence`, or a run entry over a request carrying
  `export_evidence`), and the other member readings named under One-of
  group. A `native-monitor/1` request has two readings: *monitor-run* (a
  run entry, which runs the request's `records` file and requires it,
  FR-644) and *monitor* (the monitor entry, which reads the stream its
  caller gives and refuses a `records` member, FR-644). Each other format has
  one reading.
- **Required member**: a member the reading refuses the request for lacking,
  that has no default and belongs to no one-of group.
- **One-of group**: members of which the reading requires exactly one. The
  groups are exactly these:

  | Reading | Groups |
  | --- | --- |
  | run | `program` / `package` (FR-100); `call` / `clause` / `items` (FR-100) |
  | export | `program` / `package` (FR-1015, FR-100) |
  | analyze | `program` / `package` (FR-620) |
  | evidence, compile, monitor-run, monitor, replay | none |

  A run-reading request carrying `tl_lowering`, `synthesis`,
  `verification_plan`, `model_based_test` or `refinement_transfer` is read
  by that member's requirement and does not lack the `call` / `clause` /
  `items` group. Alternatives inside a member's own value, such as FR-312's
  `clause` selection forms or FR-1039's `package` forms (`{file}` or
  `{directory}`), are not one-of groups of this requirement: they belong to
  the member's subtree (Scope of the list).
- **Missing entry**: a missing required member, written as its RFC 6901 JSON
  pointer from the document root (`/request/models`), or a missing one-of
  group, the array of its members' pointers in the order the table lists
  them.
- **Scope of the list**: the members whose absence this requirement lists
  are those of the envelope; of the `request` object; and, inside a present
  member, of `program`, of each `models` entry, of each `libraries` entry, of
  each of their `source` selections, of `call`, and of each `clauses` entry
  and of `trace` of `native-monitor/1`. For `native-replay/1`, the scope is
  the envelope and the fourteen top-level FSpec FR-336 members of `request`.
  Every other subtree (`export`, `export_evidence`, `package`, `clause`,
  `items`, `subject`, `requirements`, the member readings above, and the
  FR-336 members' interiors) is read by its owning requirement, which keeps
  its own code, cause and pointer for a missing member inside it; this
  requirement does not change those refusals.
- **Member order**: the order of the members of each object, used both by
  the example and by the missing-entry list:

  | Object | Member order |
  | --- | --- |
  | envelope | `format`, `request` |
  | `request`, run reading | `models`, `program` \| `package`, `call` \| `clause` \| `items` |
  | `request`, export reading | `program` \| `package`, `export` |
  | `request`, evidence reading | `models`, `export_evidence` |
  | `request`, `native-compile/1` | `program` |
  | `request`, `native-analyze/1` | `program` \| `package` |
  | `request`, monitor-run reading | `models`, `program`, `clauses`, `trace`, `records` |
  | `request`, monitor reading | `models`, `program`, `clauses`, `trace` |
  | `request`, `native-replay/1` | FR-862's list, `profile_selections` to `byte_provision` |
  | `program` | `source` |
  | source selection | `file`, `authority`, `identity`, `document`, `formal_revision` |
  | `models` entry | `format`, `source` |
  | `libraries` entry | `identity`, `source` |
  | `call` | `function`, `arguments` |
  | `clauses` entry (monitor) | `name` |
  | `trace` (monitor) | `kind` |

  A group takes its row position. The missing-entry list walks this order
  depth first: an object's own missing entries in its row order, each
  present in-scope member's missing entries immediately after that member's
  position, and the entries of an array in index order.

## Behavior

### Reading the envelope

- A document carrying an `argv` member is an argument document of FR-661 or
  FSpec FR-562 (`check --extensions`, through FR-662 and FR-660), not a
  request envelope; the run entries read it as those
  requirements state and this requirement does not apply to it.
- The entry SHALL read the envelope before `request` (FR-026). When
  `format` is a string, the entry SHALL apply the format rule below before
  it checks `request`, so `{"format":"bogus/1"}` is refused for its format.
  When `format` is absent, the entry SHALL list as missing only `/format`
  and, if absent, `/request`.
- The run entries SHALL keep dispatching on `format`: a `native-monitor/1`
  request is read by FR-644 and a `native-replay/1` request by FR-862.

### Missing members

- When a request document's only decode defects within the scope of the
  list are missing required members and missing one-of groups, the entry
  SHALL refuse `invalid-request` at stage `request`, exit 20, before any
  file the request names is read, with one diagnostic that lists every
  missing entry at once in member order:
  - `details` SHALL be `{"missing": [...]}`, one element per entry in that
    order: the pointer string for a required member, the array of pointer
    strings for a group;
  - `message` SHALL be `invalid request: missing required member ` followed
    by the one entry, or `invalid request: missing required members `
    followed by the entries joined by `, `, a group written as its pointers
    joined by ` | `, except for the request "A selected source with no run
    member" states.
- `details` is the authoritative list. Pointer segments are escaped as RFC
  6901 states (`~0`, `~1`) and nothing else, so a caller-chosen key holding
  `, ` or ` | ` makes `message` ambiguous but never `details`; a caller reads
  the members from `details`, never from `message`.
- The message and details of this refusal SHALL NOT carry a line or a
  column. A document that is not JSON keeps its line and column (FR-026).
- A request that also carries an unknown member, a member of the wrong JSON
  kind or a repeated member is refused for that defect as its owning
  requirement states; this requirement adds no ordering among those defects.

### A selected source with no run member

- When a run-reading request selects exactly one of `program` and
  `package`, and its only missing entry is the `call` / `clause` / `items`
  group, the entry SHALL refuse as "Missing members" states, with the same
  code, stage and exit, `details` `{"missing": [["/request/call",
  "/request/clause", "/request/items"]]}`, and `message` exactly
  `a 1-draft program runs through the spine; the request selects no call`
  (FR-100-AC-25), in place of the missing-members message. The entry reads
  no program, package or other named file, so it does not read the declared
  edition first.
- A run-reader entry reads a request carrying `export` or `export_evidence`
  as the run reading (Definitions), so neither member exempts it.
- If a row of the table below applies to such a request at an entry the row
  names, then the entry SHALL give that row's refusal or limit outcome in
  place of this one, with its own code, stage, exit, message and `details`,
  no `missing` list, and no named file read. This requirement adds no
  ordering among the rows.

  | Request | Entries | What comes first | Criterion |
  | --- | --- | --- | --- |
  | either source | each run and run-reader entry that takes the limit | the request size limit (FR-026) | FR-1047-AC-11 |
  | either source | each run and run-reader entry | a member the typed request reader refuses while decoding (unknown, repeated, of the wrong JSON kind, or a value the decoder rejects); request-level `requirements` without `verification_plan`; a caller-supplied `clocks` override that FR-640 refuses | FR-1047-AC-11 |
  | `program` | each run and run-reader entry | `sources` (FR-100-AC-23) | FR-1047-AC-12 |
  | `package` | each run and run-reader entry | `sources` that FR-1039's caller source table refuses; `integer_division`, `libraries` or `refinement_internals` beside `package` (FR-1039); `clocks` beside `package` (FR-640) | FR-1047-AC-13 |
  | `program` | the run entries, for a request without `program.extraction`; `prepare_run` and `prepare_run_with_overrides`, for every request | `refinement_internals` without `package`; an `integer_division` value, on the request or on a library, that FR-767 refuses; an extension entry FR-660 refuses from the request alone; more selected files than the FR-026 file ceiling admits; a `program.source` `document` or `formal_revision` its constructor refuses (`invalid-identifier`); a model in a format other than `semantic-ir/2.0.0` and `native-model-unit/1` (FR-100) | FR-1047-AC-14 |
  | `program` with `program.extraction`, in a build with `quire-extraction` | the run entries and `read_clause_run_request` | `extraction-clause-count` (FR-031) | FR-1047-AC-15 |

- A provider that `run_with_providers` cannot register is FR-961's refusal.
  This requirement neither states nor changes its order against a request
  refusal, this one included.
- Every other check comes after this refusal, so this refusal is given in its
  place (FR-1047-AC-16). That includes every check that reads a named file,
  the declared edition among them; over a `package` request, the checks of
  the `package` selection's own members and of model formats;
  `extraction-clause-count` at `prepare_run` and
  `prepare_run_with_overrides`; and, at `read_clause_run_request` and at a
  run entry over a request with `program.extraction`, the checks of the row
  of the table that names `refinement_internals` without `package`.
- A run-reading request that carries none of `call`, `clause` and `items`,
  and that also lacks another entry or selects both or neither of `program`
  and `package`, is refused with the missing-members message and the full
  list.

### A format the entry does not read

- When the request's `format` is a string that is not among the formats its
  entry family reads (table above), including a string that names no FSL
  format, the entry SHALL refuse `invalid-request` at stage `request`, exit
  20, before reading `request`, with `message`
  `invalid request: format <F> is not read by <family>; <family> reads <A>`,
  where `<F>` is the format read, `<family>` the entry family and `<A>` the
  formats it reads joined by `, ` in the table's order, and `details`
  `{"path": "/format", "accepted": [<A as strings>]}`. No entry refuses a
  request format `unknown_wire`; `unknown_wire` stays the code of a document
  format that is not a request envelope (FR-096, FR-1039).

### The export entry's selected member

- When the export or evidence reading lacks its member (`/request/export`
  for export, `/request/export_evidence` for evidence), that member SHALL be
  a missing required member, listed as above, and the message SHALL be the
  missing-members message followed by `: ` and the member's shape sentence:
  - export: `export reads a native-run/1 request whose request holds an export object with at least profile and obligations`
    (FR-1015);
  - evidence: `export evidence reads a native-run/1 request whose request holds an export_evidence object with package, obligation, tool and artifact`
    (FR-1022).

  The text `the selected export invocation requires` SHALL NOT appear in any
  refusal.

### A request that is not an object

- If the export entry reads an envelope whose `request` member is not a JSON
  object, then the entry SHALL refuse `invalid-request` at stage `request`,
  exit 20, with message `invalid clause request member /request: expected an
  object` and `details` `null`.

### The request example entry

- `fsl_command::command::request_example` SHALL have the signature

  ```rust
  pub fn request_example(
      format: &str,
      selection: Option<ExportSelection>,
  ) -> Result<String, RequestExampleError>
  ```

  where `ExportSelection` is the existing enum the export entry takes.
- `request_example(F, None)` for each `F` of `native-run/1`,
  `native-compile/1`, `native-analyze/1`, `native-monitor/1` and
  `native-replay/1` SHALL return the example of `F`'s reading (for
  `native-run/1`, the run reading; for `native-monitor/1`, the monitor
  reading), and
  `request_example("native-run/1", Some(Ordinary))` and `Some(Evidence)` the
  examples of the export and evidence readings.
- Each example SHALL be one JSON object whose members are exactly `format`,
  equal to `F`, and `request`. Every object in it within the scope of the
  list SHALL carry exactly its required members and, for each of its one-of
  groups, the first member of the group, in member order; each in-scope
  array member (`models`, `call.arguments`, `clauses`) is `[]`. Every value
  in the example, in scope or not, SHALL pass the value checks its reader
  applies before it reads a file: labels and names are nonblank, and
  `formal_revision` is `1`. Outside the scope: the `export` member holds
  exactly `profile` `filament.output.tla2/v1` and `obligations` holding one
  nonblank name; the `export_evidence` member holds exactly `package` `{}`,
  a nonblank `obligation`, `tool` `tlc` and a nonblank `artifact`; and the
  `native-replay/1` example's `package`, `replay_target` and
  `byte_provision` are the members of the replay example below. Files it
  names are relative paths that need not exist.
- The `native-replay/1` example's `package`, `replay_target` and
  `byte_provision` members SHALL be exactly those of this JSON object:

  ```json
  {"package": {"lock": {"sources": [{"authority": "example", "identity": "entry"}]}, "name": "example", "semantic_graph": {"nodes": [{"check_mode": "linked", "declaration": {"qualified_name": ["entry"]}, "node_id": {"ordinal": 0, "package": "example", "path": ["entry"]}, "node_tag": "function"}]}}, "replay_target": {"arguments": false, "members": {"function": "entry"}, "occurrence_key": {"node_id": {"ordinal": 0, "package": "example", "path": ["entry"]}, "ordinal": "0", "role": "declaration"}, "over": null}, "byte_provision": [{"authority": "example", "bytes_path": "entry.fsl", "identity": "entry"}]}
  ```

  Its other eleven `request` members are `profile_selections` `[]`,
  `dependencies` `[]`, `source` `{"assignments":[],"kind":"input"}`,
  `obligation_identity` `{"model":null,"node_id":{"ordinal":0,"package":"example","path":["entry"]},"ordinal":0}`,
  `backend` `"fsl"`, `state_environment` `[]`, `accounting_limits` `{}`,
  `stage_limits` `{}`, `supplied_limits` `null`, `object_environment`
  `null` and `declared_domains` `[]`.

  The lock holds exactly one source, and `byte_provision` holds exactly the
  one row whose `authority` and `identity` equal that source's. The entry
  reads `bytes_path` while decoding the request, so over an empty directory
  the example is refused at stage `request`, exit 20, with the missing
  file's read refusal naming `entry.fsl` (FR-862), as the other examples
  are refused only for a file they name. With an empty `entry.fsl` beside
  it, the example is not refused `invalid-request` and not with a read refusal naming `entry.fsl` (FR-862 decodes it in
  full and FR-071 constructs it); it is refused by FR-098's recompile or
  package admission, never as `invalid_package` for
  `/package/lock/sources` and never as `missing_import`. The example does
  not replay: its package is no complete checked package.
- The returned text SHALL be the document written with two-space
  indentation, members in member order, and no trailing newline, and SHALL
  be the same bytes on every call with the same arguments. It reads no file
  and performs no compiler, evaluator, engine or provider call.
- `RequestExampleError` SHALL be a public enum with `Display`, `Error` and
  `code()` returning `Code::InvalidRequest`, with exactly two variants:
  - `UnknownFormat { format: String, known: Vec<&'static str> }` for a
    format that is not one of the five above: `known` is the five formats
    in the order above, and `Display` is
    `unknown request format <format>; request formats are native-run/1, native-compile/1, native-analyze/1, native-monitor/1, native-replay/1`;
  - `SelectionNeedsRunFormat { format: String }` for `Some(_)` with any
    other format: `Display` is
    `an export selection reads native-run/1, not <format>`.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-1047-AC-1 | `run_with_providers` over `{"format":"native-run/1","request":{}}` refuses `invalid-request`, stage `request`, exit 20, with message exactly `invalid request: missing required members /request/models, /request/program \| /request/package, /request/call \| /request/clause \| /request/items` and `details` exactly `{"missing":["/request/models",["/request/program","/request/package"],["/request/call","/request/clause","/request/items"]]}`; neither contains `line` or `column`. | Test (TC-3170) |
| FR-1047-AC-2 | `run_with_providers` over `{}` refuses with message exactly `invalid request: missing required members /format, /request` and `details` `{"missing":["/format","/request"]}`; over `{"format":"native-run/1"}` with message exactly `invalid request: missing required member /request` and `details` `{"missing":["/request"]}`; over `{"format":"bogus/1"}` with the format refusal of AC-5 for `bogus/1`, not a missing `/request`; each `invalid-request`, stage `request`, exit 20. A document carrying `argv` that FR-661's version adapter reads is not refused by this requirement. | Test (TC-3170) |
| FR-1047-AC-3 | Over `{"format":"native-monitor/1","request":{}}`, `run_with_providers` refuses `invalid-request`, stage `request`, exit 20, with message exactly `invalid request: missing required members /request/models, /request/program, /request/clauses, /request/trace, /request/records`, and `monitor` with message exactly `invalid request: missing required members /request/models, /request/program, /request/clauses, /request/trace`; `run_with_providers` over `{"format":"native-replay/1","request":{}}` refuses with `details.missing` the fourteen pointers `/request/profile_selections` to `/request/byte_provision` in FR-862's order. | Test (TC-3170) |
| FR-1047-AC-4 | `run_with_providers` over a `native-run/1` request with `models` `[{"source":{"file":"m.json"}}]`, `call` `{"function":"main","arguments":[]}` and `program` `{"source":{"file":"p.fsl"}}`, in an empty directory, refuses `invalid-request`, stage `request`, exit 20, reading no file, with `details` exactly `{"missing":["/request/models/0/format","/request/models/0/source/authority","/request/models/0/source/identity","/request/models/0/source/document","/request/models/0/source/formal_revision","/request/program/source/authority","/request/program/source/identity","/request/program/source/document","/request/program/source/formal_revision"]}` and the message joining those pointers with `, ` after `invalid request: missing required members `. | Test (TC-3170) |
| FR-1047-AC-5 | Each of these refuses `invalid-request`, stage `request`, exit 20, never `unknown_wire`, with `details.path` `/format` and the exact message: `run_with_providers` over a `native-analyze/1` document, `invalid request: format native-analyze/1 is not read by run; run reads native-run/1, native-monitor/1, native-replay/1`, with `details.accepted` `["native-run/1","native-monitor/1","native-replay/1"]`; `prepare_run` over a `native-monitor/1` document, `invalid request: format native-monitor/1 is not read by run-reader; run-reader reads native-run/1`; `analyze_with_providers` over a `native-run/1` document, `invalid request: format native-run/1 is not read by analyze; analyze reads native-analyze/1`; `compile_outcome_with_overrides` over a `native-run/1` document, `invalid request: format native-run/1 is not read by compile; compile reads native-compile/1`; `monitor` over a `native-run/1` document, `invalid request: format native-run/1 is not read by monitor; monitor reads native-monitor/1`; `export_with_limits_and_overrides` over `{"format":"native-export/1","request":{}}`, `invalid request: format native-export/1 is not read by export; export reads native-run/1`; and `run_with_providers` over `{"format":"bogus/1"}`, `invalid request: format bogus/1 is not read by run; run reads native-run/1, native-monitor/1, native-replay/1`. | Test (TC-3170) |
| FR-1047-AC-6 | `export_with_limits_and_overrides` with `ExportSelection::Ordinary` over FR-1015's counter export request with its `export` member removed refuses `invalid-request`, stage `request`, exit 20, with message exactly `invalid request: missing required member /request/export: export reads a native-run/1 request whose request holds an export object with at least profile and obligations` and `details` `{"missing":["/request/export"]}`; with `ExportSelection::Evidence` over the same request, message exactly `invalid request: missing required member /request/export_evidence: export evidence reads a native-run/1 request whose request holds an export_evidence object with package, obligation, tool and artifact`. Neither message contains `requires export`. A request whose `export` lacks `profile` is still refused as FR-1015 states, not by this requirement. | Test (TC-3170) |
| FR-1047-AC-7 | For each of `request_example(F, None)` with `F` each of the five request formats, and `request_example("native-run/1", Some(Ordinary))` and `Some(Evidence)`: the result is `Ok`, parses as an object whose members are exactly `format` (equal to `F`) and `request`, is two-space indented in member order with no trailing newline, and equals a second call's bytes. Written to a file in an empty directory and handed to its entry (`run_with_providers` for `native-run/1` and `native-replay/1`; `monitor`, with an empty record stream, for `native-monitor/1`; `compile_outcome_with_overrides`; `analyze_with_providers_and_limits`; `export_with_limits_and_overrides` with the same selection), it is never refused at stage `envelope`, never refused by this requirement, and never refused for an unknown, repeated or wrong-kind member; the run, compile, analyze, monitor and export examples are refused only at stage `file`, naming a file the example names; the evidence example settles FR-1022's `unimplemented-meaning`; the replay example is refused at stage `request`, exit 20, with the missing file's read refusal naming `entry.fsl` (FR-862), and with an empty `entry.fsl` beside it is not refused `invalid-request` and not with a read refusal naming `entry.fsl` and is refused by FR-098's recompile or package admission, never as `invalid_package` for `/package/lock/sources` and never as `missing_import`. Removing any one member of its `request` makes the same entry refuse by this requirement with `details.missing` exactly that member's pointer, or the array of its group. | Test (TC-3170) |
| FR-1047-AC-8 | `request_example("native-export/1", None)` returns `Err(RequestExampleError::UnknownFormat { .. })` whose `known` is `["native-run/1","native-compile/1","native-analyze/1","native-monitor/1","native-replay/1"]` and whose `Display` is exactly `unknown request format native-export/1; request formats are native-run/1, native-compile/1, native-analyze/1, native-monitor/1, native-replay/1`; `request_example("native-compile/1", Some(ExportSelection::Ordinary))` returns `SelectionNeedsRunFormat` with `Display` exactly `an export selection reads native-run/1, not native-compile/1`; both have `code()` `invalid-request`. | Test (TC-3170) |
| FR-1047-AC-9 | `export_with_limits_and_overrides` with `ExportSelection::Ordinary` over `{"format":"native-run/1","request":3}` refuses `invalid-request`, stage `request`, exit 20, with message exactly `invalid clause request member /request: expected an object` and `details` `null`; no `missing` list is written, since a `request` that is not an object has no members to list. | Test (TC-3202) |
| FR-1047-AC-10 | In an empty directory, `run_with_providers`, `prepare_run` and `read_clause_run_request`, each over a `native-run/1` request holding `models` `[]` and a complete `program` naming `p.fsl`, and each over one holding `models` `[]` and `package` `{"file":"p.json"}`, with no `call`, `clause`, `items` or other member, refuse `invalid-request`, stage `request`, exit 20, reading no named file, with message exactly `a 1-draft program runs through the spine; the request selects no call` and `details` exactly `{"missing":[["/request/call","/request/clause","/request/items"]]}`. The program request without `models` refuses with message exactly `invalid request: missing required members /request/models, /request/call \| /request/clause \| /request/items` and `details` exactly `{"missing":["/request/models",["/request/call","/request/clause","/request/items"]]}`. Over the package request with a valid non-empty `libraries` selection, each of the three entries refuses as FR-1039-AC-13 states, naming `libraries`, with no `details.missing`. | Test (TC-3170) |
| FR-1047-AC-11 | Over each of AC-10's `program` and `package` requests: `prepare_run` with `request_bytes` one byte under the request's length settles `incomplete`, `resource_exhausted`, exit 22, naming `request_bytes`; `run_with_providers`, `prepare_run` and `read_clause_run_request`, with `request` also holding, one at a time, `"extra": 1`, `"extensions": 3` and `"requirements": {}`, each refuse `invalid-request`, stage `request`, exit 20; and `prepare_run_with_overrides` with a `clocks` override, over the request also holding `"clocks": {}`, refuses as FR-640-AC-14 states. None of these has `details.missing` or AC-10's message. | Test (TC-3170) |
| FR-1047-AC-12 | In an empty directory, `run_with_providers`, `prepare_run` and `read_clause_run_request` over AC-10's `program` request with `"sources": []` added each refuse `invalid-request`, stage `request`, exit 20, naming `/request/sources`, with no `details.missing`. | Test (TC-3170) |
| FR-1047-AC-13 | In an empty directory, `run_with_providers`, `prepare_run` and `read_clause_run_request` over AC-10's `package` request with one member added at a time each refuse `invalid-request`, stage `request`, exit 20, with no `details.missing`: `"integer_division": "floor"`, naming `integer_division`; `"refinement_internals": []`, naming `refinement_internals`; `"clocks": {"c": {"kind": "event_position", "sequence_authority": "a"}}`, naming `clocks`; and a `sources` array of two complete entries with one `{authority, identity}`, naming `/request/sources/1`. | Test (TC-3170) |
| FR-1047-AC-14 | In an empty directory, `run_with_providers` and `prepare_run` over AC-10's `program` request changed one way at a time each give the following, and never AC-10's refusal: `"refinement_internals": []`, `invalid-request` naming `refinement_internals`; `"integer_division": "ceiling"`, `unknown_profile`/`unsupported-selection` naming `integer_division`; one complete `libraries` entry whose `integer_division` is `"ceiling"`, the same code naming `libraries[0].integer_division`; two complete `extensions` entries with one `{authority, identity}`, `invalid_package`/`conflicting-definition`; 64 complete `libraries` entries, `incomplete`, `resource_exhausted`, exit 22, naming the selected-file count; `program.source.formal_revision` 0, `invalid-identifier`; and one complete `models` entry of format `native-rule-model/1`, `invalid-request`. | Test (TC-3170) |
| FR-1047-AC-15 | In a build with `quire-extraction` and an empty directory, `run_with_providers` and `read_clause_run_request` over AC-10's `program` request, its `program` also holding a complete `extraction` record and `"clauses": []`, each refuse `extraction-clause-count`, stage `request`, exit 20, with no `details.missing`. | Test (TC-3170) |
| FR-1047-AC-16 | In an empty directory, each of these refuses exactly as FR-1047-AC-10 states for its source: `read_clause_run_request` over AC-10's `program` request changed, one way at a time, by `"refinement_internals": []`, by `program.source.formal_revision` 0 and by one complete `models` entry of format `native-rule-model/1`; `prepare_run` over AC-10's `package` request with that `models` entry; `prepare_run` over AC-10's `package` request with `"export_evidence": {}`; and, in a build with `quire-extraction`, `prepare_run` over AC-15's request, and `run_with_providers` over AC-15's request with exactly one complete `clauses` binding and `"refinement_internals": []`. | Test (TC-3170) |

## Dependencies

- FR-026: the shared envelope, read before request members.
- FR-027, FR-100, FR-312, FR-620, FR-644, FR-862, FR-1015, FR-1022: each
  reading's members and one-of groups; FR-1015 and FR-1022 keep the
  refusals inside `export` and `export_evidence`.
- FR-096 and FR-1039: the `unknown_wire` refusals of document formats that
  are not request envelopes.
- FR-031, FR-100, FR-640 and FR-1039: the member refusals that take
  precedence over a missing `call` / `clause` / `items` group.
- FR-661 and FSpec FR-562 (with FR-662 and FR-660): the argument documents
  the run entries also read.
- FR-286: the outcome document whose diagnostic carries `message` and
  `details`.
- FR-287: the driver reaches these entries; driver FR-028 prints
  `request_example`'s text for `--example`.
