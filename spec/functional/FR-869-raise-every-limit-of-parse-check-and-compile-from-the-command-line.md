---
id: FR-869
title: "Raise every limit of parse, check and compile from the command line"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-027
    type: traces_to
  - target: ix://agent-ix/filament-spec-language/FR-255
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-256
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-026
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-027
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-287
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-866
    type: references
  - target: ix://agent-ix/filament-specification/FR-720
    type: implements
  - target: ix://agent-ix/filament-specification/FR-461
    type: references
---
# FR-869: Raise every limit of parse, check and compile from the command line

## Description

Nesting depth is never a limit (FR-256, ADR-030): a deeper source needs only
larger node, token and work budgets, and FR-255 names the setting that
raises each one. That holds at the command line only when the command that
reached the limit accepts the setting. `fsl parse`, `fsl check` and
`fsl compile` SHALL therefore accept `--limit <name>=<value>`, repeatable,
through FR-255's settings operation, for every setting of every stage the
command runs, and `fsl check` and `fsl compile` the intake byte ceilings, so
a limit any of them reports can be raised on that same command. `fsl run`
keeps its existing options (FR-287-AC-4).

### The settings each command accepts

| Command | `--limit` settings | Intake options |
| --- | --- | --- |
| `parse` | every `s1.*` row of FR-255's setting table (`fsl_cst::Limits`) | none: the source is the one file, bounded by `s1.input_bytes` |
| `check` | every setting `SpineLimits` carries except `emit.output_bytes`: `s1.*`, `intake.*`, `model.*`, `s3.*`, `environment.*`, `identity.input_bytes`, `dependency.*`, `i2.*` and `progress.covered_positions` | `--dependent-bytes` for its model and library files (FR-866) |
| `compile` | every setting `SpineLimits` carries, `emit.output_bytes` included | `--request-bytes` and `--dependent-bytes`, with FR-026's defaults, as on `run` |

### Decision: a setting a command never charges is refused

A setting whose stage the command does not run, such as `accounting.*`,
`admission.*`, `observation.*`, `prepare.*`, `temporal_evidence.*` or an
engine setting on `compile`, bounds nothing that command does. Accepting it
would tell the caller a limit was applied when none was, so the command
refuses it, naming the setting and the settings it accepts. Each such
setting is accepted on the command that runs its stage (`run`, `analyze`,
`monitor`, `prove`), so no limit is unreachable from the command line.

### Decision: compile's intake ceilings become options

FR-027 gave `compile` fixed 1 MiB request and 8 MiB dependent-file ceilings.
A program whose models and libraries exceed 8 MiB could then be compiled
only from the library. `compile` takes the same `--request-bytes` and
`--dependent-bytes` options as `run`, with the same defaults and no ceiling
of their own, and a refusal at either names the option that raises it.

### The library entry

FR-640's `RequestOverrides` SHALL gain `limits: Option<&SpineLimits>`, the
caller's stage limits from the settings operation, and the compile entry
SHALL take FR-026's `RunLimits` for its intake ceilings
(`compile_outcome_with_limits_and_overrides(path, limits, overrides)`).
A compile that is given no limits runs at FR-255's defaults (FR-255-AC-6).
`check_outcome` takes the same two values (FR-866). `parse` takes the S1
limits as its `fsl_cst::Limits` argument (FR-275).

## Inputs

`--limit <name>=<value>` operands, repeatable; `--request-bytes` and
`--dependent-bytes` on `compile`, `--dependent-bytes` on `check`.

## Outputs

The command's outcome under the configured limits.

## Behavior

- When `fsl parse`, `fsl check` or `fsl compile` is given `--limit`
  operands, the driver SHALL pass them to FR-255's settings operation and
  run the command's stages under the limits it returns.
- If an operand names a setting the command does not accept, then the
  driver SHALL refuse `invalid-request`, exit 20, naming the setting and
  listing the settings the command accepts, before any file is read.
- If an operand is malformed, names an unknown setting, has a value that is
  not a non-negative decimal integer or repeats a name, then the driver
  SHALL return FR-255 Behavior 6's usage refusal naming the operand, exit 20,
  before any file is read.
- When `fsl compile` is given `--request-bytes` or `--dependent-bytes`, the
  compile SHALL read the request and its dependent files under those
  ceilings, and a refusal at either SHALL name the limit, its value and the
  option that raises it.
- When no limit is configured, each command SHALL run at FR-255's and
  FR-026's defaults.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-869-AC-1 | A compile request whose request file is one byte over 1 MiB refuses at intake naming `request_bytes`, its maximum and the option `--request-bytes`; through `compile_outcome_with_limits_and_overrides` with `RunLimits::request_bytes` at the file's size it compiles. FR-027-AC-13's request one byte over `dependent_bytes` refuses naming `--dependent-bytes` and compiles with `dependent_bytes` raised to fit. | Test (TC-2461) |
| FR-869-AC-2 | `compile_outcome_with_limits_and_overrides` over a program holding a 6,000-deep parenthesis nest refuses `stage_limit_exceeded` naming `s1.nodes` under the default limits and compiles with `RequestOverrides::limits` carrying `s1.nodes` 1000000 and the `s1.tokens` and `s1.work_units` it needs; `emit.output_bytes` set to one below the package's byte length refuses naming `emit.output_bytes`. | Test (TC-2461) |
| FR-869-AC-3 | `fsl parse a b deep.fsl --limit s1.nodes=1000000` over the 6,000-deep nest exits 0, and without `--limit` exits 22 naming `s1.nodes`; `fsl check deep.fsl --limit s1.nodes=1000000` exits 0; `fsl compile req.json --limit s1.nodes=1000000` over a request selecting it exits 0. | Driver test (TC-2462): the `fsl` binary parses the options, so the code ticket adds the test in the filament-driver repository. |
| FR-869-AC-4 | `fsl parse a b x.fsl --limit s3.nodes=5`, `fsl check x.fsl --limit accounting.work_units=5`, `fsl check x.fsl --limit emit.output_bytes=5` and `fsl compile req.json --limit admission.work_units=5` each exit 20 `invalid-request` naming the setting and the command's accepted settings, before reading a file; `--limit s9.nodes=1`, `--limit s1.nodes=ten` and `--limit s1.nodes=5 --limit s1.nodes=6` each refuse with FR-255 Behavior 6's usage refusal naming the operand. | Driver test (TC-2462) |

## Dependencies

- [FR-255](FR-255-name-the-setting-that-raises-a-reached-limit.md): the
  setting names and the settings operation.
- [FR-256](FR-256-parse-source-at-any-nesting-depth.md): any nesting depth.
- [FR-026](FR-026-run-standalone-native-workflow.md) and
  [FR-027](FR-027-export-compiled-native-package.md): the intake ceilings.
- [FR-287](FR-287-reach-fsl-through-the-driver-cli.md): the driver's verbs.
- [FR-866](FR-866-check-a-unit-against-the-models-and-libraries-it-imports.md):
  `check`'s model and library files.
- FSpec FR-720: the CLI's half; FSpec FR-461: the naming rule. The wording
  of limit messages beyond the setting and option they name is FR-255's and
  FSpec FR-461's.
