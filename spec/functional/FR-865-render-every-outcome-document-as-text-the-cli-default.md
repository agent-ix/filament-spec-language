---
id: FR-865
title: "Render every outcome document as text, the CLI's default output"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-029
    type: traces_to
  - target: ix://agent-ix/filament-spec-language/FR-298
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-286
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-285
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-287
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-718
    type: implements
  - target: ix://agent-ix/filament-specification/FR-301
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-027
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-810
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-665
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-866
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-098
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-071
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-099
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-109
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-121
    type: references
---
# FR-865: Render every outcome document as text, the CLI's default output

## Description

Text is the driver's default output format, so the first command a reader
types, `fsl check rule.fsl`, prints a result without a flag. The verbs `parse`, `check`, `compile`, `run`, `replay` and `export` SHALL write, in text
mode, the text form of the FR-286 outcome document they write that FR-298's `render_outcome`
returns, and SHALL exit with the same FR-285 code it exits with in machine
mode. `analyze` and `monitor` keep the text forms FR-621 and FR-624 and
the driver's FR-022 and FR-025 give them; this requirement does not change
them. The text form is a display of the typed document: it is built from the
document and the admitted sources only, and the JSON bytes stay the
authoritative form (FR-298).

### Decision: a text renderer, not a JSON default

A JSON default would make the first command print a machine document that a
reader has to scan for its category. The outcome document already holds
everything a reader needs (category, stage, items, diagnostics with their
causes and loci, the result), so a text form of it costs one renderer and
keeps the two forms one document. The format changes only the bytes on
stdout, never the exit code, so a script that tests `$?` works in either
mode.

### The text form of an outcome document

`render_outcome` with `RenderForm::Text` SHALL write these lines, in order:

1. A header line `<operation>: <category> (exit <code>)`, followed by
   ` at stage <word>` when `last_stage` is not null, `<word>` the stage's
   word ("Stage words"): for example `check: refusal (exit 20) at stage
   checking` or `run: success (exit 0) at stage execution`.
   `<operation>` and `<category>` are FR-286's wire spellings and `<code>`
   is FR-285's exit code of the category.
2. One line per diagnostic, in document order, in FR-298's diagnostic form
   (`<path>:<line>:<column>: <code>: <message>`, or the source reference
   and byte offsets when the source is not provided). A diagnostic with a
   cause is followed by the line `  cause: <cause>`, a diagnostic with
   a `label` by `  label: <label>`, and a diagnostic with a `hint`
   (FR-286 "Fix hints") by `  hint: <hint>`, in that order, so the reader
   sees what went wrong, where, and what to change. A diagnostic with no
   `hint` member has no hint line; the renderer never composes a hint of
   its own, and writes the document's `hint` byte for byte, a
   `missing_import` hint included (FR-286 "Missing selections").
3. One line per item, in request order:
   `item <request_index>: <result> (<category>)`, then ` clause <name>`
   when the item names a clause, then ` <certification>` for a `proved`
   item. An item with a non-null `cause` is followed by
   `  cause: <cause>`, the cause's FR-331 wire spelling with its inner
   cause in parentheses (`undefined-evaluation (division-by-zero)`). An
   item's counterexample and trace follow it in FR-298's table and
   numbered-position forms, indented two spaces.
4. When `result` is not null and its kind is neither `exported` nor
   `packaged`, one line `result: <kind>` followed by a
   space and the canonical JSON of the result's other members, so a
   completed value reads `result: completed {"value":{"integer":"7"}}`
   and an undefined call `result: undefined {"reason":"sum-out-of-domain"}`.
   A `format` outcome's text is the formatted source itself, as it is
   today (FR-003), and has no header. A `replay` outcome's `replayed` result is
   written as one line per entry, `result <index>: <arm> <settlement>`,
   `index` the entry's zero-based position (FR-286).

An `export` outcome (FR-286 "Export outcomes") whose `result` is
`exported` is written as exactly these four lines, in order, and no item or
result line:

1. The header line of rule 1, for example `export: success (exit 0) at stage
   packaging`.
2. `target: <target_family> <mapping_profile_id>`, for example
   `target: tla filament.output.tla2/v1`.
3. `bytes written: <n>`, `<n>` the export outcome's `output.bytes_written`
   in ASCII decimal, or `0` when the outcome has no `output` member.
