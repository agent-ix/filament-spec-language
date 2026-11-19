---
id: FR-284
title: "Keep the qualified core separable by crate"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-029
    type: traces_to
  - target: ix://agent-ix/filament-spec-language/ADR-029
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/ADR-011
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-282
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-059
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-060
    type: references
---
# FR-284: Keep the qualified core separable by crate

## Description

The qualified core is the check path (S0 to S4), the prove path (E5 to S8,
with replay through S6a) and the certificate checkers through which
`analyze` enters it (ADR-029 CB-2, FR-282). Its FSL crates are
`fsl-foundation`, `fsl-cst`,
`fsl-source`, `fsl-forms`, `fsl-semantics`, `fsl-package`, `fsl-eval`,
`fsl-route` and `fsl-replay`. Outside FSL it also holds the driver library
`filament-driver`, the exact-value kernel `filament-exact` and the semantic-value
leaf `filament-semantic-value` (each its own repository, `agent-ix/filament-exact`
and `agent-ix/filament-semantic-value`, which `fsl-foundation`, `fsl-semantics`
and other core crates depend on), the walker toolkit `filament-walk` (ADR-030 FR-356, its own
repository `agent-ix/filament-walk`, which `fsl-forms` and `fsl-semantics`
depend on) and the CG, IR and RT crates the prove path reaches, under
their own records.

The crates above the core are FSL's `fsl-analyze` (layer A), `fsl-inspect`
(layer P), `fsl-jit` (layer X) and `fsl-command` (the FR-027 and FR-100
request-file adapter), `fsl-aot` (the AOT backend), and the driver's `filament-plugin-host`
and `fsl-cli` (ADR-029 CB-3 item 3). Each may
depend on the core. No core crate depends on any of them.

Separability lets a qualified binary be a thin frontend over the core with
the verbs `check`, `compile`, `prove`, `replay`, `check-certificate` and
`version`, JSON output and FR-285's exit function, added with no core change
(ADR-029 CB-3 item 4).

## Inputs

The FSL workspace's dependency graph, as `cargo tree` reports it for each
core crate.

## Outputs

The direction check's report: pass, or each offending (core crate,
dependency) pair.

## Behavior

- No FSL core crate shall depend, directly or transitively through normal
  dependencies, on:
  - an FSL workspace crate outside the core set;
  - a crate above the core: `fsl-analyze`, `fsl-inspect`, `fsl-jit`,
    `fsl-command`, `fsl-aot`, or the
    driver's `filament-driver`, `filament-plugin-host`
    or `fsl-cli`;
  - a CG or RT crate;
  - an argument parser, a terminal or colour crate, a renderer other than
    the JSON serialization of its own outcome types, a plugin host,
    or an execution backend other than the S6a interpreter.
- A third-party or first-party library crate outside those categories is
  allowed, including the IR crates ADR-011 §6.1 sanctions
  (`filament-contract-model`). Proc-macro crates and build and dev dependencies
  link nothing into a core crate and are not walked.
- The direction check shall run over the `cargo tree` of each core crate,
  with ADR-011 T-12's tooling, and fail naming each offending pair.
- No FSL core crate shall read an environment variable, a configuration
  file, a search path, a clock or a global registry. A monotonic counter,
  such as an atomic token counter, is not a global registry.
- No FSL core crate shall write to stdout or stderr, or end the process.
- Each FSL core crate shall read the bytes it needs from its request, by
  identity where an identity names them (ADR-013 O-26).
- Two temporary exemptions cover filament-core-data code that FSL cannot
  change:
  - The direction check shall not follow the edge from
    `agent-ix-extraction-frontend` to `clap`, which that crate uses only for
    its own binary. Only `fsl-semantics` may depend on
    `agent-ix-extraction-frontend`; an edge into it from any other crate
    under a core crate fails, so every other path from a core crate to
    `clap` fails. The exemption ends when the extraction frontend puts
    `clap` behind a non-default feature.
  - The ambient-input scan shall not report filesystem access inside
    `fsl-semantics` `model::intake::lift_document`, which creates a scratch
    directory because filament-core-data's `lift` writes its document only
    to an output path. Any other ambient access in that function, and
    filesystem access anywhere else, fails. The exemption ends when
    filament-core-data offers a `lift` that returns the document bytes.

  When an exemption ends, the check's code for it and its tests are
  deleted.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-284-AC-1 | The direction check passes over the FSL workspace. With a test manifest that adds `fsl-analyze` to `fsl-eval`'s dependencies, and with one that adds an argument-parser crate to `fsl-replay`'s, it fails and names the pair `(fsl-eval, fsl-analyze)` and the pair `(fsl-replay, <that crate>)`. It skips only Behavior's `agent-ix-extraction-frontend` → `clap` exemption: `fsl-semantics` reaching `clap` any other way, or any other crate depending on `agent-ix-extraction-frontend`, fails. | Test (TC-767) |
| FR-284-AC-2 | Running FR-275-AC-1's chain and FR-098's replay of a fixture counterexample in a process whose environment holds arbitrary values for every variable the test generates, whose working directory is an empty temporary directory and whose home directory is unset, gives outcomes equal to those of a run in the test's own environment. | Test (TC-768) |
| FR-284-AC-3 | During AC-2's runs, the bytes written to the process's stdout and stderr by core crates are empty, and the process exits only when the test harness ends it. | Test (TC-768) |
| FR-284-AC-4 | The ambient-input scan of each FSL core crate's shipped source, `#[cfg(test)]` code excluded, fails naming the file and line of each environment read, clock read, filesystem or search-path access (a path method such as `.exists()` or `.metadata()` included), mutable global (a `static mut`, or a `static` lock, cell, once-cell or lazy value), stdout, stderr or stdin access and process exit, and finds none of them in a comment, a string literal or test code. It skips only Behavior's `lift_document` filesystem exemption. | Test (TC-768) |

## Dependencies

- ADR-029 CB-2, CB-3: the core and separability.
- ADR-011 T-12: the dependency-direction tooling.
- ADR-013 O-26: bytes by identity.
- [FR-059](FR-059-check-backend-dependency-direction.md), [FR-060](FR-060-check-fsl-api-surface-boundary.md): the existing direction and API-surface checks this one extends.
- [FR-282](FR-282-check-analyze-certificates-in-the-qualified-core.md): the certificate checkers.

## Overlap

The driver repository owns the qualified binary's verbs and the direction
check over `filament-driver`.

## References

- QSL-390 (ARCH-50): owner ruling 1 (separable qualified core) and RU-2,
  recorded on the ticket.
