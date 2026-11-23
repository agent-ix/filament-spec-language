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

- The item settles `proved`, not `unsupported`; the start snapshot holds one `Msg` key, so there is one instance, activated at position 0, and its proof basis carries the exact value (`391/400`) of the `on origin` claim `probabilistic Eventually using inf on M over (m: M::Msg) under Even on origin { probability >= rational(9, 10) [ eventually holds(m.delivered) ] }`, its FR-741 `entries` row naming the key as `instance` with a null `binding` where that claim's row names it as `binding`.
