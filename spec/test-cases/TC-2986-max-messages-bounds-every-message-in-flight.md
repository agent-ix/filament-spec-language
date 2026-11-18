---
id: TC-2986
title: "max_messages bounds every message in flight, on channels as in the RA component"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-225
    type: verifies
---
# TC-2986: max_messages bounds every message in flight, on channels as in the RA component

## Description

Verify that the `max_messages` method bound counts the messages queued on
protocol channels with no RA access, cuts a message-offering step at the
bound before the channel's overflow policy applies, and is stated in the
record. Scope: FR-225-AC-6.

## Test Procedure

1. Analyze FR-636-AC-4's `Burst` as a `global-conformance` item with
   `temporal.limits.max_messages` 1.
2. Analyze it with `max_messages` 2.
3. Analyze it with no `max_messages`.

## Expected Results

- Step 1: the second send is unexpanded; the item settles as FR-636-AC-4
  states with its cause naming `max_messages`; the record states
  `max_messages` 1 reached.
- Step 2: the second send is rejected and the item settles `refuted` with the
  overflow as FR-572 keys it.
- Step 3: the record states `max_messages` 4, not reached.
