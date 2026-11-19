---
id: TC-2166
title: "An on-each activation checks and is recorded"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-186
    type: verifies
---
# TC-2166: An on-each activation checks and is recorded

## Description

An on-each activation checks and is recorded.

Scope: FR-186-AC-8, FR-186-AC-9.

## Test Procedure

Check the service model's claim with `on each (m: Service::Server)`, with and without `when (m.phase = Service::Phase::Idle)`; then `on each (k: Integer)` and `when (1)`. Test: `fsl-semantics/tests/it/probabilistic_declarations.rs` `probabilistic_on_each_checks` and `probabilistic_on_each_non_object_type_refused`.

Tag the tests `#[trace("TC-2166", "FR-186-AC-8")]`.

## Expected Results

- Both claims check with activation `Each`, the guard recorded only when written; their normal forms differ.
- `on each (k: Integer)`: `over-not-object-type` at `k`; `when (1)`: `ill_typed`/`type-mismatch` at `1`.
