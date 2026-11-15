---
id: TC-1821
title: "Read a tuple position"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-680
    type: verifies
---
# TC-1821: Read a tuple position

## Description

Verify FR-680-AC-2: `p[n]` reads a tuple position and `p.0` stays a syntax error.

## Test Procedure

Check `p[0]`, `p[1]`, `p[2]`, `p[i]` and parse `p.0` over `tuple Pair(Boolean, Integer);`. Stubs in `fsl-semantics/tests/it/type_decisions.rs` and the conformance cases `tuple-position-accepted.native` and `tuple-position-ill-typed.native`.

## Expected Results

`p[0]` is `Boolean`, `p[1]` is `Integer`; `p[2]` and `p[i]` are `ill_typed`/`operator-ineligible`; `p.0` is refused at S1.
