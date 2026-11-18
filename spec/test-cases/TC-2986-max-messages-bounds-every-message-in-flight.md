---
id: TC-2986
title: "max_messages also bounds each channel's occupancy, counted per channel"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-225
    type: verifies
---
# TC-2986: max_messages also bounds each channel's occupancy, counted per channel

## Description

Verify that `max_messages` bounds a protocol channel's occupancy per channel,
cutting a send offered to a channel at the bound before its overflow policy
applies, while its RA meaning (live messages per location after garbage
collection) is unchanged. Scope: FR-225-AC-6.

## Test Procedure

1. Analyze FR-636-AC-4's `Burst` as a `global-conformance` item with
   `temporal.limits.max_messages` 1.
2. Analyze it with `max_messages` 2 and with no `max_messages`.
3. Check ADR-025 §10's `SB` under `ra` with `max_messages` unset and 1.

## Expected Results

- Step 1: the second send is unexpanded; the item settles as FR-636-AC-4
  states with its cause naming `max_messages`; the record states
  `max_messages` 1 reached.
- Step 2: the second send is rejected and the item settles `refuted` with the
  overflow as FR-572 keys it; unset, the record states `max_messages` 4, not
  reached.
- Step 3: FR-225-AC-1's and AC-3's records, unchanged.
