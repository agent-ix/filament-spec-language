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
---
# FR-865: Render every outcome document as text, the CLI's default output

## Description

Text is the driver's default output format, so the first command a reader
types, `fsl check rule.fsl`, prints a result without a flag. The verbs `parse`, `check`, `compile` and `run` SHALL write, in text
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
   ` at <last_stage>` when `last_stage` is not null: for example
   `check: refusal (exit 20) at S3` or `run: success (exit 0) at S6a`.
   `<operation>` and `<category>` are FR-286's wire spellings and `<code>`
   is FR-285's exit code of the category.
2. One line per diagnostic, in document order, in FR-298's diagnostic form
   (`<path>:<line>:<column>: <code>: <message>`, or the source reference
   and byte offsets when the source is not provided). A diagnostic with a
   cause is followed by the line `  cause: <cause>`, and a diagnostic with
   a `label` by `  label: <label>`.
3. One line per item, in request order:
   `item <request_index>: <result> (<category>)`, then ` clause <name>`
   when the item names a clause, then ` <certification>` for a `proved`
   item. An item with a non-null `cause` is followed by
   `  cause: <cause>`, the cause's FR-331 wire spelling with its inner
   cause in parentheses (`undefined-evaluation (division-by-zero)`). An
   item's counterexample and trace follow it in FR-298's table and
   numbered-position forms, indented two spaces.
4. When `result` is not null, one line `result: <kind>` followed by a
   space and the canonical JSON of the result's other members, so a
   completed value reads `result: completed {"value":{"integer":"7"}}`
   and an undefined call `result: undefined {"reason":"sum-out-of-domain"}`.
   A `format` outcome's text is the formatted source itself, as it is
   today (FR-003), and has no header.

Each line is one FR-298 output unit, charged against `render.output_bytes`.

### The driver's text mode

- `--format` is optional on every verb and defaults to `text`; `--format
  json` selects machine mode. Any other value refuses `invalid-request`,
  exit 20, naming `--format` and the value, before any file is read
  (FSpec FR-301).
- In text mode the driver SHALL build the document exactly as in machine
  mode, render it through `render_outcome` with a `RenderSources` built
  from the admitted sources the command read, and write the rendered bytes
  to stdout, for `parse`, `check`, `compile` and `run`. None of these verbs
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
  code, its last stage when known, every diagnostic's code, cause and
  locus, every item's result, category and cause, and its result.
- When the driver runs a verb without `--format`, it SHALL write the text
  form of the outcome document.
- The driver's exit code for an invocation SHALL be equal in text mode and
  in machine mode.
- If `--format` names a value other than `json` or `text`, then the driver
  SHALL refuse `invalid-request`, exit 20, naming `--format` and the value,
  before reading any file.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-865-AC-1 | The text of an outcome document with operation `check`, `last_stage` S3, category refusal and one `ill_typed` diagnostic with cause `type-mismatch` begins with the line `check: refusal (exit 20) at S3`, holds FR-298's diagnostic line, and follows it with `  cause: type-mismatch`; the text of a `package` document at S4 with category success is the single line `package: success (exit 0) at S4`. | Test (TC-2455) |
| FR-865-AC-2 | The text of FR-286-AC-5's three `execute` documents ends with `result: completed {"value":{"integer":"7"}}`, `result: undefined {"reason":"sum-out-of-domain"}` and a `result: incomplete` line naming `accounting.work_units`; the text of FR-286-AC-11's monitor document holds `item 0: refuted (violation) clause <name>` and `  cause: undefined-evaluation (division-by-zero)`; the text of FR-286-AC-9's indeterminate document holds `(exit 11)` in its header. | Test (TC-2455) |
| FR-865-AC-3 | `fsl check <file>`, `fsl parse <authority> <identity> <file>`, `fsl compile <request>` and `fsl run <request>` without `--format`, each over a succeeding input and a refused input, write the text form of the document `--format json` writes for the same invocation, and exit with the same code. | Driver test (TC-2456): the `fsl` binary selects the format, so the code ticket adds the test in the filament-driver repository. |
| FR-865-AC-4 | `fsl run nosuch.json` and `fsl compile nosuch.json` without `--format` refuse with the I/O failure's code and exit 20, as with `--format json`, never `unimplemented_capability` or exit 21; `--format text` is accepted on every verb; `--format bogus` refuses `invalid-request`, exit 20, naming `--format` and `bogus`, before any file is opened. | Driver test (TC-2456) |
| FR-865-AC-5 | The text of AC-1's document rendered with a `RenderSources` holding its source admitted under path `src/rule.fsl` prints `src/rule.fsl:<line>:<column>`, and the same document rendered through the driver's text mode over a compile request prints the program source's path, because the compile outcome exposes the admitted sources. | Test (TC-2455) |

## Dependencies

- [FR-298](FR-298-render-outcomes-and-views-from-typed-values.md): the
  renderer, the diagnostic form and the output-byte budget.
- [FR-286](FR-286-serialize-every-outcome-as-one-json-outcome-document.md):
  the document rendered.
- [FR-285](FR-285-map-every-outcome-category-to-one-exit-code.md): the exit
  code shown and returned.
- [FR-287](FR-287-reach-fsl-through-the-driver-cli.md): the driver's verbs
  and streams.
- FSpec FR-718: the CLI's default text output; FSpec FR-301: the CLI
  contract.
