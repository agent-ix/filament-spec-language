---
id: TC-2804
title: "Source provision of a compiled handoff"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-992
    type: verifies
---
# TC-2804: Source provision of a compiled handoff

## Description

Verify that `CompiledHandoff::sources()` publishes exactly the compiled source bytes by declared label, usable as a replay byte provision. Scope: FR-992-AC-1, FR-992-AC-2, FR-992-AC-3.

Catches sources keyed by file name, extra or missing entries, and bytes that are not the ones compiled.

## Test Procedure

1. Write and compile a handoff of the order fixture with one supplied library; read `sources()`.
2. Build a protocol-trace replay request with `entries()` as its byte provision, then with the unit entry removed.

## Expected Results

- Step 1: two entries with the unit's and library's file bytes; `get` by the unit's provenance label returns the unit's bytes.
- Step 2: the replay reproduces; without the unit entry it is refused for the missing source.
