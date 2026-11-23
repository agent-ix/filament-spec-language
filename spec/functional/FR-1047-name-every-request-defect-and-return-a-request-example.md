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
returns a minimal valid request document for each request format. The
driver's `--example` option (driver FR-028) prints what this entry returns.

The members each format carries, their spellings and their one-of groups
stay with the requirement that owns the format: FR-100 and FR-312
(`native-run/1` read by the run entries), FR-1015 and FR-1022 (`native-run/1`
read by the export entry), FR-027 (`native-compile/1`), FR-620
(`native-analyze/1`), FR-644 (`native-monitor/1`) and FR-862
(`native-replay/1`). This requirement owns how a reader reports what is
missing or misdirected, and the example entry.

## Inputs

- A request file handed to one of the request entries below.
- For `request_example`: a request format identifier and an optional export
  selection.

### Request entries and the formats they read

| Entry family | Entries | Formats read |
| --- | --- | --- |
| run | `run`, `run_with_overrides`, `run_with_limits`, `run_with_limits_and_overrides`, `run_with_providers`, `run_with_providers_and_limits_and_overrides` | `native-run/1`, `native-monitor/1`, `native-replay/1` |
| compile | `compile`, `compile_outcome`, `compile_with_overrides`, `compile_outcome_with_overrides`, `compile_outcome_with_limits_and_overrides` | `native-compile/1` |
| analyze | `prepare_analyze`, `analyze_with_providers`, `analyze_with_providers_and_limits` | `native-analyze/1` |
| monitor | `monitor` | `native-monitor/1` |
| export | `export_with_limits_and_overrides` | `native-run/1` |

The run entries dispatch on the request's `format`: a `native-monitor/1`
request runs its record stream (FR-644) and a `native-replay/1` request
replays (FR-862). That dispatch is unchanged. There is no `native-export/1`
format: the export entry reads a `native-run/1` request carrying an `export`
member, or with `ExportSelection::Evidence` an `export_evidence` member.

## Outputs

- A request-stage refusal: code `invalid-request`, stage `request`, exit
  20, with the diagnostic `message` and `details` this requirement states.
- From `request_example`: the text of one JSON request document, or a
  `RequestExampleError`.

## Definitions

- **Required member**: a member the format's reader refuses the request for
  lacking, that has no default and belongs to no one-of group. `format` and
  `request` are the required members of every envelope (FR-026).
- **One-of group**: members of which the reader requires exactly one, as the
  format's owning requirement states. The groups FSL's readers have are:
  `program` / `package` in `native-run/1` (FR-100) and `native-analyze/1`
  (FR-620); and, in `native-run/1` read by the run entries, `call` /
  `clause` / `items` (FR-100). A run request carrying a member that selects
  another reading of `native-run/1` (`export`, `export_evidence`,
  `tl_lowering`, `synthesis`, `verification_plan`, `model_based_test`,
  `refinement_transfer`) is read under that member's requirement and does
  not lack the `call` / `clause` / `items` group. A one-of group is missing
  when the request carries none of its members.
- **Missing entry**: a missing required member, written as its RFC 6901 JSON
  pointer from the document root (`/request/models`), or a missing one-of
  group, written as the pointers of its members in the order listed above
  joined by ` | ` (`/request/program | /request/package`).
- **Example order**: the depth-first order of the members of the format's
  request example (below). Missing entries are listed in example order; a
  missing group takes the position of the group member the example carries.

## Behavior

### Missing members

- When a request document's only decode defects are missing required members
  and missing one-of groups, the entry SHALL refuse `invalid-request` at
  stage `request`, exit 20, before any file the request names is read, with
  one diagnostic that lists every missing entry at once, at every depth of
  the objects the document holds, in example order:
  - `message` is `invalid request: missing required member ` followed by the
    one entry, or `invalid request: missing required members ` followed by
    the entries joined by `, `;
  - `details` is `{"missing": [...]}`, one element per entry in the same
    order: the pointer string for a required member, and the array of the
    group's pointer strings for a group.
- The message and details of this refusal SHALL NOT carry a line or a
  column. A document that is not JSON keeps its line and column (FR-026).
- A request that also carries an unknown member, a member of the wrong JSON
  kind or a repeated member is refused for that defect as its owning
  requirement states; this requirement adds no ordering among those defects.
- The envelope is read first (FR-026): a document lacking `format` or
  `request` lists only `/format` and `/request` as missing, because the
  members of `request` depend on its format.

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

- When the export entry reads a `native-run/1` request that lacks the member
  its `ExportSelection` selects, that member (`/request/export` for
  `Ordinary`, `/request/export_evidence` for `Evidence`) SHALL be a missing
  required member, listed as above, and the message SHALL be the
  missing-members message followed by `: ` and the member's shape sentence:
  - `Ordinary`: `export reads a native-run/1 request whose request holds an export object with at least profile and obligations`
    (FR-1015);
  - `Evidence`: `export evidence reads a native-run/1 request whose request holds an export_evidence object with package, obligation, tool and artifact`
    (FR-1022).

  The text `the selected export invocation requires` SHALL NOT appear in any
  refusal.

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
  `native-replay/1` SHALL return the example for the entry family that
  reads `F` (for `native-run/1`, the run entries), and
  `request_example("native-run/1", Some(s))` the example for the export
  entry under selection `s`.
