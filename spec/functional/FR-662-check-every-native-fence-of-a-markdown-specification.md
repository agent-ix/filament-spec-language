---
id: FR-662
title: "Check every native fence of a Markdown specification and run one through the command"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-004
    type: traces_to
  - target: ix://agent-ix/filament-spec-language/US-029
    type: traces_to
  - target: ix://agent-ix/filament-spec-language/FR-030
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-031
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-004
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-275
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-286
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-355
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-287
    type: references
  - target: ix://agent-ix/filament-specification/FR-563
    type: depends_on
---
# FR-662: Check every native fence of a Markdown specification and run one through the command

## Description

Rules may be written inside a Markdown specification: an `ix:native` fence
under a `### <clause id>` heading holds one native unit. Two commands reach
them, and both are present in the command owner's default build:

- **Check.** The lifecycle operation `check_native_fences` checks every
  `ix:native` fence of one Markdown document. Each fence body is extracted
  through FR-030 and checked as a unit through S1 to S4, exactly as `check`
  checks a unit given as bytes. The driver's `check` verb calls it for a
  Markdown operand (FSpec FR-563). It complements `check_fences` (FR-355),
  which checks `quire` expression fences of spec artifacts against their
  declared object types.
- **Run.** FR-031's extraction mode of the `run` command runs one selected
  clause of the document. `fsl-command` SHALL build `fsl-replay` with its
  `quire-extraction` feature in its default build, so a `native-run/1`
  clause-run request carrying `program.extraction` reaches extraction
  through the driver's `run` verb with no build option. A build of
  `fsl-replay` without the feature keeps FR-100's request-stage refusal of
  the member; the command owner's build is never such a build.

`native-compile/1` keeps its `extraction-requires-run` refusal (FR-031): a
compile request publishes a package from whole source units, while a
document's clauses are checked by `check_native_fences` and run by `run`,
both of which the refusal names.

### Naming an extracted unit

Every extracted body is a unit with its own identity, distinct from the
original document's (FR-031). For `check_native_fences` the caller names
only the document, so the body identity is derived by one rule: authority
the original's authority, identity `<original identity>#<clause id>`, with
the clause id the Quire heading id that owns the fence. For `run`, the
body identity is the caller's `program.extraction.body` (FR-031). Every
result and diagnostic of an extracted body names its unit as
`{document: {authority, identity}, path, clause, body: {authority,
identity}}`, keeps its body span unchanged, and carries the original
document regions the verified source map gives for that span (FR-031
"Original-region correspondence"), so a reader finds the error in the
Markdown file by line and column.

## Inputs

`check_native_fences(&NativeFenceRequest, CheckLimits, &Cancel)`: the
original document's `Source` with its FR-001 labels and path, FR-030's
extraction limits, the S1 to S4 limits `check` takes, and the optional
extension catalog (FR-660).

## Outputs

`Result<Staged<NativeFenceReport>, StageFailure<FenceCheckCause>>`. A
`NativeFenceReport` holds one `NativeFenceResult` per `ix:native` fence in
document order: the unit name above, the fence's first body line in the
original, the check category and the diagnostics. `OutcomeDocument::from_native_fences`
serializes it as one `filament-outcome/1` document with operation `check`
and one item per fence, each item carrying its unit name; the document's
category is the most severe item category (FR-285). A `StageFailure`
covers the document as a whole: an unavailable extraction, the original's
byte or line limit, and cancellation.

## Behavior

- `check_native_fences` SHALL locate every fence through quire-rs extraction
  over the original bytes and SHALL verify each body's source map before it
  translates any offset (FR-004, FR-030).
- Each `ix:native` fence SHALL get exactly one `NativeFenceResult`; one
  fence's refusal SHALL NOT stop the check of the other fences. A fence of
  any other language SHALL get no result.
- A clause heading that owns more than one `ix:native` fence SHALL give each
  of its fences a result refused `extraction-clause-count` with `details`
  `{"actual": n}`. An `ix:native` fence that no clause heading owns SHALL
  give a result refused `missing_declaration`/`missing-name` located at the
  fence's first line.
- A body that checks SHALL give category success with no diagnostic; a body
  that refuses SHALL carry the refusals `check` gives for the same bytes,
  each with its body span and original regions.
- Two calls over the same input SHALL give equal reports; a cancelled
  `Cancel` SHALL return `StageFailure::Cancelled` with no report.
- The `run` command's extraction mode SHALL follow FR-031 unchanged.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-662-AC-1 | A Markdown document `spec.md` with authority `agent-ix` and identity `doc` holding two `### <id>` headings `Healthy` and `Broken`, each owning one `ix:native` fence (the first a unit that checks, the second the same unit with an ill-typed clause), and one `text` fence, gives two results in document order: `Healthy` success with no diagnostic and body identity `doc#Healthy`; `Broken` refusal `ill_typed` whose diagnostic names `{document: {agent-ix, doc}, path: spec.md, clause: Broken, body: {agent-ix, doc#Broken}}`, keeps its body span and carries the original line and column of the offending token. The outcome document has operation `check`, two items and category refusal, exit 20. | Test (TC-1743) |
| FR-662-AC-2 | A heading owning two `ix:native` fences gives each `extraction-clause-count` with `details` `{"actual": 2}`; an `ix:native` fence before the first heading gives `missing_declaration`/`missing-name` at its first line; the other fences' results are unchanged; checking twice gives equal reports and checking with a cancelled `Cancel` returns `StageFailure::Cancelled`. | Test (TC-1743) |
| FR-662-AC-3 | Through `fsl_command::command::run` in the command owner's default build, a `1-draft` clause-run request carrying `program.extraction` and two `clauses` bindings refuses `extraction-clause-count` with `details` `{"actual": 2}` at stage `request`, exit 20, not `invalid-request`; a request with one binding naming a clause the document does not hold ends with FR-030's error at stage `extraction-selection`, retaining the original identity and the binding. | Test (TC-1744) |

## Dependencies

- [FR-030](FR-030-consume-quire-extraction.md), [FR-004](FR-004-verify-source-maps.md): extraction and the verified map.
- [FR-031](FR-031-run-extracted-native-source.md): the run command's extraction mode.
- [FR-275](FR-275-take-a-typed-request-limits-and-cancel-on-every-lifecycle-operation.md), [FR-286](FR-286-serialize-every-outcome-as-one-json-outcome-document.md): operation shape and outcome document.
- FSpec FR-563: the `check` verb over a Markdown operand.
