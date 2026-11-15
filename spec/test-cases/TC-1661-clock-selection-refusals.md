---
id: TC-1661
title: "Clock selection refusals"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-640
    type: verifies
---
# TC-1661: Clock selection refusals

## Description

Verify FR-640-AC-3 and AC-4: missing, unused and mismatched clock selections, malformed configurations and a selection with a package refuse with a cause, a message and a locus; a model-steps binding needs no selection.

## Test Procedure

Use TC-1660's unit and request. Remove `plant-ms`; add `spare`; configure `events` as `timestamped_event`; give an unknown `kind`; give a zero `period`; add `clocks` to a request that selects a package. Compile FR-330-AC-1's `Counter` unit with no `clocks`.

## Expected Results

- Missing, unused and mismatched: stage `emit`, `invalid-request`, exit 20, `details.cause` `missing-clock-selection`, `unused-clock-selection`, `clock-kind-mismatch`, `details.clock` the name, a message naming the clock, and the clock argument's locus where one declaration is at fault.
- Malformed configurations and the package case: stage `request`, `invalid-request`, exit 20, naming the member.
- The `Counter` unit compiles and its row holds the derived `event_position` configuration.