4. `output: <path>`, `<path>` the export outcome's `output.path` as the
   caller gave it, or `output: none` when the outcome has no `output`
   member, because no file was named.

A non-`assembled` export also writes, after line 4, the rule 2 diagnostic
lines and, for each record with causes, `  cause: <cause>` naming the
record's obligation and cause in FR-331's wire spelling, so a floor or limit
stop shows why no file was written. An `export` outcome whose `result` is
`null` (a refusal before the mapper runs) is written by rules 1 and 2 alone.
The text names the file and its size rather than printing the target text,
because the target text is the file's content and a reader of the terminal
needs to know where it went.

A `package` outcome whose `result` is `packaged` (FR-286 "Compile results") is
written as the header line of rule 1, then:

1. `produced: checked package (<n> bytes)`, `<n>` the `package.bytes` member
   in ASCII decimal.
2. `written to: <path>`, `<path>` the `package.output` member, when it is not
   null.
3. `handoff written to: <directory>`, `<directory>` the `package.handoff`
   member, when it is not null.
4. `nothing written: no --output option, output member or handoff member was
   given`, when both members are null, and no line 2 or 3.

It writes no item line and no `result:` line, and the rule 2 diagnostic lines
do not occur on a success. A `package` outcome whose `result` is `null`
(`OutcomeDocument::from_package` over a checked package, and every compile
that is not a success) is written by rules 1 to 4 alone, so a success of
that kind is the header line only. The header names the stage the document
carries: a model-unit compile that succeeds at `I1` writes `at stage intake`.

Each line is one FR-298 output unit, charged against `render.output_bytes`.

### Stage words

The header names the stage in words: `render_outcome` writes the word that
the public function `fsl_inspect::stage_word(label: &str) -> String`, re-exported at the crate
root beside `render_outcome`,
returns for the document's `last_stage` label. The map is a function of the
text form alone; the JSON document keeps the label (FR-286).

