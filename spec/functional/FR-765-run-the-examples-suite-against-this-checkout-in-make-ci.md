---
id: FR-765
title: "Run the examples suite against this checkout in make ci"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-004
    type: traces_to
  - target: ix://agent-ix/filament-driver/FR-021
    type: depends_on
---
# FR-765: Run the examples suite against this checkout in make ci

## Description

The language repository's `make examples-check` SHALL run the filament-driver
examples gate (driver FR-021) with an `fsl` binary built from this checkout's
language crates, and `make ci` SHALL run `examples-check`. A language change
that alters a recorded example outcome then fails the language repository's
own local gate, before the driver's lock moves to it. No hosted workflow runs
the gate: hosted CI stays manual, and no workflow SHALL declare a `push` or
`pull_request` trigger.

## Rationale

The examples suite records what `fsl` gives for every example. Hosted
workflows run only on `workflow_dispatch` to save CI minutes, so the local
`make ci` is the gate that runs in practice and must reach the suite.

## Inputs

- A filament-driver checkout named by the Make variable `FSL_DRIVER_DIR`,
  by default `filament-driver` beside this repository.
- A filament-examples checkout named by `FSL_EXAMPLES_DIR`, by default
  `filament-examples` beside this repository.

## Outputs

- Exit 0 when every running and staged example case holds with this
  checkout's `fsl`; a nonzero exit, with the driver gate's failure lines
  naming each changed case, otherwise.

## Behavior

- `cargo xtask examples-check` SHALL run the driver's
  `cargo test --locked --test examples_gate` in the driver checkout with
  `FSL_EXAMPLES_DIR` set and `FSL_LANGUAGE_DIR` naming this workspace, which
  makes the driver gate patch every language package of its `fsl` build to
  this checkout's crates in a copy of the driver workspace, never writing the
  driver's `Cargo.lock`.
- When the driver gate passes, `cargo xtask examples-check` SHALL fail unless
  the gate's output has a line `fsl built from language checkout <path>`
  naming this checkout's absolute path and no such line naming another path.
  A driver older than its FR-021-AC-7 ignores `FSL_LANGUAGE_DIR`, builds `fsl`
  from git and prints no such line, so it cannot pass as a check of this
  checkout.
- `make examples-check` SHALL run that command and then the
  `xtask/tests/examples_check.rs` negative check; the Makefile only passes the
  variables through.
- When either variable is empty, or names a folder without the files that
  identify its checkout (`Cargo.toml` and `tests/examples_gate.rs` for the
  driver; `examples/` and `tools/fsl-examples/Cargo.toml` for the suite),
  the target SHALL print an error naming the variable, the folder, the missing
  file and the repository, and exit nonzero without building. It never skips.
- `CI_CHECKS` SHALL list `examples-check`.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-765-AC-1 | `CI_CHECKS` lists `examples-check`, and `make -n ci-internal` prints its `xtask examples-check` and `--test examples_check` recipe lines. | Test (TC-2190) |
| FR-765-AC-2 | No `.github/workflows/*.yml` declares a `push` or `pull_request` trigger, in scalar, flow-list or block form. | Test (TC-2191) |
| FR-765-AC-3 | With `FSL_EXAMPLES_DIR` naming a copy of the suite in which one running case's recorded `category` differs from what this checkout's `fsl` gives, `make examples-check` exits nonzero and its output names that case as `<system>/<case>` and reports the failed driver gate. | Test (TC-2192) |
| FR-765-AC-4 | With `FSL_DRIVER_DIR` empty or naming an absent folder, or `FSL_EXAMPLES_DIR` naming an absent folder or one without `tools/fsl-examples/Cargo.toml`, `make examples-check` exits nonzero with a message naming the variable, the folder and the missing file, and runs no later recipe line. | Test (TC-2193) |
| FR-765-AC-5 | With a driver gate that passes and prints no `fsl built from language checkout` line, or prints one naming another path, `make examples-check` exits nonzero and names this checkout (and the other path); with one naming this checkout it exits 0. | Test (TC-2194) |

## Dependencies

- Driver FR-021 owns the gate, its suite checks and the `FSL_LANGUAGE_DIR`
  build.
- The filament-examples repository owns the cases, their records and the
  runner.
