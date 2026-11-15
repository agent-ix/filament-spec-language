---
id: FR-762
title: "Report every independent syntax error of a unit"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-001
    type: traces_to
  - target: ix://agent-ix/filament-spec-language/FR-002
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-503
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-255
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-460
    type: references
---
# FR-762: Report every independent syntax error of a unit

## Description

When a source unit holds more than one syntax error, `fsl_cst::parse` SHALL
report each independent error as its own located diagnostic, in source order,
and SHALL keep parsing after each one. The parser recovers at declaration
level: after an error it skips to the next declaration start, records the
error once, and parses the rest of the unit as it would parse it without the
skipped text. An author who makes two mistakes in two declarations sees both
in one run.

## Rationale

A parser that stops at the first error makes the author fix, rerun and meet
the next error, one round per mistake. Declarations are independent units of
meaning, so an error inside one says nothing about the next, and the next
declaration keyword is a reliable place to resume. Reporting a follow-on
error that only exists because of an earlier one (a cascade) would bury the
real errors, so each skipped region yields exactly one diagnostic.

## Inputs

- Source bytes and their `SourceIdentity`, as FR-002 states.
- The S1 limits, including the diagnostic bound `s1.diagnostics` (FR-255).

## Outputs

A recovered `ParsedSource` that retains every source byte, holds every
reported syntax diagnostic in source order, holds the CST of every declaration
that parsed, and is not admissible; or, for a unit with no syntax error, the
same `ParsedSource` FR-002 produces.

## Behavior

### Lexical errors

- The lexer SHALL continue after an invalid token. Each maximal run of
  characters that begins no token SHALL give one `invalid_syntax`/
  `invalid-token` diagnostic located at that run, and the run SHALL be kept in
  the CST as an error leaf so the CST still renders every source byte.
- The parser SHALL run over the significant tokens with the invalid runs
  removed. A parse error inside a declaration that already holds a lexical
  diagnostic is a cascade of that diagnostic and SHALL NOT be reported.

### Synchronization

- A **declaration start** is a significant token that begins a top-level
  declaration form of the unit: a declaration keyword of the complete-V1
  grammar or of a selected extension (FR-354), or a header keyword
  (`language`, `profile`, `import`, `model`).
- After a parse error at token position `p`, the parser SHALL resume at the
  first declaration start after `p` that stands at delimiter depth zero,
  counting `(`, `[` and `{` against their closers from the first token of the
  declaration that holds `p`.
- When the declaration's delimiters do not close before the end of the
  source, the parser SHALL resume at the first declaration start after `p`
  that is the first significant token of its line, and the declaration's open
  delimiters are recovered as missing closers. This generalizes FR-503's
  protocol rule to every declaration form; FR-503's attribution of the
  diagnostic to one protocol declaration is unchanged.
- In the unit header, the parser SHALL resume after the next `;` at depth zero
  or at the next declaration start, whichever comes first.
- When no resumption point exists, the parser SHALL stop recovering, and the
  error SHALL be the unit's last syntax diagnostic.
- The skipped tokens, from the failed declaration's first token to the
  resumption point, SHALL be one error node of the CST, carrying the
  diagnostic's `Recovery`.

### One diagnostic per error

- Each skipped region SHALL yield exactly one diagnostic: the first failure
  inside it, with the code, cause, locus and message FR-002 gives a unit that
  holds that error alone. No second diagnostic is reported for a token inside
  a skipped region.
- A missing `;` at the end of a declaration is one error: the next
  declaration start is the resumption point, and the next declaration parses
  as it would after a correct `;`.
- A unit with exactly one syntax error SHALL give exactly the one diagnostic,
  cause, locus and message it gives without recovery.
- Diagnostics SHALL be ordered by the start byte of their locus; a lexical
  diagnostic and a parse diagnostic at the same start byte are ordered
  lexical first.

### The diagnostic bound

- The number of syntax diagnostics one parse reports is bounded by the S1
  setting `s1.diagnostics` (FR-255), default 100. When the parser finds an
  error after it has already reported `s1.diagnostics` of them, it SHALL stop
  recovering and append one final diagnostic: code `invalid_syntax`, cause
  `diagnostic-list-cut`, located at the first unreported error's token, with
  the message `the diagnostic list was cut after <n> syntax errors; raise
  s1.diagnostics to see the rest`, `<n>` the bound.
