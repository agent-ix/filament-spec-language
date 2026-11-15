---
id: TC-2108
title: "The refinement-layering gate runs in make ci"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-345
    type: verifies
---
# TC-2108: The refinement-layering gate runs in make ci

## Description

Verify that the profile-layering gate is a CI gate and passes on its expected no-negotiation run. Scope: FR-345-AC-15.

## Test Procedure

1. Read the Makefile's `CI_CHECKS` list.
2. Run `make refinement-layering` over the generated real corpus.

## Expected Results

- Step 1: `refinement-layering` is a member.
- Step 2: the CLI exits 21 with the FR-345-AC-9 report and the target exits 0.