- Each example SHALL be one JSON object whose members are exactly `format`,
  equal to `F`, and `request`. Every object in it SHALL carry exactly its
  required members and, for each of its one-of groups, the first member of
  the group as listed in Definitions. Files it names are relative paths that
  need not exist.
- The returned text SHALL be the document written with two-space
  indentation and no trailing newline, with `format` before `request`, and
  SHALL be the same bytes on every call with the same arguments. It reads
  no file and performs no compiler, evaluator, engine or provider call.
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
| FR-1047-AC-1 | `run_with_providers` over `{"format":"native-run/1","request":{}}` refuses `invalid-request`, stage `request`, exit 20, with message exactly `invalid request: missing required members /request/models, /request/program \| /request/package, /request/call \| /request/clause \| /request/items` and `details` exactly `{"missing":["/request/models",["/request/program","/request/package"],["/request/call","/request/clause","/request/items"]]}`; neither contains `line` or `column`. | Test |
| FR-1047-AC-2 | `run_with_providers` over `{}` refuses with message exactly `invalid request: missing required members /format, /request` and `details` `{"missing":["/format","/request"]}`; over `{"format":"native-run/1"}` with message exactly `invalid request: missing required member /request` and `details` `{"missing":["/request"]}`; each `invalid-request`, stage `request`, exit 20. | Test |
| FR-1047-AC-3 | `run_with_providers` and `monitor` over `{"format":"native-monitor/1","request":{}}` each refuse `invalid-request`, stage `request`, exit 20, with message exactly `invalid request: missing required members /request/models, /request/program, /request/clauses, /request/trace`: the run entry dispatches the monitor request to the monitor reader. | Test |
| FR-1047-AC-4 | `run_with_providers` over a `native-run/1` request with `models` `[]`, `call` `{"function":"main","arguments":[]}` and `program` `{"source":{"file":"p.fsl"}}`, in an empty directory, refuses `invalid-request`, stage `request`, exit 20, reading no file, with message exactly `invalid request: missing required members /request/program/source/authority, /request/program/source/identity, /request/program/source/document, /request/program/source/formal_revision`. | Test |
| FR-1047-AC-5 | Each of these refuses `invalid-request`, stage `request`, exit 20, never `unknown_wire`, with the exact message and `details.accepted`: `run_with_providers` over a `native-analyze/1` document, `invalid request: format native-analyze/1 is not read by run; run reads native-run/1, native-monitor/1, native-replay/1`, `["native-run/1","native-monitor/1","native-replay/1"]`; `analyze_with_providers` over a `native-run/1` document, `invalid request: format native-run/1 is not read by analyze; analyze reads native-analyze/1`; `compile_outcome_with_overrides` over a `native-run/1` document, `invalid request: format native-run/1 is not read by compile; compile reads native-compile/1`; `export_with_limits_and_overrides` over `{"format":"native-export/1","request":{}}`, `invalid request: format native-export/1 is not read by export; export reads native-run/1`; and `run_with_providers` over format `bogus/1`, naming `bogus/1` the same way. Each `details.path` is `/format`. | Test |
| FR-1047-AC-6 | `export_with_limits_and_overrides` with `ExportSelection::Ordinary` over FR-1015's counter export request with its `export` member removed refuses `invalid-request`, stage `request`, exit 20, with message exactly `invalid request: missing required member /request/export: export reads a native-run/1 request whose request holds an export object with at least profile and obligations` and `details` `{"missing":["/request/export"]}`; with `ExportSelection::Evidence` over the same request without `export_evidence`, message exactly `invalid request: missing required member /request/export_evidence: export evidence reads a native-run/1 request whose request holds an export_evidence object with package, obligation, tool and artifact`. Neither message contains `requires export`. | Test |
| FR-1047-AC-7 | For each of `request_example(F, None)` with `F` each of the five request formats, and `request_example("native-run/1", Some(Ordinary))` and `Some(Evidence)`: the result is `Ok`, parses as an object whose members are exactly `format` (equal to `F`) and `request`, is two-space indented with no trailing newline, and equals a second call's bytes; written to a file in an empty directory and handed to its entry (`run_with_providers` for `native-run/1`, `native-monitor/1` and `native-replay/1`, `compile_outcome_with_overrides`, `analyze_with_providers_and_limits`, `export_with_limits_and_overrides` with the same selection), it is never refused at stage `envelope` or `request`; and removing any one member of its `request` makes the same entry refuse with a `details.missing` that holds that member's pointer, or the array of its group. | Test |
| FR-1047-AC-8 | `request_example("native-export/1", None)` returns `Err(RequestExampleError::UnknownFormat { .. })` whose `known` is `["native-run/1","native-compile/1","native-analyze/1","native-monitor/1","native-replay/1"]` and whose `Display` is exactly `unknown request format native-export/1; request formats are native-run/1, native-compile/1, native-analyze/1, native-monitor/1, native-replay/1`; `request_example("native-compile/1", Some(ExportSelection::Ordinary))` returns `SelectionNeedsRunFormat` with `Display` exactly `an export selection reads native-run/1, not native-compile/1`; both have `code()` `invalid-request`. | Test |

## Dependencies

- FR-026: the shared envelope, read before request members.
- FR-027, FR-100, FR-312, FR-620, FR-644, FR-862, FR-1015, FR-1022: each
  format's members and one-of groups.
- FR-286: the outcome document whose diagnostic carries `message` and
  `details`.
- FR-287: the driver reaches these entries; driver FR-028 prints
  `request_example`'s text for `--example`.
