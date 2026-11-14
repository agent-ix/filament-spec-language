---
id: TC-1113
title: "T12-A keeps CG's evaluator corpus compile_package call in its replay module"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-060
    type: verifies
---
# TC-1113: T12-A keeps CG's evaluator corpus compile_package call in its replay module

## Description

Verify FR-060's "T12-A and the evaluator differential corpus": CG's corpus
producer's `compile_package` call passes only inside CG's `replay` module.
Scope: FR-060-AC-8.

## Test Procedure

1. Run `arch-lint api-surface` with an FSL fixture tree whose
   `fsl-replay/src/lib.rs` exists and a CG fixture tree holding a
   `fsl_replay::compile_package(..)` call in
   `src/replay/evaluator_corpus.rs`.
2. Move the `fsl_replay::compile_package(..)` call to `src/oracle/mod.rs`
   and rerun.

## Expected Results

- Step 1: T12-A passes.
- Step 2: T12-A fails, naming the call's file, line and module.

## Metadata

- Priority: P1
- Target Integration: `tools/arch-lint/api_surface.rs`
- Automation: Automated Rust unit tests

## Dependencies

**Upstream:** [FR-060](../functional/FR-060-check-fsl-api-surface-boundary.md).
**Downstream:** none.
