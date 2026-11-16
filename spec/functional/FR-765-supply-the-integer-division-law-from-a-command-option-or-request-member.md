---
id: FR-765
title: "Supply the integer-division law from a command option or a request member"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-003
    type: implements
  - target: ix://agent-ix/filament-spec-language/US-029
    type: traces_to
  - target: ix://agent-ix/filament-spec-language/FR-495
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-670
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-026
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-027
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-100
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-275
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-620
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-644
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-662
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-480
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-287
    type: references
  - target: ix://agent-ix/filament-specification/FR-147
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-570
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-301
    type: depends_on
---
# FR-765: Supply the integer-division law from a command option or a request member

## Description

Integer `div` and `rem` need one of three laws: truncating, floor or
Euclidean (FSpec FR-147). A unit states its law in a header profile
(FR-670), and a library caller of the compile facade may supply one instead
(FR-495 "Public compile-facade division selection"). This requirement gives
the `fsl` command line and the request files the same caller-supplied input,
so a unit checked, compiled, run, analyzed, proved or monitored by `fsl` can
use `div` and `rem` exactly as a unit compiled through the library can.

The command option is `--integer-division <law>` on `check`. The request
member is `integer_division`, a string, at the top level and on each
`libraries` entry of every request that compiles a program: `native-compile/1`
(FR-027, `compile`), `native-run/1` (FR-100, `run`), `native-analyze/1`
(FR-620, `analyze` and `prove`) and `native-monitor/1` (FR-644, `monitor`). The three law names are
`euclidean`, `floor` and `truncating`, which select the definitions
`filament.value.integer-division.euclidean/v1`,
`filament.value.integer-division.floor/v1` and
`filament.value.integer-division.truncating/v1`.

## Rationale

The command line and the request file are how a person or a build script
calls the compile facade, so they take the inputs the facade takes. Without
them, a unit with no law header could use `div` and `rem` from a Rust
program and not from `fsl`, and the same unit would need a different text
for each route.

A supplied law does not give one text two meanings under one identity. The
law fills the package selection's `integer_division_profile` slot (FSpec
`value-complete.md`), which is part of the checked package's identity, so
two compiles of one text under two supplied laws produce two packages with
different identities. When the unit names a law in its header, the supplied
law can only agree with it; a disagreement is refused, so a supplied law
never overrides what the author wrote.

There is no default law. A default would choose a meaning the author never
stated, which FSpec FR-147 forbids, so a unit with `div` or `rem`, no header
law and no supplied law is still refused at each operator.

One short spelling per law keeps the option and the member easy to type and
to compare. The full definition identities stay the header's spelling.

## Inputs

- `fsl check`'s `--integer-division <law>` option, given at most once.
- The `integer_division` member of every request that compiles a program
  (`native-compile/1`, `native-run/1`, `native-analyze/1`, `native-monitor/1`),
  at its top level and on each `libraries` entry.
- The unit's header law profiles (FR-670).

## Outputs

- The admitted law, obtained through `admit_integer_division`, passed to
  spine compile and to the `check` lifecycle operation as the caller-supplied
  selection (FR-495, FR-670), and recorded in the package selection.
- Or a refusal: `invalid-request`, `unknown_profile`/`unsupported-selection`,
  `invalid_package`/`conflicting-definition`, or the existing
  `missing_declaration`/`missing-selection` at each operator.

## Behavior

### Law names

- The option value and the member value SHALL each be exactly one of
  `euclidean`, `floor` and `truncating`, compared byte for byte (`Floor` and
  a full definition identity are not law names).
- A value that is not a law name SHALL refuse
  `unknown_profile`/`unsupported-selection`, exit 20, before any source,
  model or library file is read. The refusal names where the value came from
  (`--integer-division`, `integer_division` or
  `libraries[<index>].integer_division`) and the value, and its message reads
  ``integer-division law `<value>` supplied by `<source>` is not one of
  `euclidean`, `floor`, `truncating` `` where `<source>` is that origin.

### The request member

- The request reader SHALL decode `integer_division` under FR-026's
  closed-envelope rules. A `null` member or a member that is not a string
  SHALL refuse `invalid-request` at stage `request`, exit 20, naming the
  member, before any dependent file is read.
