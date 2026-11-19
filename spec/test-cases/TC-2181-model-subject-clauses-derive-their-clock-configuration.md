---
id: TC-2181
title: "Model-subject clauses derive their clock configuration under every profile"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-640
    type: verifies
---
# TC-2181: Model-subject clauses derive their clock configuration under every profile

## Description

Model-subject clauses derive their clock configuration under every profile.

Scope: FR-640-AC-7.

## Test Procedure

Compile, with the ConfigVersion model and no `clocks`, a unit holding one fixed-sample and one timestamped-event clause over `Config::ConfigVersion` on clock `model-steps`. Test: `tests/it/clock_derivation.rs` `model_subject_clauses_derive_their_clock_configuration`.

Tag the tests `#[trace("TC-2181", "FR-640-AC-7")]`.

## Expected Results

- Exit 0.
- The two rows hold the derived `fixed_sample` and `timestamped_event` configurations of FR-640's table.
