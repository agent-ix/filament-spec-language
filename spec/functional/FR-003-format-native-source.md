---
id: FR-003
title: "Format validated source without changing token meaning"
type: FR
relationships:
  - target: "ix://agent-ix/filament-spec-language/US-001"
    type: traces_to
  - target: "ix://agent-ix/filament-spec-language/ADR-011"
    type: depends_on
  - target: "ix://agent-ix/filament-spec-language/US-035"
    type: traces_to
  - target: "ix://agent-ix/filament-spec-language/FR-255"
    type: references
---
# FR-003: Format validated source without changing token meaning

## Description

When validated source is formatted, the formatter shall preserve every token spelling and comment.

## Inputs

The S1 output `fsl_cst::ParsedSource` (its lossless CST and exact source)
and an optional selected output-byte ceiling. `format` is the layer-1
library operation `fsl_cst::format`, beside `cst` in crate `fsl-cst`,
depending on F only (ADR-011 §6.1 layer 1 and X-12; ADR-029 OP-1). ADR-011
§6.2 and §7.3 (M-6a row) retarget it from the arena parse to the CST.

## Outputs

Formatted UTF-8 source or output-budget incompleteness.

## Behavior

`format` admits a `ParsedSource` only when `ParsedSource::is_admissible()`
holds. It refuses any other input with a typed cause that holds the input's
first diagnostic code, and returns no output. ADR-011 §2.3's E1 row permits
a formatter to consume a recovering CST without requiring it, and this
requirement formats validated source only.

`format` formats complete-V1 source. A native-edition source (for example
edition `0-draft`) parses as a recovering CST that carries `unknown_edition`,
so `format` refuses it with that code, exit code 20. Native-edition
`format` retires in M-6a with no replacement (ADR-011 §7.3 M-6a row;
ADR-011 Rulings 2026-09-22, native-edition `format`).

Formatting changes whitespace and normalizes layout to LF while retaining grouping and original token order. The library returns the complete string and leaves files untouched; writing it to stdout is the driver's (FR-287, Overlap). The `format` outcome document carries the same string as its `result` (FR-286), so a JSON caller gets the text a plain-text caller gets.

The format(source) API applies the published default output limit of 1 MiB. The
typed `FormatLimits.output_bytes` field is the `format.output_bytes` setting in
FR-255, and `format_with_limits(source, limits)` consumes that value.
`format_with_limit(source, output_bytes)` is the convenience form that builds
the same limits value. Both forms accept any limit, including zero and values
above the default, and use it as given. A refusal at the limit uses the
existing `resource_exhausted`/`input-bytes-exceeded` catalog cause for
compatibility with FSL's shared limit kind, while its rendered diagnostic
calls the condition a format output-byte limit, names `format.output_bytes`,
reports the configured value and emitted UTF-8 byte count, and identifies
`format_with_limit` as the way to raise it. It SHALL NOT describe the emitted
count as input or source bytes.
The byte limit is inclusive and counts spaces, indentation, comments and the
final newline.
Every append is checked before its growth; a refused operation returns no
partial string. Allocation capacity is an implementation detail and is not a
separate heap-accounting promise. Reuse the existing declarative token vocabulary.

### Canonical style

The formatter's output is the canonical layout of FSL source, and it is the
form the standard's grammar text writes (`present(x)`, `forall(r in c: p)`,
`over (v: T)`, `Int[0, 9]`, `eventually[0, 5] holds(p)`). Documentation and
examples show source as the formatter prints it. The formatter inserts,
deletes and reorders no significant token; it decides only the whitespace
between tokens, by these rules, which use the CST to tell the role of a
token:

