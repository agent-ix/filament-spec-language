---
id: FR-567
title: "Qualify FSL on the final path the driver calls"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-029
    type: traces_to
  - target: ix://agent-ix/filament-spec-language/US-035
    type: traces_to
  - target: ix://agent-ix/filament-spec-language/FR-350
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-353
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-352
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-563
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-565
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-287
    type: references
  - target: "ix://agent-ix/filament-specification/FR-306"
    type: references
---
# FR-567: Qualify FSL on the final path the driver calls

## Description

The final path is the path a user's run takes: a `quire` verb in the driver
calls an FSL library operation (FR-287), which runs on the interpreter or on
a non-interpreter backend the build links (FR-296). The `qualify` gate's
verdict SHALL cover that path, not only the entry points the corpus runner
calls: the gate (FR-353) runs three parts in every run and reports
`qualified` only when all three pass.

| Part | Passes when | Name of a failure |
| --- | --- | --- |
| `corpus` | FR-353's verdict over the FR-350 run is `qualified` | each failing capability, as FR-353 names it |
| `lifecycle-matrix` | every `Case` of FR-565's `lifecycle_matrix()` gives a document or item whose category and exit code equal the case's expected ones | `lifecycle-matrix/<verb>/<kind>` for each failing case |
| `parity` | FR-563's runner over every non-interpreter backend the FSL workspace links reports no finding | `parity/<backend>` with the backend's findings |

The `lifecycle-matrix` part passes when every `Case` passes.

When the workspace links no non-interpreter backend, the `parity` part is
reported `no-backend` and passes: the final path is then the interpreter
alone, which the other two parts cover. Every parity finding, a
`reference-fault`, `reference-prepare-limit`, `generator-exhausted` or
`unbuildable-case` included, fails the part. FR-353's corpus preflight governs
`qualify` before any part runs, and its refusal exits 2 before any vector
executes. A `ParityRefusal` for an unreadable corpus, a document-level failure
only (FR-563), is likewise a preflight refusal of `qualify`. In a corpus that
passes the preflight, a problem confined to one vector refuses nothing: the
`corpus` part settles it per vector (FR-350), and where the parity runner
selects that vector it is an unbuildable case, which fails the `parity` part
when FR-563 records `unbuildable-case` for it.

The FR-352 report gains one `parts` member naming each part and its result;
its capability coverage is unchanged. The FSL version the report names is the
build's own: `env!("CARGO_PKG_VERSION")` of the `filament-spec-language`
package the gate ran against, never a version written into the gate's
source.

The gate's library entry is `xtask::qualify::run(inputs: &QualifyInputs)`,
which `cargo run --package xtask -- qualify` calls with the defaults:
FR-565's `lifecycle_matrix()` and the non-interpreter backends the workspace
links. Under xtask's `test-support` feature, `QualifyInputs` also accepts a
replacement matrix cell set, replacement backends, a substituted parity
reference and a replacement parity generated-unit count, so a test can make
each part fail without running the default generated units; the command line
offers none of them. With no replacement, and always from the command line,
the `parity` part takes FR-295's default seed and count 10,000 and
`ParityLimits::default()`. The replacement count sets how many units are
generated and excuses no finding on a unit that runs. The exit contract stays
FR-353's: 0 when `qualified`, 1 when any part fails, 2 on a refusal before a
verdict. The `complete-v1: qualified` label keeps FR-353's meaning and also
requires the `lifecycle-matrix` and `parity` parts to pass.

A non-interpreter backend's results are on the final path only through the
`parity` part: a backend the build links and the parity part has not passed
leaves the run `not-qualified`.

## Inputs

FR-353's inputs; the FR-565 cases; the non-interpreter backends the
workspace links.

## Outputs

The FR-352 report with the three parts and the verdict; the exit code.

## Behavior

- The `qualify` gate shall run the `corpus`, `lifecycle-matrix` and `parity`
  parts in every run.
- When all three parts pass, the gate shall report `qualified` and exit 0.
- If a `lifecycle-matrix` case's category or exit code differs from the
  case's expected one, then the gate shall report `not-qualified`, naming
  `lifecycle-matrix/<verb>/<kind>`, and exit 1.
- If the parity runner reports a finding for a linked backend, then the
  `qualify` gate shall report `not-qualified`, naming `parity/<backend>`, and exit 1.
