---
id: FR-866
title: "Check a unit against the models and libraries it imports"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-029
    type: traces_to
  - target: ix://agent-ix/filament-spec-language/FR-027
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-056
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-099
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-287
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-767
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-869
    type: references
  - target: ix://agent-ix/filament-specification/FR-719
    type: implements
  - target: ix://agent-ix/filament-specification/FR-154
    type: references
---
# FR-866: Check a unit against the models and libraries it imports

## Description

`fsl check` checks one source file. A unit with a `model` line selects a
domain package by identity, and a unit with an `import` names a library by
identity; both have to be supplied for the unit to check. `fsl check`
SHALL take them the way a compile request does, so a reader who wrote a
rule over a model can check it with one command and no request file:

```
fsl check rule.fsl --model example/config-version=model.semantic-ir.json \
  --library test/geometry=geometry.fsl
```

### Decision: check takes the inputs, not a narrower scope

The alternative, documenting `check` as a command for units with no model,
would leave every model-backed rule without a check step. The inputs `check`
needs are the ones a compile request already carries, so `check` takes the
same two selections and hands them to the same intake.

### The options

- `--model <identity>=<file>`, repeatable: `<file>` is a `semantic-ir/2.0.0`
  domain package document supplied under `<identity>`, read and admitted
  exactly as one entry of a compile request's `models` member (FR-027,
  FR-056, FSpec FR-154). The unit's `model` declarations select it by
  identity.
- `--library <identity>=<file>`, repeatable: `<file>` is the source of the
  library the unit imports as `<identity>`, given the same two FR-001
  source labels the checked source gets from its path, and handed to S4 as
  one entry of the dependency input, as a compile request's `libraries`
  entry is (FR-099). A library's integer-division law is its unit header's
  law (FR-570, FR-767); a library that needs `integer_division` or
  `refinement_internals` from its request entry is checked through a compile
  request, which carries those members.
- The operand splits at its first `=`. A file path is resolved against the
  working directory.
- `--dependent-bytes <bytes>` sets FR-026's `dependent_bytes` ceiling for
  the model and library files (default 8 MiB); every model and library file
  counts toward it and toward FR-026's 64-file ceiling, as in a compile.

### The library entry

`fsl_command::command::check_outcome(source: &Path, inputs: &CheckInputs)`
returns the check's `OutcomeDocument` (operation `check`, FR-286). Its
`CheckInputs` holds the model selections and library selections as
`(identity, path)` pairs in option order, the optional integer-division law
(FSpec FR-301's `check --integer-division`), the caller's spine limits
(FR-869) and `dependent_bytes`. It reads every file through the command's
bounded intake, admits the models through `select` and checks the unit
through `check` with the libraries as its dependency input, the same
composition a compile request runs up to `package` (FR-278). The driver
calls it and reads no model or library file itself.

### Refusals

- An operand with no `=`, an empty identity or an empty path refuses
  `invalid-request`, exit 20, naming the option and the operand, before any
  file is read.
- The same identity given twice to `--model`, or twice to `--library`,
  refuses `invalid-request`, exit 20, naming the option and the identity,
  before any file is read, as FSpec FR-154 refuses a second document under
  one identity.
- A `--model` identity that no `model` declaration of the unit selects, or
  a `--library` identity that no `import` of the unit names, refuses
  `invalid-request`, exit 20, naming the option and the identity, after the
  source parses and before the file is read: an input that changes nothing
  is refused, as FR-640 refuses an unused clock selection.
- A unit whose `model` declaration selects an identity no `--model`
  supplies refuses `missing_import`/`missing-selection` at stage I1 naming
  the identity (FSpec FR-154). The diagnostic's message SHALL name the
  identity and the two inputs that supply it: the option
  `--model <identity>=<file>` and a compile request's `models` member. The
  same message is given by a compile request that omits the model.

## Inputs

The source path; the `--model`, `--library`, `--integer-division`,
`--dependent-bytes` and `--limit` options.

## Outputs

FR-286's `check` outcome document, in the format `--format` selects
(FR-865), and FR-285's exit code of its category.

## Behavior

- When `fsl check` is given `--model` or `--library` operands, it SHALL
  supply each model to `select` and each library to `check`'s dependency
  input, through `check_outcome`, with compile's intake and ceilings.
- `check_outcome` SHALL produce the same category, diagnostics and last
  stage that a compile request selecting the same program, models and
  libraries produces up to S4.
- If an operand is malformed or an identity is repeated, then `fsl check`
  SHALL refuse `invalid-request`, exit 20, naming the option, before any
  file is read.
- If a supplied model or library is selected by no declaration of the unit,
  then `check_outcome` SHALL refuse `invalid-request`, exit 20, naming the
  option and the identity.
- If a selected model is not supplied, then the `missing_import` diagnostic
  SHALL name the identity, `--model` and the request's `models` member.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-866-AC-1 | `check_outcome` over FR-108's ConfigVersion unit (`model Config = "example/config-version"`) with the model selection `("example/config-version", model.semantic-ir.json)` returns a `check` document with category success, `last_stage` S4 and no diagnostic; over the same unit with an ill-typed invariant it returns the `ill_typed` refusal a compile request over the same files returns, with equal code, cause, locus and stage. | Test (TC-2457) |
| FR-866-AC-2 | `check_outcome` over FR-027-AC-10's program importing `test/geometry`, with that library selected by file, returns success at S4; without the library it returns the `missing_import` refusal the compile request without `libraries` returns. | Test (TC-2457) |
| FR-866-AC-3 | A compile request over FR-108's unit with `models` `[]`, and `check_outcome` with no model selection, each refuse `missing_import`/`missing-selection` at stage I1 with a message naming `example/config-version`, `--model` and `models`. | Test (TC-2457) |
| FR-866-AC-4 | `check_outcome` with a model selection under `example/other` over FR-108's unit, and with any model selection over `tests/fixtures/spine-compile.native` (no `model` line), each refuse `invalid-request` naming `--model` and the identity, and read no model file; a library selection the unit does not import refuses the same way naming `--library`. | Test (TC-2457) |
| FR-866-AC-5 | `fsl check im.fsl --model example/config-version=m.json` over the unit `model Config = "example/config-version"; invariant NoCycle using v on Config::ConfigVersion at current { not reaches(self, self, parent) }` exits 0; without `--model` it exits 20 with `missing_import` naming the identity and `--model`; `--model example/config-version=m.json` given twice, `--model m.json` (no `=`) and `--model =m.json` each exit 20 `invalid-request` naming `--model` before reading `m.json`. | Driver test (TC-2458): the `fsl` binary parses the options, so the code ticket adds the test in the filament-driver repository. |

## Dependencies

- [FR-027](FR-027-export-compiled-native-package.md): the compile request's
  `models` and `libraries` and their intake.
- [FR-056](FR-056-admit-domain-package-model-declarations.md) and FSpec
  FR-154: model admission by identity.
- [FR-099](FR-099-compile-against-supplied-libraries.md): the dependency
  input.
- [FR-287](FR-287-reach-fsl-through-the-driver-cli.md): the driver's verbs.
- [FR-869](FR-869-raise-every-limit-of-parse-check-and-compile-from-the-command-line.md):
  `--limit` on `check`.
- FSpec FR-719: the CLI's half.
