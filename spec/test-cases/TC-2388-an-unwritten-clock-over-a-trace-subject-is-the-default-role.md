---
id: TC-2388
title: "An unwritten clock over a trace subject is the default role"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-610
    type: verifies
---
# TC-2388: An unwritten clock over a trace subject is the default role

## Description

An unwritten clock over a trace subject is the default role.

Scope: FR-610-AC-5.

## Test Procedure

Check the clause with no `clock` under the infinite-trace, event-position and fixed-sample profiles and compare each with the clause written `clock "default"`. Test: `fsl-semantics/src/check/temporal_clause/tests.rs` `an_unwritten_clock_is_the_default_role`.

Tag the tests `#[trace("TC-2388", "FR-610-AC-5")]`.

## Expected Results

- Each checks, records the role `default` and has the written form's horizon.
