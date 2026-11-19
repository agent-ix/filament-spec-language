---
id: TC-3124
title: "One live-occurrence count per application, whatever its root"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/NFR-130
    type: verifies
---
# TC-3124: One live-occurrence count per application, whatever its root

## Description

The live-occurrence limit counts every frame of an application in one sum, whether the root is an interpreted function or a registered host body.

Scope: NFR-130-AC-5.

## Test Procedure

Acquire a checked closure with `c(a: Integer, b: Integer, x: Integer)` calling `n(a: Integer, b: Integer, x: Integer)`. Through `ApplicationRegistry::begin_call` with `live_occurrences` 10, run (a) an interpreted root `p(a: Integer, b: Integer)` that calls `c`, and (b) a registered host body for `p` that requests `c` and propagates its result. Repeat with a two-argument `c`. Test: `fsl-replay/tests/it/application_live_occurrences.rs (new)` `live_occurrences_aggregate_across_host_and_interpreted_roots`.

Tag the tests `#[trace("TC-3124", "NFR-130-AC-5")]`.

## Expected Results

- Three-argument `c`: both runs stop incomplete with `LiveOccurrences`, limit 10, consumed 7, next charge 4, at `c`'s call of `n`; the outcomes are equal.
- Two-argument `c`: both complete with equal values and `peak_live_occurrences` 10.
