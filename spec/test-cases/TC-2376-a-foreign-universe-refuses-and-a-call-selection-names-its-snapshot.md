---
id: TC-2376
title: "A foreign universe refuses and a call selection names its snapshot"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-650
    type: verifies
---
# TC-2376: A foreign universe refuses and a call selection names its snapshot

## Description

A foreign universe refuses and a call selection names its snapshot.

Scope: FR-650-AC-15.

## Test Procedure

Call `sameIdentity` with a foreign universe; run an `items` request whose `call` selection names an observation key as `snapshot`. Test: `tests/it/run_value_spellings.rs` `a_foreign_universe_refuses_and_call_selection_snapshot`.

Tag the tests `#[trace("TC-2376", "FR-650-AC-15")]`.

## Expected Results

- The foreign universe refuses `foreign_reference`/`foreign-universe`, position 0, exit 20.
- The `call` selection completes `{"boolean": true}`.
