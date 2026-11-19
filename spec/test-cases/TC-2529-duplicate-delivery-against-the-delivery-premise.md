---
id: TC-2529
title: "A duplicate delivery is reported against the channel's delivery premise"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-894
    type: verifies
---
# TC-2529: A duplicate delivery is reported against the channel's delivery premise

## Description

Verify that a duplicate delivery is reported against the channel's delivery premise. Scope: FR-894-AC-5.

Catches a duplicate delivery accepted on a channel whose premise forbids it.

## Test Procedure

1. Assess a recording with deliveries 0 and 1 of one message on an `at-least-once` channel, then on an `exactly-once-premise` channel.

## Expected Results

- Step 1: The first reports `duplicate-delivery`; the second reports a violated delivery premise; each record names the duplicate kind.
