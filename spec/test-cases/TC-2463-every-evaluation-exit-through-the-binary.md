---
id: TC-2463
title: "Every evaluation exit code is produced through the fsl binary"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-870
    type: verifies
---
# TC-2463: Every evaluation exit code is produced through the fsl binary

## Description

Verify exits 10, 11, 20 and 22 from the `fsl` process. Scope: FR-870-AC-1 to FR-870-AC-5; AC-1 to AC-4 are driver tests, AC-5 an inspection of the filament-examples suite.

## Test Procedure

1. Run the violating-parent clause request and the `7 mod 0` call request through `fsl run`.
2. Run FR-870's timestamped finite-window request with `p` absent at stamp 2, then with `p` true at stamp 5.
3. Run an unknown-function request and `seven` with `accounting.work_units` 0.
4. Inspect the filament-examples exit-11 case and run the examples runner.

## Expected Results

- Step 1: exit 10 twice, categories violation and undefined (`division-by-zero`).
- Step 2: exit 11, category indeterminate, `MissingValuation`; then exit 0.
- Step 3: exit 20 and exit 22.
- Step 4: the case expects 11 and the runner fails on any other exit.

Test: filament-driver `fsl-cli/tests/binary_process.rs`.
