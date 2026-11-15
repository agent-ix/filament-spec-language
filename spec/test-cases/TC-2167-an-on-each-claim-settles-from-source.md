---
id: TC-2167
title: "An on-each probabilistic claim settles from source"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-186
    type: verifies
---
# TC-2167: An on-each probabilistic claim settles from source

## Description

An on-each probabilistic claim settles from source.

Scope: FR-186-AC-10.

## Test Procedure

Run `fsl analyze` over the `Link` model with the on-each claim of FR-186-AC-10. Test: `fsl-analyze/tests/it/probability_from_source.rs` `probabilistic_on_each_settles_per_instance`.

Tag the tests `#[trace("TC-2167", "FR-186-AC-10")]`.

## Expected Results

- The item settles `proved`, not `unsupported`.
