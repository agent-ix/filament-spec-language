---
id: TC-3085
title: "Model declaration nodes are located at their selection"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-094
    type: verifies
---
# TC-3085: Model declaration nodes are located at their selection

## Description

Verify that every model declaration node `check` builds carries one
`generated` occurrence at ordinal 0 at the span of the occurrence that
selected its domain package, including a tuple or variant type reached only
through a `clock_place` `resolved_path` row, and that the emitted package
reads back strictly with its clock places.

Scope: FR-094-AC-17; FSpec FR-322-AC-485.

## Test Procedure

Fixture: TC-1860's timed domain package (tuple `Slot(Clock, Integer)`, variant
type `Phase` with cases `Armed` (`payloadType` `ix://filament/native/Clock`)
and `Idle`, and fields `s: Slot` and `p: Phase` on `ConfigVersion`), selected
by a unit under a dense time source on `ConfigVersion`.

1. Check and `emit_checked` the unit.
2. Read the `Slot` and `Phase` nodes' occurrences and `source_map` entries and
   the `model_root` entry of the selected identity.
3. Read the bytes strictly with the domain document as evidence and list the
   clock places `s.0` and `p.Armed` with their `resolved_path` rows.
4. Repeat steps 1 to 3 with the package also admitted without an alias.

## Expected Results

- Step 1: emission succeeds; no `UnlocatedOccurrence` refusal.
- Step 2: each node has exactly one occurrence and one entry, role
  `generated` at ordinal 0, whose region is the selecting alias's declaration
  span and equals the `model_root` entry's region; neither carries role
  `expression` nor a `generated_location` region.
- Step 3: the read is Verified and both places read back with their rows.
- Step 4: the results of steps 1 to 3 are unchanged.
