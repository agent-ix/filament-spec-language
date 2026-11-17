---
id: FR-624
title: "Name every analyze refusal and every unrouted verb with a document and an exit code"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-010
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-620
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-285
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-286
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-287
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-301
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-522
    type: depends_on
  - target: ix://agent-ix/filament-specification/ADR-004
    type: depends_on
---
# FR-624: Name every analyze refusal and every unrouted verb with a document and an exit code

## Description

Every `analyze` outcome, a refusal included, SHALL produce one FR-286
document in machine mode or one diagnostic line in text mode, and the FR-285
exit code of its category. The driver SHALL never exit without output.

### Decision: named refusals on the existing exit codes

Exit 21 already means unsupported and exit 20 refused input (FSpec ADR-004).
What was missing is a machine-readable cause, so the codes stay and each
refusal gets a catalog code and cause. A verb the driver accepts but whose
library operation the build does not route is refused with the new cause
`operation-not-routed` naming the verb. This gives the period before a verb
is wired a named outcome; it removes no verb: each verb's operation stays
required by its own requirement.

## Inputs

An analyze request, or any driver verb invocation.

## Outputs

One `filament-outcome/1` document or text diagnostic, and an exit code.

## Behavior

### Analyze refusals

| Condition | Stage | Code / cause | Category | Exit |
| --- | --- | --- | --- | --- |
| Unreadable, malformed or wrong-format request; unknown, forbidden or mistyped member (FR-620) | request | `invalid-request` | refusal | 20 |
| Byte or selected-file limit reached | request | FR-026's resource cause naming the limit | refusal | 20 |
| Unknown edition, unknown language, source refusal at S1 to S4 | that stage | that stage's code | refusal, or unsupported for an `unsupported_construct` | 20 or 21 |
| Subject binding refused or incomplete (FR-281 `AnalyzeRefusal::Binding`) | analyze | the admission record's code and cause | refusal or incomplete | 20 or 22 |
| Missing source bytes (FR-281 `AnalyzeRefusal::Provision`) | analyze | `missing_import`/`missing-selection` | refusal | 20 |
| Cancellation or deadline before the first item | the stage reached | `cancelled` with cause `requested` or `deadline` | incomplete | 22 |
| A refusal of one item | item record | FR-281 and FR-620's per-item causes | that item's | most severe item code |

- A whole-request refusal writes FR-286's document with `items` empty and one
  diagnostic carrying the code, cause and locus; text mode writes one line
  `fsl analyze: <code>: <message>` to stderr.
- Stdout is empty in machine mode only when the stream itself fails, which is
  the driver's internal failure, exit 30 (FR-287).

### Unrouted verbs

Verb routing and text rendering belong to the driver binary: the command
adapter (`fsl_command::command`) reads request documents by their `format` and
takes no verb, so the obligations below are the driver's, verified by driver
tests.

- For every verb the driver accepts, `analyze`, `monitor`, `generate`,
  `prove`, `replay`, `inspect` and `render` included, when the build does not
  route that verb to its library operation, the driver SHALL write a
  `filament-outcome/1` document with `operation` the verb, `last_stage`
  `null`, category `unsupported`, and one diagnostic with code
  `unsupported_operation`, cause `operation-not-routed` and payload
  `{"verb": "<verb>"}`, and exit 21.
- In text mode it writes `fsl <verb>: this build does not route the <verb> operation (unsupported_operation)`
  to stderr and exits 21.
- The refusal happens after argument parsing and before any file is read, so
  an unknown option still refuses with exit 20 under FSpec FR-301-AC-3.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-624-AC-1 | A malformed `native-analyze/1` document gives a machine-mode document with category `refusal`, empty `items`, one diagnostic `invalid-request`, and exit 20; text mode prints one `fsl analyze: invalid-request:` line and exits 20. | Test (TC-1584) |
| FR-624-AC-2 | A program the S3 checker refuses gives a document with `last_stage` `S3`, the stage identifier FR-286 writes for the checker, the refusal's code, empty `items`, exit 20. | Test (TC-1584) |
| FR-624-AC-3 | A request whose snapshot names an undeclared population gives the binding refusal's code, empty `items`, exit 20. | Test (TC-1584) |
| FR-624-AC-4 | For each of `generate`, `prove`, `replay`, `inspect`, `render` and `monitor` invoked in a build that does not route it, the driver binary writes a document with that `operation`, category `unsupported`, code `unsupported_operation`, cause `operation-not-routed` and payload `verb` naming it, and exits 21; stdout is never empty. | Driver test (TC-1584): the `fsl` binary decides verb routing, so the driver code ticket QSL-1995 adds the test in the driver repository; the command adapter takes no verb. |
| FR-624-AC-5 | For the same unrouted invocations in text mode, the driver binary prints the named message on stderr and exits 21. | Driver test (TC-1584): text rendering exists only in the driver binary, so QSL-1995 adds the test in the driver repository. |

## Dependencies

- FR-285 owns the category-to-exit function; FSpec FR-522 adds the
  `unsupported_operation` code and `operation-not-routed` cause to the
  catalog and states the rule for every conforming CLI.
