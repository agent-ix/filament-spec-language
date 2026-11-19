---
id: TC-3062
title: "An on-each item reports its conditional certificate and instance"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-741
    type: verifies
---
# TC-3062: An on-each item reports its conditional certificate and instance

## Description

Through `fsl analyze`, an `on each` proof carries a `conditional`
certificate and its refutation names the failing instance in the
counterexample's `instance` member.

Scope: FR-741-AC-5.

## Test Procedure

Run `fsl analyze` over the `Link` model under `Even`, with a start snapshot
holding `m` undelivered and `n` delivered, for FR-875's claim `E` at
`rational(9, 10)` and at `rational(99, 100)`. Test:
`fsl-analyze/tests/it/probability_from_source.rs`
`probabilistic_on_each_reports_conditional_certificate_and_instance`.

Tag the test `#[trace("TC-3062", "FR-741-AC-5")]`.

## Expected Results

- At `9/10`: `proved`, certificate kind `conditional`, mode `workload`,
  numerator `391/400` and denominator 1 at `m`'s initial key; the document
  is accepted by `fsl-probability-check` in a separate process.
- At `99/100`: `refuted`, counterexample `binding` null and `instance`
  naming `m`.
