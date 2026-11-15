---
id: TC-1706
title: "A clause-run function selection admits the run value form"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-650
    type: verifies
---
# TC-1706: A clause-run function selection admits the run value form

## Description

A clause-run function selection admits the run value form.

Scope: FR-650-AC-7.

## Test Procedure

Run FR-109-AC-4's `sameIdentity` as an FR-312 `function` selection with an identity-form and a key-form reference to `child`; run `short` with a Text argument.

Tag the tests `#[trace("TC-1706", "FR-650-AC-7")]`; they live in `tests/it/run_values.rs`.

## Expected Results

Both report `success`, `truth: true`, exit 0.
