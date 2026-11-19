---
id: TC-1822
title: "Check a source map type, its reads and its equality"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-680
    type: verifies
---
# TC-1822: Check a source map type, its reads and its equality

## Description

Verify FR-680-AC-3: the source `Map<K, V>` type, its key rule, `m[k]` under its obligation and map equality admission.

## Test Procedure

Check the map parameter and reads of FR-680-AC-3 in `fsl-semantics/tests/it/type_decisions.rs`, the conformance cases `map-accepted.native` and `map-ill-typed.native`, and `TypeEnvironment::check_equality` over two map types in `fsl-eval/tests/it/equality_matrix.rs`.

## Expected Results

The declarations and guarded read check, the unguarded read is `unproved-presence` at `k`, a record key type is `operator-ineligible`, and map equality is admitted.
