---
id: TC-154
title: "Refuse a capability carrier with an unsupported vocabulary version"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-057
    type: verifies
---
# TC-154: Refuse a capability carrier with an unsupported vocabulary version

## Description

Verify that a serialized carrier of capability labels that FSL reads is read
only under vocabulary identity `filament.capability-kind/v1`. The
carrier is the first one FSL reads, as #211 assigns. Scope: FR-057-AC-3.

## Test Procedure

1. Read a carrier that declares `filament.capability-kind/v1` and holds two
   admitted labels, as the control.
2. Remove the identity declaration and read it again.
3. Declare, one at a time, `filament.capability-kind/v0`,
   `filament.capability-kind/v2` and `filament.capability-kind/V1`, keeping the same
   labels.
4. Property: generate arbitrary identity strings other than
   `filament.capability-kind/v1`.

## Expected Results

- Step 1 reads both labels.
- Steps 2 to 4 each refuse the carrier with
  `invalid_capability`/`unsupported-version`, naming the received identity or
  its absence. No label of the refused carrier is read or reported as a kind,
  and no request report is produced from it.
- Assertions compare typed codes, causes and received bytes, never message
  text.
