---
id: TC-2975
title: "A compile request selects Kernel checking"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-952
    type: verifies
---
# TC-2975: A compile request selects Kernel checking

## Description

Verify the `native-compile/1` member `check_mode`: its two values, its
`linked` default, its effect on the written package and handoff, and its
refusals. Scope: FR-952-AC-5.

## Test Procedure

1. Compile the unit with `g` and `r` (`10 mod x`) with `"check_mode": "kernel"`, `output` and `handoff`; read `check_mode` on every function node in both.
2. Compile the same unit with no `check_mode` and with `"linked"`; compile it without `r` and with no `check_mode`.
3. Compile with `"check_mode"` `"Kernel"`, `1` and `null`.
4. Replay at `r`'s `10 mod x` record over the Kernel handoff.

Test: `tests/it/compile_package_output.rs`.

## Expected Results

- Step 1: exit 0; `kernel` on `g` and `r`, `linked` on the `precondition` clause function, in both files.
- Step 2: `undefined_expression`/`unproved-nonzero` at `r` and no package; `linked` on `g` without `r`.
- Step 3: `invalid-request` at stage `request`, exit 20, naming `check_mode`.
- Step 4: as FR-098-AC-34.
