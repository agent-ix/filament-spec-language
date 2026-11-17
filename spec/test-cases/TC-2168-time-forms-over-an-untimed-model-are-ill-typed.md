---
id: TC-2168
title: "Time forms over an untimed model are ill typed"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-186
    type: verifies
---
# TC-2168: Time forms over an untimed model are ill typed

## Description

Time forms over an untimed model are ill typed.

Scope: FR-186-AC-11.

## Test Procedure

Check the three forms of FR-186-AC-11 over the untimed service model. Test: `fsl-semantics/tests/it/probabilistic_declarations.rs` `time_forms_over_an_untimed_model_are_ill_typed`.

Tag the tests `#[trace("TC-2168", "FR-186-AC-11")]`.

## Expected Results

- Each: `ill_typed`/`type-mismatch` at the form's quantity, measure or bound; none `not-yet-implemented`.
