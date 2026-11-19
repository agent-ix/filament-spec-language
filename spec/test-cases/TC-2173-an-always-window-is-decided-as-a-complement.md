---
id: TC-2173
title: "An always window is decided as a complement"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-204
    type: verifies
---
# TC-2173: An always window is decided as a complement

## Description

An always window is decided as a complement.

Scope: FR-204-AC-14.

## Test Procedure

Build and decide `Always` over `Retx` over every scheduler. Test: `fsl-analyze/tests/it/digital_clocks.rs` `always_window_is_decided_as_a_complement`.

Tag the tests `#[trace("TC-2173", "FR-204-AC-14")]`.

## Expected Results

- A product, no `TimedFormShape`; exact minimum `0`.
