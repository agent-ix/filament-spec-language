---
id: TC-2775
title: "A typed size query runs to the same length as size"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-981
    type: verifies
---
# TC-2775: A typed size query runs to the same length as size

## Description

A typed size query runs to the same length as size.

Scope: FR-981-AC-4.

## Test Procedure

Run FR-981-AC-4's function claim through `command::run` over the ConfigVersion spine. Test: `tests/it/typed_size_run.rs`.

## Expected Results

- The claim holds, exit 0.