- The top-level member SHALL apply to every program unit of the request,
  including each unit of a multi-unit `program` (FR-480). It SHALL NOT reach a
  library: a library entry takes only its own `integer_division` member, and
  a library compiled with neither a header law nor its own member refuses its
  `div` and `rem` as FR-670 states, whatever the program carries.
- A `native-run/1` or `native-analyze/1` request that selects an existing
  package (`package`) uses that package's recorded law and compiles nothing, so
  a request carrying both `package` and a top-level `integer_division` SHALL
  refuse `invalid-request` naming
  `integer_division`, exit 20, before any file is read.

### The command option

- `fsl check <file> --integer-division <law>` SHALL pass the admitted law to
  the `check` lifecycle operation for the checked unit, including every fence
  of `check --markdown`.
- The option given twice, even with the same value, SHALL exit 20 with the
  verb's usage on stderr and no stdout byte, as every repeated single-valued
  option does (FSpec FR-301).
- `compile`, `run`, `analyze`, `prove` and `monitor` read the law from their
  request file, so the option on any of them SHALL refuse as an unknown option, exit 20, before the request file
  is read (FSpec FR-301-AC-3). `parse` and `format` compile nothing and
  refuse it the same way.
- The `check` lifecycle operation (FR-275) SHALL take an optional admitted
  selection beside its other inputs, and with one SHALL check exactly as
  spine compile does with the same caller-supplied selection; with no caller
  selection it checks under the header law alone. The operation is
  `spine::check_with_integer_division(parsed, package_name, models,
  dependencies, &AdmittedIntegerDivision, limits, cancel)`, its multi-unit
  form `spine::check_units_with_integer_division` takes the same selection for
  every unit of an admitted unit list, and FR-662's `NativeFenceRequest`
  carries an optional `integer_division` that applies to every fence.

### Composing with the header law

The supplied law is FR-670's caller-supplied selection, so FR-670's rules
apply to it unchanged:

- A unit with no header law and a supplied law SHALL check, lower and run
  every `div` and `rem` under the supplied law.
- A unit whose header law names the supplied law SHALL select that law once.
- A unit whose header law names a different law SHALL refuse
  `invalid_package`/`conflicting-definition` at the header's identity
  literal, before checking, retaining both definition references, exit 20,
  with no package bytes. Its message reads ``integer-division law `<header
  law>` selected by header profile `<alias>` conflicts with `<supplied law>`
  supplied by `<source>` `` where `<source>` is `--integer-division`,
  `integer_division` or `libraries[<index>].integer_division`.
- A unit with neither SHALL refuse each `div` and `rem`
  `missing_declaration`/`missing-selection` for the `IntegerDivision` role at
  the operator. No law is chosen by default.
- A supplied law over a unit with no `div` or `rem` SHALL be admitted, and the
  package selection holds no integer-division role (the `integer_div_rem`
  trigger is absent), so the package bytes equal those of the same request
  without the law.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-765-AC-1 | For each law `L` of `euclidean`, `floor` and `truncating`, a `native-compile/1` request with `"integer_division": "L"` over FR-670-AC-1's unit without its law header compiles with exit 0 and a package whose selection holds `L`'s integer-division role; a `native-run/1` request with the same member calling `q` and `r` at `(-7, 3)` returns `-2` and `-1` under truncating and `-3` and `2` under floor and Euclidean. | Test (TC-2188) |