- Reaching `s1.diagnostics` SHALL NOT make the outcome incomplete: the unit is
  already refused, and the bound limits only how much of the refusal is
  reported. The outcome stays a refusal with exit code 20.
- When `s1.work_units` is exhausted while the parser recovers after at least
  one recorded error, the parser SHALL stop recovering and append the same
  `diagnostic-list-cut` diagnostic with the message `the diagnostic list was
  cut after <n> syntax errors because s1.work_units was reached; raise
  s1.work_units to see the rest`, `<n>` the count reported. The outcome is
  still the refusal, for the same reason. An exhaustion before any error is
  recorded is FR-002's `stage_limit_exceeded`.
- The FR-663 `parsed` report's `usage` keeps its four resource settings; the
  diagnostic bound bounds the report, not the parser's work.

### Admission and cost

- A recovered `ParsedSource` SHALL NOT be admissible, whatever it holds, and
  the `parse`, `check` and `compile` operations SHALL refuse it at S1 as
  FR-002 states. Recovery never admits a declaration to S2 on its own.
- A unit with no syntax error SHALL be parsed in one pass with no recovery
  work: recovery starts only after a failure.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-762-AC-1 | `fsl_cst::parse` over a unit of three `record` declarations in which the first is missing a `;` between two fields and the third is missing a field type returns a `ParsedSource` that is not admissible and holds exactly two `invalid_syntax`/`unexpected-token` diagnostics, in source order, the first at the first record's offending token and the second at the third record's. | Test (TC-2148) |
| FR-762-AC-2 | A unit holding `$` in one declaration and `$` in a later declaration gives exactly two `invalid_syntax`/`invalid-token` diagnostics, each at its own `$`, and no `unexpected-token` diagnostic. | Test (TC-2149) |
| FR-762-AC-3 | In AC-1's unit, the second record's `Declaration` node is present in the recovered CST and its span covers exactly the second record's text; the CST renders the unit's exact bytes. | Test (TC-2150) |
| FR-762-AC-4 | A unit of two `record` declarations whose first lacks its final `;` before the second gives exactly one diagnostic, and the second record's `Declaration` node is present. | Test (TC-2151) |
| FR-762-AC-5 | Each unit of FR-002-AC-2 (`a < b < c`, `a = b != c`, `a < b + c = d`), which holds one error, gives exactly the one diagnostic, cause and locus it gives without recovery: `invalid_syntax` at the second comparison operator. | Test (TC-2152) |
| FR-762-AC-6 | A unit of 150 `record` declarations, each missing a field type, under the default limits gives 101 diagnostics: 100 `unexpected-token` diagnostics, then one `invalid_syntax`/`diagnostic-list-cut` diagnostic located at the 101st record's offending token whose message names `s1.diagnostics` and the bound 100. The outcome is a refusal, not `stage_limit_exceeded`. | Test (TC-2153) |
| FR-762-AC-7 | AC-1's recovered `ParsedSource` is not admissible and still holds both diagnostics after recovery; FR-002's `parse` operation over it refuses at S1 with category `refusal` and exit 20, so no later stage receives it. | Test (TC-2154) |
| FR-762-AC-8 | Over AC-1's unit, under the smallest `s1.work_units` at which `fsl_cst::parse` records the first record's error rather than stopping with `stage_limit_exceeded`, the report ends with one `invalid_syntax`/`diagnostic-list-cut` diagnostic whose message names `s1.work_units`, no diagnostic carries a limit, and the `ParsedSource` is not admissible (the outcome is a refusal). | Test (TC-2187) |
| FR-762-AC-9 | A unit whose header line `profile v = ;` lacks its identity string and whose later `record A { x: Integer y: Integer; }` lacks a `;` gives exactly two diagnostics, the first at the header's `;` and the second at the record's `y`; the record's `Declaration` node is present. | Test (TC-2187) |
| FR-762-AC-10 | A unit whose last declaration holds a syntax error and is followed by no declaration start gives that error as its last diagnostic, with no diagnostic after it and no `diagnostic-list-cut`. | Test (TC-2187) |

## Dependencies

- [FR-002](FR-002-parse-native-units.md): S1 parsing and the one-error diagnostic.
- [FR-503](FR-503-confine-a-protocol-declaration-s-refusal-to-that-declaration-at-every-stage.md): the protocol delimiter recovery this generalizes.
- [FR-255](FR-255-name-the-setting-that-raises-a-reached-limit.md): the `s1.diagnostics` setting.
- FSpec FR-460: diagnostic counts are an admitted resource limit kind.
