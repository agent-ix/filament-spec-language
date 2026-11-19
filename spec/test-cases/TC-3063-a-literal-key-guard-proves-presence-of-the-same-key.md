---
id: TC-3063
title: "A literal-key guard proves presence of the same key"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-680
    type: verifies
---
# TC-3063: A literal-key guard proves presence of the same key

## Description

Verify FR-680-AC-8: `contains(m, 1)` discharges the presence obligation of `m[1]` and of no read of another literal or of a path, and `contains(m, k)` discharges no literal read.

## Test Procedure

Run `fsl check` over the command-intake collections corpus: one `.native` unit per body of FR-680-AC-8, each declaring `profile v = "filament.value.complete/v1"` and a function over `m: Map<Int[0,9],Integer>[0,3]` (and `k: Int[0,9]` where named), compared against its `.expected.json`.

## Expected Results

The two guarded literal reads succeed at S4 with exit 0; the three mismatched-key reads refuse `undefined_expression`/`unproved-presence` at the index with exit 20.
