---
id: TC-2213
title: "Replay recovers a library-only clock from the proving package"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-640
    type: verifies
  - target: ix://agent-ix/filament-specification/TC-1662
    type: references
---
# TC-2213: Replay recovers a library-only clock from the proving package

## Description

Verify that the replay recompile serves a library clause's own clock from
the proving package's lock `dependency_clock_selections`, with no clock in
the request, and refuses a missing record, an unused entry and an entry of
the wrong kind. Scope: FR-640-AC-11.

## Test Procedure

1. Compile TC-2212 step 1's program (`ms`) and replay `entry` through
   `fsl_replay::replay` with `test/plant` as its one `dependencies` entry.
2. Replay with the member removed from the package; with an added entry
   `spare`; with the `lib-ms` entry a `fixed_sample` configuration.
3. Compile a variant whose program also has a clause on `lib-ms`, remove the
   member from its package and replay it.

Test: `tests/it/replay_library_clocks.rs`
`replay_recovers_a_library_only_clock_from_the_proving_package`, tagged
`#[trace("TC-2213", "FR-640-AC-11", "FSpec-TC-1662", "FSpec-FR-323-AC-38")]`.

## Expected Results

- Step 1: the replay settles with value `true`.
- Step 2: `ReplayRefusal::Recompile` with `missing-clock-selection` naming
  `lib-ms` and `test/plant` at
  `/package/lock/dependency_clock_selections`; `unused-clock-selection` at
  `/package/lock/dependency_clock_selections/1`; `clock-kind-mismatch` at
  `/package/lock/dependency_clock_selections/0/clock/kind`. No result.
- Step 3: `missing-clock-selection` naming `lib-ms` and `test/plant`; the
  program's row does not serve the library.