- **Indentation and lines.** A `{` ends its line, and the lines inside it are
  indented two spaces deeper than the line that opened it. A `}` stands on its
  own line at the opening line's indentation. After a `}`, a following `;`,
  `,`, `)` or `]` stays on the `}` line, a following `{` stays on it after one
  space (`} {`, as after a hyper clause's `match { .. }` block), and any other
  token starts a new line, at every depth: a protocol `run`, `parallel`,
  `await`, `finish` or other statement after a closing brace begins its own
  line, never `} run ..`. A `;` ends its line, except inside `[..]`, where it is
  followed by one space (`Decimal[0, 9; 0, 2; exact]`). Blank lines are not kept, a
  comment stands on its own line, line ends are LF, and the output ends in one
  newline.
- **Parentheses.** No space precedes a `(` that opens an argument or
  parameter list: after an identifier or qualified name (a call, a
  constructor, `function f(` and `using v(`), after a built-in form's word
  (`present`, `value`, `deref`, `ref`, `pre`, `size`, `contains`, `forall`,
  `exists`, `filter`, `map`, `collect`, `flatMap`, `flatten`, `dropFirst`,
  `removeOne`, `fold`, `reduce`, `count`, `sum`, `only`, `seq`, `holds`,
  `reaches`, `rational`, `decimal`, `float32`, `float64`, `Text`, `Some`,
  `decreases`), and after the `>` that closes a type-argument list
  (`convert<T>(`, `size<N>(`). One space precedes every other `(`: a binding
  list after a keyword (`over (v: T)`, `each (e: E)`, `when (..)`,
  `as (r: Boolean)`) and a grouping parenthesis after an operator or keyword
  (`not (a or b)`, `always (..)`). No space follows `(` or precedes `)`.
- **Brackets.** No space precedes a `[` that opens type bounds (`Int[0, 9]`,
  `Sequence<Integer>[0, 4]`), an index (`xs[0]`), a collection value
  (`sequence[1, 2]`) or the interval of a temporal operator or window
  (`always[0, 30]`, `eventually[0, 5]`, `past[0, 1]`). One space precedes any
  other `[` (`over [0, 10000]`, `join all [a, b]`). No space follows `[` or
  precedes `]`.
- **Type arguments.** No space follows a `<` that opens type arguments, and
  no space precedes the `>` that closes them (`Option<Int[0, 9]>`, `count<N>`,
  `convert<T>`); no space precedes that `<`. What follows the closing `>` is
  spaced by the next token's own rule: none before `(` or `[`
  (`convert<T>(e)`, `Sequence<Integer>[0, 4]`), one before an operator
  (`Option<Int[0, 9]> = none`), and one before a word or `{`. A comparison `<`, `<=`, `>`, `>=` has one space on each side.
- **Separators.** A `:` has no space before it and one after it (parameters,
  fields, binders, `let x: T`, record field values, `forall(x in c: p)`). A
  `,` has no space before and one after; a `;` has no space before it, ends
  its line outside brackets and is followed by one space inside `[..]`.
  `.`, `::` and `?` touch their neighbours.
- **Operators.** Every binary operator, `@`, `^`, `=`, `in`, `implies` and the
  temporal connectives have one space on each side; unary `-` touches its
  operand (`-1`); `not` is followed by one space.
- **Words.** Two adjacent words, a word and a literal, and a word and a `{`
  are separated by one space, and so are a closing `)`, `]` or type-argument
  `>` and a following word or `{` (`(r: Boolean) {`, `Int[0, 9] pure`,
  `Option<Int[0, 9]> pure`).

Formatting keeps meaning: checking the formatted source yields the same
checked package identity as checking the original source.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-003-AC-1 | Formatting preserves the ordered token spellings. | Test (TC-013) |
| FR-003-AC-2 | Formatting preserves comments. | Test (TC-013) |
| FR-003-AC-3 | A second formatting pass produces identical bytes. | Test (TC-013) |
| FR-003-AC-4 | Output beyond its selected ceiling receives `resource_exhausted`/`input-bytes-exceeded`; its rendered diagnostic calls this a format output-byte limit, reports the configured bound and emitted UTF-8 byte count, names `format.output_bytes`, and does not present the emitted count as input or source bytes. | Test (TC-931) |
| FR-003-AC-5 | Exactly-at-ceiling output succeeds, including the final newline; zero or one-byte-short ceilings refuse without returning a partial string. | Test (TC-016) |
| FR-003-AC-6 | format(source) and format_with_limit(source, 1048576) return identical bytes. Source whose formatted output exceeds 1 MiB refuses under format(source) naming the limit, its value and format_with_limit, and formats in full under format_with_limit with a limit raised to fit it. | Test (TC-016) |
| FR-003-AC-7 | format and format_with_limit take a `fsl_cst::ParsedSource`. `fsl-cst/src/format.rs` depends on layer 1 and layer F only, and formatting complete-V1 source succeeds under AC-1 to AC-3. | Test (TC-404) |
| FR-003-AC-8 | format and format_with_limit refuse a `ParsedSource` whose CST carries a recovery, and one that carries a diagnostic and no recovery, each with a typed cause and no output string; neither call panics. | Test (TC-404) |
| FR-003-AC-9 | For each complete-V1 fixture unit in the repository's tests that checks, checking its formatted source through the S1 to S4 spine yields a checked package identity equal to the original's, and formatting the formatted source again yields identical bytes. | Test (TC-885) |
| FR-003-AC-10 | `FormatLimits` maps its only field, `output_bytes`, to FR-255's `format.output_bytes` through `SettingLimits`; `with_output_bytes` changes only that field, and the settings operation's `format.output_bytes=<n>` operand supplies the resulting value to `format_with_limits`. | Test (TC-931) |
| FR-003-AC-11 | Formatting a protocol whose `run sequence Main { .. }` block is followed by `parallel Both { .. }`, `await Refund ..` and `finish Closed as (r: Boolean) { .. }` prints each of `parallel`, `await` and `finish` at the start of its own line and no line holding `}` followed by a word; the significant token sequence of the output equals the input's, so no `;` is inserted between `await` clauses. | Test (TC-2783) |
| FR-003-AC-12 | Formatting `present (x)`, `forall (r in c : p)`, a parameter `(v : T)`, `Int [0,9]`, `always [0,30] holds (p)`, `convert < T > (e)`, `Option < Int[0,9] >` and `a<b` prints `present(x)`, `forall(r in c: p)`, `(v: T)`, `Int[0, 9]`, `always[0, 30] holds(p)`, `convert<T>(e)`, `Option<Int[0, 9]>` and `a < b`; `over (o: M::O)`, `when (w.l = 0)` and `as (r: Boolean)` keep one space before `(`; removing all whitespace from input and output gives equal bytes. | Test (TC-2783) |
| FR-003-AC-13 | Formatting `hyper NI using inf over behaviours (v: V::Vault) forall trace a of V forall trace b of V match { a.step.op = b.step.op } { always holds(v.l @ a = v.l @ b) }` prints the `match` block's closing brace and the body's opening brace as `} {` on one line, and `v.l @ a` with one space on each side of `@`. | Test (TC-2784) |
| FR-003-AC-14 | Every unit of the complete-grammar corpus formats; for each, formatting the output again gives identical bytes, the output's significant token sequence equals the input's, and the output parses to the same production tree. | Test (TC-2784) |
| FR-003-AC-15 | Formatting `function f using v(): Boolean pure { let o: Option < Int[0,9] > = none in present(o) }` prints `let o: Option<Int[0, 9]> = none in present(o)`; formatting `function g using v(x: Option<Int[0,9]>): Option<Int[0,9]>pure { x }` prints `g using v(x: Option<Int[0, 9]>): Option<Int[0, 9]> pure {`; formatting `function d using v(x: Decimal[0,9;0,2;exact]): Boolean pure { true }` prints `x: Decimal[0, 9; 0, 2; exact]` on one line; formatting a protocol `finish Closed as (r: Boolean){ true };` prints `finish Closed as (r: Boolean) {`. | Test (TC-2783) |

## Dependencies

- [US-001](../usecase/US-001-author-native-source.md) supplies the user need.
- [ADR-011](../decisions/ADR-011-stage-dag-and-dependency-architecture.md)
  §6.1 (layer 1), §6.2 (`format` row) and §7.3 (M-6a row) set the input
  to the CST.
- [Detailed contract or implementation evidence](../../fsl-cst/src/format.rs) supplies the scoped context.

## References

- [FR-255](FR-255-name-the-setting-that-raises-a-reached-limit.md) owns the
  `format.output_bytes` setting row and shared limits/settings seam that this
  formatter contract qualifies.