| FR-765-AC-2 | The AC-1 unit with a floor law header and `"integer_division": "floor"` compiles with the floor law; with `"integer_division": "truncating"` it refuses `invalid_package`/`conflicting-definition` at the header's identity literal, exit 20, with no package bytes and a message naming `floor`, the alias, `truncating` and `integer_division`. | Test (TC-2188) |
| FR-765-AC-3 | With the program file absent, a request whose `integer_division` is `null` and one whose value is `3` each refuse `invalid-request` at stage `request` naming `integer_division`; one whose value is `ceiling` and one whose value is `Floor` each refuse `unknown_profile`/`unsupported-selection` naming the member and the value; each exits 20 and none reads a file other than the request. A `native-run/1` request carrying `package` and `integer_division` refuses `invalid-request` naming `integer_division`. | Test (TC-2188) |
| FR-765-AC-4 | The AC-1 unit with no member and no header refuses `missing_declaration`/`missing-selection` at each `div` and `rem`, exit 20; a unit using only `mod` compiles with no member. | Test (TC-2188) |
| FR-765-AC-5 | A request whose top-level `integer_division` is `floor` and whose library entry, with no law header, carries `"integer_division": "truncating"` runs the library's `ld(a: Int[-7, 7], b: Int[1, 3]): Int[-7, 7] { a div b }` at `(-7, 3)` to `-2` and the program's `q(-7, 3)` to `-3`; the same request without the library entry's member refuses `missing_declaration`/`missing-selection` at the library's `div`. | Test (TC-2188) |
| FR-765-AC-6 | A request with `"integer_division": "floor"` over a unit whose only operators are `mod` and `+` writes package bytes equal to the same request without the member. | Test (TC-2188) |
| FR-765-AC-7 | Through the driver: `fsl check u.fsl --integer-division floor --format json` over FR-670-AC-1's unit without its law header exits 0; without the option it exits 20 with `missing_declaration`/`missing-selection`; with `--integer-division ceiling` it exits 20 with `unknown_profile`/`unsupported-selection` naming `--integer-division` and `ceiling` before reading `u.fsl`; with the option twice it exits 20 with usage on stderr and no stdout byte; with a floor header and `--integer-division truncating` it exits 20 with `invalid_package`/`conflicting-definition`. `fsl run` over a request with `"integer_division": "floor"` calling `q(-7, 3)` exits 0 with value `-3`, and `fsl run`, `fsl compile`, `fsl analyze`, `fsl prove` and `fsl monitor` with `--integer-division floor` exit 20 as unknown options. | Driver test (TC-2189): the `fsl` binary parses the option and renders the outcome, so the code ticket QSL-1989 adds the test in the filament-driver repository. |
| FR-765-AC-8 | `spine::check_with_integer_division` given the floor law over FR-670-AC-1's unit without its law header checks, its `div` and `rem` nodes typed under floor; the same unit through `spine::check` with no selection refuses `missing_declaration`/`missing-selection` at each operator; a Markdown document of two `ix:native` fences, each declaring a `div` function with no law header, checked through `check_native_fences` with the floor law reports both fences checked; and a `native-compile/1` request whose `program` is an array of two such units with `"integer_division": "floor"` compiles, every unit's `div` under floor. | Test (TC-2191) |
| FR-765-AC-9 | A `native-analyze/1` request whose program declares `function half using v(a: Int[0, 9]): Int[0, 9] pure { a div 2 }` beside a temporal claim, with `"integer_division": "floor"`, compiles its program and settles the claim's item; without the member it refuses the whole request `missing_declaration`/`missing-selection` at the `div`, exit 20, with no item settled; with `"integer_division": "ceiling"` it refuses `unknown_profile`/`unsupported-selection` before any file but the request is read. | Test (TC-2191) |
| FR-765-AC-10 | A `native-monitor/1` request whose program declares the AC-9 `half` function and a temporal clause `always holds(half(r.level) < 5)` over a `Reading` record, with `"integer_division": "floor"`, compiles its program and reports the clause over the observations; without the member it refuses `missing_declaration`/`missing-selection` at the `div`, exit 20, before any observation is read. | Test (TC-2191) |

## Dependencies

- FR-670 owns header law resolution and the caller-supplied selection rules
  this requirement reuses; FR-495 owns the compile facade, `admit_integer_division`
  and the `MissingSelection` refusal.
- FR-026, FR-027, FR-100, FR-620 and FR-644 own the request envelopes this
  member joins; FR-480 owns multi-unit programs; FR-275 owns the `check`
  lifecycle operation and FR-662 the Markdown fence check; FR-287 and FSpec FR-301 own the
  driver's verbs and argument refusal.
- FSpec FR-570 states the selection rule for every implementation; FSpec
  FR-147 owns the laws.