| Label | Word |
| --- | --- |
| `S0` | `source` |
| `S1` | `syntax` |
| `S2` | `forms` |
| `I1` | `intake` |
| `I2` | `imports` (the function only: no document's `last_stage` is `I2`) |
| `S3` | `checking` |
| `S4` | `packaging` |
| `S5` | `contract` |
| `S6a` | `execution` |
| `S6b` | `proof` |
| `S6c` | `model-checking` |
| `S7` | `witness` |
| `S8` | `replay` |

`stage_word` returns a label that is not in the table unchanged, so a stage
added to the document before it is added here still renders. The function
takes a string because `OutcomeStage` and the schema's `last_stage` values
hold the twelve other labels and no `I2`; adding `I2` to the wire for a
stage no document reaches would change `filament-outcome/1` for no reader,
and `render_outcome` takes a typed document that cannot hold an unknown
label, so the `I2` and unknown-label cases are asserted on `stage_word`. The
words describe ADR-011's stages: source bytes, the lossless CST, parsed
forms, domain-package intake, the dependency import view, the checked graph,
the linked checked package, contract IR, reference execution, bounded proof,
native model checking, the typed witness and native replay.

### Rationale: the texts this requirement replaces (QSL-2937)

Before QSL-2937 the header wrote the label (`package: success (exit 0) at
S4`, `check: refusal (exit 20) at I1`), a successful compile wrote that one
line and so did not say that a package was produced or where it went, and
every `missing_import` diagnostic of cause `missing-selection` carried the
one hint `Import the package that supplies the selected name.`, beside a
message that named a model, a library or a byte-provision entry. The blind-help
audit (QSL-2493) and the driver lane (FR-002-AC-13: the CLI text equals FSL's
document) put all three texts here, because the driver cannot change what
FSL's document says. The text stays a render of FSL's document alone: the
size and path of a compile and the kind of a missing selection are therefore
members of the document (FR-286 "Compile results", "Missing selections"),
not state the renderer holds. The bare `  cause: missing-selection` line is
not changed: explaining causes is outside QSL-2937's four texts.

### The driver's text mode

- `--format` is optional on every verb and defaults to `text`; `--format
  json` selects machine mode. Any other value refuses `invalid-request`,
  exit 20, naming `--format` and the value, before any file is read
  (FSpec FR-301).
- In text mode the driver SHALL build the document exactly as in machine
  mode, render it through `render_outcome` with a `RenderSources` built
  from the admitted sources the command read, and write the rendered bytes
  to stdout, for `parse`, `check`, `compile`, `run`, `replay` and `export`. None of these verbs
  refuses text mode as `unimplemented_capability`.
- `command`'s compile, run and check outcomes, and `RunError`, SHALL expose
  the admitted FR-001 `Source` values the command read, so the driver
  provides them to `RenderSources::from_sources` without reading a source a
  second time.
- A request file that does not exist, or a malformed request, refuses in
  text mode with the same catalog code and exit code as in machine mode, so
  a missing path is reported as the I/O failure it is.

## Inputs

An `OutcomeDocument`, `RenderForm::Text`, the `RenderSources` of the
admitted sources, `RenderLimits` and `&Cancel`.

## Outputs

The rendered text bytes; in the driver, those bytes on stdout and FR-285's
exit code of the document's category.

## Behavior

- When `render_outcome` renders an outcome document as text, it SHALL write
  the header line, the diagnostic lines, the item lines and the result line
  in that order.
- The text of a document SHALL name its operation, its category, its exit
  code, its last stage when known, every diagnostic's code, cause, locus
  and hint when it has one, every item's result, category and cause, and its result.
- When a diagnostic carries `hint`, `render_outcome` SHALL write the line
  `  hint: <hint>` after the diagnostic's cause and label lines, and SHALL
  write no hint line for a diagnostic without one.
- When the driver runs a verb without `--format`, it SHALL write the text
  form of the outcome document.
- When `render_outcome` writes a header line for a document whose
  `last_stage` is not null, it SHALL write ` at stage ` and the word
  `stage_word` returns for the label.
- The header line SHALL NOT contain a stage label of the "Stage words"
  table.
- If the label handed to `stage_word` is not in the "Stage words" table,
  then `stage_word` SHALL return it unchanged.
- When `render_outcome` renders a `package` outcome whose `result` is
  `packaged`, it SHALL write the lines of that result's text form in
  "The text form of an outcome document".
- When `fsl compile` runs in text mode and succeeds, the driver SHALL write
  the text form of the compile's outcome document, with the CLI command
  `compile` in place of the operation in the header (driver FR-002-AC-13).
- When `render_outcome` renders a diagnostic that carries `hint`, it SHALL
  write the hint line the document holds, including the hint of each
  `selection` of a `missing_import` diagnostic.
- When a producer refuses `missing_import`/`missing-selection`, it SHALL
  set `selection` on its `OutcomeDiagnostic` to the kind of FR-286 "Missing
  selections", except plugin replay's defensive `UnresolvedDependency`.
- The driver's exit code for an invocation SHALL be equal in text mode and
  in machine mode.
- If `--format` names a value other than `json` or `text`, then the driver
  SHALL refuse `invalid-request`, exit 20, naming `--format` and the value,
  before reading any file.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-865-AC-1 | The text of an outcome document with operation `check`, `last_stage` S3, category refusal and one `ill_typed` diagnostic with cause `type-mismatch` begins with the line `check: refusal (exit 20) at stage checking`, holds FR-298's diagnostic line, and follows it with `  cause: type-mismatch`; the text of a `package` document at S4 with category success and `result` `null` is the single line `package: success (exit 0) at stage packaging`. | Test (TC-2455) |
| FR-865-AC-2 | The text of FR-286-AC-5's three `execute` documents ends with `result: completed {"value":{"integer":"7"}}`, `result: undefined {"reason":"sum-out-of-domain"}` and a `result: incomplete` line naming `accounting.work_units`; the text of FR-286-AC-11's monitor document holds `item 0: refuted (violation) clause <name>` and `  cause: undefined-evaluation (division-by-zero)`; the text of FR-286-AC-9's indeterminate document holds `(exit 11)` in its header. | Test (TC-2455) |
| FR-865-AC-3 | `fsl check <file>`, `fsl parse <authority> <identity> <file>`, `fsl compile <request>` and `fsl run <request>` without `--format`, each over a succeeding input and a refused input, write the text form of the document `--format json` writes for the same invocation, and exit with the same code. | Driver test (TC-2456): the `fsl` binary selects the format, so the code ticket adds the test in the filament-driver repository. |
| FR-865-AC-4 | `fsl run nosuch.json` and `fsl compile nosuch.json` without `--format` refuse with the I/O failure's code and exit 20, as with `--format json`, never `unimplemented_capability` or exit 21; `--format text` is accepted on every verb; `--format bogus` refuses `invalid-request`, exit 20, naming `--format` and `bogus`, before any file is opened. | Driver test (TC-2456) |
| FR-865-AC-5 | The text of AC-1's document rendered with a `RenderSources` holding its source admitted under path `src/rule.fsl` prints `src/rule.fsl:<line>:<column>`, and the same document rendered through the driver's text mode over a compile request prints the program source's path, because the compile outcome exposes the admitted sources. | Test (TC-2455) |
| FR-865-AC-6 | `render_outcome` with `RenderForm::Text` over FR-286-AC-21's replay outcome writes `replay: success (exit 0) at stage replay` followed by `result 0: input reproduced-without-witness`; over the refused replay of FR-862-AC-1 it writes `replay: refusal (exit 20)` and one diagnostic line with code `invalid-request`, and no `result` line. | Test (TC-2963) |
| FR-865-AC-7 | `render_outcome` with `RenderForm::Text` over FR-286-AC-22's counter export document written to `m.tla` writes exactly `export: success (exit 0) at stage packaging`, `target: tla filament.output.tla2/v1`, `bytes written: <n>` with `<n>` the target text's length, and `output: m.tla`, and no other line; over the same export with no output file named its last two lines are `bytes written: 0` and `output: none`; over the export under `export.output_bytes` 1 it writes `export: incomplete (exit 22) at stage packaging`, `bytes written: 0` and a line naming `export.output_bytes`. | Test (TC-2996) |
| FR-865-AC-8 | The text of FR-286-AC-28's `check` document follows its diagnostic line with `  cause: type-mismatch` and then `  hint: <text>`, `<text>` the diagnostic's `hint` byte for byte; the text of FR-286-AC-28's `io-error` document follows its diagnostic line with `  hint: <text>`, the diagnostic's `hint`; the texts of FR-286-AC-15's blank-label document (lines `  cause: blank-label` then `  label: <label>`) and of FR-286-AC-23's `output-failure` document, whose diagnostics have no `hint`, hold no line beginning `  hint:`. | Test (TC-3114) |
| FR-865-AC-9 | `render_outcome` with `RenderForm::Text` over an outcome document whose `last_stage` is each of `S0`, `S1`, `S2`, `I1`, `S3`, `S4`, `S5`, `S6a`, `S6b`, `S6c`, `S7` and `S8` writes a header line ending ` at stage source`, ` at stage syntax`, ` at stage forms`, ` at stage intake`, ` at stage checking`, ` at stage packaging`, ` at stage contract`, ` at stage execution`, ` at stage proof`, ` at stage model-checking`, ` at stage witness` and ` at stage replay` in that order, and over a document whose `last_stage` is null a header with no ` at stage `. | Test (TC-3197) |
| FR-865-AC-10 | `stage_word("I2")` returns `imports`, `stage_word("X9")` returns `X9`, and `stage_word` of each AC-9 label returns the word AC-9 lists; the header of no AC-9 document contains a label of the table (`S4`, `I1`, ...) as a word. | Test (TC-3197) |
| FR-865-AC-11 | The JSON of each AC-9 document holds `last_stage` as the label string (`"S4"`, `"I1"`), unchanged by the rendering. | Test (TC-3197) |
| FR-865-AC-12 | `render_outcome` over FR-286-AC-39's document of the request with `output` writes exactly `package: success (exit 0) at stage packaging`, `produced: checked package (<n> bytes)` with `<n>` the document's `package.bytes`, and `written to: reqs/spine.package.json`. | Test (TC-3198) |
| FR-865-AC-13 | `render_outcome` over FR-286-AC-39's document of the request without `output` and without `handoff` writes exactly `package: success (exit 0) at stage packaging`, `produced: checked package (<n> bytes)` and `nothing written: no --output option, output member or handoff member was given`. | Test (TC-3198) |
| FR-865-AC-14 | `render_outcome` over FR-286-AC-39's document of the request with `handoff` and no `output` writes exactly the header, the `produced:` line and `handoff written to: reqs/<directory>`, and no `written to:` line. | Test (TC-3198) |
| FR-865-AC-15 | `render_outcome` over the document of a compile of a model unit that succeeds at `I1` writes a header ending ` at stage intake`; over a `package` document whose `result` is `null` it writes the header line only. | Test (TC-3198) |
| FR-865-AC-16 | `fsl compile <request>` and `fsl compile <request> --output <file>` without `--format`, over a succeeding request, write the text form of the document `--format json` writes for the same invocation, whose header names `compile` as driver FR-002-AC-13 states and which holds the `produced:` line and the `written to:` or `nothing written:` line of AC-12 and AC-13, and exit 0. | Driver test (TC-3198): the `fsl` binary selects the format, so the code ticket adds the test in the filament-driver repository. |
| FR-865-AC-17 | The text of FR-866-AC-3's `check_outcome` refusal holds the line `  hint: Supply the model in the request's models member, or with --model <identity>=<file> on fsl check.`; the text of its compile request with `models` `[]` holds the same line; the JSON `hint` of each is that text and `selection` is `model`. | Test (TC-3199) |
| FR-865-AC-18 | The text of FR-099-AC-3's refusal of an import with no library supplied holds the line `  hint: Supply the library in the request's libraries member, or with --library <identity>=<file> on fsl check.`, and its JSON `selection` is `library`. | Test (TC-3199) |
| FR-865-AC-19 | The text of FR-121-AC-35's refusal of `{test/other, big}` (`test/other` supplied but imported by no `import`) holds the line `  hint: Call a function of a library that the unit imports directly, or add the import.`, and its JSON `selection` is `unimported-library`. | Test (TC-3199) |
| FR-865-AC-20 | The text of FR-098-AC-33's refusal of the `function_owner` `test/absent` holds the line `  hint: Name a function_owner that the program imports directly, or add the import.`, and its JSON `selection` is `function-owner`. | Test (TC-3199) |
| FR-865-AC-21 | The text of FR-071-AC-5's refusal of a package reference with no matching byte-provision entry holds the line `  hint: Add a byte_provision entry for the named source to the replay request.`, and its JSON `selection` is `byte-provision`. | Test (TC-3199) |
| FR-865-AC-22 | The text of a refusal of a unit whose `import "sibling/unit" as s;` names a supplied sibling unit of identity `sibling/unit` while no library is supplied as `sibling/unit` holds the line `  hint: Supply the imported unit as a library, not as a sibling unit.`, and its JSON `selection` is `sibling-unit`. | Test (TC-3199) |
| FR-865-AC-23 | For each of the `source`, `extension-definition` and `package-reference` kinds, the diagnostic of its producer in FR-286 "Missing selections" renders a line `  hint: <text>` with `<text>` the table's text for the kind, byte for byte. | Test (TC-3199) |
| FR-865-AC-24 | For every diagnostic of AC-17 to AC-23, the text's `  hint:` line equals the JSON diagnostic's `hint` member byte for byte, and no text line is `  hint: Import the package that supplies the selected name.`. | Test (TC-3199) |

## Dependencies

- [FR-298](FR-298-render-outcomes-and-views-from-typed-values.md): the
  renderer, the diagnostic form and the output-byte budget.
- [FR-286](FR-286-serialize-every-outcome-as-one-json-outcome-document.md):
  the document rendered.
- [FR-285](FR-285-map-every-outcome-category-to-one-exit-code.md): the exit
  code shown and returned.
- [FR-287](FR-287-reach-fsl-through-the-driver-cli.md): the driver's verbs
  and streams.
- [FR-027](FR-027-export-compiled-native-package.md) and
  [FR-810](FR-810-write-the-checked-package-to-a-file-from-a-compile-request.md):
  the compile outcome whose document carries `packaged` (FR-286 "Compile
  results"), and the `output` it names.
- [FR-866](FR-866-check-a-unit-against-the-models-and-libraries-it-imports.md),
  FR-099, FR-109, FR-121, FR-098 and FR-071: the refusals whose hints AC-17
  to AC-22 pin.
- FSpec FR-718: the CLI's default text output; FSpec FR-301: the CLI
  contract.