- When the workspace links no non-interpreter backend, the gate shall report
  the `parity` part `no-backend` and treat it as passed.
- If the parity runner records `unbuildable-case` for a vector of a corpus
  that passes FR-353's preflight, then the gate shall report `not-qualified`,
  naming `parity/<backend>`, and exit 1 without refusing.
- The `qualify` command line shall offer no setting of the parity
  generated-unit count.
- The FR-352 report shall name each part and its result.
- The FR-352 report shall name as the FSL version the
  `filament-spec-language` package's `CARGO_PKG_VERSION` from the build that
  ran.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-567-AC-1 | Over FR-353-AC-1's passing fixture corpus, with every matrix case passing and a linked test backend that delegates to the interpreter, `qualify` reports `complete-v1: qualified`, names the three parts as passed, and exits 0. | Test (TC-1396) |
| FR-567-AC-2 | Through `xtask::qualify::run` under `test-support`, the AC-1 run with the matrix case set replaced by one whose `check`/`refusal` case expects exit 21 reports `not-qualified`, naming `lifecycle-matrix/check/refusal`, and exits 1, although the `corpus` part passes. | Test (TC-1396) |
| FR-567-AC-3 | Through `xtask::qualify::run` under `test-support`, the AC-1 run with FR-295-AC-1's rounding backend supplied in place of the delegating one reports `not-qualified`, naming `parity/jit` and the finding, and exits 1. | Test (TC-1396) |
| FR-567-AC-4 | The AC-1 run with no non-interpreter backend linked reports the `parity` part `no-backend` and `complete-v1: qualified`, exit 0; the AC-1 run whose parity reference (substituted under `test-support`) returns `Fault` reports `not-qualified`, naming `parity/jit` with a `reference-fault` finding, exit 1. | Test (TC-1396) |
| FR-567-AC-5 | The AC-1 report's FSL version is the filament-spec-language crate version (the root package's version, not the `xtask` crate's) and equals `env!("CARGO_PKG_VERSION")` evaluated in the `filament-spec-language` crate of the same build; built from a scratch copy whose `filament-spec-language` package version is changed to `9.9.9`, the report names `9.9.9`. | Test (TC-1396) |
| FR-567-AC-7 | Through `xtask::qualify::run` under `test-support`, the AC-1 run with a replacement generated-unit count of 3 and a delegating backend that counts its `prepare` calls passes, with one call for each corpus parity case and three more; `cargo xtask qualify` given `--count 3` refuses it as a malformed argument, exit 2; with no replacement the `parity` part takes FR-295's default seed, count 10,000 and `ParityLimits::default()`. | Test (TC-1396) and Inspection |
| FR-567-AC-8 | The AC-1 run over a corpus that passes the preflight and also holds a `runtime`/`run` vector that expects a value and names an absent source file does not refuse: it executes the other vectors, reports `not-qualified`, naming `parity/jit` with an `unbuildable-case` finding carrying that vector id, and exits 1; the AC-1 run over a corpus with two vectors of one id, and the AC-1 run over a corpus with a parent-traversing input path, each refuse under FR-353's preflight before any part runs, calling no backend's `prepare`, give no verdict and exit 2. | Test (TC-1396) |

## Dependencies

- [FR-350](FR-350-run-the-conformance-corpus-against-fsl.md), [FR-352](FR-352-report-capability-coverage.md), [FR-353](FR-353-settle-the-complete-v1-qualification-verdict.md): the corpus run, the report and the verdict, whose "if and only if" names the two parts this requirement adds.
- [FR-563](FR-563-run-the-parity-gate-and-fail-on-any-difference-from-the-interpreter.md): the parity runner.
- [FR-565](FR-565-publish-the-lifecycle-matrix-cases-the-driver-cli-half-runs.md): the matrix cases.
- [FR-287](FR-287-reach-fsl-through-the-driver-cli.md): the driver CLI that calls these operations.
- FSpec FR-306: backend parity.

## Overlap

The driver repository qualifies its own half of the path: its CLI half of
the matrix (FR-565). FSL qualifies the parity of `fsl-aot`'s build-and-run harness (FR-563), and the driver repository runs the real-generator AOT parity run.

## References

- QSL-390 (ARCH-50).
